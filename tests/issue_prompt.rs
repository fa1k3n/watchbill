//! `wb prompt` for an issue that has a file (#76), run as a person runs it, in a repository made for the test.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use watchbill::manifest::hash_bytes;

const GOOD: &str = "issue: '54'
from: 3f0c1d9e
requirements:
  - id: '54.1'
    text: The tool reads the file.
    source: the goal
  - id: '54.2'
    text: The tool names a fault.
    source: the tests
    guard: test_WB_X_1_holds
interface: |
  fn read(path: &str) -> String
changes: [src/issue.rs, tests/issue.rs]
reads:
  tester: [docs/tester-only.md]
  build: [docs/build-only.md]
  review: [docs/review-only.md]
  fix: [docs/fix-only.md]
existing_tests_changed: []
questions: []
";

const ROLES: [&str; 4] = ["tester", "build", "review", "fix"];

/// The options a role needs beside the issue: where the work is, and for the review and the fix a change and a round.
const REST: [&str; 10] =
    ["--branch", "issue-54", "--issue_address", "tracker 54", "--change", "34", "--round", "1", "--change_address", "server 34"];

/// A repository set up by `wb init --yes`, with a git identity of its own.
fn set_up() -> Folder {
    let folder = repository(None);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    commit_config(&folder.path);
    folder
}

/// A repository with the file of issue 54, accepted.
fn accepted_with(file: &str) -> Folder {
    let folder = set_up();
    put(&folder.path, ".watchbill/issues/54.yml", file);
    let run = wb(&folder.path, &["plan", "accept", "54", "--no-commit"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    // The file and its acceptance are committed, as no role is handed out without (#97).
    git(&folder.path, &["add", "--", ".watchbill/issues/54.yml", ".watchbill/issues/54.accepted"]);
    git(&folder.path, &["-c", "commit.gpgsign=false", "commit", "-q", "-m", "x"]);
    folder
}

fn prompt(folder: &Path, role: &str, extra: &[&str]) -> Output {
    let mut args = vec!["prompt", role, "54"];
    args.extend(REST);
    args.extend(extra);
    wb(folder, &args, "")
}

fn ok(run: &Output) -> String {
    assert_eq!(code(run), Some(0), "{}", text(&run.stderr));
    text(&run.stdout)
}

fn snapshot(folder: &Path) -> Vec<(String, Vec<u8>)> {
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

/// Whether `sha256sum -c` passes on the lines, run at the root of the repository.
fn sha256sum_passes(folder: &Path, lines: &str) -> bool {
    let mut child = Command::new("sha256sum")
        .args(["-c", "-"])
        .current_dir(folder)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("sha256sum could not be started");
    child.stdin.take().unwrap().write_all(lines.as_bytes()).unwrap();
    child.wait().unwrap().success()
}

#[test]
fn test_WB_PROMPT_25_an_accepted_file_fills_the_requirements_with_ids_and_texts_in_the_tester_build_and_review() {
    let folder = accepted_with(GOOD);
    // The text of the fix does not hold `{requirements}`, so there is nothing in it to fill.
    for role in ["tester", "build", "review"] {
        let said = ok(&prompt(&folder.path, role, &[]));
        assert!(said.contains("54.1: The tool reads the file."), "{}: {}", role, said);
        assert!(said.contains("54.2: The tool names a fault."), "{}: {}", role, said);
        assert!(!said.contains("{requirements}"), "{}", role);
        assert!(!said.contains("the numbered requirements of issue"), "{}", role);
    }
}

#[test]
fn test_WB_PROMPT_26_the_interface_and_the_lists_come_after_the_role_and_before_the_project_text() {
    let folder = accepted_with(GOOD);
    put_committed(&folder.path, ".watchbill/project/all.md", "PROJECT-ALL\n");
    for role in ROLES {
        let said = ok(&prompt(&folder.path, role, &[]));
        let interface = said.find("fn read(path: &str) -> String").unwrap_or_else(|| panic!("{}: no interface", role));
        let changes = said.find("src/issue.rs").unwrap_or_else(|| panic!("{}: no changes", role));
        assert!(said.contains("tests/issue.rs"), "{}", role);
        let fixed = said.find(watchbill::prompt::FIXED_LINE).unwrap();
        let project = said.find("PROJECT-ALL").unwrap();
        assert!(interface < changes, "{}", role);
        assert!(changes < fixed && fixed < project, "{}: the project text comes last", role);
        // The role's own text comes first: it says the issue before the interface is given.
        assert!(said.find("54").unwrap() < interface, "{}", role);
    }
}

#[test]
fn test_WB_PROMPT_26_each_role_is_given_its_own_list_of_what_it_may_read_and_no_other() {
    let folder = accepted_with(GOOD);
    for role in ROLES {
        let said = ok(&prompt(&folder.path, role, &[]));
        for other in ROLES {
            let path = format!("docs/{}-only.md", other);
            assert_eq!(said.contains(&path), other == role, "{} and the list of {}", role, other);
        }
    }
}

#[test]
fn test_WB_PROMPT_30_parts_and_stamp_name_the_issue_file_and_its_hash_after_the_role() {
    let folder = accepted_with(GOOD);
    let line = format!("{}  .watchbill/issues/54.yml", hash_bytes(GOOD.as_bytes()));
    let parts = ok(&prompt(&folder.path, "build", &["--parts"]));
    let lines: Vec<&str> = parts.lines().collect();
    assert_eq!(lines.len(), 2, "{}", parts);
    assert!(lines[0].ends_with("  .watchbill/revision/roles/build.md"), "{}", parts);
    assert_eq!(lines[1], line);
    assert!(sha256sum_passes(&folder.path, &parts), "sha256sum -c failed");
    let stamp = ok(&prompt(&folder.path, "build", &["--stamp"]));
    assert!(stamp.contains(&format!("part: {}\n", line)), "{}", stamp);
}

#[test]
fn test_WB_PROMPT_41_an_option_for_the_requirements_wins_and_the_stamp_says_so() {
    let folder = accepted_with(GOOD);
    let said = ok(&prompt(&folder.path, "build", &["--requirements", "ONLY-THIS"]));
    assert!(said.contains("ONLY-THIS"));
    assert!(!said.contains("The tool reads the file."));
    // The interface and the lists are still handed out.
    assert!(said.contains("fn read(path: &str) -> String"));
    let stamp = ok(&prompt(&folder.path, "build", &["--requirements", "ONLY-THIS", "--stamp"]));
    assert!(stamp.contains("\nrequirements: given\n"), "{}", stamp);
    let plain = ok(&prompt(&folder.path, "build", &["--stamp"]));
    assert!(!plain.contains("requirements:"), "{}", plain);
}

#[test]
fn test_WB_PROMPT_27_a_file_that_is_not_accepted_hands_out_nothing_and_ends_with_1() {
    let folder = set_up();
    put(&folder.path, ".watchbill/issues/54.yml", GOOD);
    let before = snapshot(&folder.path);
    for role in ROLES {
        for extra in [&[][..], &["--parts"], &["--stamp"]] {
            let run = prompt(&folder.path, role, extra);
            assert_eq!(code(&run), Some(1), "{} {:?}", role, extra);
            assert_eq!(text(&run.stdout), "", "{} {:?}", role, extra);
            let why = text(&run.stderr);
            assert!(why.contains("not accepted"), "{}", why);
            assert!(why.contains("wb accept 54"), "{}", why);
        }
    }
    assert_eq!(snapshot(&folder.path), before);
}

#[test]
fn test_WB_PROMPT_27_a_file_changed_after_its_acceptance_hands_out_nothing_and_ends_with_1() {
    let folder = accepted_with(GOOD);
    put(&folder.path, ".watchbill/issues/54.yml", &GOOD.replace("reads the file", "reads a file"));
    for role in ROLES {
        let run = prompt(&folder.path, role, &[]);
        assert_eq!(code(&run), Some(1), "{}", role);
        assert_eq!(text(&run.stdout), "", "{}", role);
        let why = text(&run.stderr);
        assert!(why.contains("not accepted") && why.contains("changed"), "{}", why);
    }
}

#[test]
fn test_WB_PROMPT_27_a_file_with_a_fault_of_form_and_a_hand_written_acceptance_is_refused() {
    let folder = set_up();
    let broken = GOOD.replace("interface:", "intreface:");
    put(&folder.path, ".watchbill/issues/54.yml", &broken);
    let body = format!("sha256: {}\nby: x <y>\ntime: 2026-11-18T10:00:00Z\n", hash_bytes(broken.as_bytes()));
    put(&folder.path, ".watchbill/issues/54.accepted", &body);
    let run = prompt(&folder.path, "build", &[]);
    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("intreface"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_40_a_path_with_a_control_character_or_a_line_or_bidi_mark_is_refused_by_name() {
    for path in ["\\e[31m/abs", "a\\u2028b", "a\\u2029b", "a\\u202Eb"] {
        let file = GOOD.replace("changes: [src/issue.rs, tests/issue.rs]", &format!("changes: [\"{}\"]", path));
        let folder = accepted_with(&file);
        let run = prompt(&folder.path, "build", &[]);
        assert_eq!(code(&run), Some(1), "{}: {}", path, text(&run.stderr));
        assert_eq!(text(&run.stdout), "", "{}", path);
        let why = text(&run.stderr);
        assert!(why.contains("changes") && why.contains("control"), "{}: {}", path, why);
        // The refusal does not carry the character itself.
        for bad in ['\u{1b}', '\u{2028}', '\u{2029}', '\u{202e}'] {
            assert!(!why.contains(bad), "{}", why);
        }
    }
}

#[test]
fn test_WB_TASK_17_with_no_file_the_prompt_is_handed_out_and_a_file_of_another_issue_is_not_used() {
    // Was test_WB_TASK_14_with_no_file_nothing_is_handed_out_and_a_file_of_another_issue_does_not_help, and
    // before that test_WB_PROMPT_28. Since #116 an issue with no file is handed out again.
    let folder = set_up();
    let said = ok(&prompt(&folder.path, "build", &[]));
    assert!(said.contains("the numbered requirements of issue 54"), "{}", said);
    assert!(!said.contains("## The interface"), "{}", said);
    // A file for issue 54 is no part of the prompt of issue 55.
    let other = accepted_with(GOOD);
    let mut args = vec!["prompt", "build", "55"];
    args.extend(REST);
    let said = ok(&wb(&other.path, &args, ""));
    assert!(said.contains("the numbered requirements of issue 55"), "{}", said);
    assert!(!said.contains("fn read(path: &str) -> String") && !said.contains("54.1"), "{}", said);
}

#[test]
fn test_WB_PROMPT_25_a_file_with_no_requirement_says_so_where_the_requirements_stand() {
    let folder = accepted_with(&GOOD.replace(
        "requirements:\n  - id: '54.1'\n    text: The tool reads the file.\n    source: the goal\n  - id: '54.2'\n    text: The tool names a fault.\n    source: the tests\n    guard: test_WB_X_1_holds\n",
        "requirements: []\n",
    ));
    let said = ok(&prompt(&folder.path, "build", &[]));
    assert!(said.contains("the issue's file holds no requirement"), "{}", said);
    assert!(!said.contains("{requirements}"));
}

#[test]
fn test_WB_PROMPT_42_the_fix_is_given_the_interface_and_the_lists_and_no_requirement() {
    let folder = accepted_with(GOOD);
    let said = ok(&prompt(&folder.path, "fix", &[]));
    assert!(said.contains("fn read(path: &str) -> String"), "{}", said);
    assert!(said.contains("src/issue.rs") && said.contains("docs/fix-only.md"), "{}", said);
    assert!(!said.contains("The tool reads the file."), "{}", said);
}

/// A file with one place of GOOD replaced, accepted, and the prompt of the build refused: the name of the
/// place is on standard error, the character is not, and nothing is on standard output.
fn refused_at(from: &str, to: &str, field: &str) {
    assert!(GOOD.contains(from), "{}", from);
    let folder = accepted_with(&GOOD.replace(from, to));
    let run = prompt(&folder.path, "build", &[]);
    assert_eq!(code(&run), Some(1), "{}: {}", field, text(&run.stderr));
    assert_eq!(text(&run.stdout), "", "{}", field);
    let why = text(&run.stderr);
    assert!(why.contains(field), "{}: {}", field, why);
    assert!(why.contains("control"), "{}", why);
    assert!(!why.chars().any(|c| c.is_control() && c != '\n'), "{:?}", why);
    for bad in ['\u{2028}', '\u{2029}', '\u{202a}', '\u{202e}', '\u{2066}', '\u{2069}'] {
        assert!(!why.contains(bad), "{:?}", why);
    }
}

#[test]
fn test_WB_PROMPT_43_a_control_or_bidi_character_in_any_value_that_reaches_a_prompt_is_refused_by_name() {
    for bad in ["\\e[31m", "\\u2028", "\\u2029", "\\u202A", "\\u202D", "\\u202E", "\\u2066", "\\u2069", "\\r"] {
        refused_at("interface: |\n  fn read(path: &str) -> String\n", &format!("interface: \"fn x(){}\"\n", bad), "interface");
        refused_at("text: The tool reads the file.", &format!("text: \"reads{}\"", bad), "requirements");
        refused_at("id: '54.1'", &format!("id: \"54.{}1\"", bad), "requirements");
        refused_at("tester: [docs/tester-only.md]", &format!("tester: [\"a{}b\"]", bad), "reads.tester");
    }
}

#[test]
fn test_WB_PROMPT_43_a_line_break_in_a_requirement_is_refused_and_one_in_the_interface_is_not() {
    refused_at("text: The tool reads the file.", "text: \"reads\\nthe file\"", "requirements");
    let folder = accepted_with(&GOOD.replace("interface: |\n  fn read(path: &str) -> String\n", "interface: \"fn a()\\tb\\nfn c()\\n\"\n"));
    let said = ok(&prompt(&folder.path, "build", &[]));
    assert!(said.contains("fn a()\tb\nfn c()"), "{}", said);
}

#[test]
fn test_WB_PROMPT_43_the_bidi_control_is_called_by_what_it_is() {
    let file = GOOD.replace("changes: [src/issue.rs, tests/issue.rs]", "changes: [\"a\\u202Eb\"]");
    let folder = accepted_with(&file);
    let run = prompt(&folder.path, "build", &[]);
    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stderr).contains("bidirectional"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_44_an_issue_file_over_one_mebibyte_is_refused_with_a_message() {
    let folder = set_up();
    let mut big = GOOD.to_string();
    big.push_str(&format!("# {}\n", "x".repeat(1024 * 1024)));
    put(&folder.path, ".watchbill/issues/54.yml", &big);
    let before = snapshot(&folder.path);
    for extra in [&[][..], &["--parts"]] {
        let run = prompt(&folder.path, "build", extra);
        assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
        assert_eq!(text(&run.stdout), "");
        assert!(text(&run.stderr).contains("1 MiB"), "{}", text(&run.stderr));
    }
    assert_eq!(snapshot(&folder.path), before);
}
