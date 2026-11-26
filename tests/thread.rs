//! Questions and answers are commits on the job's ref, with a copy posted out (#238; `docs/revision.md`, point 15).
//!
//! A question commit is written by `wb start` after a session, signed by the starter (the role `start`); an answer
//! commit is written by `wb answer`, signed by the person (the role `answer`). Both hold the tree of the commit before
//! them. `wb status` and `wb answer` read them from the ref, and read no `[questions]` or `[answers]` of the tracker
//! for a job whose ref holds a questions commit. The tracker is a folder `.tracker/` of stand-ins that git does not
//! see: never the network. The tests of the questions a start writes are in `tests/view.rs`, where a session runs.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

/// What a stand-in tracker has: the first lines and whole comments of each issue, and where a copy is posted. The
/// stand-in that posts keeps each comment whole, and the commit the ref held when it was posted.
const TRACKER: &str = "issue_comments: \"cat .tracker/{issue}.comments\"
issue_comment_bodies: \"cat .tracker/{issue}.bodies\"
issue_comment: \"sh .tracker/post.sh {issue}\"
";

const POST: &str = "n=$(ls .tracker | grep -c \"^posted-$1-\")\ncat > \".tracker/posted-$1-$n\"\ngit rev-parse -q --verify \"refs/watchbill/$1\" > \".tracker/ref-$1-$n\"\necho \"POSTED $1\" >&2\n";

const ROLES: &str = "accept,start,answer";

/// A repository set up by `wb init --yes` with the patterns given, committed on `main`, and Ada's key in the list with
/// `roles`; no list where `roles` is empty.
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
    put(&folder.path, ".tracker/post.sh", POST);
    git(&folder.path, &["add", "--", ".watchbill"]);
    git(&folder.path, &["commit", "-q", "-m", "Set up"]);
    git(&folder.path, &["branch", "-M", "main"]);
    if !roles.is_empty() {
        signing_as(&folder.path, roles);
    }
    folder
}

/// The usual: a tracker, and Ada's key with `accept`, `start` and `answer`.
fn set_up() -> Folder {
    set_up_with(TRACKER, ROLES)
}

/// Where no tracker is set up: no pattern of `watchbill.yml` reads or posts a comment.
fn no_tracker() -> Folder {
    set_up_with("", ROLES)
}

fn git_text(folder: &Path, args: &[&str]) -> String {
    let run = Command::new("git").arg("-C").arg(folder).args(args).env_clear().envs(env()).stderr(Stdio::null()).output().expect("git could not be started");
    text(&run.stdout)
}

fn tip(folder: &Path, issue: &str) -> String {
    git_text(folder, &[ "rev-parse", "-q", "--verify", &format!("refs/watchbill/{}", issue)]).trim().to_string()
}

/// The message of a commit: what follows the header, with the signature's lines in it left out.
fn message_of(folder: &Path, commit: &str) -> String {
    let raw = git_text(folder, &["cat-file", "commit", commit]);
    raw.split_once("\n\n").map(|(_, message)| message.to_string()).unwrap_or_default()
}

fn header_of(folder: &Path, commit: &str) -> String {
    git_text(folder, &["cat-file", "commit", commit]).split_once("\n\n").map(|(header, _)| header.to_string()).unwrap_or_default()
}

fn status(folder: &Path, args: &[&str]) -> Output {
    let mut all = vec!["status"];
    all.extend(args);
    wb(folder, &all, "")
}

fn answer(folder: &Path, args: &[&str], typed: &str) -> Output {
    let mut all = vec!["answer"];
    all.extend(args);
    wb(folder, &all, typed)
}

/// Two questions of the tester, the first with choices that name roles.
const ASKED: &str = "[questions] 54.tester q1 q2

q1: Which brake?
<details>

Why it matters: the test differs.

1. (tester) The front brake.
2. (person) Both.

Suggested: 1, because it is first.

</details>

q2: Which unit?
";

/// What `wb answer` writes for the two.
const ANSWERED: &str = "[answers] 54.tester q1>tester q2>tester\n\nq1: 1\nq2: millimetres\n";

fn out_of(run: &Output) -> String {
    text(&run.stdout)
}

fn err_of(run: &Output) -> String {
    text(&run.stderr)
}

// ---- WB-STATUS-43: the open questions are read from the job's ref, with no tracker ----

#[test]
fn test_WB_STATUS_43_status_shows_the_open_questions_of_the_job_s_ref_with_no_tracker_set_up() {
    let folder = no_tracker();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);

    let run = status(&folder.path, &["54"]);

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(out_of(&run).contains("questions open: 54.tester.q1, 54.tester.q2\n"), "{}", out_of(&run));
}

#[test]
fn test_WB_STATUS_43_the_commit_of_questions_is_one_of_the_lines_status_worked_it_out_from() {
    let folder = no_tracker();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);

    let run = status(&folder.path, &["54"]);

    let out = out_of(&run);
    assert!(out.contains("[questions] 54.tester q1 q2  (on refs/watchbill/54, signed by ada@example.org)"), "{}", out);
}

// ---- WB-STATUS-44: a signed commit with its answer closes the question ----

#[test]
fn test_WB_STATUS_44_a_question_is_answered_once_a_signed_commit_holds_its_answer() {
    let folder = no_tracker();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);
    stamps_on_ref(&folder.path, "54", &["[answers] 54.tester q1>tester\n\nq1: 1\n"]);

    let run = status(&folder.path, &["54"]);

    let out = out_of(&run);
    assert!(out.contains("questions open: 54.tester.q2\n"), "{}", out);
    assert!(out.contains("waits for: tester (54.tester.q1)"), "{}", out);
    stamps_on_ref(&folder.path, "54", &["[answers] 54.tester q2>tester\n\nq2: millimetres\n"]);
    let out = out_of(&status(&folder.path, &["54"]));
    assert!(!out.contains("questions open: "), "{}", out);
}

// ---- WB-STATUS-45: an answer typed at the tracker is not read, and wb status says so ----

#[test]
fn test_WB_STATUS_45_status_says_where_it_sees_an_open_question_that_an_answer_at_the_tracker_is_not_read() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);

    let run = status(&folder.path, &["54"]);

    let err = err_of(&run);
    assert!(err.contains("an answer typed at the tracker is not read") && err.contains("`wb answer 54`"), "{}", err);
    // Nothing open, nothing said.
    stamps_on_ref(&folder.path, "54", &[ANSWERED]);
    assert!(!err_of(&status(&folder.path, &["54"])).contains("typed at the tracker"));
}

#[test]
fn test_WB_STATUS_45_the_list_says_it_for_each_issue_with_an_open_question_on_its_ref_and_for_no_other() {
    let folder = set_up();
    git(&folder.path, &["checkout", "-q", "-b", "issue-54-work"]);
    git(&folder.path, &["commit", "-q", "--allow-empty", "-m", "Work"]);
    git(&folder.path, &["update-ref", "refs/remotes/origin/issue-54-work", "HEAD"]);
    git(&folder.path, &["checkout", "-q", "main"]);
    stamps_on_ref(&folder.path, "54", &[ASKED]);

    let run = status(&folder.path, &[]);

    assert!(err_of(&run).contains("issue 54 has an open question") && err_of(&run).contains("not read"), "{}", err_of(&run));
    assert!(out_of(&run).contains("\t54  issue-54-work  2 questions open\n"), "{}", out_of(&run));
}

#[test]
fn test_WB_STATUS_45_an_issue_read_from_the_tracker_is_not_told_that_its_tracker_is_not_read() {
    let folder = set_up();
    put(&folder.path, ".tracker/54.comments", "[questions] 54.tester q1\n");
    let run = status(&folder.path, &["54"]);
    assert!(out_of(&run).contains("questions open: 54.tester.q1\n"), "{}", out_of(&run));
    assert!(!err_of(&run).contains("typed at the tracker"), "{}", err_of(&run));
}

// ---- WB-ANSWER-24: a comment of the tracker answers nothing ----

#[test]
fn test_WB_ANSWER_24_an_answers_comment_on_the_tracker_alone_answers_no_question() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);
    put(&folder.path, ".tracker/54.comments", "[answers] 54.tester q1>tester q2>tester\n[answer] 54 suggested\n");

    let run = status(&folder.path, &["54"]);

    assert!(out_of(&run).contains("questions open: 54.tester.q1, 54.tester.q2\n"), "{}", out_of(&run));
    assert!(!out_of(&run).contains("waits for"), "{}", out_of(&run));
}

#[test]
fn test_WB_ANSWER_24_wb_answer_shows_the_question_a_comment_on_the_tracker_claims_to_have_answered() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);
    put(&folder.path, ".tracker/54.bodies", "[answers] 54.tester q1>tester q2>tester\nq1: 1\nq2: x\n\0");

    let run = answer(&folder.path, &["54", "--show"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(out_of(&run).contains("54.tester.q1 (1 of 2): Which brake?") && out_of(&run).contains("54.tester.q2 (2 of 2): Which unit?"), "{}", out_of(&run));
}

// ---- WB-ANSWER-25 and 26: which of the two is read ----

#[test]
fn test_WB_ANSWER_25_an_issue_with_no_job_ref_is_read_from_the_tracker_as_before() {
    let folder = set_up();
    put(&folder.path, ".tracker/54.comments", "[questions] 54.tester q1 q2\n[answers] 54.tester q1>tester\n");

    let run = status(&folder.path, &["54"]);

    assert!(out_of(&run).contains("questions open: 54.tester.q2\n"), "{}", out_of(&run));
    assert!(out_of(&run).contains("waits for: tester (54.tester.q1)"), "{}", out_of(&run));
}

#[test]
fn test_WB_ANSWER_52_an_issue_begun_before_with_comment_stamps_only_and_no_job_ref_is_read_from_the_tracker() {
    let folder = set_up();
    put(&folder.path, ".tracker/54.comments", "[stamp] tester round 1\n[questions] 54.build q1\n");

    let run = status(&folder.path, &["54"]);

    assert!(out_of(&run).contains("questions open: 54.build.q1\n"), "{}", out_of(&run));
}

#[test]
fn test_WB_ANSWER_25_wb_answer_of_an_issue_with_no_questions_commit_posts_a_comment_as_before() {
    let folder = set_up();
    put(&folder.path, ".tracker/54.bodies", &format!("{}\0", ASKED));
    let before = tip(&folder.path, "54");

    let run = answer(&folder.path, &["54", "tester.q1", "1", "tester.q2", "words"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert_eq!(tip(&folder.path, "54"), before);
    let posted = read(&folder.path.join(".tracker/posted-54-0"));
    assert_eq!(posted, "[answers] 54.tester q1>tester q2>tester\nq1: 1\nq2: words\n");
}

#[test]
fn test_WB_ANSWER_26_an_issue_whose_ref_holds_a_questions_commit_reads_no_question_or_answer_of_the_tracker() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);
    put(&folder.path, ".tracker/54.comments", "[questions] 54.build q1\n[questions] 54 q1\n");

    let run = status(&folder.path, &["54"]);

    assert!(out_of(&run).contains("questions open: 54.tester.q1, 54.tester.q2\n"), "{}", out_of(&run));
    assert!(!out_of(&run).contains("54.build.q1") && !out_of(&run).contains("54.q1"), "{}", out_of(&run));
}

#[test]
fn test_WB_ANSWER_26_a_questions_commit_that_does_not_count_still_shuts_the_tracker_out() {
    let folder = set_up();
    unsigned_stamps_on_ref(&folder.path, "54", &[ASKED]);
    put(&folder.path, ".tracker/54.comments", "[questions] 54.build q1\n");

    let run = status(&folder.path, &["54"]);

    assert!(!out_of(&run).contains("questions open: "), "{}", out_of(&run));
}

// ---- WB-ANSWER-22 and 23: wb answer writes a signed commit, pushes it and posts the copy ----

#[test]
fn test_WB_ANSWER_22_wb_answer_writes_a_signed_commit_on_the_job_s_ref_and_the_question_is_answered() {
    let folder = no_tracker();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);
    let before = tip(&folder.path, "54");

    let run = answer(&folder.path, &["54", "tester.q1", "1", "tester.q2", "millimetres"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    let now = tip(&folder.path, "54");
    assert_ne!(now, before);
    assert_eq!(git_text(&folder.path, &["rev-parse", &format!("{}^", now)]).trim(), before);
    assert!(header_of(&folder.path, &now).contains("\ngpgsig "), "{}", header_of(&folder.path, &now));
    let message = message_of(&folder.path, &now);
    assert!(message.starts_with("[answers] 54.tester q1>tester q2>tester\nissue: 54\n"), "{}", message);
    assert!(message.contains("\n\nq1: 1\nq2: millimetres\n"), "{}", message);
    assert_eq!(git_text(&folder.path, &["rev-parse", &format!("{}^{{tree}}", now)]), git_text(&folder.path, &["rev-parse", &format!("{}^{{tree}}", before)]));
    // The question is answered, with no tracker.
    let out = out_of(&status(&folder.path, &["54"]));
    assert!(!out.contains("questions open: "), "{}", out);
    assert!(out.contains("[answers] 54.tester q1>tester q2>tester  (on refs/watchbill/54, signed by ada@example.org)"), "{}", out);
}

#[test]
fn test_WB_ANSWER_22_one_commit_is_written_for_each_role_in_the_order_asked() {
    let folder = no_tracker();
    let build = "[questions] 54.build q1\n\nq1: Which unit?\n";
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED, "[stamp] build round 1", build]);
    let before = tip(&folder.path, "54");

    let run = answer(&folder.path, &["54", "--suggested", "build.q1", "ms", "tester.q2", "mm"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    let now = tip(&folder.path, "54");
    let second = message_of(&folder.path, &now);
    let first = message_of(&folder.path, git_text(&folder.path, &["rev-parse", &format!("{}^", now)]).trim());
    assert!(first.starts_with("[answers] 54.tester q1>tester q2>tester\n"), "{}", first);
    assert!(second.starts_with("[answers] 54.build q1\n") || second.starts_with("[answers] 54.build q1>build\n"), "{}", second);
    assert_eq!(git_text(&folder.path, &["rev-list", "--count", &format!("{}..{}", before, now)]).trim(), "2");
    assert!(!out_of(&status(&folder.path, &["54"])).contains("questions open: "));
}

#[test]
fn test_WB_ANSWER_22_the_commit_is_pushed_to_origin_with_no_force() {
    let folder = repository(Some("ssh://example.invalid/brakes/controller.git"));
    let bare = push_beside(&folder.path);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    git(&folder.path, &["add", "--", ".watchbill"]);
    git(&folder.path, &["commit", "-q", "-m", "Set up"]);
    git(&folder.path, &["branch", "-M", "main"]);
    signing_as(&folder.path, ROLES);
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);
    git(&folder.path, &["push", "-q", "origin", "refs/watchbill/54:refs/watchbill/54"]);

    let run = answer(&folder.path, &["54", "tester.q1", "1", "tester.q2", "mm"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    let now = tip(&folder.path, "54");
    let at_origin = git_text(&folder.path, &["--git-dir", bare.to_str().unwrap(), "rev-parse", "refs/watchbill/54"]);
    assert_eq!(at_origin.trim(), now);
    assert!(err_of(&run).contains("pushed it to origin"), "{}", err_of(&run));
}

#[test]
fn test_WB_ANSWER_22_the_questions_are_taken_from_the_ref_and_the_typed_answers_are_asked_one_by_one() {
    let folder = no_tracker();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);

    let run = answer(&folder.path, &["54"], "1\nmillimetres\n\n");

    assert_eq!(code(&run), Some(0), "{}{}", out_of(&run), err_of(&run));
    assert!(out_of(&run).contains("2 open questions on issue 54."), "{}", out_of(&run));
    let message = message_of(&folder.path, &tip(&folder.path, "54"));
    assert!(message.starts_with("[answers] 54.tester q1>tester q2>tester\n"), "{}", message);
}

#[test]
fn test_WB_ANSWER_22_a_key_the_list_holds_without_answer_is_refused_before_anything_is_written() {
    let folder = set_up_with(TRACKER, "accept,start");
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);
    let before = tip(&folder.path, "54");

    let run = answer(&folder.path, &["54", "tester.q1", "1"], "");

    assert_eq!(code(&run), Some(1), "{}", err_of(&run));
    assert!(err_of(&run).contains("without the role answer"), "{}", err_of(&run));
    assert_eq!(tip(&folder.path, "54"), before);
    assert!(!folder.path.join(".tracker/posted-54-0").exists());
}

#[test]
fn test_WB_ANSWER_22_with_no_signing_key_wb_answer_is_refused_naming_the_setting() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);
    git(&folder.path, &["config", "--unset", "user.signingkey"]);
    let before = tip(&folder.path, "54");

    let run = answer(&folder.path, &["54", "tester.q1", "1"], "");

    assert_eq!(code(&run), Some(1), "{}", err_of(&run));
    assert!(err_of(&run).contains("user.signingkey"), "{}", err_of(&run));
    assert_eq!(tip(&folder.path, "54"), before);
}

#[test]
fn test_WB_ANSWER_22_a_ref_that_is_not_as_wb_writes_it_is_refused_and_nothing_is_written() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);
    unsigned_stamps_on_ref(&folder.path, "54", &["[stamp] build round 1"]);
    let before = tip(&folder.path, "54");

    let run = answer(&folder.path, &["54", "tester.q1", "1"], "");

    assert_eq!(code(&run), Some(1), "{}", err_of(&run));
    assert!(err_of(&run).contains("is not as wb writes it"), "{}", err_of(&run));
    assert_eq!(tip(&folder.path, "54"), before);
}

#[test]
fn test_WB_ANSWER_22_show_reads_the_questions_of_the_ref_and_needs_no_tracker_and_no_key_for_answers() {
    let folder = set_up_with("", "accept,start");
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);
    let before = tip(&folder.path, "54");

    let run = answer(&folder.path, &["54", "--show"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(out_of(&run).contains("54.tester.q1 (1 of 2): Which brake?"), "{}", out_of(&run));
    assert_eq!(tip(&folder.path, "54"), before);
}

#[test]
fn test_WB_ANSWER_23_the_copy_is_posted_after_the_commit_is_written() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);

    let run = answer(&folder.path, &["54", "tester.q1", "1", "tester.q2", "words"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    let posted = read(&folder.path.join(".tracker/posted-54-0"));
    // The copy names the commit it copies, in a last line (#240).
    assert_eq!(posted, format!("[answers] 54.tester q1>tester q2>tester\nq1: 1\nq2: words\ncommit: {}\n", tip(&folder.path, "54")));
    // The commit was on the ref when the copy was posted.
    assert_eq!(read(&folder.path.join(".tracker/ref-54-0")).trim(), tip(&folder.path, "54"));
    assert!(err_of(&run).contains("will post on issue 54, by `issue_comment` of watchbill.yml"), "{}", err_of(&run));
}

#[test]
fn test_WB_ANSWER_23_where_no_tracker_is_set_up_no_copy_is_posted_and_the_commit_is_written() {
    let folder = no_tracker();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);

    let run = answer(&folder.path, &["54", "tester.q1", "1", "tester.q2", "words"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(err_of(&run).contains("no `issue_comment`") && err_of(&run).contains("no copy"), "{}", err_of(&run));
}

#[test]
fn test_WB_ANSWER_23_a_copy_that_cannot_be_posted_ends_with_1_and_the_commit_stays() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);
    let before = tip(&folder.path, "54");
    put(&folder.path, ".tracker/post.sh", "cat > /dev/null\nexit 1\n");

    let run = answer(&folder.path, &["54", "tester.q1", "1", "tester.q2", "words"], "");

    assert_eq!(code(&run), Some(1), "{}", err_of(&run));
    assert_ne!(tip(&folder.path, "54"), before);
    assert!(err_of(&run).contains("is on refs/watchbill/54") && err_of(&run).contains("copy may not be posted"), "{}", err_of(&run));
}

// ---- WB-ANSWER-27 and 28: an answer counts only with the signature that the list gives the role ----

/// A repository whose list holds Ada with `accept,start,answer` and Bob with the roles given, and the key git signs
/// with, which is Ada's.
fn with_bob(roles: &str) -> Folder {
    let folder = repository(None);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    git(&folder.path, &["add", "--", ".watchbill"]);
    git(&folder.path, &["commit", "-q", "-m", "Set up"]);
    git(&folder.path, &["branch", "-M", "main"]);
    let list = format!("{}bob@example.org roles=\"{}\" {}\n", ada_line_with(&folder.path, ROLES), roles, test_public(&folder.path, "bob"));
    put(&folder.path, ".watchbill/signers", &list);
    git(&folder.path, &["add", "--", ".watchbill/signers"]);
    git(&folder.path, &["commit", "-q", "-m", "The keys"]);
    signing(&folder.path);
    folder
}

/// Signs the next commits of the test with the key of `name`.
fn sign_with(folder: &Path, name: &str) {
    let key = test_key(folder, name);
    git(folder, &["config", "user.signingkey", key.to_str().unwrap()]);
}

#[test]
fn test_WB_ANSWER_27_an_answer_commit_that_is_unsigned_answers_nothing_and_status_says_so() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);
    unsigned_stamps_on_ref(&folder.path, "54", &[ANSWERED]);

    let run = status(&folder.path, &["54"]);

    let out = out_of(&run);
    assert!(out.contains("is not as wb writes it") && out.contains("is an answer commit, and unsigned"), "{}", out);
    assert!(!out.contains("waits for"), "{}", out);
    assert!(out.starts_with("Not read:\n  54  "), "{}", out);
    assert!(out.contains("[answers] 54.tester q1>tester q2>tester  (on refs/watchbill/54, not counted: "), "{}", out);
}

#[test]
fn test_WB_ANSWER_27_an_answer_commit_signed_by_a_key_without_answer_answers_nothing_and_status_says_so() {
    let folder = with_bob("start");
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);
    sign_with(&folder.path, "bob");
    stamps_on_ref(&folder.path, "54", &[ANSWERED]);

    let run = status(&folder.path, &["54"]);

    let out = out_of(&run);
    assert!(out.contains("is an answer commit, and signed by the key of bob@example.org") && out.contains("without the role answer"), "{}", out);
    assert!(!out.contains("waits for"), "{}", out);
}

#[test]
fn test_WB_ANSWER_27_a_questions_commit_signed_by_a_key_without_start_breaks_the_ref_as_a_stamp_does() {
    let folder = with_bob("answer");
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1"]);
    sign_with(&folder.path, "bob");
    stamps_on_ref(&folder.path, "54", &[ASKED]);

    let out = out_of(&status(&folder.path, &["54"]));

    assert!(out.contains("is a questions commit, and signed by the key of bob@example.org") && out.contains("without the role start"), "{}", out);
    assert!(!out.contains("questions open: "), "{}", out);
}

#[test]
fn test_WB_ANSWER_28_with_no_list_an_answer_commit_answers_nothing_and_status_says_once_that_nothing_is_signed() {
    let folder = set_up_with(TRACKER, "");
    unsigned_stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED, ANSWERED]);

    let run = status(&folder.path, &["54"]);

    let out = out_of(&run);
    assert_eq!(out.matches("nothing is signed, so nothing counts").count(), 1, "{}", out);
    assert!(out.contains("[answers] 54.tester q1>tester q2>tester  (on refs/watchbill/54, not counted: nothing is signed)"), "{}", out);
    assert!(!out.contains("waits for"), "{}", out);
}

#[test]
fn test_WB_ANSWER_28_with_no_list_wb_answer_is_refused_with_1_before_it_writes_or_posts_anything() {
    let folder = set_up_with(TRACKER, "");
    unsigned_stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);
    let before = tip(&folder.path, "54");

    let run = answer(&folder.path, &["54", "tester.q1", "1"], "");

    assert_eq!(code(&run), Some(1), "{}", err_of(&run));
    assert!(err_of(&run).contains(".watchbill/signers") && err_of(&run).contains("nothing is signed"), "{}", err_of(&run));
    assert_eq!(tip(&folder.path, "54"), before);
    assert!(!folder.path.join(".tracker/posted-54-0").exists());
}

// ---- WB-ANSWER-21: what a questions commit and an answer commit are, for the walk of the ref ----

#[test]
fn test_WB_ANSWER_21_a_questions_commit_with_a_tree_other_than_the_one_before_it_breaks_the_ref() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1"]);
    // A signed commit of questions that also adds a file to the tree.
    let parent = tip(&folder.path, "54");
    let tree = git_text(&folder.path, &["rev-parse", &format!("{}^{{tree}}", parent)]).trim().to_string();
    let blob = {
        let mut child = Command::new("git").arg("-C").arg(&folder.path).args(["hash-object", "-w", "--stdin"]).env_clear().envs(env()).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
        child.stdin.take().unwrap().write_all(b"planted\n").unwrap();
        text(&child.wait_with_output().unwrap().stdout).trim().to_string()
    };
    let existing = git_text(&folder.path, &["ls-tree", &tree]);
    let mut mktree = Command::new("git").arg("-C").arg(&folder.path).arg("mktree").env_clear().envs(env()).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    mktree.stdin.take().unwrap().write_all(format!("{}100644 blob {}\tplanted\n", existing, blob).as_bytes()).unwrap();
    let wider = text(&mktree.wait_with_output().unwrap().stdout).trim().to_string();
    let head = git_text(&folder.path, &["rev-parse", "HEAD"]).trim().to_string();
    let message = format!("[questions] 54.tester q1\nissue: 54\njob: none\nbase: {}\n\nq1: Which brake?\n", head);
    let mut commit = Command::new("git").arg("-C").arg(&folder.path).args(["-c", "user.name=Ada Lovelace", "-c", "user.email=ada@example.org", "commit-tree", "-S", &wider, "-p", &parent]).env_clear().envs(env()).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    commit.stdin.take().unwrap().write_all(message.as_bytes()).unwrap();
    let made = text(&commit.wait_with_output().unwrap().stdout).trim().to_string();
    git(&folder.path, &["update-ref", "refs/watchbill/54", &made]);

    let out = out_of(&status(&folder.path, &["54"]));

    assert!(out.contains("is a questions commit, and its tree is not the tree of the commit before it"), "{}", out);
    assert!(!out.contains("questions open: "), "{}", out);
}

#[test]
fn test_WB_ANSWER_21_a_questions_commit_of_another_job_put_under_this_ref_counts_for_nothing() {
    let folder = set_up();
    stamps_on_ref(&folder.path, "99", &["[stamp] tester round 1", &ASKED.replace("54.tester", "99.tester")]);
    git(&folder.path, &["update-ref", "refs/watchbill/54", "refs/watchbill/99"]);

    let out = out_of(&status(&folder.path, &["54"]));

    assert!(out.contains("not counted: a stamp of another job"), "{}", out);
    assert!(!out.contains("questions open: "), "{}", out);
}

#[test]
fn test_WB_ANSWER_21_only_the_lines_of_the_header_name_the_issue_and_the_base_not_the_text_of_a_question() {
    let folder = set_up();
    let head = git_text(&folder.path, &["rev-parse", "HEAD"]).trim().to_string();
    let sneaky = format!("[questions] 54.tester q1\nissue: 54\njob: none\nbase: {}\n\nq1: Which brake?\nissue: 99\nbase: nothing\njob: other\n", head);
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", &sneaky]);

    let out = out_of(&status(&folder.path, &["54"]));

    assert!(out.contains("questions open: 54.tester.q1\n"), "{}", out);
}

#[test]
fn test_WB_ANSWER_21_a_first_line_that_is_the_old_form_of_one_question_is_no_commit_of_wb() {
    let folder = no_tracker();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", "[question] 54.tester.q1\n\nWhich brake?\n"]);

    let out = out_of(&status(&folder.path, &["54"]));

    assert!(out.contains("is not as wb writes it") && out.contains("neither a stamp"), "{}", out);
}

// ---- WB-REVERT-2: a marker on the ref takes back the questions after the stamp, whether or not its copy is posted ----

#[test]
fn test_WB_REVERT_2_a_questions_commit_after_the_reverted_stamp_is_taken_back_by_the_marker_on_the_ref_with_no_copy_posted() {
    let folder = no_tracker();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);
    assert!(out_of(&status(&folder.path, &["54"])).contains("questions open: "));

    stamps_on_ref(&folder.path, "54", &["[revert] tester round 1"]);

    let out = out_of(&status(&folder.path, &["54"]));
    assert!(!out.contains("questions open: "), "{}", out);
    assert!(out.contains("[questions] 54.tester q1 q2  (on refs/watchbill/54, taken back by a [revert] marker)"), "{}", out);
}

#[test]
fn test_WB_REVERT_2_an_answer_commit_after_the_reverted_stamp_is_taken_back_and_wb_answer_shows_nothing_of_it() {
    let folder = no_tracker();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED, "[stamp] build round 1", ANSWERED, "[revert] build round 1"]);

    let out = out_of(&status(&folder.path, &["54"]));

    assert!(out.contains("questions open: 54.tester.q1, 54.tester.q2\n"), "{}", out);
    assert!(!out.contains("waits for"), "{}", out);
    let shown = answer(&folder.path, &["54", "--show"], "");
    assert!(out_of(&shown).contains("54.tester.q1 (1 of 2)"), "{}", out_of(&shown));
}

#[test]
fn test_WB_REVERT_2_wb_revert_writes_a_marker_that_takes_back_the_questions_of_the_stamp() {
    let folder = no_tracker();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED]);

    // No `issue_comment` is set up, so no copy of the marker is posted: the marker on the ref alone decides.
    let run = wb(&folder.path, &["revert", "54", "tester"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(err_of(&run).contains("no copy of the marker is posted"), "{}", err_of(&run));
    let out = out_of(&status(&folder.path, &["54"]));
    assert!(!out.contains("questions open: "), "{}", out);
}

// ---- WB-ANSWER-21 / 26: the questions a session is brought are those of the ref ----

#[test]
fn test_WB_ANSWER_26_wb_accept_reads_the_open_questions_from_the_ref_and_not_from_the_tracker() {
    let folder = set_up();
    git(&folder.path, &["checkout", "-q", "-b", "issue-54-work"]);
    put(&folder.path, ".watchbill/issues/54.yml", &BARE_ISSUE.replace("{issue}", "54"));
    git(&folder.path, &["add", "--", ".watchbill/issues/54.yml"]);
    git(&folder.path, &["commit", "-q", "-m", "The issue's file (54)"]);
    let run = wb(&folder.path, &["plan", "accept", "54"], "");
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    put(&folder.path, "src/work.c", "int work;\n");
    git(&folder.path, &["add", "--", "src/work.c"]);
    git(&folder.path, &["commit", "-q", "-m", "The work"]);
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", "[stamp] build round 1", "[questions] 54.build q1\n\nq1: Which unit?\n"]);
    // The tracker says every question is answered, which is not read.
    put(&folder.path, ".tracker/54.comments", "[answers] 54.build q1>build\n");

    let run = wb(&folder.path, &["accept", "54"], "");

    assert_eq!(code(&run), Some(1), "{}", err_of(&run));
    assert!(err_of(&run).contains("54.build.q1") && err_of(&run).contains("question"), "{}", err_of(&run));
}

// ---- WB-ANSWER-47 to 52 (#333): a job whose ref holds a stamp is read from the ref alone ----

/// A tracker that notes each pattern run in `.tracker/ran`, so that a test can see that none was run.
const WATCHED: &str = "issue_comments: \"sh .tracker/read.sh comments {issue}\"
issue_comment_bodies: \"sh .tracker/read.sh bodies {issue}\"
issue_comment: \"sh .tracker/post.sh {issue}\"
";

const READ: &str = "echo \"$1 $2\" >> .tracker/ran\ncat \".tracker/$2.$1\" 2>/dev/null\nexit 0\n";

/// The usual, with a tracker whose reads are noted.
fn watched() -> Folder {
    let folder = set_up_with(WATCHED, ROLES);
    put(&folder.path, ".tracker/read.sh", READ);
    folder
}

/// The patterns run so far on the tracker: none where the file is not there.
fn ran(folder: &Path) -> String {
    fs::read_to_string(folder.join(".tracker/ran")).unwrap_or_default()
}

/// What a forged comment would open and close, at the tracker.
const FORGED: &str = "[questions] 54.build q1 q2\n[questions] 54 q1\n[answers] 54.tester q1>tester\n";

#[test]
fn test_WB_ANSWER_47_a_forged_questions_comment_opens_no_question_for_a_job_whose_ref_holds_a_stamp_and_no_questions_commit() {
    let folder = watched();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1"]);
    put(&folder.path, ".tracker/54.comments", FORGED);

    let run = status(&folder.path, &["54"]);

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(!out_of(&run).contains("questions open: "), "{}", out_of(&run));
    assert!(!out_of(&run).contains("54.build.q1"), "{}", out_of(&run));
}

#[test]
fn test_WB_ANSWER_47_a_stamp_that_counts_for_nothing_shuts_the_tracker_out_all_the_same() {
    let folder = watched();
    unsigned_stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1"]);
    put(&folder.path, ".tracker/54.comments", FORGED);

    let run = status(&folder.path, &["54"]);

    assert!(!out_of(&run).contains("questions open: "), "{}", out_of(&run));
    assert_eq!(ran(&folder.path), "");
}

#[test]
fn test_WB_ANSWER_47_status_runs_no_issue_comments_for_a_job_whose_ref_holds_a_stamp() {
    let folder = watched();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1"]);
    put(&folder.path, ".tracker/54.comments", FORGED);

    let run = status(&folder.path, &["54"]);

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert_eq!(ran(&folder.path), "");
}

#[test]
fn test_WB_ANSWER_47_a_forged_comment_is_not_shown_or_answered_by_wb_answer_for_a_job_whose_ref_holds_a_stamp() {
    let folder = watched();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1"]);
    put(&folder.path, ".tracker/54.bodies", &format!("{}\0", ASKED));
    let before = tip(&folder.path, "54");

    let run = answer(&folder.path, &["54", "--show"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert_eq!(out_of(&run), "");
    assert!(err_of(&run).contains("no open question"), "{}", err_of(&run));
    let run = answer(&folder.path, &["54", "tester.q1", "1"], "");
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(err_of(&run).contains("no open question"), "{}", err_of(&run));
    assert_eq!(tip(&folder.path, "54"), before);
    assert_eq!(ran(&folder.path), "");
    assert!(!folder.path.join(".tracker/posted-54-0").exists());
}

#[test]
fn test_WB_ANSWER_48_wb_answer_of_a_job_whose_ref_holds_a_stamp_needs_no_tracker_pattern() {
    let folder = no_tracker();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1"]);

    let run = answer(&folder.path, &["54"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(err_of(&run).contains("no open question") && !err_of(&run).contains("no `issue_comment_bodies`"), "{}", err_of(&run));
}

#[test]
fn test_WB_ANSWER_48_wb_answer_of_a_job_with_no_stamp_and_no_job_ref_still_needs_issue_comment_bodies() {
    let folder = no_tracker();

    let run = answer(&folder.path, &["54"], "");

    assert_eq!(code(&run), Some(1), "{}", err_of(&run));
    assert!(err_of(&run).contains("no `issue_comment_bodies`"), "{}", err_of(&run));
}

/// A branch `issue-54-work` with the issue's file accepted and one commit of work, as `wb accept` needs.
fn accepted_work(folder: &Path) {
    git(folder, &["checkout", "-q", "-b", "issue-54-work"]);
    put(folder, ".watchbill/issues/54.yml", &BARE_ISSUE.replace("{issue}", "54"));
    git(folder, &["add", "--", ".watchbill/issues/54.yml"]);
    git(folder, &["commit", "-q", "-m", "The issue's file (54)"]);
    let run = wb(folder, &["plan", "accept", "54"], "");
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    put(folder, "src/work.c", "int work;\n");
    git(folder, &["add", "--", "src/work.c"]);
    git(folder, &["commit", "-q", "-m", "The work"]);
}

#[test]
fn test_WB_ANSWER_50_wb_accept_of_a_job_whose_ref_holds_a_stamp_needs_no_issue_comments_and_runs_none() {
    for tracker in [true, false] {
        let folder = if tracker { watched() } else { no_tracker() };
        accepted_work(&folder.path);
        stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", "[stamp] build round 1"]);
        // A forged comment would open a question, and a missing pattern would refuse.
        put(&folder.path, ".tracker/54.comments", FORGED);

        let run = wb(&folder.path, &["accept", "54"], "");

        assert_eq!(code(&run), Some(0), "tracker {}: {}", tracker, err_of(&run));
        assert_eq!(ran(&folder.path), "");
    }
}

#[test]
fn test_WB_ANSWER_50_wb_accept_of_a_job_with_no_stamp_on_its_ref_and_no_issue_comments_is_still_refused() {
    let folder = no_tracker();
    accepted_work(&folder.path);

    let run = wb(&folder.path, &["accept", "54"], "");

    assert_eq!(code(&run), Some(1), "{}", err_of(&run));
    assert!(err_of(&run).contains("issue_comments"), "{}", err_of(&run));
}

#[test]
fn test_WB_ANSWER_51_status_gives_a_job_whose_ref_holds_a_stamp_its_stage_with_no_tracker_pattern_and_says_nothing_of_one() {
    let folder = no_tracker();
    git(&folder.path, &["checkout", "-q", "-b", "issue-54-work"]);
    put(&folder.path, ".watchbill/issues/54.yml", &BARE_ISSUE.replace("{issue}", "54"));
    git(&folder.path, &["add", "--", ".watchbill/issues/54.yml"]);
    git(&folder.path, &["commit", "-q", "-m", "The issue's file (54)"]);
    git(&folder.path, &["update-ref", "refs/remotes/origin/issue-54-work", "HEAD"]);
    stamps_on_ref(&folder.path, "54", &["[stamp] planner round 1"]);

    let run = status(&folder.path, &[]);

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(out_of(&run).contains("Jobs waiting for you:\n"), "{}", out_of(&run));
    assert!(!err_of(&run).contains("issue_comments"), "{}", err_of(&run));
}

#[test]
fn test_WB_ANSWER_51_status_says_once_that_a_job_with_no_stamp_on_its_ref_is_read_with_no_tracker() {
    let folder = no_tracker();
    git(&folder.path, &["checkout", "-q", "-b", "issue-54-work"]);
    git(&folder.path, &["commit", "-q", "--allow-empty", "-m", "Work"]);
    git(&folder.path, &["update-ref", "refs/remotes/origin/issue-54-work", "HEAD"]);

    let run = status(&folder.path, &[]);

    assert_eq!(err_of(&run).matches("has no `issue_comments`").count(), 1, "{}", err_of(&run));
}

#[test]
fn test_WB_ANSWER_52_an_issue_begun_before_has_its_questions_and_answers_read_from_the_tracker_until_its_first_stamp_on_the_ref() {
    let folder = watched();
    put(&folder.path, ".tracker/54.comments", "[stamp] tester round 1\n[questions] 54.tester q1 q2\n[answers] 54.tester q1>tester\n");

    let run = status(&folder.path, &["54"]);

    assert!(out_of(&run).contains("questions open: 54.tester.q2\n"), "{}", out_of(&run));
    assert_eq!(ran(&folder.path), "comments 54\n");
    // The first stamp on the ref ends it: from then on the comments are not read.
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1"]);
    let run = status(&folder.path, &["54"]);
    assert!(!out_of(&run).contains("questions open: "), "{}", out_of(&run));
    assert_eq!(ran(&folder.path), "comments 54\n");
}

// ---- #333, the review of #334: a commit of `wb plan` counts as the ref too ----

#[test]
fn test_WB_ANSWER_47_a_forged_comment_opens_no_question_for_a_job_whose_ref_holds_a_plan_commit() {
    let folder = watched();
    // The organisation's file is the test's own: `wb plan add` needs one, and the machine's is never read.
    let (_place, organisation) = cleared(&folder.path);
    let args: Vec<std::ffi::OsString> = ["add", "54", "-m", "Brakes: the front one."].iter().map(std::ffi::OsString::from).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let added = watchbill::plan(&args, &folder.path, &organisation, &mut std::io::Cursor::new(Vec::new()), &mut out, &mut err);
    assert_eq!(added, 0, "{}{}", text(&out), text(&err));
    put(&folder.path, ".tracker/54.comments", FORGED);
    put(&folder.path, ".tracker/54.bodies", &format!("{}\0", ASKED));

    let run = status(&folder.path, &["54"]);

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(!out_of(&run).contains("questions open: "), "{}", out_of(&run));
    let run = answer(&folder.path, &["54", "--show"], "");
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert_eq!(out_of(&run), "");
    assert_eq!(ran(&folder.path), "");
}

// ---- WB-PUSH-13: a notice names the commit it copies ----

#[test]
fn test_WB_PUSH_13_the_copy_of_each_answer_commit_names_its_own_commit() {
    let folder = set_up();
    let build = "[questions] 54.build q1\n\nq1: Which unit?\n";
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1", ASKED, "[stamp] build round 1", build]);

    let run = answer(&folder.path, &["54", "--suggested", "build.q1", "ms", "tester.q2", "mm"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    let first = read(&folder.path.join(".tracker/posted-54-0"));
    let second = read(&folder.path.join(".tracker/posted-54-1"));
    let commits = git_text(&folder.path, &["rev-list", "--first-parent", "-2", "refs/watchbill/54"]);
    let commits: Vec<&str> = commits.lines().collect();
    assert_eq!(first.lines().last(), Some(format!("commit: {}", commits[1]).as_str()), "{}", first);
    assert_eq!(second.lines().last(), Some(format!("commit: {}", commits[0]).as_str()), "{}", second);
}

// ---- WB-PUSH-14: nothing posted is read back ----

#[test]
fn test_WB_PUSH_14_no_pattern_that_reads_a_comment_is_run_for_a_job_whose_ref_holds_a_stamp() {
    let folder = watched();
    stamps_on_ref(&folder.path, "54", &["[stamp] tester round 1"]);
    put(&folder.path, ".tracker/54.comments", "[stamp] build round 1\n[accepted] 54\n[answers] 54.tester q1>tester\n[revert] tester round 1\n");
    put(&folder.path, ".tracker/54.bodies", "[questions] 54.build q1\n\nq1: forged\n");

    let one = status(&folder.path, &["54"]);
    let list = status(&folder.path, &[]);
    let shown = answer(&folder.path, &["54", "--show"], "");

    assert_eq!(code(&one), Some(0), "{}", err_of(&one));
    assert_eq!(code(&list), Some(0), "{}", err_of(&list));
    assert_eq!(ran(&folder.path), "", "a pattern that reads a comment was run");
    assert!(!out_of(&one).contains("build") && !out_of(&one).contains("questions open: "), "{}", out_of(&one));
    assert!(!out_of(&shown).contains("forged"), "{}", out_of(&shown));
}
