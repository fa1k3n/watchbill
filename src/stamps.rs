//! The stamps on the job's ref (#236; `docs/revision.md`, points 12 and 15).
//!
//! A stamp is a commit on `refs/watchbill/<name>`, after the commit before it on the ref. Its message is the stamp's
//! lines, the first `[stamp] <role> round <n>`; its tree holds the job's tree (what the commit before held), the view's
//! list as `view` and the issue's file as `issue.yml`. A marker of `wb revert` is a commit of the same kind whose first
//! line is `[revert] <role> round <n>`, and whose tree is the one before it. Each is signed by the starter, and counts
//! only where its signature checks against a key of `.watchbill/signers` of the default branch with the role `start`.
//! Where the default branch holds no list, none counts and none is written (#323). Each also holds one line
//! `issue: <name>`, one `base:` that is a commit this repository has, and the `job:` lines of the plan that the commit
//! before it holds: one signed in another repository and put on this ref is "a stamp of another repository" (#324).
//!
//! `wb start` pushes the ref with no force before the session starts: a push that is not a fast-forward is refused by
//! the remote, so two starts of one role cannot both go on. The rounds are counted from these commits, never from a
//! comment: a `[stamp]` comment is a copy posted out, and `wb` never reads it back.
//!
//! Every read that decides what counts runs git with no replace ref and no variable `GIT_*` of this process
//! (`signers::git`); the push runs as the person, as the push of a branch does (#151).

use std::io::Write;
use std::process::{Command, Stdio};

use crate::signers::{self, Keys, Verdict};

/// The most commits of a job's ref that are read: far above the rounds of any job.
pub const MOST: usize = 4096;

/// The most a commit of a stamp is read at: far above any stamp `wb` writes. A larger commit is not a stamp.
pub const CAP: u64 = 64 * 1024;

/// The entry of a stamp's tree that holds the view's list.
pub const VIEW_ENTRY: &str = "view";

/// The entry of a stamp's tree that holds the issue's file.
pub const ISSUE_ENTRY: &str = "issue.yml";

/// What a stamp is, as `signers::signing_ready` says it.
pub const STAMP_SIGNED: &str = "a stamp is a commit signed by the starter";

/// The job's ref of `name`.
pub fn full(name: &str) -> String {
    format!("{}{}", crate::plan::PREFIX, name)
}

fn git_line(root: &str, args: &[&str], input: Option<&[u8]>) -> Result<String, String> {
    signers::git(root, args, input).map(|printed| String::from_utf8_lossy(&printed).trim_end_matches('\n').to_string())
}

/// Whether a first line is that of a stamp or of a marker of `wb revert`.
fn is_entry(first: &str) -> bool {
    crate::starter::role_and_round("[stamp] ", first).is_some() || crate::starter::role_and_round("[revert] ", first).is_some()
}

/// One commit of a job's ref that is a stamp or a marker.
#[derive(Clone, Debug)]
pub struct Entry {
    pub commit: String,
    /// The first line of its message.
    pub first: String,
    /// Its whole message.
    pub message: String,
    /// Whether it counts: by whom it is signed, or why it does not.
    pub counts: Result<String, String>,
}

/// Why a stamp or a marker does not count where the list is missing: the reason is said once, as `Ledger::missing`.
const NOTHING_SIGNED: &str = "nothing is signed";

/// The stamps and the markers on a job's ref, oldest first.
#[derive(Clone, Debug)]
pub struct Ledger {
    /// The ref.
    pub full: String,
    pub entries: Vec<Entry>,
    /// Where the default branch holds no list, or there is none, why nothing counts (#323).
    pub missing: Option<String>,
}

impl Ledger {
    /// The first lines of the commits that count, one to a line, in the order they were written.
    pub fn lines(&self) -> String {
        self.entries.iter().filter(|entry| entry.counts.is_ok()).map(|entry| format!("{}\n", entry.first)).collect()
    }

    /// The role and the round of each stamp that counts, with what a marker took back left out (`starter::counted`).
    pub fn stamps(&self) -> Vec<(String, u64)> {
        crate::starter::stamps_in(&self.lines())
    }

    /// The stamps as the round is worked out from them: on the ref, and printed by no pattern.
    pub fn as_stamps(&self) -> crate::starter::Stamps {
        crate::starter::Stamps { on: self.full.clone(), key: String::new(), found: self.stamps() }
    }

    /// The whole messages of the commits that count, in order: what the times of the planner's stamps are read from.
    pub fn bodies(&self) -> Vec<String> {
        self.entries.iter().filter(|entry| entry.counts.is_ok()).map(|entry| entry.message.clone()).collect()
    }

    /// Whether a `[stamp] planner` counts.
    pub fn has_planner(&self) -> bool {
        self.stamps().iter().any(|(role, _)| role == "planner")
    }

    /// One line for each commit, as `wb status <issue>` says what it worked out from: whether it counts and why, and
    /// what a marker took back; and, where the list is missing, why nothing counts, once.
    pub fn said(&self) -> Vec<String> {
        let gone = crate::starter::taken_back(&self.lines());
        let mut counting = gone.into_iter();
        let mut lines = Vec::new();
        for entry in &self.entries {
            let shown = crate::issue::shown(&entry.first);
            let line = match &entry.counts {
                Err(why) => format!("{}  (on {}, not counted: {})", shown, self.full, crate::issue::shown(why)),
                Ok(_) if counting.next().unwrap_or(false) => format!("{}  (on {}, taken back by a [revert] marker)", shown, self.full),
                Ok(by) => format!("{}  (on {}, signed by {})", shown, self.full, crate::issue::shown(by)),
            };
            lines.push(line);
        }
        if let (Some(why), false) = (&self.missing, self.entries.is_empty()) {
            lines.push(why.clone());
        }
        lines
    }
}

/// The objects of `commits`, each as `git cat-file commit` prints it, or, for one larger than `CAP`, `None` with its size:
/// it is not read, and is said.
fn objects(root: &str, commits: &[String]) -> Result<Vec<(String, Result<Vec<u8>, u64>)>, String> {
    if commits.is_empty() {
        return Ok(Vec::new());
    }
    let input: String = commits.iter().map(|commit| format!("{}\n", commit)).collect();
    let sizes = git_line(root, &["cat-file", "--batch-check"], Some(input.as_bytes()))?;
    let mut small = Vec::new();
    let mut found = Vec::new();
    for line in sizes.lines() {
        let words: Vec<&str> = line.split(' ').collect();
        if let [object, "commit", size] = words.as_slice() {
            match size.parse::<u64>() {
                Ok(size) if size <= CAP => small.push(object.to_string()),
                Ok(size) => found.push((object.to_string(), Err(size))),
                Err(_) => return Err("git cat-file printed a size that is not a number".to_string()),
            }
        }
    }
    if small.is_empty() {
        return Ok(found);
    }
    let input: String = small.iter().map(|commit| format!("{}\n", commit)).collect();
    let printed = signers::git(root, &["cat-file", "--batch"], Some(input.as_bytes()))?;
    let mut at = 0;
    while at < printed.len() {
        let end = printed[at..].iter().position(|byte| *byte == b'\n').map(|end| at + end).ok_or("git cat-file printed what is not an object")?;
        let head = String::from_utf8_lossy(&printed[at..end]).to_string();
        let words: Vec<&str> = head.split(' ').collect();
        let [object, "commit", size] = words.as_slice() else {
            return Err(format!("git cat-file printed {} where a commit was asked for", crate::issue::shown(&head)));
        };
        let size: usize = size.parse().map_err(|_| "git cat-file printed a size that is not a number".to_string())?;
        let body = printed.get(end + 1..end + 1 + size).ok_or("git cat-file printed less than the size it gave")?;
        found.push((object.to_string(), Ok(body.to_vec())));
        at = end + 1 + size + 1;
    }
    Ok(found)
}

/// The line `issue: <name>` that every stamp and marker holds (the review of #322): a commit counts only on the ref of
/// the issue it names, so a signed stamp of another job, put under a commit of this ref, is not a stamp of this one.
pub fn issue_line(name: &str) -> String {
    format!("issue: {}", name)
}

/// `message` with the line `issue: <name>`: as it is where it holds it, else with an empty `issue:` line filled, else
/// with the line after its first.
pub fn with_issue(message: &str, name: &str) -> String {
    let line = issue_line(name);
    if message.lines().any(|held| held == line) {
        return message.to_string();
    }
    let mut lines: Vec<String> = message.lines().map(str::to_string).collect();
    match lines.iter().position(|held| held == "issue:") {
        Some(at) => lines[at] = line,
        None => lines.insert(1.min(lines.len()), line),
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out
}

/// The stamps and markers on the job's ref of `name` in the repository at `root`, each checked against the list of
/// the default branch and against the issue it names: an empty ledger where the ref is not there. `Err` where the ref
/// cannot be read.
pub fn read(root: &str, name: &str) -> Result<Ledger, String> {
    let full = full(name);
    let keys = signers::load(root);
    let missing = match &keys {
        Keys::Missing(why) => Some(why.clone()),
        _ => None,
    };
    let mut ledger = Ledger { full: full.clone(), entries: Vec::new(), missing };
    if git_line(root, &["rev-parse", "--verify", "-q", &full], None).is_err() {
        return Ok(ledger);
    }
    let tip = git_line(root, &["rev-parse", "--verify", "-q", &format!("{}^{{commit}}", full)], None).map_err(|_| format!("{} is not a commit", full))?;
    let most = format!("--max-count={}", MOST + 1);
    let listed = git_line(root, &["rev-list", "--first-parent", &most, &tip], None)?;
    let mut commits: Vec<String> = listed.lines().map(str::to_string).collect();
    if commits.len() > MOST {
        return Err(format!("{} holds more than {} commits, and no more are read", full, MOST));
    }
    commits.reverse();
    let mut read = objects(root, &commits)?;
    // In the order of the ref, oldest first.
    read.sort_by_key(|(object, _)| commits.iter().position(|commit| commit == object));
    for (commit, raw) in read {
        // A commit over the cap is not read, and so is no stamp: it is said, not left out unseen.
        let raw = match raw {
            Ok(raw) => raw,
            Err(size) => {
                let first = format!("commit {}", commit);
                let counts = Err(format!("larger than 64 KiB ({} bytes), so it is not read", size));
                ledger.entries.push(Entry { commit, first, message: String::new(), counts });
                continue;
            }
        };
        let Some(split) = raw.windows(2).position(|pair| pair == b"\n\n") else { continue };
        let message = String::from_utf8_lossy(&raw[split + 2..]).to_string();
        let first = message.split('\n').next().unwrap_or("").trim_end_matches('\r').to_string();
        if !is_entry(&first) {
            continue;
        }
        let signed = match &keys {
            Keys::Missing(_) => Err(NOTHING_SIGNED.to_string()),
            Keys::Fault(why) => Err(format!("{}, so no stamp counts", why)),
            Keys::List(list) => match signers::verify_commit(&raw, "start", list) {
                Ok(Verdict::Good(identity)) => Ok(identity),
                Ok(Verdict::Unsigned) => Err(format!("unsigned, and {} of {} is there", signers::PATH, list.on)),
                Ok(Verdict::NotListed) => Err(format!("signed by a key that is not in {} of {}", signers::PATH, list.on)),
                Ok(Verdict::WithoutRole(identity)) => {
                    Err(format!("signed by the key of {}, which {} of {} holds without the role start", crate::issue::shown(&identity), signers::PATH, list.on))
                }
                Ok(Verdict::Broken) => Err("its signature does not check: what was signed is not what the commit holds".to_string()),
                Err(why) => Err(format!("its signature could not be checked: {}", why)),
            },
        };
        let parent = commits.iter().position(|held| *held == commit).and_then(|at| at.checked_sub(1)).map(|at| commits[at].as_str());
        let counts = signed.and_then(|by| {
            of_this_issue(&message, name)?;
            of_this_repository(root, &message, name, parent)?;
            Ok(by)
        });
        ledger.entries.push(Entry { commit, first, message, counts });
    }
    Ok(ledger)
}

/// Checks that a message names the issue of the ref it is on, in exactly one line `issue: <name>` (WB-BUILD-161): a
/// signed stamp of another job, put under a commit of this ref, is not a stamp of this one.
fn of_this_issue(message: &str, name: &str) -> Result<(), String> {
    let named: Vec<&str> = message.lines().filter_map(|held| held.strip_prefix("issue:")).collect();
    let shown_name = crate::issue::shown(name);
    match named.as_slice() {
        [] => Err(format!("a stamp of another job: it names no issue, and this is the ref of {}", shown_name)),
        [one] if *one == format!(" {}", name) => Ok(()),
        [one] if one.trim() == name => Err(format!("a stamp with no clear issue: its line is not exactly `{}` (no space around the number)", issue_line(name))),
        [one] => Err(format!("a stamp of another job: it names issue {}, not {}", crate::issue::shown(one.trim()), shown_name)),
        _ => Err(format!("a stamp with no clear issue: it holds more than one line `issue:`, and only `{}` is one of this ref", issue_line(name))),
    }
}

/// Checks that a message was written for this repository (WB-BUILD-163 to 165): its one `base:` is a commit this
/// repository has, and its `job:` lines are the hashes of the plan that `parent`, the commit before it on the ref, holds
/// (`job: none` where it holds none). A genuine stamp of another repository, put on this ref, fails one of the two.
fn of_this_repository(root: &str, message: &str, name: &str, parent: Option<&str>) -> Result<(), String> {
    const OTHER: &str = "a stamp of another repository";
    let bases: Vec<&str> = message.lines().filter_map(|held| held.strip_prefix("base:")).collect();
    let base = match bases.as_slice() {
        [one] => one.strip_prefix(' ').unwrap_or(one),
        [] => return Err(format!("{}: it holds no line `base:`", OTHER)),
        _ => return Err(format!("{}: it holds more than one line `base:`", OTHER)),
    };
    let hash = !base.is_empty() && matches!(base.len(), 40 | 64) && base.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !hash || git_line(root, &["cat-file", "-e", &format!("{}^{{commit}}", base)], None).is_err() {
        return Err(format!("{}: its base {} is not a commit of this repository", OTHER, crate::issue::shown(base)));
    }
    let held: Vec<&str> = message.lines().filter_map(|line| line.strip_prefix("job:")).map(|rest| rest.strip_prefix(' ').unwrap_or(rest)).collect();
    let mut expected = parent.map(|commit| crate::plan::source_hashes_at(root, name, commit)).unwrap_or_default();
    if expected.is_empty() {
        expected.push("none".to_string());
    }
    if held != expected {
        return Err(format!("{}: its lines `job:` are not those of the job on {}", OTHER, full(name)));
    }
    Ok(())
}

/// The line `base:` that a stamp or a marker holds: the commit `HEAD` is at, read with no replace ref; empty where there
/// is none, and then no stamp counts.
pub fn base_line(root: &str) -> String {
    let base = signers::git(root, &["rev-parse", "--verify", "-q", "HEAD^{commit}"], None).map(|printed| String::from_utf8_lossy(&printed).trim().to_string()).unwrap_or_default();
    if base.is_empty() { "base:\n".to_string() } else { format!("base: {}\n", base) }
}

/// How the stamp is signed, as worked out before anything is written: by the key git signs with, which the list holds
/// with `start` for this identity.
pub struct Signing {
    list: signers::List,
    identity: String,
}

/// What the stamp's commit is signed with, before anything is written (WB-BUILD-50 to 53): git must sign with an SSH
/// key that the list holds with `start`. A list that is missing (#323) or cannot be read is refused, as no stamp would
/// count. `Err` says why, and names the setting, the key, or how to add one. Where the repository has remotes and none
/// is `origin`, it is refused too (the review of #322): the lock is the push to `origin`, and a stamp kept local there
/// would hold none.
pub fn ready(root: &str) -> Result<Signing, String> {
    let remotes = remotes(root)?;
    if !remotes.is_empty() && !remotes.iter().any(|remote| remote == "origin") {
        let shown: Vec<String> = remotes.iter().map(|remote| crate::issue::shown(remote)).collect();
        return Err(format!(
            "the stamp is pushed to the remote origin, whose ref is the lock, and this repository has the remote{} {} and none named origin. Name the shared remote origin (git remote rename <name> origin)",
            if remotes.len() == 1 { "" } else { "s" },
            shown.join(", ")
        ));
    }
    match signers::load(root) {
        keys @ Keys::Missing(_) => Err(signers::missing(&keys, "start").unwrap_or_default()),
        Keys::Fault(why) => Err(format!("{}, so no stamp would count", why)),
        Keys::List(list) => {
            signers::signing_ready(root, STAMP_SIGNED)?;
            let identity = signers::signer_for(root, &list, "start")?;
            Ok(Signing { list, identity })
        }
    }
}

/// A commit made for the job's ref, and not yet on it.
pub struct Made {
    /// The job's ref.
    pub full: String,
    pub commit: String,
    /// The commit before it on the ref, where there was one.
    pub old: Option<String>,
    /// The refs moved to it: every name of the job at the commit before, or the job's ref alone.
    pub moved: Vec<String>,
}

/// The tip of the job's ref of `name`, where it is there.
fn tip(root: &str, full: &str) -> Result<Option<String>, String> {
    if git_line(root, &["rev-parse", "--verify", "-q", full], None).is_err() {
        return Ok(None);
    }
    git_line(root, &["rev-parse", "--verify", "-q", &format!("{}^{{commit}}", full)], None).map(Some).map_err(|_| format!("{} is not a commit", full))
}

/// What a commit for the job's ref holds.
pub enum Held<'a> {
    /// A stamp: the job's tree, with the view's list and the issue's file where there are.
    Stamp { view: Option<&'a str>, issue: Option<&'a [u8]> },
    /// A marker of `wb revert`: the tree of the commit before, as it is.
    Marker,
}

impl Held<'_> {
    fn word(&self) -> &'static str {
        match self {
            Held::Stamp { .. } => "stamp",
            Held::Marker => "marker",
        }
    }
}

/// Says on `err`, after `name`, what `make` will write: the commit and how it is signed, and where it goes.
pub fn say(name: &str, root: &str, job: &str, held: &Held, signing: &Signing, err: &mut dyn Write) {
    let full = full(job);
    let how = format!("signed by {}, with the role start in {}", crate::issue::shown(&signing.identity), signers::PATH);
    let after = match tip(root, &full) {
        Ok(Some(commit)) => format!("after {}", commit),
        _ => "the first on it".to_string(),
    };
    let then = if has_origin(root) { "and push it to origin with no force" } else { "and keep it on the local ref: there is no remote" };
    let _ = writeln!(err, "{}: will write the {} as a commit on {}, {}, {}, {}", name, held.word(), full, after, how, then);
}

/// Makes the commit of `message` for the job's ref of `job`: for a stamp, its tree is the tree of the commit before,
/// with the view's list and the issue's file in place of what it held under their names; for a marker, the tree before.
/// It is signed as `signing` says, and checked again. No ref is moved: `push` and `record` do that.
pub fn make(root: &str, job: &str, message: &str, held: &Held, signing: &Signing) -> Result<Made, String> {
    let full = full(job);
    let old = tip(root, &full)?;
    let tree = match (held, &old) {
        (Held::Marker, Some(commit)) => git_line(root, &["rev-parse", "--verify", "-q", &format!("{}^{{tree}}", commit)], None)?,
        (Held::Marker, None) => crate::plan::mktree(root, &[])?,
        (Held::Stamp { view, issue }, _) => {
            let mut top = match &old {
                Some(commit) => crate::plan::entries(root, &format!("{}^{{tree}}", commit), &[VIEW_ENTRY, ISSUE_ENTRY])?,
                None => Vec::new(),
            };
            for (entry, bytes) in [(VIEW_ENTRY, view.map(str::as_bytes)), (ISSUE_ENTRY, *issue)] {
                if let Some(bytes) = bytes {
                    let blob = signers::blob_of(root, bytes, true)?;
                    top.push(format!("100644 blob {}\t{}", blob, entry));
                }
            }
            crate::plan::mktree(root, &top)?
        }
    };
    let mut args = vec!["commit-tree", tree.as_str()];
    if let Some(commit) = &old {
        args.push("-p");
        args.push(commit);
    }
    args.push("-S");
    // Every stamp and marker names its issue (the review of #322), whatever the prompt holds.
    let message = with_issue(message, job);
    let commit = git_line(root, &args, Some(message.as_bytes())).map_err(|why| format!("the commit of the stamp could not be made: {}", why))?;
    let list = &signing.list;
    let raw = signers::git(root, &["cat-file", "commit", &commit], None)?;
    match signers::verify_commit(&raw, "start", list) {
        Ok(Verdict::Good(_)) => {}
        Ok(Verdict::Unsigned) => return Err("git made the commit of the stamp unsigned".to_string()),
        Ok(_) => return Err(format!("the commit of the stamp does not check against {} of {}", signers::PATH, list.on)),
        Err(why) => return Err(format!("the signature of the stamp could not be checked: {}", why)),
    }
    let moved = match &old {
        Some(commit) => {
            let at = git_line(root, &["for-each-ref", "--points-at", commit, "--format=%(refname)", crate::plan::PREFIX], None)?;
            let mut moved: Vec<String> = at.lines().map(str::to_string).collect();
            if !moved.contains(&full) {
                moved.push(full.clone());
            }
            moved
        }
        None => vec![full.clone()],
    };
    Ok(Made { full, commit, old, moved })
}

/// How the push of a stamp went.
pub enum Pushed {
    /// The remote took it.
    Remote,
    /// There is no remote `origin`: the stamp stays on the local ref.
    Local,
    /// The remote refused it as not a fast-forward: another start went first. What git said.
    Lost(String),
    /// The push failed otherwise. What git said.
    Failed(String),
}

/// The remotes of the repository at `root`, as `git remote` prints them.
pub fn remotes(root: &str) -> Result<Vec<String>, String> {
    git_line(root, &["remote"], None).map(|printed| printed.lines().map(str::trim).filter(|name| !name.is_empty()).map(str::to_string).collect())
}

/// Whether the repository at `root` has a remote `origin`.
pub fn has_origin(root: &str) -> bool {
    remotes(root).is_ok_and(|remotes| remotes.iter().any(|remote| remote == "origin"))
}

/// Pushes the commit `made` to every ref it moves at `origin`, with no force and all or none (`--atomic`), as the
/// person, outside any sandbox. Where there is no remote at all, nothing is pushed; where there are remotes and none
/// is `origin`, the push fails (`ready` refuses that before anything is written).
pub fn push(root: &str, made: &Made) -> Pushed {
    match remotes(root) {
        Ok(remotes) if remotes.is_empty() => return Pushed::Local,
        Ok(remotes) if !remotes.iter().any(|remote| remote == "origin") => return Pushed::Failed("there is no remote origin to push to".to_string()),
        Ok(_) => {}
        Err(why) => return Pushed::Failed(why),
    }
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args(["push", "--porcelain", "--atomic", "origin"]);
    for full in &made.moved {
        command.arg(format!("{}:{}", made.commit, full));
    }
    let output = match command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).output() {
        Ok(output) => output,
        Err(error) => return Pushed::Failed(format!("git could not be run: {}", error)),
    };
    if output.status.success() {
        return Pushed::Remote;
    }
    let printed = String::from_utf8_lossy(&output.stdout).to_string();
    let said: String = String::from_utf8_lossy(&output.stderr).lines().map(str::trim).filter(|line| !line.is_empty()).map(crate::issue::shown).collect::<Vec<_>>().join("; ");
    let lost = printed.lines().any(|line| line.starts_with('!') && (line.contains("(non-fast-forward)") || line.contains("(fetch first)")));
    let words = if said.is_empty() { printed.lines().map(crate::issue::shown).collect::<Vec<_>>().join("; ") } else { said };
    if lost {
        Pushed::Lost(words)
    } else {
        Pushed::Failed(if words.is_empty() { "git push failed".to_string() } else { words })
    }
}

/// Moves the refs of `made` to its commit from the one before, in one transaction: none where one of them is no
/// longer what it was read as.
pub fn record(root: &str, made: &Made) -> Result<(), String> {
    let mut input = String::new();
    for full in &made.moved {
        // The push moves a ref that the fetch line of the jobs maps back onto itself (`wb init`): it is there already.
        if git_line(root, &["rev-parse", "--verify", "-q", full], None).is_ok_and(|now| now == made.commit) {
            continue;
        }
        match &made.old {
            Some(old) => input.push_str(&format!("update {} {} {}\n", full, made.commit, old)),
            None => input.push_str(&format!("create {} {}\n", full, made.commit)),
        }
    }
    signers::git(root, &["update-ref", "--stdin"], Some(input.as_bytes())).map(|_| ())
}

/// What a start says where a later step failed after origin took its stamp `commit` (the review of #322): the stamp holds
/// the round, and `wb revert` takes it back.
pub fn round_held(job: &str, commit: &str, message: &str) -> String {
    let role = message.split(' ').nth(1).unwrap_or("<role>");
    format!(
        "The stamp {} is on {} and holds the round: take it back with `wb revert {} {}` if the round is to be run again.",
        commit,
        full(job),
        job,
        role
    )
}

/// Writes the commit of `message` on the job's ref of `job`, as `make` makes it, pushes it, and moves the local refs
/// (WB-BUILD-43 to 46): the commit, and whether it is only local, where it went on; else why not, with nothing moved
/// and nothing to follow from it. What it does is said on `err` after `name`.
pub fn write(name: &str, root: &str, job: &str, message: &str, held: &Held, signing: &Signing, err: &mut dyn Write) -> Result<(String, bool), String> {
    let (word, nothing, other) = match held {
        Held::Stamp { .. } => ("stamp", "Nothing is started", "another start went first"),
        Held::Marker => ("marker", "Nothing is posted", "another start or marker went first"),
    };
    say(name, root, job, held, signing, err);
    let made = make(root, job, message, held, signing)?;
    let local = match push(root, &made) {
        Pushed::Remote => false,
        Pushed::Local => true,
        Pushed::Lost(said) => {
            return Err(format!(
                "{}: origin refused the {} {} as not a fast-forward of its {} ({}). Fetch it with `git fetch origin`, and see the job with `wb status`. {}",
                other, word, made.commit, made.full, said, nothing
            ))
        }
        Pushed::Failed(said) => return Err(format!("the {} could not be pushed to origin, so it holds no lock: {}. {}", word, said, nothing)),
    };
    record(root, &made).map_err(|why| {
        let held = match (local, held) {
            (false, Held::Stamp { .. }) => format!(" {}", round_held(job, &made.commit, message)),
            _ => String::new(),
        };
        format!("{} could not be moved to {}: {}. {}.{}", made.full, made.commit, why, nothing, held)
    })?;
    if local {
        let _ = writeln!(err, "{}: there is no remote origin, so the {} {} is on the local {} alone, and no start elsewhere is locked out by it", name, word, made.commit, made.full);
    } else {
        crate::refusal::inform(err, format_args!("{}: wrote the {} {} on {}, and pushed it to origin", name, word, made.commit, made.full));
    }
    Ok((made.commit, local))
}

