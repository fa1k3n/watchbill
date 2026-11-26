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
    // An acceptance is refused without `.watchbill/signers` on the default branch (#323).
    signing(&folder.path);
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

    let run = wb(&folder.path, &["plan", "accept", "54"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    assert_eq!(count(&folder.path), before + 1);
    assert_eq!(say(&folder.path, &["show", "--name-only", "--format=", "HEAD"]).trim(), ACCEPTED);
    assert_eq!(say(&folder.path, &["log", "-1", "--format=%s"]).trim(), MESSAGE);
    assert_eq!(say(&folder.path, &["log", "-1", "--format=%an <%ae>"]).trim(), "Ada Lovelace <ada@example.org>");
    assert_eq!(say(&folder.path, &["status", "--porcelain"]), "");
    let status = wb(&folder.path, &["status", "54"], "");
    let said = text(&status.stdout);
    assert!(said.contains("\t54  issue-54") && said.contains("  accepted"), "{}", said);
}

#[test]
fn test_WB_TASK_23_wb_accept_pushes_nothing_and_says_the_branch_is_to_be_pushed() {
    let folder = on_the_branch();
    let run = wb(&folder.path, &["plan", "accept", "54"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let said = text(&run.stdout);
    assert!(said.contains("issue-54") && said.contains("push"), "{}", said);
}

#[test]
fn test_WB_TASK_24_what_was_staged_before_stays_staged_and_is_not_in_the_commit() {
    let folder = on_the_branch();
    put(&folder.path, "src/other.rs", "fn other() {}\n");
    git(&folder.path, &["add", "--", "src/other.rs"]);

    let run = wb(&folder.path, &["plan", "accept", "54"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    assert_eq!(say(&folder.path, &["show", "--name-only", "--format=", "HEAD"]).trim(), ACCEPTED);
    assert_eq!(say(&folder.path, &["diff", "--cached", "--name-only"]).trim(), "src/other.rs");
}

#[test]
fn test_WB_TASK_25_it_says_what_it_will_commit_before_it_commits() {
    let folder = on_the_branch();
    // The hook copies what `wb` has said so far, at the moment of the commit.
    let said_file = folder.path.join("..").join("said-so-far");
    let out = folder.path.join("..").join("stdout");
    let hook = folder.path.join(".git/hooks/pre-commit");
    fs::write(&hook, format!("#!/bin/sh\ncp {} {}\nexit 0\n", out.display(), said_file.display())).unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();

    let run = Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(["plan", "accept", "54"])
        .current_dir(&folder.path)
        .env_clear()
        .envs(env())
        .stdout(fs::File::create(&out).unwrap())
        .output()
        .unwrap();

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let before = fs::read_to_string(&said_file).expect("the hook did not run");
    assert!(before.contains("will commit") && before.contains(ACCEPTED) && before.contains(MESSAGE), "{}", before);
    assert!(!before.contains("committed .watchbill"), "{}", before);
}

#[test]
fn test_WB_TASK_26_not_on_the_issues_branch_refuses_with_both_branches_named_and_writes_nothing() {
    let folder = on_the_branch();
    git(&folder.path, &["checkout", "-q", "-b", "other-work"]);
    let before = files(&folder.path);
    let commits = count(&folder.path);

    let run = wb(&folder.path, &["plan", "accept", "54"], "");

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

    let run = wb(&folder.path, &["plan", "accept", "54"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    assert!(text(&run.stderr).contains(YML) && text(&run.stderr).contains("commit"), "{}", text(&run.stderr));
    assert_eq!(files(&folder.path), before);
}

#[test]
fn test_WB_TASK_27_an_issues_file_changed_since_head_refuses_with_1_and_writes_nothing() {
    let folder = on_the_branch();
    put(&folder.path, YML, &GOOD.replace("The tool reads the file.", "The tool reads it."));
    let before = files(&folder.path);

    let run = wb(&folder.path, &["plan", "accept", "54"], "");

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

    let run = wb(&folder.path, &["plan", "accept", "54"], "");

    assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
    assert!(folder.path.join(ACCEPTED).is_file());
    assert_eq!(count(&folder.path), commits);
    let said = text(&run.stdout) + &text(&run.stderr);
    assert!(said.contains("commit") && said.contains("fail"), "{}", said);
    assert!(said.contains("git add"), "{}", said);
    assert_eq!(say(&folder.path, &["diff", "--cached", "--name-only"]), "", "the acceptance was left staged");
    assert!(said.contains("unstaged"), "{}", said);
}

#[test]
fn test_WB_TASK_28_what_was_staged_before_stays_staged_after_a_failed_commit() {
    let folder = on_the_branch();
    put(&folder.path, "src/other.rs", "fn other() {}\n");
    git(&folder.path, &["add", "--", "src/other.rs"]);
    let hook = folder.path.join(".git/hooks/pre-commit");
    fs::write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();

    let run = wb(&folder.path, &["plan", "accept", "54"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(say(&folder.path, &["diff", "--cached", "--name-only"]).trim(), "src/other.rs");
}

#[test]
fn test_WB_TASK_28_a_hook_that_adds_another_file_is_refused_and_says_what_the_commit_holds() {
    let folder = on_the_branch();
    put(&folder.path, "src/z.rs", "fn z() {}\n");
    let hook = folder.path.join(".git/hooks/pre-commit");
    fs::write(&hook, "#!/bin/sh\ngit add -- src/z.rs\nexit 0\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();

    let run = wb(&folder.path, &["plan", "accept", "54"], "");

    assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
    let said = text(&run.stdout) + &text(&run.stderr);
    assert!(said.contains("src/z.rs") && said.contains("not alone"), "{}", said);
    assert!(!said.contains("committed .watchbill/issues/54.accepted alone"), "{}", said);
}

/// `wb accept 54` run with more of the environment set.
fn wb_with(folder: &Path, extra: &[(&str, &str)]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(["plan", "accept", "54"])
        .current_dir(folder)
        .env_clear()
        .envs(env())
        .envs(extra.iter().copied())
        .output()
        .unwrap()
}

#[test]
fn test_WB_TASK_23_the_author_is_the_identity_of_the_line_by_whatever_the_environment_says() {
    let folder = on_the_branch();
    let run = wb_with(&folder.path, &[("GIT_AUTHOR_NAME", "Mallory"), ("GIT_AUTHOR_EMAIL", "m@example.org")]);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(say(&folder.path, &["log", "-1", "--format=%an <%ae>"]).trim(), "Ada Lovelace <ada@example.org>");
}

#[test]
fn test_WB_TASK_26_the_branch_of_the_issue_with_words_after_it_is_taken() {
    let folder = on_the_branch();
    git(&folder.path, &["branch", "-m", "issue-54-words"]);
    let run = wb(&folder.path, &["plan", "accept", "54"], "");
    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
}

#[test]
fn test_WB_TASK_26_the_branch_of_issue_541_is_not_the_branch_of_54() {
    let folder = on_the_branch();
    git(&folder.path, &["branch", "-m", "issue-541"]);
    let before = files(&folder.path);
    let run = wb(&folder.path, &["plan", "accept", "54"], "");
    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stderr).contains("issue-541") && text(&run.stderr).contains("issue-54"), "{}", text(&run.stderr));
    assert_eq!(files(&folder.path), before);
}

#[test]
fn test_WB_TASK_26_a_detached_head_is_refused_and_nothing_is_written() {
    let folder = on_the_branch();
    git(&folder.path, &["checkout", "-q", "--detach"]);
    let before = files(&folder.path);
    let run = wb(&folder.path, &["plan", "accept", "54"], "");
    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stderr).contains("issue-54"), "{}", text(&run.stderr));
    assert_eq!(files(&folder.path), before);
}

#[test]
fn test_WB_TASK_26_without_a_pattern_branch_it_refuses_with_1_and_no_commit_still_accepts() {
    let folder = on_the_branch();
    let yml = read(&config(&folder.path));
    let without: String = yml.lines().filter(|line| !line.starts_with("branch:")).map(|line| format!("{}\n", line)).collect();
    assert_ne!(yml, without);
    put(&folder.path, ".watchbill/watchbill.yml", &without);
    commit_config(&folder.path);
    let before = files(&folder.path);
    let run = wb(&folder.path, &["plan", "accept", "54"], "");
    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    assert!(text(&run.stderr).contains("branch"), "{}", text(&run.stderr));
    assert_eq!(files(&folder.path), before);
    let run = wb(&folder.path, &["plan", "accept", "54", "--no-commit"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
}

#[test]
fn test_WB_TASK_6_a_refusal_on_the_plain_path_says_nothing_of_a_write_and_commits_nothing() {
    let folder = on_the_branch();
    let asking = GOOD.replace("questions: []", "questions: [what?]");
    put(&folder.path, YML, &asking);
    git(&folder.path, &["add", "--", YML]);
    git(&folder.path, &["commit", "-q", "-m", "A question"]);
    let before = files(&folder.path);
    let commits = count(&folder.path);

    let run = wb(&folder.path, &["plan", "accept", "54"], "");

    assert_eq!(code(&run), Some(1));
    assert!(!text(&run.stdout).contains("will write"), "{}", text(&run.stdout));
    assert_eq!(files(&folder.path), before);
    assert_eq!(count(&folder.path), commits);
}

#[test]
fn test_WB_TASK_8_on_the_plain_path_an_acceptance_that_is_there_is_never_written_over() {
    let folder = on_the_branch();
    put(&folder.path, ACCEPTED, "sha256: old\n");
    let commits = count(&folder.path);

    let run = wb(&folder.path, &["plan", "accept", "54"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(read(&folder.path.join(ACCEPTED)), "sha256: old\n");
    assert_eq!(count(&folder.path), commits);
}

#[test]
fn test_WB_TASK_9_on_the_plain_path_no_identity_refuses_with_1_and_writes_nothing() {
    let folder = on_the_branch();
    git(&folder.path, &["config", "--unset", "user.name"]);
    git(&folder.path, &["config", "--unset", "user.email"]);
    let home = folder.path.join("..").join("no-home");
    fs::create_dir_all(&home).unwrap();
    let before = files(&folder.path);
    let commits = count(&folder.path);

    let run = wb_with(
        &folder.path,
        &[("HOME", home.to_str().unwrap()), ("XDG_CONFIG_HOME", home.to_str().unwrap()), ("GIT_CONFIG_GLOBAL", "/dev/null"), ("GIT_CONFIG_NOSYSTEM", "1")],
    );

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    assert_eq!(files(&folder.path), before);
    assert_eq!(count(&folder.path), commits);
}

#[test]
fn test_WB_TASK_30_the_plain_path_writes_nothing_but_the_acceptance_and_commits_nothing_but_that_file() {
    let folder = on_the_branch();
    let before = files(&folder.path);

    let run = wb(&folder.path, &["plan", "accept", "54"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    let after = files(&folder.path);
    let new: Vec<&String> = after.iter().map(|(name, _)| name).filter(|name| !before.iter().any(|(old, _)| &old == name)).collect();
    assert_eq!(new, [&ACCEPTED.to_string()]);
    assert_eq!(after.iter().filter(|(name, bytes)| before.iter().any(|(old, was)| old == name && was != bytes)).count(), 0);
    assert_eq!(say(&folder.path, &["show", "--name-only", "--format=", "HEAD"]).trim(), ACCEPTED);
}

#[test]
fn test_WB_TASK_29_no_commit_writes_the_acceptance_commits_nothing_and_prints_the_command() {
    let folder = on_the_branch();
    let commits = count(&folder.path);

    let run = wb(&folder.path, &["plan", "accept", "54", "--no-commit"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    assert!(folder.path.join(ACCEPTED).is_file());
    assert_eq!(count(&folder.path), commits);
    let said = text(&run.stdout);
    assert!(said.contains(&format!("git add {} && git commit", ACCEPTED)), "{}", said);
}
