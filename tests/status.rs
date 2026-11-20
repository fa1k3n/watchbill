//! `wb status` (#139): where each issue stands, under the last step it has done, run as a person runs it, in a
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

/// What a stand-in tracker says: the open issues, and the first lines of the comments of each.
const PATTERNS: &str = "open_issues: \"cat .tracker/open\"
issue_comments: \"cat .tracker/{issue}.comments\"
";

/// A repository set up by `wb init --yes`, with a git identity, the stand-in patterns and the folder `.tracker/`.
fn set_up() -> Folder {
    let folder = repository(None);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let mut file = fs::OpenOptions::new().append(true).open(config(&folder.path)).unwrap();
    file.write_all(PATTERNS.as_bytes()).unwrap();
    put(&folder.path, ".tracker/open", "");
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

/// The open issues of the stand-in tracker: number and title.
fn open(folder: &Path, issues: &[(&str, &str)]) {
    let lines: String = issues.iter().map(|(number, title)| format!("{}\t{}\n", number, title)).collect();
    put(folder, ".tracker/open", &lines);
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
    git(folder, &["checkout", "-q", "-b", &format!("issue-{}-work", issue)]);
    put(folder, &format!(".watchbill/issues/{}.yml", issue), &BARE_ISSUE.replace("{issue}", issue));
    let mut added = vec![format!(".watchbill/issues/{}.yml", issue)];
    if accepted {
        let run = wb(folder, &["accept", "--", issue], "");
        assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
        added.push(format!(".watchbill/issues/{}.accepted", issue));
    }
    let mut add = vec!["add", "--"];
    add.extend(added.iter().map(String::as_str));
    git(folder, &add);
    git(folder, &["commit", "-q", "-m", "The issue's file"]);
    git(folder, &["checkout", "-q", &back]);
}

/// Seven issues, one at each step, and the stamps that put them there.
fn at_every_step() -> Folder {
    let folder = set_up();
    open(
        &folder.path,
        &[
            ("1", "Plain issue"),
            ("2", "Broken down issue"),
            ("3", "Accepted issue"),
            ("4", "Tested issue"),
            ("5", "Built issue"),
            ("6", "Reviewed issue"),
            ("7", "Fixed issue"),
        ],
    );
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
    let expected = "Not broken down:
  1  Plain issue

Broken down:
  2  Broken down issue

Accepted:
  3  Accepted issue

Tested:
  4  Tested issue

Built:
  5  Built issue

Reviewed:
  6  Reviewed issue

Fixed:
  7  Fixed issue
";
    assert_eq!(out, expected);
}

#[test]
fn test_WB_STATUS_1_each_issue_is_a_line_of_its_number_and_its_title_under_the_heading_of_its_step() {
    let folder = set_up();
    open(&folder.path, &[("176", "The page shows how old the ledger is")]);
    comments(&folder.path, "176", &["[stamp] build round 2"]);
    branch(&folder.path, "176", true);
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Built:\n  176  The page shows how old the ledger is\n");
}

#[test]
fn test_WB_STATUS_11_a_heading_with_no_issue_is_not_printed() {
    let folder = at_every_step();
    let (out, _) = stdout(&folder.path, &["status", "--stage", "fixed"]);
    assert_eq!(out, "Fixed:\n  7  Fixed issue\n");
    let folder = set_up();
    open(&folder.path, &[("1", "A")]);
    comments(&folder.path, "1", &[]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Not broken down:\n  1  A\n");
    for heading in ["Broken down", "Accepted", "Tested", "Built", "Reviewed", "Fixed", "Questions open", "Not read"] {
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
    let at: Vec<usize> = ["Not broken down:", "Broken down:", "Accepted:", "Tested:", "Built:", "Reviewed:", "Fixed:"]
        .iter()
        .map(|heading| out.find(&format!("\n{}", heading)).or_else(|| out.starts_with(heading).then_some(0)).expect(heading))
        .collect();
    let mut sorted = at.clone();
    sorted.sort();
    assert_eq!(at, sorted, "{}", out);
}

#[test]
fn test_WB_STATUS_10_a_title_is_printed_with_its_control_characters_escaped() {
    let folder = set_up();
    open(&folder.path, &[("5", "Bell \u{7} and \u{1b}[31mred")]);
    comments(&folder.path, "5", &[]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Not broken down:\n  5  Bell \\u{7} and \\u{1b}[31mred\n");
}

#[test]
fn test_WB_STATUS_2_an_issue_file_that_is_not_accepted_as_it_stands_is_broken_down_only() {
    let folder = set_up();
    open(&folder.path, &[("3", "Changed after it was accepted")]);
    comments(&folder.path, "3", &[]);
    branch(&folder.path, "3", true);
    // The file is changed on the branch after its acceptance: the hash no longer holds.
    git(&folder.path, &["checkout", "-q", "issue-3-work"]);
    put(&folder.path, ".watchbill/issues/3.yml", &format!("{}# changed\n", BARE_ISSUE.replace("{issue}", "3")));
    git(&folder.path, &["commit", "-q", "-a", "-m", "Changed"]);
    git(&folder.path, &["checkout", "-q", "-"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Broken down:\n  3  Changed after it was accepted\n");
}

#[test]
fn test_WB_STATUS_2_the_last_stamp_decides_between_reviewed_and_fixed() {
    let folder = set_up();
    open(&folder.path, &[("8", "Reviewed again")]);
    comments(&folder.path, "8", &["[stamp] review round 1", "[stamp] fix round 1", "[stamp] review round 2"]);
    branch(&folder.path, "8", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Reviewed:\n  8  Reviewed again\n");
}

#[test]
fn test_WB_STATUS_18_the_files_are_read_from_the_origin_branch_when_there_is_no_local_one() {
    let folder = set_up();
    open(&folder.path, &[("9", "Only at origin")]);
    comments(&folder.path, "9", &[]);
    branch(&folder.path, "9", false);
    // Make the branch a remote branch only.
    let tip = git_text(&folder.path, &["rev-parse", "issue-9-work"]);
    git(&folder.path, &["update-ref", "refs/remotes/origin/issue-9-work", tip.trim()]);
    git(&folder.path, &["branch", "-q", "-D", "issue-9-work"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Broken down:\n  9  Only at origin\n");
}

#[test]
fn test_WB_STATUS_18_an_acceptance_that_is_written_and_not_committed_gives_broken_down() {
    let folder = set_up();
    open(&folder.path, &[("2", "Accepted, not committed")]);
    comments(&folder.path, "2", &[]);
    branch(&folder.path, "2", false);
    git(&folder.path, &["checkout", "-q", "issue-2-work"]);
    let run = wb(&folder.path, &["accept", "--", "2"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(folder.path.join(".watchbill/issues/2.accepted").exists());
    git(&folder.path, &["checkout", "-q", "-"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Broken down:\n  2  Accepted, not committed\n");
}

#[test]
fn test_WB_STATUS_9_questions_open_is_printed_after_the_steps() {
    let folder = set_up();
    open(&folder.path, &[("176", "Ledger age"), ("177", "Review count")]);
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
        "Not broken down:\n  177  Review count\n\nBuilt:\n  176  Ledger age\n\nQuestions open:\n  176  176.build.q2\n"
    );
}

#[test]
fn test_WB_STATUS_22_an_issue_with_an_open_question_stays_under_its_step() {
    let folder = set_up();
    open(&folder.path, &[("176", "Ledger age")]);
    comments(&folder.path, "176", &["[stamp] build round 1", "[question] 176.build.q1"]);
    branch(&folder.path, "176", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert!(out.starts_with("Built:\n  176  Ledger age\n"), "{}", out);
    let (out, _) = stdout(&folder.path, &["status", "--stage", "built"]);
    assert_eq!(out, "Built:\n  176  Ledger age\n");
}

#[test]
fn test_WB_STATUS_9_the_open_questions_of_an_issue_are_named_in_the_order_they_were_asked() {
    let folder = set_up();
    open(&folder.path, &[("176", "Ledger age")]);
    comments(&folder.path, "176", &["[question] 176.q1", "[question] 176.q2"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Not broken down:\n  176  Ledger age\n\nQuestions open:\n  176  176.q1, 176.q2\n");
}

#[test]
fn test_WB_STATUS_3_a_question_is_closed_by_an_answer_to_its_name_and_by_nothing_else() {
    let folder = set_up();
    open(&folder.path, &[("1", "A")]);
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
    open(&folder.path, &[("1", "A")]);
    comments(&folder.path, "1", &["[question] 1.q1", "[answer] 1.q1 only for the first command"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Not broken down:\n  1  A\n");
}

#[test]
fn test_WB_STATUS_14_a_line_that_starts_with_question_is_never_an_answer() {
    let folder = set_up();
    open(&folder.path, &[("1", "A")]);
    comments(&folder.path, "1", &["[question] 1.q1", "[question] [answer] 1.q1", "[question] 1.q1 [answer] 1.q1"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert!(out.ends_with("Questions open:\n  1  1.q1\n"), "{}", out);
}

#[test]
fn test_WB_STATUS_15_every_answer_line_counts_as_a_persons_until_wb_reads_who_wrote_a_comment() {
    // `issue_comments` prints no author: an answer from an account that is not a person's cannot be told apart.
    let folder = set_up();
    open(&folder.path, &[("1", "A")]);
    comments(&folder.path, "1", &["[question] 1.q1", "[answer] 1.q1 2"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Not broken down:\n  1  A\n");
}

#[test]
fn test_WB_STATUS_30_an_answer_closes_its_question_whatever_follows_the_name() {
    // A choice number is not held against the choices of the question: only first lines are read.
    let folder = set_up();
    open(&folder.path, &[("5", "A")]);
    comments(&folder.path, "5", &["[question] 5.q1", "[answer] 5.q1 7"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Not broken down:\n  5  A\n");
}

#[test]
fn test_WB_STATUS_29_an_answer_of_suggested_closes_the_questions_before_it_and_not_those_after() {
    let folder = set_up();
    open(&folder.path, &[("1", "A")]);
    comments(&folder.path, "1", &["[question] 1.q1", "[answer] 1 suggested", "[question] 1.q2"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert!(out.ends_with("Questions open:\n  1  1.q2\n"), "{}", out);
}

#[test]
fn test_WB_STATUS_4_without_open_issues_the_issues_are_those_of_the_branches_and_the_files_with_no_title() {
    let folder = set_up();
    // Take the pattern away.
    let file = config(&folder.path);
    let kept: String = read(&file).lines().filter(|line| !line.starts_with("open_issues:")).map(|line| format!("{}\n", line)).collect();
    fs::write(&file, kept).unwrap();
    git(&folder.path, &["commit", "-q", "-a", "-m", "No open_issues"]);
    comments(&folder.path, "11", &[]);
    comments(&folder.path, "12", &[]);
    comments(&folder.path, "13", &[]);
    branch(&folder.path, "11", true);
    branch(&folder.path, "12", false);
    // A file with no branch.
    put(&folder.path, ".watchbill/issues/13.yml", &BARE_ISSUE.replace("{issue}", "13"));
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Not broken down:\n  13\n\nBroken down:\n  12\n\nAccepted:\n  11\n");
}

#[test]
fn test_WB_STATUS_4_a_branch_of_another_issue_whose_number_starts_the_same_is_not_taken() {
    let folder = set_up();
    open(&folder.path, &[("1", "One"), ("12", "Twelve")]);
    comments(&folder.path, "1", &[]);
    comments(&folder.path, "12", &[]);
    branch(&folder.path, "12", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Not broken down:\n  1  One\n\nAccepted:\n  12  Twelve\n");
}

#[test]
fn test_WB_STATUS_5_one_issue_prints_its_heading_and_its_line_and_what_they_were_worked_out_from() {
    let folder = set_up();
    open(&folder.path, &[("176", "Ledger age"), ("177", "Other")]);
    comments(&folder.path, "176", &["[stamp] tester round 1", "[stamp] build round 1", "[question] 176.build.q1"]);
    comments(&folder.path, "177", &[]);
    branch(&folder.path, "176", true);
    let (out, status) = stdout(&folder.path, &["status", "176"]);
    assert_eq!(status, Some(0));
    assert!(out.starts_with("Built:\n  176  Ledger age\n"), "{}", out);
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
    open(&folder.path, &[("1", "A")]);
    comments(&folder.path, "1", &["[stamp] tester round 1"]);
    let (out, _) = stdout(&folder.path, &["status", "1"]);
    assert_eq!(out, "Tested:\n  1  A\n\nWorked out from:\n  no branch of the issue, from the pattern `branch`: issue-{issue}\n  [stamp] tester round 1\n");
    // The full list has no such heading.
    let (out, _) = stdout(&folder.path, &["status"]);
    assert!(!out.contains("Worked out from"), "{}", out);
}

#[test]
fn test_WB_STATUS_6_stage_prints_only_that_heading_matched_without_regard_to_case() {
    let folder = at_every_step();
    let (out, status) = stdout(&folder.path, &["status", "--stage", "not BROKEN down"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Not broken down:\n  1  Plain issue\n");
    let (out, _) = stdout(&folder.path, &["status", "--stage", "broken down"]);
    assert_eq!(out, "Broken down:\n  2  Broken down issue\n");
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
    open(&folder.path, &[("1", "A")]);
    comments(&folder.path, "1", &["[question] 1.q1"]);
    let (out, _) = stdout(&folder.path, &["status", "--stage", "questions open"]);
    assert_eq!(out, "Questions open:\n  1  1.q1\n");
}

#[test]
fn test_WB_STATUS_7_a_pattern_that_fails_for_one_issue_lists_it_under_not_read_and_the_others_are_printed() {
    let folder = set_up();
    open(&folder.path, &[("1", "Fine"), ("2", "Broken pattern")]);
    comments(&folder.path, "1", &[]);
    // No comments file for issue 2: `cat` fails.
    let run = wb(&folder.path, &["status"], "");
    assert_eq!(code(&run), Some(1));
    let out = text(&run.stdout);
    assert!(out.starts_with("Not broken down:\n  1  Fine\n"), "{}", out);
    assert!(out.contains("\nNot read:\n  2  `issue_comments` of watchbill.yml ended with 1\n"), "{}", out);
}

#[test]
fn test_WB_STATUS_23_a_failing_open_issues_ends_with_1_and_lists_nothing() {
    let folder = set_up();
    fs::remove_file(folder.path.join(".tracker/open")).unwrap();
    let run = wb(&folder.path, &["status"], "");
    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stdout).is_empty(), "{}", text(&run.stdout));
    assert!(text(&run.stderr).contains("wb status: `open_issues` of watchbill.yml ended with 1"), "{}", text(&run.stderr));
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
fn test_WB_STATUS_19_it_runs_no_pattern_but_open_issues_and_issue_comments() {
    let folder = set_up();
    let others = ["change_of_branch", "change_comment", "change_comments", "issue_comment", "issue_address", "change_address"];
    let lines: String = others.iter().map(|key| format!("{}: \"mkdir -p .marks && touch .marks/{}\"\n", key, key)).collect();
    let mut file = fs::OpenOptions::new().append(true).open(config(&folder.path)).unwrap();
    file.write_all(lines.as_bytes()).unwrap();
    open(&folder.path, &[("1", "A")]);
    comments(&folder.path, "1", &["[stamp] review round 1"]);
    branch(&folder.path, "1", true);
    for args in [vec!["status"], vec!["status", "1"], vec!["status", "--stage", "reviewed"]] {
        assert_eq!(code(&wb(&folder.path, &args, "")), Some(0));
    }
    assert!(!folder.path.join(".marks").exists(), "a pattern other than open_issues and issue_comments was run");
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
    open(&folder.path, &[("1", "A")]);
    comments(&folder.path, "1", &[]);
    fs::create_dir_all(folder.path.join("src/deep")).unwrap();
    let (out, status) = stdout(&folder.path.join("src/deep"), &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Not broken down:\n  1  A\n");
}

#[test]
fn test_WB_STATUS_27_help_names_status() {
    let folder = set_up();
    let (out, _) = stdout(&folder.path, &["--help"]);
    assert!(out.contains("wb status [<issue>] [--stage <heading>]"), "{}", out);
}

#[test]
fn test_WB_STATUS_25_a_line_of_open_issues_whose_first_field_is_not_a_plain_name_is_listed_under_not_read() {
    let folder = set_up();
    put(&folder.path, ".tracker/open", "bad name\tTitle\n8\tFine\n; touch x\tEvil\n");
    comments(&folder.path, "8", &[]);
    let run = wb(&folder.path, &["status"], "");
    assert_eq!(code(&run), Some(1));
    let out = text(&run.stdout);
    assert!(out.starts_with("Not broken down:\n  8  Fine\n"), "{}", out);
    assert!(out.contains("\nNot read:\n  bad name  "), "{}", out);
    assert!(out.contains("\n  ; touch x  "), "{}", out);
    assert!(out.contains("not a plain issue name"), "{}", out);
    assert!(!folder.path.join("x").exists());
}

#[test]
fn test_WB_STATUS_26_without_issue_comments_it_says_so_once_on_standard_error() {
    let folder = set_up();
    let file = config(&folder.path);
    let kept: String = read(&file).lines().filter(|line| !line.starts_with("issue_comments:")).map(|line| format!("{}\n", line)).collect();
    fs::write(&file, kept).unwrap();
    open(&folder.path, &[("1", "A"), ("2", "B"), ("3", "C")]);
    let run = wb(&folder.path, &["status"], "");
    assert_eq!(code(&run), Some(0));
    assert_eq!(text(&run.stdout), "Not broken down:\n  1  A\n  2  B\n  3  C\n");
    let said = text(&run.stderr);
    assert_eq!(said.lines().count(), 1, "{}", said);
    assert!(said.starts_with("wb status:") && said.contains("issue_comments"), "{}", said);
}

#[test]
fn test_WB_STATUS_31_stamps_before_the_last_breakdown_stamp_no_longer_decide_the_step() {
    let folder = set_up();
    open(&folder.path, &[("1", "Taken back, accepted"), ("2", "Taken back, not accepted"), ("3", "Built after the breakdown")]);
    let before = ["[stamp] breakdown round 1", "[stamp] tester round 1", "[stamp] build round 1", "[stamp] review round 1", "[stamp] fix round 1"];
    let with = |issue: &str, after: &[&str]| {
        let all: Vec<&str> = before.iter().copied().chain(["[stamp] breakdown round 2"]).chain(after.iter().copied()).collect();
        comments(&folder.path, issue, &all);
    };
    with("1", &[]);
    with("2", &[]);
    with("3", &["[stamp] build round 1"]);
    branch(&folder.path, "1", true);
    branch(&folder.path, "2", false);
    branch(&folder.path, "3", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Broken down:\n  2  Taken back, not accepted\n\nAccepted:\n  1  Taken back, accepted\n\nBuilt:\n  3  Built after the breakdown\n");
}

// ---- The questions folded into one comment per session, and the answers in one (#145) ----

/// The open questions `wb status --stage "questions open"` prints for issue 1, with these first lines.
fn open_of(lines: &[&str]) -> String {
    let folder = set_up();
    open(&folder.path, &[("1", "A")]);
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
    // The breakdown names its questions with the issue alone.
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
