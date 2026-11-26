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
//! Every commit the ref reaches is read, not only the first parents (#327). The ref is as `wb` writes it only where it
//! is one line of commits, with no merge, each signed by a key of the list with the role it needs: `start` for a stamp
//! or a marker, `accept` for a commit of `wb plan`. Where it is not, nothing on it counts, `wb status` names the first
//! commit that is not, and nothing more is written on it until a person repairs it.
//!
//! `wb start` pushes the ref with no force before the session starts: a push that is not a fast-forward is refused by
//! the remote, so two starts of one role cannot both go on. The rounds are counted from these commits, never from a
//! comment: a `[stamp]` comment is a copy posted out, and `wb` never reads it back.
//!
//! The questions and the answers are commits on the ref too (#238). A questions commit is written by `wb start` after a
//! session, signed with the role `start`: its first line is `[questions] <prefix> q1 q2 …`. An answers commit is written
//! by `wb answer`, signed with the role `answer`: its first line is `[answers] <prefix> q1>role …`. Each holds the tree of
//! the commit before it, and a header of `issue:`, `job:` and `base:` as a stamp has, then one blank line and the form
//! of the comment that is posted out as its copy. Only the header is read for the three lines: what a question says
//! counts for nothing there. Where a job's ref holds a stamp (#333), or any other commit of `wb`, the questions and the
//! answers are read from the ref alone, from that commit on, and the tracker is not read for them (`Ledger::on_ref`):
//! no questions commit is needed, and none is open before the first.
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

/// What a commit of the ref that counts, or would, is: a stamp, a marker of `wb revert`, a commit of questions or of
/// answers (#238).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Stamp,
    Marker,
    Questions,
    Answers,
    /// What a session of any role ended needing a person for (#311).
    NeedsPerson,
    /// A commit over the cap, which is not read.
    Unread,
}

/// What a first line says its commit is, or none: it is a commit of `wb plan`, or not `wb`'s.
fn kind_of(first: &str) -> Option<Kind> {
    if crate::starter::role_and_round("[stamp] ", first).is_some() {
        Some(Kind::Stamp)
    } else if crate::starter::role_and_round("[revert] ", first).is_some() {
        Some(Kind::Marker)
    } else if first.starts_with("[questions] ") {
        Some(Kind::Questions)
    } else if first.starts_with("[answers] ") {
        Some(Kind::Answers)
    } else if first.starts_with("[needs_person] ") {
        Some(Kind::NeedsPerson)
    } else {
        None
    }
}

/// The most a message of a questions commit or an answers commit may be: a commit over `CAP` is not read, and breaks
/// the ref, so a larger one is never written.
pub const MESSAGE_LIMIT: usize = 48 * 1024;

/// The lines of a message that the checks read: the whole message of a stamp or a marker, and for a commit of questions
/// or answers its first line and its header, up to the first blank line, so that a line in what a question says is never
/// taken for one of the header.
fn held_lines(message: &str, kind: Kind) -> String {
    match kind {
        Kind::Questions | Kind::Answers | Kind::NeedsPerson => {
            let mut kept: Vec<&str> = Vec::new();
            for (at, line) in message.split('\n').enumerate() {
                if at > 0 && line.trim_end_matches('\r').is_empty() {
                    break;
                }
                kept.push(line);
            }
            let mut text = kept.join("\n");
            text.push('\n');
            text
        }
        _ => message.to_string(),
    }
}

/// A notice posted out as the copy of a commit (#240): `text`, ended by a line break, and a last line `commit: <hash>` that
/// names the commit on the job's ref that it copies. The line is added to the copy and never to the commit, and no
/// reader of `wb` takes it for part of a stamp, a question or an answer.
pub fn with_commit(text: &str, commit: &str) -> String {
    let mut copy = text.to_string();
    if !copy.ends_with('\n') {
        copy.push('\n');
    }
    copy.push_str(&format!("commit: {}\n", commit));
    copy
}

/// The form a commit of questions or answers is posted out in, as its copy and as what a session is brought: the first
/// line, then what follows the header.
pub fn copy_of(message: &str) -> String {
    let mut lines = message.split('\n');
    let first = lines.next().unwrap_or("");
    let rest: Vec<&str> = lines.skip_while(|line| !line.trim_end_matches('\r').is_empty()).skip(1).collect();
    let mut text = format!("{}\n{}", first, rest.join("\n"));
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

/// One commit of a job's ref that is a stamp or a marker.
#[derive(Clone, Debug)]
pub struct Entry {
    pub commit: String,
    pub kind: Kind,
    /// The first line of its message.
    pub first: String,
    /// Its whole message.
    pub message: String,
    /// Whether it counts: by whom it is signed, or why it does not.
    pub counts: Result<String, String>,
    /// Whether it does not count while the plan is there, but would where the plan is removed: it is signed, of this
    /// issue and repository, with a good `base:`, and holds `job: none`, wherever it is on the ref (#329).
    pub again: bool,
}

/// Why a stamp or a marker does not count where the list is missing: the reason is said once, as `Ledger::missing`.
const NOTHING_SIGNED: &str = "nothing is signed";

/// The start of why a stamp of a plan removed does not count (#329).
const MADE_UNDER: &str = "made under the plan removed in ";

/// The stamps and the markers on a job's ref, oldest first.
#[derive(Clone, Debug)]
pub struct Ledger {
    /// The ref.
    pub full: String,
    pub entries: Vec<Entry>,
    /// Where the default branch holds no list, or there is none, why nothing counts (#323).
    pub missing: Option<String>,
    /// Where the ref is not as `wb` writes it, the one line that names its first commit that is not, and why (#327):
    /// then nothing on it counts.
    pub broken: Option<String>,
    /// Whether the ref holds a commit of `wb plan` (`plan add` or `plan remove`), whether it counts or not (#333).
    pub plan_commit: bool,
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

    /// The role and the round of each stamp that does not count while the plan is there and counts again where it is
    /// removed (#329).
    pub fn counting_again(&self) -> Vec<(String, u64)> {
        let lines: String = self.entries.iter().filter(|entry| entry.again && entry.counts.is_err()).map(|entry| format!("{}\n", entry.first)).collect();
        crate::starter::stamps_in(&lines)
    }

    /// The stamps as the round is worked out from them: on the ref, and printed by no pattern.
    pub fn as_stamps(&self) -> crate::starter::Stamps {
        crate::starter::Stamps { on: self.full.clone(), key: String::new(), found: self.stamps() }
    }

    /// The whole messages of the stamps and markers that count, in order: what the times of the planner's stamps are read from.
    pub fn bodies(&self) -> Vec<String> {
        self.entries.iter().filter(|entry| entry.counts.is_ok() && matches!(entry.kind, Kind::Stamp | Kind::Marker)).map(|entry| entry.message.clone()).collect()
    }

    /// Whether the job is read from its ref alone (#333, #238): the ref holds a commit of `wb`, a stamp, a marker, the
    /// questions or answers of a session, or a commit of `wb plan`, whether it counts or not, or is not as `wb` writes
    /// it. Since #236 every stamp is a commit on the ref, and 0.12.0 is the first release with them, and `wb plan` is in
    /// no release before it, so a ref that holds one is a job begun with this revision. From its first stamp on, the questions and the answers are those of the ref, none is open before
    /// its first questions commit, and the tracker is not read for them: a commit that does not count shuts it out all
    /// the same, so that a comment cannot take its place. A job with no such commit, or no ref, is one begun before, and
    /// its questions and answers are read from the tracker.
    pub fn on_ref(&self) -> bool {
        self.broken.is_some() || self.plan_commit || !self.entries.is_empty()
    }

    /// What the questions and answers are read from, as the whole comments of the tracker are: each commit that counts,
    /// in order, a commit of questions or answers in the form of its copy and the others as they are, so that a marker
    /// takes back what follows the stamp it names (`answer::open_questions`).
    pub fn comments(&self) -> Vec<String> {
        self.entries
            .iter()
            .filter(|entry| entry.counts.is_ok() && entry.kind != Kind::NeedsPerson)
            .map(|entry| if matches!(entry.kind, Kind::Questions | Kind::Answers) { copy_of(&entry.message) } else { entry.message.clone() })
            .collect()
    }

    /// The commits of questions and answers that count and that no marker took back, in order, each in the form of its
    /// copy: what a session is brought of its own questions and answers.
    pub fn thread(&self) -> Vec<String> {
        let counting: Vec<&Entry> = self.entries.iter().filter(|entry| entry.counts.is_ok()).collect();
        let firsts: String = counting.iter().map(|entry| format!("{}\n", entry.first)).collect();
        let gone = crate::starter::taken_back(&firsts);
        counting.iter().zip(gone).filter(|(entry, gone)| !gone && matches!(entry.kind, Kind::Questions | Kind::Answers)).map(|(entry, _)| copy_of(&entry.message)).collect()
    }

    /// What a person is waited for, as the newest of the stamps and the needs_person commits that count says (#311): the
    /// reason of a needs_person commit that no stamp of any role came after, or none. What a marker of `wb revert` took
    /// back counts for nothing, a stamp that it took back closes nothing, and a questions or answers commit closes nothing.
    pub fn person(&self) -> Option<String> {
        let counting: Vec<&Entry> = self.entries.iter().filter(|entry| entry.counts.is_ok()).collect();
        let firsts: String = counting.iter().map(|entry| format!("{}\n", entry.first)).collect();
        let gone = crate::starter::taken_back(&firsts);
        let newest = counting.iter().zip(gone).filter(|(entry, gone)| !gone && matches!(entry.kind, Kind::Stamp | Kind::NeedsPerson)).map(|(entry, _)| entry).last()?;
        if newest.kind == Kind::NeedsPerson {
            Some(copy_of(&newest.message).split_once('\n').map(|(_, reason)| reason.to_string()).unwrap_or_default())
        } else {
            None
        }
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
            // What ran (#336), for a stamp that counts and has the line; a stamp made before has none.
            let ran = entry.message.lines().find_map(|line| line.strip_prefix("ran: ")).filter(|_| entry.counts.is_ok() && entry.kind == Kind::Stamp);
            let line = match ran {
                Some(ran) => format!("{}  ran: {}", line, crate::issue::shown(ran)),
                None => line,
            };
            lines.push(line);
        }
        if let Some(broken) = &self.broken {
            lines.push(broken.clone());
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
    let mut ledger = Ledger { full: full.clone(), entries: Vec::new(), missing, broken: None, plan_commit: false };
    let Ok(object) = git_line(root, &["rev-parse", "--verify", "-q", &full], None) else {
        return Ok(ledger);
    };
    let said = |what: String| format!("{} is not as wb writes it: {}. Nothing on it counts until a person repairs the ref (see the README)", full, what);
    // The ref names a commit, as `wb` writes it: never a tag, a tree or a blob (the review of #328).
    let kind = git_line(root, &["cat-file", "-t", &object], None)?;
    if kind != "commit" {
        ledger.broken = Some(said(format!("it names the {} {}, not a commit", crate::issue::shown(&kind), object)));
        return Ok(ledger);
    }
    let tip = object;
    // In a shallow repository git reads a commit at the boundary as one with no parent, so the stamps before it are not
    // seen (the review of #328). Grafts are turned off in `signers::git`; a shallow repository is said.
    if git_line(root, &["rev-parse", "--is-shallow-repository"], None).is_ok_and(|shallow| shallow == "true") {
        ledger.broken = Some(format!(
            "{} is not as wb writes it: this repository is shallow (git rev-parse --is-shallow-repository), so not every commit of the ref can be read. Nothing on it counts until a person makes it whole (git fetch --unshallow)",
            full
        ));
        return Ok(ledger);
    }
    // Every commit the ref reaches, not only the first parents (#327): a merge or a commit on a side chain is seen.
    // Each line is a commit and its parents; in the order of `--topo-order`, a commit before its parents.
    let most = format!("--max-count={}", MOST + 1);
    let listed = git_line(root, &["rev-list", "--topo-order", "--parents", &most, &tip], None)?;
    let mut walked: Vec<(String, Vec<String>)> = listed
        .lines()
        .filter_map(|line| {
            let mut words = line.split(' ').map(str::to_string);
            words.next().filter(|commit| !commit.is_empty()).map(|commit| (commit, words.collect()))
        })
        .collect();
    if walked.len() > MOST {
        return Err(format!("{} holds more than {} commits, and no more are read", full, MOST));
    }
    walked.reverse();
    let commits: Vec<String> = walked.iter().map(|(commit, _)| commit.clone()).collect();
    let parents_of: std::collections::HashMap<&str, &[String]> = walked.iter().map(|(commit, parents)| (commit.as_str(), parents.as_slice())).collect();
    let mut read = objects(root, &commits)?;
    // In the order of the ref, oldest first.
    read.sort_by_key(|(object, _)| commits.iter().position(|commit| commit == object));
    // The tree of each commit that was read, from its header.
    let trees: std::collections::HashMap<String, String> = read
        .iter()
        .filter_map(|(commit, raw)| {
            let raw = raw.as_ref().ok()?;
            let tree = raw.split(|byte| *byte == b'\n').next()?.strip_prefix(b"tree ")?;
            Some((commit.clone(), String::from_utf8_lossy(tree).to_string()))
        })
        .collect();
    // The plan as the walked tip holds it (#234, the review of #328): what every stamp's `job:` lines are checked
    // against, so that a signed stamp brought onto the ref from elsewhere is held to this repository's job.
    let sources = crate::plan::source_hashes_at(root, &tip);
    // Where the tip holds a plan removed: the commit that removed it and the sources it had (#329).
    let removed = if sources.is_empty() { crate::plan::removed_plan(root, &tip) } else { None };
    let checking = matches!(keys, Keys::List(_));
    // Why each commit is not as `wb` writes it, where it is not (#327).
    let mut faults: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut note = |commit: &str, why: String| {
        faults.entry(commit.to_string()).or_insert(why);
    };
    // Where the walk is: whether it has passed the commit that removed the plan.
    let mut passed_removal = false;
    for (commit, raw) in read {
        let after_removal = passed_removal;
        if removed.as_ref().is_some_and(|(at, _, _)| *at == commit) {
            passed_removal = true;
        }
        let parents = parents_of.get(commit.as_str()).copied().unwrap_or(&[]);
        // A commit over the cap is not read, and so is no stamp: it is said, not left out unseen.
        let raw = match raw {
            Ok(raw) => raw,
            Err(size) => {
                note(&commit, format!("is larger than 64 KiB ({} bytes), so it is not read", size));
                let first = format!("commit {}", commit);
                let counts = Err(format!("larger than 64 KiB ({} bytes), so it is not read", size));
                ledger.entries.push(Entry { commit, kind: Kind::Unread, first, message: String::new(), counts, again: false });
                continue;
            }
        };
        let message = raw.windows(2).position(|pair| pair == b"\n\n").map(|split| String::from_utf8_lossy(&raw[split + 2..]).to_string()).unwrap_or_default();
        let first = message.split('\n').next().unwrap_or("").trim_end_matches('\r').to_string();
        // The role the commit needs: `start` for a stamp, a marker or the questions of a session, `answer` for an answer,
        // `accept` for a commit of `wb plan` (#327, #238).
        let kind = kind_of(&first);
        let (what, role) = match kind {
            Some(Kind::Stamp | Kind::Marker) => ("a stamp or a marker", Some("start")),
            Some(Kind::Questions) => ("a questions commit", Some("start")),
            Some(Kind::NeedsPerson) => ("a needs_person commit", Some("start")),
            Some(Kind::Answers) => ("an answer commit", Some("answer")),
            _ if is_plan_commit(&first) => ("a commit of wb plan", Some("accept")),
            _ => ("neither a stamp, a marker of wb revert, a commit of wb plan, a questions commit nor an answer commit", None),
        };
        let verdict = match (&keys, role) {
            (Keys::List(list), Some(role)) => Some(signed_as(&raw, role, list)),
            _ => None,
        };
        if checking {
            match (&verdict, role) {
                (_, None) => note(&commit, format!("is {}", what)),
                (Some(Err(why)), Some(_)) => note(&commit, format!("is {}, and {}", what, why)),
                _ => {}
            }
            // The tree of a stamp is its parent's with `view` and `issue.yml` alone put in or taken out, and the tree of a
            // marker is its parent's, as `make` writes them (the review of #328): a stamp cannot change the plan. So is the
            // tree of the questions of a session and of an answer (#238).
            if let (Some(kind), true) = (kind, parents.len() <= 1) {
                // The parent's tree, or none for the first commit; a parent that was not read is said for itself.
                let parent_tree = match parents.first() {
                    None => Some(None),
                    Some(parent) => trees.get(parent).map(|tree| Some(tree.as_str())),
                };
                if let (Some(tree), Some(parent_tree)) = (trees.get(&commit), parent_tree) {
                    if let Err(why) = tree_as_written(root, kind, tree, parent_tree) {
                        note(&commit, format!("is {}, and {}", what, why));
                    }
                }
            }
        }
        if is_plan_commit(&first) {
            ledger.plan_commit = true;
        }
        let Some(kind) = kind else { continue };
        let signed = match (&keys, verdict) {
            (Keys::Missing(_), _) => Err(NOTHING_SIGNED.to_string()),
            (Keys::Fault(why), _) => Err(format!("{}, so no stamp counts", why)),
            (Keys::List(_), Some(verdict)) => verdict,
            (Keys::List(_), None) => Err("its signature could not be checked".to_string()),
        };
        let signed_ok = signed.is_ok();
        // The label of a plan removed goes only to a stamp before the commit that removed it.
        let label = if after_removal { None } else { removed.as_ref().map(|(_, short, before)| (short.clone(), before.clone())) };
        // The header alone names the issue, the job and the base of a commit of questions or answers.
        let held = held_lines(&message, kind);
        let counts = signed.and_then(|by| {
            of_this_issue(&held, name)?;
            of_this_repository(root, &held, &sources, label.as_ref())?;
            Ok(by)
        });
        let again = counts.is_err() && signed_ok && of_this_issue(&held, name).is_ok() && of_this_repository(root, &held, &[], None).is_ok();
        ledger.entries.push(Entry { commit, kind, first, message, counts, again });
    }
    // The first commit that is not as `wb` writes it, on the line of first parents from the oldest (the review of #328):
    // a merge is one in itself, so a side chain is named by its merge, and the commit before the one named is always
    // the last that is as `wb` writes it, which the README's repair moves the ref back to.
    let mut line = vec![tip.clone()];
    while let Some(parent) = line.last().and_then(|commit| parents_of.get(commit.as_str())).and_then(|parents| parents.first()) {
        if line.len() > MOST {
            break;
        }
        line.push(parent.clone());
    }
    let first_bad = line.iter().rev().find_map(|commit| {
        let parents = parents_of.get(commit.as_str()).map_or(0, |parents| parents.len());
        if parents > 1 {
            Some((commit.clone(), format!("is a merge of {} commits, and wb writes one line of commits", parents)))
        } else {
            faults.get(commit).map(|why| (commit.clone(), why.clone()))
        }
    });
    if let Some((commit, why)) = first_bad {
        // A stamp is not said to be made under a plan removed where the commit that removed it may not be wb's (#329).
        for entry in ledger.entries.iter_mut().filter(|entry| entry.counts.is_ok() || entry.counts.as_ref().is_err_and(|why| why.starts_with(MADE_UNDER))) {
            entry.counts = Err("nothing on the ref counts, as the line below says".to_string());
        }
        ledger.broken = Some(said(format!("its commit {} {}", commit, crate::issue::shown(&why))));
    }
    Ok(ledger)
}

/// Checks the tree `tree` of a stamp or a marker whose first line is `first` against `parent`, the tree of the commit
/// before it, or none for the first commit of the ref (the review of #328): a stamp's tree is the parent's with only
/// `view` and `issue.yml` put in, changed or taken out, each a plain file; a marker's tree is the parent's as it is.
fn tree_as_written(root: &str, kind: Kind, tree: &str, parent: Option<&str>) -> Result<(), String> {
    let listed = |tree: Option<&str>| -> Result<Vec<String>, String> {
        let Some(tree) = tree else { return Ok(Vec::new()) };
        let printed = signers::git(root, &["ls-tree", "-z", tree], None)?;
        Ok(printed.split(|byte| *byte == 0).filter(|entry| !entry.is_empty()).map(|entry| String::from_utf8_lossy(entry).to_string()).collect())
    };
    let name_of = |entry: &str| entry.split_once('\t').map(|(_, name)| name.to_string()).unwrap_or_default();
    let own = |entry: &String| [VIEW_ENTRY, ISSUE_ENTRY].contains(&name_of(entry).as_str());
    let (mine, before) = (listed(Some(tree))?, listed(parent)?);
    let same = match kind {
        Kind::Marker => Some("a marker's"),
        Kind::Questions => Some("a questions commit's"),
        Kind::Answers => Some("an answer commit's"),
        Kind::NeedsPerson => Some("a needs_person commit's"),
        Kind::Stamp | Kind::Unread => None,
    };
    if let Some(whose) = same {
        return if mine == before { Ok(()) } else { Err(format!("its tree is not the tree of the commit before it, as {} is", whose)) };
    }
    if mine.iter().filter(|entry| own(entry)).any(|entry| !entry.starts_with("100644 blob ")) {
        return Err(format!("its tree holds {} or {} as other than a plain file", VIEW_ENTRY, ISSUE_ENTRY));
    }
    let kept = |entries: &[String]| -> Vec<String> { entries.iter().filter(|entry| !own(entry)).cloned().collect() };
    if kept(&mine) != kept(&before) {
        return Err(format!("its tree changes more than {} and {} of the tree of the commit before it, as a stamp's does not", VIEW_ENTRY, ISSUE_ENTRY));
    }
    Ok(())
}

/// Whether a first line is that of a commit of `wb plan`: `plan add <name>` or `plan remove <name>`.
pub fn is_plan_commit(first: &str) -> bool {
    first.strip_prefix("plan add ").or_else(|| first.strip_prefix("plan remove ")).is_some_and(|name| !name.is_empty())
}

/// Whether the commit `raw` is signed by a key of `list` with `role`: by whom, or why not, in words.
fn signed_as(raw: &[u8], role: &str, list: &signers::List) -> Result<String, String> {
    match signers::verify_commit(raw, role, list) {
        Ok(Verdict::Good(identity)) => Ok(identity),
        Ok(Verdict::Unsigned) => Err(format!("unsigned, and {} of {} is there", signers::PATH, list.on)),
        Ok(Verdict::NotListed) => Err(format!("signed by a key that is not in {} of {}", signers::PATH, list.on)),
        Ok(Verdict::WithoutRole(identity)) => {
            Err(format!("signed by the key of {}, which {} of {} holds without the role {}", crate::issue::shown(&identity), signers::PATH, list.on, role))
        }
        Ok(Verdict::Broken) => Err("its signature does not check: what was signed is not what the commit holds".to_string()),
        Err(why) => Err(format!("its signature could not be checked: {}", why)),
    }
}

/// Why the job's ref of `name` is not as `wb` writes it, where it is not (#327): what a command that writes on it
/// refuses with, before it writes anything. `Err` where the ref cannot be read.
pub fn not_as_written(root: &str, name: &str) -> Result<Option<String>, String> {
    read(root, name).map(|ledger| ledger.broken)
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

/// Checks that a message was written for this repository (WB-BUILD-163 to 165): its one `base:` is a commit object this
/// repository has, and its `job:` lines are `job: none` where the tip of the ref holds no plan (or a removed one), and
/// else a non-empty prefix, in order, of the tip's `sources`: a later `wb plan add` only adds a source at the end, so
/// the stamps before it stay counted. A genuine stamp of another repository, put on this ref, fails one of the two.
fn of_this_repository(root: &str, message: &str, sources: &[String], removed: Option<&(String, Vec<String>)>) -> Result<(), String> {
    const OTHER: &str = "a stamp of another repository";
    let bases: Vec<&str> = message.lines().filter_map(|held| held.strip_prefix("base:")).collect();
    let base = match bases.as_slice() {
        [one] => one.strip_prefix(' ').unwrap_or(one),
        [] => return Err(format!("{}: it holds no line `base:`", OTHER)),
        _ => return Err(format!("{}: it holds more than one line `base:`", OTHER)),
    };
    let hash = !base.is_empty() && matches!(base.len(), 40 | 64) && base.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !hash || git_line(root, &["cat-file", "-t", base], None).map_or(true, |kind| kind != "commit") {
        return Err(format!("{}: its base {} is not a commit of this repository", OTHER, crate::issue::shown(base)));
    }
    let held: Vec<&str> = message.lines().filter_map(|line| line.strip_prefix("job:")).map(|rest| rest.strip_prefix(' ').unwrap_or(rest)).collect();
    let fits = if sources.is_empty() { held == ["none"] } else { !held.is_empty() && held.len() <= sources.len() && held.iter().zip(sources).all(|(held, source)| *held == source.as_str()) };
    if !fits {
        // Lines that are a start of the sources of the plan removed on this ref are that plan's own stamps (#329).
        if let Some((short, before)) = removed {
            if !held.is_empty() && held.len() <= before.len() && held.iter().zip(before).all(|(held, source)| *held == source.as_str()) {
                return Err(format!("{}{}", MADE_UNDER, short));
            }
        }
        return Err(format!("{}: its lines `job:` are not those of the job this repository holds on its ref", OTHER));
    }
    Ok(())
}

/// The line `base:` that a stamp or a marker holds: the commit `HEAD` is at, read with no replace ref; empty where there
/// is none, and then no stamp counts.
pub fn base_line(root: &str) -> String {
    let base = signers::git(root, &["rev-parse", "--verify", "-q", "HEAD^{commit}"], None).map(|printed| String::from_utf8_lossy(&printed).trim().to_string()).unwrap_or_default();
    if base.is_empty() { "base:\n".to_string() } else { format!("base: {}\n", base) }
}

/// The message of a commit of questions or of answers (#238), made of the form of its copy: the first line of `copy`, the
/// header of the issue, the job and the base as a stamp has them, one blank line, and the rest of `copy`. `Err` where it
/// would be over `MESSAGE_LIMIT`, or holds a NUL: no such commit is written.
pub fn thread_message(root: &str, job: &str, copy: &str) -> Result<String, String> {
    let (first, rest) = copy.split_once('\n').unwrap_or((copy, ""));
    let jobs = crate::plan::source_hashes(root, job);
    let job_lines: String = if jobs.is_empty() { "job: none\n".to_string() } else { jobs.iter().map(|hash| format!("job: {}\n", hash)).collect() };
    let message = format!("{}\n{}\n{}{}\n{}", first, issue_line(job), job_lines, base_line(root), rest);
    if message.contains('\0') {
        return Err("it holds a NUL byte".to_string());
    }
    if message.len() > MESSAGE_LIMIT {
        return Err(format!(
            "it is {} bytes, and a message of the ref may be {} KiB at most (a commit over {} KiB is not read, and breaks the ref)",
            message.len(),
            MESSAGE_LIMIT / 1024,
            CAP / 1024
        ));
    }
    Ok(message)
}

/// How the stamp is signed, as worked out before anything is written: by the key git signs with, which the list holds
/// with the role for this identity: `start` for a stamp, a marker or the questions of a session, `answer` for an answer.
pub struct Signing {
    list: signers::List,
    identity: String,
    role: &'static str,
}

/// What the stamp's commit is signed with, before anything is written (WB-BUILD-50 to 53): git must sign with an SSH
/// key that the list holds with `start`. A list that is missing (#323) or cannot be read is refused, as no stamp would
/// count. `Err` says why, and names the setting, the key, or how to add one. Where the repository has remotes and none
/// is `origin`, it is refused too (the review of #322): the lock is the push to `origin`, and a stamp kept local there
/// would hold none.
pub fn ready(root: &str) -> Result<Signing, String> {
    ready_as(root, "start", ("stamp", "a stamp", STAMP_SIGNED))
}

/// What an answer commit is, as `signers::signing_ready` says it.
pub const ANSWER_SIGNED: &str = "an answer is a commit signed by the person who answers";

/// As `ready`, for the commit of an answer (#238): the key git signs with must be held by the list with `answer`.
pub fn ready_for_answer(root: &str) -> Result<Signing, String> {
    ready_as(root, "answer", ("answer commit", "an answer commit", ANSWER_SIGNED))
}

/// `ready` for the `role` a commit needs, with the words `(noun, noun with its article, what it is)` a refusal uses.
fn ready_as(root: &str, role: &'static str, (noun, with_article, what): (&str, &str, &str)) -> Result<Signing, String> {
    // The commit names the commit of `HEAD` it is made from (`base:`), and none counts without one (WB-BUILD-163): where
    // there is none, it is refused before anything is written.
    if signers::git(root, &["rev-parse", "--verify", "-q", "HEAD^{commit}"], None).is_err() {
        return Err(format!("there is no commit at HEAD to base the {} on, and {} with no base counts for nothing", noun, with_article));
    }
    let remotes = remotes(root)?;
    if !remotes.is_empty() && !remotes.iter().any(|remote| remote == "origin") {
        let shown: Vec<String> = remotes.iter().map(|remote| crate::issue::shown(remote)).collect();
        return Err(format!(
            "the {} is pushed to the remote origin, whose ref is the lock, and this repository has the remote{} {} and none named origin. Name the shared remote origin (git remote rename <name> origin)",
            noun,
            if remotes.len() == 1 { "" } else { "s" },
            shown.join(", ")
        ));
    }
    match signers::load(root) {
        keys @ Keys::Missing(_) => Err(signers::missing(&keys, role).unwrap_or_default()),
        Keys::Fault(why) => Err(format!("{}, so no {} would count", why, noun)),
        Keys::List(list) => {
            signers::signing_ready(root, what)?;
            let identity = signers::signer_for(root, &list, role)?;
            Ok(Signing { list, identity, role })
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
    /// The questions of a session (#238): the tree of the commit before, as it is.
    Questions,
    /// The answers of `wb answer` (#238): the tree of the commit before, as it is.
    Answers,
    /// What a session ended needing a person for (#311): the tree of the commit before, as it is.
    NeedsPerson,
}

impl Held<'_> {
    fn word(&self) -> &'static str {
        match self {
            Held::Stamp { .. } => "stamp",
            Held::Marker => "marker",
            Held::Questions => "questions commit",
            Held::Answers => "answer commit",
            Held::NeedsPerson => "needs_person commit",
        }
    }
}

/// Says on `err`, after `name`, what `make` will write: the commit and how it is signed, and where it goes.
pub fn say(name: &str, root: &str, job: &str, held: &Held, signing: &Signing, err: &mut dyn Write) {
    let full = full(job);
    let how = format!("signed by {}, with the role {} in {}", crate::issue::shown(&signing.identity), signing.role, signers::PATH);
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
        (Held::Marker | Held::Questions | Held::Answers | Held::NeedsPerson, Some(commit)) => git_line(root, &["rev-parse", "--verify", "-q", &format!("{}^{{tree}}", commit)], None)?,
        (Held::Marker | Held::Questions | Held::Answers | Held::NeedsPerson, None) => crate::plan::mktree(root, &[])?,
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
    let word = held.word();
    let commit = git_line(root, &args, Some(message.as_bytes())).map_err(|why| format!("the commit of the {} could not be made: {}", word, why))?;
    let list = &signing.list;
    let raw = signers::git(root, &["cat-file", "commit", &commit], None)?;
    match signers::verify_commit(&raw, signing.role, list) {
        Ok(Verdict::Good(_)) => {}
        Ok(Verdict::Unsigned) => return Err(format!("git made the commit of the {} unsigned", word)),
        Ok(_) => return Err(format!("the commit of the {} does not check against {} of {}", word, signers::PATH, list.on)),
        Err(why) => return Err(format!("the signature of the {} could not be checked: {}", word, why)),
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
    let specs: Vec<String> = made.moved.iter().map(|full| format!("{}:{}", made.commit, full)).collect();
    push_refs(root, &specs)
}

/// Pushes each of `specs` (`<source>:<ref>`, never with a leading `+`) to `origin`, as `push` does: with no force and all
/// or none, as the person. The notes of #237 go this way too.
pub fn push_refs(root: &str, specs: &[String]) -> Pushed {
    match remotes(root) {
        Ok(remotes) if remotes.is_empty() => return Pushed::Local,
        Ok(remotes) if !remotes.iter().any(|remote| remote == "origin") => return Pushed::Failed("there is no remote origin to push to".to_string()),
        Ok(_) => {}
        Err(why) => return Pushed::Failed(why),
    }
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args(["push", "--porcelain", "--atomic", "origin"]);
    for spec in specs {
        command.arg(spec);
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

/// The newest stamp that counts on the job's ref of `name` whose whole message is `text`, as a start leaves it in the
/// `stamp` of its lines, or none: where it is not there, the ref is not as `wb` writes it, or none counts (#237). A
/// note names a commit found so, never one that only claims to be a session's.
pub fn commit_of(root: &str, name: &str, text: &str) -> Option<String> {
    let ledger = read(root, name).ok()?;
    ledger.entries.iter().rev().find(|entry| entry.counts.is_ok() && entry.first.starts_with("[stamp] ") && entry.message.trim_end() == text.trim_end()).map(|entry| entry.commit.clone())
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

/// What is said where the push of a questions commit is refused or fails (#238): how the questions get asked. It holds
/// where the repository has the fetch line `wb init` adds, and where it has not. The text ends with no full stop: the
/// caller says that the questions are not written, and says them whole.
fn questions_not_pushed(made: &Made, other: &str, said: &str, behind: bool) -> String {
    let full = &made.full;
    let text = "the questions are in the text below, and the next round of the planner also writes its own again from its file";
    if behind {
        return format!(
            "{}: origin refused the questions commit {} as not a fast-forward of its {} ({}). To get the questions asked, fetch the ref first with `git fetch origin {}:{}` (a fetch of the name alone does not bring it where the repository lacks the fetch line `+refs/watchbill/*:refs/watchbill/*` that `wb init` adds); until then nothing asks them: {}",
            other, made.commit, full, said, full, full, text
        );
    }
    format!(
        "the questions commit could not be pushed to origin: {}. To get the questions asked, push it by hand: `git push origin {}:{}`; once it is pushed, the questions are on the ref and asked. Where the repository lacks the fetch line `+refs/watchbill/*:refs/watchbill/*` that `wb init` adds, that push does not move the local ref, and the person fetches it with `git fetch origin {}:{}`. Until it is pushed nothing asks them: {}",
        said, made.commit, full, full, full, text
    )
}

/// Writes the commit of `message` on the job's ref of `job`, as `make` makes it, pushes it, and moves the local refs
/// (WB-BUILD-43 to 46): the commit, and whether it is only local, where it went on; else why not, with nothing moved
/// and nothing to follow from it. What it does is said on `err` after `name`.
pub fn write(name: &str, root: &str, job: &str, message: &str, held: &Held, signing: &Signing, err: &mut dyn Write) -> Result<(String, bool), String> {
    let (word, nothing, other) = match held {
        Held::Stamp { .. } => ("stamp", "Nothing is started", "another start went first"),
        Held::Marker => ("marker", "Nothing is posted", "another start or marker went first"),
        Held::Questions => ("questions commit", "The questions are not written", "another start or answer went first"),
        Held::Answers => ("answer commit", "No more answers are written and nothing is posted", "another start or answer went first"),
        Held::NeedsPerson => ("needs_person commit", "The needs_person is not written on the ref", "another start or answer went first"),
    };
    say(name, root, job, held, signing, err);
    let made = make(root, job, message, held, signing)?;
    let local = match push(root, &made) {
        Pushed::Remote => false,
        Pushed::Local => true,
        Pushed::Lost(said) if matches!(held, Held::Questions) => return Err(questions_not_pushed(&made, other, &said, true)),
        Pushed::Failed(said) if matches!(held, Held::Questions) => return Err(questions_not_pushed(&made, other, &said, false)),
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

