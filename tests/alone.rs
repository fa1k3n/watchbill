//! A person starts a role alone (#116): an issue with no file is handed out, `breakdown:` in
//! `watchbill.yml` is refused by name, and a review or a fix of a change takes no issue. Run as a person
//! runs it, in a repository made for the test.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::path::Path;
use std::process::Output;
use watchbill::USAGE;

const GH: &str = "git@github.com:example/brake-controller.git";
const ROLES: [&str; 4] = ["tester", "build", "review", "fix"];

/// A repository with a GitHub `origin`, set up by `wb init --yes` and committed on `main`. `origin/HEAD`
/// names `main`, as a clone has it. The branch of issue 54 is there, and `feature-x`, the branch of a
/// change with no issue, is checked out. No issue has a file.
fn set_up() -> Folder {
    let folder = repository(Some(GH));
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    git(&folder.path, &["add", "--", ".watchbill"]);
    git(&folder.path, &["commit", "-q", "-m", "Watchbill"]);
    git(&folder.path, &["branch", "-M", "main"]);
    git(&folder.path, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(&folder.path, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"]);
    // No role is handed out without `.watchbill/signers` (#323).
    signing(&folder.path);
    git(&folder.path, &["branch", "issue-54"]);
    git(&folder.path, &["checkout", "-q", "-b", "feature-x"]);
    folder
}

fn run(folder: &Path, args: &[&str]) -> Output {
    let mut all = vec!["prompt"];
    all.extend(args);
    wb(folder, &all, "")
}

fn ok(run: &Output) -> String {
    assert_eq!(code(run), Some(0), "{}", text(&run.stderr));
    text(&run.stdout)
}

/// The value of one line `name: value` of a stamp; `Some("")` for a line of the name and the colon alone.
fn field<'a>(stamp: &'a str, name: &str) -> Option<&'a str> {
    stamp.lines().find_map(|line| {
        let rest = line.strip_prefix(name)?.strip_prefix(':')?;
        Some(rest.strip_prefix(' ').unwrap_or(rest))
    })
}

fn add_line(folder: &Path, line: &str) -> usize {
    let mut yml = read(&config(folder));
    yml.push_str(line);
    put(folder, ".watchbill/watchbill.yml", &yml);
    commit_config(folder);
    yml.lines().count()
}

// ---- WB-TASK-17: an issue with no file is handed out ----

#[test]
fn test_WB_TASK_17_an_issue_with_no_file_hands_out_the_tester_the_build_the_review_and_the_fix() {
    let folder = set_up();
    for role in ROLES {
        let base = [role, "54", "--change", "34", "--round", "1"];
        let said = ok(&run(&folder.path, &base));
        assert!(said.contains("54"), "{}: {}", role, said);
        // No part of an issue's file is in it.
        assert!(!said.contains("## The interface"), "{}: {}", role, said);
        let mut parts = base.to_vec();
        parts.push("--parts");
        let lines = ok(&run(&folder.path, &parts));
        assert_eq!(lines.lines().count(), 1, "{}: {}", role, lines);
        assert!(lines.ends_with(&format!("  .watchbill/revision/roles/{}.md\n", role)), "{}: {}", role, lines);
        let mut stamp = base.to_vec();
        stamp.push("--stamp");
        let stamp = ok(&run(&folder.path, &stamp));
        assert_eq!(field(&stamp, "issue"), Some("54"), "{}: {}", role, stamp);
        assert_eq!(field(&stamp, "branch"), Some("issue-54"), "{}: {}", role, stamp);
    }
}

#[test]
fn test_WB_TASK_17_with_no_file_requirements_is_the_numbered_requirements_of_the_issue() {
    let folder = set_up();
    // The text of the fix holds no `{requirements}`: it answers findings.
    for role in ["tester", "build", "review"] {
        let said = ok(&run(&folder.path, &[role, "54", "--change", "34", "--round", "1"]));
        assert!(said.contains("the numbered requirements of issue 54"), "{}: {}", role, said);
    }
}

#[test]
fn test_WB_TASK_17_with_no_file_an_option_for_the_requirements_wins() {
    let folder = set_up();
    for role in ["tester", "build", "review"] {
        let said = ok(&run(&folder.path, &[role, "54", "--change", "34", "--round", "1", "--requirements", "WB-X-1"]));
        assert!(said.contains("WB-X-1"), "{}: {}", role, said);
        assert!(!said.contains("the numbered requirements of issue 54"), "{}: {}", role, said);
    }
}

#[test]
fn test_WB_TASK_17_with_no_file_an_issue_that_is_no_name_of_a_file_is_handed_out() {
    let folder = set_up();
    let said = ok(&run(&folder.path, &["build", "--issue", "a/b", "--branch", "issue-54", "--issue_address", "tracker"]));
    assert!(said.contains("a/b"), "{}", said);
}

// ---- WB-TASK-18: the key `breakdown` is refused by name ----

#[test]
fn test_WB_TASK_18_wb_check_refuses_the_key_breakdown_names_its_line_and_says_to_remove_it() {
    for value in ["none", "other", "[a, b]"] {
        let folder = set_up();
        let line = add_line(&folder.path, &format!("breakdown: {}\n", value));
        let run = wb(&folder.path, &["check"], "");
        assert_eq!(code(&run), Some(1), "{}: {}", value, text(&run.stdout));
        let out = text(&run.stdout);
        let said = out.lines().find(|said| said.contains("breakdown")).unwrap_or_else(|| panic!("{}: {}", value, out));
        assert!(said.contains(&format!("line {}", line)), "{}: {}", value, said);
        assert!(said.contains("Remove the line"), "{}: {}", value, said);
        assert!(!said.contains("rule table"), "{}: {}", value, said);
    }
}

#[test]
fn test_WB_TASK_18_wb_prompt_refuses_the_key_breakdown_with_1_and_prints_nothing() {
    let folder = set_up();
    let line = add_line(&folder.path, "breakdown: none\n");
    for role in ROLES {
        for extra in [&[][..], &["--parts"], &["--stamp"]] {
            let mut args = vec![role, "54", "--change", "34", "--round", "1"];
            args.extend(extra);
            let found = run(&folder.path, &args);
            assert_eq!(code(&found), Some(1), "{}: {}", role, text(&found.stderr));
            assert_eq!(text(&found.stdout), "", "{}", role);
            let why = text(&found.stderr);
            assert!(why.contains(&format!("line {}", line)) && why.contains("Remove the line"), "{}: {}", role, why);
        }
    }
}

#[test]
fn test_WB_TASK_18_the_derivation_of_wb_prompt_refuses_the_key_breakdown_by_itself() {
    let folder = set_up();
    add_line(&folder.path, "breakdown: none\n");
    let root = folder.path.to_str().unwrap();
    let given = [("issue", "54".to_string()), ("branch", "issue-54".to_string())];
    match watchbill::prompt::derive(root, "build", &given) {
        Err(watchbill::prompt::Derive::Refused(why)) => {
            let why = why.to_string();
            assert!(why.contains("breakdown") && why.contains("Remove the line"), "{}", why);
        }
        other => panic!("not refused: {:?}", other),
    }
}

// ---- WB-PROMPT-53: a review or a fix of a change with no issue ----

#[test]
fn test_WB_PROMPT_53_a_review_of_a_change_with_no_issue_is_handed_out() {
    let folder = set_up();
    let said = ok(&run(&folder.path, &["review", "--change", "174", "--round", "1"]));
    assert!(said.contains("reviewer of change 174, made for issue none."), "{}", said);
    assert!(said.contains("The requirements are what the description of change 174 says the change does."), "{}", said);
    // The change is read as `wb` brings it in, by no address (#153).
    assert!(said.contains("/run/watchbill/in/change.text"), "{}", said);
    assert!(said.contains("on the branch feature-x."), "{}", said);
    assert!(!said.contains("{issue"), "{}", said);
}

#[test]
fn test_WB_PROMPT_53_the_stamp_of_a_review_or_a_fix_with_no_issue_has_an_empty_issue() {
    let folder = set_up();
    for role in ["review", "fix"] {
        let stamp = ok(&run(&folder.path, &[role, "--change", "174", "--round", "1", "--stamp"]));
        assert_eq!(field(&stamp, "role"), Some(role), "{}", stamp);
        assert_eq!(field(&stamp, "issue"), Some(""), "{}", stamp);
        assert!(stamp.lines().any(|line| line == "issue:"), "{}", stamp);
        assert_eq!(field(&stamp, "change"), Some("174"), "{}", stamp);
        assert_eq!(field(&stamp, "round"), Some("1"), "{}", stamp);
        assert_eq!(stamp.lines().filter(|line| line.starts_with("part: ")).count(), 1, "{}", stamp);
    }
}

#[test]
fn test_WB_PROMPT_53_a_fix_of_a_change_with_no_issue_is_handed_out() {
    let folder = set_up();
    let said = ok(&run(&folder.path, &["fix", "--change", "174", "--round", "1", "--branch", "feature-x"]));
    assert!(said.contains("fixer of change 174, made for issue none."), "{}", said);
    // The change is read as `wb` brings it in, by no address (#153).
    assert!(said.contains("/run/watchbill/in/change.text"), "{}", said);
    assert!(!said.contains("{issue"), "{}", said);
}

#[test]
fn test_WB_PROMPT_53_with_no_issue_an_option_for_the_requirements_wins() {
    let folder = set_up();
    let said = ok(&run(&folder.path, &["review", "--change", "174", "--round", "1", "--requirements", "WB-X-1"]));
    assert!(said.contains("The requirements are WB-X-1."), "{}", said);
    assert!(!said.contains("says the change does"), "{}", said);
}

// ---- WB-PROMPT-54: the branch of a change with no issue, and the tester and the build ----

#[test]
fn test_WB_PROMPT_54_with_no_issue_the_fix_takes_the_branch_given() {
    let folder = set_up();
    git(&folder.path, &["checkout", "-q", "main"]);
    let stamp = ok(&run(&folder.path, &["fix", "--change", "174", "--round", "1", "--branch", "feature-x", "--stamp"]));
    assert_eq!(field(&stamp, "branch"), Some("feature-x"), "{}", stamp);
}

#[test]
fn test_WB_PROMPT_54_with_no_issue_the_fix_takes_the_branch_checked_out_when_it_is_the_change_s() {
    let folder = set_up();
    let stamp = ok(&run(&folder.path, &["fix", "--change", "174", "--round", "1", "--stamp"]));
    assert_eq!(field(&stamp, "branch"), Some("feature-x"), "{}", stamp);
}

fn refused_for_the_branch(folder: &Path, case: &str) {
    for extra in [&[][..], &["--parts"], &["--stamp"]] {
        let mut args = vec!["fix", "--change", "174", "--round", "1"];
        args.extend(extra);
        let found = run(folder, &args);
        assert_eq!(code(&found), Some(2), "{}: {}", case, text(&found.stderr));
        assert_eq!(text(&found.stdout), "", "{}", case);
        let why = text(&found.stderr);
        assert!(why.contains("Give --branch"), "{}: {}", case, why);
        assert!(why.ends_with(USAGE), "{}: {}", case, why);
    }
}

#[test]
fn test_WB_PROMPT_54_with_no_issue_the_fix_is_refused_with_2_when_the_branch_checked_out_is_not_the_change_s() {
    // The branch that `origin/HEAD` names is where changes go, and no change's own.
    let folder = set_up();
    git(&folder.path, &["checkout", "-q", "main"]);
    refused_for_the_branch(&folder.path, "main");
    // No branch is checked out.
    git(&folder.path, &["checkout", "-q", "--detach", "feature-x"]);
    refused_for_the_branch(&folder.path, "detached");
    // Where `origin/HEAD` is not there, the branch checked out cannot be told from the one changes go into.
    git(&folder.path, &["checkout", "-q", "feature-x"]);
    git(&folder.path, &["symbolic-ref", "--delete", "refs/remotes/origin/HEAD"]);
    refused_for_the_branch(&folder.path, "no origin/HEAD");
}

#[test]
fn test_WB_PROMPT_54_the_tester_and_the_build_with_no_issue_end_with_2_and_the_usage() {
    let folder = set_up();
    for role in ["tester", "build"] {
        for extra in [&[][..], &["--change", "174"], &["--change", "174", "--round", "1", "--branch", "feature-x"], &["--stamp"], &["--parts"]] {
            let mut args = vec![role];
            args.extend(extra);
            let found = run(&folder.path, &args);
            assert_eq!(code(&found), Some(2), "{} {:?}: {}", role, extra, text(&found.stderr));
            assert_eq!(text(&found.stdout), "", "{} {:?}", role, extra);
            let why = text(&found.stderr);
            assert!(why.starts_with("wb prompt: ") && why.ends_with(USAGE), "{} {:?}: {}", role, extra, why);
        }
    }
}
