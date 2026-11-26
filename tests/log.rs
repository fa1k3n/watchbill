//! `wb log <issue>` (#177): the history of an issue in the flow, from the job's ref and git alone. Each step is a
//! commit of `refs/watchbill/<issue>` that counts (#236, #238, #327): a stamp, a marker of `wb revert`, a commit of
//! questions or of answers. The commits taken back are named by notes (#237), which are not signed, and the outcome
//! of a session is read from the lines of its start on this machine (#124). Run as a person runs it, in a repository
//! made for the test; the commits are written as the tests of the ref write them (`stamps_on_ref`).

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const NOTES: &str = "refs/notes/watchbill";

/// A repository set up by `wb init --yes`, with a git identity and the keys of the test: Ada may start, accept and
/// answer.
fn set_up() -> Folder {
    let folder = repository(None);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    git(&folder.path, &["add", "--", ".watchbill"]);
    git(&folder.path, &["commit", "-q", "-m", "Set up"]);
    signing_as(&folder.path, "accept,start,answer");
    folder
}

fn git_text(folder: &Path, args: &[&str]) -> String {
    let run = Command::new("git").arg("-C").arg(folder).args(args).env_clear().envs(env()).stderr(Stdio::null()).output().expect("git could not be started");
    text(&run.stdout).trim().to_string()
}

fn tip(folder: &Path, issue: &str) -> String {
    git_text(folder, &["rev-parse", &format!("refs/watchbill/{}", issue)])
}

fn stamp_text(role: &str, round: u32, model: &str, time: &str) -> String {
    format!("[stamp] {} round {}\nmodel: {}\neffort: medium\nrevision: 0123456789abcdef0123456789abcdef\ntime: {}\n", role, round, model, time)
}

/// `wb log` as a person runs it, with a home, a config and a state folder of the test.
fn log_in(folder: &Path, state: &Path, args: &[&str]) -> (String, String, Option<i32>) {
    let home = folder.parent().unwrap().join("home");
    fs::create_dir_all(&home).unwrap();
    let mut all = vec!["log"];
    all.extend(args);
    let run = Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(&all)
        .current_dir(folder)
        .env_clear()
        .envs(env().into_iter().filter(|(name, _)| !["XDG_STATE_HOME", "HOME", "XDG_CONFIG_HOME", "GIT_CONFIG_GLOBAL"].contains(&name.as_str())))
        .env("XDG_STATE_HOME", state)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .stdin(Stdio::null())
        .output()
        .expect("wb could not be started");
    (text(&run.stdout), text(&run.stderr), code(&run))
}

fn log(folder: &Path, args: &[&str]) -> (String, String, Option<i32>) {
    log_in(folder, Path::new(NO_STATE), args)
}

/// Issue 12: a tester's stamp, its questions, the answers, and a build's stamp.
fn with_history() -> (Folder, Vec<String>) {
    let folder = set_up();
    let mut commits = Vec::new();
    let messages = [
        stamp_text("tester", 1, "model-a", "2026-11-24T10:00:00Z"),
        "[questions] 12.tester q1\n\nq1: Which unit?\n".to_string(),
        "[answers] 12.tester q1>tester\n\nq1: millimetres\n".to_string(),
        stamp_text("build", 1, "model-b", "2026-11-24T11:00:00Z"),
    ];
    for message in &messages {
        stamps_on_ref(&folder.path, "12", &[message]);
        commits.push(tip(&folder.path, "12"));
    }
    (folder, commits)
}

fn note(folder: &Path, object: &str, stamp: &str, role: &str) {
    git(folder, &["notes", &format!("--ref={}", NOTES), "add", "-m", &format!("stamp: {}\nrole: {}", stamp, role), object]);
}

/// Commits on a branch of the issue, one for each subject, as a take-back leaves them, each with a note naming `stamp`.
fn commits_with_notes(folder: &Path, issue: &str, stamp: &str, role: &str, subjects: &[&str]) -> Vec<String> {
    let back = git_text(folder, &["rev-parse", "--abbrev-ref", "HEAD"]);
    git(folder, &["checkout", "-q", "-b", &format!("issue-{}-work", issue)]);
    let mut commits = Vec::new();
    for subject in subjects {
        git(folder, &["commit", "-q", "--allow-empty", "-m", subject]);
        let commit = git_text(folder, &["rev-parse", "HEAD"]);
        note(folder, &commit, stamp, role);
        commits.push(commit);
    }
    git(folder, &["checkout", "-q", &back]);
    commits
}

/// The lines of one start, as `wb start` leaves them below `state`.
fn lines_of_start(state: &Path, stamp_text: &str, role: &str, issue: &str, outcome: &str) {
    let lines = state.join("watchbill/sessions").join(format!("2026-11-24T100000Z-{}-{}", role, issue));
    fs::create_dir_all(&lines).unwrap();
    fs::write(lines.join("stamp"), stamp_text).unwrap();
    fs::write(lines.join("outcome"), outcome).unwrap();
}

fn message_of(folder: &Path, commit: &str) -> String {
    let raw = git_text(folder, &["cat-file", "commit", commit]);
    raw.split_once("\n\n").map(|(_, message)| format!("{}\n", message)).unwrap_or_default()
}

// ---- WB-STATUS-97: one entry for each commit that counts, oldest first, with what a stamp says ----

#[test]
fn test_WB_STATUS_97_each_commit_that_counts_is_an_entry_oldest_first() {
    let (folder, commits) = with_history();
    let (out, err, status) = log(&folder.path, &["12"]);
    assert_eq!(status, Some(0), "{}", err);
    let at: Vec<usize> = commits.iter().map(|commit| out.find(commit.as_str()).unwrap_or_else(|| panic!("{} is not in {}", commit, out))).collect();
    assert!(at.windows(2).all(|pair| pair[0] < pair[1]), "not oldest first: {}", out);
    let headers: Vec<&str> = out.lines().filter(|line| !line.starts_with(' ') && line.contains(' ') && commits.iter().any(|commit| line.ends_with(commit.as_str()))).collect();
    assert_eq!(headers, [format!("stamp {}", commits[0]), format!("questions {}", commits[1]), format!("answers {}", commits[2]), format!("stamp {}", commits[3])]);
}

#[test]
fn test_WB_STATUS_97_a_stamp_says_its_role_round_model_effort_revision_and_time() {
    let (folder, _) = with_history();
    let (out, _, _) = log(&folder.path, &["12"]);
    let lines: Vec<&str> = out.lines().collect();
    for wanted in [
        "    Role: tester round 1",
        "    Model: model-a, effort medium",
        "    Revision: 0123456789abcdef0123456789abcdef",
        "    Time: 2026-11-24T10:00:00Z",
        "    Role: build round 1",
        "    Model: model-b, effort medium",
        "    Time: 2026-11-24T11:00:00Z",
    ] {
        assert!(lines.contains(&wanted), "{} is not in {}", wanted, out);
    }
    assert!(lines.iter().any(|line| line.starts_with("    Signed: by ada@example.org")), "{}", out);
}

#[test]
fn test_WB_STATUS_97_the_questions_and_the_answers_are_entries_that_name_what_they_ask_and_answer() {
    let (folder, _) = with_history();
    let (out, _, _) = log(&folder.path, &["12"]);
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines.contains(&"    Questions: 12.tester q1"), "{}", out);
    assert!(lines.contains(&"    Answers: 12.tester q1>tester"), "{}", out);
    // What a question says counts for nothing, and is not printed.
    assert!(!out.contains("Which unit?") && !out.contains("millimetres"), "{}", out);
}

// ---- WB-STATUS-98: the outcome, the commits taken back, a marker and what it took back ----

#[test]
fn test_WB_STATUS_98_a_session_that_ran_here_says_how_it_ended_and_any_other_is_not_known() {
    let (folder, commits) = with_history();
    let state = folder_named("state");
    lines_of_start(&state.path, &message_of(&folder.path, &commits[0]), "tester", "12", "outcome: ended\nstatus: 0\nseconds: 5\n");
    let (out, err, status) = log_in(&folder.path, &state.path, &["12"]);
    assert_eq!(status, Some(0), "{}", err);
    let lines: Vec<&str> = out.lines().collect();
    let first = lines.iter().position(|line| line.starts_with("    Ended") && line.contains("diagnostics")).unwrap_or_else(|| panic!("{}", out));
    assert_eq!(&lines[first + 1..first + 4], ["      outcome: ended", "      status: 0", "      seconds: 5"], "{}", out);
    // The build's stamp has no lines on this machine.
    assert_eq!(lines.iter().filter(|line| **line == "    Ended: not known on this machine").count(), 1, "{}", out);
}

#[test]
fn test_WB_STATUS_98_the_commits_that_a_note_names_are_said_to_be_notes_and_are_no_entry() {
    let (folder, commits) = with_history();
    let taken = commits_with_notes(&folder.path, "12", &commits[0], "tester", &["Write the first test", "Write the second test"]);
    // A note that names the stamp of the build does not name these for the tester.
    let other = commits_with_notes(&folder.path, "12", &commits[3], "build", &["Build it"]);
    let (out, err, status) = log(&folder.path, &["12"]);
    assert_eq!(status, Some(0), "{}", err);
    let lines: Vec<&str> = out.lines().collect();
    let head = lines.iter().position(|line| line.starts_with("    Taken back") && line.contains(NOTES) && line.contains("not signed")).unwrap_or_else(|| panic!("{}", out));
    assert_eq!(lines[head + 1], format!("      {}  Write the first test", taken[0]), "{}", out);
    assert_eq!(lines[head + 2], format!("      {}  Write the second test", taken[1]), "{}", out);
    assert!(out.contains(&format!("      {}  Build it", other[0])), "{}", out);
    // Not an entry: no header of its own, and the entries are as many as before.
    let entries = out.lines().filter(|line| line.starts_with("stamp ") || line.starts_with("questions ") || line.starts_with("answers ") || line.starts_with("marker ")).count();
    assert_eq!(entries, 4, "{}", out);
}

#[test]
fn test_WB_STATUS_98_a_marker_says_what_it_took_back_and_what_it_took_back_says_so() {
    let (folder, commits) = with_history();
    stamps_on_ref(&folder.path, "12", &["[revert] build round 1\n"]);
    let marker = tip(&folder.path, "12");
    let (out, err, status) = log(&folder.path, &["12"]);
    assert_eq!(status, Some(0), "{}", err);
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines.contains(&format!("marker {}", marker).as_str()), "{}", out);
    assert!(lines.contains(&"    Role: build round 1"), "{}", out);
    assert!(lines.contains(&format!("      {}  [stamp] build round 1", commits[3]).as_str()), "{}", out);
    assert!(lines.iter().any(|line| line.starts_with("    Took back")), "{}", out);
    // The stamp is still in the history, said to be taken back; the tester's is not.
    let build = lines.iter().position(|line| *line == format!("stamp {}", commits[3])).unwrap();
    assert!(lines[build + 1..].iter().take_while(|line| line.starts_with(' ')).any(|line| line.starts_with("    Taken back: by a [revert] marker")), "{}", out);
    let tester = lines.iter().position(|line| *line == format!("stamp {}", commits[0])).unwrap();
    assert!(!lines[tester + 1..].iter().take_while(|line| line.starts_with(' ')).any(|line| line.starts_with("    Taken back: by")), "{}", out);
}

// ---- WB-STATUS-99: what does not count is no entry, and a ref that is not as wb writes it shows none ----

#[test]
fn test_WB_STATUS_99_a_ref_that_is_not_as_wb_writes_it_prints_no_entry_and_says_so() {
    let (folder, _) = with_history();
    unsigned_stamps_on_ref(&folder.path, "12", &["[stamp] build round 2\ntime: 2026-11-24T12:00:00Z\n"]);
    let (out, err, status) = log(&folder.path, &["12"]);
    assert_eq!(status, Some(1), "{}", err);
    assert_eq!(out, "");
    assert!(err.contains("refs/watchbill/12 is not as wb writes it"), "{}", err);
}

#[test]
fn test_WB_STATUS_99_a_stamp_of_another_issue_is_no_entry_and_the_exit_status_is_1() {
    let (folder, _) = with_history();
    stamps_on_ref(&folder.path, "12", &["[stamp] build round 2\nissue: 99\ntime: 2026-11-24T12:00:00Z\n"]);
    let strange = tip(&folder.path, "12");
    let (out, err, status) = log(&folder.path, &["12"]);
    assert_eq!(status, Some(1), "{}", err);
    assert!(!out.contains(&strange), "{}", out);
    assert!(err.contains("not counted") || err.contains("not as wb writes it"), "{}", err);
}

#[test]
fn test_WB_STATUS_99_an_issue_with_no_ref_has_no_history_and_ends_with_1() {
    let folder = set_up();
    let (out, err, status) = log(&folder.path, &["12"]);
    assert_eq!(status, Some(1), "{}", err);
    assert_eq!(out, "");
    assert!(err.contains("refs/watchbill/12 holds no commit"), "{}", err);
}

// ---- WB-STATUS-100: --oneline ----

#[test]
fn test_WB_STATUS_100_oneline_gives_one_line_to_each_entry_and_no_other_word() {
    let (folder, commits) = with_history();
    stamps_on_ref(&folder.path, "12", &["[revert] build round 1\n"]);
    let marker = tip(&folder.path, "12");
    commits_with_notes(&folder.path, "12", &commits[0], "tester", &["Write the first test"]);
    let (out, err, status) = log(&folder.path, &["--oneline", "12"]);
    assert_eq!(status, Some(0), "{}", err);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 5, "{}", out);
    assert_eq!(lines[0], format!("{} stamp tester round 1, model model-a, effort medium, revision 0123456789ab, time 2026-11-24T10:00:00Z, ended not known on this machine, 1 commit taken back (from notes)", &commits[0][..12]));
    assert_eq!(lines[1], format!("{} questions 12.tester q1", &commits[1][..12]));
    assert_eq!(lines[2], format!("{} answers 12.tester q1>tester", &commits[2][..12]));
    assert!(lines[3].starts_with(&format!("{} stamp build round 1,", &commits[3][..12])) && lines[3].contains("taken back by a [revert] marker"), "{}", out);
    assert_eq!(lines[4], format!("{} marker build round 1, took back 1", &marker[..12]));
    // The options come in either order.
    let (again, _, _) = log(&folder.path, &["12", "--oneline"]);
    assert_eq!(again, out);
}

// ---- WB-STATUS-101: it only reads ----

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
fn test_WB_STATUS_101_it_writes_nothing() {
    let (folder, commits) = with_history();
    commits_with_notes(&folder.path, "12", &commits[0], "tester", &["Write the first test"]);
    let state = folder_named("state");
    lines_of_start(&state.path, &message_of(&folder.path, &commits[0]), "tester", "12", "outcome: ended\nstatus: 0\nseconds: 5\n");
    let before = (tree(&folder.path), tree(&state.path), git_text(&folder.path, &["for-each-ref"]), git_text(&folder.path, &["reflog"]));
    for args in [&["12"][..], &["--oneline", "12"][..], &["13"][..], &[][..]] {
        let (_, err, _) = log_in(&folder.path, &state.path, args);
        assert!(!err.contains("panicked"), "{}", err);
    }
    let after = (tree(&folder.path), tree(&state.path), git_text(&folder.path, &["for-each-ref"]), git_text(&folder.path, &["reflog"]));
    assert!(before == after, "a file, a ref or a reflog entry was written");
}

#[test]
fn test_WB_STATUS_101_it_asks_nothing_of_the_network_and_nothing_of_a_tracker() {
    let (folder, _) = with_history();
    let (without, err, status) = log(&folder.path, &["12"]);
    assert_eq!(status, Some(0), "{}", err);
    let bare = folder.path.parent().unwrap().join("origin.git");
    git(&folder.path, &["init", "-q", "--bare", bare.to_str().unwrap()]);
    let mark = folder.path.parent().unwrap().join("asked");
    let program = folder.path.parent().unwrap().join("serve");
    fs::write(&program, format!("#!/bin/sh\ntouch '{}'\nexit 1\n", mark.display())).unwrap();
    fs::set_permissions(&program, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    git(&folder.path, &["remote", "add", "origin", bare.to_str().unwrap()]);
    git(&folder.path, &["config", "remote.origin.uploadpack", program.to_str().unwrap()]);
    git(&folder.path, &["config", "remote.origin.receivepack", program.to_str().unwrap()]);
    let (with, err, status) = log(&folder.path, &["12"]);
    assert_eq!(status, Some(0), "{}", err);
    assert_eq!(without, with);
    assert!(!mark.exists(), "origin was asked something");
    let source = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/log.rs")).unwrap();
    let code: String = source.lines().take_while(|line| !line.starts_with("#[cfg(test)]")).collect::<Vec<_>>().join("\n");
    for word in ["TcpStream", "UdpSocket", "curl", "fetch\"", "ls-remote", "\"push\"", "\"pull\"", "ssh\"", "issue_comments", "pattern_of"] {
        assert!(!code.contains(word), "src/log.rs names {}", word);
    }
}

#[test]
fn test_WB_STATUS_101_the_git_it_runs_reads_no_replace_ref_and_no_lazy_fetch() {
    let source = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/log.rs")).unwrap();
    let code: String = source.lines().take_while(|line| !line.starts_with("#[cfg(test)]")).collect::<Vec<_>>().join("\n");
    assert!(!code.contains("Command::new"), "src/log.rs runs a process itself, not through signers::git");
    assert!(code.contains("stamps::read"), "src/log.rs reads the ref through stamps::read");
}

// ---- WB-STATUS-102: used wrongly, and not set up ----

#[test]
fn test_WB_STATUS_102_a_command_used_wrongly_ends_with_2_and_prints_nothing_to_standard_output() {
    let (folder, _) = with_history();
    for args in [
        &[][..],
        &["12", "13"][..],
        &["--oneline"][..],
        &["--oneline", "--oneline", "12"][..],
        &["-q", "--quiet", "12"][..],
        &["--nothing", "12"][..],
        &[".hidden"][..],
        &["a/b"][..],
    ] {
        let (out, err, status) = log(&folder.path, args);
        assert_eq!(status, Some(2), "{:?}: {}", args, err);
        assert_eq!(out, "", "{:?}", args);
        assert!(err.starts_with("wb log: "), "{:?}: {}", args, err);
    }
}

#[test]
fn test_WB_STATUS_102_quiet_is_taken_and_changes_nothing() {
    let (folder, _) = with_history();
    let (loud, _, _) = log(&folder.path, &["12"]);
    let (quiet, err, status) = log(&folder.path, &["-q", "12"]);
    assert_eq!(status, Some(0), "{}", err);
    assert_eq!(loud, quiet);
}

#[test]
fn test_WB_STATUS_102_a_folder_that_is_not_in_a_repository_or_not_set_up_ends_with_2() {
    let plain = folder();
    let (out, err, status) = log(&plain.path, &["12"]);
    assert_eq!((status, out.as_str()), (Some(2), ""), "{}", err);
    assert!(err.contains("not in a git repository"), "{}", err);
    let bare = repository(None);
    git(&bare.path, &["config", "user.name", "Ada Lovelace"]);
    git(&bare.path, &["config", "user.email", "ada@example.org"]);
    git(&bare.path, &["commit", "-q", "--allow-empty", "-m", "Nothing"]);
    let (out, err, status) = log(&bare.path, &["12"]);
    assert_eq!((status, out.as_str()), (Some(2), ""), "{}", err);
    assert!(err.contains("wb log: ") && err.contains("not set up"), "{}", err);
}

#[test]
fn test_WB_STATUS_102_the_help_of_log_gives_its_usage_and_it_is_no_longer_planned() {
    let folder = folder();
    let run = wb(&folder.path, &["help", "log"], "");
    assert_eq!(code(&run), Some(0));
    let out = text(&run.stdout);
    assert!(out.contains("wb log <issue> [--oneline]"), "{}", out);
    assert!(!out.contains("planned"), "{}", out);
}
