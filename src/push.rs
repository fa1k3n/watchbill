//! The push after a session (#151): once the commits of a session of the tester, the build or the fix of an issue
//! are taken back whole, `wb` pushes the issue's branch to `origin` with git, as the person and outside the sandbox,
//! and after a build with no change open it opens one as a draft by `draft_change`, with the description the session
//! wrote. The session holds no login to the code host. Where `wb` does not push, it says why and the commands that
//! would, as 0.0.9 did (#145).

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::{bounded, issue, prompt, refusal, starter, view};

/// The roles whose branch `wb` pushes after their session.
pub const PUSHED: [&str; 3] = ["tester", "build", "fix"];

/// The name of the description's file in the folder `wb` makes for a session (`view::DESCRIPTION` in the sandbox).
const DESCRIPTION_FILE: &str = "description";

/// The cap of the description: 64 KiB, well below the 128 KiB that Linux holds one word of a command to, as
/// `draft_change` of GitLab passes it to `glab` as one word.
pub const DESCRIPTION_CAP: u64 = 64 * 1024;

/// The paths of a login of the code host on this machine, which no runner's bind may hold or lie in (#151): the
/// folders of `gh` and `glab`, ssh's, and git's stored credentials, below `home` and below `XDG_CONFIG_HOME`, the
/// folders that `GH_CONFIG_DIR` and `GLAB_CONFIG_DIR` name, the keyrings below `XDG_RUNTIME_DIR` and the session bus
/// that leads to them, and the folder of the ssh agent's socket. `var` reads a variable of the environment.
fn logins(home: Option<&Path>, var: &dyn Fn(&str) -> Option<OsString>) -> Vec<PathBuf> {
    let absolute = |name: &str| var(name).map(PathBuf::from).filter(|path| path.is_absolute());
    let mut found = Vec::new();
    if let Some(home) = home {
        for below in [".config/gh", ".config/glab-cli", ".ssh", ".git-credentials", ".config/git/credentials", ".netrc"] {
            found.push(home.join(below));
        }
    }
    if let Some(config) = absolute("XDG_CONFIG_HOME") {
        for below in ["gh", "glab-cli", "git/credentials"] {
            found.push(config.join(below));
        }
    }
    for name in ["GH_CONFIG_DIR", "GLAB_CONFIG_DIR"] {
        found.extend(absolute(name));
    }
    if let Some(runtime) = absolute("XDG_RUNTIME_DIR") {
        for below in ["keyring", "gnupg", "gcr", "bus"] {
            found.push(runtime.join(below));
        }
    }
    if let Some(socket) = absolute("SSH_AUTH_SOCK") {
        found.push(socket.parent().map(Path::to_path_buf).unwrap_or(socket));
    }
    found
}

/// The login of the code host that a runner's bind `bind` holds or lies in, if any (`logins`), and the folder
/// `XDG_RUNTIME_DIR` where `bind` is it or holds it: the person's runtime folder holds the ssh agent's socket and the
/// keyrings wherever a desktop puts them. A folder below it that is none of those is let through. Each path is
/// taken as it is written and as its links lead.
pub fn login_of(bind: &Path, home: Option<&Path>, var: &dyn Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let forms = |path: &Path| -> Vec<String> {
        let mut found = vec![path.display().to_string()];
        if let Ok(real) = std::fs::canonicalize(path) {
            found.push(real.display().to_string());
        }
        found
    };
    let binds = forms(bind);
    let meets = |login: &Path| binds.iter().any(|one| forms(login).iter().any(|two| starter::meet(one, two)));
    if let Some(login) = logins(home, var).into_iter().find(|login| meets(login)) {
        return Some(login);
    }
    let runtime = var("XDG_RUNTIME_DIR").map(PathBuf::from).filter(|path| path.is_absolute())?;
    let holds = binds.iter().any(|one| forms(&runtime).iter().any(|two| starter::meet(one, two) && Path::new(two).starts_with(one)));
    holds.then_some(runtime)
}

/// Makes the folder of what a session writes for `wb`, new and empty, readable by the person alone:
/// `<state>/watchbill/out/<the view's name>`. It is bound at `view::OUT_FOLDER` in the sandbox, outside the view.
pub fn make_out(state: &Path, view: &Path) -> Result<PathBuf, String> {
    use std::os::unix::fs::DirBuilderExt;
    let below = state.join("watchbill").join("out");
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&below)
        .map_err(|error| format!("the folder {} could not be made: {}", below.display(), error))?;
    let name = view.file_name().ok_or_else(|| format!("the view {} has no name", view.display()))?;
    let out = below.join(name);
    std::fs::DirBuilder::new().mode(0o700).create(&out).map_err(|error| format!("the folder {} could not be made: {}", out.display(), error))?;
    Ok(out)
}

/// Removes the folder of what a session wrote for `wb`, with what is in it. A link in it is removed, never followed.
pub fn remove_out(out: &Path) {
    let _ = std::fs::remove_dir_all(out);
}

/// What the session wrote as the change's description: none where the file is not there or holds only white
/// space. A file that is not a plain file, is over the cap, is not UTF-8, or holds a character that no text `wb`
/// passes on holds (a control character but a line break or a tab, a line separator, a bidirectional control), is
/// refused: the error says why, and holds nothing of it.
pub fn read_description(out: &Path) -> Result<Option<String>, String> {
    let path = out.join(DESCRIPTION_FILE);
    let bytes = match bounded::read_plain(&path, DESCRIPTION_CAP) {
        Ok(bytes) => bytes,
        Err(bounded::Fault::NotFound) => return Ok(None),
        Err(fault) => return Err(format!("the description's file {} was not read: {}", view::DESCRIPTION, fault)),
    };
    let text = String::from_utf8(bytes).map_err(|_| format!("the description's file {} is not UTF-8 text", view::DESCRIPTION))?;
    if text.chars().any(|c| issue::is_unsafe(c, true)) {
        return Err(format!(
            "the description's file {} holds a control character, a line separator or a bidirectional control",
            view::DESCRIPTION
        ));
    }
    if text.trim().is_empty() {
        return Ok(None);
    }
    // `-` alone has `glab` open an editor, as the person.
    if text.trim() == "-" {
        return Err(format!("the description's file {} holds `-` alone, which a command that opens a change reads as no description", view::DESCRIPTION));
    }
    Ok(Some(text))
}

/// The description of a change whose session wrote none.
fn no_description(issue: &str) -> String {
    format!("Issue {}.\n\nThe build's session wrote no description of this change.\n", issue)
}

/// The end of a session, as the push sees it.
pub struct Ended<'a> {
    pub name: &'a str,
    pub role: &'a str,
    pub root: &'a str,
    pub issue: Option<&'a str>,
    /// The stamp's branch, which the commits went back onto.
    pub branch: &'a str,
    pub back: &'a view::Back,
    /// Whether the session ended with 0.
    pub ended_well: bool,
    pub no_push: bool,
    /// What `read_description` found, read before the folder was removed.
    pub description: Result<Option<String>, String>,
}

/// What came of the push: the status it adds (0 or 1), and the file `push` of the session's folder, where the
/// role is one `wb` pushes for.
pub struct Pushed {
    pub code: i32,
    pub lines: Option<String>,
}

/// After the commits of a session were taken back: pushes the branch and opens the change, or says why not and the
/// commands that would.
pub fn after(ended: &Ended, err: &mut dyn Write) -> Pushed {
    let issue = ended.issue.filter(|_| PUSHED.contains(&ended.role));
    let Some(issue) = issue else {
        if !ended.back.taken.is_empty() {
            let why = match ended.issue {
                None => "there is no issue, so there is no issue's branch to push".to_string(),
                Some(_) => format!("wb pushes after the tester, the build and the fix, and this is the {}", issue::shown(ended.role)),
            };
            let _ = writeln!(err, "{}: nothing is pushed: {}.", ended.name, why);
            say_commands(ended.name, ended.role, ended.root, ended.branch, err);
        }
        return Pushed { code: 0, lines: None };
    };
    let Some((commit, _)) = ended.back.taken.last() else {
        return Pushed { code: 0, lines: Some("not pushed: no commit of the session came back\n".to_string()) };
    };
    if let Some(why) = held_back(ended) {
        let _ = writeln!(err, "{}: nothing is pushed: {}.", ended.name, why);
        say_commands(ended.name, ended.role, ended.root, ended.branch, err);
        return Pushed { code: 0, lines: Some(format!("not pushed: {}\n", why)) };
    }
    let refspec = format!("{}:refs/heads/{}", commit, ended.branch);
    let _ = writeln!(
        err,
        "{}: will push {} to the branch {} at origin, as the person, outside the sandbox:\n  git push origin {}",
        ended.name, commit, ended.branch, refspec
    );
    if let Err(why) = push(ended.root, &refspec, err) {
        let why = format!("the branch {} was not pushed, and no change is opened: {}", ended.branch, why);
        let code = refusal::limit(err, ended.name, &why);
        return Pushed { code, lines: Some(format!("not pushed: {}\n", issue::shown(&why))) };
    }
    let mut lines = format!("pushed: {}\n", commit);
    if ended.role != "build" {
        return Pushed { code: 0, lines: Some(lines) };
    }
    let (code, change) = open_change(ended, issue, err);
    lines.push_str(&change);
    Pushed { code, lines: Some(lines) }
}

/// Why the branch is not pushed after a session of a role that `wb` pushes for, or none.
fn held_back(ended: &Ended) -> Option<String> {
    if ended.no_push {
        return Some("--no-push was given".to_string());
    }
    if ended.back.refused_any() {
        return Some("a commit of the session was refused at the taking back".to_string());
    }
    if !ended.back.all_taken() {
        return Some("work of the session was left uncommitted in the view, or not taken".to_string());
    }
    if !ended.ended_well {
        return Some("the session did not end with 0".to_string());
    }
    if !prompt::is_plain_name(ended.branch) {
        return Some(format!("the branch '{}' is not a plain name that fills the patterns", ended.branch.escape_debug()));
    }
    for key in ["change_of_branch", "draft_change"] {
        match prompt::pattern_of(ended.root, key) {
            Ok(Some(_)) => {}
            // The build is told the line; the tester and the fix open no change, and are told of no line that opens one.
            Ok(None) if ended.role == "build" => {
                return Some(format!("watchbill.yml has no `{}`, and wb pushes only where it has both `change_of_branch` and `draft_change`", key))
            }
            Ok(None) => return Some("wb pushes only where watchbill.yml has both lines of the change, and it lacks one (see The patterns in the README)".to_string()),
            Err(why) => return Some(why.to_string()),
        }
    }
    None
}

/// Runs `git push origin <refspec>` at the root of the checkout, as the person: with the person's settings and
/// login, and nothing on standard input. What git said goes to `err`, escaped. The error says how it ended.
fn push(root: &str, refspec: &str, err: &mut dyn Write) -> Result<(), String> {
    let output = Command::new("git")
        .args(["push", "origin", refspec])
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("git could not be run: {}", error))?;
    for said in [&output.stdout, &output.stderr] {
        for line in String::from_utf8_lossy(said).lines() {
            let _ = writeln!(err, "  {}", issue::shown(line));
        }
    }
    match output.status.code() {
        Some(0) => Ok(()),
        Some(code) => Err(format!("git push ended with {}", code)),
        None => Err("git push was ended by a signal".to_string()),
    }
}

/// After a build that was pushed: the change of the branch, opened as a draft where none is open. The status it
/// adds, and the line `change:` of the file `push`.
fn open_change(ended: &Ended, issue: &str, err: &mut dyn Write) -> (i32, String) {
    let not_opened = |err: &mut dyn Write, why: String| {
        let code = refusal::limit(err, ended.name, format_args!("the change is not opened: {}", why));
        (code, format!("change: not opened: {}\n", issue::shown(&why)))
    };
    let pattern = |key: &str| match prompt::pattern_of(ended.root, key) {
        Ok(Some(line)) => Ok(line.replace("{branch}", ended.branch)),
        Ok(None) => Err(format!("watchbill.yml has no `{}`", key)),
        Err(why) => Err(why.to_string()),
    };
    let open = match pattern("change_of_branch").and_then(|line| starter::open_changes(ended.root, &line)) {
        Ok(open) => open,
        Err(why) => return not_opened(err, why),
    };
    match open.as_slice() {
        [] => {}
        [one] => {
            let _ = writeln!(err, "{}: the change {} is open from the branch {}, so none is opened.", ended.name, issue::shown(one), ended.branch);
            return (0, format!("change: {}\n", issue::shown(one)));
        }
        more => {
            let named: Vec<String> = more.iter().map(|one| issue::shown(one)).collect();
            let why = format!("more than one change is open from the branch {}: {}", ended.branch, named.join(", "));
            let _ = writeln!(err, "{}: {}, so none is opened.", ended.name, why);
            return (0, format!("change: not opened: {}\n", why));
        }
    }
    let description = match &ended.description {
        Ok(Some(text)) => text.clone(),
        Ok(None) => {
            let _ = writeln!(err, "{}: the session wrote no description of the change in {}.", ended.name, view::DESCRIPTION);
            no_description(issue)
        }
        Err(why) => return not_opened(err, why.clone()),
    };
    let line = match pattern("draft_change") {
        Ok(line) => line,
        Err(why) => return not_opened(err, why),
    };
    let _ = write!(
        err,
        "{}: will open the change of {} as a draft, by `draft_change` of watchbill.yml:\n  {}\nwith this description on standard input:\n{}",
        ended.name, ended.branch, line, description
    );
    if !description.ends_with('\n') {
        let _ = writeln!(err);
    }
    match starter::run(ended.root, "draft_change", &line, Some(description.as_bytes())) {
        Ok(said) => {
            let _ = err.write_all(&said);
        }
        Err(why) => return not_opened(err, why),
    }
    match pattern("change_of_branch").and_then(|line| starter::open_changes(ended.root, &line)) {
        Ok(open) if open.len() == 1 => {
            let _ = writeln!(err, "{}: the change {} is opened as a draft.", ended.name, issue::shown(&open[0]));
            (0, format!("change: {}\n", issue::shown(&open[0])))
        }
        Ok(open) => {
            let why = format!("it was opened, and `change_of_branch` printed {} changes for the branch after it", open.len());
            let _ = writeln!(err, "{}: {}.", ended.name, why);
            (0, format!("change: not read: {}\n", why))
        }
        Err(why) => {
            let code = refusal::limit(err, ended.name, format_args!("the change was opened, and its number could not be read: {}", why));
            (code, format!("change: not read: {}\n", issue::shown(&why)))
        }
    }
}

/// Says the command that pushes `branch` and, after a build, the command that opens the change as a draft: the
/// pattern `draft_change` of `watchbill.yml`, filled. `wb` runs neither here (#145): a person does.
pub fn say_commands(name: &str, role: &str, root: &str, branch: &str, err: &mut dyn Write) {
    let _ = writeln!(err, "{}: to push the branch, run:\n  git push origin {}", name, branch);
    if role != "build" {
        return;
    }
    match prompt::pattern_of(root, "draft_change") {
        Ok(Some(line)) if prompt::is_plain_name(branch) => {
            let _ = writeln!(err, "{}: to open the change as a draft, where none is open yet, run:\n  {}", name, line.replace("{branch}", branch));
        }
        Ok(Some(_)) => {
            let _ = writeln!(err, "{}: the branch '{}' is not a plain name that fills `draft_change`: open the change as a draft at the git server.", name, branch.escape_debug());
        }
        Ok(None) => {
            let _ = writeln!(err, "{}: watchbill.yml has no `draft_change`, so the command that opens the change is not known: open it as a draft at the git server, where none is open yet.", name);
        }
        Err(why) => {
            let _ = writeln!(err, "{}: {}", name, why);
        }
    }
}
