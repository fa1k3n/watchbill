//! `wb doctor` (#178): what this machine needs to run Watchbill for this repository, one line to a check, before a
//! session fails on it. It reads only and asks nothing of the network.
//!
//! A line is `ok: <check>: <what holds>`, `warning: <check>: <what to know>` or `fault: <check>: <what is wrong and
//! what to do>`. Only a fault ends the command with 1. The checks reuse what `wb start` does: the sandbox's own check
//! (`view::works`), the organisation's file, the runner's binds and the login of the code host (`push::login_of`),
//! the hooks of the runner (`hooks::said`, #160) and the command `needs` of `watchbill.yml`. What is new is the
//! tools the patterns call, and the ports that listen on this machine.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};

use crate::confidentiality::{self, Organisation, Runner};
use crate::{bounded, hooks, issue, push, starter, view, yamlfile};

/// What `wb doctor` looks at on this machine. `run` gives what is fixed in `wb` and the environment; a test gives
/// its own.
pub struct Host<'a> {
    /// The organisation's file.
    pub organisation: &'a Path,
    /// bubblewrap, which a session in a view runs in.
    pub sandbox: &'a Path,
    /// The home folder that `~/` of a runner's `binds` names, or none.
    pub home: Option<&'a Path>,
    /// `PATH`, as a session inherits it.
    pub path: Option<&'a OsStr>,
    /// The folder of the listings of the sockets, `/proc/net`, with `tcp` and `tcp6` in it.
    pub net: &'a Path,
}

/// The folder of the listings of the sockets.
pub const NET: &str = "/proc/net";

/// The patterns of `watchbill.yml` that are commands: the keys whose lines are looked at for the tools they call.
pub const PATTERNS: [&str; 14] = [
    "issue_comment",
    "issue_comments",
    "change_comment",
    "change_comments",
    "change_of_branch",
    "change_checks",
    "issue_comment_bodies",
    "change_comment_bodies",
    "change_text",
    "draft_change",
    "issue_text",
    "issue_address",
    "change_address",
    "needs",
];

/// How many ports are named at most; the rest are counted.
pub const NAMED: usize = 20;

/// The most a listing of the sockets is read.
pub const CAP: u64 = bounded::TEXT_CAP;

/// The state of a socket that listens, in `/proc/net/tcp`.
const LISTEN: &str = "0A";

/// What a check found.
enum Kind {
    Ok,
    Warning,
    Fault,
}

/// The lines of the report, in order, and whether one is a fault.
struct Report {
    lines: Vec<String>,
    faults: usize,
}

impl Report {
    fn say(&mut self, kind: Kind, check: &str, text: &str) {
        let word = match kind {
            Kind::Ok => "ok",
            Kind::Warning => "warning",
            Kind::Fault => {
                self.faults += 1;
                "fault"
            }
        };
        // What comes from a file or a pattern is said escaped, so that a line stays one line.
        self.lines.push(issue::shown(&format!("{}: {}: {}", word, check, text.split_whitespace().collect::<Vec<_>>().join(" "))));
    }

    fn ok(&mut self, check: &str, text: &str) {
        self.say(Kind::Ok, check, text);
    }

    fn warning(&mut self, check: &str, text: &str) {
        self.say(Kind::Warning, check, text);
    }

    fn fault(&mut self, check: &str, text: &str) {
        self.say(Kind::Fault, check, text);
    }
}

/// An executable file called `name` in a folder of `path`, which holds only absolute folders.
fn on_path(name: &str, path: Option<&OsStr>) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    std::env::split_paths(path.unwrap_or_default())
        .filter(|folder| folder.is_absolute())
        .map(|folder| folder.join(name))
        .find(|candidate| std::fs::metadata(candidate).is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0))
}

/// `wb doctor`, in the repository at `root`, on the machine `host` says. Gives the status.
pub fn doctor(root: &str, host: &Host, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let (_guard, differs) = match starter::read_only(root) {
        Ok(read) => read,
        Err(not) => {
            if not.status == 1 {
                return crate::refusal::limit(err, "wb doctor", not.why);
            }
            let _ = writeln!(err, "wb doctor: {}.", not.why.trim_end_matches('.'));
            return not.status;
        }
    };
    if differs {
        let _ = writeln!(err, "{}", starter::drift_line("wb doctor"));
    }
    let mut report = Report { lines: Vec::new(), faults: 0 };
    sandbox(&mut report, host);
    git(&mut report, host);
    tools(&mut report, root, host);
    let organisation = organisation(&mut report, host);
    let runner = runner(&mut report, &organisation);
    let binds = binds(&mut report, host, runner);
    hooks_of(&mut report, root, host, runner, &binds);
    needs(&mut report, root);
    ports(&mut report, host);
    for line in &report.lines {
        let _ = writeln!(out, "{}", line);
    }
    if report.faults > 0 {
        let _ = writeln!(err, "wb doctor: {} {} wrong.", report.faults, if report.faults == 1 { "check is" } else { "checks are" });
        crate::refusal::answer();
        return 1;
    }
    0
}

/// Bubblewrap is there, and can make a sandbox: the check that `wb start` makes (`view::works`).
fn sandbox(report: &mut Report, host: &Host) {
    if !host.sandbox.is_file() {
        let why = view::works(host.sandbox).err().unwrap_or_default();
        report.fault("bubblewrap", &why);
        report.warning("user namespaces", "not looked at: there is no bubblewrap");
        return;
    }
    report.ok("bubblewrap", &format!("{} is there", host.sandbox.display()));
    match view::works(host.sandbox) {
        Ok(()) => report.ok("user namespaces", "bubblewrap makes a sandbox here"),
        Err(why) => report.fault("user namespaces", &why),
    }
}

/// `git` is on `PATH`.
fn git(report: &mut Report, host: &Host) {
    match on_path("git", host.path) {
        Some(found) => report.ok("git", &format!("{}", found.display())),
        None => report.fault("git", "there is no git on PATH, and wb reads and writes the repository through it. Install the package git"),
    }
}

/// The program of each command of `line`, as the shell takes the first word of each: after `|`, `&`, `;`, a line break
/// or a parenthesis, outside quotes. A word that holds a variable, a quote, a substitution or a glob is passed over,
/// as is anything that is not a plain name or a path.
pub fn programs_of(line: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    // Words, each with whether it started a command, and whether it is plain.
    let mut words: Vec<(String, bool, bool)> = Vec::new();
    let (mut word, mut plain, mut open) = (String::new(), true, false);
    let mut start = true;
    let mut quote: Option<char> = None;
    let mut chars = line.chars().peekable();
    let flush = |word: &mut String, plain: &mut bool, open: &mut bool, start: &mut bool, words: &mut Vec<(String, bool, bool)>| {
        if *open {
            words.push((std::mem::take(word), *start, *plain));
            *start = false;
        }
        *plain = true;
        *open = false;
    };
    while let Some(c) = chars.next() {
        match quote {
            Some(q) => {
                if c == q {
                    quote = None;
                } else if q == '"' && c == '\\' {
                    chars.next();
                }
            }
            None => match c {
                '\'' | '"' => {
                    quote = Some(c);
                    plain = false;
                    open = true;
                }
                ' ' | '\t' => flush(&mut word, &mut plain, &mut open, &mut start, &mut words),
                '|' | '&' | ';' | '\n' | '(' | ')' => {
                    flush(&mut word, &mut plain, &mut open, &mut start, &mut words);
                    start = true;
                }
                '<' | '>' => {
                    flush(&mut word, &mut plain, &mut open, &mut start, &mut words);
                    // The word after a redirection is a file.
                    chars.next_if(|next| *next == c || *next == '&');
                    while chars.next_if(|next| *next == ' ').is_some() {}
                    while chars.next_if(|next| !matches!(next, ' ' | '\t' | '|' | '&' | ';' | '\n' | '(' | ')')).is_some() {}
                }
                '$' | '`' | '\\' | '*' | '?' | '[' | '{' | '}' | '~' | '!' | '#' => {
                    plain = false;
                    open = true;
                    word.push(c);
                }
                _ => {
                    open = true;
                    word.push(c);
                }
            },
        }
    }
    flush(&mut word, &mut plain, &mut open, &mut start, &mut words);
    // The words that start a command; an assignment and a keyword leave the next word at the start.
    let mut at_start = false;
    for (word, starts, plain) in words {
        if starts {
            at_start = true;
        }
        if !at_start {
            continue;
        }
        if plain && word.contains('=') && word.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') {
            continue;
        }
        if plain && matches!(word.as_str(), "for" | "select" | "case") {
            // The words up to the next command are a variable and a list, not programs.
            at_start = false;
            continue;
        }
        if plain && hooks::RUNS_ANOTHER.contains(&word.as_str()) {
            continue;
        }
        // An option of `command`, `exec` or `time` is not the program.
        if plain && word.len() > 1 && word.starts_with('-') {
            continue;
        }
        at_start = false;
        if !plain || hooks::BUILTINS.contains(&word.as_str()) || word.is_empty() {
            continue;
        }
        let name = word.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-' | '/'));
        if name && !found.contains(&word) {
            found.push(word);
        }
    }
    found
}

/// The tools the patterns of `watchbill.yml` call, each on `PATH`, one line each, naming the keys that call it.
fn tools(report: &mut Report, root: &str, host: &Host) {
    let settings = match starter::settings(root) {
        Ok(settings) => settings,
        Err(why) => {
            report.fault("tools", &format!("watchbill.yml was not read: {}", why.lines().next().unwrap_or_default()));
            return;
        }
    };
    let mut calls: Vec<(String, Vec<&str>)> = Vec::new();
    for key in PATTERNS {
        let place = [yamlfile::Part::from(key)];
        if !settings.has(&place) {
            continue;
        }
        let Ok(line) = settings.text(&place) else { continue };
        for program in programs_of(&line) {
            match calls.iter_mut().find(|(name, _)| *name == program) {
                Some((_, keys)) => keys.push(key),
                None => calls.push((program, vec![key])),
            }
        }
    }
    if calls.is_empty() {
        report.ok("tools", "no pattern of watchbill.yml calls a tool");
        return;
    }
    for (program, keys) in calls {
        let named = keys.join(", ");
        let found = if program.contains('/') {
            let path = if program.starts_with('/') { PathBuf::from(&program) } else { Path::new(root).join(&program) };
            path.is_file().then_some(path)
        } else {
            on_path(&program, host.path)
        };
        match found {
            Some(path) => report.ok("tools", &format!("{} at {}, called by {}", program, path.display(), named)),
            None => {
                let where_ = if program.contains('/') { "is not there" } else { "is on no folder of PATH" };
                report.fault("tools", &format!("{}, called by {}, {}. Install it, or change the pattern", program, named, where_));
            }
        }
    }
}

/// What the check of the organisation's file found: its text, or why it is not held.
fn organisation(report: &mut Report, host: &Host) -> Result<Organisation, String> {
    match confidentiality::read(host.organisation) {
        Ok(organisation) => {
            report.ok("organisation", &format!("{} is there and reads", host.organisation.display()));
            Ok(organisation)
        }
        Err(why) => {
            let one = why.split_whitespace().collect::<Vec<_>>().join(" ");
            report.fault("organisation", &format!("{}. The organisation's own means lay it out", one.trim_end_matches('.')));
            Err(one)
        }
    }
}

/// The default runner of the organisation's file, said; none where there is none.
fn runner<'a>(report: &mut Report, organisation: &'a Result<Organisation, String>) -> Option<&'a Runner> {
    let Ok(organisation) = organisation else {
        report.warning("default runner", "not looked at: the organisation's file does not hold");
        return None;
    };
    let Some(name) = &organisation.default else {
        report.warning(
            "default runner",
            "the organisation's file has no default, so wb start needs --runner <name>; the program, the binds and the hooks of a runner are not looked at",
        );
        return None;
    };
    organisation.runners.iter().find(|runner| &runner.name == name)
}

/// The runner's `binds`, each from the root, as `wb start` resolves them; a bind in the home folder with no home is
/// said, and none is made of it.
fn resolve(runner: &Runner, host: &Host, report: &mut Report) -> Vec<(PathBuf, bool)> {
    let mut resolved = Vec::new();
    for bind in &runner.binds {
        match (bind.path.strip_prefix("~/"), host.home) {
            (None, _) => resolved.push((PathBuf::from(&bind.path), bind.writable)),
            (Some(rest), Some(home)) => resolved.push((home.join(rest), bind.writable)),
            (Some(_), None) => report.fault(
                "binds",
                &format!("the bind {} is in the home folder, and HOME names no absolute folder. Set HOME, or take it out of `binds`", bind.path),
            ),
        }
    }
    resolved
}

/// The runner's `binds` and its program; the binds are given back for the hooks.
fn binds(report: &mut Report, host: &Host, runner: Option<&Runner>) -> Vec<(PathBuf, bool)> {
    let Some(runner) = runner else {
        report.warning("binds", "not looked at: there is no default runner");
        return Vec::new();
    };
    let resolved = resolve(runner, host, report);
    match &runner.start {
        None => report.warning(
            "default runner",
            &format!("the runner {} has no `start`, so wb start starts nothing on it; its program is not looked at", runner.name),
        ),
        Some(start) => {
            let program = start.first().map(String::as_str).unwrap_or_default();
            if let Some(why) = crate::outside_the_sandbox(program, &resolved) {
                report.fault("default runner", &format!("{}: {}", runner.name, why));
            } else if program.starts_with('/') {
                if Path::new(program).is_file() {
                    report.ok("default runner", &format!("{} starts {}", runner.name, program));
                } else {
                    report.fault("default runner", &format!("the program {} of the runner {} is not there", program, runner.name));
                }
            } else {
                match on_path(program, host.path) {
                    Some(found) => report.ok("default runner", &format!("{} starts {} ({})", runner.name, program, found.display())),
                    None => report.fault(
                        "default runner",
                        &format!("the program {} of the runner {} is on no folder of PATH. Install it, or change `start` in the organisation's file", program, runner.name),
                    ),
                }
            }
        }
    }
    if resolved.is_empty() && runner.binds.is_empty() {
        report.ok("binds", &format!("the runner {} has none", runner.name));
    }
    for (path, _) in &resolved {
        if let Some(login) = push::login_of(path, host.home, &|name| std::env::var_os(name)) {
            report.fault(
                "binds",
                &format!(
                    "the bind {} holds or lies in {}, a login of the code host, and no session holds one: wb pushes outside the sandbox. Take it out of `binds` in the organisation's file",
                    path.display(),
                    login.display()
                ),
            );
        } else if std::fs::metadata(path).is_err() {
            report.fault("binds", &format!("the bind {} is not there. Make it, or take it out of `binds` in the organisation's file", path.display()));
        } else {
            report.ok("binds", &format!("{} is there", path.display()));
        }
    }
    resolved
}

/// The hooks of the runner's settings whose program the sandbox does not hold (#160), with the checkout standing in
/// for the view.
fn hooks_of(report: &mut Report, root: &str, host: &Host, runner: Option<&Runner>, binds: &[(PathBuf, bool)]) {
    let Some(runner) = runner else {
        report.warning("hooks", "not looked at: there is no default runner");
        return;
    };
    let Some(program) = runner.start.as_ref().and_then(|start| start.first()) else {
        report.warning("hooks", "not looked at: the runner has no `start`, and its program is not known");
        return;
    };
    let fixed = Path::new(confidentiality::ORGANISATION_FILE).parent().filter(|folder| folder.is_dir());
    let given = host.organisation.parent().filter(|folder| folder.is_dir());
    let covered: Vec<PathBuf> = [fixed, given].into_iter().flatten().map(Path::to_path_buf).collect();
    let sandbox = hooks::Sandbox { checkout: Path::new(root), view: Path::new(root), home: host.home, binds, covered: &covered, path: host.path };
    let lines = hooks::said(program, root, &sandbox);
    if lines.is_empty() {
        report.ok("hooks", "no hook of the runner's settings names a program the sandbox does not hold");
    }
    for line in lines {
        if line.contains(hooks::FAILS) || line.contains("more hooks that") {
            report.fault("hooks", &line);
        } else {
            report.warning("hooks", &line);
        }
    }
}

/// The command `needs` of `watchbill.yml`, run once, for the file of `watchbill.yml` itself.
fn needs(report: &mut Report, root: &str) {
    let settings = match starter::settings(root) {
        Ok(settings) => settings,
        Err(why) => {
            report.fault("needs", &format!("watchbill.yml was not read: {}", why.lines().next().unwrap_or_default()));
            return;
        }
    };
    let place = [yamlfile::Part::from("needs")];
    if !settings.has(&place) {
        report.ok("needs", "watchbill.yml has no `needs`, so there is no command to run");
        return;
    }
    let pattern = match settings.text(&place) {
        Ok(pattern) => pattern,
        Err(why) => {
            report.fault("needs", &why.to_string());
            return;
        }
    };
    let file = format!("{}/{}", crate::init::CONFIG_FOLDER, crate::init::CONFIG_FILE);
    let line = pattern.replace("{file}", &format!("'{}'", file.replace('\'', "'\\''")));
    match starter::run(root, "needs", &line, None) {
        Ok(_) => report.ok("needs", &format!("`needs` of watchbill.yml was run once, for {}, and ended with 0", file)),
        Err(why) => report.fault("needs", &format!("{}, run for {}. Mend the command, or take `needs` out of watchbill.yml", why, file)),
    }
}

/// One address and port of a socket that listens.
type Socket = (u16, String, &'static str);

/// The sockets that listen in a listing of `/proc/net/tcp` (`six` false) or `tcp6`: the port, the address as it is
/// written, and where it listens.
fn listening(text: &str, six: bool) -> Vec<Socket> {
    let mut found = Vec::new();
    for line in text.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.get(3) != Some(&LISTEN) {
            continue;
        }
        let Some((address, port)) = fields.get(1).and_then(|local| local.split_once(':')) else { continue };
        let Ok(port) = u16::from_str_radix(port, 16) else { continue };
        let group = |at: usize| u32::from_str_radix(address.get(at * 8..at * 8 + 8)?, 16).ok().map(u32::to_le_bytes);
        let (shown, loopback, any) = if six && address.len() == 32 {
            let mut bytes = [0u8; 16];
            for at in 0..4 {
                let Some(group) = group(at) else { return found };
                bytes[at * 4..at * 4 + 4].copy_from_slice(&group);
            }
            let ip = Ipv6Addr::from(bytes);
            match ip.to_ipv4_mapped() {
                Some(mapped) => (format!("{}:{}", mapped, port), mapped.is_loopback(), mapped.is_unspecified()),
                None => (format!("[{}]:{}", ip, port), ip.is_loopback(), ip.is_unspecified()),
            }
        } else if !six && address.len() == 8 {
            let Some(bytes) = group(0) else { continue };
            let ip = Ipv4Addr::from(bytes);
            (format!("{}:{}", ip, port), ip.is_loopback(), ip.is_unspecified())
        } else {
            continue;
        };
        let at = if loopback {
            "loopback"
        } else if any {
            "every address of this machine"
        } else {
            "an address of this machine"
        };
        found.push((port, shown, at));
    }
    found
}

/// The ports that listen on this machine, from `/proc/net/tcp` and `tcp6`, read through a cap. A session in a view
/// shares the machine's network, and `wb` cannot tell whether a port has a login, so it names every one. It warns and
/// refuses nothing.
fn ports(report: &mut Report, host: &Host) {
    let mut sockets: BTreeSet<Socket> = BTreeSet::new();
    for (name, six) in [("tcp", false), ("tcp6", true)] {
        let path = host.net.join(name);
        let read = bounded::open_plain(&path).and_then(|(file, _)| {
            let mut bytes = Vec::new();
            file.take(CAP + 1).read_to_end(&mut bytes).map_err(bounded::Fault::from)?;
            Ok(bytes)
        });
        let mut bytes = match read {
            Ok(bytes) => bytes,
            Err(why) => {
                report.warning("ports", &format!("{} was not read ({}), so its ports are not named", path.display(), why));
                continue;
            }
        };
        if bytes.len() as u64 > CAP {
            bytes.truncate(CAP as usize);
            // The last line may be cut short: it is dropped.
            if let Some(end) = bytes.iter().rposition(|byte| *byte == b'\n') {
                bytes.truncate(end + 1);
            }
            report.warning("ports", &format!("{} is over 1 MiB, and no more of it is read", path.display()));
        }
        sockets.extend(listening(&String::from_utf8_lossy(&bytes), six));
    }
    if sockets.is_empty() {
        report.ok("ports", "no port is listening on this machine");
        return;
    }
    for (_, shown, at) in sockets.iter().take(NAMED) {
        report.warning(
            "ports",
            &format!(
                "{} listens on {}; wb cannot tell whether it has a login, and a session in a view can reach it (a view shares this machine's network, until #195)",
                shown, at
            ),
        );
    }
    if sockets.len() > NAMED {
        report.warning("ports", &format!("and {} more ports are listening", sockets.len() - NAMED));
    }
}
