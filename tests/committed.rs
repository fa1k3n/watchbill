//! No role after the breakdown until the issue's file and its acceptance are committed (#97), run as a
//! person runs it, in a repository made for the test.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::path::Path;
use std::process::Output;

const GOOD: &str = "issue: '54'
from: 3f0c1d9e
requirements:
  - id: '54.1'
    text: The tool reads the file.
    source: the goal
interface: |
  fn read(path: &str) -> String
changes: [src/issue.rs]
reads:
  tester: []
  build: []
  review: []
  fix: []
existing_tests_changed: []
questions: []
";

const ROLES: [&str; 4] = ["tester", "build", "review", "fix"];
const YML: &str = ".watchbill/issues/54.yml";
const ACCEPTED: &str = ".watchbill/issues/54.accepted";

const REST: [&str; 10] =
    ["--branch", "issue-54", "--issue_address", "tracker 54", "--change", "34", "--round", "1", "--change_address", "server 34"];

fn set_up() -> Folder {
    let folder = repository(None);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    folder
}

fn commit(folder: &Path, files: &[&str]) {
    let mut add = vec!["add", "--"];
    add.extend(files);
    git(folder, &add);
    git(folder, &["commit", "-q", "-m", "x"]);
}

/// The revision's files are committed, the file of issue 54 is written and accepted, and nothing of the issue is committed.
fn accepted_apart() -> Folder {
    let folder = set_up();
    commit(&folder.path, &[".watchbill"]);
    put(&folder.path, YML, GOOD);
    let run = wb(&folder.path, &["accept", "54"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    folder
}

fn prompt(folder: &Path, role: &str, extra: &[&str]) -> Output {
    let mut args = vec!["prompt", role, "54"];
    args.extend(REST);
    args.extend(extra);
    wb(folder, &args, "")
}

fn refused(run: &Output, named: &str) {
    assert_eq!(code(run), Some(1), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains(named), "{}", text(&run.stderr));
}

#[test]
fn test_WB_TASK_13_a_file_and_an_acceptance_that_are_committed_hand_out_the_prompt() {
    let folder = accepted_apart();
    commit(&folder.path, &[YML, ACCEPTED]);
    for role in ROLES {
        let run = prompt(&folder.path, role, &[]);
        assert_eq!(code(&run), Some(0), "{}: {}", role, text(&run.stderr));
        // The text of the fix holds no requirements, so only the others show them.
        assert!(role == "fix" || text(&run.stdout).contains("54.1: The tool reads the file."), "{}", role);
        assert!(text(&run.stdout).contains("fn read(path: &str) -> String"), "{}", role);
    }
}

#[test]
fn test_WB_TASK_13_an_acceptance_that_is_written_and_not_committed_is_refused_with_its_name() {
    let folder = accepted_apart();
    commit(&folder.path, &[YML]);
    for role in ROLES {
        for extra in [&[][..], &["--parts"], &["--stamp"]] {
            let run = prompt(&folder.path, role, extra);
            refused(&run, ACCEPTED);
            assert!(text(&run.stderr).contains("commit"), "{}", text(&run.stderr));
        }
    }
}

#[test]
fn test_WB_TASK_13_a_file_that_is_not_committed_is_refused_with_its_name() {
    let folder = accepted_apart();
    commit(&folder.path, &[ACCEPTED]);
    for role in ROLES {
        refused(&prompt(&folder.path, role, &[]), YML);
    }
}

#[test]
fn test_WB_TASK_13_nothing_is_committed_when_the_repository_has_no_commit_yet() {
    let folder = set_up();
    put(&folder.path, YML, GOOD);
    assert_eq!(code(&wb(&folder.path, &["accept", "54"], "")), Some(0));
    refused(&prompt(&folder.path, "build", &[]), YML);
}

#[test]
fn test_WB_TASK_13_a_file_changed_in_the_working_tree_after_its_commit_is_refused() {
    let folder = accepted_apart();
    commit(&folder.path, &[YML, ACCEPTED]);
    put(&folder.path, YML, &GOOD.replace("reads the file", "reads a file"));
    for role in ROLES {
        refused(&prompt(&folder.path, role, &[]), YML);
    }
}

#[test]
fn test_WB_TASK_13_an_acceptance_changed_in_the_working_tree_after_its_commit_is_refused_with_its_name() {
    let folder = accepted_apart();
    commit(&folder.path, &[YML, ACCEPTED]);
    // The line of the hash still holds, so only the commit tells that the file is not as it was.
    let mut body = read(&folder.path.join(ACCEPTED));
    body.push_str("note: changed later\n");
    put(&folder.path, ACCEPTED, &body);
    for role in ROLES {
        refused(&prompt(&folder.path, role, &[]), ACCEPTED);
    }
}

#[test]
fn test_WB_TASK_13_the_breakdown_is_never_asked_for_an_acceptance() {
    let folder = set_up();
    let run = prompt(&folder.path, "breakdown", &[]);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    put(&folder.path, YML, GOOD);
    let run = prompt(&folder.path, "breakdown", &[]);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
}

#[test]
fn test_WB_TASK_14_an_issue_with_no_file_is_refused_with_the_file_named() {
    let folder = set_up();
    commit(&folder.path, &[".watchbill"]);
    for role in ROLES {
        for extra in [&[][..], &["--parts"], &["--stamp"]] {
            let run = prompt(&folder.path, role, extra);
            refused(&run, YML);
            assert!(text(&run.stderr).contains("breakdown"), "{}", text(&run.stderr));
        }
    }
}

#[test]
fn test_WB_TASK_14_where_breakdown_is_not_written_the_file_is_needed() {
    let folder = set_up();
    let before = read(&config(&folder.path));
    assert!(!before.contains("breakdown"));
    refused(&prompt(&folder.path, "build", &[]), YML);
}

#[test]
fn test_WB_TASK_14_breakdown_none_means_no_file_is_needed_where_the_rule_table_lacks_the_breakdown() {
    let folder = set_up();
    let mut yml = read(&config(&folder.path));
    yml.push_str("breakdown: none\n");
    put(&folder.path, ".watchbill/watchbill.yml", &yml);
    let needed = |holds| watchbill::issue::file_needed(&folder.path, holds);
    assert_eq!(needed(false), Ok(false));
    assert!(needed(true).is_err());
    let bare = set_up();
    assert_eq!(watchbill::issue::file_needed(&bare.path, false), Ok(true));
    assert_eq!(watchbill::issue::file_needed(&bare.path, true), Ok(true));
}

#[test]
fn test_WB_TASK_14_a_breakdown_that_is_not_none_is_refused_by_its_name() {
    let folder = set_up();
    let mut yml = read(&config(&folder.path));
    yml.push_str("breakdown: sometimes\n");
    put(&folder.path, ".watchbill/watchbill.yml", &yml);
    let run = prompt(&folder.path, "build", &[]);
    refused(&run, "breakdown");
    assert!(text(&run.stderr).contains("none"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_TASK_15_breakdown_none_is_refused_when_the_rule_table_holds_the_breakdown() {
    let folder = set_up();
    commit(&folder.path, &[".watchbill"]);
    let mut yml = read(&config(&folder.path));
    yml.push_str("breakdown: none\n");
    put(&folder.path, ".watchbill/watchbill.yml", &yml);
    for role in ROLES {
        let run = prompt(&folder.path, role, &[]);
        refused(&run, "breakdown: none");
        assert!(text(&run.stderr).contains("rule table"), "{}", text(&run.stderr));
    }
}

#[test]
fn test_WB_TASK_16_wb_accept_says_that_the_acceptance_is_committed_alone_with_the_command() {
    let folder = set_up();
    put(&folder.path, YML, GOOD);
    let run = wb(&folder.path, &["accept", "54"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let said = text(&run.stdout);
    assert!(said.contains("alone"), "{}", said);
    assert!(said.contains(&format!("git add {} && git commit", ACCEPTED)), "{}", said);
    assert!(said.contains(&format!("-- {}", ACCEPTED)), "{}", said);
    assert!(said.find("is accepted").unwrap() < said.find("alone").unwrap(), "{}", said);
}
