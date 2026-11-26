//! `wb status --session <hash>` (#239): what one session was given, how it ended and what came back, from git alone,
//! offline. The stamp is a commit on the job's ref (#236), the commits taken back carry a note naming it (#237), and
//! the lines of a start are on the machine where it ran (#124). Run as a person runs it, in a repository made for the
//! test; the stamps are written as the tests of the ref write them (`stamps_on_ref`), the notes with `git notes`.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const NOTES: &str = "refs/notes/watchbill";

/// A repository set up by `wb init --yes`, with a git identity and the keys of the test (`signing`).
fn set_up() -> Folder {
    let folder = repository(None);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    git(&folder.path, &["add", "--", ".watchbill"]);
    git(&folder.path, &["commit", "-q", "-m", "Set up"]);
    signing(&folder.path);
    folder
}

fn git_text(folder: &Path, args: &[&str]) -> String {
    let run = Command::new("git").arg("-C").arg(folder).args(args).env_clear().envs(env()).stderr(Stdio::null()).output().expect("git could not be started");
    text(&run.stdout).trim().to_string()
}

/// The message of a commit, as `wb` reads it: what follows the headers.
fn message_of(folder: &Path, commit: &str) -> String {
    let raw = git_text(folder, &["cat-file", "commit", commit]);
    raw.split_once("\n\n").map(|(_, message)| message.to_string()).unwrap_or_default()
}

/// What a session of the test is: the stamp's commit, the commit it was made from, and the commits taken back.
struct Session {
    stamp: String,
    base: String,
    commits: Vec<String>,
}

/// A stamp of `role`, round 1, on the job's ref of `issue`, made from `HEAD`.
fn stamp(folder: &Path, issue: &str, role: &str) -> (String, String) {
    let base = git_text(folder, &["rev-parse", "HEAD"]);
    stamps_on_ref(folder, issue, &[&format!("[stamp] {} round 1\ntime: 2026-11-24T10:00:00Z\n", role)]);
    (git_text(folder, &["rev-parse", &format!("refs/watchbill/{}", issue)]), base)
}

/// A note on `object` that names `stamp`, as `wb` writes one.
fn note(folder: &Path, object: &str, stamp: &str, role: &str) {
    git(folder, &["notes", &format!("--ref={}", NOTES), "add", "-m", &format!("stamp: {}\nrole: {}", stamp, role), object]);
}

/// Commits on a branch of the issue, one for each subject, as a take-back leaves them, and a note on each that names
/// `stamp`. The checkout goes back to the branch it was on.
fn commits_with_notes(folder: &Path, issue: &str, stamp: &str, role: &str, subjects: &[&str]) -> Vec<String> {
    let back = git_text(folder, &["rev-parse", "--abbrev-ref", "HEAD"]);
    git(folder, &["checkout", "-q", "-b", &format!("issue-{}-work", issue)]);
    let mut commits = Vec::new();
    for subject in subjects {
        git(folder, &["commit", "-q", "--allow-empty", "--allow-empty-message", "-m", subject]);
        let commit = git_text(folder, &["rev-parse", "HEAD"]);
        note(folder, &commit, stamp, role);
        commits.push(commit);
    }
    git(folder, &["checkout", "-q", &back]);
    commits
}

/// Issue 12: a tester's session with two commits taken back.
fn with_two_commits() -> (Folder, Session) {
    let folder = set_up();
    let (stamp, base) = stamp(&folder.path, "12", "tester");
    let commits = commits_with_notes(&folder.path, "12", &stamp, "tester", &["Write the first test", "Write the second test"]);
    (folder, Session { stamp, base, commits })
}

/// The line `Last:` of a commit: it says in the line that it comes from a note, which is not signed.
fn last(commit: &str) -> String {
    format!("Last: {} (from a note, which is not signed)", commit)
}

fn status(folder: &Path, args: &[&str]) -> (String, String, Option<i32>) {
    let mut all = vec!["status"];
    all.extend(args);
    let run = wb(folder, &all, "");
    (text(&run.stdout), text(&run.stderr), code(&run))
}

/// `wb status --session` with the state folder `state`.
fn status_in(folder: &Path, state: &Path, hash: &str) -> (String, String, Option<i32>) {
    let run = Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(["status", "--session", hash])
        .current_dir(folder)
        .env_clear()
        .envs(env().into_iter().filter(|(name, _)| name != "XDG_STATE_HOME"))
        .env("XDG_STATE_HOME", state)
        .stdin(Stdio::null())
        .output()
        .expect("wb could not be started");
    (text(&run.stdout), text(&run.stderr), code(&run))
}

/// A folder of the lines of one start, as `wb start` leaves it, below `state`: the stamp's text and the outcome given.
fn lines_of_start(state: &Path, stamp_text: &str, role: &str, issue: &str, outcome: &str) {
    let lines = state.join("watchbill/sessions").join(format!("2026-11-24T100000Z-{}-{}", role, issue));
    fs::create_dir_all(&lines).unwrap();
    fs::write(lines.join("stamp"), stamp_text).unwrap();
    fs::write(lines.join("outcome"), outcome).unwrap();
}

// ---- WB-STATUS-46: the stamp, the base, the commits, the last ----

#[test]
fn test_WB_STATUS_46_a_session_with_two_commits_prints_the_stamp_the_base_and_both_commits_newest_last() {
    let (folder, session) = with_two_commits();
    let (out, err, status) = status(&folder.path, &["--session", &session.stamp]);
    assert_eq!(status, Some(0), "{}", err);
    let lines: Vec<&str> = out.lines().collect();
    // The stamp's lines, as the commit holds them.
    assert!(lines.contains(&"  [stamp] tester round 1"), "{}", out);
    assert!(lines.contains(&"  issue: 12"), "{}", out);
    assert!(lines.contains(&format!("  base: {}", session.base).as_str()), "{}", out);
    assert!(out.contains(&session.stamp), "{}", out);
    assert!(out.contains("refs/watchbill/12"), "{}", out);
    // The base on a line of its own, where a person or a script takes it.
    assert!(lines.contains(&format!("Base: {}", session.base).as_str()), "{}", out);
    // Both commits, the older first, and the last of them.
    let first = lines.iter().position(|line| *line == format!("  {}  Write the first test", session.commits[0])).unwrap_or_else(|| panic!("{}", out));
    let second = lines.iter().position(|line| *line == format!("  {}  Write the second test", session.commits[1])).unwrap_or_else(|| panic!("{}", out));
    assert_eq!(first + 1, second, "{}", out);
    assert!(lines.contains(&last(&session.commits[1]).as_str()), "{}", out);
    // Where the commits come from is said: a note is not signed.
    assert!(out.contains(NOTES) && out.contains("not signed"), "{}", out);
}

#[test]
fn test_WB_STATUS_46_a_commit_with_an_empty_subject_does_not_lose_the_place_or_change_the_status() {
    // A real session commit with an empty message, and a note on an empty-message commit: git prints no subject line for
    // either, and a reader that counts lines would take the next commit's header for a subject and end with 1.
    let folder = set_up();
    let (stamp, _) = stamp(&folder.path, "12", "tester");
    let commits = commits_with_notes(&folder.path, "12", &stamp, "tester", &["", "Write the first test", "", "Write the second test", ""]);
    let (out, err, status) = status(&folder.path, &["--session", &stamp]);
    assert_eq!(status, Some(0), "{}", err);
    let lines: Vec<&str> = out.lines().collect();
    let at = lines.iter().position(|line| *line == format!("  {}", commits[0])).unwrap_or_else(|| panic!("{}", out));
    assert_eq!(lines[at + 1], format!("  {}  Write the first test", commits[1]), "{}", out);
    assert_eq!(lines[at + 2], format!("  {}", commits[2]), "{}", out);
    assert_eq!(lines[at + 3], format!("  {}  Write the second test", commits[3]), "{}", out);
    assert_eq!(lines[at + 4], format!("  {}", commits[4]), "{}", out);
    assert!(out.lines().any(|line| line.starts_with(&format!("Last: {}", commits[4]))), "{}", out);
}

#[test]
fn test_WB_STATUS_46_a_prefix_of_12_digits_and_the_form_with_an_equals_sign_name_the_same_session() {
    let (folder, session) = with_two_commits();
    let (whole, _, _) = status(&folder.path, &["--session", &session.stamp]);
    let (short, err, status_short) = status(&folder.path, &["--session", &session.stamp[..12]]);
    assert_eq!(status_short, Some(0), "{}", err);
    assert_eq!(whole, short);
    let (equals, _, status_equals) = status(&folder.path, &[&format!("--session={}", &session.stamp[..20])]);
    assert_eq!(status_equals, Some(0));
    assert_eq!(whole, equals);
}

#[test]
fn test_WB_STATUS_46_a_commit_is_listed_only_where_a_note_names_this_stamp() {
    let (folder, session) = with_two_commits();
    // A second session of the same job, and a commit that its note names.
    stamps_on_ref(&folder.path, "12", &["[stamp] build round 1\ntime: 2026-11-24T11:00:00Z\n"]);
    let other = git_text(&folder.path, &["rev-parse", "refs/watchbill/12"]);
    let theirs = commits_with_notes(&folder.path, "12-b", &other, "build", &["Write the code"]);
    // A commit with no note at all.
    git(&folder.path, &["checkout", "-q", "issue-12-b-work"]);
    git(&folder.path, &["commit", "-q", "--allow-empty", "-m", "Nobody's"]);
    git(&folder.path, &["checkout", "-q", "-"]);
    let (out, err, status) = status(&folder.path, &["--session", &session.stamp]);
    assert_eq!(status, Some(0), "{}", err);
    assert!(!out.contains(&theirs[0]) && !out.contains("Write the code") && !out.contains("Nobody's"), "{}", out);
    let (out, _, _) = self::status(&folder.path, &["--session", &other]);
    assert!(out.contains(&last(&theirs[0])), "{}", out);
    assert!(!out.contains("Write the first test"), "{}", out);
}

#[test]
fn test_WB_STATUS_46_an_object_that_a_note_names_and_that_is_no_commit_of_this_repository_is_left_out() {
    let (folder, session) = with_two_commits();
    // A note on a blob.
    let blob = git_text(&folder.path, &["rev-parse", "HEAD:.watchbill/watchbill.yml"]);
    note(&folder.path, &blob, &session.stamp, "tester");
    let (out, err, status) = status(&folder.path, &["--session", &session.stamp]);
    assert_eq!(status, Some(0), "{}", err);
    assert!(out.contains("1 object named by a note is not a commit of this repository, and is left out"), "{}", out);
    // The commits are as they were, and the blob is not one of them.
    assert!(!out.contains(&blob), "{}", out);
    assert!(out.contains(&last(&session.commits[1])), "{}", out);
}

#[test]
fn test_WB_STATUS_46_a_session_that_a_revert_marker_took_back_says_so() {
    let (folder, session) = with_two_commits();
    stamps_on_ref(&folder.path, "12", &["[revert] tester round 1\n"]);
    let (out, err, status) = status(&folder.path, &["--session", &session.stamp]);
    assert_eq!(status, Some(0), "{}", err);
    assert!(out.lines().any(|line| line == "Taken back: by a [revert] marker"), "{}", out);
}

#[test]
fn test_WB_STATUS_46_a_listed_commit_that_does_not_descend_from_the_base_has_a_line_of_its_own() {
    let (folder, session) = with_two_commits();
    // A commit with no parent, which a forged note names for the stamp: it is listed as the notes say, and marked.
    let tree = git_text(&folder.path, &["rev-parse", &format!("{}^{{tree}}", session.base)]);
    let orphan = git_text(&folder.path, &["commit-tree", &tree, "-m", "Not the session's"]);
    note(&folder.path, &orphan, &session.stamp, "tester");
    let (out, err, status) = status(&folder.path, &["--session", &session.stamp]);
    assert_eq!(status, Some(0), "{}", err);
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines.contains(&format!("  {}  Not the session's", orphan).as_str()), "{}", out);
    assert!(lines.contains(&format!("Not from Base: {}, a commit that does not descend from Base", orphan).as_str()), "{}", out);
    // The commits that do descend from it have no such line, and nothing is decided: the status is 0 and `Last:` is the
    // newest listed.
    assert_eq!(lines.iter().filter(|line| line.starts_with("Not from Base:")).count(), 1, "{}", out);
    // And with only the commits of the session there is none.
    let (clean, _, _) = self::status(&folder.path, &["--session", &session.stamp[..12]]);
    assert_eq!(clean, out);
}

#[test]
fn test_WB_STATUS_46_no_line_of_a_session_with_commits_says_not_from_base_where_all_descend_from_it() {
    let (folder, session) = with_two_commits();
    let (out, _, _) = status(&folder.path, &["--session", &session.stamp]);
    assert!(!out.contains("Not from Base"), "{}", out);
}

#[test]
fn test_WB_STATUS_47_where_no_note_names_the_stamp_the_line_says_notes_are_not_signed() {
    let folder = set_up();
    let (stamp, _) = stamp(&folder.path, "12", "tester");
    let (out, err, status) = status(&folder.path, &["--session", &stamp]);
    assert_eq!(status, Some(0), "{}", err);
    let line = out.lines().find(|line| line.starts_with("Commits: none")).unwrap_or_else(|| panic!("{}", out));
    assert!(line.contains("notes are not signed: a note removed or never fetched is not seen"), "{}", line);
}

// ---- WB-STATUS-47: a session with no commits ----

#[test]
fn test_WB_STATUS_47_a_session_with_no_commits_says_none_and_ends_with_0() {
    let folder = set_up();
    let (stamp, base) = stamp(&folder.path, "12", "tester");
    let (out, err, status) = status(&folder.path, &["--session", &stamp]);
    assert_eq!(status, Some(0), "{}", err);
    assert!(out.lines().any(|line| line.starts_with("Commits: none")), "{}", out);
    assert!(out.lines().any(|line| line == format!("Base: {}", base)), "{}", out);
    assert!(!out.lines().any(|line| line.starts_with("Last:")), "{}", out);
}

// ---- WB-STATUS-48: how it ended ----

#[test]
fn test_WB_STATUS_48_a_session_that_did_not_run_on_this_machine_is_not_known_to_have_ended_any_way() {
    let (folder, session) = with_two_commits();
    let (out, err, status) = status(&folder.path, &["--session", &session.stamp]);
    assert_eq!(status, Some(0), "{}", err);
    assert!(out.lines().any(|line| line == "Ended: not known on this machine"), "{}", out);
    // Lines of a start of another stamp say nothing of this one.
    let state = folder_named("state");
    lines_of_start(&state.path, "[stamp] tester round 1\ntime: 2000-01-01T00:00:00Z\n", "tester", "12", "outcome: ended\nstatus: 3\nseconds: 9\n");
    let (out, _, status) = status_in(&folder.path, &state.path, &session.stamp);
    assert_eq!(status, Some(0));
    assert!(out.lines().any(|line| line == "Ended: not known on this machine"), "{}", out);
}

#[test]
fn test_WB_STATUS_48_a_session_that_ran_here_says_how_it_ended_from_the_lines_of_its_start() {
    let (folder, session) = with_two_commits();
    let state = folder_named("state");
    lines_of_start(&state.path, &message_of(&folder.path, &session.stamp), "tester", "12", "outcome: ended\nstatus: 0\nseconds: 5\n");
    let (out, err, status) = status_in(&folder.path, &state.path, &session.stamp);
    assert_eq!(status, Some(0), "{}", err);
    assert!(!out.contains("not known on this machine"), "{}", out);
    let at = out.lines().position(|line| line.starts_with("Ended")).unwrap_or_else(|| panic!("{}", out));
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(&lines[at + 1..at + 4], ["  outcome: ended", "  status: 0", "  seconds: 5"], "{}", out);
    assert!(lines[at].contains("diagnostics"), "{}", out);
    // The lines decide nothing: what the commits and the base are does not change with them.
    assert!(out.lines().any(|line| line == last(&session.commits[1])), "{}", out);
}

#[test]
fn test_WB_STATUS_48_the_lines_of_a_start_whose_wb_was_cut_off_say_so() {
    let (folder, session) = with_two_commits();
    let state = folder_named("state");
    lines_of_start(&state.path, &message_of(&folder.path, &session.stamp), "tester", "12", "outcome: running\npid: 4194304\n");
    let (out, err, status) = status_in(&folder.path, &state.path, &session.stamp);
    assert_eq!(status, Some(0), "{}", err);
    assert!(out.lines().any(|line| line.starts_with("Ended: not known") && line.contains("cut off")), "{}", out);
}

#[test]
fn test_WB_STATUS_48_the_lines_are_printed_with_their_control_characters_escaped() {
    let (folder, session) = with_two_commits();
    let state = folder_named("state");
    lines_of_start(&state.path, &message_of(&folder.path, &session.stamp), "tester", "12", "outcome: refused\nmessage: a \u{1b}[31mred\u{1b}[0m word\n");
    let (out, _, status) = status_in(&folder.path, &state.path, &session.stamp);
    assert_eq!(status, Some(0));
    assert!(!out.contains('\u{1b}'), "{:?}", out);
    assert!(out.contains("\\u{1b}[31mred"), "{}", out);
}

// ---- WB-STATUS-49: the hash is used wrongly ----

#[test]
fn test_WB_STATUS_49_a_prefix_of_fewer_than_12_digits_ends_with_2_and_prints_nothing() {
    let (folder, session) = with_two_commits();
    for given in [&session.stamp[..11], &session.stamp[..4], "", "zzzzzzzzzzzz", "12abcdefghij"] {
        let (out, err, status) = self::status(&folder.path, &["--session", given]);
        assert_eq!(status, Some(2), "{:?}: {}", given, err);
        assert_eq!(out, "", "{:?}", given);
        assert!(err.contains("'--session' needs the hash of a stamp, 12 to 64 hex digits"), "{:?}: {}", given, err);
    }
}

#[test]
fn test_WB_STATUS_49_session_with_no_value_beside_an_issue_or_a_stage_or_twice_is_used_wrongly() {
    let (folder, session) = with_two_commits();
    let hash = session.stamp.as_str();
    let cases: Vec<(Vec<&str>, &str)> = vec![
        (vec!["--session"], "'--session' needs a hash"),
        (vec!["12", "--session", hash], "'--session' takes no issue and no --stage"),
        (vec!["--session", hash, "12"], "'--session' takes no issue and no --stage"),
        (vec!["--session", hash, "--stage", "built"], "'--session' takes no issue and no --stage"),
        (vec!["--session", hash, "--session", hash], "'--session' is given once"),
    ];
    for (args, said) in cases {
        let (out, err, status) = self::status(&folder.path, &args);
        assert_eq!(status, Some(2), "{:?}: {}", args, err);
        assert_eq!(out, "", "{:?}", args);
        assert!(err.contains(said), "{:?}: {}", args, err);
    }
}

// ---- WB-STATUS-50: not unique, or no such stamp ----

#[test]
fn test_WB_STATUS_50_a_prefix_that_names_no_object_ends_with_1_and_says_no_stamp_starts_with_it() {
    let (folder, _) = with_two_commits();
    let (out, err, status) = self::status(&folder.path, &["--session", "000000000000"]);
    assert_eq!(status, Some(1), "{}", err);
    assert_eq!(out, "");
    assert!(err.contains("no stamp starts with 000000000000"), "{}", err);
    // More digits than a hash has name nothing either.
    let (out, err, status) = self::status(&folder.path, &["--session", &"a".repeat(65)]);
    assert_eq!(status, Some(1), "{}", err);
    assert_eq!(out, "");
}

#[test]
fn test_WB_STATUS_50_a_hash_longer_than_the_hash_of_an_object_names_nothing() {
    // Git cuts a prefix down to the length of its hashes, so the stamp and four zeros would name the stamp. No object's
    // name starts with them: it ends with 1, and prints nothing.
    let (folder, session) = with_two_commits();
    assert_eq!(session.stamp.len(), 40, "the test is of a SHA-1 repository");
    for longer in [format!("{}0000", session.stamp), format!("{}{}", session.stamp, "0".repeat(24)), format!("{}{}", session.stamp, "f")] {
        let (out, err, status) = self::status(&folder.path, &["--session", &longer]);
        assert_eq!(status, Some(1), "{}: {}", longer, err);
        assert_eq!(out, "", "{}", longer);
        assert!(err.contains(&format!("no stamp starts with {}", longer)), "{}", err);
    }
}

#[test]
fn test_WB_STATUS_50_a_prefix_that_starts_two_objects_ends_with_1_and_says_it_is_not_unique() {
    let (folder, session) = with_two_commits();
    // A second object that starts as the stamp does: a file of the object store under the same first digits. Git lists
    // an object by its name when it is asked what a prefix may mean.
    let (dir, rest) = session.stamp.split_at(2);
    let objects = PathBuf::from(git_text(&folder.path, &["rev-parse", "--absolute-git-dir"])).join("objects").join(dir);
    fs::create_dir_all(&objects).unwrap();
    fs::write(objects.join(format!("{}{}", &rest[..10], "0".repeat(rest.len() - 10))), b"").unwrap();
    let (out, err, status) = self::status(&folder.path, &["--session", &session.stamp[..12]]);
    assert_eq!(status, Some(1), "{}", err);
    assert_eq!(out, "");
    assert!(err.contains(&format!("{} is not unique", &session.stamp[..12])), "{}", err);
    assert!(err.contains(&session.stamp), "{}", err);
    // The whole hash is not ambiguous.
    let (_, err, status) = self::status(&folder.path, &["--session", &session.stamp]);
    assert_eq!(status, Some(0), "{}", err);
}

#[test]
fn test_WB_STATUS_50_what_is_not_a_stamp_that_counts_is_named_for_what_it_is_and_refused() {
    let (folder, session) = with_two_commits();
    // The commit the stamp was made from, a commit taken back, a marker, and the tree of the stamp.
    stamps_on_ref(&folder.path, "12", &["[revert] tester round 1\n"]);
    let marker = git_text(&folder.path, &["rev-parse", "refs/watchbill/12"]);
    let tree = git_text(&folder.path, &["rev-parse", &format!("{}^{{tree}}", session.stamp)]);
    for (hash, what) in [(&session.base, "is not on a job's ref (its message names no issue)"), (&session.commits[0], "is not on a job's ref (its message names no issue)"), (&marker, "is a marker"), (&tree, "is a tree, not a commit")] {
        let (out, err, status) = self::status(&folder.path, &["--session", hash]);
        assert_eq!(status, Some(1), "{}: {}", hash, err);
        assert_eq!(out, "", "{}", hash);
        assert!(err.contains(what), "{} {}: {}", hash, what, err);
    }
}

#[test]
fn test_WB_STATUS_50_a_stamp_that_does_not_count_is_refused_with_why_and_a_note_makes_it_count_for_nothing() {
    let folder = set_up();
    // Unsigned: it counts for nothing, so nothing of it is printed, whatever notes name it.
    unsigned_stamps_on_ref(&folder.path, "12", &["[stamp] tester round 1\ntime: 2026-11-24T10:00:00Z\n"]);
    let stamp = git_text(&folder.path, &["rev-parse", "refs/watchbill/12"]);
    let commits = commits_with_notes(&folder.path, "12", &stamp, "tester", &["Write the first test"]);
    let (out, err, status) = self::status(&folder.path, &["--session", &stamp]);
    assert_eq!(status, Some(1), "{}", err);
    assert_eq!(out, "", "nothing of a stamp that does not count is printed");
    assert!(err.contains("does not count"), "{}", err);
    assert!(!out.contains(&commits[0]));
}

#[test]
fn test_WB_STATUS_50_a_ref_that_is_not_as_wb_writes_it_shows_no_session() {
    let (folder, session) = with_two_commits();
    // A commit that nobody signed after the stamp: the ref is not as `wb` writes it, and nothing on it counts.
    unsigned_stamps_on_ref(&folder.path, "12", &["[stamp] build round 1\ntime: 2026-11-24T11:00:00Z\n"]);
    let (out, err, status) = self::status(&folder.path, &["--session", &session.stamp]);
    assert_eq!(status, Some(1), "{}", err);
    assert_eq!(out, "");
    assert!(err.contains("refs/watchbill/12 is not as wb writes it"), "{}", err);
}

#[test]
fn test_WB_STATUS_50_a_hash_that_only_a_note_calls_a_stamp_shows_no_session() {
    let (folder, _) = with_two_commits();
    // A note names a commit of the checkout as a stamp: the note is never read to find a stamp.
    let head = git_text(&folder.path, &["rev-parse", "HEAD"]);
    note(&folder.path, &head, &head, "tester");
    let (out, err, status) = self::status(&folder.path, &["--session", &head]);
    assert_eq!(status, Some(1), "{}", err);
    assert_eq!(out, "");
    assert!(err.contains("is not on a job's ref (its message names no issue)"), "{}", err);
}

// ---- WB-STATUS-51: it writes nothing and asks nothing of the network ----

fn tree(folder: &Path) -> Vec<(String, u64)> {
    let mut found = Vec::new();
    let mut todo = vec![folder.to_path_buf()];
    while let Some(path) = todo.pop() {
        for entry in fs::read_dir(&path).unwrap() {
            let entry = entry.unwrap();
            let meta = entry.metadata().unwrap();
            if meta.is_dir() {
                todo.push(entry.path());
            } else {
                found.push((entry.path().display().to_string(), meta.len()));
            }
        }
    }
    found.sort();
    found
}

#[test]
fn test_WB_STATUS_51_it_writes_nothing() {
    let (folder, session) = with_two_commits();
    let state = folder_named("state");
    lines_of_start(&state.path, &message_of(&folder.path, &session.stamp), "tester", "12", "outcome: ended\nstatus: 0\nseconds: 5\n");
    // The whole of the repository, its refs and objects too, and the state folder.
    let before = (tree(&folder.path), tree(&state.path), git_text(&folder.path, &["for-each-ref"]), git_text(&folder.path, &["reflog"]));
    for (hash, wanted) in [(&session.stamp[..12], Some(0)), (&session.stamp[..], Some(0)), ("000000000000", Some(1))] {
        let (_, err, status) = status_in(&folder.path, &state.path, hash);
        assert_eq!(status, wanted, "{}: {}", hash, err);
    }
    let (_, _, status) = self::status(&folder.path, &["--session", "abc"]);
    assert_eq!(status, Some(2));
    let after = (tree(&folder.path), tree(&state.path), git_text(&folder.path, &["for-each-ref"]), git_text(&folder.path, &["reflog"]));
    assert!(before == after, "a file, a ref or a reflog entry was written");
}

#[test]
fn test_WB_STATUS_51_it_asks_nothing_of_the_network_and_gives_the_same_output_with_origin_gone_or_there() {
    let (folder, session) = with_two_commits();
    let (without, err, status) = self::status(&folder.path, &["--session", &session.stamp]);
    assert_eq!(status, Some(0), "{}", err);
    // An origin that would say so if it were asked anything: the program that serves a fetch leaves a mark.
    let bare = folder.path.parent().unwrap().join("origin.git");
    git(&folder.path, &["init", "-q", "--bare", bare.to_str().unwrap()]);
    let mark = folder.path.parent().unwrap().join("asked");
    let program = folder.path.parent().unwrap().join("serve");
    fs::write(&program, format!("#!/bin/sh\ntouch '{}'\nexit 1\n", mark.display())).unwrap();
    fs::set_permissions(&program, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    git(&folder.path, &["remote", "add", "origin", bare.to_str().unwrap()]);
    git(&folder.path, &["config", "remote.origin.uploadpack", program.to_str().unwrap()]);
    git(&folder.path, &["config", "remote.origin.receivepack", program.to_str().unwrap()]);
    let (with, err, status) = self::status(&folder.path, &["--session", &session.stamp]);
    assert_eq!(status, Some(0), "{}", err);
    assert_eq!(without, with);
    assert!(!mark.exists(), "origin was asked something");
    // And the code names no way to the network.
    let source = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/session_status.rs")).unwrap();
    let code: String = source.lines().take_while(|line| !line.starts_with("#[cfg(test)]")).collect::<Vec<_>>().join("\n");
    for word in ["TcpStream", "UdpSocket", "curl", "fetch\"", "ls-remote", "\"push\"", "\"pull\"", "ssh\""] {
        assert!(!code.contains(word), "src/session_status.rs names {}", word);
    }
}

#[test]
fn test_WB_STATUS_51_the_git_it_runs_reads_no_replace_ref_and_no_lazy_fetch() {
    // Every git of the module goes through the helpers that turn replace refs, grafts and the lazy fetch off.
    let source = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/session_status.rs")).unwrap();
    let code: String = source.lines().take_while(|line| !line.starts_with("#[cfg(test)]")).collect::<Vec<_>>().join("\n");
    assert!(!code.contains("Command::new"), "src/session_status.rs runs a process itself, not through signers::git");
    assert!(code.contains("signers::git"), "src/session_status.rs reads git through signers::git");
}
