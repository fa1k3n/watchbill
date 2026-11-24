//! A runner's hooks in a view (#160). A hook's program, and what it reads, are in the sandbox only where the
//! runner's `binds` put them there. Before a session in a view starts, `wb` reads the settings of a runner it knows,
//! Claude Code, and names each hook command whose program the sandbox does not hold, so that a hook does not fail in
//! silence. It starts the session all the same. It only reads.
//!
//! - **The settings** are the person's `~/.claude/settings.json` and the project's `.claude/settings.json` and
//!   `.claude/settings.local.json` as `HEAD` has them. Each is read as a plain file, never through a link, through a
//!   cap, by the one loader of `wb`.
//! - **The program** of a hook's command is its first word, as the shell takes it. It is held where the sandbox has it
//!   as an executable file: under the system's folders or a bind of the runner, as written and as resolved, or in the
//!   checkout where the view holds it. A word `wb` cannot tell is named.

use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

use crate::bounded::{self, Fault};
use crate::yamlfile::{self, Node, Value};

/// The file name of the program of the one runner whose settings `wb` knows: Claude Code.
pub const PROGRAM: &str = "claude";

/// The person's folder of the runner, from the home folder.
pub const USER_FOLDER: &str = ".claude";

/// The person's settings, from the home folder.
pub const USER_SETTINGS: &str = ".claude/settings.json";

/// The project's settings, from the root, as `HEAD` has them.
pub const PROJECT_SETTINGS: [&str; 2] = [".claude/settings.json", ".claude/settings.local.json"];

/// The most a settings file is read.
pub const CAP: u64 = bounded::TEXT_CAP;

/// How many commands are named at most; the rest are counted.
pub const NAMED: usize = 20;

/// How many characters of a command are shown at most.
pub const SHOWN: usize = 200;

/// The longest path the kernel takes: a longer first word is not a program.
const PATH_MAX: usize = 4096;

/// What the line of a hook that will fail says.
pub const FAILS: &str = "will fail in the view";

/// The shell's builtins that run no program.
const BUILTINS: [&str; 16] = [":", "true", "false", "exit", "echo", "printf", "test", "[", "cd", "export", "set", "unset", "read", "return", "shift", "wait"];

/// The shell's words that run another program, or more than one: the program cannot be told from the first word.
const RUNS_ANOTHER: [&str; 22] = [
    "exec", "eval", "source", ".", "command", "builtin", "if", "then", "else", "elif", "fi", "for", "while", "until", "do", "done", "case", "esac",
    "select", "function", "time", "coproc",
];

/// The variable the runner sets to the checkout for a hook's command.
const PROJECT_DIR: &str = "CLAUDE_PROJECT_DIR";

/// The sandbox a session would run in, as far as a hook's program is concerned.
pub struct Sandbox<'a> {
    /// The checkout, where the view stands in the sandbox.
    pub checkout: &'a Path,
    /// The folder of the view.
    pub view: &'a Path,
    /// The home folder, which `~/` names, or none.
    pub home: Option<&'a Path>,
    /// The runner's binds, each from the root.
    pub binds: &'a [(PathBuf, bool)],
    /// The folders of the organisation's file, which the sandbox covers.
    pub covered: &'a [PathBuf],
    /// `PATH`, as the session inherits it.
    pub path: Option<&'a OsStr>,
}

/// The first word of a command.
enum Word {
    /// A builtin of the shell that runs no program.
    Builtin,
    /// A program looked for on `PATH`.
    Name(String),
    /// A program named by its path.
    Path(PathBuf),
}

/// Why the program of `command` is not in the sandbox, or none where the sandbox holds it.
pub fn unheld(command: &str, sandbox: &Sandbox) -> Option<String> {
    match first_word(command, sandbox.checkout, sandbox.home) {
        None => Some("wb cannot tell its program from the command".to_string()),
        Some(Word::Builtin) => None,
        Some(Word::Path(path)) => reachable(&path, sandbox).err(),
        Some(Word::Name(name)) => {
            let mut found = None;
            for folder in std::env::split_paths(sandbox.path.unwrap_or_default()) {
                if !folder.is_absolute() {
                    continue;
                }
                let candidate = folder.join(&name);
                match reachable(&candidate, sandbox) {
                    Ok(()) => return None,
                    Err(why) => {
                        if found.is_none() && fs::symlink_metadata(&candidate).is_ok() {
                            found = Some(why);
                        }
                    }
                }
            }
            Some(found.unwrap_or_else(|| format!("{} is on no folder of PATH", name.escape_debug())))
        }
    }
}

/// The first word of `command`, as the shell would take it, with `$CLAUDE_PROJECT_DIR` and a leading `~/` filled; none
/// where it cannot be told.
fn first_word(command: &str, checkout: &Path, home: Option<&Path>) -> Option<Word> {
    let text = command.trim_start_matches([' ', '\t', '\n']);
    let chars: Vec<char> = text.chars().collect();
    let checkout = checkout.to_str()?;
    let mut word = String::new();
    let mut at = 0;
    // The variable of the checkout at `at`, a `$`: how many characters it takes.
    // It is compared in place: nothing is copied for each `$`.
    let name: Vec<char> = PROJECT_DIR.chars().collect();
    let project = |at: usize| -> Option<usize> {
        let rest = &chars[at + 1..];
        if rest.first() == Some(&'{') && rest[1..].starts_with(&name) && rest.get(name.len() + 1) == Some(&'}') {
            return Some(name.len() + 3);
        }
        let after = rest.strip_prefix(&name[..])?;
        (!after.first().is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_')).then_some(name.len() + 1)
    };
    while let Some(&c) = chars.get(at) {
        // A word longer than a path can be is not a program.
        if word.len() > PATH_MAX {
            return None;
        }
        match c {
            ' ' | '\t' | '\n' | ';' | '&' | '|' | '<' | '>' | '(' | ')' => break,
            '\'' => {
                let end = chars[at + 1..].iter().position(|c| *c == '\'')?;
                word.extend(&chars[at + 1..at + 1 + end]);
                at += end + 2;
            }
            '"' => {
                at += 1;
                loop {
                    match chars.get(at)? {
                        '"' => break,
                        '$' => {
                            word.push_str(checkout);
                            at += project(at)?;
                        }
                        '`' | '\\' => return None,
                        other => {
                            word.push(*other);
                            at += 1;
                        }
                    }
                }
                at += 1;
            }
            '$' => {
                word.push_str(checkout);
                at += project(at)?;
            }
            '~' if at == 0 => match chars.get(1) {
                None | Some('/') => {
                    word.push_str(home?.to_str()?);
                    at += 1;
                }
                _ => return None,
            },
            '`' | '\\' | '*' | '?' | '[' | ']' | '{' | '}' | '!' | '#' | '=' => {
                // `[` alone is the builtin `test`.
                if c == '[' && at == 0 && chars.get(1).is_none_or(|next| next.is_whitespace()) {
                    return Some(Word::Builtin);
                }
                return None;
            }
            other => {
                word.push(other);
                at += 1;
            }
        }
    }
    if word.is_empty() || word.len() > PATH_MAX || RUNS_ANOTHER.contains(&word.as_str()) {
        return None;
    }
    if BUILTINS.contains(&word.as_str()) {
        return Some(Word::Builtin);
    }
    if !word.contains('/') {
        return Some(Word::Name(word));
    }
    let path = if word.starts_with('/') { PathBuf::from(&word) } else { Path::new(checkout).join(&word) };
    Some(Word::Path(normal(&path)))
}

/// `path` with `.` and `..` taken out by its words alone, as the kernel takes an absolute path with no link in it.
fn normal(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Where the file that the sandbox has at `path` is on the host, or why the sandbox does not hold `path`. Bubblewrap
/// binds what a bind leads to at the bind's own path, so a bind that is a link holds what it leads to. The folder of the
/// organisation's file is covered.
fn host_of(path: &Path, sandbox: &Sandbox) -> Result<PathBuf, String> {
    if sandbox.covered.iter().any(|folder| path.starts_with(folder)) {
        return Err(format!("{} lies in the folder of the organisation's file, which the sandbox covers", path.display()));
    }
    let bind = sandbox.binds.iter().map(|(bind, _)| bind).filter(|bind| path.starts_with(bind)).max_by_key(|bind| bind.components().count());
    if let Some(bind) = bind {
        let base = fs::canonicalize(bind).map_err(|_| format!("{} is not there", path.display()))?;
        let rest = path.strip_prefix(bind).unwrap_or(Path::new(""));
        return Ok(if rest.as_os_str().is_empty() { base } else { base.join(rest) });
    }
    if crate::view::SYSTEM.iter().any(|folder| path.starts_with(folder)) {
        return Ok(path.to_path_buf());
    }
    Err(format!("{} lies outside what the sandbox holds", path.display()))
}

fn executable(meta: &fs::Metadata) -> bool {
    meta.is_file() && meta.permissions().mode() & 0o111 != 0
}

/// Whether the sandbox holds `path` as a program; the error says why not.
fn reachable(path: &Path, sandbox: &Sandbox) -> Result<(), String> {
    if let Ok(inside) = path.strip_prefix(sandbox.checkout) {
        return match fs::symlink_metadata(sandbox.view.join(inside)) {
            Ok(meta) if executable(&meta) => Ok(()),
            _ => Err(format!(
                "{} is in the checkout, and the view does not hold it as a program: a bind in the checkout is refused, so a hook's program is installed outside the checkout and bound there",
                path.display()
            )),
        };
    }
    let host = host_of(path, sandbox)?;
    let real = fs::canonicalize(&host).map_err(|_| format!("{} is not there", path.display()))?;
    if real != host {
        // A link below a bind or in the system is followed in the sandbox: what it leads to must be there, at its path.
        if real.starts_with(sandbox.checkout) {
            return reachable(&real, sandbox);
        }
        if host_of(&real, sandbox).ok().as_ref() != Some(&real) {
            return Err(format!("{} leads to {}, which lies outside what the sandbox holds", path.display(), real.display()));
        }
    }
    match fs::metadata(&real) {
        Ok(meta) if executable(&meta) => Ok(()),
        _ => Err(format!("{} is not an executable file", path.display())),
    }
}

/// A hook of a settings file: its event, and its command where its `type` is `command`.
struct Hook {
    event: String,
    command: Option<String>,
}

/// A text of the file, quoted or written plain.
fn text_of(node: &Node) -> Option<&str> {
    match &node.value {
        Value::Quoted(text) | Value::Plain(text, _) => Some(text),
        _ => None,
    }
}

fn key<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    match &node.value {
        Value::Table(entries) => entries.iter().find(|(key, _)| key.text == name).map(|(_, value)| value),
        _ => None,
    }
}

/// The hooks of a settings file, in the order of the file: `hooks.<event>[].hooks[]`. None where `hooks` is there in
/// another form.
fn hooks_of(root: &Node) -> Option<Vec<Hook>> {
    let Some(hooks) = key(root, "hooks") else { return Some(Vec::new()) };
    let Value::Table(events) = &hooks.value else { return None };
    let mut found = Vec::new();
    for (event, matchers) in events {
        let Value::List(matchers) = &matchers.value else { return None };
        for matcher in matchers {
            let Value::List(each) = &key(matcher, "hooks")?.value else { return None };
            for hook in each {
                let Value::Table(_) = &hook.value else { return None };
                let kind = text_of(key(hook, "type")?)?;
                let command = match kind {
                    "command" => Some(text_of(key(hook, "command")?)?.to_string()),
                    _ => None,
                };
                found.push(Hook { event: event.text.clone(), command });
            }
        }
    }
    Some(found)
}

/// A settings file as `wb` found it.
struct Settings {
    /// The file, as it is said.
    shown: String,
    /// Its bytes, or why they were not read; none where it is not there.
    read: Option<Result<Vec<u8>, String>>,
    /// Why the runner in the view does not read it, or none where it does.
    not_shown: Option<&'static str>,
    /// What is said of the file of the working tree where it is not as `HEAD` has it, which the view never holds.
    working: Option<String>,
}

/// git, in the checkout at `root`, with no replace refs and no `GIT_*` of `wb`'s.
fn git(root: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).arg("--no-replace-objects").args(args);
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    command.env("GIT_LITERAL_PATHSPECS", "1").stdin(Stdio::null()).stderr(Stdio::null());
    let output = command.output().map_err(|error| format!("git could not be run: {}", error))?;
    if !output.status.success() {
        return Err(format!("`git {}` ended with {}", args.first().unwrap_or(&""), output.status.code().unwrap_or(-1)));
    }
    Ok(output.stdout)
}

/// The entry of `path` in `HEAD`: its mode, its object and its size (`-` for what is not a blob), or none.
fn head_entry(root: &str, path: &str) -> Result<Option<(String, String, String)>, String> {
    let listed = git(root, &["ls-tree", "-z", "-l", "--full-tree", "HEAD", "--", path])?;
    let entry = String::from_utf8_lossy(&listed);
    let Some((meta, _)) = entry.split('\0').find(|entry| !entry.is_empty()).and_then(|entry| entry.split_once('\t')) else { return Ok(None) };
    match meta.split_whitespace().collect::<Vec<&str>>()[..] {
        [mode, _, object, size] => Ok(Some((mode.to_string(), object.to_string(), size.to_string()))),
        _ => Err(Fault::NotPlain.to_string()),
    }
}

/// What a settings file of `HEAD` holds: none where it is not there, and the error where it is not a plain blob of
/// `CAP` bytes or less.
fn head_blob(root: &str, entry: Option<&(String, String, String)>) -> Result<Option<Vec<u8>>, String> {
    let Some((mode, object, size)) = entry else { return Ok(None) };
    if !matches!(mode.as_str(), "100644" | "100755") {
        return Err(Fault::NotPlain.to_string());
    }
    if size.parse::<u64>().map_or(true, |size| size > CAP) {
        return Err(Fault::TooBig(CAP).to_string());
    }
    let bytes = git(root, &["cat-file", "blob", object])?;
    if bytes.len() as u64 > CAP {
        return Err(Fault::TooBig(CAP).to_string());
    }
    Ok(Some(bytes))
}

/// Whether the file of the working tree at `path` is there and not as `HEAD` has it, by its kind, size and mode alone:
/// it is looked at, never read.
fn working_differs(root: &str, path: &str, entry: Option<&(String, String, String)>) -> bool {
    let Ok(meta) = fs::symlink_metadata(Path::new(root).join(path)) else { return false };
    match entry {
        None => true,
        Some((mode, _, size)) => {
            !(meta.is_file() && size.parse::<u64>() == Ok(meta.len()) && (meta.permissions().mode() & 0o111 != 0) == (mode == "100755"))
        }
    }
}

/// The settings of the runner whose program is `program`, where `wb` knows them: the person's, and the project's as
/// `HEAD` of the checkout at `root` has them.
fn settings(program: &str, root: &str, sandbox: &Sandbox) -> Vec<Settings> {
    if Path::new(program).file_name() != Some(OsStr::new(PROGRAM)) {
        return Vec::new();
    }
    let mut found = Vec::new();
    if let Some(home) = sandbox.home {
        let folder = home.join(USER_FOLDER);
        let path = home.join(USER_SETTINGS);
        // The folder is a plain folder too: one that is a link is not followed.
        let read = match fs::symlink_metadata(&folder) {
            Err(_) => None,
            Ok(meta) if !meta.is_dir() => Some(Err(format!("~/{} is not a plain folder", USER_FOLDER))),
            Ok(_) => match bounded::read_plain(&path, CAP) {
                Err(Fault::NotFound) => None,
                Err(fault) => Some(Err(fault.to_string())),
                Ok(bytes) => Some(Ok(bytes)),
            },
        };
        let bound = sandbox.binds.iter().any(|(bind, _)| path.starts_with(bind));
        found.push(Settings {
            shown: format!("~/{}", USER_SETTINGS),
            read,
            not_shown: (!bound).then_some("is not in the runner's `binds`: the runner does not read it in the view"),
            working: None,
        });
    }
    for path in PROJECT_SETTINGS {
        let (read, working) = match head_entry(root, path) {
            Err(why) => (Some(Err(why)), None),
            Ok(entry) => (
                head_blob(root, entry.as_ref()).transpose(),
                working_differs(root, path, entry.as_ref())
                    .then(|| format!("{} of the working tree is not as HEAD has it: the runner does not read it in the view, and none of its hooks runs in the view", path)),
            ),
        };
        let in_view = fs::symlink_metadata(sandbox.view.join(path)).is_ok_and(|meta| meta.is_file());
        found.push(Settings { shown: format!("{} of HEAD", path), read, not_shown: (!in_view).then_some("is not in the view: the runner does not read it there"), working });
    }
    found
}

/// A command whose program the sandbox does not hold, with its events, in the order of the file.
struct Failing {
    shown: String,
    command: String,
    events: Vec<String>,
    why: String,
}

/// A command as it is said: cut at `SHOWN` characters. The whole line is escaped after (`issue::shown`).
fn shown(command: &str) -> String {
    match command.char_indices().nth(SHOWN) {
        Some((at, _)) => format!("{}…", &command[..at]),
        None => command.to_string(),
    }
}

/// What is said before a session in a view starts on the runner whose program is `program` (#160), one line each,
/// without the name of the command: each settings file not read, each that the runner does not read in the view and
/// that has hooks, each whose hooks are of a form `wb` does not read, and each command whose program the sandbox does
/// not hold. Nothing where all is held, or where `wb` does not know the runner's settings. It only reads.
pub fn said(program: &str, root: &str, sandbox: &Sandbox) -> Vec<String> {
    let mut lines = Vec::new();
    let mut failing: Vec<Failing> = Vec::new();
    for file in settings(program, root, sandbox) {
        if let Some(working) = &file.working {
            lines.push(working.clone());
        }
        let bytes = match file.read {
            None => continue,
            Some(Err(why)) => {
                lines.push(format!("{} was not read ({}), so its hooks are not looked at", file.shown, why));
                continue;
            }
            Some(Ok(bytes)) => bytes,
        };
        let document = match yamlfile::load_bytes(&file.shown, bytes) {
            Ok(document) => document,
            Err(error) => {
                let first = error.message().lines().next().unwrap_or_default();
                let why = first.strip_prefix(&file.shown).map(|rest| rest.trim_start_matches([',', ':', ' '])).unwrap_or(first);
                lines.push(format!("{} was not read ({}), so its hooks are not looked at", file.shown, why));
                continue;
            }
        };
        let Some(hooks) = hooks_of(document.root()) else {
            lines.push(format!("the hooks of {} are not in the form wb reads, so they are not looked at", file.shown));
            continue;
        };
        if let Some(not_shown) = file.not_shown {
            if !hooks.is_empty() {
                lines.push(format!("{} {}, and none of its {} hooks runs in the view", file.shown, not_shown, hooks.len()));
            }
            continue;
        }
        // Each command of this file once, by where it stands in `failing`.
        let mut known: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for hook in hooks {
            let Some(command) = hook.command else { continue };
            if let Some(&at) = known.get(&command) {
                let events = &mut failing[at].events;
                if !events.contains(&hook.event) {
                    events.push(hook.event);
                }
                continue;
            }
            if let Some(why) = unheld(&command, sandbox) {
                known.insert(command.clone(), failing.len());
                failing.push(Failing { shown: file.shown.clone(), command, events: vec![hook.event], why });
            }
        }
    }
    for one in failing.iter().take(NAMED) {
        lines.push(format!("the hook `{}` of {} ({}) {}: {}", shown(&one.command), one.shown, one.events.join(", "), FAILS, one.why));
    }
    if failing.len() > NAMED {
        lines.push(format!("and {} more hooks that {}", failing.len() - NAMED, FAILS));
    }
    // Every part that comes from a file (a path, an event, a command, the loader's words) is said escaped, so that a
    // line stays one line and no control character reaches the terminal.
    lines.iter().map(|line| crate::issue::shown(line)).collect()
}
