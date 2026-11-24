//! A view kept with work not taken back, and a session cut off (#222). A start whose `wb` was ended before its
//! session leaves a folder of its lines (`sessions`) whose `outcome` still says `running`, and a view with what the
//! session committed in it. `wb start` refuses where such a view of its issue is kept, and `wb status` names it.
//!
//! What is read is the state folder of this machine: the folders of the lines of the starts, and the views. A view is
//! hostile: a session may have written anything into it. So nothing of it is read except through `view::work_left`,
//! which reads its objects through a repository of `wb`'s own and runs no hook and no setting of the view; the files
//! of the lines are small plain files that `wb` wrote, read with a bound, and a path in them is held to a folder
//! directly below the folder of the views.

use std::fs;
use std::path::{Path, PathBuf};

use crate::view::{self, View};

/// The most folders of the lines that are looked at.
const MOST_FOLDERS: usize = 20_000;
/// How many sessions cut off `wb status` names.
pub const MOST_CUT_OFF: usize = 5;

/// A small plain file, read with a bound (`view::small_file`: opened first, the open file looked at, read through a cap).
fn small(path: &Path) -> Option<String> {
    view::small_file(path)
}

/// The value of the line `<name>: <value>` of a text, the first one.
fn value<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.lines().find_map(|line| line.strip_prefix(name).and_then(|rest| rest.strip_prefix(": ")))
}

/// The start time of a process, in clock ticks since the machine started, as `/proc/<pid>/stat` has it (field 22).
pub fn start_time(pid: u32) -> Option<u64> {
    let stat = fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
    // The name of the program may hold spaces and brackets: the fields start after the last bracket.
    let after = &stat[stat.rfind(')')? + 1..];
    after.split_whitespace().nth(19)?.parse().ok()
}

/// Whether the process is a `wb`, by the program it runs, and, where a start time is given, the one that started
/// then: process IDs are reused.
fn is_wb(pid: u32, started: Option<u64>) -> bool {
    if pid == 0 {
        return false;
    }
    let name = match fs::read_link(format!("/proc/{}/exe", pid)) {
        Ok(path) => path.file_name().map(|name| name.to_string_lossy().trim_end_matches(" (deleted)").to_string()),
        Err(_) => fs::read_to_string(format!("/proc/{}/comm", pid)).ok().map(|name| name.trim().to_string()),
    };
    name.as_deref() == Some("wb") && started.map_or(true, |ticks| start_time(pid) == Some(ticks))
}

/// What a start wrote in `folder`, the file naming its view: the view's folder, the commit it was made with, the
/// commit of the checkout it was made from, the branch, and the issue.
pub fn folder_text(view: &View, issue: Option<&str>) -> Option<String> {
    let folder = view.folder.to_str()?;
    Some(format!("folder: {}\nbase: {}\nhead: {}\nbranch: {}\nissue: {}\n", folder, view.base, view.head, view.branch, issue?))
}

/// The folder of the lines of one start, as it is read.
struct Session {
    folder: PathBuf,
    role: String,
    /// The round of its stamp, or none.
    round: Option<String>,
    /// What stands after the role in its name: the issue, and `-<n>` where the folder was made again.
    rest: String,
    /// The view it names, where it names one.
    place: Option<Place>,
}

/// The view that `folder` names.
struct Place {
    view: View,
    issue: String,
}

impl Session {
    /// Whether the start was for `issue`.
    fn is_for(&self, issue: &str) -> bool {
        if let Some(place) = &self.place {
            return place.issue == issue;
        }
        self.rest == issue
            || self.rest.strip_prefix(issue).and_then(|more| more.strip_prefix('-')).is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    }

    fn round(&self) -> String {
        self.round.clone().unwrap_or_else(|| "?".to_string())
    }

    /// Whether `outcome` still says `running`: `Some(true)` where the `wb` that wrote it is alive (the session runs),
    /// `Some(false)` where it is not (the session was cut off), none where it says anything else.
    fn running(&self) -> Option<bool> {
        let outcome = small(&self.folder.join("outcome"))?;
        if outcome.lines().next() != Some("outcome: running") {
            return None;
        }
        let Some(pid) = value(&outcome, "pid").and_then(|pid| pid.parse::<u32>().ok()) else { return Some(false) };
        let started = value(&outcome, "started").and_then(|ticks| ticks.parse::<u64>().ok());
        Some(is_wb(pid, started))
    }

    fn cut_off(&self) -> bool {
        self.running() == Some(false)
    }
}

/// The folder of the views below the state folder.
fn views_of(state: &Path) -> PathBuf {
    state.join("watchbill").join("views")
}

/// A view's folder as the lines name it: directly below the folder of the views, and a folder that is not a link.
fn held_folder(state: &Path, folder: &str) -> Option<PathBuf> {
    let path = PathBuf::from(folder);
    let views = views_of(state);
    let plain = path.parent() == Some(views.as_path()) && path.file_name().is_some_and(|name| name != "." && name != "..");
    (plain && fs::symlink_metadata(&path).ok()?.is_dir()).then_some(path)
}

fn place_of(state: &Path, text: &str) -> Option<Place> {
    let folder = held_folder(state, value(text, "folder")?)?;
    let (base, head, branch, issue) = (value(text, "base")?, value(text, "head")?, value(text, "branch")?, value(text, "issue")?);
    if !view::is_object(base) || !view::is_object(head) || !branch.starts_with("refs/heads/") {
        return None;
    }
    Some(Place { view: View { folder, base: base.to_string(), head: head.to_string(), branch: branch.to_string() }, issue: issue.to_string() })
}

/// The folders of the lines of the starts below the state folder, newest first, of `issue` where one is given.
fn sessions(state: &Path, issue: Option<&str>) -> Vec<Session> {
    let below = state.join("watchbill").join("sessions");
    match fs::symlink_metadata(&below) {
        Ok(found) if found.is_dir() => {}
        _ => return Vec::new(),
    }
    let Ok(entries) = fs::read_dir(&below) else { return Vec::new() };
    let mut names: Vec<String> = entries.filter_map(|entry| entry.ok()).filter_map(|entry| entry.file_name().into_string().ok()).take(MOST_FOLDERS).collect();
    names.sort();
    names.reverse();
    let mut found = Vec::new();
    for name in names {
        // `<time>-<role>-<issue>`: the time is written `2026-11-22T100000Z`, so it ends at the first `Z-`.
        let Some((_time, after)) = name.split_once("Z-") else { continue };
        let Some((role, rest)) = after.split_once('-') else { continue };
        let folder = below.join(&name);
        if !fs::symlink_metadata(&folder).is_ok_and(|found| found.is_dir()) {
            continue;
        }
        let stamp = small(&folder.join("stamp")).unwrap_or_default();
        let first = stamp.lines().next().unwrap_or_default();
        let round = first.strip_prefix(&format!("[stamp] {} round ", role)).filter(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())).map(str::to_string);
        let place = small(&folder.join("folder")).and_then(|text| place_of(state, &text));
        let session = Session { folder, role: role.to_string(), round, rest: rest.to_string(), place };
        if issue.map_or(true, |issue| session.is_for(issue)) {
            found.push(session);
        }
    }
    found
}

/// A view kept with work not taken back.
#[derive(Debug)]
pub struct Kept {
    pub role: String,
    pub round: String,
    pub folder: PathBuf,
    /// The commits not taken: each its name and its subject (and, for a commit refused, why).
    pub commits: Vec<(String, String)>,
    /// How many more commits there are than are named (#265).
    pub more: usize,
    /// The paths left uncommitted, each written escaped.
    pub uncommitted: Vec<String>,
    /// Why the view could not be read, where it could not: what it holds is then not known.
    pub unread: Option<String>,
}

/// The lines of `back` that a take-back left last: those after the last line `take-back`, or all of it.
fn section(back: &str) -> String {
    let (mut start, mut at) = (0, 0);
    for line in back.split_inclusive('\n') {
        at += line.len();
        if line.trim_end_matches('\n') == "take-back" {
            start = at;
        }
    }
    back[start..].to_string()
}

/// The commits of the view that the take-backs of a start put on the branch (#277): the first word of each line `took:`
/// of its file `back`, where it is the name of an object.
pub fn taken_of(folder: &Path) -> Vec<String> {
    let Some(back) = small(&folder.join("back")) else { return Vec::new() };
    back.lines().filter_map(|line| line.strip_prefix("took: ")?.split(' ').next()).filter(|commit| view::is_object(commit)).map(str::to_string).collect()
}

/// The view that `session` kept, where it kept one with work in it.
///
/// A session that runs, with its `wb` alive, is skipped: its view is the session's own, and is not read. With `look`
/// false no git is run on any view: a view that git would have to read is counted, as `not looked at`.
fn kept_of(root: &str, state: &Path, session: &Session, look: bool) -> Option<Kept> {
    if session.running() == Some(true) {
        return None;
    }
    let made = |folder: PathBuf, commits, uncommitted, unread| Kept { role: session.role.clone(), round: session.round(), folder, commits, more: 0, uncommitted, unread };
    let back = small(&session.folder.join("back"));
    // A take-back appends its lines after a line `take-back` (#223): only they say what is left.
    let back = back.as_deref().map(section);
    let back = back.as_deref();
    // A start that ended with its view kept says so in `back`, and what was not taken.
    if let Some(back) = back.filter(|back| !back.lines().any(|line| line.starts_with("failed: "))) {
        let folder = held_folder(state, back.lines().last()?.strip_prefix("view: kept ")?)?;
        let mut commits = Vec::new();
        let mut uncommitted = Vec::new();
        for line in back.lines() {
            if let Some(rest) = line.strip_prefix("refused: ").or_else(|| line.strip_prefix("not taken: ")) {
                let (commit, subject) = rest.split_once(' ').unwrap_or((rest, ""));
                commits.push((view::short(commit).to_string(), subject.to_string()));
            } else if let Some(path) = line.strip_prefix("uncommitted: ") {
                uncommitted.push(path.to_string());
            }
        }
        return (!commits.is_empty() || !uncommitted.is_empty()).then(|| made(folder, commits, uncommitted, None));
    }
    // No `back`, or one that failed: the view is read, as the file `folder` names it.
    let folder_of_back = back.and_then(|back| back.lines().last()?.strip_prefix("view: kept ")).and_then(|folder| held_folder(state, folder));
    let not_looked_at = |folder: PathBuf| made(folder, Vec::new(), Vec::new(), Some("not looked at".to_string()));
    if !look {
        return session.place.as_ref().map(|place| place.view.folder.clone()).or(folder_of_back).map(not_looked_at);
    }
    let Some(place) = &session.place else {
        return folder_of_back.map(|folder| made(folder, Vec::new(), Vec::new(), Some("it was not read: its session left no file `folder`".to_string())));
    };
    match view::work_left(root, &place.view, &taken_of(&session.folder)) {
        Ok(left) => {
            let commits: Vec<(String, String)> = left.commits.into_iter().map(|(commit, subject)| (view::short(&commit).to_string(), subject)).collect();
            let uncommitted: Vec<String> = left.uncommitted.iter().map(|path| crate::issue::shown(path)).collect();
            (!commits.is_empty() || !uncommitted.is_empty()).then(|| Kept { more: left.more, ..made(place.view.folder.clone(), commits, uncommitted, None) })
        }
        Err(why) => Some(made(place.view.folder.clone(), Vec::new(), Vec::new(), Some(crate::issue::shown(&why)))),
    }
}

/// The view that a take-back takes (#223).
pub struct ToTake {
    /// The folder of the lines of the start that kept it.
    pub session: PathBuf,
    pub role: String,
    pub round: String,
    pub view: View,
}

/// The newest view of `role` for `issue` that is kept and that the lines of its start name, for a take-back. A
/// session that runs, with its `wb` alive, is not taken from: the error says so.
pub fn find(state: &Path, role: &str, issue: &str) -> Result<ToTake, String> {
    let mut runs = false;
    for session in sessions(state, Some(issue)) {
        if session.role != role {
            continue;
        }
        let Some(place) = &session.place else { continue };
        if session.running() == Some(true) {
            runs = true;
            continue;
        }
        return Ok(ToTake { session: session.folder.clone(), role: session.role.clone(), round: session.round(), view: place.view.clone() });
    }
    let issue = crate::issue::shown(issue);
    if runs {
        return Err(format!("the session of the {} for issue {} still runs, and its view is not taken back while it does", role, issue));
    }
    Err(format!("no kept view of the {} for issue {}", crate::issue::shown(role), issue))
}

/// The views of `issue` kept with work not taken back, from any role, oldest first, each once.
pub fn scan(root: &str, state: &Path, issue: &str) -> Vec<Kept> {
    kept_all(root, state, Some(issue), true)
}

/// How many views of `issue` are kept with work, as `scan` finds them, for `--ignore-kept`: no git is run on any
/// view. A view named by `back` with work in it is counted; one named by `folder` and a `back` that failed or none
/// is counted, whatever it holds.
pub fn count(root: &str, state: &Path, issue: &str) -> usize {
    kept_all(root, state, Some(issue), false).len()
}

/// The same for every issue (`wb status` with no issue counts them).
pub fn scan_all(root: &str, state: &Path) -> Vec<Kept> {
    kept_all(root, state, None, true)
}

fn kept_all(root: &str, state: &Path, issue: Option<&str>, look: bool) -> Vec<Kept> {
    let mut found: Vec<Kept> = Vec::new();
    // Newest first: a view named by two folders is the newest one's.
    for session in sessions(state, issue) {
        if let Some(kept) = kept_of(root, state, &session, look) {
            if !found.iter().any(|one| one.folder == kept.folder) {
                found.push(kept);
            }
        }
    }
    found.reverse();
    found
}

/// A session of an issue that was cut off: its `outcome` says `running` and its `wb` is not alive.
pub struct CutOff {
    pub role: String,
    pub round: String,
    pub folder: PathBuf,
}

/// The sessions of `issue` that were cut off, oldest first: at most the newest `MOST_CUT_OFF`.
pub fn cut_off(state: &Path, issue: &str) -> Vec<CutOff> {
    let mut found: Vec<CutOff> =
        sessions(state, Some(issue)).iter().filter(|session| session.cut_off()).take(MOST_CUT_OFF).map(|s| CutOff { role: s.role.clone(), round: s.round(), folder: s.folder.clone() }).collect();
    found.reverse();
    found
}

impl Kept {
    /// How many commits were not taken.
    pub fn count(&self) -> usize {
        self.commits.len()
    }

    /// The line of `wb status`.
    pub fn line(&self) -> String {
        let mut parts = vec![format!("kept: {} round {}", self.role, self.round)];
        match (self.count(), &self.unread) {
            (_, Some(why)) => parts.push(format!("not read: {}", why)),
            (0, None) if !self.uncommitted.is_empty() => parts.push("work left uncommitted".to_string()),
            (1, None) => parts.push("1 commit not taken".to_string()),
            (n, None) if self.more > 0 => parts.push(format!("{} commits not taken and {} more", n, self.more)),
            (n, None) => parts.push(format!("{} commits not taken", n)),
        }
        if !self.uncommitted.is_empty() {
            parts.push(format!("{} paths uncommitted", self.uncommitted.len()));
        }
        parts.push(crate::issue::shown(&self.folder.display().to_string()));
        parts.join(", ")
    }
}

impl CutOff {
    pub fn line(&self) -> String {
        format!("cut off: {} round {}, {}", self.role, self.round, crate::issue::shown(&self.folder.display().to_string()))
    }
}

/// The refusal of a start for `issue` where these views are kept: each is named with its role and round, the
/// commits not taken, and what is left uncommitted; and how to go on.
pub fn refusal(issue: &str, found: &[Kept]) -> String {
    let issue = crate::issue::shown(issue);
    let mut text = match found.len() {
        1 => format!("a view of issue {} is kept with work not taken back, and a session started now would not see that work:", issue),
        n => format!("{} views of issue {} are kept with work not taken back, and a session started now would not see that work:", n, issue),
    };
    for one in found {
        text.push_str(&format!("\n  {}: {} round {}", crate::issue::shown(&one.folder.display().to_string()), one.role, one.round));
        for (commit, subject) in &one.commits {
            text.push_str(&format!("\n    {} {}", commit, subject));
        }
        if one.more > 0 {
            text.push_str(&format!("\n    and {} more", one.more));
        }
        if one.commits.is_empty() && !one.uncommitted.is_empty() && one.unread.is_none() {
            text.push_str("\n    work left uncommitted:");
        }
        for path in &one.uncommitted {
            text.push_str(&format!("\n    uncommitted: {}", path));
        }
        if let Some(why) = &one.unread {
            text.push_str(&format!("\n    not read: {}", why));
        }
    }
    let mut roles: Vec<&str> = Vec::new();
    for one in found {
        if !roles.contains(&one.role.as_str()) {
            roles.push(&one.role);
        }
    }
    let take: Vec<String> = roles.iter().map(|role| format!("`wb start {} {} --take-back`", crate::issue::shown(role), issue)).collect();
    let remove = match found {
        [one] => format!("remove {}", crate::issue::shown(&one.folder.display().to_string())),
        _ => "remove each folder named above".to_string(),
    };
    text.push_str(&format!(
        "\nTake it back with {} (the newest of a role), or {} when its work is not wanted. `--ignore-kept` starts without looking at it, and the stamp says so.",
        take.join(" or "),
        remove
    ));
    text
}
