//! A job is done (#282): `wb accept <name>` accepts the work and leaves a record beside the acceptance of the issue's
//! file; a merged job, or one whose work is accepted, is done: `wb status` does not list it, `wb status <name>` says
//! so, `wb revert` ends with 1, and `wb clean` removes the views of it that hold only what is in the default branch.
//!
//! Run as a person runs `wb`, in a repository made for the test. The patterns of `watchbill.yml` are small commands
//! that read files of a folder `.tracker/`: stand-ins, never the network.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const PATTERNS: &str = "issue_comments: \"cat .tracker/{issue}.comments\"
issue_comment: \"tee -a .tracker/{issue}.bodies | head -n 1 >> .tracker/{issue}.comments\"
";

const RECORD: &str = ".watchbill/issues/54.finished";

fn set_up() -> Folder {
    let folder = repository(None);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let mut file = fs::OpenOptions::new().append(true).open(config(&folder.path)).unwrap();
    file.write_all(PATTERNS.as_bytes()).unwrap();
    put(&folder.path, "src/brake.c", "int brake(void) { return 1; }\n");
    put(&folder.path, ".git/info/exclude", ".tracker/\n");
    git(&folder.path, &["add", "--", ".watchbill", "src/brake.c"]);
    git(&folder.path, &["commit", "-q", "-m", "Set up"]);
    git(&folder.path, &["branch", "-M", "main"]);
    // An acceptance and a stamp count only against `.watchbill/signers` of `main` (#323).
    signing(&folder.path);
    folder
}

fn git_text(folder: &Path, args: &[&str]) -> String {
    let run = Command::new("git").arg("-C").arg(folder).args(args).env_clear().envs(env()).stderr(Stdio::null()).output().expect("git could not be started");
    text(&run.stdout)
}

fn comments(folder: &Path, issue: &str, lines: &[&str]) {
    let all: String = lines.iter().map(|line| format!("{}\n", line)).collect();
    put(folder, &format!(".tracker/{}.comments", issue), &all);
    // Since #236 the stamps and the markers are commits on the job's ref, and `wb` reads no `[stamp]` comment.
    stamps_from_lines(folder, issue, lines, &[]);
}

fn default_branch(_folder: &Path) -> String {
    "main".to_string()
}

fn head(folder: &Path) -> String {
    git_text(folder, &["rev-parse", "HEAD"]).trim().to_string()
}

/// Job 54: on its branch, the issue's file accepted and committed, and a commit of work. The checkout stays there.
fn job(folder: &Path, stamps: &[&str]) {
    git(folder, &["checkout", "-q", "-b", "issue-54-work"]);
    put(folder, ".watchbill/issues/54.yml", &BARE_ISSUE.replace("{issue}", "54"));
    git(folder, &["add", "--", ".watchbill/issues/54.yml"]);
    git(folder, &["commit", "-q", "-m", "The issue's file (54)"]);
    let run = wb(folder, &["plan", "accept", "54"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    put(folder, "src/work.c", "int work;\n");
    git(folder, &["add", "--", "src/work.c"]);
    git(folder, &["commit", "-q", "-m", "The work"]);
    comments(folder, "54", stamps);
}

const BUILT: [&str; 2] = ["[stamp] tester round 1", "[stamp] build round 1"];

fn done_job() -> Folder {
    let folder = set_up();
    job(&folder.path, &BUILT);
    let run = wb(&folder.path, &["accept", "54"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    folder
}

fn out_of(run: &Output) -> String {
    text(&run.stdout)
}

fn err_of(run: &Output) -> String {
    text(&run.stderr)
}

/// The paths and refs that a command that only reads must leave as they were.
fn snapshot(folder: &Path) -> (String, String, String) {
    (git_text(folder, &["for-each-ref"]), git_text(folder, &["status", "--porcelain", "--ignored"]), git_text(folder, &["log", "--all", "--format=%H"]))
}

// ---- wb accept accepts the work ----

#[test]
fn test_WB_TASK_43_accept_says_first_that_it_accepts_the_work_and_which_roles_did_not_run_then_commits_the_record_alone() {
    let folder = set_up();
    job(&folder.path, &BUILT);
    let before = head(&folder.path);
    let run = wb(&folder.path, &["accept", "54"], "");
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    let out = out_of(&run);
    assert_eq!(out.lines().next().unwrap(), format!("wb accept: accepting the work {} (not run: review, fix)", &before[..12]), "{}", out);
    assert!(out.find("accepting the work").unwrap() < out.find("will write").unwrap(), "{}", out);
    assert!(folder.path.join(RECORD).is_file());
    assert_eq!(git_text(&folder.path, &["show", "--name-only", "--format=%P", "HEAD"]).trim(), format!("{}\n\n{}", before, RECORD));
    assert!(git_text(&folder.path, &["status", "--porcelain"]).is_empty());
}

#[test]
fn test_WB_TASK_43_with_every_role_run_the_words_name_no_role_left_out() {
    let folder = set_up();
    job(&folder.path, &["[stamp] tester round 1", "[stamp] build round 1", "[stamp] review round 1", "[stamp] fix round 1"]);
    let before = head(&folder.path);
    let run = wb(&folder.path, &["accept", "54"], "");
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert_eq!(out_of(&run).lines().next().unwrap(), format!("wb accept: accepting the work {}", &before[..12]));
    assert!(!read(&folder.path.join(RECORD)).contains("not run"));
}

#[test]
fn test_WB_TASK_44_the_record_binds_the_head_commit_and_says_who_when_and_what_did_not_run() {
    let folder = set_up();
    job(&folder.path, &BUILT);
    let before = head(&folder.path);
    assert_eq!(code(&wb(&folder.path, &["accept", "54"], "")), Some(0));
    let record = read(&folder.path.join(RECORD));
    let lines: Vec<&str> = record.lines().collect();
    assert_eq!(lines.len(), 4, "{}", record);
    assert_eq!(lines[0], format!("commit: {}", before));
    assert_eq!(lines[1], "by: Ada Lovelace <ada@example.org>");
    assert!(lines[2].starts_with("time: 20") && lines[2].ends_with('Z'), "{}", record);
    assert_eq!(lines[3], "not run: review, fix");
}

#[test]
fn test_WB_TASK_45_before_any_role_committed_it_ends_with_1_and_writes_nothing() {
    let folder = set_up();
    job(&folder.path, &[]);
    let before = snapshot(&folder.path);
    let run = wb(&folder.path, &["accept", "54"], "");
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains("limit:") && err_of(&run).contains("no role"), "{}", err_of(&run));
    assert!(!folder.path.join(RECORD).exists());
    assert_eq!(snapshot(&folder.path), before);
    // A review alone is not a commit of work.
    comments(&folder.path, "54", &["[stamp] review round 1"]);
    assert_eq!(code(&wb(&folder.path, &["accept", "54"], "")), Some(1));
}

#[test]
fn test_WB_TASK_46_with_an_open_question_it_ends_with_1_naming_it() {
    let folder = set_up();
    job(&folder.path, &[BUILT[0], BUILT[1]]);
    // The job's ref holds stamps (#333), so the question is a commit on it.
    stamps_on_ref(&folder.path, "54", &["[questions] 54.build q1\n\nq1: Which unit?\n"]);
    let run = wb(&folder.path, &["accept", "54"], "");
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains("q1"), "{}", err_of(&run));
    assert!(!folder.path.join(RECORD).exists());
}

/// A view of the repository kept for a session with a path left uncommitted, in the state folder given.
fn keep_view(repo: &Path, state: &Path, session: &str, issue: &str) -> PathBuf {
    let root = fs::canonicalize(repo).unwrap();
    let below = state.join("watchbill").join("views");
    let made = watchbill::view::make(root.to_str().unwrap(), &["src/brake.c".to_string()], &below, session).unwrap();
    put(&made.folder, "left.txt", "left\n");
    let folder = state.join("watchbill").join("sessions").join(session);
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("stamp"), format!("[stamp] build round 1 {}\n", issue)).unwrap();
    fs::write(folder.join("outcome"), "outcome: ended\nstatus: 0\nseconds: 5\n").unwrap();
    fs::write(folder.join("folder"), format!("folder: {}\nbase: {}\nhead: {}\nbranch: {}\nissue: {}\n", made.folder.display(), made.base, made.head, made.branch, issue)).unwrap();
    made.folder
}

fn wb_state(folder: &Path, state: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wb")).args(args).current_dir(folder).env_clear().envs(env()).env("XDG_STATE_HOME", state).stdin(Stdio::null()).output().unwrap()
}

#[test]
fn test_WB_TASK_47_with_a_view_kept_that_holds_work_it_ends_with_1_naming_the_view() {
    let folder = set_up();
    let state = folder_named("state");
    let view = keep_view(&folder.path, &state.path, "2026-11-22T100000Z-build-54", "54");
    job(&folder.path, &BUILT);
    let run = wb_state(&folder.path, &state.path, &["accept", "54"]);
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains(&view.display().to_string()), "{}", err_of(&run));
    assert!(!folder.path.join(RECORD).exists());
}

#[test]
fn test_WB_TASK_48_a_change_that_touches_classed_code_ends_with_1_naming_the_verifier_required() {
    let folder = set_up();
    let file = fs::read_to_string(config(&folder.path)).unwrap().replace("classed: []", "classed: [src]");
    assert!(file.contains("classed: [src]"));
    fs::write(config(&folder.path), file).unwrap();
    commit_config(&folder.path);
    job(&folder.path, &BUILT);
    let before = snapshot(&folder.path);
    let run = wb(&folder.path, &["accept", "54"], "");
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    let err = err_of(&run);
    assert!(err.contains("limit:") && err.contains("src/work.c") && err.contains("a person as verifier"), "{}", err);
    assert_eq!(snapshot(&folder.path), before);
}

#[test]
fn test_WB_TASK_49_off_the_branch_of_the_issue_or_with_no_issue_comments_it_ends_with_1_and_writes_nothing() {
    let folder = set_up();
    job(&folder.path, &BUILT);
    let main = default_branch(&folder.path);
    git(&folder.path, &["checkout", "-q", "-b", "elsewhere"]);
    let before = snapshot(&folder.path);
    let run = wb(&folder.path, &["accept", "54"], "");
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains("not on the branch of the issue"), "{}", err_of(&run));
    assert_eq!(snapshot(&folder.path), before);
    git(&folder.path, &["checkout", "-q", &main]);

    let bare = repository(None);
    git(&bare.path, &["config", "user.name", "Ada Lovelace"]);
    git(&bare.path, &["config", "user.email", "ada@example.org"]);
    git(&bare.path, &["config", "commit.gpgsign", "false"]);
    assert_eq!(code(&wb(&bare.path, &["init", "--yes"], "")), Some(0));
    put(&bare.path, "src/brake.c", "int brake(void) { return 1; }\n");
    git(&bare.path, &["add", "--", ".watchbill", "src/brake.c"]);
    git(&bare.path, &["commit", "-q", "-m", "Set up"]);
    git(&bare.path, &["branch", "-M", "main"]);
    // An acceptance is refused without `.watchbill/signers` on `main` (#323).
    signing(&bare.path);
    // A job whose ref holds a stamp needs no `issue_comments` (#333, WB-ANSWER-50): this one has none, so its questions
    // are the tracker's, which cannot be read.
    job(&bare.path, &[]);
    let run = wb(&bare.path, &["accept", "54"], "");
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains("issue_comments"), "{}", err_of(&run));
    assert!(!bare.path.join(RECORD).exists());
}

#[test]
fn test_WB_TASK_50_a_record_that_is_there_is_never_written_over() {
    let folder = done_job();
    let record = read(&folder.path.join(RECORD));
    let commits = git_text(&folder.path, &["rev-list", "--count", "HEAD"]);
    let run = wb(&folder.path, &["accept", "54"], "");
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains("54.finished"), "{}", err_of(&run));
    assert_eq!(read(&folder.path.join(RECORD)), record);
    assert_eq!(git_text(&folder.path, &["rev-list", "--count", "HEAD"]), commits);
}

#[test]
fn test_WB_TASK_51_no_commit_writes_the_record_commits_nothing_and_says_the_command() {
    let folder = set_up();
    job(&folder.path, &BUILT);
    let before = head(&folder.path);
    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(folder.path.join(RECORD).is_file());
    assert_eq!(head(&folder.path), before);
    assert!(out_of(&run).contains("git add .watchbill/issues/54.finished"), "{}", out_of(&run));
    // Not committed, so not an acceptance yet: the job is still in the list.
    assert!(out_of(&wb(&folder.path, &["status"], "")).contains("54"));
}

#[test]
fn test_WB_TASK_52_a_commit_after_the_record_voids_it() {
    let folder = done_job();
    assert_eq!(out_of(&wb(&folder.path, &["status"], "")), "");
    put(&folder.path, "src/more.c", "int more;\n");
    git(&folder.path, &["add", "--", "src/more.c"]);
    git(&folder.path, &["commit", "-q", "-m", "More"]);
    let list = out_of(&wb(&folder.path, &["status"], ""));
    assert!(list.starts_with("Built:\n  54  issue-54-work") && list.lines().count() == 2, "{}", list);
    let one = out_of(&wb(&folder.path, &["status", "54"], ""));
    assert!(one.starts_with("Built:\n"), "{}", one);
    assert!(!one.contains("accepted: the work"), "{}", one);
    // It can be accepted again: the record is void, but it is there, and is never written over.
    assert_eq!(code(&wb(&folder.path, &["accept", "54"], "")), Some(1));
}

#[test]
fn test_WB_TASK_53_wb_accept_of_a_file_not_accepted_ends_with_2_naming_wb_plan_accept_which_accepts_a_file_with_no_plan() {
    let folder = set_up();
    git(&folder.path, &["checkout", "-q", "-b", "issue-54-work"]);
    put(&folder.path, ".watchbill/issues/54.yml", &BARE_ISSUE.replace("{issue}", "54"));
    git(&folder.path, &["add", "--", ".watchbill/issues/54.yml"]);
    git(&folder.path, &["commit", "-q", "-m", "The issue's file (54)"]);
    let run = wb(&folder.path, &["accept", "54"], "");
    assert_eq!(code(&run), Some(2), "{}", out_of(&run));
    assert!(err_of(&run).contains("wb plan accept 54"), "{}", err_of(&run));
    assert!(!folder.path.join(".watchbill/issues/54.accepted").exists());
    let run = wb(&folder.path, &["plan", "accept", "54"], "");
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(out_of(&run).contains("will write .watchbill/issues/54.accepted"), "{}", out_of(&run));
    assert!(folder.path.join(".watchbill/issues/54.accepted").is_file());
    assert!(!folder.path.join(RECORD).exists());
}

#[test]
fn test_WB_TASK_54_revert_of_a_done_job_ends_with_1_and_posts_nothing() {
    let folder = done_job();
    let run = wb(&folder.path, &["revert", "54", "build"], "");
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains("limit:") && err_of(&run).contains("done"), "{}", err_of(&run));
    assert!(!folder.path.join(".tracker/54.bodies").exists());
}

#[test]
fn test_WB_TASK_54_revert_of_a_merged_job_ends_with_1_too() {
    let folder = set_up();
    job(&folder.path, &BUILT);
    let main = default_branch(&folder.path);
    git(&folder.path, &["checkout", "-q", &main]);
    git(&folder.path, &["merge", "-q", "--no-edit", "issue-54-work"]);
    let run = wb(&folder.path, &["revert", "54", "build"], "");
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains("merged"), "{}", err_of(&run));
    assert!(!folder.path.join(".tracker/54.bodies").exists());
}

// ---- wb status ----

#[test]
fn test_WB_STATUS_89_a_merged_job_is_not_listed_and_wb_status_of_it_says_done_and_merged() {
    let folder = set_up();
    job(&folder.path, &BUILT);
    let main = default_branch(&folder.path);
    git(&folder.path, &["checkout", "-q", &main]);
    git(&folder.path, &["merge", "-q", "--no-edit", "issue-54-work"]);
    assert_eq!(out_of(&wb(&folder.path, &["status"], "")), "");
    let run = wb(&folder.path, &["status", "54"], "");
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    let out = out_of(&run);
    assert!(out.starts_with("Done:\n  54\n"), "{}", out);
    assert!(out.contains(&format!("merged: into {}", main)), "{}", out);
}

#[test]
fn test_WB_STATUS_123_a_job_that_is_done_waits_for_no_one() {
    let folder = done_job();
    stamps_on_ref(&folder.path, "54", &["[needs_person] 54.build\n\nThe part is not there.\n"]);

    let out = out_of(&wb(&folder.path, &["status", "54"], ""));

    assert!(out.starts_with("Done:\n  54"), "{}", out);
    assert!(!out.contains("waits for: person") && !out.contains("Needs a person"), "{}", out);
}

#[test]
fn test_WB_STATUS_90_a_squash_merged_job_is_done_too() {
    let folder = set_up();
    job(&folder.path, &BUILT);
    let main = default_branch(&folder.path);
    git(&folder.path, &["checkout", "-q", &main]);
    git(&folder.path, &["merge", "-q", "--squash", "issue-54-work"]);
    git(&folder.path, &["commit", "-q", "-m", "Squashed"]);
    assert_eq!(out_of(&wb(&folder.path, &["status"], "")), "");
    assert!(out_of(&wb(&folder.path, &["status", "54"], "")).starts_with("Done:\n  54\n"));
}

#[test]
fn test_WB_STATUS_91_a_job_whose_work_is_accepted_is_not_listed_and_wb_status_of_it_says_done() {
    let folder = done_job();
    assert_eq!(out_of(&wb(&folder.path, &["status"], "")), "");
    let out = out_of(&wb(&folder.path, &["status", "54"], ""));
    assert!(out.starts_with("Done:\n  54  issue-54-work"), "{}", out);
    assert!(!out.contains("merged:"), "{}", out);
}

#[test]
fn test_WB_STATUS_92_wb_status_of_a_job_prints_one_line_for_each_acceptance_by_its_object() {
    let folder = set_up();
    job(&folder.path, &BUILT);
    let before = head(&folder.path);
    let one = out_of(&wb(&folder.path, &["status", "54"], ""));
    let hash: String = read(&folder.path.join(".watchbill/issues/54.accepted")).lines().next().unwrap().strip_prefix("sha256: ").unwrap().chars().take(12).collect();
    assert!(one.contains(&format!("accepted: the issue's file {} (Ada Lovelace <ada@example.org>, 20", hash)), "{}", one);
    assert!(!one.contains("accepted: the work"), "{}", one);
    assert_eq!(code(&wb(&folder.path, &["accept", "54"], "")), Some(0));
    let both = out_of(&wb(&folder.path, &["status", "54"], ""));
    assert_eq!(both.matches("accepted: ").count(), 2, "{}", both);
    assert!(both.contains(&format!("accepted: the issue's file {} (", hash)), "{}", both);
    assert!(both.contains(&format!("accepted: the work {} (Ada Lovelace <ada@example.org>, 20", &before[..12])), "{}", both);
    assert!(both.contains("; not run: review, fix)"), "{}", both);
}

#[test]
fn test_WB_STATUS_93_wb_status_writes_nothing_for_a_done_job() {
    let folder = done_job();
    let state = folder_named("state");
    let before = snapshot(&folder.path);
    let _ = wb_state(&folder.path, &state.path, &["status"]);
    let _ = wb_state(&folder.path, &state.path, &["status", "54"]);
    assert_eq!(snapshot(&folder.path), before);
    assert_eq!(fs::read_dir(&state.path).unwrap().count(), 0);
}

#[test]
fn test_WB_STATUS_94_there_is_no_stage_done_and_a_done_job_is_in_no_stage_of_the_list() {
    let folder = done_job();
    let run = wb(&folder.path, &["status", "--stage", "done"], "");
    assert_eq!(code(&run), Some(2), "{}", out_of(&run));
    for stage in ["built", "accepted", "tested"] {
        assert_eq!(out_of(&wb(&folder.path, &["status", "--stage", stage], "")), "");
    }
}

#[test]
fn test_WB_STATUS_96_with_no_branch_of_the_issue_left_a_file_on_the_default_branch_is_not_a_merge() {
    let folder = set_up();
    issues_ready(&folder.path, &["54"]);
    comments(&folder.path, "54", &BUILT);
    let out = out_of(&wb(&folder.path, &["status", "54"], ""));
    assert!(out.starts_with("Built:\n  54\n"), "{}", out);
    assert!(!out.contains("merged:"), "{}", out);
}

// ---- wb clean ----

/// A repository with job 54 merged into the default branch, a state folder, and what `keep_in` of clean.rs does: a
/// view kept for a session of the job with one commit in it. When `merged`, the commit is in the default branch.
fn kept_of_a_done_job(merged: bool) -> (Folder, Folder, PathBuf) {
    let folder = set_up();
    job(&folder.path, &BUILT);
    let main = default_branch(&folder.path);
    git(&folder.path, &["checkout", "-q", &main]);
    git(&folder.path, &["merge", "-q", "--no-edit", "issue-54-work"]);
    let state = folder_named("state");
    let view = keep_view(&folder.path, &state.path, "2026-11-22T100000Z-build-54", "54");
    fs::remove_file(view.join("left.txt")).unwrap();
    put(&view, "made.c", "int made;\n");
    git(&view, &["add", "--", "made.c"]);
    git(&view, &["-c", "user.name=Ada Lovelace", "-c", "user.email=ada@example.org", "-c", "commit.gpgsign=false", "-c", "maintenance.auto=false", "commit", "-q", "-m", "Made in the view"]);
    if merged {
        git(&folder.path, &["fetch", "-q", view.to_str().unwrap(), "HEAD"]);
        git(&folder.path, &["merge", "-q", "--no-edit", "--allow-unrelated-histories", "FETCH_HEAD"]);
    }
    (folder, state, view)
}

#[test]
fn test_WB_STATUS_95_clean_removes_the_view_of_a_done_job_whose_commits_are_all_in_the_default_branch_and_says_so_first() {
    let (folder, state, view) = kept_of_a_done_job(true);
    let dry = wb_state(&folder.path, &state.path, &["clean", "--dry-run"]);
    assert_eq!(code(&dry), Some(0), "{}", err_of(&dry));
    assert!(out_of(&dry).starts_with(&format!("Would remove {}", view.display())), "{}", out_of(&dry));
    assert!(view.exists());
    let run = wb_state(&folder.path, &state.path, &["clean"]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(out_of(&run).starts_with(&format!("Removing {}", view.display())), "{}", out_of(&run));
    assert!(!view.exists());
}

#[test]
fn test_WB_STATUS_95_a_view_of_a_done_job_with_a_commit_not_in_the_default_branch_or_a_path_uncommitted_is_kept_and_named() {
    let (folder, state, view) = kept_of_a_done_job(false);
    let run = wb_state(&folder.path, &state.path, &["clean"]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(out_of(&run).starts_with(&format!("Keeping {}: ", view.display())) && out_of(&run).contains("1 commit not taken"), "{}", out_of(&run));
    assert!(view.exists());

    let (folder, state, view) = kept_of_a_done_job(true);
    put(&view, "left.txt", "left\n");
    let run = wb_state(&folder.path, &state.path, &["clean"]);
    assert!(out_of(&run).starts_with(&format!("Keeping {}: ", view.display())) && out_of(&run).contains("1 path uncommitted"), "{}", out_of(&run));
    assert!(view.exists());
}

#[test]
fn test_WB_STATUS_95_a_view_of_a_job_that_is_not_done_with_its_commit_in_the_default_branch_is_kept() {
    let folder = set_up();
    let state = folder_named("state");
    let view = keep_view(&folder.path, &state.path, "2026-11-22T100000Z-build-77", "77");
    fs::remove_file(view.join("left.txt")).unwrap();
    put(&view, "made.c", "int made;\n");
    git(&view, &["add", "--", "made.c"]);
    git(&view, &["-c", "user.name=Ada Lovelace", "-c", "user.email=ada@example.org", "-c", "commit.gpgsign=false", "-c", "maintenance.auto=false", "commit", "-q", "-m", "Made in the view"]);
    git(&folder.path, &["fetch", "-q", view.to_str().unwrap(), "HEAD"]);
    git(&folder.path, &["merge", "-q", "--no-edit", "--allow-unrelated-histories", "FETCH_HEAD"]);
    let run = wb_state(&folder.path, &state.path, &["clean"]);
    assert!(out_of(&run).starts_with(&format!("Keeping {}: ", view.display())), "{}", out_of(&run));
    assert!(view.exists());
}

#[test]
fn test_WB_STATUS_96_a_branch_made_where_the_file_and_its_acceptance_are_already_in_the_default_branch_is_not_a_merge() {
    let folder = set_up();
    issues_ready(&folder.path, &["54"]);
    git(&folder.path, &["branch", "issue-54-work"]);
    comments(&folder.path, "54", &["[stamp] tester round 1"]);
    let out = out_of(&wb(&folder.path, &["status", "54"], ""));
    assert!(!out.contains("Done") && !out.contains("merged:"), "{}", out);
    let run = wb(&folder.path, &["revert", "54", "tester"], "");
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(folder.path.join(".tracker/54.bodies").exists());
}

#[test]
fn test_WB_STATUS_91_accepted_work_that_is_merged_is_done_with_its_branches_deleted() {
    let folder = done_job();
    let main = default_branch(&folder.path);
    git(&folder.path, &["checkout", "-q", &main]);
    git(&folder.path, &["merge", "-q", "--no-edit", "issue-54-work"]);
    git(&folder.path, &["branch", "-q", "-D", "issue-54-work"]);
    assert_eq!(out_of(&wb(&folder.path, &["status"], "")), "");
    let out = out_of(&wb(&folder.path, &["status", "54"], ""));
    assert!(out.starts_with("Done:\n  54\n") && out.contains("accepted: the work "), "{}", out);
    let run = wb(&folder.path, &["revert", "54", "build"], "");
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(!folder.path.join(".tracker/54.bodies").exists());
}

#[test]
fn test_WB_TASK_50_the_refusal_says_how_to_accept_again() {
    let folder = done_job();
    let run = wb(&folder.path, &["accept", "54"], "");
    assert!(err_of(&run).contains("remove it in a commit of its own"), "{}", err_of(&run));
}

#[test]
fn test_WB_TASK_52_new_work_on_a_branch_of_the_issue_after_the_record_was_merged_puts_the_job_back_in_the_list() {
    let folder = done_job();
    let main = default_branch(&folder.path);
    git(&folder.path, &["checkout", "-q", &main]);
    git(&folder.path, &["merge", "-q", "--no-edit", "issue-54-work"]);
    git(&folder.path, &["checkout", "-q", "issue-54-work"]);
    put(&folder.path, "src/more.c", "int more;\n");
    git(&folder.path, &["add", "--", "src/more.c"]);
    git(&folder.path, &["commit", "-q", "-m", "More"]);
    git(&folder.path, &["checkout", "-q", &main]);
    let list = out_of(&wb(&folder.path, &["status"], ""));
    assert!(list.starts_with("Built:\n  54  issue-54-work"), "{}", list);
    let one = out_of(&wb(&folder.path, &["status", "54"], ""));
    assert!(one.starts_with("Built:\n"), "{}", one);
    let run = wb(&folder.path, &["revert", "54", "build"], "");
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
}

#[test]
fn test_WB_TASK_43_quiet_leaves_out_the_lines_that_confirm_and_n_is_no_commit() {
    let folder = set_up();
    job(&folder.path, &BUILT);
    let run = wb(&folder.path, &["accept", "-q", "54"], "");
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    let out = out_of(&run);
    assert!(out.starts_with("wb accept: accepting the work "), "{}", out);
    assert!(out.contains("will write") && out.contains("will commit") && out.contains("Push the branch"), "{}", out);
    assert!(!out.contains("is accepted") && !out.contains("committed "), "{}", out);

    let other = set_up();
    job(&other.path, &BUILT);
    let before = head(&other.path);
    let run = wb(&other.path, &["accept", "-n", "54"], "");
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert_eq!(head(&other.path), before);
    assert!(other.path.join(RECORD).is_file());
    assert!(out_of(&run).contains("git add .watchbill/issues/54.finished"), "{}", out_of(&run));
}
