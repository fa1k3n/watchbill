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
    assert!(text.contains("a job on the job's ref"), "the files of a job on the ref are not placed");
    assert!(text.contains("begun before"), "the tracker is not left to a job begun before");
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
    assert!(at < readme.find("\n## Setting it up in a repository\n").unwrap(), "the section is not near the top");
}
