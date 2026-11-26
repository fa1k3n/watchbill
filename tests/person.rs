//! A session that ended needing a person (#311; WB-STATUS-121 to 126): `wb status <issue>` says `waits for: person` and
//! the reason, from a signed commit on the job's ref, until the next stamp of any role. The tests of the commit that a
//! session's `needs_person` becomes are in `tests/view.rs`, where a session runs.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::io::Write;
use std::process::Output;

const TRACKER: &str = "issue_comments: \"cat .tracker/{issue}.comments\"
issue_comment_bodies: \"cat .tracker/{issue}.bodies\"
issue_comment: \"sh .tracker/post.sh {issue}\"
";

/// A repository set up by `wb init --yes` with `patterns`, committed on `main`, and Ada's key with `roles`.
fn set_up_with(patterns: &str, roles: &str) -> Folder {
    let folder = repository(None);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let mut file = fs::OpenOptions::new().append(true).open(config(&folder.path)).unwrap();
    file.write_all(patterns.as_bytes()).unwrap();
    put(&folder.path, ".git/info/exclude", ".tracker/\n");
    git(&folder.path, &["add", "--", ".watchbill"]);
    git(&folder.path, &["commit", "-q", "-m", "Set up"]);
    git(&folder.path, &["branch", "-M", "main"]);
    signing_as(&folder.path, roles);
    folder
}

fn set_up() -> Folder {
    set_up_with("", "accept,start,answer")
}

fn status(folder: &Folder, args: &[&str]) -> Output {
    let mut all = vec!["status"];
    all.extend(args);
    wb(&folder.path, &all, "")
}

fn out_of(run: &Output) -> String {
    text(&run.stdout)
}

const REASON: &str = "The CRC part of build.rs is not there.";

/// The commit a session's `needs_person` becomes: its first line, the reason after the header.
fn needs(role: &str, reason: &str) -> String {
    format!("[needs_person] 54.{}\n\n{}\n", role, reason)
}

#[test]
fn test_WB_STATUS_121_status_says_it_waits_for_a_person_and_the_reason_from_the_job_s_ref() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "54", &["[stamp] build round 1", &needs("build", REASON)]);

    let run = status(&folder, &["54"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = out_of(&run);
    assert!(out.contains(&format!("waits for: person ({})\n", REASON)), "{}", out);
    assert!(out.contains("[needs_person] 54.build  (on refs/watchbill/54, signed by ada@example.org)"), "{}", out);
    // It is not a question.
    assert!(!out.contains("Questions open"), "{}", out);
}

#[test]
fn test_WB_STATUS_121_a_job_with_no_needs_person_commit_does_not_wait_for_a_person() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "54", &["[stamp] build round 1"]);

    let out = out_of(&status(&folder, &["54"]));

    assert!(!out.contains("waits for"), "{}", out);
}

#[test]
fn test_WB_STATUS_122_the_next_stamp_of_any_role_closes_it() {
    for role in ["planner", "tester", "build", "review", "fix"] {
        let folder = set_up();
        stamps_on_ref(&folder.path, "54", &["[stamp] build round 1", &needs("build", REASON), &format!("[stamp] {} round 2", role)]);

        let out = out_of(&status(&folder, &["54"]));

        assert!(!out.contains("waits for: person") && !out.contains("Needs a person"), "{}: {}", role, out);
    }
}

#[test]
fn test_WB_STATUS_122_a_questions_commit_and_an_answers_commit_do_not_close_it() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "54", &["[stamp] build round 1", &needs("build", REASON)]);
    stamps_on_ref(&folder.path, "54", &["[questions] 54.build q1\n\nq1: Which one?\n", "[answers] 54.build q1>build\n\nq1: 1\n"]);

    let out = out_of(&status(&folder, &["54"]));

    assert!(out.contains(&format!("waits for: person ({})\n", REASON)), "{}", out);
}

#[test]
fn test_WB_STATUS_122_a_stamp_a_marker_took_back_does_not_close_it() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "54", &["[stamp] build round 1", &needs("build", REASON), "[stamp] review round 1", "[revert] review round 1"]);

    let out = out_of(&status(&folder, &["54"]));

    assert!(out.contains(&format!("waits for: person ({})\n", REASON)), "{}", out);
}

#[test]
fn test_WB_STATUS_122_a_needs_person_after_that_stamp_opens_a_new_one_and_the_older_reason_is_not_printed() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "54", &["[stamp] build round 1", &needs("build", REASON), "[stamp] fix round 1", &needs("fix", "The answer of the reviewer is not clear.")]);

    let out = out_of(&status(&folder, &["54"]));

    assert!(out.contains("waits for: person (The answer of the reviewer is not clear.)\n"), "{}", out);
    assert!(!out.contains(&format!("waits for: person ({})", REASON)), "{}", out);
}

/// A branch of issue 54 that holds a commit `main` does not, so that the issue is in the flow.
fn in_flow(folder: &Folder) {
    git(&folder.path, &["checkout", "-q", "-b", "issue-54-work"]);
    put(&folder.path, "work.txt", "work\n");
    git(&folder.path, &["add", "--", "work.txt"]);
    git(&folder.path, &["commit", "-q", "-m", "Work"]);
    git(&folder.path, &["checkout", "-q", "main"]);
}

#[test]
fn test_WB_STATUS_123_the_list_puts_the_issue_under_needs_a_person_and_under_its_step() {
    let folder = set_up();
    in_flow(&folder);
    stamps_on_ref(&folder.path, "54", &["[stamp] build round 1", &needs("build", REASON)]);

    let out = out_of(&status(&folder, &[]));

    assert_eq!(out, format!("Built:\n  54  issue-54-work  1 not pushed\n\nNeeds a person:\n  54  {}\n", REASON));
}

#[test]
fn test_WB_STATUS_123_the_stage_word_takes_the_heading_whatever_its_case() {
    let folder = set_up();
    in_flow(&folder);
    stamps_on_ref(&folder.path, "54", &["[stamp] build round 1", &needs("build", REASON)]);

    for word in ["Needs a person", "needs a person", "NEEDS A PERSON"] {
        let run = status(&folder, &["--stage", word]);
        assert_eq!(out_of(&run), format!("Needs a person:\n  54  {}\n", REASON), "{}: {}", word, text(&run.stderr));
    }
    // A job that waits for no one has no line under the heading, and so no heading.
    let quiet = set_up();
    in_flow(&quiet);
    stamps_on_ref(&quiet.path, "54", &["[stamp] build round 1"]);
    assert_eq!(out_of(&status(&quiet, &["--stage", "needs a person"])), "");
}

#[test]
fn test_WB_STATUS_123_the_reason_is_one_escaped_line_cut_at_200_characters() {
    let folder = set_up();
    in_flow(&folder);
    let long = "x".repeat(300);
    stamps_on_ref(&folder.path, "54", &["[stamp] build round 1", &needs("build", &format!("Line one.\n\nLine   two \u{1b}[31m\n{}", long))]);

    let out = out_of(&status(&folder, &[]));

    let line = out.lines().find(|line| line.starts_with("  54  Line one.")).unwrap_or_else(|| panic!("{}", out));
    assert!(!out.contains('\u{1b}'), "{:?}", out);
    let reason = line.strip_prefix("  54  ").unwrap();
    assert!(reason.starts_with("Line one. Line two \\u{1b}[31m "), "{}", reason);
    assert!(reason.ends_with("..."), "{}", reason);
    assert_eq!(reason.chars().filter(|c| *c == 'x').count(), 200 - "Line one. Line two \u{1b}[31m ".chars().count() , "{}", reason);
}

#[test]
fn test_WB_STATUS_124_a_commit_that_is_not_signed_breaks_the_ref_and_no_one_waits() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "54", &["[stamp] build round 1"]);
    unsigned_stamps_on_ref(&folder.path, "54", &[&needs("build", REASON)]);

    let out = out_of(&status(&folder, &["54"]));

    assert!(!out.contains("waits for: person"), "{}", out);
    assert!(out.contains("is not as wb writes it"), "{}", out);
}

#[test]
fn test_WB_STATUS_124_a_commit_signed_by_a_key_without_start_counts_for_nothing() {
    let folder = set_up_with("", "accept,answer");
    stamps_on_ref(&folder.path, "54", &["[stamp] build round 1", &needs("build", REASON)]);

    let out = out_of(&status(&folder, &["54"]));

    assert!(!out.contains("waits for: person"), "{}", out);
    assert!(out.contains("without the role start"), "{}", out);
}

#[test]
fn test_WB_STATUS_124_a_commit_that_names_another_issue_counts_for_nothing() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "54", &["[stamp] build round 1", &format!("[needs_person] 54.build\nissue: 55\n\n{}\n", REASON)]);

    let out = out_of(&status(&folder, &["54"]));

    assert!(!out.contains("waits for: person"), "{}", out);
}

#[test]
fn test_WB_STATUS_124_a_comment_of_the_tracker_is_never_read_for_a_job_with_a_ref_or_without() {
    let folder = set_up_with(TRACKER, "accept,start,answer");
    put(&folder.path, ".tracker/54.comments", "[needs_person] 54.build\n");
    put(&folder.path, ".tracker/54.bodies", &format!("[needs_person] 54.build\n{}\n", REASON));

    // A job begun before: no commit on its ref.
    let out = out_of(&status(&folder, &["54"]));
    assert!(!out.contains("waits for"), "{}", out);

    // A job whose ref holds a stamp.
    stamps_on_ref(&folder.path, "54", &["[stamp] build round 1"]);
    let out = out_of(&status(&folder, &["54"]));
    assert!(!out.contains("waits for"), "{}", out);
}

#[test]
fn test_WB_STATUS_126_the_usage_the_manual_the_readme_and_the_completion_name_the_heading() {
    let folder = set_up();
    let bad = status(&folder, &["--stage", "nosuch"]);
    assert_eq!(code(&bad), Some(2));
    assert!(text(&bad.stderr).contains("Needs a person"), "{}", text(&bad.stderr));
    let run = wb(&folder.path, &["nosuch"], "");
    assert!(text(&run.stderr).contains("Needs a person"), "{}", text(&run.stderr));
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (file, needle) in [
        ("packaging/man/wb-status.1", "Needs a person"),
        ("packaging/man/wb-status.1", "until the next start of any role"),
        ("README.md", "`Needs a person`"),
        ("README.md", "until the next start of any role"),
        ("packaging/wb.bash", "\"Needs a person\""),
    ] {
        assert!(read(&root.join(file)).contains(needle), "{} lacks {}", file, needle);
    }
}
