//! `wb revert` (#142): takes an issue, or a change with no issue, back to before a step, by one marker that
//! deletes nothing. The marker is a comment whose first line is `[revert] <role> round <n>`; `starter::counted`
//! reads it, and from then on the stamps, questions and answers from that stamp up to the marker do not count.
//!
//! Only the comment is posted, by `issue_comment` (or `change_comment`). The branch, the issue's file and its
//! acceptance, and the change are not touched: the commits of the steps taken back stay, and taking them back is
//! git's.

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
    if role == "breakdown" {
        let _ = writeln!(
            err,
            "wb revert: the breakdown is not taken back by a marker. The issue's file and its acceptance are files of the repository, and git takes them back."
        );
        return 1;
    }
    let mut commands = Vec::new();
    for key in [read, post] {
        match prompt::pattern_of(root, key) {
            Ok(Some(line)) if prompt::is_plain_name(value) => commands.push(line.replace(&format!("{{{}}}", word), value)),
            Ok(Some(_)) => {
                let _ = writeln!(err, "wb revert: '{}' is not a plain name that fills `{}`.", value.escape_debug(), key);
                return 1;
            }
            Ok(None) => {
                let _ = writeln!(err, "wb revert: watchbill.yml has no `{}`. Add the line (see the README).", key);
                return 1;
            }
            Err(why) => {
                let _ = writeln!(err, "wb revert: {}", why);
                return 1;
            }
        }
    }
    let printed = match starter::run(root, read, &commands[0], None) {
        Ok(printed) => printed,
        Err(why) => {
            let _ = writeln!(err, "wb revert: {}", why);
            return 1;
        }
    };
    let stamps = starter::stamps_in(&String::from_utf8_lossy(&printed));
    let found = stamps.iter().enumerate().rev().find(|(_, (name, n))| name == role && round.map_or(true, |wanted| wanted == *n));
    let Some((at, (_, n))) = found else {
        let which = match round {
            Some(round) => format!("no `[stamp] {} round {}` that still counts", role, round),
            None => format!("no `[stamp] {}` that still counts", role),
        };
        let _ = writeln!(err, "wb revert: there is {} on {} {} (from `{}` of watchbill.yml), so there is no {} to take back.", which, word, value, read, role);
        return 1;
    };
    if stamps[at..].iter().any(|(name, _)| name == "breakdown") {
        let _ = writeln!(
            err,
            "wb revert: a later breakdown took the issue back ({} {}), so the stamp `{} round {}` before it no longer counts and is not reverted. Nothing is posted.",
            word, value, role, n
        );
        return 1;
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
            let _ = writeln!(err, "wb revert: {}. The marker may not be posted.", why);
            return 1;
        }
    }
    let _ = writeln!(err, "wb revert: the commits of the steps taken back stay on the branch, and the issue's file, its acceptance and the change are as they were. Taking commits back is git's.");
    0
}
