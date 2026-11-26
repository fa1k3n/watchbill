//! The role prompts of the revision say the stamp and the questions are on the job's ref, and the
//! README says what signing needs before a start (#338).

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

use std::fs;
use std::path::Path;

const ROLES: [&str; 5] = ["planner", "build", "tester", "review", "fix"];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn role(name: &str) -> String {
    fs::read_to_string(root().join("share/watchbill/revision/roles").join(format!("{}.md", name))).unwrap()
}

#[test]
fn test_WB_PROMPT_82_no_prompt_says_the_stamp_is_a_comment() {
    for name in ROLES {
        let text = role(name);
        assert!(!text.contains("comment whose first line starts with `[stamp]"), "{}: the stamp is called a comment", name);
        assert!(!text.contains("stamp on the issue"), "{}: the stamp is called one on the issue", name);
        assert!(text.contains("`refs/watchbill/"), "{}: the stamp is not placed on the job's ref", name);
        assert!(text.contains("signed commit"), "{}: the stamp is not called a signed commit", name);
    }
}

#[test]
fn test_WB_PROMPT_83_the_planner_reads_the_issue_and_questions_from_the_ref_for_a_job_on_the_ref() {
    let text = role("planner");
    assert!(!text.contains("read both from the tracker before your session"), "the files are said to come from the tracker alone");
    assert!(!text.contains("for a job on the job's ref"), "the awkward phrase is back");
    assert!(!text.contains("only for a job begun before"), "the first start of a job is left out");
    assert!(text.contains("the questions and the answers from the commits of the job's ref"), "the questions are not placed on the ref");
    assert!(text.contains("the plan's sources from its ref"), "the source of a plan's text is not said");
    assert!(text.contains("until the job's ref holds its first commit (a job begun before, or its first start)"), "the tracker is not limited to the time before the first commit");
}

#[test]
fn test_WB_PROMPT_83_the_answer_line_says_wb_answer_writes_a_commit_with_a_copy_posted() {
    let text = role("planner");
    let line = text.lines().find(|l| l.starts_with("- A person answers with `wb answer`")).expect("no answer line");
    assert!(line.contains("commit on the job's ref"), "{}", line);
    assert!(line.contains("copy"), "{}", line);
    assert!(!line.contains("Until the program reads from the tracker who wrote a comment"), "{}", line);
}

#[test]
fn test_WB_PROMPT_83_every_role_says_wb_answer_writes_a_commit_and_the_questions_come_from_the_ref() {
    for name in ROLES {
        let text = role(name);
        let line = text.lines().find(|l| l.starts_with("- A person answers with `wb answer`")).expect("no answer line");
        assert!(line.contains("commit on the job's ref") && line.contains("copy"), "{}: {}", name, line);
        assert!(!text.contains("The program read them from the tracker before your session"), "{}", name);
        // The conditions of an answer (#338).
        assert!(line.contains("For a job on the ref, an answer counts only as a commit of `wb answer`, signed with the role `answer`"), "{}: {}", name, line);
        assert!(line.contains("For a job begun before, and for a change with no issue, an answer counts only from a person"), "{}: {}", name, line);
        assert!(line.contains("a comment that starts with `[question]` or `[questions]` is never an answer"), "{}: {}", name, line);
    }
}

#[test]
fn test_WB_PROMPT_83_each_role_says_the_questions_come_from_the_commits_of_the_ref_and_are_recorded_there() {
    for name in ROLES {
        let text = role(name);
        assert!(text.contains("from the commits of the job's ref"), "{}: the questions are not said to come from the ref", name);
        assert!(!text.contains("posts the new questions on the issue"), "{}", name);
        assert!(!text.contains("are also posted on the issue, in one comment for the session"), "{}", name);
        assert!(!text.contains("which the program posts after your session"), "{}: asked says posts", name);
        assert!(text.contains("which the program records after your session"), "{}", name);
    }
}

#[test]
fn test_WB_PROMPT_82_the_review_and_the_fix_say_a_change_with_no_issue_has_its_stamp_as_a_comment() {
    for name in ["review", "fix"] {
        assert!(role(name).contains("or, for a change with no issue, a comment on the change"), "{}", name);
    }
}

#[test]
fn test_WB_PROMPT_84_the_readme_has_a_section_before_you_start_on_signing_that_names_the_two_settings() {
    let readme = fs::read_to_string(root().join("README.md")).unwrap();
    let at = readme.find("\n## Before you start: signing\n").expect("no section 'Before you start: signing'");
    let end = readme[at + 1..].find("\n## ").map(|n| at + 1 + n).unwrap_or(readme.len());
    let section = &readme[at..end];
    assert!(section.contains("`gpg.format`") && section.contains("`user.signingkey`"), "the two settings are not named");
    assert!(section.contains(".watchbill/signers"), "the list is not named");
    assert!(section.contains("no start, acceptance, answer, plan commit or revert of an issue"), "the list of what needs signing is not said");
    assert!(at < readme.find("\n## Setting it up in a repository\n").unwrap(), "the section is not near the top");
}
