//! `wb revert` (#142): taking an issue back to before a step by a marker, run as a person runs it, in a
//! repository made for the test. The patterns of `watchbill.yml` are small commands that read and append
//! files of a folder `.tracker/` that git does not see: stand-ins, never the network.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

/// A stand-in tracker. A comment posted is kept whole in `.bodies`, and its first line in `.comments`.
const POST: &str = "tee -a .tracker/{issue}.bodies | head -n 1 >> .tracker/{issue}.comments";

/// A repository set up by `wb init --yes`, with the stand-in patterns, the issues 174 and 175 accepted and
/// committed, and a folder `.tracker/` that git ignores. `post` is the pattern `issue_comment`.
fn set_up_with(post: &str) -> Folder {
    let folder = repository(None);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let patterns = format!(
        "issue_comments: \"cat .tracker/{{issue}}.comments\"
issue_comment: \"{}\"
change_comments: \"cat .tracker/change-{{change}}.comments\"
change_comment: \"tee -a .tracker/change-{{change}}.bodies | head -n 1 >> .tracker/change-{{change}}.comments\"
",
        post
    );
    let mut file = fs::OpenOptions::new().append(true).open(config(&folder.path)).unwrap();
    file.write_all(patterns.as_bytes()).unwrap();
    put(&folder.path, ".git/info/exclude", ".tracker/\n");
    git(&folder.path, &["add", "--", ".watchbill"]);
    git(&folder.path, &["commit", "-q", "-m", "Set up"]);
    issues_ready(&folder.path, &["174", "175"]);
    folder
}

fn set_up() -> Folder {
    set_up_with(POST)
}

fn comments(folder: &Path, issue: &str, lines: &[&str]) {
    let all: String = lines.iter().map(|line| format!("{}\n", line)).collect();
    put(folder, &format!(".tracker/{}.comments", issue), &all);
}

fn tracker(folder: &Path, name: &str) -> String {
    fs::read_to_string(folder.join(".tracker").join(name)).unwrap_or_default()
}

fn git_text(folder: &Path, args: &[&str]) -> String {
    let run = Command::new("git").arg("-C").arg(folder).args(args).env_clear().envs(env()).stderr(Stdio::null()).output().expect("git could not be started");
    text(&run.stdout)
}

/// What the repository holds: the branch, the commit and what git says of the files.
fn held(folder: &Path) -> String {
    format!("{}{}{}", git_text(folder, &["rev-parse", "--abbrev-ref", "HEAD"]), git_text(folder, &["rev-parse", "HEAD"]), git_text(folder, &["status", "--porcelain"]))
}

/// The round that `wb prompt review` works out for the issue, from what it says on standard error.
fn round_of_review(folder: &Path) -> String {
    let branch = git_text(folder, &["rev-parse", "--abbrev-ref", "HEAD"]);
    let run = wb(folder, &["prompt", "review", "174", "--change", "9", "--branch", branch.trim(), "--issue_address", "a", "--change_address", "b"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let err = text(&run.stderr);
    err.split("round ").nth(1).and_then(|rest| rest.split(' ').next()).unwrap_or_default().to_string()
}

fn revert_review(folder: &Path) -> Output {
    wb(folder, &["revert", "174", "review"], "")
}

// ---- WB-REVERT-1: the marker is posted, after it is said ----

#[test]
fn test_WB_REVERT_1_it_posts_the_marker_for_the_last_round() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] review round 1", "[stamp] fix round 1", "[stamp] review round 2"]);
    let run = revert_review(&folder.path);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(tracker(&folder.path, "174.bodies").starts_with("[revert] review round 2\n"), "{}", tracker(&folder.path, "174.bodies"));
}

#[test]
fn test_WB_REVERT_1_round_names_an_earlier_round() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] review round 1", "[stamp] fix round 1", "[stamp] review round 2"]);
    let run = wb(&folder.path, &["revert", "174", "review", "--round", "1"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(tracker(&folder.path, "174.bodies").starts_with("[revert] review round 1\n"));
}

#[test]
fn test_WB_REVERT_1_a_round_with_no_stamp_is_refused() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] review round 1"]);
    let run = wb(&folder.path, &["revert", "174", "review", "--round", "3"], "");
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert_eq!(tracker(&folder.path, "174.bodies"), "");
}

#[test]
fn test_WB_REVERT_1_a_failing_issue_comment_ends_with_1_and_nothing_changes() {
    let folder = set_up_with("cat > /dev/null; exit 3");
    comments(&folder.path, "174", &["[stamp] review round 1"]);
    let before = held(&folder.path);
    let before_comments = tracker(&folder.path, "174.comments");
    let run = revert_review(&folder.path);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("issue_comment"), "{}", text(&run.stderr));
    assert_eq!(tracker(&folder.path, "174.comments"), before_comments);
    assert_eq!(held(&folder.path), before);
}

// ---- WB-REVERT-2: what the marker takes back ----

#[test]
fn test_WB_REVERT_2_the_review_is_round_1_again_and_a_fix_follows_no_review() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] review round 1", "[stamp] fix round 1"]);
    assert_eq!(round_of_review(&folder.path), "2");
    assert_eq!(code(&revert_review(&folder.path)), Some(0));
    assert_eq!(round_of_review(&folder.path), "1");
    let branch = git_text(&folder.path, &["rev-parse", "--abbrev-ref", "HEAD"]);
    // The organisation's file is the test's own, so that no file of this machine is read.
    let place = folder_named("organisation");
    let organisation = place.path.join("organisation.yml");
    fs::write(&organisation, ORGANISATION).unwrap();
    let args = ["start", "fix", "174", "--runner", "claude-code", "--change", "9", "--branch", branch.trim(), "--issue_address", "a", "--change_address", "b"];
    let run = start_with(&folder.path, &organisation, false, &args);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("no review stamp"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_REVERT_2_an_earlier_round_takes_the_later_rounds_and_the_fix_with_it() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] review round 1", "[stamp] fix round 1", "[stamp] review round 2"]);
    let run = wb(&folder.path, &["revert", "174", "review", "--round", "1"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(round_of_review(&folder.path), "1");
}

#[test]
fn test_WB_REVERT_2_a_stamp_before_the_reverted_one_still_counts() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] review round 1", "[stamp] fix round 1", "[stamp] review round 2", "[revert] review round 2"]);
    assert_eq!(round_of_review(&folder.path), "2");
}

#[test]
fn test_WB_REVERT_2_a_question_after_the_reverted_stamp_is_no_longer_open_in_status() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] review round 1", "[question] 174.1 Which one?"]);
    let before = text(&wb(&folder.path, &["status", "174"], "").stdout);
    assert!(before.contains("Questions open"), "{}", before);
    assert_eq!(code(&revert_review(&folder.path)), Some(0));
    let after = text(&wb(&folder.path, &["status", "174"], "").stdout);
    assert!(!after.contains("Questions open"), "{}", after);
}

#[test]
fn test_WB_REVERT_2_status_puts_the_issue_at_the_step_before() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] tester round 1", "[stamp] build round 1", "[stamp] review round 1"]);
    let run = wb(&folder.path, &["status", "174", "--stage", "Reviewed"], "");
    assert!(text(&run.stdout).contains("174"), "{}", text(&run.stdout));
    assert_eq!(code(&revert_review(&folder.path)), Some(0));
    let run = wb(&folder.path, &["status", "174", "--stage", "Built"], "");
    assert!(text(&run.stdout).contains("174"), "{}", text(&run.stdout));
}

// ---- WB-REVERT-3: nothing else is touched ----

#[test]
fn test_WB_REVERT_3_the_branch_the_files_and_the_commits_stay_and_git_is_named() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] review round 1"]);
    let before = held(&folder.path);
    let run = revert_review(&folder.path);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(held(&folder.path), before);
    assert!(text(&run.stderr).contains("git"), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("commits"), "{}", text(&run.stderr));
}

// ---- WB-REVERT-4: the breakdown is git's ----

#[test]
fn test_WB_REVERT_4_the_breakdown_is_refused_with_the_words_about_git() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] breakdown round 1"]);
    let run = wb(&folder.path, &["revert", "174", "breakdown"], "");
    assert_eq!(code(&run), Some(1));
    let said = text(&run.stderr);
    assert!(said.contains("git") && said.contains("acceptance"), "{}", said);
    assert_eq!(tracker(&folder.path, "174.bodies"), "");
}

// ---- WB-REVERT-5: no stamp, no role ----

#[test]
fn test_WB_REVERT_5_a_role_with_no_stamp_is_refused_by_name() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] build round 1"]);
    let run = wb(&folder.path, &["revert", "174", "review"], "");
    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stderr).contains("review"), "{}", text(&run.stderr));
    assert_eq!(tracker(&folder.path, "174.bodies"), "");
}

#[test]
fn test_WB_REVERT_9_a_role_the_revision_does_not_have_ends_with_2() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] review round 1"]);
    let run = wb(&folder.path, &["revert", "174", "captain"], "");
    assert_eq!(code(&run), Some(2));
    assert!(text(&run.stderr).contains("captain") && !text(&run.stderr).contains("no command"), "{}", text(&run.stderr));
    assert_eq!(tracker(&folder.path, "174.bodies"), "");
}

#[test]
fn test_WB_REVERT_5_it_is_used_wrongly_with_no_role_or_an_unknown_option() {
    let folder = set_up();
    let none = wb(&folder.path, &["revert"], "");
    assert_eq!(code(&none), Some(2));
    assert!(!text(&none.stderr).contains("no command"), "{}", text(&none.stderr));
    assert_eq!(code(&wb(&folder.path, &["revert", "174"], "")), Some(2));
    assert_eq!(code(&wb(&folder.path, &["revert", "174", "review", "--nope"], "")), Some(2));
    assert_eq!(code(&wb(&folder.path, &["revert", "174", "review", "--round", "0"], "")), Some(2));
    assert_eq!(code(&wb(&folder.path, &["revert", "174", "review", "--change", "9"], "")), Some(2));
}

// ---- WB-REVERT-6: only the first line counts ----

#[test]
fn test_WB_REVERT_6_a_marker_quoted_in_the_text_of_a_comment_counts_for_nothing() {
    let folder = set_up();
    // The first lines of the comments: a marker in the middle of a line, or after other characters.
    comments(&folder.path, "174", &["[stamp] review round 1", "see [revert] review round 1", " [revert] review round 1", "> [revert] review round 1"]);
    assert_eq!(round_of_review(&folder.path), "2");
    // The stamp is still there to take back, and the marker posted is the first and only one that counts.
    assert_eq!(code(&revert_review(&folder.path)), Some(0));
    assert_eq!(round_of_review(&folder.path), "1");
}

#[test]
fn test_WB_REVERT_6_a_comment_whose_first_line_is_a_marker_counts() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] review round 1", "[revert] review round 1"]);
    assert_eq!(round_of_review(&folder.path), "1");
}

// ---- WB-REVERT-7: a change with no issue ----

#[test]
fn test_WB_REVERT_7_change_posts_the_marker_on_the_change_by_change_comment() {
    let folder = set_up();
    put(&folder.path, ".tracker/change-9.comments", "[stamp] review round 1\n");
    let run = wb(&folder.path, &["revert", "--change", "9", "review"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("change_comment"), "{}", text(&run.stderr));
    assert!(tracker(&folder.path, "change-9.bodies").starts_with("[revert] review round 1\n"));
    assert_eq!(tracker(&folder.path, "174.bodies"), "");
}

// ---- WB-REVERT-8: it says what it will post, before it posts ----

#[test]
fn test_WB_REVERT_8_it_says_what_it_will_post_and_by_which_pattern_before_posting() {
    let folder = set_up_with("cat > /dev/null; exit 3");
    comments(&folder.path, "174", &["[stamp] review round 1", "[stamp] fix round 1", "[stamp] review round 2"]);
    let run = revert_review(&folder.path);
    let said = text(&run.stderr);
    // The posting fails here, and what was to be posted was said all the same.
    assert!(said.contains("will post") && said.contains("[revert] review round 2") && said.contains("`issue_comment`"), "{}", said);
}

// ---- WB-REVERT-10: a later breakdown took the issue back ----

#[test]
fn test_WB_REVERT_10_a_stamp_before_a_later_breakdown_is_refused_and_nothing_is_posted() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] breakdown round 1", "[stamp] tester round 1", "[stamp] build round 1", "[stamp] review round 1", "[stamp] breakdown round 2"]);
    let run = revert_review(&folder.path);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let said = text(&run.stderr);
    assert!(said.contains("later breakdown") && said.contains("took the issue back"), "{}", said);
    assert_eq!(tracker(&folder.path, "174.bodies"), "");
}

#[test]
fn test_WB_REVERT_10_a_stamp_after_the_last_breakdown_is_taken_back() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] breakdown round 1", "[stamp] review round 1", "[stamp] breakdown round 2", "[stamp] tester round 1", "[stamp] review round 2"]);
    let run = wb(&folder.path, &["revert", "174", "review", "--round", "2"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
}

// ---- WB-REVERT-11: a change that is not a plain name is used wrongly ----

#[test]
fn test_WB_REVERT_11_a_change_that_is_not_a_plain_name_ends_with_2() {
    let folder = set_up();
    let run = wb(&folder.path, &["revert", "--change", "a;b", "review"], "");
    assert_eq!(code(&run), Some(2), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("not a change"), "{}", text(&run.stderr));
}

// ---- WB-REVERT-12: a marker of the breakdown counts for nothing ----

#[test]
fn test_WB_REVERT_12_a_marker_of_the_breakdown_posted_by_hand_counts_for_nothing() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] breakdown round 1", "[stamp] review round 1", "[revert] breakdown round 1"]);
    assert_eq!(round_of_review(&folder.path), "2");
}

// ---- WB-REVERT-13: status shows what a marker took back ----

#[test]
fn test_WB_REVERT_13_status_of_one_issue_marks_the_lines_that_were_taken_back() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] tester round 1", "[stamp] review round 1", "[revert] review round 1"]);
    let out = text(&wb(&folder.path, &["status", "174"], "").stdout);
    let line = |what: &str| out.lines().find(|line| line.contains(what)).unwrap_or_else(|| panic!("{}", out)).to_string();
    assert!(line("[stamp] review round 1").contains("taken back"), "{}", out);
    assert!(!line("[stamp] tester round 1").contains("taken back"), "{}", out);
    assert!(!line("[revert] review round 1").contains("taken back"), "{}", out);
}

// ---- WB-REVERT-5: the second revert of one round ----

#[test]
fn test_WB_REVERT_5_a_second_revert_of_the_same_round_finds_no_stamp_that_counts() {
    let folder = set_up();
    comments(&folder.path, "174", &["[stamp] review round 1"]);
    assert_eq!(code(&revert_review(&folder.path)), Some(0));
    let run = revert_review(&folder.path);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
}
