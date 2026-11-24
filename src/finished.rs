//! The final acceptance (#282): `wb accept <name>` where the issue's file is accepted accepts the work. A person says
//! that the work is as wanted, at any stage after a role has committed; the job is then done.
//!
//! The record is `.watchbill/issues/<issue>.finished`, beside `.accepted`, committed alone on the issue's branch. It
//! names the head commit of the branch, who accepted and when, and the roles that did not run. It holds while the tip
//! of the branch is the one commit that added it, whose parent is the commit it names: a later commit voids it. A
//! signed tag beside it is #235's.
//!
//! The command refuses where a limit is not passed: an open question, a kept view holding work, no role that has
//! committed work, a change that touches classed code (a person is the verifier, `classed_code.verifier` of the rule
//! table), a checkout off the branch of the issue.

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::issue::{self, FOLDER};

/// The path of the record of `issue`, from the root of the repository.
pub fn path_of(issue: &str) -> String {
    format!("{}/{}.finished", FOLDER, issue)
}

/// What the record says.
#[derive(Debug, PartialEq, Eq)]
pub struct Record {
    pub commit: String,
    pub by: String,
    pub time: String,
    pub not_run: Vec<String>,
}

pub(crate) fn is_object(text: &str) -> bool {
    matches!(text.len(), 40 | 64) && text.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// The record from its text, or none where a line it needs is not there.
pub fn parse(text: &str) -> Option<Record> {
    let value = |key: &str| text.lines().find_map(|line| line.strip_prefix(key)).map(str::to_string);
    let commit = value("commit: ").filter(|commit| is_object(commit))?;
    let not_run = value("not run: ").map(|roles| roles.split(", ").map(str::to_string).collect()).unwrap_or_default();
    Some(Record { commit, by: value("by: ")?, time: value("time: ")?, not_run })
}

/// The words that say the acceptance, as `wb accept` says it first and `wb status` says it: the commit by its first 12
/// digits, and the roles that did not run.
pub fn said(commit: &str, not_run: &[String]) -> String {
    let short: String = commit.chars().take(12).collect();
    if not_run.is_empty() {
        format!("the work {}", short)
    } else {
        format!("the work {} (not run: {})", short, not_run.join(", "))
    }
}

/// The record that holds on the ref `full`: it is committed there, and the tip is the one commit after the commit it
/// names, which changed the record alone.
pub fn holding(root: &str, issue: &str, full: &str) -> Option<Record> {
    let path = path_of(issue);
    let body = crate::status::git(root, &["show", &format!("{}:{}", full, path)])?;
    let record = parse(&String::from_utf8_lossy(&body))?;
    let listed = crate::status::git(root, &["rev-list", &format!("{}..{}", record.commit, full)])?;
    let listed = String::from_utf8_lossy(&listed).to_string();
    let commits: Vec<&str> = listed.lines().collect();
    let [tip] = commits.as_slice() else { return None };
    let parent = crate::status::git(root, &["rev-parse", &format!("{}^", tip)])?;
    if String::from_utf8_lossy(&parent).trim() != record.commit {
        return None;
    }
    let names = crate::status::git(root, &["diff-tree", "--no-commit-id", "--name-only", "-r", "--root", tip])?;
    (String::from_utf8_lossy(&names).lines().collect::<Vec<_>>() == [path.as_str()]).then_some(record)
}

fn commit_message(issue: &str) -> String {
    format!("Accept the work ({})", issue)
}

/// `wb accept <issue>` where the issue's file is accepted: 0 the work is accepted, 1 it is refused, 2 Watchbill is not
/// set up here. It says first which acceptance it writes, and never writes over a record.
pub fn accept(root: &str, issue: &str, commit: bool, state: Option<&Path>, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    const NAME: &str = "wb accept";
    let base = Path::new(root);
    if let Some(why) = issue::way_fault(base) {
        return crate::refusal::limit(err, NAME, format_args!("{}. Nothing read, nothing written.", why));
    }
    if crate::bounded::is_absent(&base.join(".watchbill").join("watchbill.yml")) {
        let _ = writeln!(err, "{}: .watchbill/watchbill.yml is not there. Watchbill is not set up in this repository.", NAME);
        return 2;
    }
    if let Err(why) = crate::starter::held(root) {
        return crate::refusal::limit(err, NAME, format_args!("{}. Nothing read, nothing written.", why));
    }
    let path = path_of(issue);
    let record = base.join(&path);
    if fs::symlink_metadata(&record).is_ok() {
        return crate::refusal::limit(err, NAME, format_args!("{} is there already, and is never written over. Nothing written. Where a later commit voided it, remove it in a commit of its own and run wb accept again.", path));
    }
    if let Err(not_ready) = issue::ready_to_commit(root, issue) {
        let (issue::NotReady::Fault(why) | issue::NotReady::Guard(why)) = not_ready;
        return crate::refusal::limit(err, NAME, format_args!("{} Nothing written. The work is accepted on the branch of the issue.", why));
    }
    let Some(branch) = issue::current_branch(root) else {
        return crate::refusal::limit(err, NAME, format_args!("the checkout is on no branch. Nothing written."));
    };
    match crate::status::git(root, &["status", "--porcelain", "--untracked-files=no"]) {
        Some(changed) if changed.is_empty() => {}
        Some(_) => {
            return crate::refusal::limit(err, NAME, format_args!("a file of the checkout differs from HEAD, and the work is accepted as the history has it. Commit it or put it away. Nothing written."));
        }
        None => return crate::refusal::limit(err, NAME, format_args!("git could not say what the checkout holds. Nothing written.")),
    }
    let Some(head) = crate::status::git(root, &["rev-parse", "HEAD"]).map(|head| String::from_utf8_lossy(&head).trim().to_string()).filter(|head| is_object(head)) else {
        return crate::refusal::limit(err, NAME, format_args!("git gave no HEAD. Nothing written."));
    };
    let seen = match crate::status::inspect(root, issue, state) {
        Ok(seen) => seen,
        Err(why) => return crate::refusal::limit(err, NAME, format_args!("{}. Nothing written.", why)),
    };
    if !seen.roles.iter().any(|role| ["tester", "build", "fix"].contains(&role.as_str())) {
        return crate::refusal::limit(
            err,
            NAME,
            format_args!(
                "no role has committed work on the branch {} (no tester, build or fix stamp counts for issue {}), so there is no work to accept. Nothing written.",
                issue::shown(&branch),
                issue::shown(issue)
            ),
        );
    }
    if !seen.questions.is_empty() {
        let said: String = seen.questions.iter().map(|name| format!("\n  - {}", issue::shown(name))).collect();
        return crate::refusal::limit(err, NAME, format_args!("issue {} has a question that no answer has closed. Nothing written. Answer them first:{}", issue::shown(issue), said));
    }
    if let Some(state) = state {
        let found = crate::kept::scan(root, state, issue);
        if !found.is_empty() {
            return crate::refusal::limit(err, NAME, format_args!("{}\nNothing written.", crate::kept::refusal(issue, &found)));
        }
    }
    let classed = match crate::starter::classed(root) {
        Ok(classed) => classed,
        Err(why) => return crate::refusal::limit(err, NAME, format_args!("{}. Nothing written.", why)),
    };
    // The rule table of revision 1 has one rule on classed code, WB-RULE-5: a person is the verifier. This follows it by
    // hand until the table has more than one rule.
    let touched = crate::prompt::touched_text(root, &classed, Some(&branch), Some(issue));
    if touched != "none" {
        return crate::refusal::limit(
            err,
            NAME,
            format_args!(
                "the change of the branch {} touches classed code, or whether it does cannot be told, and the rule table needs a person as verifier (`classed_code.verifier` of the rule table), which no accept passes: {}. Nothing written.",
                issue::shown(&branch),
                touched.replace('\n', ", ")
            ),
        );
    }
    let (user, email) = match issue::git_value(root, "user.name").and_then(|name| issue::git_value(root, "user.email").map(|email| (name, email))) {
        Ok(identity) => identity,
        Err(why) => return crate::refusal::limit(err, NAME, format_args!("{} Nothing written.", why)),
    };
    let Ok(seconds) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|since| since.as_secs()) else {
        return crate::refusal::limit(err, NAME, format_args!("the clock of this machine reads before 1970 or cannot be read. Nothing written."));
    };
    let not_run: Vec<String> = issue::ROLES.iter().filter(|role| !seen.roles.iter().any(|ran| ran == *role)).map(|role| role.to_string()).collect();
    let mut body = format!("commit: {}\nby: {} <{}>\ntime: {}\n", head, user, email, crate::prompt::utc(seconds));
    if !not_run.is_empty() {
        body.push_str(&format!("not run: {}\n", not_run.join(", ")));
    }
    let _ = writeln!(out, "{}: accepting {}", NAME, said(&head, &not_run));
    let _ = writeln!(out, "{}: will write {}:\n{}", NAME, path, body);
    // Whole or not at all: a file of its own, linked to the name, which fails where the name is there.
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|since| since.subsec_nanos()).unwrap_or(0);
    let mut written = None;
    for attempt in 0..100u32 {
        let part = base.join(FOLDER).join(format!(".{}.finished.{}-{}-{}", issue, std::process::id(), nanos, attempt));
        match fs::OpenOptions::new().write(true).create_new(true).open(&part) {
            Ok(opened) => {
                written = Some((part, opened));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return crate::refusal::limit(err, NAME, format_args!("{} cannot be written ({}). Nothing written.", path, error)),
        }
    }
    let Some((part, mut opened)) = written else {
        return crate::refusal::limit(err, NAME, format_args!("{} cannot be written. Nothing written.", path));
    };
    let result = opened.write_all(body.as_bytes()).and_then(|_| opened.sync_all());
    drop(opened);
    let result = result.and_then(|_| fs::hard_link(&part, &record));
    let _ = fs::remove_file(&part);
    if let Err(error) = result {
        return crate::refusal::limit(err, NAME, format_args!("{} could not be written ({}). Nothing written.", path, error));
    }
    let _ = writeln!(out, "{}: the work of issue {} is accepted.", NAME, issue::shown(issue));
    let command = format!("git add {} && git commit -m \"{}\" -- {}", path, commit_message(issue), path);
    if !commit {
        let _ = writeln!(out, "{}: commit the record alone, so the history holds it:\n  {}", NAME, command);
        return 0;
    }
    let _ = writeln!(out, "{}: will commit {} alone, with the message \"{}\", as {} <{}>.", NAME, path, commit_message(issue), user, email);
    let _ = out.flush();
    match issue::commit_path(root, &path, &commit_message(issue), &user, &email) {
        Ok(held) if held.iter().all(|one| *one == path) => {
            let _ = writeln!(out, "{}: committed {} alone. wb pushes nothing. Push the branch {} so that the history elsewhere holds it.", NAME, path, issue::shown(&branch));
            0
        }
        Ok(held) => crate::refusal::limit(
            err,
            NAME,
            format_args!("the commit is not alone: it holds {}. A hook of git added files to it. Nothing more is done: undo or amend it as you decide.", held.iter().map(|one| issue::shown(one)).collect::<Vec<_>>().join(", ")),
        ),
        Err(why) => {
            let code = crate::refusal::limit(err, NAME, format_args!("the commit failed: {}", why));
            let _ = writeln!(err, "{}: {} is written and stays, and is unstaged again. Commit it alone:\n  {}", NAME, path, command);
            code
        }
    }
}
