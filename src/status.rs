//! `wb status` (#139, #155, #284): the jobs in the flow, as `git status` lists files: under two headings by whose move it
//! is next, each with the stage it has reached as a word, coloured and with hint lines as git does, and in git's short and
//! porcelain forms. It only reads, and writes nothing but a temporary repository in $TMPDIR, removed after, to read a kept
//! view (#222). Where the person is to move it says what to do, in git's form, unless `status_hints: false` is written in
//! `watchbill.yml`.
//!
//! The issues in the flow are those whose branch, by the pattern `branch`, local or at `origin`, holds a commit
//! that the default branch of `origin` does not. They are found with git alone, from the refs as they are:
//! nothing is fetched. An issue that is done is not listed: its branch is merged into the default branch or
//! deleted, or (as the owner squash-merges) its whole change is one commit on the default branch, by
//! `git patch-id --stable`, or its upstream at `origin` is gone. `wb status <issue>` reads it all the same, and
//! says that it is not in the flow.
//!
//! What it reads besides: the comments of each issue from `issue_comments`, the open change of its branch from
//! `change_of_branch`, the title of one issue from `issue_address`, and the issue's file and its acceptance from
//! the branch of the issue with `git show`, never by switching branches.
//!
//! `issue_comments` prints only the first line of each comment and no author. An answer therefore cannot be told
//! from a person's: until the program reads who wrote a comment (#83), every `[answers]` and `[answer]` line counts
//! as a person's. A line that starts with `[question]` or `[questions]` is never an answer. The questions and the
//! answers are read from first lines alone (`answer::open_names`, #145): `[questions] <issue>.<role> q1 q2` names
//! every question of its comment, as `[answers]` names every answer. A choice number that is not one of the
//! choices is not seen.

use std::io::Write;

use crate::{answer, issue, prompt, starter};

/// The steps of the flow, in order, as they are worked out. A step is shown to a person as its stage word.
pub const STEPS: [&str; 7] = ["Not planned", "Planned", "Accepted", "Tested", "Built", "Reviewed", "Fixed"];

/// The stage words (#284): the step of a job, as a word and as the letter of the short form. A job not broken down has none.
const STAGES: [(&str, &str, char); 6] = [("Planned", "planned", 'P'), ("Accepted", "accepted", 'A'), ("Tested", "tested", 'T'), ("Built", "built", 'B'), ("Reviewed", "reviewed", 'R'), ("Fixed", "fixed", 'F')];

/// The heading of a job that is done (#282): printed for `wb status <issue>` alone, never in the list, and not a stage.
pub const DONE: &str = "Done";

/// The heading of the jobs whose next move is a person's (#284).
pub const WAITING: &str = "Jobs waiting for you";

/// The heading of the jobs whose next move is an agent's (#284).
pub const FLOW: &str = "Jobs in the flow";

/// The most characters of a reason that a line says before it is cut (#311).
const REASON_LIMIT: usize = 200;

/// The last heading, for an issue that could not be read.
pub const NOT_READ: &str = "Not read";

/// What `--stage` takes besides the stage words: the issues that could not be read.
const NOT_READ_WORD: &str = "not read";

/// The words `--stage` takes, as a list (#284).
pub fn stage_words() -> Vec<&'static str> {
    STAGES.iter().map(|stage| stage.1).chain([NOT_READ_WORD]).collect()
}

/// The word that `word` names, matched without regard to case, or `None` when it is no stage word (#284).
pub fn stage_word_of(word: &str) -> Option<&'static str> {
    stage_words().into_iter().find(|known| known.eq_ignore_ascii_case(word))
}

/// How the jobs are printed (#284): as `git status` is, with its short and its porcelain forms.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    Long,
    Short,
    Porcelain,
}

/// What a listing looks like: its form, and whether it is coloured (never in the porcelain form).
#[derive(Clone, Copy, Debug)]
pub struct Look {
    pub form: Form,
    pub color: bool,
}

/// An issue as it was read.
struct Found {
    number: String,
    /// The branch of the issue in the flow that is shown on its line, or `None` when it is not in the flow.
    branch: Option<String>,
    /// The open changes of that branch.
    changes: Vec<String>,
    /// How many commits of the local branch the branch at `origin` does not hold.
    unpushed: u64,
    /// The title, for `wb status <issue>` alone.
    title: Option<String>,
    step: &'static str,
    questions: Vec<String>,
    /// The role and the question the newest answer that names a role waits for (#226).
    waits: Option<(String, String)>,
    /// The reason of a needs_person that no stamp has come after, as one line, escaped and cut (#311).
    person: Option<String>,
    /// The lines it was worked out from, for `wb status <issue>`.
    from: Vec<String>,
    /// The rounds that are known to have brought nothing back (#300), as the role and the number.
    empty: Vec<(String, u64)>,
    /// The roles whose stamps count, in the order of the stamps.
    roles: Vec<String>,
    /// One line for each acceptance that holds, by its object (#282).
    acceptances: Vec<String>,
    /// How the job was closed, where it was: by the acceptance of the work or by a merge (#282).
    closed: Option<Closed>,
    /// Whether the questions are not known: they are on the tracker (the ref holds no questions commit), and the
    /// pattern that reads it is not there (#238).
    tracker_unread: bool,
    /// Whether the questions are read from the ref and one is open: an answer at the tracker is not read (#238).
    tracker_not_read: bool,
    /// The stamps and markers of the job's ref, for the sessions of `wb status <issue>` (#284).
    ledger: Option<crate::stamps::Ledger>,
    /// Whether anything of `wb` is on the job's ref: a stamp, a marker or a commit of questions or of needs_person, counted
    /// or not (#284). A job so begun has entered, with or without an issue's file.
    started: bool,
    /// Why the job's ref is not as `wb` writes it, where it is not: nothing on it counts (#284).
    broken: Option<String>,
}

impl Found {
    /// The stage of the job as a word and a letter, or none for a job not broken down or one that is done (#284).
    fn stage(&self) -> Option<(&'static str, char)> {
        STAGES.iter().find(|stage| stage.0 == self.step).map(|stage| (stage.1, stage.2))
    }

    /// Why a person must move, as the letter of the short form: `Q` a question is open, `N` a session ended needing a
    /// person, `W` an answer waits for a person, `P` a plan awaits its acceptance (#284). None where an agent moves.
    fn person_move(&self) -> Option<char> {
        if self.closed.is_some() {
            return None;
        }
        if !self.questions.is_empty() {
            Some('Q')
        } else if self.person.is_some() {
            Some('N')
        } else if self.waits.as_ref().is_some_and(|(role, _)| role == "person") {
            Some('W')
        } else if self.step == "Planned" {
            Some('P')
        } else if self.step == "Not planned" && (self.started || self.unpushed > 0) {
            // No issue's file yet, but `wb` has begun the job or work is not pushed: a person must look (#284).
            Some('L')
        } else {
            None
        }
    }

    /// Whether the list has a line for the job (#284): it has a stage or a person to move, and is not done.
    fn listed(&self) -> bool {
        self.closed.is_none() && (self.stage().is_some() || self.person_move().is_some())
    }

    /// The text after the columns: the stage word, and what a person must do or knows (#284).
    fn text(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some((word, _)) = self.stage() {
            parts.push(word.to_string());
        }
        match self.questions.len() {
            0 => {}
            1 => parts.push("1 question open".to_string()),
            n => parts.push(format!("{} questions open", n)),
        }
        if let Some((_, name)) = self.waits.as_ref().filter(|(role, _)| role == "person") {
            parts.push(format!("waits for person ({})", issue::shown(name)));
        }
        if self.person_move() == Some('L') {
            parts.push("no issue's file yet".to_string());
        }
        if self.unpushed > 0 {
            parts.push(format!("{} not pushed", self.unpushed));
        }
        // The reason is last: it may be long.
        if let Some(reason) = &self.person {
            parts.push(if reason.is_empty() { "needs a person".to_string() } else { format!("needs a person: {}", reason) });
        }
        parts.join(", ")
    }
}

/// How a job is done (#282).
struct Closed {
    /// The line that says it, for `wb status <issue>`.
    line: String,
    /// The sentence that says it, for a refusal.
    said: String,
}

/// A branch or a remote branch: how it is read with `git show`, and the name it has.
struct Branch {
    full: String,
    name: String,
}

/// Runs git in `root`, with no replace ref: the files, the acceptances and the commits an acceptance's tag is matched
/// against are what the objects are, never what a replace ref says they are (#235).
pub(crate) fn git(root: &str, args: &[&str]) -> Option<Vec<u8>> {
    let mut all = vec!["--no-replace-objects", "-C", root];
    all.extend(args);
    crate::detect::git_output(&all, None).filter(|run| run.status.success()).map(|run| run.stdout)
}

/// The local branches and those of `origin`, local first.
fn branches(root: &str) -> Vec<Branch> {
    let Some(listed) = git(root, &["for-each-ref", "--format=%(refname)", "refs/heads", "refs/remotes/origin"]) else {
        return Vec::new();
    };
    let listed = String::from_utf8_lossy(&listed).to_string();
    let mut found: Vec<Branch> = Vec::new();
    for full in listed.lines() {
        let name = full.strip_prefix("refs/heads/").or_else(|| full.strip_prefix("refs/remotes/origin/"));
        if let Some(name) = name.filter(|name| *name != "HEAD") {
            found.push(Branch { full: full.to_string(), name: name.to_string() });
        }
    }
    found.sort_by_key(|branch| !branch.full.starts_with("refs/heads/"));
    found
}

/// The ref of the default branch of `origin`: `refs/remotes/origin/HEAD`, else `origin/main`, `main`,
/// `origin/master`, `master`. `None` when none is there.
pub(crate) fn default_branch(root: &str) -> Option<String> {
    let exists = |full: &str| git(root, &["rev-parse", "--verify", "--quiet", &format!("{}^{{commit}}", full)]).is_some();
    if let Some(target) = git(root, &["symbolic-ref", "--quiet", "refs/remotes/origin/HEAD"]) {
        let target = String::from_utf8_lossy(&target).trim().to_string();
        if exists(&target) {
            return Some(target);
        }
    }
    ["refs/remotes/origin/main", "refs/heads/main", "refs/remotes/origin/master", "refs/heads/master"].iter().find(|full| exists(full)).map(|full| full.to_string())
}

/// The patch ids (`git patch-id --stable`) of a diff or of the output of `git log -p`, one for each commit.
fn patch_ids(root: &str, input: Vec<u8>) -> Vec<String> {
    use std::process::{Command, Stdio};
    let Ok(mut child) = Command::new("git").args(["-C", root, "patch-id", "--stable"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() else {
        return Vec::new();
    };
    let Some(mut stdin) = child.stdin.take() else {
        return Vec::new();
    };
    // The input is written from a thread, so that a long one cannot wait for the output to be read.
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&input);
    });
    let output = child.wait_with_output();
    let _ = writer.join();
    match output {
        Ok(run) => String::from_utf8_lossy(&run.stdout).lines().filter_map(|line| line.split_whitespace().next()).map(str::to_string).collect(),
        Err(_) => Vec::new(),
    }
}

/// Whether the whole change of the branch is a commit on the default branch: the patch id of the diff from their
/// merge base to the branch is that of a commit between the merge base and the default branch. This finds a
/// squash merge, whose commit is never an ancestor of the branch. A squash whose diff differs from the
/// branch's is not found.
fn squashed(root: &str, default: &str, branch: &str) -> bool {
    let Some(base) = git(root, &["merge-base", default, branch]) else {
        return false;
    };
    let base = String::from_utf8_lossy(&base).trim().to_string();
    let Some(diff) = git(root, &["diff", "--no-ext-diff", &base, branch]).filter(|diff| !diff.is_empty()) else {
        return false;
    };
    let Some(mine) = patch_ids(root, diff).into_iter().next() else {
        return false;
    };
    let Some(log) = git(root, &["log", "-p", "--no-ext-diff", &format!("{}..{}", base, default)]) else {
        return false;
    };
    patch_ids(root, log).contains(&mine)
}

/// The local branches whose upstream at `origin` is gone, as after `git fetch --prune`.
fn upstream_gone(root: &str) -> Option<Vec<String>> {
    let listed = git(root, &["for-each-ref", "--format=%(refname) %(upstream:track)", "refs/heads"])?;
    Some(String::from_utf8_lossy(&listed).lines().filter_map(|line| line.strip_suffix(" [gone]")).map(str::to_string).collect())
}

/// The refs of the branches in the flow: those whose tip holds a commit that the default branch does not, less
/// the branches of an issue that are done (their upstream is gone, or their whole change is on the default
/// branch). `Err` when git could not list the refs.
pub(crate) fn in_flow(root: &str, default: &str, pattern: &str) -> Result<Vec<String>, String> {
    let fail = || "git could not list the branches, so no issue is read".to_string();
    let listed = git(root, &["for-each-ref", "--no-merged", default, "--format=%(refname)", "refs/heads", "refs/remotes/origin"]).ok_or_else(fail)?;
    let gone = upstream_gone(root).ok_or_else(fail)?;
    let mut refs: Vec<String> = Vec::new();
    for full in String::from_utf8_lossy(&listed).lines() {
        let name = full.strip_prefix("refs/heads/").or_else(|| full.strip_prefix("refs/remotes/origin/")).unwrap_or(full);
        if issue_of(pattern, name).is_some() && (gone.iter().any(|branch| branch == full) || squashed(root, default, full)) {
            continue;
        }
        refs.push(full.to_string());
    }
    Ok(refs)
}

/// How many commits `local` holds that `other` does not.
fn ahead(root: &str, local: &str, other: &str) -> u64 {
    git(root, &["rev-list", "--count", local, &format!("^{}", other)]).and_then(|count| String::from_utf8_lossy(&count).trim().parse().ok()).unwrap_or(0)
}

/// Whether a branch is the issue's: it is the pattern `branch` with the issue filled, or starts with it and a `-`.
pub(crate) fn is_of(pattern: &str, issue: &str, name: &str) -> bool {
    let start = pattern.replace("{issue}", issue);
    pattern.contains("{issue}") && (name == start || name.starts_with(&format!("{}-", start)))
}

/// The issue that a branch is of, by the pattern: what stands where `{issue}` stands, up to the digits' end.
fn issue_of(pattern: &str, name: &str) -> Option<String> {
    let (before, _) = pattern.split_once("{issue}")?;
    let rest = name.strip_prefix(before)?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let after = &rest[digits.len()..];
    (!digits.is_empty() && (after.is_empty() || after.starts_with('-'))).then_some(digits)
}

/// The issue's file and, where its acceptance holds, the text of the acceptance, on the first branch of the issue that
/// has them.
fn on_branches(root: &str, number: &str, pattern: &str, all: &[Branch], from: &mut Vec<String>) -> (bool, Option<String>) {
    let file = format!("{}/{}.yml", issue::FOLDER, number);
    let accepted = format!("{}/{}.accepted", issue::FOLDER, number);
    let mine: Vec<&Branch> = all.iter().filter(|branch| is_of(pattern, number, &branch.name)).collect();
    if mine.is_empty() {
        from.push(format!("no branch of the issue, from the pattern `branch`: {}", pattern));
    }
    let mut has_file = false;
    // The project's keys, read once, where an acceptance is found to hold by its hash (#235).
    let mut keys: Option<crate::signers::Keys> = None;
    for branch in mine {
        let Some(bytes) = git(root, &["show", &format!("{}:{}", branch.full, file)]) else {
            from.push(format!("{}: no {}", branch.full, file));
            continue;
        };
        has_file = true;
        let hash = crate::manifest::hash_bytes(&bytes);
        let body = git(root, &["show", &format!("{}:{}", branch.full, accepted)]).map(|text| String::from_utf8_lossy(&text).to_string());
        let held = body.as_deref().is_some_and(|text| text.lines().find_map(|line| line.strip_prefix("sha256: ")) == Some(hash.as_str()));
        from.push(if held {
            format!("{}: {} and {}, which holds for it", branch.full, file, accepted)
        } else {
            format!("{}: {}, and {} is not there or does not hold for it", branch.full, file, accepted)
        });
        if let (true, Some(text)) = (held, body.as_deref()) {
            let keys = keys.get_or_insert_with(|| crate::signers::load(root));
            match crate::signers::file_signed(root, keys, number, &bytes, text) {
                crate::signers::Signed::Void(why) => from.push(why),
                crate::signers::Signed::Good(said) => {
                    from.push(said);
                    return (true, body);
                }
            }
        }
    }
    (has_file, None)
}

/// Whether the acceptance `body` of the issue's file `bytes` of `number` counts by its signature (#235): it does where
/// its signed tag checks, and never where `.watchbill/signers` is missing (#323).
fn signed_holds(root: &str, number: &str, bytes: &[u8], body: &str) -> bool {
    !matches!(crate::signers::file_signed(root, &crate::signers::load(root), number, bytes, body), crate::signers::Signed::Void(_))
}

/// Where `wb status` reads the issue's file of `number` (#290, for `wb plan show`): the first branch of the issue, by
/// `pattern`, local before `origin`, whose acceptance holds for its file, else the first that holds the file. Its ref,
/// the file, and the text of the acceptance on it, where there is one.
pub(crate) fn file_of_issue(root: &str, number: &str, pattern: &str) -> Option<(String, Vec<u8>, Option<String>)> {
    let file = format!("{}/{}.yml", issue::FOLDER, number);
    let accepted = format!("{}/{}.accepted", issue::FOLDER, number);
    let mut first = None;
    for branch in branches(root).iter().filter(|branch| is_of(pattern, number, &branch.name)) {
        let Some(bytes) = git(root, &["show", &format!("{}:{}", branch.full, file)]) else {
            continue;
        };
        let hash = crate::manifest::hash_bytes(&bytes);
        let body = git(root, &["show", &format!("{}:{}", branch.full, accepted)]).map(|text| String::from_utf8_lossy(&text).to_string());
        if body.as_deref().is_some_and(|text| text.lines().find_map(|line| line.strip_prefix("sha256: ")) == Some(hash.as_str())) {
            return Some((branch.full.clone(), bytes, body));
        }
        if first.is_none() {
            first = Some((branch.full.clone(), bytes, body));
        }
    }
    first
}

/// Whether the acceptance `body` of the issue `number` is void (#290): a source of its plan added after it, or, where
/// `planner` says a `[stamp] planner` counts on the job's ref, one whose time is not before it (#236). What decided it is
/// said in `from`.
fn voided(root: &str, number: &str, body: &str, ledger: &crate::stamps::Ledger, from: &mut Vec<String>) -> Result<bool, String> {
    // On a job's ref that is not as `wb` writes it, a planner round cannot be told, so the acceptance counts as void
    // (the review of #328); the line of the ledger names the commit.
    if ledger.broken.is_some() {
        from.push(format!("the acceptance counts for nothing: nothing on {} counts", ledger.full));
        return Ok(true);
    }
    let planner = ledger.has_planner();
    if crate::plan::sources_after_acceptance(root, number, body) {
        from.push(format!("{}{}: a source added after the acceptance", crate::plan::PREFIX, number));
        return Ok(true);
    }
    if planner && crate::plan::planned_again(root, number, &crate::plan::accepted_time(body))? {
        from.push("a [stamp] planner not before the acceptance: planned again after it".to_string());
        return Ok(true);
    }
    Ok(false)
}

/// The value of the line `<key>: ` of an acceptance's text.
fn value_of<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|line| line.strip_prefix(key))
}

/// The line `accepted: the issue's file ...` of an acceptance that holds, by its object (#282).
fn file_line(body: &str) -> String {
    let hash: String = value_of(body, "sha256: ").unwrap_or("").chars().take(12).collect();
    format!("accepted: the issue's file {} ({}, {})", issue::shown(&hash), issue::shown(value_of(body, "by: ").unwrap_or("")), issue::shown(value_of(body, "time: ").unwrap_or("")))
}

/// The name of a ref as a person writes it: without `refs/heads/` and `refs/remotes/`.
fn short_ref(full: &str) -> &str {
    full.strip_prefix("refs/heads/").or_else(|| full.strip_prefix("refs/remotes/")).unwrap_or(full)
}

/// Whether the default branch holds the issue's file and an acceptance that holds for it.
fn default_holds(root: &str, default: &str, number: &str) -> bool {
    let file = format!("{}/{}.yml", issue::FOLDER, number);
    let accepted = format!("{}/{}.accepted", issue::FOLDER, number);
    let (Some(bytes), Some(body)) = (git(root, &["show", &format!("{}:{}", default, file)]), git(root, &["show", &format!("{}:{}", default, accepted)])) else {
        return false;
    };
    let hash = crate::manifest::hash_bytes(&bytes);
    let body = String::from_utf8_lossy(&body).to_string();
    body.lines().find_map(|line| line.strip_prefix("sha256: ")) == Some(hash.as_str()) && signed_holds(root, number, &bytes, &body)
}

/// How a job is closed, from git alone (#282): the record of the acceptance of its work holds on a branch of the issue,
/// or no branch of the issue is in the flow and the default branch holds the issue's file with its acceptance. The
/// record is read on every branch of the issue, a local one before one at `origin`. A record counts only where its
/// signed tag checks against `.watchbill/signers` of the default branch (#235, #323); why one does not is said in `from`.
fn closing(root: &str, number: &str, pattern: &str, all: &[Branch], flow: &[String], default: &str, from: &mut Vec<String>) -> Option<Closed> {
    let mine = || all.iter().filter(|branch| is_of(pattern, number, &branch.name));
    let work = |record: &crate::finished::Record, on: &str| {
        let said = crate::finished::said(&record.commit, &[]);
        let not_run = if record.not_run.is_empty() { String::new() } else { format!("; not run: {}", issue::shown(&record.not_run.join(", "))) };
        let line = format!("accepted: {} ({}, {}{})", said, issue::shown(&record.by), issue::shown(&record.time), not_run);
        Closed { line, said: format!("its work is accepted ({}, on {})", said, issue::shown(on)) }
    };
    let mut keys: Option<crate::signers::Keys> = None;
    let mut signed = |record: &crate::finished::Record, body: &str, from: &mut Vec<String>| {
        let keys = keys.get_or_insert_with(|| crate::signers::load(root));
        match crate::signers::work_signed(root, keys, number, &record.commit, body) {
            crate::signers::Signed::Void(why) => {
                from.push(why);
                false
            }
            _ => true,
        }
    };
    for branch in mine() {
        if let Some((record, body)) = crate::finished::holding(root, number, &branch.full) {
            if signed(&record, &body, from) {
                return Some(work(&record, &branch.name));
            }
        }
    }
    // The record reached the default branch with the merge of the branch: no branch is needed then, but a branch of the
    // issue in the flow, with a commit after it, puts the job back (#282).
    if !mine().any(|branch| flow.contains(&branch.full)) && default_holds(root, default, number) {
        let body = git(root, &["show", &format!("{}:{}", default, crate::finished::path_of(number))]).map(|text| String::from_utf8_lossy(&text).to_string());
        if let Some((record, body)) = body.and_then(|body| crate::finished::parse(&body).map(|record| (record, body))) {
            if signed(&record, &body, from) {
                return Some(work(&record, short_ref(default)));
            }
        }
    }
    // A branch of the issue is there, none is in the flow, one holds work after the acceptance, and the default branch
    // holds the file with its acceptance. With no branch left, or one with no work, git does not say that the change
    // on the default branch was this issue's.
    if mine().next().is_none() || mine().any(|branch| flow.contains(&branch.full)) || !default_holds(root, default, number) || !mine().any(|branch| has_work(root, number, &branch.full)) {
        return None;
    }
    let into = short_ref(default);
    Some(Closed { line: format!("merged: into {}", issue::shown(into)), said: format!("its change is merged into {}", issue::shown(into)) })
}

/// Whether the branch `full` holds work after the commit that added the acceptance: a path other than the issue's own
/// files changes between that commit and the branch's tip. Trees are compared, so a squash does not hide it.
fn has_work(root: &str, number: &str, full: &str) -> bool {
    let accepted = format!("{}/{}.accepted", issue::FOLDER, number);
    let Some(added) = git(root, &["log", "--diff-filter=A", "--format=%H", "-1", full, "--", &accepted]) else { return false };
    let added = String::from_utf8_lossy(&added).trim().to_string();
    if added.is_empty() {
        return false;
    }
    let own: Vec<String> = ["yml", "accepted", "finished"].iter().map(|end| format!("{}/{}.{}", issue::FOLDER, number, end)).collect();
    let Some(names) = git(root, &["diff", "--name-only", "--no-renames", "-z", &added, full]) else { return false };
    names.split(|byte| *byte == 0).filter(|name| !name.is_empty()).any(|name| !own.iter().any(|one| one.as_bytes() == name))
}

/// Why the job `number` is done, in words, or none where it is not or git cannot say (#282): for `wb revert` and for
/// the views that `wb clean` removes.
pub(crate) fn done_of(root: &str, number: &str) -> Option<String> {
    let pattern = prompt::pattern_of(root, "branch").ok().flatten().unwrap_or_else(|| "issue-{issue}".to_string());
    let default = default_branch(root)?;
    let flow = in_flow(root, &default, &pattern).ok()?;
    closing(root, number, &pattern, &branches(root), &flow, &default, &mut Vec::new()).map(|closed| closed.said)
}

/// Whether the commit `commit` (as the views name it) is in the default branch: git says it is an ancestor of it.
pub(crate) fn in_default(root: &str, commit: &str) -> bool {
    if !crate::finished::is_object(commit) {
        return false;
    }
    let Some(default) = default_branch(root) else { return false };
    crate::detect::git_output(&["--no-replace-objects", "-C", root, "merge-base", "--is-ancestor", &format!("{}^{{commit}}", commit), &default], None).is_some_and(|run| run.status.code() == Some(0))
}

/// What `wb accept` needs to know of a job (#282).
pub(crate) struct Inspected {
    /// The roles whose stamps count: after the last planner, not taken back, a round that brought nothing back left out.
    pub roles: Vec<String>,
    /// The questions no answer has closed.
    pub questions: Vec<String>,
}

/// Reads the job `number` as `wb status` does, for `wb accept`: `Err` says why it cannot be read.
pub(crate) fn inspect(root: &str, number: &str, state: Option<&std::path::Path>) -> Result<Inspected, String> {
    let get = |key: &str| prompt::pattern_of(root, key).map_err(|why| why.to_string());
    let branch = get("branch")?.unwrap_or_else(|| "issue-{issue}".to_string());
    let comments = get("issue_comments")?;
    let patterns = Patterns { branch, comments, change_of_branch: None, issue_address: None };
    let default = default_branch(root).ok_or_else(|| "no default branch was found".to_string())?;
    let flow = in_flow(root, &default, &patterns.branch)?;
    let starts = state.map(crate::kept::Starts::read);
    let found = read_issue(root, number, &patterns, &branches(root), &flow, &default, false, starts.as_ref())?;
    // A job whose ref holds no stamp has its questions on the tracker, and they are not known without the pattern
    // (#238); one whose ref holds a stamp has them on the ref (#333).
    if found.tracker_unread {
        return Err("watchbill.yml has no `issue_comments`, so the roles that ran and the questions are not known".to_string());
    }
    Ok(Inspected { roles: found.roles, questions: found.questions })
}

/// Reads one issue, or says why it could not be read.
fn read_issue(root: &str, number: &str, patterns: &Patterns, all: &[Branch], flow: &[String], default: &str, with_title: bool, starts: Option<&crate::kept::Starts>) -> Result<Found, String> {
    let (comments, pattern) = (patterns.comments.as_deref(), patterns.branch.as_str());
    let mut from = Vec::new();
    let mut lines = Vec::new();
    // The stamps are the commits on the job's ref that count (#236), with what a marker took back left out; a `[stamp]`
    // comment is a copy, and decides nothing.
    let ledger = crate::stamps::read(root, number)?;
    // Where the ref holds a stamp (#333) or a questions commit (#238), the questions and the answers are on the ref too,
    // and the tracker is not read for them: no pattern is run. Else they are read from the tracker, as for a job begun
    // before.
    let on_ref = ledger.on_ref();
    if let (Some(command), false) = (comments, on_ref) {
        if !prompt::is_plain_name(number) {
            return Err(format!("{} is not a plain name that fills `issue_comments`", issue::shown(number)));
        }
        let printed = starter::run(root, "issue_comments", &command.replace("{issue}", number), None)?;
        lines = String::from_utf8_lossy(&printed).lines().map(|line| line.trim_end_matches('\r').to_string()).collect();
    }
    let stamps = ledger.stamps();
    // What a `[revert]` marker took back (#142) counts for no question: on the ref, the markers in its order; at the
    // tracker, the `[stamp]` and `[revert]` lines of the comments.
    let counted = if on_ref { starter::counted(&ledger.lines()) } else { starter::counted(&lines.join("\n")) };
    let (file, acceptance) = on_branches(root, number, pattern, all, &mut from);
    // An acceptance is void after a source added to the plan, or a planner round, later than it (#290).
    let accepted = match &acceptance {
        None => false,
        Some(body) => !voided(root, number, body, &ledger, &mut from)?,
    };
    let mut acceptances: Vec<String> = acceptance.iter().filter(|_| accepted).map(|body| file_line(body)).collect();
    // The branch that is shown: the first of the issue's branches in the flow, a local one before one at `origin`.
    let shown = all.iter().find(|branch| is_of(pattern, number, &branch.name) && flow.contains(&branch.full));
    let (mut changes, mut unpushed) = (Vec::new(), 0);
    let mut not_in_flow = None;
    match shown {
        None => not_in_flow = Some(format!("not in the flow: no branch of the issue holds a commit that {} does not", default)),
        Some(branch) => {
            if branch.full.starts_with("refs/heads/") {
                let at_origin = format!("refs/remotes/origin/{}", branch.name);
                let against = if all.iter().any(|other| other.full == at_origin) { at_origin } else { default.to_string() };
                unpushed = ahead(root, &branch.full, &against);
            }
            if let Some(command) = &patterns.change_of_branch {
                if !prompt::is_plain_name(&branch.name) {
                    return Err(format!("the branch {} is not a plain name that fills `change_of_branch`", issue::shown(&branch.name)));
                }
                changes = starter::open_changes(root, &command.replace("{branch}", &branch.name))?;
            }
        }
    }
    if let Some(line) = not_in_flow {
        from.insert(0, line);
    }
    let title = match (&patterns.issue_address, with_title) {
        (Some(command), true) => {
            let printed = starter::run(root, "issue_address", &command.replace("{issue}", number), None)?;
            String::from_utf8_lossy(&printed).lines().next().map(|line| line.trim_end_matches('\r').to_string())
        }
        _ => None,
    };
    // A new planner takes the issue back: only the stamps after the last `[stamp] planner` decide the step.
    let since = stamps.iter().rposition(|(name, _)| name == "planner").map_or(0, |at| at + 1);
    // A round of a role whose work is commits counts only where something of it came back (#300): the round that the
    // start on this machine recorded as bringing nothing back is left out. A round not recorded counts by its stamp.
    let mut empty = starts.map(|starts| starts.empty_rounds(number)).unwrap_or_default();
    // Only a round whose stamp still counts is said: not one before the last planner, nor one a `[revert]` took back.
    empty.retain(|round| stamps[since..].iter().any(|(role, number)| *role == round.0 && *number == round.1));
    let stamps: Vec<(String, u64)> = stamps[since..].iter().filter(|(role, round)| !empty.iter().any(|(name, number)| name == role && number == round)).cloned().collect();
    let has = |role: &str| stamps.iter().any(|(name, _)| name == role);
    let mut roles: Vec<String> = Vec::new();
    for (name, _) in &stamps {
        if !roles.contains(name) {
            roles.push(name.clone());
        }
    }
    let closed = closing(root, number, pattern, all, flow, default, &mut from);
    if let Some(closed) = &closed {
        from.push(format!("done: {}", closed.said));
        if closed.line.starts_with("accepted: ") {
            acceptances.push(closed.line.clone());
        }
    }
    let step = match stamps.last().map(|(name, _)| name.as_str()) {
        _ if closed.is_some() => DONE,
        Some("fix") => "Fixed",
        Some("review") => "Reviewed",
        _ if has("build") => "Built",
        _ if has("tester") => "Tested",
        _ if accepted => "Accepted",
        _ if file => "Planned",
        _ => "Not planned",
    };
    // The stamps and the markers on the job's ref, each with whether it counts (#236); of the comments, the questions and
    // the answers. A `[stamp]` or `[revert]` comment is a copy posted out, and is not read.
    from.extend(ledger.said());
    let gone = starter::taken_back(&lines.join("\n"));
    for (line, gone) in lines.iter().zip(gone) {
        if ["[question] ", "[questions] ", "[answer] ", "[answers] "].iter().any(|start| line.starts_with(start)) {
            from.push(if gone { format!("{}  (taken back by a [revert] marker)", issue::shown(line)) } else { issue::shown(line) });
        }
    }
    // Where `.watchbill/signers` is missing, why nothing counts is said once (#323), however many things it voids.
    if let Some(missing) = &ledger.missing {
        let mut said = false;
        from.retain(|line| line != missing || !std::mem::replace(&mut said, true));
    }
    let questions = answer::open_names(number, &counted);
    let waits = answer::waits_for(number, &counted);
    // Only the job's ref says it, never a comment; a job that is done waits for no one (#311).
    let person = ledger.person().filter(|_| closed.is_none()).map(|reason| reason_line(&reason));
    let tracker_unread = !on_ref && comments.is_none();
    let tracker_not_read = on_ref && !questions.is_empty();
    let (started, broken) = (!ledger.entries.is_empty() || ledger.broken.is_some(), ledger.broken.clone());
    Ok(Found {
        number: number.to_string(),
        branch: shown.map(|branch| branch.name.clone()),
        changes,
        unpushed,
        title,
        step,
        questions,
        waits,
        person,
        from,
        empty,
        roles,
        acceptances,
        closed,
        tracker_unread,
        tracker_not_read,
        ledger: with_title.then_some(ledger),
        started,
        broken,
    })
}

/// A reason as one line (#311): each run of white space as one space, escaped as every text of `wb status` is, and cut
/// so that the line is at most `REASON_LIMIT` characters as printed, the `...` among them. The cut is made between the
/// escaped characters, never inside an escape.
fn reason_line(reason: &str) -> String {
    let one: String = reason.split_whitespace().collect::<Vec<_>>().join(" ");
    let pieces: Vec<String> = one.chars().map(|c| issue::shown(&c.to_string())).collect();
    if pieces.iter().map(|piece| piece.chars().count()).sum::<usize>() <= REASON_LIMIT {
        return pieces.concat();
    }
    let mut kept = String::new();
    let mut length = 0;
    for piece in &pieces {
        let more = piece.chars().count();
        if length + more > REASON_LIMIT - 3 {
            break;
        }
        kept.push_str(piece);
        length += more;
    }
    format!("{}...", kept)
}

/// The patterns of `watchbill.yml` that `wb status` reads.
struct Patterns {
    branch: String,
    comments: Option<String>,
    change_of_branch: Option<String>,
    issue_address: Option<String>,
}

/// Numbers in the order of their value, then the other names in the order of their letters.
fn order(number: &str) -> (u8, u64, String) {
    match number.parse::<u64>() {
        Ok(value) => (0, value, String::new()),
        Err(_) => (1, 0, number.to_string()),
    }
}
/// `wb status` only reads (#254): a file that is not as `HEAD` has it is said, and every pattern is read from the file of
/// `HEAD`. Where the file is not in `HEAD` at all, the repository is not set up: 2, and no refusal; a file of `HEAD` over
/// its bound is a limit (#261), 1. Shared with `wb status --session` (#239).
pub(crate) fn read_only(root: &str, err: &mut dyn Write) -> Result<starter::ReadOnly, i32> {
    match starter::read_only(root) {
        Err(not_read) if not_read.status == 2 => {
            let _ = writeln!(err, "wb status: {}.", not_read.why);
            Err(not_read.status)
        }
        Err(not_read) => Err(crate::refusal::limit(err, "wb status", format_args!("{}.", not_read.why))),
        Ok((guard, differs)) => {
            if differs {
                let _ = writeln!(err, "{}", starter::drift_line("wb status"));
            }
            Ok(guard)
        }
    }
}

/// `wb status [<issue>] [--stage <word>]` in the repository at `root`: 0 every issue could be read, 1 a
/// pattern failed, 2 the repository is not set up or has no default branch. `stage`, when given, is a word that
/// `stage_word_of` knows. `look` says the form and the colour (#284).
pub fn status(root: &str, only: Option<&str>, stage: Option<&'static str>, look: Look, state: Option<&std::path::Path>, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let _read_only = match read_only(root, err) {
        Ok(guard) => guard,
        Err(code) => return code,
    };
    // The hints are on unless `status_hints: false` is written in `watchbill.yml` (#284).
    let hints = match hints_on(root) {
        Ok(on) => on,
        Err(why) => {
            // A value of `watchbill.yml` that the loader refuses ends with 1, as for `wb check` (#284).
            let _ = writeln!(err, "wb status: {}", why);
            crate::refusal::answer();
            return 1;
        }
    };
    let pattern = |key: &str| prompt::pattern_of(root, key);
    // `issue_address` is read for one issue only: the list never runs it.
    let address = if only.is_some() { pattern("issue_address") } else { Ok(None) };
    let patterns = match (pattern("branch"), pattern("issue_comments"), pattern("change_of_branch"), address) {
        (Ok(branch), Ok(comments), Ok(change_of_branch), Ok(issue_address)) => {
            Patterns { branch: branch.unwrap_or_else(|| "issue-{issue}".to_string()), comments, change_of_branch, issue_address }
        }
        (Err(why), ..) | (_, Err(why), ..) | (_, _, Err(why), _) | (.., Err(why)) => {
            let _ = writeln!(err, "wb status: {}", why);
            return 2;
        }
    };
    let Some(default) = default_branch(root) else {
        let _ = writeln!(err, "wb status: no default branch was found: not refs/remotes/origin/HEAD, nor main nor master, at origin or here. The issues in the flow are those whose branch holds a commit that it does not.");
        return 2;
    };
    let all = branches(root);
    let flow = match in_flow(root, &default, &patterns.branch) {
        Ok(flow) => flow,
        Err(why) => {
            let _ = writeln!(err, "wb status: {}", why);
            return 2;
        }
    };
    let mut issues: Vec<String> = match only {
        Some(one) => vec![one.to_string()],
        None => flow.iter().filter_map(|full| full.strip_prefix("refs/heads/").or_else(|| full.strip_prefix("refs/remotes/origin/"))).filter_map(|name| issue_of(&patterns.branch, name)).collect(),
    };
    issues.sort_by_key(|number| order(number));
    issues.dedup();

    let mut failed: Vec<(String, String)> = Vec::new();
    let mut read: Vec<Found> = Vec::new();
    // The folders of the starts are read once for every issue (#300).
    let starts = state.map(crate::kept::Starts::read);
    for number in issues {
        match read_issue(root, &number, &patterns, &all, &flow, &default, only.is_some(), starts.as_ref()) {
            // A job whose ref is not as `wb writes it is not read: nothing on it counts, and it is never left out (#284).
            Ok(found) if only.is_none() && found.broken.is_some() => failed.push((issue::shown(&number), issue::shown(found.broken.as_deref().unwrap_or("")))),
            Ok(found) => read.push(found),
            Err(why) => failed.push((issue::shown(&number), issue::shown(&why))),
        }
    }

    // Without `issue_comments`, a job whose ref holds no stamp has no question or answer read (#333): said once. A job
    // whose ref holds a stamp is read from its ref, and there is nothing to say of it.
    if patterns.comments.is_none() && read.iter().any(|found| found.tracker_unread) {
        let _ = writeln!(err, "wb status: watchbill.yml has no `issue_comments`, so no question or answer is read from the tracker for a job whose ref holds no stamp; the questions and answers of a job whose ref holds a stamp are read from its ref.");
    }
    // An open question that is read from the job's ref: an answer typed at the tracker is not read (#238).
    for found in read.iter().filter(|found| found.tracker_not_read && (only.is_some() || found.closed.is_none())) {
        let shown = issue::shown(&found.number);
        let _ = writeln!(err, "wb status: issue {} has an open question: an answer typed at the tracker is not read, and is given with `wb answer {}`.", shown, shown);
    }
    // A job that is done is not listed (#282): only `wb status <issue>`, asked by name, says it. A job not broken down has
    // no line, unless a person's move is open on it (#284).
    if only.is_none() {
        read.retain(|found| found.listed());
    }
    let wanted = |found: &Found| stage.map_or(true, |word| found.stage().is_some_and(|(shown, _)| shown == word));
    let jobs: Vec<&Found> = read.iter().filter(|found| wanted(found)).collect();
    let show_failed = stage.map_or(true, |word| word == NOT_READ_WORD);
    let failed_code = if failed.is_empty() {
        0
    } else {
        // The issue is listed under Not read, and the rest is printed: not a refusal.
        crate::refusal::answer();
        1
    };
    // The short and the porcelain forms: one line for each job, and nothing else on standard output (#284).
    if look.form != Form::Long {
        let color = look.color && look.form == Form::Short;
        let mut text = String::new();
        for found in jobs.iter().filter(|found| only.is_none() || (found.listed() && found.branch.is_some())) {
            text.push_str(&short_line(found, color));
        }
        if show_failed {
            for (number, why) in &failed {
                text.push_str(&if color { format!("\x1b[31m**\x1b[m {}\n", number) } else { format!("** {}\n", number) });
                let _ = writeln!(err, "wb status: issue {} was not read: {}", number, why);
            }
        }
        let _ = out.write_all(text.as_bytes());
        kept_count(root, only, state, err);
        return failed_code;
    }
    for (number, why) in failed.iter().filter(|_| show_failed) {
        let _ = writeln!(err, "wb status: issue {} was not read: {}", number, why);
    }
    let mut sections: Vec<String> = Vec::new();
    // By stage, then by number: the jobs with no stage first (#284).
    let mut jobs = jobs;
    jobs.sort_by_key(|found| (STAGES.iter().position(|stage| stage.0 == found.step).map_or(0, |at| at + 1), order(&found.number)));
    let widths = Widths::of(&jobs);
    for (heading, code) in [(WAITING, "31"), (FLOW, "32"), (DONE, "")] {
        let here: Vec<&&Found> = jobs
            .iter()
            .filter(|found| match heading {
                WAITING => found.closed.is_none() && found.person_move().is_some(),
                FLOW => found.closed.is_none() && found.person_move().is_none(),
                _ => found.closed.is_some(),
            })
            .collect();
        if here.is_empty() {
            continue;
        }
        let mut text = format!("{}:\n", heading);
        if hints && heading == WAITING {
            if here.iter().any(|found| found.step == "Planned") {
                text.push_str("  (use \"wb plan accept <name>...\" to accept the plan)\n");
            }
            if here.iter().any(|found| !found.questions.is_empty()) {
                text.push_str("  (use \"wb answer <issue>...\" to answer its questions)\n");
            }
        }
        for found in here {
            let row = widths.row(found);
            if look.color && !code.is_empty() {
                text.push_str(&format!("\t\x1b[{}m{}\x1b[m\n", code, row));
            } else {
                text.push_str(&format!("\t{}\n", row));
            }
        }
        sections.push(text);
    }
    // One issue: each acceptance that holds, by its object, and the merge that closed it (#282).
    if let (Some(_), None, Some(found)) = (only, stage, read.first()) {
        let mut lines: Vec<String> = found.acceptances.clone();
        lines.extend(found.closed.iter().map(|closed| closed.line.clone()).filter(|line| line.starts_with("merged: ")));
        if !lines.is_empty() {
            sections.push(lines.iter().map(|line| format!("{}\n", line)).collect());
        }
    }
    // One issue: each session, from its stamp on the job's ref and the lines of its start on this machine (#284).
    if let (Some(_), None, Some(Found { ledger: Some(ledger), .. })) = (only, stage, read.first()) {
        let lines = crate::session_status::sessions(ledger, state);
        if !lines.is_empty() {
            sections.push(format!("Sessions:\n{}", lines.iter().map(|line| format!("{}\n", line)).collect::<String>()));
        }
    }
    // One issue: a source that changed since `wb plan add` took it in is said, and nothing is taken in (#240). A pattern
    // that fails is said on standard error, and the status is the same.
    if let (Some(number), None) = (only, stage) {
        match crate::plan::changed_source(root, number) {
            Ok(Some(line)) => sections.push(format!("{}\n", line)),
            Ok(None) => {}
            Err(why) => {
                let _ = writeln!(err, "wb status: the source of {} was not looked at: {}", issue::shown(number), issue::shown(&why));
            }
        }
    }
    if show_failed && !failed.is_empty() {
        let lines: String = failed.iter().map(|(number, why)| format!("  {}  {}\n", number, why)).collect();
        sections.push(format!("{}:\n{}", NOT_READ, lines));
    }
    // One issue: the names of its open questions, in the order they were asked (#284). It is a fact about the issue.
    if let (Some(_), None, Some(found)) = (only, stage, read.first()) {
        if !found.questions.is_empty() {
            sections.push(format!("questions open: {}\n", found.questions.iter().map(|name| issue::shown(name)).collect::<Vec<_>>().join(", ")));
        }
    }
    // One issue: who the newest answer waits for (#226). It is a fact about the issue, not a command.
    if let (Some(_), None, Some(Found { waits: Some((role, name)), .. })) = (only, stage, read.first()) {
        sections.push(format!("waits for: {} ({})\n", issue::shown(role), issue::shown(name)));
    }
    // One issue: a session ended needing a person, and nothing has started since (#311).
    if let (Some(_), None, Some(Found { person: Some(reason), .. })) = (only, stage, read.first()) {
        sections.push(format!("waits for: person ({})\n", reason));
    }
    // One issue: the lines it was worked out from, below what was printed of it.
    if let (Some(_), false, Some(found)) = (only, sections.is_empty(), read.first()) {
        let lines: String = found.from.iter().map(|line| format!("  {}\n", line)).collect();
        sections.push(format!("Worked out from:\n{}", lines));
    }
    // What the sessions of this machine left (#222): the views kept with work not taken back, and the sessions cut
    // off, of one issue below what was printed of it; for the list, only the count of the views, on standard error.
    // Only the state folder of this machine and the objects of a view are read, and the view's settings never.
    if let Some(state) = state {
        match (only, stage) {
            (Some(number), None) => {
                let kept = crate::kept::scan(root, state, number);
                // A round that brought nothing back is said, with the view kept for it where there is one (#300).
                // How to start again over a view that was read (#284), as a hint, where the hints are on.
                let hint = |view: &crate::kept::Kept| {
                    let said = format!("  (use \"wb start {} {} --ignore-kept\" to start again, or remove the folder)", issue::shown(&view.role), issue::shown(number));
                    (hints && view.unread.is_none()).then_some(said)
                };
                let mut lines: Vec<String> = Vec::new();
                for (role, round) in read.first().map(|found| found.empty.iter()).into_iter().flatten() {
                    let mut line = format!("{} round {}: no commit came back", issue::shown(role), round);
                    let view = kept.iter().find(|view| view.role == *role && view.round == round.to_string());
                    if let Some(view) = view {
                        line.push_str(&format!("; kept: {}", view.detail()));
                    }
                    lines.push(line);
                    lines.extend(view.and_then(hint));
                }
                // A view named beside its round is not named again on a `kept:` line.
                let named = |view: &crate::kept::Kept| read.first().is_some_and(|found| found.empty.iter().any(|(role, round)| view.role == *role && view.round == round.to_string()));
                for view in kept.iter().filter(|view| !named(view)) {
                    lines.push(view.line());
                    lines.extend(hint(view));
                }
                lines.extend(crate::kept::cut_off(state, number).iter().map(|cut| cut.line()));
                // A session that runs, with how long it has run and been quiet (#267).
                lines.extend(crate::kept::running(root, state, number).iter().map(|running| running.line()));
                if !lines.is_empty() {
                    sections.push(lines.iter().map(|line| format!("{}\n", line)).collect());
                }
            }
            _ => {}
        }
    }
    kept_count(root, only, state, err);
    let _ = out.write_all(sections.join("\n").as_bytes());
    failed_code
}

/// For the list: how many views with work not taken back are kept on this machine, on standard error (#222).
fn kept_count(root: &str, only: Option<&str>, state: Option<&std::path::Path>, err: &mut dyn Write) {
    let (None, Some(state)) = (only, state) else { return };
    let count = crate::kept::scan_all(root, state).len();
    if count > 0 {
        let _ = writeln!(err, "wb status: {} kept {} with work not taken back on this machine. `wb status <issue>` names them.", count, if count == 1 { "view" } else { "views" });
    }
}

/// The fault of `status_hints` in `watchbill.yml`, for `wb check`; none where it holds or is not there (#284).
pub fn hints_fault(document: &crate::yamlfile::Document) -> Option<String> {
    let place = [crate::yamlfile::Part::from("status_hints")];
    document.has(&place).then(|| document.boolean(&place).err().map(|why| why.to_string())).flatten()
}

/// Whether the hints are on (#284): they are, unless `status_hints: false` is written in `watchbill.yml`. Only `true` and
/// `false` are values of it.
fn hints_on(root: &str) -> Result<bool, String> {
    let document = starter::settings(root)?;
    let place = [crate::yamlfile::Part::from("status_hints")];
    if !document.has(&place) {
        return Ok(true);
    }
    document.boolean(&place).map_err(|why| why.to_string())
}

/// The line of a job in the short and the porcelain forms (#284): two columns as git has them, the stage letter and why a
/// person must move, then the number and the branch. Coloured as git colours them: the first column green, the second red.
fn short_line(found: &Found, color: bool) -> String {
    let column = |letter: Option<char>, code: &str| match letter {
        None => " ".to_string(),
        Some(letter) if color => format!("\x1b[{}m{}\x1b[m", code, letter),
        Some(letter) => letter.to_string(),
    };
    let mut line = format!("{}{} {}", column(found.stage().map(|stage| stage.1), "32"), column(found.person_move(), "31"), issue::shown(&found.number));
    if let Some(branch) = &found.branch {
        line.push(' ');
        line.push_str(&issue::shown(branch));
    }
    line.push('\n');
    line
}

/// How wide the columns of the long form are: as wide as the widest job on either heading (#284). A column no job has is left out.
struct Widths {
    number: usize,
    branch: usize,
    change: usize,
}

impl Widths {
    fn of(jobs: &[&Found]) -> Widths {
        let widest = |of: &dyn Fn(&Found) -> String| jobs.iter().map(|found| of(found).chars().count()).max().unwrap_or(0);
        Widths { number: widest(&|found| issue::shown(&found.number)), branch: widest(&|found| found.branch.as_deref().map(issue::shown).unwrap_or_default()), change: widest(&changes_of) }
    }

    /// The line of a job, without its indent: the columns, the text, and the title last where there is one.
    fn row(&self, found: &Found) -> String {
        let pad = |text: String, width: usize| format!("{}{}", text, " ".repeat(width.saturating_sub(text.chars().count())));
        let mut columns = vec![pad(issue::shown(&found.number), self.number)];
        if self.branch > 0 {
            columns.push(pad(found.branch.as_deref().map(issue::shown).unwrap_or_default(), self.branch));
        }
        if self.change > 0 {
            columns.push(pad(changes_of(found), self.change));
        }
        let text = found.text();
        if !text.is_empty() {
            columns.push(text);
        }
        if let Some(title) = &found.title {
            columns.push(issue::shown(title));
        }
        columns.join("  ").trim_end().to_string()
    }
}

/// The open changes of a job, as `#<n>`.
fn changes_of(found: &Found) -> String {
    found.changes.iter().map(|change| format!("#{}", issue::shown(change))).collect::<Vec<_>>().join(" ")
}
