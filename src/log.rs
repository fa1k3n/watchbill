//! `wb log <issue>` (#177; `docs/revision.md`, point 15): the history of an issue in the flow, as `git log` prints a
//! branch's, from the job's ref and git alone, offline. It only reads: it writes nothing, and asks nothing of the
//! network or of a tracker.
//!
//! One entry for each commit of `refs/watchbill/<issue>` that counts (`stamps::read`, #327): a stamp, a marker of
//! `wb revert`, a commit of questions or of answers, oldest first (git's own log is newest first: this one is read
//! as a story, from the first step on). A commit that does not count is no entry: it is said on standard error, with
//! why, and the command ends with 1. On a ref that is not as `wb` writes it nothing counts, so nothing is printed
//! and the line that `wb status` gives is said.
//!
//! A stamp says its role, round, model, effort, revision and time, as it holds them, and by whom it is signed. Where
//! this machine holds the lines of its start (`kept::ended`, #124) the outcome follows, as diagnostics; else it is
//! said to be not known here. The commits taken back from its session are those that a note of `refs/notes/watchbill`
//! names (#237): **a note is not signed**, so they are listed and said to be notes, and never counted and never an
//! entry (`session_status::named_by_notes`, #239). A marker says which commits it took back; a commit it took back
//! stays in the history and is said to be taken back.
//!
//! What a question or an answer says counts for nothing and is not printed: only which questions or answers.
//!
//! `--oneline` gives one line to each entry. Every read of git goes through `stamps::read` and `signers::git`: no
//! replace ref, no graft, no lazy fetch, no variable `GIT_*`.

use std::io::Write;
use std::path::Path;

use crate::issue::shown;
use crate::kept::Ended;
use crate::session_status::{named_by_notes, Named};
use crate::stamps::{self, Entry, Kind};
use crate::{refusal, starter};

/// The most characters of a field of a stamp that are printed.
const FIELD_CAP: usize = 200;

/// The value of the line `name:` of a message, or none where it has none or it is empty.
fn field(message: &str, name: &str) -> Option<String> {
    let prefix = format!("{}:", name);
    message
        .lines()
        .find_map(|line| line.strip_prefix(prefix.as_str()))
        .map(|value| shown(value.trim()).chars().take(FIELD_CAP).collect::<String>())
        .filter(|value| !value.is_empty())
}

/// What a first line says after the tag that names its kind: `[questions] 54.tester q1 q2` is `54.tester q1 q2`.
fn after_tag(first: &str) -> String {
    shown(first.split_once("] ").map(|(_, rest)| rest).unwrap_or(first).trim())
}

/// The outcome of a session, in lines for the default form and one line for `--oneline`.
fn ended_lines(ended: &Ended) -> (String, Vec<String>) {
    match ended {
        Ended::Unknown => ("not known on this machine".to_string(), Vec::new()),
        Ended::NoOutcome(_) => ("not known: the lines of its start on this machine hold no outcome".to_string(), Vec::new()),
        Ended::Runs(_) => ("not yet: its start runs on this machine".to_string(), Vec::new()),
        Ended::CutOff(_) => ("not known: its start was cut off, the wb that ran it is gone".to_string(), Vec::new()),
        Ended::Lines(_, lines) => {
            let lines: Vec<String> = lines.iter().map(|line| shown(line)).collect();
            (format!("(from the lines of its start on this machine; diagnostics, nothing is decided from them)"), lines)
        }
    }
}

/// The entries that took back others: for the marker at place `at` of `counting` (the first lines of the commits that
/// count), the places it took back.
fn took_back(counting: &[&Entry], at: usize) -> Vec<usize> {
    let upto = |end: usize| -> Vec<bool> {
        let text: String = counting[..end].iter().map(|entry| format!("{}\n", entry.first)).collect();
        starter::taken_back(&text)
    };
    let with = upto(at + 1);
    let before = upto(at);
    (0..at).filter(|place| with[*place] && !before[*place]).collect()
}

/// `wb log <issue> [--oneline]` in the repository at `root`: 0 the history is printed, 1 the issue has none or a commit
/// does not count, or git could not be read. `state` is the state folder of this machine, for how a start ended.
pub fn run(root: &str, issue: &str, oneline: bool, state: Option<&Path>, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let name = "wb log";
    let ledger = match stamps::read(root, issue) {
        Ok(ledger) => ledger,
        Err(why) => return refusal::limit(err, name, format_args!("the job's ref of {} cannot be read: {}", shown(issue), shown(&why))),
    };
    if let Some(broken) = &ledger.broken {
        // Nothing on a ref that is not as `wb` writes it counts: none of it is printed.
        let _ = writeln!(err, "{}: {}", name, shown(broken));
        refusal::answer();
        return 1;
    }
    if ledger.entries.is_empty() {
        let _ = writeln!(err, "{}: {} holds no commit of wb: nothing has been done to {} that the job's ref says", name, ledger.full, shown(issue));
        if let Some(missing) = &ledger.missing {
            let _ = writeln!(err, "{}: {}", name, shown(missing));
        }
        refusal::answer();
        return 1;
    }
    let counting: Vec<&Entry> = ledger.entries.iter().filter(|entry| entry.counts.is_ok()).collect();
    let mut not_counted = 0;
    for entry in ledger.entries.iter().filter(|entry| entry.counts.is_err()) {
        not_counted += 1;
        let why = entry.counts.as_ref().err().map(String::as_str).unwrap_or("");
        let _ = writeln!(err, "{}: {} on {} is not counted, and is no entry: {}", name, entry.commit, ledger.full, shown(why));
    }
    if not_counted > 0 {
        if let Some(missing) = &ledger.missing {
            let _ = writeln!(err, "{}: {}", name, shown(missing));
        }
    }
    let gone = starter::taken_back(&ledger.lines());
    // Everything is read before anything is printed: a note that cannot be read leaves nothing half said.
    let mut notes: Vec<Option<Named>> = Vec::new();
    for entry in &counting {
        if entry.kind != Kind::Stamp {
            notes.push(None);
            continue;
        }
        let base = field(&entry.message, "base");
        match named_by_notes(root, &entry.commit, base.as_deref()) {
            Ok(named) => notes.push(Some(named)),
            Err(why) => return refusal::limit(err, name, format_args!("the notes of {} cannot be read: {}", crate::notes::REF, shown(&why))),
        }
    }
    let mut text = String::new();
    for (at, entry) in counting.iter().enumerate() {
        // A needs_person commit (#311) is not in the history of #177: it is left out, and not an entry.
        if entry.kind == Kind::NeedsPerson {
            continue;
        }
        let taken = gone.get(at).copied().unwrap_or(false);
        let named = notes[at].as_ref();
        let dropped: Vec<usize> = if entry.kind == Kind::Marker { took_back(&counting, at) } else { Vec::new() };
        let commit = entry.commit.as_str();
        let by = entry.counts.as_ref().map(String::as_str).unwrap_or("");
        let (role, round) = match entry.kind {
            Kind::Stamp => starter::role_and_round("[stamp] ", &entry.first),
            Kind::Marker => starter::role_and_round("[revert] ", &entry.first),
            _ => None,
        }
        .map(|(role, round)| (shown(&role), round))
        .unwrap_or_default();
        if oneline {
            let short: String = commit.chars().take(12).collect();
            let mut line = match entry.kind {
                Kind::Stamp => {
                    let mut line = format!("{} stamp {} round {}", short, role, round);
                    if let Some(model) = field(&entry.message, "model") {
                        line.push_str(&format!(", model {}", model));
                    }
                    if let Some(effort) = field(&entry.message, "effort") {
                        line.push_str(&format!(", effort {}", effort));
                    }
                    if let Some(revision) = field(&entry.message, "revision") {
                        line.push_str(&format!(", revision {}", revision.chars().take(12).collect::<String>()));
                    }
                    if let Some(time) = field(&entry.message, "time") {
                        line.push_str(&format!(", time {}", time));
                    }
                    let (ended, lines) = ended_lines(&crate::kept::ended(state, &entry.message));
                    if lines.is_empty() {
                        line.push_str(&format!(", ended {}", ended));
                    } else {
                        line.push_str(&format!(", ended {}", lines.join(" ")));
                    }
                    line
                }
                Kind::Marker => format!("{} marker {} round {}, took back {}", short, role, round, dropped.len()),
                Kind::Questions => format!("{} questions {}", short, after_tag(&entry.first)),
                Kind::Answers => format!("{} answers {}", short, after_tag(&entry.first)),
                Kind::Unread => format!("{} commit that is not read", short),
                // Not in the history of #177: left out, and not an entry.
                Kind::NeedsPerson => String::new(),
            };
            if let Some(named) = named.filter(|named| !named.commits.is_empty()) {
                let n = named.commits.len();
                line.push_str(&format!(", {} commit{} taken back (from notes)", n, if n == 1 { "" } else { "s" }));
            }
            if taken {
                line.push_str(", taken back by a [revert] marker");
            }
            text.push_str(&line);
            text.push('\n');
            continue;
        }
        if !text.is_empty() {
            text.push('\n');
        }
        match entry.kind {
            Kind::Stamp => {
                text.push_str(&format!("stamp {}\n", commit));
                text.push_str(&format!("    Role: {} round {}\n", role, round));
                let model = field(&entry.message, "model").unwrap_or_else(|| "none".to_string());
                let effort = field(&entry.message, "effort").unwrap_or_else(|| "none".to_string());
                text.push_str(&format!("    Model: {}, effort {}\n", model, effort));
                text.push_str(&format!("    Revision: {}\n", field(&entry.message, "revision").unwrap_or_else(|| "none".to_string())));
                text.push_str(&format!("    Time: {}\n", field(&entry.message, "time").unwrap_or_else(|| "none".to_string())));
            }
            Kind::Marker => {
                text.push_str(&format!("marker {}\n", commit));
                text.push_str(&format!("    Role: {} round {}\n", role, round));
            }
            Kind::Questions => {
                text.push_str(&format!("questions {}\n", commit));
                text.push_str(&format!("    Questions: {}\n", after_tag(&entry.first)));
            }
            Kind::Answers => {
                text.push_str(&format!("answers {}\n", commit));
                text.push_str(&format!("    Answers: {}\n", after_tag(&entry.first)));
            }
            Kind::Unread => text.push_str(&format!("commit {}\n    A commit that is not read\n", commit)),
            Kind::NeedsPerson => {}
        }
        text.push_str(&format!("    Signed: by {} (on {})\n", shown(by), ledger.full));
        if taken {
            text.push_str("    Taken back: by a [revert] marker\n");
        }
        if !dropped.is_empty() {
            text.push_str(&format!("    Took back {}:\n", dropped.len()));
            for place in &dropped {
                text.push_str(&format!("      {}  {}\n", counting[*place].commit, shown(&counting[*place].first)));
            }
        }
        if entry.kind == Kind::Stamp {
            let (ended, lines) = ended_lines(&crate::kept::ended(state, &entry.message));
            if lines.is_empty() {
                text.push_str(&format!("    Ended: {}\n", ended));
            } else {
                text.push_str(&format!("    Ended {}:\n", ended));
                for line in lines {
                    text.push_str(&format!("      {}\n", line));
                }
            }
        }
        if let Some(named) = named {
            if !named.commits.is_empty() {
                text.push_str(&format!("    Taken back (named by a note in {}, which is not signed: an index, and nothing counts from it), newest last:\n", crate::notes::REF));
                for (commit, subject) in &named.commits {
                    if subject.is_empty() {
                        text.push_str(&format!("      {}\n", commit));
                    } else {
                        text.push_str(&format!("      {}  {}\n", commit, shown(subject)));
                    }
                }
            }
            match named.left_out {
                0 => {}
                1 => text.push_str("    Left out: 1 object named by a note is not a commit of this repository\n"),
                n => text.push_str(&format!("    Left out: {} objects named by a note are not commits of this repository\n", n)),
            }
        }
    }
    let _ = out.write_all(text.as_bytes());
    if not_counted > 0 {
        let _ = writeln!(err, "{}: {} commit{} of {} not counted, and not shown", name, not_counted, if not_counted == 1 { "" } else { "s" }, ledger.full);
        refusal::answer();
        return 1;
    }
    0
}
