//! `wb revert` (#142): takes an issue, or a change with no issue, back to before a step, by one marker that
//! deletes nothing. The marker's first line is `[revert] <role> round <n>`; from then on the stamps, questions and
//! answers from that stamp up to the marker do not count (`starter::counted`).
//!
//! For an issue (#236) the marker is a commit on its job's ref, after the stamps, naming the issue and the job, signed
//! and pushed to `origin` as a stamp is (`stamps::write`); its copy is posted by `issue_comment` where `watchbill.yml`
//! has it, and never read back for a round. For a change with no issue it is a comment, posted by `change_comment`.
//! The branch, the issue's file and its acceptance, and the change are not touched: the commits of the steps taken back
//! stay, and taking them back is git's.

use std::io::Write;

use crate::{prompt, starter};

/// Where the marker is posted: an issue, or a change with no issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum On<'a> {
    Issue(&'a str),
    Change(&'a str),
}

/// `wb revert` for `role` (a role the revision has) in the repository at `root`: 0 the marker is posted, 1 it was
/// refused or the posting failed. `round` is the round to take back, or the last of the role.
pub fn revert(root: &str, on: On, role: &str, round: Option<u64>, err: &mut dyn Write) -> i32 {
    let (word, value, read, post) = match on {
        On::Issue(issue) => ("issue", issue, "issue_comments", "issue_comment"),
        On::Change(change) => ("change", change, "change_comments", "change_comment"),
    };
    // A job that is done is not taken back by a marker (#282): its work is accepted, or its change is merged.
    if let On::Issue(issue) = on {
        if let Some(why) = crate::status::done_of(root, issue) {
            return crate::refusal::limit(err, "wb revert", format_args!("issue {} is done: {}, and there is no turning back by a marker. Nothing is posted.", crate::issue::shown(issue), why));
        }
    }
    if role == "planner" {
        return crate::refusal::limit(
            err,
            "wb revert",
            "the planner is not taken back by a marker. The issue's file and its acceptance are files of the repository, and git takes them back.",
        );
    }
    if let Err(why) = starter::held(root) {
        return crate::refusal::limit(err, "wb revert", format_args!("{}.", why));
    }
    // The stamps of an issue are commits on its job's ref (#236): so is the marker that takes them back.
    if let On::Issue(issue) = on {
        return on_ref(root, issue, role, round, err);
    }
    let mut commands = Vec::new();
    for key in [read, post] {
        match prompt::pattern_of(root, key) {
            Ok(Some(line)) if prompt::is_plain_name(value) => commands.push(line.replace(&format!("{{{}}}", word), value)),
            Ok(Some(_)) => {
                return crate::refusal::limit(err, "wb revert", format_args!("'{}' is not a plain name that fills `{}`.", value.escape_debug(), key));
            }
            Ok(None) => {
                return crate::refusal::limit(err, "wb revert", format_args!("watchbill.yml has no `{}`. Add the line (see the README).", key));
            }
            Err(why) => {
                return crate::refusal::limit(err, "wb revert", format_args!("{}", why));
            }
        }
    }
    let printed = match starter::run(root, read, &commands[0], None) {
        Ok(printed) => printed,
        Err(why) => {
            return crate::refusal::limit(err, "wb revert", format_args!("{}", why));
        }
    };
    let stamps = starter::stamps_in(&String::from_utf8_lossy(&printed));
    let found = stamps.iter().enumerate().rev().find(|(_, (name, n))| name == role && round.map_or(true, |wanted| wanted == *n));
    let Some((at, (_, n))) = found else {
        let which = match round {
            Some(round) => format!("no `[stamp] {} round {}` that still counts", role, round),
            None => format!("no `[stamp] {}` that still counts", role),
        };
        return crate::refusal::limit(err, "wb revert", format_args!("there is {} on {} {} (from `{}` of watchbill.yml), so there is no {} to take back.", which, word, value, read, role));
    };
    if stamps[at..].iter().any(|(name, _)| name == "planner") {
        return crate::refusal::limit(
            err,
            "wb revert",
            format_args!(
                "a later planner took the issue back ({} {}), so the stamp `{} round {}` before it no longer counts and is not reverted. Nothing is posted.",
                word, value, role, n
            ),
        );
    }
    let marker = format!("[revert] {} round {}\n", role, n);
    let _ = write!(
        err,
        "wb revert: will post on {} {}, by `{}` of watchbill.yml:\n{}From the stamp `{} round {}` up to this marker, no stamp, question or answer counts. Nothing is deleted.\n",
        word, value, post, marker, role, n
    );
    match starter::run(root, post, &commands[1], Some(marker.as_bytes())) {
        Ok(said) => {
            let _ = err.write_all(&said);
        }
        Err(why) => {
            return crate::refusal::limit(err, "wb revert", format_args!("{}. The marker may not be posted.", why));
        }
    }
    // Said with `-q` too: the commits stay, and taking them back is the person's.
    let _ = writeln!(err, "wb revert: the commits of the steps taken back stay on the branch, and the issue's file, its acceptance and the change are as they were. Taking commits back is git's.");
    0
}

/// `wb revert` of an issue (#236): the stamp is found among the commits on the job's ref that count, and the marker is
/// a commit on that ref after them, signed and pushed as a stamp is. Its comment is posted by `issue_comment` as a copy,
/// where `watchbill.yml` has the pattern; `wb` never reads it back.
fn on_ref(root: &str, issue: &str, role: &str, round: Option<u64>, err: &mut dyn Write) -> i32 {
    let limit = |err: &mut dyn Write, why: String| crate::refusal::limit(err, "wb revert", why);
    if !prompt::is_plain_name(issue) {
        return limit(err, format!("'{}' is not the name of an issue.", issue.escape_debug()));
    }
    let post = match prompt::pattern_of(root, "issue_comment") {
        Ok(post) => post,
        Err(why) => return limit(err, format!("{}", why)),
    };
    let signing = match crate::stamps::ready(root) {
        Ok(signing) => signing,
        Err(why) => return limit(err, format!("{}. Nothing is written.", why)),
    };
    let ledger = match crate::stamps::read(root, issue) {
        Ok(ledger) => ledger,
        Err(why) => return limit(err, format!("{}.", why)),
    };
    // A job's ref that is not as `wb` writes it (#327): no marker is written after it until a person repairs it.
    if let Some(why) = &ledger.broken {
        return limit(err, format!("{}. Nothing is written.", why));
    }
    let stamps = ledger.stamps();
    let found = stamps.iter().enumerate().rev().find(|(_, (name, n))| name == role && round.map_or(true, |wanted| wanted == *n));
    let Some((at, (_, n))) = found else {
        let which = match round {
            Some(round) => format!("no `[stamp] {} round {}` that still counts", role, round),
            None => format!("no `[stamp] {}` that still counts", role),
        };
        return limit(err, format!("there is {} on {}, so there is no {} to take back.", which, ledger.full, role));
    };
    if stamps[at..].iter().any(|(name, _)| name == "planner") {
        return limit(
            err,
            format!("a later planner took the issue back ({}), so the stamp `{} round {}` before it no longer counts and is not reverted. Nothing is posted.", ledger.full, role, n),
        );
    }
    // The marker names its issue, its job and its base (the review of #322, #324): on the ref of another issue it counts for nothing.
    let jobs = crate::plan::source_hashes(root, issue);
    let job_lines: String = if jobs.is_empty() { "job: none\n".to_string() } else { jobs.iter().map(|job| format!("job: {}\n", job)).collect() };
    let marker = format!("[revert] {} round {}\n{}\n{}{}", role, n, crate::stamps::issue_line(issue), job_lines, crate::stamps::base_line(root));
    let _ = write!(err, "wb revert: the marker:\n{}From the stamp `{} round {}` up to this marker, no stamp, question or answer counts. Nothing is deleted.\n", marker, role, n);
    let commit = match crate::stamps::write("wb revert", root, issue, &marker, &crate::stamps::Held::Marker, &signing, err) {
        Ok((commit, _)) => commit,
        Err(why) => return limit(err, format!("{}.", why)),
    };
    match post {
        None => {
            let _ = writeln!(err, "wb revert: watchbill.yml has no `issue_comment`, so no copy of the marker is posted: the marker is the commit on {}.", ledger.full);
        }
        Some(line) => {
            let _ = writeln!(err, "wb revert: will post on issue {}, by `issue_comment` of watchbill.yml, a copy of the marker.", issue);
            // The copy names the commit it copies (#240).
            let copy = crate::stamps::with_commit(&marker, &commit);
            match starter::run(root, "issue_comment", &line.replace("{issue}", issue), Some(copy.as_bytes())) {
                Ok(said) => {
                    let _ = err.write_all(&said);
                }
                Err(why) => return limit(err, format!("{}. The marker is on {}; its copy may not be posted.", why, ledger.full)),
            }
        }
    }
    let _ = writeln!(err, "wb revert: the commits of the steps taken back stay on the branch, and the issue's file, its acceptance and the change are as they were. Taking commits back is git's.");
    0
}
