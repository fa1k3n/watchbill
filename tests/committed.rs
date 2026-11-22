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
    commit_config(&folder.path);
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
    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");
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
    assert_eq!(code(&wb(&folder.path, &["accept", "54", "--no-commit"], "")), Some(0));
    refused(&prompt(&folder.path, "build", &[]), YML);
}

#[test]
fn test_WB_TASK_13_a_file_changed_in_the_working_tree_after_its_commit_is_refused() {
    let folder = accepted_apart();
    commit(&folder.path, &[YML, ACCEPTED]);
    // Accepted again, so that only the commit can tell that the file is not as it was.
    put(&folder.path, YML, &GOOD.replace("reads the file", "reads a file"));
    std::fs::remove_file(folder.path.join(ACCEPTED)).unwrap();
    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    for role in ROLES {
        refused(&prompt(&folder.path, role, &[]), YML);
    }
}

#[test]
fn test_WB_TASK_13_a_checkout_that_git_calls_clean_is_not_refused_for_its_line_ends() {
    let folder = set_up();
    commit(&folder.path, &[".watchbill"]);
    put(&folder.path, ".gitattributes", ".watchbill/issues/* text eol=crlf\n");
    commit(&folder.path, &[".gitattributes"]);
    put(&folder.path, YML, GOOD);
    commit(&folder.path, &[YML]);
    // The checkout writes the file with CRLF; the acceptance is made of it as it is, then checked out the same way.
    std::fs::remove_file(folder.path.join(YML)).unwrap();
    git(&folder.path, &["checkout", "--", YML]);
    assert!(read(&folder.path.join(YML)).contains("\r\n"));
    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    commit(&folder.path, &[ACCEPTED]);
    std::fs::remove_file(folder.path.join(ACCEPTED)).unwrap();
    git(&folder.path, &["checkout", "--", ACCEPTED]);
    let status = std::process::Command::new("git").args(["status", "--porcelain"]).current_dir(&folder.path).output().unwrap();
    assert_eq!(text(&status.stdout), "", "git calls the checkout clean");
    for role in ROLES {
        let run = prompt(&folder.path, role, &[]);
        assert_eq!(code(&run), Some(0), "{}: {}", role, text(&run.stderr));
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
    // The breakdown is handed out only to a runner the organisation cleared for it (#93).
    let (_place, organisation) = cleared(&folder.path);
    let breakdown = |folder: &Path| {
        let mut args = vec!["breakdown", "54"];
        args.extend(REST);
        args.extend(["--runner", "claude-code"]);
        prompt_with(folder, &organisation, &args)
    };
    let run = breakdown(&folder.path);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    put(&folder.path, YML, GOOD);
    let run = breakdown(&folder.path);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
}

#[test]
fn test_WB_TASK_17_an_issue_with_no_file_is_handed_out_to_each_role_and_no_file_is_named() {
    // Was test_WB_TASK_14_an_issue_with_no_file_is_refused_with_the_file_named. Since #116 an issue with
    // no file is handed out.
    let folder = set_up();
    commit(&folder.path, &[".watchbill"]);
    for role in ROLES {
        for extra in [&[][..], &["--parts"], &["--stamp"]] {
            let run = prompt(&folder.path, role, extra);
            assert_eq!(code(&run), Some(0), "{} {:?}: {}", role, extra, text(&run.stderr));
            // No part of the issue's file is printed, whatever the role: its headings are only in that part (#227).
            for heading in ["## The interface", "## What you may change and read"] {
                assert!(!text(&run.stdout).contains(heading), "{} {:?}: {}", role, extra, heading);
            }
            // The review's text names the issue's file as a file of the process, and says it is no finding (#227).
            assert!(role == "review" && extra.is_empty() || !text(&run.stdout).contains(YML), "{} {:?}", role, extra);
            assert_eq!(text(&run.stderr), "", "{} {:?}", role, extra);
        }
    }
}

#[test]
fn test_WB_TASK_17_where_breakdown_is_not_written_no_file_is_needed() {
    // Was test_WB_TASK_14_where_breakdown_is_not_written_the_file_is_needed.
    let folder = set_up();
    let before = read(&config(&folder.path));
    assert!(!before.contains("breakdown"));
    let run = prompt(&folder.path, "build", &[]);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
}

// test_WB_PROMPT_50_breakdown_none_means_no_file_is_needed_where_the_rule_table_lacks_the_breakdown is
// removed with `issue::file_needed`, which it called (#116). WB-PROMPT-50 is replaced by WB-TASK-17.

#[test]
fn test_WB_TASK_18_a_breakdown_that_is_not_none_is_refused_by_its_name() {
    // Was test_WB_TASK_14_a_breakdown_that_is_not_none_is_refused_by_its_name: any value is refused now.
    let folder = set_up();
    let mut yml = read(&config(&folder.path));
    yml.push_str("breakdown: sometimes\n");
    put(&folder.path, ".watchbill/watchbill.yml", &yml);
    commit_config(&folder.path);
    let run = prompt(&folder.path, "build", &[]);
    refused(&run, "breakdown: sometimes");
    assert!(text(&run.stderr).contains("Remove the line"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_TASK_18_breakdown_none_is_refused_for_each_role() {
    // Was test_WB_TASK_15_breakdown_none_is_refused_when_the_rule_table_holds_the_breakdown: refused
    // whatever the rule table holds.
    let folder = set_up();
    commit(&folder.path, &[".watchbill"]);
    let mut yml = read(&config(&folder.path));
    yml.push_str("breakdown: none\n");
    put(&folder.path, ".watchbill/watchbill.yml", &yml);
    commit_config(&folder.path);
    for role in ROLES {
        let run = prompt(&folder.path, role, &[]);
        refused(&run, "breakdown: none");
        assert!(text(&run.stderr).contains("Remove the line"), "{}", text(&run.stderr));
    }
}

#[test]
fn test_WB_TASK_16_wb_accept_says_that_the_acceptance_is_committed_alone_with_the_command() {
    let folder = set_up();
    put(&folder.path, YML, GOOD);
    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let said = text(&run.stdout);
    assert!(said.contains("alone"), "{}", said);
    assert!(said.contains(&format!("git add {} && git commit", ACCEPTED)), "{}", said);
    assert!(said.contains(&format!("-- {}", ACCEPTED)), "{}", said);
    assert!(said.find("is accepted").unwrap() < said.find("alone").unwrap(), "{}", said);
}

#[test]
fn test_WB_TASK_18_wb_check_refuses_breakdown_none_and_any_other_value() {
    // Was test_WB_TASK_15_wb_check_refuses_breakdown_none_when_the_rule_table_holds_the_breakdown.
    let folder = set_up();
    assert_eq!(code(&wb(&folder.path, &["check"], "")), Some(0));
    let mut yml = read(&config(&folder.path));
    yml.push_str("breakdown: none\n");
    put(&folder.path, ".watchbill/watchbill.yml", &yml);
    commit_config(&folder.path);
    let run = wb(&folder.path, &["check"], "");
    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    let said = text(&run.stdout) + &text(&run.stderr);
    assert!(said.contains("breakdown: none") && said.contains("Remove the line"), "{}", said);
    let mut other = read(&config(&folder.path)).replace("breakdown: none", "breakdown: sometimes");
    other.push('\n');
    put(&folder.path, ".watchbill/watchbill.yml", &other);
    assert_eq!(code(&wb(&folder.path, &["check"], "")), Some(1));
}

#[test]
fn test_WB_TASK_17_with_no_file_requirements_is_the_numbered_requirements_of_the_issue_in_derive() {
    // Was test_WB_PROMPT_51_with_breakdown_none_and_no_file_requirements_is_the_numbered_requirements_of_the_issue.
    // Since #116 no line of `watchbill.yml` is needed for it.
    let folder = set_up();
    let root = folder.path.to_str().unwrap();
    let given = [("issue", "54".to_string()), ("branch", "issue-54".to_string())];
    let values = watchbill::prompt::derive(root, "build", &given).ok().expect("derive");
    let found = values.iter().find(|(name, _)| *name == "requirements").map(|(_, value)| value.as_str());
    assert_eq!(found, Some("the numbered requirements of issue 54"));
    let given = [("issue", "54".to_string()), ("branch", "issue-54".to_string()), ("requirements", "R".to_string())];
    let values = watchbill::prompt::derive(root, "build", &given).ok().expect("derive");
    assert_eq!(values.iter().filter(|(name, _)| *name == "requirements").count(), 1);
    assert!(values.contains(&("requirements", "R".to_string())));
}
