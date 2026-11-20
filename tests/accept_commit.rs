//! `wb accept` commits the acceptance itself, alone, on the issue's branch (#157), run as a person runs it,
//! in a repository made for the test.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::path::Path;
use std::process::Command;

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

const YML: &str = ".watchbill/issues/54.yml";
const ACCEPTED: &str = ".watchbill/issues/54.accepted";
const MESSAGE: &str = "Accept the issue's file (54)";

/// What git says, as the repository's own git, with its output.
fn say(folder: &Path, args: &[&str]) -> String {
    let run = Command::new("git").arg("-C").arg(folder).args(args).env_clear().envs(env()).output().expect("git could not be started");
    assert!(run.status.success(), "git {:?}: {}", args, text(&run.stderr));
    text(&run.stdout)
}

/// A repository set up by `wb init --yes`, on the branch `issue-54`, with the issue's file committed alone.
fn on_the_branch() -> Folder {
    let folder = repository(None);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    git(&folder.path, &["add", "--", ".watchbill"]);
    git(&folder.path, &["commit", "-q", "-m", "Set up"]);
    git(&folder.path, &["checkout", "-q", "-b", "issue-54"]);
    put(&folder.path, YML, GOOD);
    git(&folder.path, &["add", "--", YML]);
    git(&folder.path, &["commit", "-q", "-m", "The issue's file (54)"]);
    folder
}

fn count(folder: &Path) -> usize {
    say(folder, &["rev-list", "--count", "HEAD"]).trim().parse().unwrap()
}

/// Every file under a folder with its bytes, apart from `.git`.
fn files(folder: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &Path, at: &Path, found: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().map(|name| name == ".git").unwrap_or(false) {
                continue;
            }
            if path.is_dir() {
                walk(base, &path, found);
            } else {
                found.push((path.strip_prefix(base).unwrap().to_str().unwrap().to_string(), fs::read(&path).unwrap()));
            }
        }
    }
    let mut found = Vec::new();
    walk(folder, folder, &mut found);
    found.sort();
    found
}

#[test]
fn test_WB_TASK_23_the_acceptance_is_committed_alone_with_the_message_and_the_author() {
    let folder = on_the_branch();
    let before = count(&folder.path);

    let run = wb(&folder.path, &["accept", "54"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    assert_eq!(count(&folder.path), before + 1);
    assert_eq!(say(&folder.path, &["show", "--name-only", "--format=", "HEAD"]).trim(), ACCEPTED);
    assert_eq!(say(&folder.path, &["log", "-1", "--format=%s"]).trim(), MESSAGE);
    assert_eq!(say(&folder.path, &["log", "-1", "--format=%an <%ae>"]).trim(), "Ada Lovelace <ada@example.org>");
    assert_eq!(say(&folder.path, &["status", "--porcelain"]), "");
    let status = wb(&folder.path, &["status", "54"], "");
    let said = text(&status.stdout);
    assert!(said.contains("Accepted"), "{}", said);
}

#[test]
fn test_WB_TASK_23_wb_accept_pushes_nothing_and_says_the_branch_is_to_be_pushed() {
    let folder = on_the_branch();
    let run = wb(&folder.path, &["accept", "54"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let said = text(&run.stdout);
    assert!(said.contains("issue-54") && said.contains("push"), "{}", said);
}

#[test]
fn test_WB_TASK_24_what_was_staged_before_stays_staged_and_is_not_in_the_commit() {
    let folder = on_the_branch();
    put(&folder.path, "src/other.rs", "fn other() {}\n");
    git(&folder.path, &["add", "--", "src/other.rs"]);

    let run = wb(&folder.path, &["accept", "54"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    assert_eq!(say(&folder.path, &["show", "--name-only", "--format=", "HEAD"]).trim(), ACCEPTED);
    assert_eq!(say(&folder.path, &["diff", "--cached", "--name-only"]).trim(), "src/other.rs");
}

#[test]
fn test_WB_TASK_25_it_says_what_it_will_commit_before_it_commits() {
    let folder = on_the_branch();
    // A hook that reads what the command has said so far cannot be had; the order is in what it printed.
    let run = wb(&folder.path, &["accept", "54"], "");
    let said = text(&run.stdout);
    let will = said.find("will commit").expect(&said);
    assert!(said[will..].contains(ACCEPTED) && said[will..].contains(MESSAGE), "{}", said);
    assert!(will > said.find("will write").expect(&said), "{}", said);
    assert!(will < said.find("is accepted").expect(&said) || said.find("committed").map(|done| will < done).unwrap_or(false), "{}", said);
}

#[test]
fn test_WB_TASK_26_not_on_the_issues_branch_refuses_with_both_branches_named_and_writes_nothing() {
    let folder = on_the_branch();
    git(&folder.path, &["checkout", "-q", "-b", "other-work"]);
    let before = files(&folder.path);
    let commits = count(&folder.path);

    let run = wb(&folder.path, &["accept", "54"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    let said = text(&run.stderr);
    assert!(said.contains("other-work") && said.contains("issue-54"), "{}", said);
    assert_eq!(files(&folder.path), before);
    assert_eq!(count(&folder.path), commits);
}

#[test]
fn test_WB_TASK_27_an_issues_file_that_is_not_committed_refuses_with_1_and_writes_nothing() {
    let folder = on_the_branch();
    git(&folder.path, &["rm", "-q", "--cached", "--", YML]);
    git(&folder.path, &["commit", "-q", "-m", "Drop it"]);
    let before = files(&folder.path);

    let run = wb(&folder.path, &["accept", "54"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    assert!(text(&run.stderr).contains(YML) && text(&run.stderr).contains("commit"), "{}", text(&run.stderr));
    assert_eq!(files(&folder.path), before);
}

#[test]
fn test_WB_TASK_27_an_issues_file_changed_since_head_refuses_with_1_and_writes_nothing() {
    let folder = on_the_branch();
    put(&folder.path, YML, &GOOD.replace("The tool reads the file.", "The tool reads it."));
    let before = files(&folder.path);

    let run = wb(&folder.path, &["accept", "54"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    assert!(text(&run.stderr).contains(YML) && text(&run.stderr).contains("commit"), "{}", text(&run.stderr));
    assert_eq!(files(&folder.path), before);
}

#[test]
fn test_WB_TASK_28_a_commit_that_fails_leaves_the_acceptance_says_so_and_ends_with_1() {
    let folder = on_the_branch();
    let hook = folder.path.join(".git/hooks/pre-commit");
    fs::write(&hook, "#!/bin/sh\necho hook says no >&2\nexit 1\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    let commits = count(&folder.path);

    let run = wb(&folder.path, &["accept", "54"], "");

    assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
    assert!(folder.path.join(ACCEPTED).is_file());
    assert_eq!(count(&folder.path), commits);
    let said = text(&run.stdout) + &text(&run.stderr);
    assert!(said.contains("commit") && said.contains("fail"), "{}", said);
    assert!(said.contains("git add"), "{}", said);
}

#[test]
fn test_WB_TASK_29_no_commit_writes_the_acceptance_commits_nothing_and_prints_the_command() {
    let folder = on_the_branch();
    let commits = count(&folder.path);

    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    assert!(folder.path.join(ACCEPTED).is_file());
    assert_eq!(count(&folder.path), commits);
    let said = text(&run.stdout);
    assert!(said.contains(&format!("git add {} && git commit", ACCEPTED)), "{}", said);
}
