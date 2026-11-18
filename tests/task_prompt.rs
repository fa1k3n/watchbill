//! `wb prompt` for a task that has a file (#76), run as a person runs it, in a repository made for the test.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use watchbill::manifest::hash_bytes;

const GOOD: &str = "task: '54'
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
changes: [src/task.rs, tests/task.rs]
reads:
  tester: [docs/tester-only.md]
  build: [docs/build-only.md]
  review: [docs/review-only.md]
  fix: [docs/fix-only.md]
existing_tests_changed: []
questions: []
";

const ROLES: [&str; 4] = ["tester", "build", "review", "fix"];

/// The options a role needs beside the task: where the work is, and for the review and the fix a change and a round.
const REST: [&str; 10] =
    ["--branch", "task-54", "--task_address", "tracker 54", "--change", "34", "--round", "1", "--change_address", "server 34"];

/// A repository set up by `wb init --yes`, with a git identity of its own.
fn set_up() -> Folder {
    let folder = repository(None);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    folder
}

/// A repository with the file of task 54, accepted.
fn accepted_with(file: &str) -> Folder {
    let folder = set_up();
    put(&folder.path, ".watchbill/tasks/54.yml", file);
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
fn test_WB_PROMPT_25_an_accepted_file_fills_the_requirements_with_ids_and_texts_in_every_role() {
    let folder = accepted_with(GOOD);
    // The text of the fix does not hold `{requirements}`, so there is nothing in it to fill.
    for role in ["tester", "build", "review"] {
        let said = ok(&prompt(&folder.path, role, &[]));
        assert!(said.contains("54.1: The tool reads the file."), "{}: {}", role, said);
        assert!(said.contains("54.2: The tool names a fault."), "{}: {}", role, said);
        assert!(!said.contains("{requirements}"), "{}", role);
        assert!(!said.contains("the numbered requirements of task"), "{}", role);
    }
}

#[test]
fn test_WB_PROMPT_26_the_interface_and_the_lists_come_after_the_role_and_before_the_project_text() {
    let folder = accepted_with(GOOD);
    put(&folder.path, ".watchbill/project/all.md", "PROJECT-ALL\n");
    for role in ROLES {
        let said = ok(&prompt(&folder.path, role, &[]));
        let interface = said.find("fn read(path: &str) -> String").unwrap_or_else(|| panic!("{}: no interface", role));
        let changes = said.find("src/task.rs").unwrap_or_else(|| panic!("{}: no changes", role));
        assert!(said.contains("tests/task.rs"), "{}", role);
        let fixed = said.find(watchbill::prompt::FIXED_LINE).unwrap();
        let project = said.find("PROJECT-ALL").unwrap();
        assert!(interface < changes, "{}", role);
        assert!(changes < fixed && fixed < project, "{}: the project text comes last", role);
        // The role's own text comes first: it says the task before the interface is given.
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
fn test_WB_PROMPT_30_parts_and_stamp_name_the_task_file_and_its_hash_after_the_role() {
    let folder = accepted_with(GOOD);
    let line = format!("{}  .watchbill/tasks/54.yml", hash_bytes(GOOD.as_bytes()));
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
    put(&folder.path, ".watchbill/tasks/54.yml", GOOD);
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
    put(&folder.path, ".watchbill/tasks/54.yml", &GOOD.replace("reads the file", "reads a file"));
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
    put(&folder.path, ".watchbill/tasks/54.yml", &broken);
    let body = format!("sha256: {}\nby: x <y>\ntime: 2026-11-18T10:00:00Z\n", hash_bytes(broken.as_bytes()));
    put(&folder.path, ".watchbill/tasks/54.accepted", &body);
    let run = prompt(&folder.path, "build", &[]);
    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("intreface"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_40_a_path_with_a_control_character_or_a_line_or_bidi_mark_is_refused_by_name() {
    for path in ["\\e[31m/abs", "a\\u2028b", "a\\u2029b", "a\\u202Eb"] {
        let file = GOOD.replace("changes: [src/task.rs, tests/task.rs]", &format!("changes: [\"{}\"]", path));
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
fn test_WB_PROMPT_28_with_no_file_the_prompt_is_as_before_and_so_is_one_for_another_task() {
    let folder = set_up();
    let none = ok(&prompt(&folder.path, "build", &[]));
    assert!(none.contains("the numbered requirements of task 54"), "{}", none);
    assert!(!none.contains("fn read"), "{}", none);
    let parts = ok(&prompt(&folder.path, "build", &["--parts"]));
    assert_eq!(parts.lines().count(), 1);
    // A file for task 54 does not touch the prompt of task 55.
    put(&folder.path, ".watchbill/tasks/54.yml", GOOD);
    let mut args = vec!["prompt", "build", "55"];
    args.extend(REST);
    let other = wb(&folder.path, &args, "");
    assert!(ok(&other).contains("the numbered requirements of task 55"));
}
