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
//! What a question or an answer says counts for nothing and is not printed: only which questions or answers. A
//! needs_person commit (#311) is an entry with its role and its reason, cut as `wb status` cuts it.
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

/// The value of the line `name:` of a message, or none where it has none or it is empty: as one line, escaped and cut
/// after the escape as `wb status` cuts a reason (`status::reason_line`, #311), never inside an escape.
fn field(message: &str, name: &str) -> Option<String> {
    let prefix = format!("{}:", name);
    message.lines().find_map(|line| line.strip_prefix(prefix.as_str())).map(|value| crate::status::reason_line(value.trim())).filter(|value| !value.is_empty())
}

/// What a first line says after the tag that names its kind: `[questions] 54.tester q1 q2` is `54.tester q1 q2`.
fn after_tag(first: &str) -> String {
    shown(first.split_once("] ").map(|(_, rest)| rest).unwrap_or(first).trim())
}

/// The outcome of a session, in lines for the default form, and the one word of it for `--oneline`: the value of the
/// line `outcome:`, the diagnostics being left to the full form.
fn ended_lines(ended: &Ended) -> (String, Vec<String>, String) {
    match ended {
        Ended::Unknown => ("not known on this machine".to_string(), Vec::new(), "not known on this machine".to_string()),
        Ended::NoOutcome(_) => ("not known: the lines of its start on this machine hold no outcome".to_string(), Vec::new(), "not known".to_string()),
        Ended::Runs(_) => ("not yet: its start runs on this machine".to_string(), Vec::new(), "not yet".to_string()),
        Ended::CutOff(_) => ("not known: its start was cut off, the wb that ran it is gone".to_string(), Vec::new(), "not known, cut off".to_string()),
        Ended::Lines(_, lines) => {
            let word = lines.iter().find_map(|line| line.strip_prefix("outcome:")).map(|value| crate::status::reason_line(value.trim())).filter(|value| !value.is_empty()).unwrap_or_else(|| "not known".to_string());
            let lines: Vec<String> = lines.iter().map(|line| shown(line)).collect();
            ("(from the lines of its start on this machine; diagnostics, nothing is decided from them)".to_string(), lines, word)
        }
    }
}

/// The role of a needs_person commit, from its first line (`[needs_person] <issue>.<role>`; none for the planner's), and
/// its reason: what follows its header, as one line, escaped and cut as `wb status` cuts it.
fn needs_person(entry: &Entry, issue: &str) -> (Option<String>, String) {
    let who = stamps::needs_person_role(&entry.first, issue).flatten().map(str::to_string);
    let reason = stamps::copy_of(&entry.message).split_once('\n').map(|(_, reason)| crate::status::reason_line(reason)).unwrap_or_default();
    (who, reason)
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

/// Why the marker at place `at` of `counting` took nothing back, in words true of its case: a planner's marker takes none
/// (`starter::taken_back`), a second marker for a stamp already taken back is told by which, else no stamp is before it.
fn nothing_because(counting: &[&Entry], at: usize, role: &str, round: u64) -> (String, String) {
    if role == "planner" {
        return ("nothing, a planner's marker takes nothing back".to_string(), "a planner's marker takes nothing back".to_string());
    }
    let earlier = (0..at).find(|place| {
        counting[*place].kind == Kind::Marker && starter::role_and_round("[revert] ", &counting[*place].first) == Some((role.to_string(), round)) && !took_back(counting, *place).is_empty()
    });
    match earlier {
        Some(place) => {
            let by = &counting[place].commit;
            (format!("nothing more, [stamp] {} round {} is already taken back by {}", role, round, by), format!("nothing more, already taken back by {}", by.chars().take(12).collect::<String>()))
        }
        None => (format!("nothing, no [stamp] {} round {} before it", role, round), "nothing".to_string()),
    }
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
        let base = entry.message.lines().find_map(|line| line.strip_prefix("base: ")).map(str::to_string);
        match named_by_notes(root, &entry.commit, base.as_deref()) {
            Ok(named) => notes.push(Some(named)),
            Err(why) => return refusal::limit(err, name, format_args!("the notes of {} cannot be read: {}", crate::notes::REF, shown(&why))),
        }
    }
    let mut text = String::new();
    for (at, entry) in counting.iter().enumerate() {
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
                    let (_, _, word) = ended_lines(&crate::kept::ended(state, &entry.message));
                    line.push_str(&format!(", outcome {}", word));
                    line
                }
                Kind::Marker if dropped.is_empty() => {
                    let (_, brief) = nothing_because(&counting, at, &role, round);
                    if brief == "nothing" {
                        format!("{} marker {} round {}, took back nothing", short, role, round)
                    } else if brief.starts_with("nothing more") {
                        format!("{} marker {} round {}, took back {}", short, role, round, brief)
                    } else {
                        format!("{} marker {} round {}, took back nothing, {}", short, role, round, brief)
                    }
                }
                Kind::Marker => format!("{} marker {} round {}, took back {}", short, role, round, dropped.len()),
                Kind::Questions => format!("{} questions {}", short, after_tag(&entry.first)),
                Kind::Answers => format!("{} answers {}", short, after_tag(&entry.first)),
                Kind::Unread => format!("{} commit that is not read", short),
                Kind::NeedsPerson => {
                    let (who, reason) = needs_person(entry, issue);
                    let mut line = format!("{} needs_person", short);
                    if let Some(who) = who {
                        line.push_str(&format!(" {}", who));
                    }
                    if !reason.is_empty() {
                        line.push_str(&format!(", reason {}", reason));
                    }
                    line
                }
            };
            if let Some(named) = named.filter(|named| !named.commits.is_empty()) {
                let n = named.commits.len();
                line.push_str(&format!(", {} commit{} taken back (from notes)", n, if n == 1 { "" } else { "s" }));
                if !named.not_from_base.is_empty() {
                    line.push_str(&format!(", {} not from Base", named.not_from_base.len()));
                }
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
            Kind::NeedsPerson => {
                text.push_str(&format!("needs_person {}\n", commit));
                let (who, reason) = needs_person(entry, issue);
                if let Some(who) = who {
                    text.push_str(&format!("    Role: {}\n", who));
                }
                text.push_str(&format!("    Reason: {}\n", reason));
            }
        }
        text.push_str(&format!("    Signed: by {} (on {})\n", shown(by), ledger.full));
        if taken {
            text.push_str("    Taken back: by a [revert] marker\n");
        }
        if entry.kind == Kind::Marker && dropped.is_empty() {
            let (full, _) = nothing_because(&counting, at, &role, round);
            text.push_str(&format!("    Took back: {}\n", full));
        }
        if !dropped.is_empty() {
            text.push_str(&format!("    Took back {}:\n", dropped.len()));
            for place in &dropped {
                text.push_str(&format!("      {}  {}\n", counting[*place].commit, shown(&counting[*place].first)));
            }
        }
        if entry.kind == Kind::Stamp {
            let (ended, lines, _) = ended_lines(&crate::kept::ended(state, &entry.message));
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
            for commit in &named.not_from_base {
                text.push_str(&format!("    Not from Base: {}, a commit that does not descend from Base\n", commit));
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
