//! `wb status --session <hash>` (#239; `docs/revision.md`, point 15): what one session was given, how it ended and what
//! came back, from git alone, offline. It only reads: it writes nothing, and asks nothing of the network.
//!
//! `<hash>` names the commit of a stamp on a job's ref (#236), in full or as a unique start of 12 to 64 hex digits, and
//! only a stamp that counts: signed with `start` by a key of `.watchbill/signers`, of this issue and repository, on a
//! ref that is as `wb` writes it (`stamps::read`, #327). A stamp that does not count is refused with why, and none of it
//! is printed. What the stamp holds, its lines and its base, is what `stamps::read` found; nothing else is trusted.
//!
//! The commits of a session are those that a note of `refs/notes/watchbill` names (#237). **A note is not signed, and
//! this reads one for nothing but that:** the commits are found by it, listed and said so, and nothing counts from
//! them: no stamp, no round and no session is found or counted by a note. Only a note whose first line is
//! `stamp: <the whole hash of this stamp>` names a commit here, and an object it names that is no commit of this
//! repository is left out and counted.
//!
//! How the session ended is read from the lines its start left on this machine (`kept::ended`, #124): diagnostics,
//! never a decision. Where this machine holds none, it says so.
//!
//! Every read of git goes through `signers::git`: no replace ref, no graft, no lazy fetch, no variable `GIT_*`.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::Path;

use crate::issue::shown;
use crate::kept::Ended;
use crate::stamps::{self, Kind};
use crate::{refusal, signers};

/// The fewest hex digits of a hash given to `--session`.
pub const LEAST: usize = 12;

/// The most hex digits a hash has (SHA-256).
pub const MOST: usize = 64;

/// The most objects that a prefix is said to start, where it is not unique.
const MOST_LISTED: usize = 10;

/// The most notes that are read: far above the commits of any project.
const MOST_NOTES: usize = 200_000;

/// The most a note is read at, in bytes: `wb` writes two short lines.
const NOTE_CAP: u64 = 1024;

/// The most characters of a subject that are printed.
const SUBJECT_CAP: usize = 200;

/// What `--session` is given where it is used wrongly: its words.
pub const NEEDS: &str = "'--session' needs the hash of a stamp, 12 to 64 hex digits (a stamp's commit on a job's ref, or the start of it)";

/// Whether `given` is as `--session` is given: 12 to 64 hex digits is a hash, anything else is used wrongly. More
/// than 64 hex digits is a hash that names nothing, so it is for `run` to say.
pub fn is_given_rightly(given: &str) -> bool {
    given.len() >= LEAST && given.chars().all(|c| c.is_ascii_hexdigit())
}

/// What a note names.
struct Named {
    /// The commits of the session, newest last, each with its subject.
    commits: Vec<(String, String)>,
    /// How many objects that a note of the stamp names are no commit of this repository.
    left_out: usize,
    /// The listed commits that do not descend from the stamp's base, in the order listed: a hint for a person, which
    /// decides nothing and counts for nothing.
    not_from_base: Vec<String>,
}

/// `wb status --session <hash>` in the repository at `root`: 0 the session is printed, 1 `hash` names no stamp that
/// counts (or not one only) or the notes cannot be read. `state` is the state folder of this machine, for how the start
/// ended. The caller has held `hash` to `is_given_rightly`.
pub fn run(root: &str, hash: &str, state: Option<&Path>, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let name = "wb status";
    let hash = hash.to_ascii_lowercase();
    let none = |err: &mut dyn Write, why: String| {
        let _ = writeln!(err, "{}: {}", name, why);
        refusal::answer();
        1
    };
    // A prefix names an object of the repository; git lists those that start so, whatever they are.
    let found = if hash.len() > MOST { Ok(Vec::new()) } else { objects_starting(root, &hash) };
    // Git cuts a prefix down to the length of its own hashes, so 41 to 64 digits in a SHA-1 repository would name the
    // object that the first 40 do: only a name that starts with the whole hash given is one.
    let found = found.map(|names| names.into_iter().filter(|name| name.starts_with(&hash)).collect::<Vec<String>>());
    let found = match found {
        Ok(found) => found,
        Err(why) => return refusal::limit(err, name, format_args!("git could not say what {} names: {}", hash, shown(&why))),
    };
    let stamp = match found.as_slice() {
        [] => return none(err, format!("no stamp starts with {}: no object of this repository does.", hash)),
        [one] => one.clone(),
        many => {
            let listed: Vec<&str> = many.iter().take(MOST_LISTED).map(String::as_str).collect();
            let more = if many.len() > MOST_LISTED { format!(" and {} more", many.len() - MOST_LISTED) } else { String::new() };
            return none(err, format!("{} is not unique: it starts {} objects of this repository: {}{}. Give more digits.", hash, many.len(), listed.join(", "), more));
        }
    };
    // What it is: a commit that a job's ref holds, as a stamp that counts.
    let kind = match signers::git(root, &["cat-file", "-t", &stamp], None) {
        Ok(kind) => String::from_utf8_lossy(&kind).trim().to_string(),
        Err(why) => return refusal::limit(err, name, format_args!("{} could not be read: {}", stamp, shown(&why))),
    };
    if kind != "commit" {
        return none(err, format!("{} is a {}, not a commit: a stamp is a commit of a job's ref.", stamp, shown(&kind)));
    }
    let message = match message_of(root, &stamp) {
        Ok(message) => message,
        Err(why) => return none(err, format!("{} is not on a job's ref ({}).", stamp, why)),
    };
    let Some(issue) = message.lines().find_map(|line| line.strip_prefix("issue: ")).filter(|issue| crate::issue::is_name(issue)).map(str::to_string) else {
        return none(err, format!("{} is not on a job's ref (its message names no issue).", stamp));
    };
    let ledger = match stamps::read(root, &issue) {
        Ok(ledger) => ledger,
        Err(why) => return refusal::limit(err, name, format_args!("the job's ref of {} cannot be read: {}", shown(&issue), shown(&why))),
    };
    let Some(at) = ledger.entries.iter().position(|entry| entry.commit == stamp) else {
        return none(err, format!("{} is not on a job's ref ({} does not hold it).", stamp, ledger.full));
    };
    let entry = &ledger.entries[at];
    match entry.kind {
        Kind::Stamp => {}
        Kind::Marker => return none(err, format!("{} is a marker of wb revert on {}, not a stamp.", stamp, ledger.full)),
        Kind::Questions => return none(err, format!("{} is a questions commit on {}, not a stamp.", stamp, ledger.full)),
        Kind::Answers => return none(err, format!("{} is an answer commit on {}, not a stamp.", stamp, ledger.full)),
        Kind::NeedsPerson => return none(err, format!("{} is a needs_person commit on {}, not a stamp.", stamp, ledger.full)),
        Kind::Unread => return none(err, format!("{} is a commit on {} that is not read, not a stamp.", stamp, ledger.full)),
    }
    // A stamp that does not count is refused, with why; nothing of it is printed. A ref that is not as `wb` writes it, or
    // no list of keys, is said with it.
    let signed_by = match &entry.counts {
        Ok(by) => by.clone(),
        Err(why) => {
            let mut said = format!("the stamp {} on {} is a stamp that does not count: {}.", stamp, ledger.full, shown(why));
            for more in ledger.broken.iter().chain(ledger.missing.iter()) {
                said.push_str(&format!(" {}.", shown(more)));
            }
            return refusal::limit(err, name, said);
        }
    };
    // Whether a `[revert]` marker took it back: counted among the stamps and markers that count, as `Ledger::said` does.
    let counting: Vec<usize> = ledger.entries.iter().enumerate().filter(|(_, entry)| entry.counts.is_ok()).map(|(at, _)| at).collect();
    let gone = crate::starter::taken_back(&ledger.lines());
    let taken_back = counting.iter().position(|held| *held == at).and_then(|place| gone.get(place).copied()).unwrap_or(false);
    // Everything is read before anything is printed: a note that cannot be read leaves nothing half said.
    let base = entry.message.lines().find_map(|line| line.strip_prefix("base: ")).map(str::to_string);
    let named = match named_by_notes(root, &stamp, base.as_deref()) {
        Ok(named) => named,
        Err(why) => return refusal::limit(err, name, format_args!("the notes of {} cannot be read: {}", crate::notes::REF, shown(&why))),
    };
    let ended = crate::kept::ended(state, &entry.message);

    let mut text = String::new();
    text.push_str(&format!("Session {}\n", stamp));
    text.push_str(&format!("Stamp, on {}, signed by {}:\n", ledger.full, shown(&signed_by)));
    for line in entry.message.trim_end().lines() {
        text.push_str(&format!("  {}\n", shown(line)));
    }
    if taken_back {
        text.push_str("Taken back: by a [revert] marker\n");
    }
    match base {
        Some(base) => text.push_str(&format!("Base: {}\n", shown(&base))),
        None => text.push_str("Base: none\n"),
    }
    if named.commits.is_empty() {
        text.push_str(&format!(
            "Commits: none, no note of {} names this stamp (notes are not signed: a note removed or never fetched is not seen)\n",
            crate::notes::REF
        ));
    } else {
        text.push_str(&format!("Commits (named by a note in {}, which is not signed: an index, and nothing counts from it), newest last:\n", crate::notes::REF));
        for (commit, subject) in &named.commits {
            if subject.is_empty() {
                text.push_str(&format!("  {}\n", commit));
            } else {
                text.push_str(&format!("  {}  {}\n", commit, shown(subject)));
            }
        }
    }
    match named.left_out {
        0 => {}
        1 => text.push_str("Left out: 1 object named by a note is not a commit of this repository, and is left out\n"),
        n => text.push_str(&format!("Left out: {} objects named by a note are not commits of this repository, and are left out\n", n)),
    }
    for commit in &named.not_from_base {
        text.push_str(&format!("Not from Base: {}, a commit that does not descend from Base\n", commit));
    }
    if let Some((last, _)) = named.commits.last() {
        text.push_str(&format!("Last: {} (from a note, which is not signed)\n", last));
    }
    match ended {
        Ended::Unknown => text.push_str("Ended: not known on this machine\n"),
        Ended::NoOutcome(folder) => text.push_str(&format!("Ended: not known: the lines of its start on this machine, in {}, hold no outcome\n", shown(&folder.display().to_string()))),
        Ended::Runs(folder) => text.push_str(&format!("Ended: not yet: its start runs on this machine, in {}\n", shown(&folder.display().to_string()))),
        Ended::CutOff(folder) => text.push_str(&format!(
            "Ended: not known: its start was cut off, the wb that ran it is gone (the lines in {} say `outcome: running`)\n",
            shown(&folder.display().to_string())
        )),
        Ended::Lines(folder, lines) => {
            text.push_str(&format!("Ended (from the lines of its start on this machine, in {}; diagnostics, nothing is decided from them):\n", shown(&folder.display().to_string())));
            for line in lines {
                text.push_str(&format!("  {}\n", shown(&line)));
            }
        }
    }
    let _ = out.write_all(text.as_bytes());
    0
}

/// The objects of the repository whose names start with `prefix`, as git lists them.
fn objects_starting(root: &str, prefix: &str) -> Result<Vec<String>, String> {
    let listed = signers::git(root, &["rev-parse", &format!("--disambiguate={}", prefix)], None)?;
    let mut found: Vec<String> = String::from_utf8_lossy(&listed).lines().map(str::to_string).filter(|line| !line.is_empty()).collect();
    found.sort();
    found.dedup();
    Ok(found)
}

/// The message of the commit `commit`, or why it is not read: over the cap of a stamp, or not UTF-8.
fn message_of(root: &str, commit: &str) -> Result<String, String> {
    let size = signers::git(root, &["cat-file", "-s", commit], None)?;
    match String::from_utf8_lossy(&size).trim().parse::<u64>() {
        Ok(size) if size <= stamps::CAP => {}
        Ok(_) => return Err("it is larger than 64 KiB".to_string()),
        Err(_) => return Err("git could not say its size".to_string()),
    }
    let raw = signers::git(root, &["cat-file", "commit", commit], None)?;
    let split = raw.windows(2).position(|pair| pair == b"\n\n").ok_or("it has no message")?;
    Ok(String::from_utf8_lossy(&raw[split + 2..]).to_string())
}

/// The types of `objects`, in the order given, as `git cat-file --batch-check` says them: `missing` for one it lacks.
fn kinds(root: &str, objects: &[String]) -> Result<Vec<(String, u64)>, String> {
    if objects.is_empty() {
        return Ok(Vec::new());
    }
    let input: String = objects.iter().map(|object| format!("{}\n", object)).collect();
    let printed = signers::git(root, &["cat-file", "--batch-check"], Some(input.as_bytes()))?;
    let printed = String::from_utf8_lossy(&printed).to_string();
    let rows: Vec<(String, u64)> = printed
        .lines()
        .map(|line| {
            let words: Vec<&str> = line.split(' ').collect();
            match words.as_slice() {
                [_, kind, size] => (kind.to_string(), size.parse().unwrap_or(0)),
                _ => ("missing".to_string(), 0),
            }
        })
        .collect();
    if rows.len() != objects.len() {
        return Err("git cat-file printed a line less or more than it was asked for".to_string());
    }
    Ok(rows)
}

/// The commits that a note of `refs/notes/watchbill` names for `stamp`: those whose note starts with the line
/// `stamp: <stamp>`, that are commits of this repository, newest last. A notes ref that is not there names none.
fn named_by_notes(root: &str, stamp: &str, base: Option<&str>) -> Result<Named, String> {
    let empty = Named { commits: Vec::new(), left_out: 0, not_from_base: Vec::new() };
    if signers::git(root, &["rev-parse", "--verify", "-q", crate::notes::REF], None).is_err() {
        return Ok(empty);
    }
    let ref_arg = format!("--ref={}", crate::notes::REF);
    let listed = signers::git(root, &["notes", &ref_arg, "list"], None)?;
    let listed = String::from_utf8_lossy(&listed).to_string();
    // `<note blob> <annotated object>`
    let pairs: Vec<(&str, &str)> = listed.lines().filter_map(|line| line.split_once(' ')).collect();
    if pairs.len() > MOST_NOTES {
        return Err(format!("it holds more than {} notes, and no more are read", MOST_NOTES));
    }
    // Each distinct note once: notes of one text are one blob.
    let mut blobs: Vec<String> = pairs.iter().map(|(blob, _)| blob.to_string()).collect::<HashSet<_>>().into_iter().collect();
    blobs.sort();
    let sizes = kinds(root, &blobs)?;
    let small: Vec<String> = blobs.iter().zip(&sizes).filter(|(_, (kind, size))| kind == "blob" && *size <= NOTE_CAP).map(|(blob, _)| blob.clone()).collect();
    let first_lines = first_lines_of(root, &small)?;
    let wanted = format!("stamp: {}", stamp);
    let mut names: Vec<String> = pairs.iter().filter(|(blob, _)| first_lines.get(*blob).is_some_and(|first| *first == wanted)).map(|(_, object)| object.to_string()).collect();
    names.sort();
    names.dedup();
    // Of what they name, the commits this repository has.
    let types = kinds(root, &names)?;
    let commits: Vec<String> = names.iter().zip(&types).filter(|(_, (kind, _))| kind == "commit").map(|(object, _)| object.clone()).collect();
    let left_out = names.len() - commits.len();
    if commits.is_empty() {
        return Ok(Named { commits: Vec::new(), left_out, not_from_base: Vec::new() });
    }
    // Newest last: git's `--topo-order` puts a commit before its parents, so its reverse puts a commit after them. No time
    // is read.
    let input: String = commits.iter().map(|commit| format!("{}\n", commit)).collect();
    let walked = signers::git(root, &["rev-list", "--topo-order", "--stdin"], Some(input.as_bytes()))?;
    let wanted: HashSet<&str> = commits.iter().map(String::as_str).collect();
    let mut ordered: Vec<String> = String::from_utf8_lossy(&walked).lines().filter(|line| wanted.contains(*line)).map(str::to_string).collect();
    ordered.reverse();
    // The subjects, with NUL between the hash and the subject and after it, so that an empty subject cannot lose the place.
    // They are for the eye alone: where git does not print them as asked, they are left empty, and nothing else changes.
    let input: String = ordered.iter().map(|commit| format!("{}\n", commit)).collect();
    let printed = signers::git(root, &["log", "-z", "--no-walk=unsorted", "--stdin", "--no-show-signature", "--no-notes", "--format=%H%x00%s"], Some(input.as_bytes())).unwrap_or_default();
    let printed = String::from_utf8_lossy(&printed).to_string();
    let mut tokens = printed.split('\0');
    let mut in_place = true;
    let mut commits = Vec::new();
    for commit in &ordered {
        let mut subject = String::new();
        if in_place {
            match (tokens.next(), tokens.next()) {
                (Some(hash), Some(said)) if hash == commit => subject = said.chars().take(SUBJECT_CAP).collect(),
                _ => in_place = false,
            }
        }
        commits.push((commit.clone(), subject));
    }
    // The listed commits that do not descend from the base: those that `rev-list --ancestry-path` of `base..` does not
    // reach. A hint for a person; where git cannot say, none is marked.
    let mut not_from_base = Vec::new();
    if let Some(base) = base.filter(|base| !base.is_empty() && base.len() <= MOST && base.chars().all(|c| c.is_ascii_hexdigit())) {
        let input: String = ordered.iter().map(|commit| format!("{}\n", commit)).chain(std::iter::once(format!("^{}\n", base))).collect();
        if let Ok(reached) = signers::git(root, &["rev-list", "--ancestry-path", "--stdin"], Some(input.as_bytes())) {
            let reached: HashSet<String> = String::from_utf8_lossy(&reached).lines().map(str::to_string).collect();
            not_from_base = ordered.iter().filter(|commit| !reached.contains(*commit)).cloned().collect();
        }
    }
    Ok(Named { commits, left_out, not_from_base })
}

/// The first line of each blob of `blobs` (note blobs, each of `NOTE_CAP` bytes or less), by the blob.
fn first_lines_of(root: &str, blobs: &[String]) -> Result<HashMap<String, String>, String> {
    let mut found = HashMap::new();
    if blobs.is_empty() {
        return Ok(found);
    }
    let input: String = blobs.iter().map(|blob| format!("{}\n", blob)).collect();
    let printed = signers::git(root, &["cat-file", "--batch"], Some(input.as_bytes()))?;
    let mut at = 0;
    while at < printed.len() {
        let end = printed[at..].iter().position(|byte| *byte == b'\n').map(|end| at + end).ok_or("git cat-file printed what is not an object")?;
        let head = String::from_utf8_lossy(&printed[at..end]).to_string();
        let words: Vec<&str> = head.split(' ').collect();
        let [object, "blob", size] = words.as_slice() else {
            return Err(format!("git cat-file printed {} where a note was asked for", shown(&head)));
        };
        let size: usize = size.parse().map_err(|_| "git cat-file printed a size that is not a number".to_string())?;
        let body = printed.get(end + 1..end + 1 + size).ok_or("git cat-file printed less than the size it gave")?;
        let first = String::from_utf8_lossy(body).lines().next().unwrap_or("").trim_end_matches('\r').to_string();
        found.insert(object.to_string(), first);
        at = end + 1 + size + 1;
    }
    Ok(found)
}
