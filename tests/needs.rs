//! `wb accept` checks the lists of an issue's file (#125): each file the tester and the build see has every
//! file it needs to build in that role's view, by the project's own command `needs`, and no path of
//! `changes` is a file of the build, by `build_files`. Run as a person runs it, in a repository made for the test.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::io::Write;
use std::path::Path;

const ISSUE: &str = "issue: '54'
from: 3f0c1d9e
requirements:
  - id: '54.1'
    text: The brake holds.
    source: the goal
interface: |
  int brake_hold(void);
changes: [src/brake.c, tests/test_brake.c]
reads:
  tester: [include/brake.h]
  build: [include/brake.h]
  review: [docs/review.md]
  fix: []
existing_tests_changed: []
questions: []
";

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

/// Adds lines to `watchbill.yml`.
fn add(folder: &Folder, lines: &str) {
    let mut file = fs::OpenOptions::new().append(true).open(config(&folder.path)).unwrap();
    file.write_all(lines.as_bytes()).unwrap();
    commit_config(&folder.path);
}

/// Gives `watchbill.yml` the pattern `needs`, written in single quotes.
fn needs(folder: &Folder, command: &str) {
    add(folder, &format!("needs: '{}'\n", command.replace('\'', "''")));
}

fn with_issue(folder: &Folder, issue_file: &str) {
    put(&folder.path, ".watchbill/issues/54.yml", issue_file);
}

fn accepted(folder: &Folder) -> std::path::PathBuf {
    folder.path.join(".watchbill/issues/54.accepted")
}

/// Every plain file under a folder, with its bytes, sorted by path.
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

/// `wb accept 54` refused: status 1, nothing written. What it said on standard error is the result.
fn refused(folder: &Folder) -> String {
    let before = snapshot(&folder.path);
    let run = wb(&folder.path, &["plan", "accept", "54", "--no-commit"], "");
    assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
    assert_eq!(snapshot(&folder.path), before, "something was written");
    assert!(!accepted(folder).exists());
    text(&run.stderr)
}

/// `wb accept 54` taken: status 0, and the acceptance written. What it said on standard output is the result.
fn taken(folder: &Folder) -> String {
    let run = wb(&folder.path, &["plan", "accept", "54", "--no-commit"], "");
    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    assert!(accepted(folder).is_file());
    text(&run.stdout)
}

/// `wb accept 54` taken, with `needs`: it says that the lists were checked.
fn checked(folder: &Folder) {
    let out = taken(folder);
    assert!(out.contains("were checked by `needs`"), "{}", out);
    assert!(!out.contains("not checked"), "{}", out);
}

/// The lines of a refusal that name a missing path.
fn misses(said: &str) -> Vec<String> {
    said.lines().filter(|line| line.starts_with("  - ")).map(String::from).collect()
}

#[test]
fn test_WB_TASK_19_a_path_missing_from_the_build_s_reads_is_refused_with_the_role_the_file_and_the_path() {
    let folder = set_up();
    with_issue(&folder, &ISSUE.replace("tester: [include/brake.h]", "tester: [include/brake.h, include/regs.h]"));
    needs(&folder, "case {file} in src/brake.c) echo include/brake.h; echo include/regs.h;; esac");

    let err = refused(&folder);

    let lines = misses(&err);
    assert_eq!(lines.len(), 1, "{}", err);
    for what in ["build", "src/brake.c", "include/regs.h"] {
        assert!(lines[0].contains(what), "{} is not named: {}", what, err);
    }
    assert!(!lines[0].contains("tester"), "{}", err);
}

#[test]
fn test_WB_TASK_19_a_path_missing_from_both_lists_is_named_for_each_role() {
    let folder = set_up();
    with_issue(&folder, ISSUE);
    needs(&folder, "case {file} in src/brake.c) echo include/regs.h;; esac");

    let err = refused(&folder);

    let lines = misses(&err);
    assert_eq!(lines.len(), 2, "{}", err);
    assert!(lines.iter().any(|line| line.contains("tester") && line.contains("src/brake.c") && line.contains("include/regs.h")), "{}", err);
    assert!(lines.iter().any(|line| line.contains("build") && line.contains("src/brake.c") && line.contains("include/regs.h")), "{}", err);
}

#[test]
fn test_WB_TASK_19_a_path_missing_from_the_tester_s_reads_is_refused_for_the_tester_alone() {
    let folder = set_up();
    with_issue(&folder, &ISSUE.replace("tester: [include/brake.h]", "tester: [include/brake.h, tests/fixture.c]"));
    needs(&folder, "case {file} in tests/fixture.c) echo tests/helpers.h;; esac");

    let err = refused(&folder);

    let lines = misses(&err);
    assert_eq!(lines.len(), 1, "{}", err);
    for what in ["tester", "tests/fixture.c", "tests/helpers.h"] {
        assert!(lines[0].contains(what), "{} is not named: {}", what, err);
    }
}

#[test]
fn test_WB_TASK_19_the_same_path_in_changes_is_accepted() {
    let folder = set_up();
    with_issue(&folder, &ISSUE.replace("changes: [src/brake.c, tests/test_brake.c]", "changes: [src/brake.c, tests/test_brake.c, include/regs.h]"));
    needs(&folder, "case {file} in src/brake.c) echo include/brake.h; echo include/regs.h;; esac");

    checked(&folder);
}

#[test]
fn test_WB_TASK_19_the_same_path_in_the_role_s_reads_is_accepted() {
    let folder = set_up();
    let file = ISSUE
        .replace("tester: [include/brake.h]", "tester: [include/brake.h, include/regs.h]")
        .replace("build: [include/brake.h]", "build: [include/brake.h, include/regs.h]");
    with_issue(&folder, &file);
    needs(&folder, "case {file} in src/brake.c) echo include/brake.h; echo include/regs.h;; esac");

    checked(&folder);
}

#[test]
fn test_WB_TASK_19_a_path_under_a_folder_of_the_list_is_in_it() {
    let folder = set_up();
    let file = ISSUE.replace("tester: [include/brake.h]", "tester: [include/]").replace("build: [include/brake.h]", "build: [include]");
    with_issue(&folder, &file);
    needs(&folder, "case {file} in src/brake.c) echo include/regs.h;; esac");

    checked(&folder);
}

#[test]
fn test_WB_TASK_19_a_printed_path_is_read_from_the_root_with_dot_and_dot_dot_worked_out() {
    let folder = set_up();
    with_issue(&folder, ISSUE);
    // As `cc -MM` prints a header that a file includes as `../include/brake.h`.
    needs(&folder, "case {file} in src/brake.c) echo ./src/../include/brake.h; echo src//brake.c;; esac");

    checked(&folder);
}

#[test]
fn test_WB_TASK_19_the_reads_of_the_review_and_the_fix_are_not_run() {
    let folder = set_up();
    with_issue(&folder, &ISSUE.replace("fix: []", "fix: [docs/fix.md]"));
    needs(&folder, "case {file} in docs/review.md|docs/fix.md) echo secret/key.h;; esac");

    checked(&folder);
}

#[test]
fn test_WB_TASK_19_the_file_fills_the_pattern_as_one_word_of_the_shell() {
    let folder = set_up();
    // Unquoted, the first would be split in two and the second would run `echo include`.
    let file = ISSUE.replace("changes: [src/brake.c, tests/test_brake.c]", "changes: ['src/it''s a brake.c', 'src/$(echo include).c']");
    with_issue(&folder, &file);
    needs(&folder, "printf \"%s\\n\" {file}");

    checked(&folder);
}

#[test]
fn test_WB_TASK_19_needs_is_run_at_the_root_of_the_checkout() {
    let folder = set_up();
    with_issue(&folder, ISSUE);
    put(&folder.path, "src/brake.c", "");
    needs(&folder, "test -f .watchbill/watchbill.yml || exit 5");

    let run = wb(&folder.path.join("src"), &["plan", "accept", "54", "--no-commit"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    assert!(text(&run.stdout).contains("were checked by `needs`"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_TASK_19_needs_that_is_not_a_text_is_refused_through_the_loader() {
    let folder = set_up();
    with_issue(&folder, ISSUE);
    add(&folder, "needs: [cc, -MM]\n");

    let err = refused(&folder);

    assert!(err.contains("needs"), "{}", err);
}

#[test]
fn test_WB_TASK_20_a_command_that_ends_with_1_is_refused_and_names_the_file() {
    let folder = set_up();
    with_issue(&folder, ISSUE);
    needs(&folder, "case {file} in tests/test_brake.c) exit 1;; esac");

    let err = refused(&folder);

    assert!(err.contains("tests/test_brake.c"), "{}", err);
    assert!(err.contains("needs"), "{}", err);
}

#[test]
fn test_WB_TASK_21_with_no_needs_it_is_accepted_and_says_the_lists_were_not_checked() {
    let folder = set_up();
    with_issue(&folder, ISSUE);

    let out = taken(&folder);

    assert!(out.contains("`needs`") && out.contains("not checked"), "{}", out);
}

#[test]
fn test_WB_TASK_19_with_needs_it_says_the_lists_were_checked() {
    let folder = set_up();
    with_issue(&folder, ISSUE);
    needs(&folder, "true");

    checked(&folder);
}

#[test]
fn test_WB_TASK_22_a_path_of_changes_under_build_files_is_refused_and_named() {
    let folder = set_up();
    with_issue(&folder, &ISSUE.replace("changes: [src/brake.c, tests/test_brake.c]", "changes: [src/brake.c, Makefile]"));
    add(&folder, "build_files: [Makefile, ci]\n");

    let err = refused(&folder);

    assert!(err.contains("Makefile"), "{}", err);
    assert!(err.contains("build_files"), "{}", err);
}

#[test]
fn test_WB_TASK_22_a_path_in_a_folder_of_build_files_is_refused_and_named() {
    let folder = set_up();
    with_issue(&folder, &ISSUE.replace("changes: [src/brake.c, tests/test_brake.c]", "changes: [src/brake.c, ci/build.sh]"));
    add(&folder, "build_files: [Makefile, ci/]\n");

    let err = refused(&folder);

    assert!(err.contains("ci/build.sh"), "{}", err);
}

#[test]
fn test_WB_TASK_22_a_folder_of_changes_that_holds_a_file_of_the_build_is_refused_and_named() {
    let folder = set_up();
    with_issue(&folder, &ISSUE.replace("changes: [src/brake.c, tests/test_brake.c]", "changes: [src/brake.c, tools]"));
    add(&folder, "build_files: [tools/build.sh]\n");

    let err = refused(&folder);

    assert!(err.contains("tools"), "{}", err);
}

#[test]
fn test_WB_TASK_22_a_path_beside_build_files_is_accepted() {
    let folder = set_up();
    with_issue(&folder, &ISSUE.replace("changes: [src/brake.c, tests/test_brake.c]", "changes: [src/brake.c, Makefile.inc, ci2/build.sh]"));
    add(&folder, "build_files: [Makefile, ci]\n");

    let out = taken(&folder);

    assert!(out.contains("no path of `changes` is under `build_files`"), "{}", out);
}

#[test]
fn test_WB_TASK_22_the_ci_file_wb_init_found_is_refused_in_changes() {
    let folder = repository(None);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    put(&folder.path, ".gitlab-ci.yml", "");
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    commit_config(&folder.path);
    with_issue(&folder, &ISSUE.replace("changes: [src/brake.c, tests/test_brake.c]", "changes: [src/brake.c, .gitlab-ci.yml]"));

    let err = refused(&folder);

    assert!(err.contains(".gitlab-ci.yml"), "{}", err);
}
