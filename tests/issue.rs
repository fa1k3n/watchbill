//! The issue's file and `wb accept <issue>`, run as a person runs it, in a repository made for the test.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use watchbill::manifest;
use watchbill::issue::is_accepted;

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
  tester: [src/lib.rs]
  build: []
  review: [README.md]
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

fn with_good(folder: &Folder) {
    put(&folder.path, ".watchbill/issues/54.yml", GOOD);
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

/// A file refused: status 1, the fault named on standard error, and nothing written.
fn refused(folder: &Folder, issue_file: &str) -> String {
    put(&folder.path, ".watchbill/issues/54.yml", issue_file);
    let before = snapshot(&folder.path);
    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");
    assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
    assert_eq!(snapshot(&folder.path), before, "something was written");
    assert!(!accepted(folder).exists());
    text(&run.stderr)
}

/// `wb accept` with no git identity: the folder has none, and neither has the home of the user.
fn wb_without_identity(folder: &Path, args: &[&str]) -> Output {
    let home = folder.join("no-home");
    fs::create_dir_all(&home).unwrap();
    Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(args)
        .current_dir(folder)
        .env_clear()
        .envs(env())
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", &home)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .stdin(Stdio::null())
        .output()
        .expect("wb could not be started")
}

#[test]
fn test_WB_TASK_1_a_good_file_is_accepted_and_the_acceptance_is_written() {
    let folder = set_up();
    with_good(&folder);

    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(accepted(&folder).is_file());
}

#[test]
fn test_WB_TASK_1_an_issue_that_has_no_file_ends_with_1() {
    let folder = set_up();

    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    assert!(text(&run.stderr).contains("54.yml"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_TASK_1_a_value_written_plain_that_is_not_a_text_is_refused_through_the_loader() {
    let folder = set_up();

    let err = refused(&folder, &GOOD.replace("issue: '54'", "issue: 54"));

    assert!(err.contains("54.yml"), "{}", err);
    assert!(err.contains("quotes"), "{}", err);
}

#[test]
fn test_WB_TASK_1_a_key_written_twice_and_an_alias_are_refused_through_the_loader() {
    let folder = set_up();

    let twice = refused(&folder, &format!("{}questions: []\n", GOOD));
    let alias = refused(&folder, &GOOD.replace("from: 3f0c1d9e", "from: &a x").replace("interface: |", "interface: *a\nxx: |"));

    assert!(twice.contains("54.yml"), "{}", twice);
    assert!(alias.contains("54.yml"), "{}", alias);
}

#[test]
fn test_WB_TASK_1_the_issue_of_the_file_is_the_issue_asked_for() {
    let folder = set_up();

    let err = refused(&folder, &GOOD.replace("issue: '54'", "issue: '55'"));

    assert!(err.contains("55") && err.contains("54"), "{}", err);
}

#[test]
fn test_WB_TASK_1_the_usage_names_accept() {
    let run = wb(&folder().path, &["help", "accept"], "");

    assert!(text(&run.stdout).contains("  wb accept <issue> [--no-commit]\n"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_TASK_1_a_wrong_use_ends_with_2_and_the_usage() {
    let folder = set_up();
    with_good(&folder);
    let before = snapshot(&folder.path);
    for args in [
        vec!["accept"],
        vec!["accept", "54", "55"],
        vec!["accept", "--force", "54"],
        vec!["accept", "../54"],
        vec!["accept", "a/b"],
        vec!["accept", ".54"],
        vec!["accept", ""],
    ] {
        let run = wb(&folder.path, &args, "");

        assert_eq!(code(&run), Some(2), "{:?}", args);
        assert!(text(&run.stderr).contains("usage:"), "{:?}: {}", args, text(&run.stderr));
    }
    assert_eq!(snapshot(&folder.path), before);
}

#[test]
fn test_WB_TASK_1_outside_a_repository_and_where_watchbill_is_not_set_up_ends_with_2() {
    let outside = folder();
    let run = wb(&outside.path, &["accept", "54", "--no-commit"], "");
    assert_eq!(code(&run), Some(2), "{}", text(&run.stderr));

    let plain = repository(None);
    put(&plain.path, ".watchbill/issues/54.yml", GOOD);
    let run = wb(&plain.path, &["accept", "54", "--no-commit"], "");
    assert_eq!(code(&run), Some(2), "{}", text(&run.stderr));
    assert!(!plain.path.join(".watchbill/issues/54.accepted").exists());
}

#[test]
fn test_WB_TASK_2_a_key_that_is_missing_is_named() {
    let folder = set_up();

    let err = refused(&folder, &GOOD.replace("from: 3f0c1d9e\n", ""));

    assert!(err.contains("from") && err.contains("missing"), "{}", err);
}

#[test]
fn test_WB_TASK_2_a_key_out_of_order_is_named() {
    let folder = set_up();
    let swapped = GOOD
        .replace("changes: [src/issue.rs, tests/issue.rs]\n", "")
        .replace("existing_tests_changed: []\n", "existing_tests_changed: []\nchanges: [src/issue.rs]\n");

    let err = refused(&folder, &swapped);

    assert!(err.contains("changes") && err.contains("order"), "{}", err);
}

#[test]
fn test_WB_TASK_2_a_key_that_is_not_known_is_named() {
    let folder = set_up();

    let err = refused(&folder, &format!("{}notes: []\n", GOOD));

    assert!(err.contains("notes") && err.contains("not known"), "{}", err);
}

#[test]
fn test_WB_TASK_2_a_key_of_a_requirement_or_of_reads_is_held_to_the_form() {
    let folder = set_up();

    let source = refused(&folder, &GOOD.replace("    source: the goal\n", ""));
    let extra = refused(&folder, &GOOD.replace("    source: the goal\n", "    source: the goal\n    note: x\n"));
    let reads = refused(&folder, &GOOD.replace("  fix: []\n", "  fix: []\n  other: []\n"));
    let order = refused(&folder, &GOOD.replace("  tester: [src/lib.rs]\n  build: []\n", "  build: []\n  tester: [src/lib.rs]\n"));

    assert!(source.contains("source") && source.contains("missing"), "{}", source);
    assert!(extra.contains("note") && extra.contains("not known"), "{}", extra);
    assert!(reads.contains("other") && reads.contains("not known"), "{}", reads);
    assert!(order.contains("order"), "{}", order);
}

#[test]
fn test_WB_TASK_2_a_value_of_the_wrong_type_is_named() {
    let folder = set_up();

    let list = refused(&folder, &GOOD.replace("changes: [src/issue.rs, tests/issue.rs]", "changes: src/issue.rs"));
    let empty = refused(&folder, &GOOD.replace("from: 3f0c1d9e", "from: ''"));

    assert!(list.contains("changes"), "{}", list);
    assert!(empty.contains("from"), "{}", empty);
}

#[test]
fn test_WB_TASK_3_an_id_that_stands_twice_is_named() {
    let folder = set_up();

    let err = refused(&folder, &GOOD.replace("id: '54.2'", "id: '54.1'"));

    assert!(err.contains("54.1") && err.contains("twice"), "{}", err);
}

#[test]
fn test_WB_TASK_4_a_path_that_is_absolute_holds_dot_dot_or_lies_under_watchbill_is_named() {
    let folder = set_up();
    for (bad, why) in [
        ("/etc/passwd", "absolute"),
        ("src/../../x", ".."),
        ("..", ".."),
        (".watchbill/issues/54.yml", ".watchbill"),
        ("./.watchbill/revision/x", ".watchbill"),
        (".watchbill", ".watchbill"),
    ] {
        let changes = refused(&folder, &GOOD.replace("[src/issue.rs, tests/issue.rs]", &format!("[src/issue.rs, '{}']", bad)));
        let reads = refused(&folder, &GOOD.replace("build: []", &format!("build: ['{}']", bad)));

        for err in [changes, reads] {
            assert!(err.contains(bad) && err.contains(why), "{}: {}", bad, err);
        }
    }
}

#[test]
fn test_WB_TASK_5_a_file_with_questions_is_refused_and_they_are_named() {
    let folder = set_up();

    let err = refused(&folder, &GOOD.replace("questions: []", "questions: [Which way round?, Is it a list?]"));

    assert!(err.contains("Which way round?") && err.contains("Is it a list?"), "{}", err);
}

#[test]
fn test_WB_TASK_6_it_says_what_it_will_write_before_it_writes() {
    let folder = set_up();
    with_good(&folder);

    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");

    let out = text(&run.stdout);
    assert!(out.contains(".watchbill/issues/54.accepted"), "{}", out);
    for line in read(&accepted(&folder)).lines() {
        assert!(out.contains(line), "{} is not said: {}", line, out);
    }
}

#[test]
fn test_WB_TASK_6_a_refusal_says_nothing_of_a_write() {
    let folder = set_up();
    put(&folder.path, ".watchbill/issues/54.yml", &GOOD.replace("questions: []", "questions: [x]"));

    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");

    assert!(!text(&run.stdout).contains("54.accepted"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_TASK_7_the_acceptance_holds_the_hash_the_identity_and_the_time() {
    let folder = set_up();
    with_good(&folder);

    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let lines: Vec<String> = read(&accepted(&folder)).lines().map(String::from).collect();
    assert_eq!(lines.len(), 3, "{:?}", lines);
    assert_eq!(lines[0], format!("sha256: {}", manifest::hash_bytes(GOOD.as_bytes())));
    assert_eq!(lines[1], "by: Ada Lovelace <ada@example.org>");
    let time = lines[2].strip_prefix("time: ").expect("the third line is the time");
    let shape = b"dddd-dd-ddTdd:dd:ddZ";
    assert_eq!(time.len(), shape.len(), "{}", time);
    for (found, want) in time.bytes().zip(shape) {
        assert!(if *want == b'd' { found.is_ascii_digit() } else { found == *want }, "{}", time);
    }
    assert!(time > "2026-11-18T00:00:00Z", "{}", time);
}

#[test]
fn test_WB_TASK_8_an_acceptance_that_is_there_is_never_written_over() {
    let folder = set_up();
    with_good(&folder);
    put(&folder.path, ".watchbill/issues/54.accepted", "an old one\n");

    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    assert_eq!(read(&accepted(&folder)), "an old one\n");
    assert!(text(&run.stderr).contains("remove"), "{}", text(&run.stderr));
    assert!(!text(&run.stdout).contains("will write"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_TASK_8_a_second_accept_does_not_change_the_first() {
    let folder = set_up();
    with_good(&folder);
    assert_eq!(code(&wb(&folder.path, &["accept", "54", "--no-commit"], "")), Some(0));
    let first = read(&accepted(&folder));

    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(read(&accepted(&folder)), first);
}

#[test]
fn test_WB_TASK_8_a_link_in_the_place_of_the_acceptance_is_not_written_through() {
    let folder = set_up();
    with_good(&folder);
    let target = folder.path.join("elsewhere");
    std::os::unix::fs::symlink(&target, accepted(&folder)).unwrap();

    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    assert!(!target.exists());
}

#[test]
fn test_WB_TASK_9_no_git_name_or_no_git_email_refuses_with_1() {
    for (key, other, value) in [("user.name", "user.email", "a@b.c"), ("user.email", "user.name", "Ada")] {
        let folder = repository(None);
        assert_eq!(code(&wb(&folder.path, &["init", "--yes"], "")), Some(0));
        commit_config(&folder.path);
        git(&folder.path, &["config", other, value]);
        with_good(&folder);
        let before = snapshot(&folder.path);

        let run = wb_without_identity(&folder.path, &["accept", "54", "--no-commit"]);

        assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
        assert!(text(&run.stderr).contains(key), "{}", text(&run.stderr));
        assert_eq!(snapshot(&folder.path), before);
    }
}

#[test]
fn test_WB_TASK_9_no_identity_at_all_refuses_with_1() {
    let folder = repository(None);
    assert_eq!(code(&wb(&folder.path, &["init", "--yes"], "")), Some(0));
    commit_config(&folder.path);
    with_good(&folder);

    let run = wb_without_identity(&folder.path, &["accept", "54", "--no-commit"]);

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(!accepted(&folder).exists());
}

#[test]
fn test_WB_TASK_10_a_file_counts_as_accepted_while_its_hash_is_that_of_the_acceptance() {
    let folder = set_up();
    with_good(&folder);
    assert!(!is_accepted(&folder.path, "54"), "no acceptance yet");
    assert_eq!(code(&wb(&folder.path, &["accept", "54", "--no-commit"], "")), Some(0));

    assert!(is_accepted(&folder.path, "54"));
}

#[test]
fn test_WB_TASK_10_one_byte_changed_and_the_file_is_not_accepted() {
    let folder = set_up();
    with_good(&folder);
    assert_eq!(code(&wb(&folder.path, &["accept", "54", "--no-commit"], "")), Some(0));
    let file = folder.path.join(".watchbill/issues/54.yml");

    fs::write(&file, GOOD.replace("reads the file", "reads the files")).unwrap();
    assert!(!is_accepted(&folder.path, "54"));
    fs::write(&file, GOOD).unwrap();
    assert!(is_accepted(&folder.path, "54"), "put back, it is accepted again");
    fs::write(&file, format!("{}\n", GOOD)).unwrap();
    assert!(!is_accepted(&folder.path, "54"), "one byte added");
}

#[test]
fn test_WB_TASK_10_an_acceptance_that_cannot_be_read_does_not_count() {
    let folder = set_up();
    with_good(&folder);
    for body in ["", "garbage\n", "sha256: \n", "by: x\n"] {
        put(&folder.path, ".watchbill/issues/54.accepted", body);

        assert!(!is_accepted(&folder.path, "54"), "{:?}", body);
    }
    assert!(!is_accepted(&folder.path, "99"), "an issue with no file");
}

#[test]
fn test_WB_TASK_11_wb_check_does_not_fail_on_the_files_under_issues() {
    let folder = set_up();
    with_good(&folder);
    put(&folder.path, ".watchbill/issues/other.txt", "x\n");
    assert_eq!(code(&wb(&folder.path, &["accept", "54", "--no-commit"], "")), Some(0));

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stdout));
}

#[test]
fn test_WB_TASK_12_nothing_but_the_acceptance_is_written() {
    let folder = set_up();
    with_good(&folder);
    let before = snapshot(&folder.path);

    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let after = snapshot(&folder.path);
    let new: Vec<&str> = after.iter().filter(|file| !before.contains(file)).map(|file| file.0.as_str()).collect();
    assert_eq!(new, vec![".watchbill/issues/54.accepted"]);
    assert_eq!(after.len(), before.len() + 1);
}

#[test]
fn test_WB_TASK_12_a_refusal_writes_nothing() {
    let folder = set_up();
    put(&folder.path, ".watchbill/issues/54.yml", "issue: '54'\n");
    let before = snapshot(&folder.path);

    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(snapshot(&folder.path), before);
}

// Round 1 of the review of #80.

#[test]
fn test_WB_TASK_1_a_watchbill_folder_that_is_a_link_is_refused_by_name_and_not_written_through() {
    let folder = set_up();
    with_good(&folder);
    fs::rename(folder.path.join(".watchbill"), folder.path.join("real")).unwrap();
    std::os::unix::fs::symlink("real", folder.path.join(".watchbill")).unwrap();

    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");

    assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
    let err = text(&run.stderr);
    assert!(err.contains(".watchbill") && err.contains("link"), "{}", err);
    assert!(!folder.path.join("real/issues/54.accepted").exists());
    assert!(!is_accepted(&folder.path, "54"));
}

#[test]
fn test_WB_TASK_1_the_issues_folder_that_is_a_link_is_refused_by_name_and_not_written_through() {
    let folder = set_up();
    with_good(&folder);
    fs::rename(folder.path.join(".watchbill/issues"), folder.path.join("elsewhere")).unwrap();
    std::os::unix::fs::symlink("../elsewhere", folder.path.join(".watchbill/issues")).unwrap();

    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");

    assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
    let err = text(&run.stderr);
    assert!(err.contains(".watchbill/issues") && err.contains("link"), "{}", err);
    assert!(!folder.path.join("elsewhere/54.accepted").exists());
    assert!(!is_accepted(&folder.path, "54"));
}

#[test]
fn test_WB_TASK_1_an_issues_path_that_is_not_a_folder_is_refused_by_name() {
    let folder = set_up();
    put(&folder.path, ".watchbill/issues", "not a folder\n");

    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");

    assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
    assert!(text(&run.stderr).contains(".watchbill/issues"), "{}", text(&run.stderr));
    assert_eq!(read(&folder.path.join(".watchbill/issues")), "not a folder\n");
}

#[test]
fn test_WB_TASK_4_a_path_that_names_the_root_is_refused() {
    let folder = set_up();
    for bad in [".", "./", "./.", ".//"] {
        let changes = refused(&folder, &GOOD.replace("[src/issue.rs, tests/issue.rs]", &format!("[src/issue.rs, '{}']", bad)));
        let reads = refused(&folder, &GOOD.replace("build: []", &format!("build: ['{}']", bad)));

        for err in [changes, reads] {
            assert!(err.contains(&format!("'{}'", bad)) && err.contains("root"), "{}: {}", bad, err);
        }
    }
}

#[test]
fn test_WB_TASK_5_what_is_printed_of_the_file_has_its_control_characters_escaped() {
    let folder = set_up();

    let err = refused(&folder, &GOOD.replace("questions: []", "questions: [\"a\\e[31mred\", \"two\\nlines\"]"));

    assert!(!err.contains('\x1b'), "{:?}", err);
    assert!(err.contains("a\\u{1b}[31mred") && err.contains("two\\nlines"), "{:?}", err);
    assert_eq!(err.lines().filter(|line| line.starts_with("  - ")).count(), 2, "{:?}", err);
}

#[test]
fn test_WB_TASK_1_the_issue_of_the_file_is_escaped_when_it_is_named() {
    let folder = set_up();

    let err = refused(&folder, &GOOD.replace("issue: '54'", "issue: \"5\\e5\""));

    assert!(!err.contains('\x1b'), "{:?}", err);
    assert!(err.contains("5\\u{1b}5"), "{:?}", err);
}

#[test]
fn test_WB_TASK_1_a_name_of_more_than_64_characters_is_used_wrongly() {
    let folder = set_up();
    let long = "a".repeat(65);
    let run = wb(&folder.path, &["accept", "--no-commit", &long], "");
    assert_eq!(code(&run), Some(2), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("usage:"));

    // 64 is a name: it has no file, which is a refusal of the file, not a wrong use.
    let run = wb(&folder.path, &["accept", "--no-commit", &"a".repeat(64)], "");
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
}

#[test]
fn test_WB_TASK_8_a_temporary_file_left_behind_does_not_block_a_later_accept() {
    let folder = set_up();
    with_good(&folder);
    for name in [".54.accepted.tmp", ".54.accepted.0", "54.accepted.tmp"] {
        put(&folder.path, &format!(".watchbill/issues/{}", name), "half");
    }

    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(is_accepted(&folder.path, "54"));
    for name in [".54.accepted.tmp", ".54.accepted.0", "54.accepted.tmp"] {
        assert_eq!(read(&folder.path.join(".watchbill/issues").join(name)), "half");
    }
}

#[test]
fn test_WB_TASK_8_a_part_file_under_the_name_is_not_taken_as_accepted_and_not_written_over() {
    let folder = set_up();
    with_good(&folder);
    put(&folder.path, ".watchbill/issues/54.accepted", "sha256: ");

    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");

    assert_eq!(code(&run), Some(1));
    assert!(!is_accepted(&folder.path, "54"));
    assert_eq!(read(&accepted(&folder)), "sha256: ");
}

#[test]
fn test_WB_TASK_9_a_git_name_or_email_with_angle_brackets_is_refused() {
    for (key, value) in [
        ("user.name", "Ada <evil>"),
        ("user.name", "Ada >"),
        ("user.name", "<Ada"),
        ("user.email", "a@b.c>"),
        ("user.email", "<a@b.c"),
    ] {
        let folder = set_up();
        with_good(&folder);
        git(&folder.path, &["config", key, value]);
        let before = snapshot(&folder.path);

        let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");

        assert_eq!(code(&run), Some(1), "{} {}: {}", key, value, text(&run.stderr));
        assert!(text(&run.stderr).contains(key), "{}", text(&run.stderr));
        assert_eq!(snapshot(&folder.path), before);
    }
}

#[test]
fn test_WB_TASK_1_a_fault_names_the_path_from_the_root_of_the_repository() {
    let folder = set_up();

    let err = refused(&folder, &GOOD.replace("from: 3f0c1d9e\n", ""));

    assert!(err.contains(".watchbill/issues/54.yml"), "{}", err);
    assert!(!err.contains(folder.path.to_str().unwrap()), "{}", err);
}
