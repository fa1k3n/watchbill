//! `wb status` (#139, #155): where each issue in the flow stands, under the last step it has done, run as a person runs it, in a
//! repository made for the test. The patterns of `watchbill.yml` are small commands that read files of a folder
//! `.tracker/` in the repository: stand-ins, never the network.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// What a stand-in tracker says: the first lines of the comments of each issue.
const PATTERNS: &str = "issue_comments: \"cat .tracker/{issue}.comments\"
";

/// A repository set up by `wb init --yes`, with a git identity, the stand-in patterns and the folder `.tracker/`.
fn set_up() -> Folder {
    set_up_with(PATTERNS)
}

/// The same, with the patterns the test gives.
fn set_up_with(patterns: &str) -> Folder {
    let folder = repository(None);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let mut file = fs::OpenOptions::new().append(true).open(config(&folder.path)).unwrap();
    file.write_all(patterns.as_bytes()).unwrap();
    git(&folder.path, &["add", "--", ".watchbill"]);
    git(&folder.path, &["commit", "-q", "-m", "Set up"]);
    folder
}

fn git_text(folder: &Path, args: &[&str]) -> String {
    let run = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(args)
        .env_clear()
        .envs(env())
        .stderr(Stdio::null())
        .output()
        .expect("git could not be started");
    text(&run.stdout)
}

/// Puts the issues in the flow: each gets a branch `issue-<issue>-work` that holds a commit the default branch
/// does not, and the same branch at `origin`, unless it has a branch already.
fn open(folder: &Path, issues: &[&str]) {
    for issue in issues {
        let name = format!("issue-{}-work", issue);
        if !git_text(folder, &["branch", "--list", &name]).trim().is_empty() {
            continue;
        }
        let back = git_text(folder, &["rev-parse", "--abbrev-ref", "HEAD"]).trim().to_string();
        git(folder, &["checkout", "-q", "-b", &name]);
        git(folder, &["commit", "-q", "--allow-empty", "-m", "Work"]);
        git(folder, &["checkout", "-q", &back]);
        at_origin(folder, &name);
    }
}

/// Makes the branch of `origin` of that name as the local one is.
fn at_origin(folder: &Path, name: &str) {
    let tip = git_text(folder, &["rev-parse", name]);
    git(folder, &["update-ref", &format!("refs/remotes/origin/{}", name), tip.trim()]);
}

/// The first lines of the comments of an issue.
fn comments(folder: &Path, issue: &str, lines: &[&str]) {
    let all: String = lines.iter().map(|line| format!("{}\n", line)).collect();
    put(folder, &format!(".tracker/{}.comments", issue), &all);
}

/// Makes the branch `issue-<issue>-work` with the issue's file, and its acceptance when `accepted`, and goes
/// back to the branch it was on.
fn branch(folder: &Path, issue: &str, accepted: bool) {
    let back = git_text(folder, &["rev-parse", "--abbrev-ref", "HEAD"]).trim().to_string();
    let name = format!("issue-{}-work", issue);
    let exists = !git_text(folder, &["branch", "--list", &name]).trim().is_empty();
    git(folder, &if exists { vec!["checkout", "-q", &name] } else { vec!["checkout", "-q", "-b", &name] });
    put(folder, &format!(".watchbill/issues/{}.yml", issue), &BARE_ISSUE.replace("{issue}", issue));
    let mut added = vec![format!(".watchbill/issues/{}.yml", issue)];
    if accepted {
        let run = wb(folder, &["accept", "--no-commit", "--", issue], "");
        assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
        added.push(format!(".watchbill/issues/{}.accepted", issue));
    }
    let mut add = vec!["add", "--"];
    add.extend(added.iter().map(String::as_str));
    git(folder, &add);
    git(folder, &["commit", "-q", "-m", "The issue's file"]);
    git(folder, &["checkout", "-q", &back]);
    at_origin(folder, &name);
}

/// Seven issues, one at each step, and the stamps that put them there.
fn at_every_step() -> Folder {
    let folder = set_up();
    open(&folder.path, &["1", "2", "3", "4", "5", "6", "7"]);
    branch(&folder.path, "2", false);
    for issue in ["3", "4", "5", "6", "7"] {
        branch(&folder.path, issue, true);
    }
    comments(&folder.path, "4", &["[stamp] tester round 1"]);
    comments(&folder.path, "5", &["[stamp] tester round 1", "[stamp] build round 1"]);
    comments(&folder.path, "6", &["[stamp] tester round 1", "[stamp] build round 1", "[stamp] review round 1"]);
    comments(&folder.path, "7", &["[stamp] build round 1", "[stamp] review round 1", "[stamp] fix round 1"]);
    for issue in ["1", "2", "3"] {
        comments(&folder.path, issue, &[]);
    }
    folder
}

fn stdout(folder: &Path, args: &[&str]) -> (String, Option<i32>) {
    let run = wb(folder, args, "");
    (text(&run.stdout), code(&run))
}

#[test]
fn test_WB_STATUS_2_each_issue_is_under_the_last_step_it_has_done() {
    let folder = at_every_step();
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    let expected = "Not planned:
  1  issue-1-work

Planned:
  2  issue-2-work

Accepted:
  3  issue-3-work

Tested:
  4  issue-4-work

Built:
  5  issue-5-work

Reviewed:
  6  issue-6-work

Fixed:
  7  issue-7-work
";
    assert_eq!(out, expected);
}

#[test]
fn test_WB_STATUS_11_a_heading_with_no_issue_is_not_printed() {
    let folder = at_every_step();
    let (out, _) = stdout(&folder.path, &["status", "--stage", "fixed"]);
    assert_eq!(out, "Fixed:\n  7  issue-7-work\n");
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", &[]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Not planned:\n  1  issue-1-work\n");
    for heading in ["Planned", "Accepted", "Tested", "Built", "Reviewed", "Fixed", "Questions open", "Not read"] {
        assert!(!out.contains(heading), "{} in {}", heading, out);
    }
}

#[test]
fn test_WB_STATUS_12_no_line_says_a_round_a_command_or_a_step_going_on() {
    let folder = at_every_step();
    let (out, _) = stdout(&folder.path, &["status"]);
    for word in ["round", "running", "going on", "next", "wb ", "run "] {
        assert!(!out.contains(word), "{} in {}", word, out);
    }
}

#[test]
fn test_WB_STATUS_13_the_steps_are_printed_in_the_order_of_the_flow() {
    let folder = at_every_step();
    let (out, _) = stdout(&folder.path, &["status"]);
    let at: Vec<usize> = ["Not planned:", "Planned:", "Accepted:", "Tested:", "Built:", "Reviewed:", "Fixed:"]
        .iter()
        .map(|heading| out.find(&format!("\n{}", heading)).or_else(|| out.starts_with(heading).then_some(0)).expect(heading))
        .collect();
    let mut sorted = at.clone();
    sorted.sort();
    assert_eq!(at, sorted, "{}", out);
}

#[test]
fn test_WB_STATUS_2_an_issue_file_that_is_not_accepted_as_it_stands_is_planned_only() {
    let folder = set_up();
    open(&folder.path, &["3"]);
    comments(&folder.path, "3", &[]);
    branch(&folder.path, "3", true);
    // The file is changed on the branch after its acceptance: the hash no longer holds.
    git(&folder.path, &["checkout", "-q", "issue-3-work"]);
    put(&folder.path, ".watchbill/issues/3.yml", &format!("{}# changed\n", BARE_ISSUE.replace("{issue}", "3")));
    git(&folder.path, &["commit", "-q", "-a", "-m", "Changed"]);
    git(&folder.path, &["checkout", "-q", "-"]);
    at_origin(&folder.path, "issue-3-work");
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Planned:\n  3  issue-3-work\n");
}

#[test]
fn test_WB_STATUS_2_the_last_stamp_decides_between_reviewed_and_fixed() {
    let folder = set_up();
    open(&folder.path, &["8"]);
    comments(&folder.path, "8", &["[stamp] review round 1", "[stamp] fix round 1", "[stamp] review round 2"]);
    branch(&folder.path, "8", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Reviewed:\n  8  issue-8-work\n");
}

#[test]
fn test_WB_STATUS_18_the_files_are_read_from_the_origin_branch_when_there_is_no_local_one() {
    let folder = set_up();
    open(&folder.path, &["9"]);
    comments(&folder.path, "9", &[]);
    branch(&folder.path, "9", false);
    // Make the branch a remote branch only.
    let tip = git_text(&folder.path, &["rev-parse", "issue-9-work"]);
    git(&folder.path, &["update-ref", "refs/remotes/origin/issue-9-work", tip.trim()]);
    git(&folder.path, &["branch", "-q", "-D", "issue-9-work"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Planned:\n  9  issue-9-work\n");
}

#[test]
fn test_WB_STATUS_18_an_acceptance_that_is_written_and_not_committed_gives_planned() {
    let folder = set_up();
    open(&folder.path, &["2"]);
    comments(&folder.path, "2", &[]);
    branch(&folder.path, "2", false);
    git(&folder.path, &["checkout", "-q", "issue-2-work"]);
    let run = wb(&folder.path, &["accept", "--no-commit", "--", "2"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(folder.path.join(".watchbill/issues/2.accepted").exists());
    git(&folder.path, &["checkout", "-q", "-"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Planned:\n  2  issue-2-work\n");
}

#[test]
fn test_WB_STATUS_9_questions_open_is_printed_after_the_steps() {
    let folder = set_up();
    open(&folder.path, &["176", "177"]);
    comments(
        &folder.path,
        "176",
        &["[stamp] build round 1", "[question] 176.build.q1", "[question] 176.build.q2", "[answer] 176.build.q1 2"],
    );
    comments(&folder.path, "177", &[]);
    branch(&folder.path, "176", true);
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(
        out,
        "Not planned:\n  177  issue-177-work\n\nBuilt:\n  176  issue-176-work\n\nQuestions open:\n  176  176.build.q2\n"
    );
}

#[test]
fn test_WB_STATUS_22_an_issue_with_an_open_question_stays_under_its_step() {
    let folder = set_up();
    open(&folder.path, &["176"]);
    comments(&folder.path, "176", &["[stamp] build round 1", "[question] 176.build.q1"]);
    branch(&folder.path, "176", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert!(out.starts_with("Built:\n  176  issue-176-work\n"), "{}", out);
    let (out, _) = stdout(&folder.path, &["status", "--stage", "built"]);
    assert_eq!(out, "Built:\n  176  issue-176-work\n");
}

#[test]
fn test_WB_STATUS_9_the_open_questions_of_an_issue_are_named_in_the_order_they_were_asked() {
    let folder = set_up();
    open(&folder.path, &["176"]);
    comments(&folder.path, "176", &["[question] 176.q1", "[question] 176.q2"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Not planned:\n  176  issue-176-work\n\nQuestions open:\n  176  176.q1, 176.q2\n");
}

#[test]
fn test_WB_STATUS_3_a_question_is_closed_by_an_answer_to_its_name_and_by_nothing_else() {
    let folder = set_up();
    open(&folder.path, &["1"]);
    // Another question's answer, another issue's answer, and lines that only mention the name.
    comments(
        &folder.path,
        "1",
        &["[question] 1.q1", "[question] 1.q2", "[answer] 1.q3 2", "[answer] 2.q1 2", "[comment] [answer] 1.q1", "[question] [answer] 1.q2"],
    );
    let (out, _) = stdout(&folder.path, &["status"]);
    assert!(out.ends_with("Questions open:\n  1  1.q1, 1.q2\n"), "{}", out);
}

#[test]
fn test_WB_STATUS_3_an_answer_in_words_closes_its_question() {
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", &["[question] 1.q1", "[answer] 1.q1 only for the first command"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Not planned:\n  1  issue-1-work\n");
}

#[test]
fn test_WB_STATUS_14_a_line_that_starts_with_question_is_never_an_answer() {
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", &["[question] 1.q1", "[question] [answer] 1.q1", "[question] 1.q1 [answer] 1.q1"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert!(out.ends_with("Questions open:\n  1  1.q1\n"), "{}", out);
}

#[test]
fn test_WB_STATUS_15_every_answer_line_counts_as_a_persons_until_wb_reads_who_wrote_a_comment() {
    // `issue_comments` prints no author: an answer from an account that is not a person's cannot be told apart.
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", &["[question] 1.q1", "[answer] 1.q1 2"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Not planned:\n  1  issue-1-work\n");
}

#[test]
fn test_WB_STATUS_30_an_answer_closes_its_question_whatever_follows_the_name() {
    // A choice number is not held against the choices of the question: only first lines are read.
    let folder = set_up();
    open(&folder.path, &["5"]);
    comments(&folder.path, "5", &["[question] 5.q1", "[answer] 5.q1 7"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Not planned:\n  5  issue-5-work\n");
}

#[test]
fn test_WB_STATUS_29_an_answer_of_suggested_closes_the_questions_before_it_and_not_those_after() {
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", &["[question] 1.q1", "[answer] 1 suggested", "[question] 1.q2"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert!(out.ends_with("Questions open:\n  1  1.q2\n"), "{}", out);
}

#[test]
fn test_WB_STATUS_4_a_branch_of_another_issue_whose_number_starts_the_same_is_not_taken() {
    let folder = set_up();
    open(&folder.path, &["1", "12"]);
    comments(&folder.path, "1", &[]);
    comments(&folder.path, "12", &[]);
    branch(&folder.path, "12", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Not planned:\n  1  issue-1-work\n\nAccepted:\n  12  issue-12-work\n");
}

#[test]
fn test_WB_STATUS_5_one_issue_prints_its_heading_and_its_line_and_what_they_were_worked_out_from() {
    let folder = set_up();
    open(&folder.path, &["176", "177"]);
    comments(&folder.path, "176", &["[stamp] tester round 1", "[stamp] build round 1", "[question] 176.build.q1"]);
    comments(&folder.path, "177", &[]);
    branch(&folder.path, "176", true);
    let (out, status) = stdout(&folder.path, &["status", "176"]);
    assert_eq!(status, Some(0));
    assert!(out.starts_with("Built:\n  176  issue-176-work\n"), "{}", out);
    assert!(!out.contains("177"), "{}", out);
    for line in [
        "[stamp] tester round 1",
        "[stamp] build round 1",
        "[question] 176.build.q1",
        "issue-176-work",
        ".watchbill/issues/176.yml",
        ".watchbill/issues/176.accepted",
    ] {
        assert!(out.contains(line), "{} not in {}", line, out);
    }
}

#[test]
fn test_WB_STATUS_24_one_issue_prints_the_heading_worked_out_from_and_its_lines_indented() {
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", &["[stamp] tester round 1"]);
    let (out, _) = stdout(&folder.path, &["status", "1"]);
    assert_eq!(out, "Tested:\n  1  issue-1-work\n\nWorked out from:\n  refs/heads/issue-1-work: no .watchbill/issues/1.yml\n  refs/remotes/origin/issue-1-work: no .watchbill/issues/1.yml\n  [stamp] tester round 1\n");
    // The full list has no such heading.
    let (out, _) = stdout(&folder.path, &["status"]);
    assert!(!out.contains("Worked out from"), "{}", out);
}

#[test]
fn test_WB_STATUS_6_stage_prints_only_that_heading_matched_without_regard_to_case() {
    let folder = at_every_step();
    let (out, status) = stdout(&folder.path, &["status", "--stage", "not PLANNED"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Not planned:\n  1  issue-1-work\n");
    let (out, _) = stdout(&folder.path, &["status", "--stage", "planned"]);
    assert_eq!(out, "Planned:\n  2  issue-2-work\n");
}

#[test]
fn test_WB_STATUS_16_a_stage_that_is_not_known_is_used_wrongly() {
    let folder = at_every_step();
    let run = wb(&folder.path, &["status", "--stage", "running"], "");
    assert_eq!(code(&run), Some(2));
    assert!(text(&run.stdout).is_empty());
    assert!(text(&run.stderr).contains("running"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_STATUS_20_a_stage_without_a_value_and_an_option_not_known_are_used_wrongly() {
    let folder = at_every_step();
    // Only `wb status` says `wb status:`; a command that is not known says `wb: no command`.
    for args in [vec!["status", "--stage"], vec!["status", "--all"], vec!["status", "../x"], vec!["status", "1", "2"]] {
        let run = wb(&folder.path, &args, "");
        assert_eq!(code(&run), Some(2), "{:?}", args);
        assert!(text(&run.stderr).starts_with("wb status:"), "{:?}: {}", args, text(&run.stderr));
    }
}

#[test]
fn test_WB_STATUS_6_stage_questions_open_prints_the_questions_alone() {
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", &["[question] 1.q1"]);
    let (out, _) = stdout(&folder.path, &["status", "--stage", "questions open"]);
    assert_eq!(out, "Questions open:\n  1  1.q1\n");
}

#[test]
fn test_WB_STATUS_7_a_pattern_that_fails_for_one_issue_lists_it_under_not_read_and_the_others_are_printed() {
    let folder = set_up();
    open(&folder.path, &["1", "2"]);
    comments(&folder.path, "1", &[]);
    // No comments file for issue 2: `cat` fails.
    let run = wb(&folder.path, &["status"], "");
    assert_eq!(code(&run), Some(1));
    let out = text(&run.stdout);
    assert!(out.starts_with("Not planned:\n  1  issue-1-work\n"), "{}", out);
    assert!(out.contains("\nNot read:\n  2  `issue_comments` of watchbill.yml ended with 1\n"), "{}", out);
}

/// Every file of the working tree but `.git`, with its size.
fn tree(folder: &Path) -> Vec<(String, u64)> {
    let mut found = Vec::new();
    let mut todo = vec![folder.to_path_buf()];
    while let Some(path) = todo.pop() {
        for entry in fs::read_dir(&path).unwrap() {
            let entry = entry.unwrap();
            let meta = entry.metadata().unwrap();
            if entry.file_name() == ".git" {
                continue;
            }
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

fn repository_state(folder: &Path) -> [String; 4] {
    [
        git_text(folder, &["rev-parse", "--abbrev-ref", "HEAD"]),
        git_text(folder, &["status", "--porcelain", "-uall"]),
        git_text(folder, &["for-each-ref"]),
        git_text(folder, &["reflog"]),
    ]
}

#[test]
fn test_WB_STATUS_8_it_writes_nothing() {
    let folder = at_every_step();
    let before = repository_state(&folder.path);
    let files = tree(&folder.path);
    for args in [vec!["status"], vec!["status", "7"], vec!["status", "--stage", "fixed"]] {
        assert_eq!(code(&wb(&folder.path, &args, "")), Some(0));
    }
    // The branches, the reflog and the index are as they were, and so is every file of the tree.
    assert_eq!(before, repository_state(&folder.path));
    assert_eq!(files, tree(&folder.path));
}

#[test]
fn test_WB_STATUS_17_it_switches_no_branch() {
    let folder = at_every_step();
    git(&folder.path, &["checkout", "-q", "issue-4-work"]);
    let before = git_text(&folder.path, &["rev-parse", "--abbrev-ref", "HEAD"]);
    assert_eq!(before.trim(), "issue-4-work");
    assert_eq!(code(&wb(&folder.path, &["status"], "")), Some(0));
    assert_eq!(git_text(&folder.path, &["rev-parse", "--abbrev-ref", "HEAD"]), before);
    assert!(git_text(&folder.path, &["reflog", "-1"]).contains("checkout: moving from"), "no checkout was made by wb");
}

#[test]
fn test_WB_STATUS_21_a_folder_that_is_not_set_up_or_not_a_repository_ends_with_2() {
    let folder = repository(None);
    let run = wb(&folder.path, &["status"], "");
    assert_eq!(code(&run), Some(2));
    assert!(text(&run.stderr).starts_with("wb status:") && text(&run.stderr).contains("watchbill.yml"), "{}", text(&run.stderr));
    let plain = folder_named("plain");
    let run = wb(&plain.path, &["status"], "");
    assert_eq!(code(&run), Some(2));
    assert!(text(&run.stderr).starts_with("wb status:"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_STATUS_28_it_runs_from_a_folder_below_the_root() {
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", &[]);
    fs::create_dir_all(folder.path.join("src/deep")).unwrap();
    let (out, status) = stdout(&folder.path.join("src/deep"), &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Not planned:\n  1  issue-1-work\n");
}

#[test]
fn test_WB_STATUS_26_without_issue_comments_it_says_so_once_on_standard_error() {
    let folder = set_up();
    let file = config(&folder.path);
    let kept: String = read(&file).lines().filter(|line| !line.starts_with("issue_comments:")).map(|line| format!("{}\n", line)).collect();
    fs::write(&file, kept).unwrap();
    commit_config(&folder.path);
    open(&folder.path, &["1", "2", "3"]);
    let run = wb(&folder.path, &["status"], "");
    assert_eq!(code(&run), Some(0));
    assert_eq!(text(&run.stdout), "Not planned:\n  1  issue-1-work\n  2  issue-2-work\n  3  issue-3-work\n");
    let said = text(&run.stderr);
    assert_eq!(said.lines().count(), 1, "{}", said);
    assert!(said.starts_with("wb status:") && said.contains("issue_comments"), "{}", said);
}

#[test]
fn test_WB_STATUS_31_stamps_before_the_last_planner_stamp_no_longer_decide_the_step() {
    let folder = set_up();
    open(&folder.path, &["1", "2", "3"]);
    let before = ["[stamp] planner round 1", "[stamp] tester round 1", "[stamp] build round 1", "[stamp] review round 1", "[stamp] fix round 1"];
    let with = |issue: &str, after: &[&str]| {
        let all: Vec<&str> = before.iter().copied().chain(["[stamp] planner round 2"]).chain(after.iter().copied()).collect();
        comments(&folder.path, issue, &all);
    };
    with("1", &[]);
    with("2", &[]);
    with("3", &["[stamp] build round 1"]);
    branch(&folder.path, "1", true);
    branch(&folder.path, "2", false);
    branch(&folder.path, "3", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Planned:\n  2  issue-2-work\n\nAccepted:\n  1  issue-1-work\n\nBuilt:\n  3  issue-3-work\n");
}

// ---- The questions folded into one comment per session, and the answers in one (#145) ----

/// The open questions `wb status --stage "questions open"` prints for issue 1, with these first lines.
fn open_of(lines: &[&str]) -> String {
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", lines);
    let (out, status) = stdout(&folder.path, &["status", "--stage", "questions open"]);
    assert_eq!(status, Some(0), "{}", out);
    out
}

#[test]
fn test_WB_ANSWER_11_a_question_of_a_questions_comment_is_open_until_an_answers_comment_names_it() {
    assert_eq!(open_of(&["[questions] 1.tester q1 q2"]), "Questions open:\n  1  1.tester.q1, 1.tester.q2\n");
    assert_eq!(open_of(&["[questions] 1.tester q1 q2", "[answers] 1.tester q1"]), "Questions open:\n  1  1.tester.q2\n");
    assert_eq!(open_of(&["[questions] 1.tester q1 q2", "[answers] 1.tester q1 q2"]), "");
    // The planner names its questions with the issue alone.
    assert_eq!(open_of(&["[questions] 1 q1 q2", "[answers] 1 q2"]), "Questions open:\n  1  1.q1\n");
}

#[test]
fn test_WB_ANSWER_11_an_answers_comment_closes_only_the_questions_of_its_own_role_and_issue() {
    let out = open_of(&["[questions] 1.tester q1", "[questions] 1.build q1", "[questions] 1 q1", "[answers] 1.build q1", "[answers] 2.tester q1"]);
    assert_eq!(out, "Questions open:\n  1  1.tester.q1, 1.q1\n");
}

#[test]
fn test_WB_ANSWER_11_words_after_the_role_that_are_no_question_name_close_nothing() {
    // The form the owner wrote in the trial of 0.0.8, which no prompt gives.
    assert_eq!(open_of(&["[questions] 1 q1", "[answers] 1 as you suggested"]), "Questions open:\n  1  1.q1\n");
    // A line that only names the form is no answer.
    assert_eq!(open_of(&["[questions] 1 q1", "[comment] [answers] 1 q1", "[questions] [answers] 1 q1"]), "Questions open:\n  1  1.q1\n");
}

// The forms of revision 0.0.8 alone are read as #139 read them: the tests of WB-STATUS-3, 9, 14, 29 and 30 keep them.

#[test]
fn test_WB_ANSWER_11_the_old_and_the_new_forms_answer_each_other() {
    let out = open_of(&["[question] 1.tester.q1", "[questions] 1.tester q2 q3", "[answers] 1.tester q1", "[answer] 1.tester.q2 1"]);
    assert_eq!(out, "Questions open:\n  1  1.tester.q3\n");
    // `[answer] <issue> suggested` closes the questions of a `[questions]` comment before it.
    assert_eq!(open_of(&["[questions] 1.tester q1", "[answer] 1 suggested"]), "");
}

#[test]
fn test_WB_ANSWER_16_a_questions_comment_from_a_reverted_stamp_up_to_its_marker_does_not_count() {
    let out = open_of(&["[questions] 1 q1", "[stamp] tester round 1", "[questions] 1.tester q1", "[revert] tester round 1"]);
    assert_eq!(out, "Questions open:\n  1  1.q1\n");
}

#[test]
fn test_WB_ANSWER_16_an_answers_comment_from_a_reverted_stamp_up_to_its_marker_does_not_count() {
    let out = open_of(&["[questions] 1 q1", "[stamp] tester round 1", "[answers] 1 q1", "[revert] tester round 1"]);
    assert_eq!(out, "Questions open:\n  1  1.q1\n");
}

// ---- Only the issues in the flow (#155) ----

/// The default branch of the repository made by `set_up`.
fn default_branch(folder: &Path) -> String {
    git_text(folder, &["rev-parse", "--abbrev-ref", "HEAD"]).trim().to_string()
}

/// Adds lines to `watchbill.yml`.
fn add_patterns(folder: &Path, lines: &str) {
    let mut file = fs::OpenOptions::new().append(true).open(config(folder)).unwrap();
    file.write_all(lines.as_bytes()).unwrap();
    commit_config(folder);
}

/// Commits a file on the branch, with the checkout back where it was.
fn commit_on(folder: &Path, name: &str, file: &str) {
    let back = default_branch(folder);
    git(folder, &["checkout", "-q", name]);
    put(folder, file, "work\n");
    git(folder, &["add", "--", file]);
    git(folder, &["commit", "-q", "-m", "More work"]);
    git(folder, &["checkout", "-q", &back]);
}

#[test]
fn test_WB_STATUS_32_an_issue_is_listed_when_its_branch_holds_a_commit_the_default_branch_does_not() {
    let folder = set_up();
    comments(&folder.path, "1", &[]);
    branch(&folder.path, "1", true);
    comments(&folder.path, "2", &[]);
    // A branch with nothing that the default branch lacks is not the flow.
    git(&folder.path, &["branch", "issue-2-work"]);
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Accepted:\n  1  issue-1-work\n");
}

#[test]
fn test_WB_STATUS_32_an_issue_that_is_merged_or_deleted_is_not_listed_with_its_file_on_the_default_branch() {
    let folder = set_up();
    for issue in ["41", "42", "43"] {
        comments(&folder.path, issue, &["[stamp] fix round 1"]);
        branch(&folder.path, issue, true);
    }
    // 41 is merged and its branch kept, 42 is merged and its branches deleted, 43 is still open.
    for issue in ["41", "42"] {
        git(&folder.path, &["merge", "-q", "--no-edit", &format!("issue-{}-work", issue)]);
    }
    for gone in ["refs/heads/issue-42-work", "refs/remotes/origin/issue-42-work"] {
        git(&folder.path, &["update-ref", "-d", gone]);
    }
    assert!(folder.path.join(".watchbill/issues/42.yml").exists());
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Fixed:\n  43  issue-43-work\n");
}

#[test]
fn test_WB_STATUS_33_the_issues_are_found_with_git_alone_and_no_pattern() {
    // Nothing but `issue_comments` is in `watchbill.yml` of the repository, and its issues are found.
    let folder = set_up();
    comments(&folder.path, "7", &[]);
    open(&folder.path, &["7"]);
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Not planned:\n  7  issue-7-work\n");
}

#[test]
fn test_WB_STATUS_34_an_issue_whose_branch_holds_no_issue_file_is_under_not_planned() {
    let folder = set_up();
    comments(&folder.path, "5", &[]);
    open(&folder.path, &["5"]);
    let (out, _) = stdout(&folder.path, &["status", "--stage", "not planned"]);
    assert_eq!(out, "Not planned:\n  5  issue-5-work\n");
}

#[test]
fn test_WB_STATUS_35_one_issue_that_is_not_in_the_flow_is_read_and_said_not_to_be() {
    let folder = set_up();
    comments(&folder.path, "41", &["[stamp] build round 1", "[stamp] review round 1", "[stamp] fix round 1"]);
    branch(&folder.path, "41", true);
    git(&folder.path, &["merge", "-q", "--no-edit", "issue-41-work"]);
    git(&folder.path, &["update-ref", "-d", "refs/heads/issue-41-work"]);
    git(&folder.path, &["update-ref", "-d", "refs/remotes/origin/issue-41-work"]);
    let (out, status) = stdout(&folder.path, &["status", "41"]);
    assert_eq!(status, Some(0));
    assert!(out.starts_with("Fixed:\n  41\n\nWorked out from:\n  not in the flow: no branch of the issue holds a commit that "), "{}", out);
    assert!(out.contains("[stamp] fix round 1"), "{}", out);
    // The list does not have it.
    assert_eq!(stdout(&folder.path, &["status"]), (String::new(), Some(0)));
}

#[test]
fn test_WB_STATUS_36_a_line_is_the_number_and_the_branch_and_no_title() {
    let folder = set_up();
    add_patterns(&folder.path, "issue_address: \"cat .tracker/{issue}.address\"\n");
    put(&folder.path, ".tracker/129.address", "Title of the issue\nThe rest\n");
    comments(&folder.path, "129", &[]);
    branch(&folder.path, "129", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Accepted:\n  129  issue-129-work\n");
}

#[test]
fn test_WB_STATUS_37_one_issue_prints_the_first_line_of_issue_address_as_its_title_escaped() {
    let folder = set_up();
    add_patterns(&folder.path, "issue_address: \"cat .tracker/{issue}.address\"\n");
    put(&folder.path, ".tracker/5.address", "Bell \u{7} and \u{1b}[31mred\nThe rest\n");
    comments(&folder.path, "5", &[]);
    open(&folder.path, &["5"]);
    let (out, status) = stdout(&folder.path, &["status", "5"]);
    assert_eq!(status, Some(0));
    assert!(out.starts_with("Not planned:\n  5  issue-5-work  Bell \\u{7} and \\u{1b}[31mred\n"), "{}", out);
    assert!(!out.contains("The rest"), "{}", out);
}

#[test]
fn test_WB_STATUS_37_a_failing_issue_address_puts_the_issue_under_not_read() {
    let folder = set_up();
    add_patterns(&folder.path, "issue_address: \"cat .tracker/{issue}.address\"\n");
    comments(&folder.path, "5", &[]);
    open(&folder.path, &["5"]);
    let run = wb(&folder.path, &["status", "5"], "");
    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stdout).contains("Not read:\n  5  `issue_address` of watchbill.yml ended with 1"), "{}", text(&run.stdout));
}

/// The calls a stand-in pattern made, as the lines it wrote, sorted.
fn calls(folder: &Path, name: &str) -> Vec<String> {
    let mut found: Vec<String> = fs::read_to_string(folder.join(".calls").join(name)).map(|text| text.lines().map(str::to_string).collect()).unwrap_or_default();
    found.sort();
    found
}

#[test]
fn test_WB_STATUS_38_it_runs_issue_comments_and_change_of_branch_once_for_each_issue_listed_and_open_issues_never() {
    let folder = set_up();
    let file = config(&folder.path);
    let kept: String = read(&file).lines().filter(|line| !line.starts_with("issue_comments:")).map(|line| format!("{}\n", line)).collect();
    fs::write(&file, kept).unwrap();
    commit_config(&folder.path);
    add_patterns(
        &folder.path,
        "issue_comments: \"mkdir -p .calls && echo {issue} >> .calls/issue_comments && cat .tracker/{issue}.comments\"
open_issues: \"mkdir -p .calls && echo x >> .calls/open_issues\"
change_of_branch: \"mkdir -p .calls && echo {branch} >> .calls/change_of_branch\"
issue_address: \"mkdir -p .calls && echo {issue} >> .calls/issue_address && echo Title\"
issue_comment: \"mkdir -p .calls && echo x >> .calls/issue_comment\"
change_comment: \"mkdir -p .calls && echo x >> .calls/change_comment\"
change_comments: \"mkdir -p .calls && echo x >> .calls/change_comments\"
change_address: \"mkdir -p .calls && echo x >> .calls/change_address\"
",
    );
    for issue in ["1", "2", "3"] {
        comments(&folder.path, issue, &[]);
    }
    open(&folder.path, &["1", "2", "3"]);
    assert_eq!(code(&wb(&folder.path, &["status"], "")), Some(0));
    assert_eq!(calls(&folder.path, "issue_comments"), ["1", "2", "3"]);
    assert_eq!(calls(&folder.path, "change_of_branch"), ["issue-1-work", "issue-2-work", "issue-3-work"]);
    for never in ["open_issues", "issue_address", "issue_comment", "change_comment", "change_comments", "change_address"] {
        assert!(calls(&folder.path, never).is_empty(), "{} was run", never);
    }
    // One issue: `issue_address` once as well.
    fs::remove_dir_all(folder.path.join(".calls")).unwrap();
    assert_eq!(code(&wb(&folder.path, &["status", "2"], "")), Some(0));
    assert_eq!(calls(&folder.path, "issue_comments"), ["2"]);
    assert_eq!(calls(&folder.path, "change_of_branch"), ["issue-2-work"]);
    assert_eq!(calls(&folder.path, "issue_address"), ["2"]);
    assert!(calls(&folder.path, "open_issues").is_empty());
}

#[test]
fn test_WB_STATUS_39_the_default_branch_is_that_of_origin_else_main_else_master_and_none_ends_with_2() {
    let folder = set_up();
    comments(&folder.path, "1", &[]);
    open(&folder.path, &["1"]);
    let own = default_branch(&folder.path);
    // `main` or `master`, whichever there is, is found with no `origin/HEAD`.
    assert_eq!(stdout(&folder.path, &["status"]), ("Not planned:\n  1  issue-1-work\n".to_string(), Some(0)));
    // `origin/HEAD` wins: here it names a branch that holds the issue's commit, so the issue is done.
    git(&folder.path, &["update-ref", "refs/remotes/origin/trunk", "issue-1-work"]);
    git(&folder.path, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/trunk"]);
    assert_eq!(stdout(&folder.path, &["status"]), (String::new(), Some(0)));
    git(&folder.path, &["symbolic-ref", "--delete", "refs/remotes/origin/HEAD"]);
    git(&folder.path, &["update-ref", "-d", "refs/remotes/origin/trunk"]);
    // No default branch: the name is neither of them.
    git(&folder.path, &["branch", "-q", "-m", &own, "trunk"]);
    let run = wb(&folder.path, &["status"], "");
    assert_eq!(code(&run), Some(2));
    assert!(text(&run.stdout).is_empty(), "{}", text(&run.stdout));
    assert!(text(&run.stderr).starts_with("wb status:") && text(&run.stderr).contains("default branch"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_STATUS_40_a_local_branch_ahead_of_origin_shows_how_many_commits_are_not_pushed() {
    let folder = set_up();
    for issue in ["1", "2", "3"] {
        comments(&folder.path, issue, &[]);
        branch(&folder.path, issue, true);
    }
    // 1 is two commits ahead of its branch at origin, 2 is level, 3 is at origin only, and 4 has no branch at origin.
    commit_on(&folder.path, "issue-1-work", "a.txt");
    commit_on(&folder.path, "issue-1-work", "b.txt");
    git(&folder.path, &["update-ref", "-d", "refs/heads/issue-3-work"]);
    comments(&folder.path, "4", &[]);
    open(&folder.path, &["4"]);
    git(&folder.path, &["update-ref", "-d", "refs/remotes/origin/issue-4-work"]);
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Not planned:\n  4  issue-4-work  1 not pushed\n\nAccepted:\n  1  issue-1-work  2 not pushed\n  2  issue-2-work\n  3  issue-3-work\n");
}

#[test]
fn test_WB_STATUS_41_it_reads_the_refs_as_they_are_and_fetches_nothing() {
    let folder = set_up();
    // `origin` is a place that is not there: a fetch would fail and say so.
    git(&folder.path, &["remote", "add", "origin", "/nonexistent/origin.git"]);
    comments(&folder.path, "1", &[]);
    open(&folder.path, &["1"]);
    let run = wb(&folder.path, &["status"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stderr).is_empty(), "{}", text(&run.stderr));
    assert!(!folder.path.join(".git/FETCH_HEAD").exists());
}

#[test]
fn test_WB_STATUS_42_the_line_shows_the_change_that_change_of_branch_prints() {
    let folder = set_up();
    add_patterns(&folder.path, "change_of_branch: \"cat .tracker/{branch}.change\"\n");
    put(&folder.path, ".tracker/issue-129-work.change", "178\n");
    put(&folder.path, ".tracker/issue-130-work.change", "");
    for issue in ["129", "130", "131"] {
        comments(&folder.path, issue, &[]);
        branch(&folder.path, issue, true);
    }
    commit_on(&folder.path, "issue-129-work", "a.txt");
    let run = wb(&folder.path, &["status"], "");
    let out = text(&run.stdout);
    // 131 has no file for its `change_of_branch`: it is not read.
    assert_eq!(code(&run), Some(1));
    assert!(out.starts_with("Accepted:\n  129  issue-129-work  #178  1 not pushed\n  130  issue-130-work\n"), "{}", out);
    assert!(out.contains("\nNot read:\n  131  `change_of_branch` of watchbill.yml ended with 1\n"), "{}", out);
    assert!(!out.contains("  131  issue-131-work"), "{}", out);
}

#[test]
fn test_WB_STATUS_42_an_open_change_does_not_decide_the_step() {
    let folder = set_up();
    add_patterns(&folder.path, "change_of_branch: \"echo 7\"\n");
    comments(&folder.path, "3", &["[stamp] build round 1"]);
    branch(&folder.path, "3", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Built:\n  3  issue-3-work  #7\n");
}

// ---- An issue that is done under squash merges (#155, the owner's decision of 2026-11-20) ----

/// Squash-merges the issue's branch into the checked-out default branch, and keeps the branch.
fn squash(folder: &Path, issue: &str) {
    git(folder, &["merge", "-q", "--squash", &format!("issue-{}-work", issue)]);
    git(folder, &["commit", "-q", "-m", &format!("Issue {} (#{})", issue, issue)]);
}

#[test]
fn test_WB_STATUS_32_a_branch_whose_whole_change_is_a_squash_commit_on_the_default_branch_is_done() {
    let folder = set_up();
    for issue in ["41", "42", "43"] {
        comments(&folder.path, issue, &[]);
        branch(&folder.path, issue, true);
    }
    // 41 is squashed and its branches kept, 42 is squashed and then has one more commit, 43 is not merged.
    squash(&folder.path, "41");
    squash(&folder.path, "42");
    commit_on(&folder.path, "issue-42-work", "later.txt");
    at_origin(&folder.path, "issue-42-work");
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Accepted:\n  42  issue-42-work\n  43  issue-43-work\n");
}

#[test]
fn test_WB_STATUS_32_a_squash_with_another_diff_than_the_branchs_is_not_found_and_the_issue_stays() {
    let folder = set_up();
    comments(&folder.path, "41", &[]);
    branch(&folder.path, "41", true);
    // The squash commit holds one more file than the branch: its change is not the branch's.
    git(&folder.path, &["merge", "-q", "--squash", "issue-41-work"]);
    put(&folder.path, "extra.txt", "edited in the web page\n");
    git(&folder.path, &["add", "--", "extra.txt"]);
    git(&folder.path, &["commit", "-q", "-m", "Issue 41 (#41)"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Accepted:\n  41  issue-41-work\n");
}

#[test]
fn test_WB_STATUS_32_a_local_branch_whose_upstream_is_gone_is_done() {
    let folder = set_up();
    for issue in ["5", "6", "7"] {
        comments(&folder.path, issue, &[]);
        branch(&folder.path, issue, true);
    }
    git(&folder.path, &["remote", "add", "origin", "/nonexistent/origin.git"]);
    // 5 and 6 follow a branch at origin; at 5 it was deleted (as after `git fetch --prune`), 6 still has it, 7 follows none.
    for issue in ["5", "6"] {
        let name = format!("issue-{}-work", issue);
        git(&folder.path, &["config", &format!("branch.{}.remote", name), "origin"]);
        git(&folder.path, &["config", &format!("branch.{}.merge", name), &format!("refs/heads/{}", name)]);
    }
    git(&folder.path, &["update-ref", "-d", "refs/remotes/origin/issue-5-work"]);
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Accepted:\n  6  issue-6-work\n  7  issue-7-work\n");
}

#[test]
fn test_WB_STATUS_32_a_refs_listing_that_fails_ends_with_2_and_lists_nothing() {
    let folder = set_up();
    comments(&folder.path, "1", &[]);
    open(&folder.path, &["1"]);
    fs::write(folder.path.join(".git/packed-refs"), "garbage\n").unwrap();
    let run = wb(&folder.path, &["status"], "");
    assert_eq!(code(&run), Some(2), "{}", text(&run.stdout));
    assert!(text(&run.stdout).is_empty(), "{}", text(&run.stdout));
    assert!(text(&run.stderr).starts_with("wb status:"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_STATUS_37_a_malformed_issue_address_does_not_fail_the_list_that_never_runs_it() {
    let folder = set_up();
    add_patterns(&folder.path, "issue_address: [one, two]\n");
    comments(&folder.path, "1", &[]);
    open(&folder.path, &["1"]);
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Not planned:\n  1  issue-1-work\n");
}

// ---- WB-STATUS-53 to 55: who the newest answer waits for (#226) ----

#[test]
fn test_WB_STATUS_53_one_issue_says_which_role_the_newest_answer_waits_for() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[stamp] build round 1", "[questions] 54.build q3 q4", "[answers] 54.build q3>tester q4>build"]);
    branch(&folder.path, "54", true);
    let (out, status) = stdout(&folder.path, &["status", "54"]);
    assert_eq!(status, Some(0));
    assert!(out.contains("\nwaits for: build (54.build.q4)\n"), "{}", out);
    assert!(!out.contains("waits for: tester"), "{}", out);
}

#[test]
fn test_WB_STATUS_53_the_newest_answer_that_names_a_role_decides() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(
        &folder.path,
        "54",
        &["[stamp] build round 1", "[questions] 54.build q3 q4", "[answers] 54.build q3>tester", "[answers] 54.build q4"],
    );
    branch(&folder.path, "54", true);
    let (out, _) = stdout(&folder.path, &["status", "54"]);
    assert!(out.contains("\nwaits for: tester (54.build.q3)\n"), "{}", out);
}

#[test]
fn test_WB_STATUS_53_an_answer_for_a_person_says_person() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[stamp] build round 1", "[questions] 54.build q3", "[answers] 54.build q3>person"]);
    branch(&folder.path, "54", true);
    let (out, _) = stdout(&folder.path, &["status", "54"]);
    assert!(out.contains("\nwaits for: person (54.build.q3)\n"), "{}", out);
}

#[test]
fn test_WB_STATUS_53_an_answer_of_the_planner_names_its_question_without_a_role_in_the_name() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[questions] 54 q1", "[answers] 54 q1>planner"]);
    let (out, _) = stdout(&folder.path, &["status", "54"]);
    assert!(out.contains("\nwaits for: planner (54.q1)\n"), "{}", out);
}

#[test]
fn test_WB_STATUS_53_the_list_prints_no_waits_for_line() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[stamp] build round 1", "[questions] 54.build q3", "[answers] 54.build q3>tester"]);
    branch(&folder.path, "54", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert!(!out.contains("waits for"), "{}", out);
}

#[test]
fn test_WB_STATUS_54_an_answer_of_0_10_0_closes_its_question_and_names_no_role() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[stamp] build round 1", "[questions] 54.build q3", "[answers] 54.build q3"]);
    branch(&folder.path, "54", true);
    let (out, status) = stdout(&folder.path, &["status", "54"]);
    assert_eq!(status, Some(0));
    assert!(!out.contains("waits for"), "{}", out);
    assert!(!out.contains("Questions open"), "{}", out);
}

#[test]
fn test_WB_STATUS_55_the_comments_are_read_once_for_each_issue_with_the_role_too() {
    let folder = set_up_with("issue_comments: \"echo run >> .tracker/count; cat .tracker/{issue}.comments\"\n");
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[stamp] build round 1", "[questions] 54.build q3", "[answers] 54.build q3>tester"]);
    branch(&folder.path, "54", true);
    let (out, status) = stdout(&folder.path, &["status", "54"]);
    assert_eq!(status, Some(0));
    assert!(out.contains("waits for: tester (54.build.q3)"), "{}", out);
    assert_eq!(fs::read_to_string(folder.path.join(".tracker/count")).unwrap().lines().count(), 1);
}

#[test]
fn test_WB_STATUS_56_a_stamp_of_the_role_newer_than_the_answer_clears_waits_for() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[stamp] build round 1", "[questions] 54.build q3", "[answers] 54.build q3>tester", "[stamp] tester round 2"]);
    branch(&folder.path, "54", true);
    let (out, _) = stdout(&folder.path, &["status", "54"]);
    assert!(!out.contains("waits for"), "{}", out);
}

#[test]
fn test_WB_STATUS_56_a_stamp_of_another_role_or_before_the_answer_does_not() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[stamp] tester round 1", "[questions] 54.build q3", "[answers] 54.build q3>tester", "[stamp] build round 2"]);
    branch(&folder.path, "54", true);
    let (out, _) = stdout(&folder.path, &["status", "54"]);
    assert!(out.contains("\nwaits for: tester (54.build.q3)\n"), "{}", out);
}

#[test]
fn test_WB_STATUS_56_person_is_never_cleared_by_a_stamp() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[questions] 54.build q3", "[answers] 54.build q3>person", "[stamp] build round 2", "[stamp] tester round 2"]);
    branch(&folder.path, "54", true);
    let (out, _) = stdout(&folder.path, &["status", "54"]);
    assert!(out.contains("\nwaits for: person (54.build.q3)\n"), "{}", out);
}

#[test]
fn test_WB_STATUS_57_a_word_that_is_no_role_closes_the_question_and_names_none() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[stamp] build round 1", "[questions] 54.build q3", "[answers] 54.build q3>optional"]);
    branch(&folder.path, "54", true);
    let (out, _) = stdout(&folder.path, &["status", "54"]);
    assert!(!out.contains("waits for"), "{}", out);
    assert!(!out.contains("Questions open"), "{}", out);
}

// ---- #300: a stage only where its work came back, and a round that brought nothing back is said ----

/// A folder of the lines of one start, as `wb start` leaves it, below `state`: its stamp, its outcome, and `back` where
/// one is given.
fn session(state: &Path, role: &str, issue: &str, round: u32, outcome: &str, back: Option<&str>) {
    let folder = state.join("watchbill/sessions").join(format!("2026-11-24T10{:02}00Z-{}-{}", round, role, issue));
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("stamp"), format!("[stamp] {} round {}\n", role, round)).unwrap();
    fs::write(folder.join("outcome"), outcome).unwrap();
    if let Some(back) = back {
        fs::write(folder.join("back"), back).unwrap();
    }
}

const ENDED: &str = "outcome: ended\nstatus: 0\nseconds: 5\n";
const TAKEN: &str = "taken: 1a2b3c4 Write the tests\nview: removed\n";
const NOTHING: &str = "view: removed\n";

/// `wb status` with the state folder `state`.
fn status_in(folder: &Path, state: &Path, args: &[&str]) -> (String, Option<i32>) {
    let run = Command::new(env!("CARGO_BIN_EXE_wb"))
        .arg("status")
        .args(args)
        .current_dir(folder)
        .env_clear()
        .envs(env().into_iter().filter(|(name, _)| name != "XDG_STATE_HOME"))
        .env("XDG_STATE_HOME", state)
        .stdin(Stdio::null())
        .output()
        .expect("wb could not be started");
    (text(&run.stdout), code(&run))
}

/// Issue 3 with a tester round and a build round stamped, and a state folder.
fn stamped_3() -> (Folder, Folder) {
    let folder = set_up();
    open(&folder.path, &["3"]);
    comments(&folder.path, "3", &["[stamp] tester round 1", "[stamp] build round 1"]);
    branch(&folder.path, "3", true);
    (folder, common::folder())
}

#[test]
fn test_WB_STATUS_79_a_build_round_that_brought_nothing_back_leaves_the_issue_at_tested() {
    let (folder, state) = stamped_3();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "build", "3", 1, ENDED, Some(NOTHING));
    let (out, status) = status_in(&folder.path, &state.path, &[]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Tested:\n  3  issue-3-work\n");
}

#[test]
fn test_WB_STATUS_79_a_tester_round_that_brought_nothing_back_leaves_the_issue_at_accepted() {
    let (folder, state) = stamped_3();
    comments(&folder.path, "3", &["[stamp] tester round 1"]);
    session(&state.path, "tester", "3", 1, ENDED, Some("failed: the branch moved\nview: kept /somewhere\n"));
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    assert_eq!(out, "Accepted:\n  3  issue-3-work\n");
}

#[test]
fn test_WB_STATUS_80_a_build_round_that_brought_a_commit_back_is_built() {
    let (folder, state) = stamped_3();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "build", "3", 1, ENDED, Some(TAKEN));
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    assert_eq!(out, "Built:\n  3  issue-3-work\n");
}

#[test]
fn test_WB_STATUS_80_a_take_back_later_that_took_a_commit_counts_for_its_round() {
    let (folder, state) = stamped_3();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "build", "3", 1, ENDED, Some("not taken: 1a2b3c4 Build it\nview: kept /somewhere\ntake-back\ntaken: 5d6e7f8 Build it\nview: removed\n"));
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    assert_eq!(out, "Built:\n  3  issue-3-work\n");
}

#[test]
fn test_WB_STATUS_81_a_round_with_no_record_on_this_machine_counts_by_its_stamp() {
    let (folder, state) = stamped_3();
    // No folder of a start at all.
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    assert_eq!(out, "Built:\n  3  issue-3-work\n");
    // A folder with no `back`.
    session(&state.path, "build", "3", 1, "outcome: ended\nstatus: 1\nseconds: 5\n", None);
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    assert_eq!(out, "Built:\n  3  issue-3-work\n");
    // A session that still runs: its process is one that is read as `wb` (a copy of `sleep` named `wb`).
    let program = state.path.join("wb");
    fs::copy("/usr/bin/sleep", &program).unwrap();
    let mut running = Command::new(&program).arg("60").spawn().expect("the stand-in for wb could not be started");
    fs::write(state.path.join("watchbill/sessions/2026-11-24T100100Z-build-3/outcome"), format!("outcome: running\npid: {}\n", running.id())).unwrap();
    fs::write(state.path.join("watchbill/sessions/2026-11-24T100100Z-build-3/back"), NOTHING).unwrap();
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    let _ = running.kill();
    let _ = running.wait();
    assert_eq!(out, "Built:\n  3  issue-3-work\n");
}

#[test]
fn test_WB_STATUS_81_a_start_of_another_issue_or_another_round_says_nothing_of_this_one() {
    let (folder, state) = stamped_3();
    session(&state.path, "build", "33", 1, ENDED, Some(NOTHING));
    session(&state.path, "build", "3", 2, ENDED, Some(NOTHING));
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    assert_eq!(out, "Built:\n  3  issue-3-work\n");
}

#[test]
fn test_WB_STATUS_82_one_issue_says_the_round_that_brought_nothing_back() {
    let (folder, state) = stamped_3();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "build", "3", 1, ENDED, Some(NOTHING));
    let (out, _) = status_in(&folder.path, &state.path, &["3"]);
    assert!(out.starts_with("Tested:\n  3  issue-3-work\n"), "{}", out);
    assert!(out.lines().any(|line| line == "build round 1: no commit came back"), "{}", out);
    assert!(!out.contains("tester round 1: no commit"), "{}", out);
    // The list says nothing of it.
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    assert!(!out.contains("no commit came back"), "{}", out);
}

#[test]
fn test_WB_STATUS_83_a_round_with_work_kept_in_a_view_names_the_view_beside_it() {
    let (folder, state) = stamped_3();
    let view = state.path.join("watchbill/views/view-1");
    fs::create_dir_all(&view).unwrap();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "build", "3", 1, ENDED, Some(&format!("uncommitted: src/a.c\nuncommitted: src/b.c\nview: kept {}\n", view.display())));
    let (out, _) = status_in(&folder.path, &state.path, &["3"]);
    let line = format!("build round 1: no commit came back; kept: 2 paths uncommitted, {}", view.display());
    assert!(out.lines().any(|one| one == line), "{}", out);
}

#[test]
fn test_WB_STATUS_84_a_review_round_is_not_held_to_a_commit() {
    let folder = set_up();
    open(&folder.path, &["3"]);
    comments(&folder.path, "3", &["[stamp] tester round 1", "[stamp] build round 1", "[stamp] review round 1"]);
    branch(&folder.path, "3", true);
    let state = common::folder();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "build", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "review", "3", 1, ENDED, Some(NOTHING));
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    assert_eq!(out, "Reviewed:\n  3  issue-3-work\n");
    let (out, _) = status_in(&folder.path, &state.path, &["3"]);
    assert!(!out.contains("no commit came back"), "{}", out);
}

#[test]
fn test_WB_STATUS_82_a_round_before_the_last_planner_stamp_is_not_said() {
    let folder = set_up();
    open(&folder.path, &["3"]);
    comments(&folder.path, "3", &["[stamp] build round 1", "[stamp] planner round 2", "[stamp] tester round 1"]);
    branch(&folder.path, "3", true);
    let state = common::folder();
    session(&state.path, "build", "3", 1, ENDED, Some(NOTHING));
    session(&state.path, "tester", "3", 1, ENDED, Some(NOTHING));
    let (out, _) = status_in(&folder.path, &state.path, &["3"]);
    assert!(!out.contains("build round 1: no commit came back"), "{}", out);
    assert!(out.contains("tester round 1: no commit came back"), "{}", out);
}

#[test]
fn test_WB_STATUS_82_a_round_that_a_revert_took_back_is_not_said() {
    let folder = set_up();
    open(&folder.path, &["3"]);
    comments(&folder.path, "3", &["[stamp] tester round 1", "[stamp] build round 1", "[revert] build round 1"]);
    branch(&folder.path, "3", true);
    let state = common::folder();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "build", "3", 1, ENDED, Some(NOTHING));
    let (out, _) = status_in(&folder.path, &state.path, &["3"]);
    assert!(!out.contains("no commit came back"), "{}", out);
}

#[test]
fn test_WB_STATUS_83_a_view_named_beside_its_round_is_not_named_again_on_a_kept_line() {
    let (folder, state) = stamped_3();
    let view = state.path.join("watchbill/views/view-1");
    fs::create_dir_all(&view).unwrap();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "build", "3", 1, ENDED, Some(&format!("uncommitted: src/a.c\nview: kept {}\n", view.display())));
    let (out, _) = status_in(&folder.path, &state.path, &["3"]);
    assert_eq!(out.matches(&view.display().to_string()).count(), 1, "{}", out);
    assert!(!out.contains("kept: build round 1"), "{}", out);
}
