//! `wb prompt <role>`: the command that prints the prompt of an assignment, run as a person runs it,
//! in a repository made for the test. How a prompt is put together is tested in `tests/prompt.rs`.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use watchbill::prompt::FIXED_LINE;
use watchbill::USAGE;

fn package() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("share/watchbill")
}

/// A repository set up by `wb init --yes`.
fn set_up() -> Folder {
    let folder = repository(None);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    folder
}

/// The text of a role of the package with each placeholder filled in.
fn filled(role: &str, values: &[(&str, &str)]) -> String {
    let mut text = fs::read_to_string(package().join("revision/roles").join(format!("{}.md", role))).unwrap();
    for (name, value) in values {
        text = text.replace(&format!("{{{}}}", name), value);
    }
    text
}

const ALL: [&str; 8] = ["--task", "T-1", "--change", "34", "--requirements", "WB-X-1", "--round", "1"];
const ALL_VALUES: [(&str, &str); 4] = [("task", "T-1"), ("change", "34"), ("requirements", "WB-X-1"), ("round", "1")];

/// Every plain file under a folder, with its bytes, sorted by path.
fn snapshot(folder: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &Path, at: &Path, found: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
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
fn test_WB_PROMPT_10_a_fresh_repository_prints_the_build_prompt_with_the_task_filled_in() {
    let folder = set_up();

    let run = wb(&folder.path, &["prompt", "build", "--task", "12", "--requirements", "WB-X-1"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), filled("build", &[("task", "12"), ("requirements", "WB-X-1")]));
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_PROMPT_10_with_project_text_the_output_is_the_three_parts_in_order() {
    let folder = set_up();
    put(&folder.path, ".watchbill/project/all.md", "Everyone: use make.\n");
    put(&folder.path, ".watchbill/project/build.md", "Builders: run make test.\n");

    let run = wb(&folder.path, &["prompt", "build", "--task", "T-1", "--requirements", "WB-X-1"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let role = filled("build", &[("task", "T-1"), ("requirements", "WB-X-1")]);
    assert_eq!(
        text(&run.stdout),
        format!("{}\n{}\n\nEveryone: use make.\n\nBuilders: run make test.\n", role, FIXED_LINE)
    );
}

#[test]
fn test_WB_PROMPT_10_an_option_may_come_before_the_role() {
    let folder = set_up();

    let run = wb(&folder.path, &["prompt", "--task", "7", "tester", "--requirements", "R"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), filled("tester", &[("task", "7"), ("requirements", "R")]));
}

#[test]
fn test_WB_PROMPT_10_every_option_given_is_filled_in_every_role() {
    let folder = set_up();

    for role in ["build", "fix", "review", "tester"] {
        let mut args = vec!["prompt", role];
        args.extend(ALL);
        let run = wb(&folder.path, &args, "");

        assert_eq!(code(&run), Some(0), "{}: {}", role, text(&run.stderr));
        assert_eq!(text(&run.stdout), filled(role, &ALL_VALUES), "{}", role);
    }
}

#[test]
fn test_WB_PROMPT_8_wb_check_passes_with_project_files_there_and_after_one_is_changed() {
    let folder = set_up();
    put(&folder.path, ".watchbill/project/all.md", "Everyone: use make.\n");
    put(&folder.path, ".watchbill/project/build.md", "Builders: run make test.\n");

    let run = wb(&folder.path, &["check"], "");
    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));

    put(&folder.path, ".watchbill/project/all.md", "Everyone: use cmake instead.\n");
    fs::write(folder.path.join(".watchbill/project/README.md"), "changed\n").unwrap();
    let run = wb(&folder.path, &["check"], "");
    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_12_one_changed_byte_of_a_prompt_ends_with_1_prints_nothing_and_names_the_file() {
    let folder = set_up();
    let file = folder.path.join(".watchbill/revision/roles/build.md");
    let mut bytes = fs::read(&file).unwrap();
    bytes.insert(0, b'x');
    fs::write(&file, bytes).unwrap();

    let run = wb(&folder.path, &["prompt", "build", "--task", "1", "--requirements", "R"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    let err = text(&run.stderr);
    assert!(err.contains("revision/roles/build.md"), "{}", err);
}

#[test]
fn test_WB_PROMPT_12_a_repository_pinned_to_another_revision_than_the_installed_wb_ends_with_1() {
    let folder = set_up();
    let file = config(&folder.path);
    let changed: String = read(&file)
        .lines()
        .map(|line| {
            if line.starts_with("watchbill: ") {
                "watchbill: 0.0.1\n".to_string()
            } else {
                format!("{}\n", line)
            }
        })
        .collect();
    fs::write(&file, changed).unwrap();

    let run = wb(&folder.path, &["prompt", "build", "--task", "1", "--requirements", "R"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("0.0.1"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_13_no_watchbill_folder_ends_with_2_and_prints_nothing() {
    let folder = repository(None);

    let run = wb(&folder.path, &["prompt", "build", "--task", "1"], "");

    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("not set up"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_13_a_folder_that_is_not_in_a_git_repository_ends_with_2() {
    let folder = folder();

    let run = wb(&folder.path, &["prompt", "build", "--task", "1"], "");

    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stdout), "");
}

#[test]
fn test_WB_PROMPT_14_a_placeholder_the_prompt_uses_with_no_option_is_refused_by_name() {
    let folder = set_up();

    let run = wb(&folder.path, &["prompt", "build", "--task", "12"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("{requirements}"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_14_no_option_at_all_is_refused_for_a_placeholder_that_is_used() {
    let folder = set_up();

    let run = wb(&folder.path, &["prompt", "build"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("got no value"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_14_a_role_the_revision_does_not_have_is_refused_with_the_roles_it_has() {
    let folder = set_up();

    let run = wb(&folder.path, &["prompt", "captain", "--task", "1"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    let err = text(&run.stderr);
    for role in ["build", "fix", "review", "tester"] {
        assert!(err.contains(role), "{}: {}", role, err);
    }
    assert!(err.contains("captain"), "{}", err);
}

#[test]
fn test_WB_PROMPT_15_an_option_that_is_no_placeholder_ends_with_2_and_the_usage() {
    let folder = set_up();

    for args in [
        vec!["prompt", "build", "--branch", "x"],
        vec!["prompt", "build", "--branch=x"],
        vec!["prompt", "build", "-t"],
    ] {
        let run = wb(&folder.path, &args, "");

        assert_eq!(code(&run), Some(2), "{:?}", args);
        assert_eq!(text(&run.stdout), "", "{:?}", args);
        assert!(text(&run.stderr).starts_with("wb prompt: '"), "{:?}: {}", args, text(&run.stderr));
        assert!(text(&run.stderr).ends_with(&format!("\n\n{}", USAGE)), "{:?}: {}", args, text(&run.stderr));
    }
}

#[test]
fn test_WB_PROMPT_15_a_missing_role_a_second_word_and_a_repeated_option_end_with_2_and_the_usage() {
    let folder = set_up();

    for args in [
        vec!["prompt"],
        vec!["prompt", "--task", "1"],
        vec!["prompt", "build", "fix"],
        vec!["prompt", "build", "--task", "1", "--task", "2"],
        vec!["prompt", "build", "--task"],
        vec!["prompt", "build", "--parts=yes"],
    ] {
        let run = wb(&folder.path, &args, "");

        assert_eq!(code(&run), Some(2), "{:?}", args);
        assert_eq!(text(&run.stdout), "", "{:?}", args);
        assert!(text(&run.stderr).ends_with(&format!("\n\n{}", USAGE)), "{:?}: {}", args, text(&run.stderr));
    }
}

#[test]
fn test_WB_PROMPT_15_the_usage_names_wb_prompt() {
    let run = wb(&folder().path, &["--help"], "");

    assert!(text(&run.stdout).contains("wb prompt <role>"), "{}", text(&run.stdout));
    assert!(text(&run.stdout).contains("--parts"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_PROMPT_11_parts_prints_one_line_per_part_and_sha256sum_passes_at_the_root() {
    let folder = set_up();
    put(&folder.path, ".watchbill/project/all.md", "Everyone: use make.\n");
    put(&folder.path, ".watchbill/project/build.md", "Builders: run make test.\n");

    let mut args = vec!["prompt", "build", "--parts"];
    args.extend(ALL);
    let run = wb(&folder.path, &args, "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    let paths: Vec<&str> = out.lines().map(|line| line.split_once("  ").unwrap().1).collect();
    assert_eq!(
        paths,
        [".watchbill/revision/roles/build.md", ".watchbill/project/all.md", ".watchbill/project/build.md"]
    );
    assert!(!out.contains("Everyone"), "{}", out);
    let sums = folder.path.join("sums.txt");
    fs::write(&sums, &out).unwrap();
    let check = Command::new("sha256sum").arg("-c").arg(&sums).current_dir(&folder.path).output().unwrap();
    assert!(check.status.success(), "{}{}", text(&check.stdout), text(&check.stderr));
}

#[test]
fn test_WB_PROMPT_11_with_no_project_text_parts_prints_the_one_line_of_the_role() {
    let folder = set_up();

    // The tester's text has placeholders with no value: refused, nothing printed.
    let run = wb(&folder.path, &["prompt", "tester", "--parts"], "");
    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");

    let mut args = vec!["prompt", "tester", "--parts"];
    args.extend(ALL);
    let run = wb(&folder.path, &args, "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout).lines().count(), 1);
}

#[test]
fn test_WB_PROMPT_16_a_value_that_starts_with_a_dash_or_holds_a_line_break_arrives_as_given() {
    let folder = set_up();

    for (args, task, requirements) in [
        (vec!["prompt", "build", "--task", "-x", "--requirements", "R"], "-x", "R"),
        (vec!["prompt", "build", "--task=-x", "--requirements=-y"], "-x", "-y"),
        (vec!["prompt", "build", "--task", "--", "--requirements", "a\nb\n\nc"], "--", "a\nb\n\nc"),
        (vec!["prompt", "build", "--task", "one\ntwo", "--requirements", ""], "one\ntwo", ""),
    ] {
        let run = wb(&folder.path, &args, "");

        assert_eq!(code(&run), Some(0), "{:?}: {}", args, text(&run.stderr));
        assert_eq!(text(&run.stdout), filled("build", &[("task", task), ("requirements", requirements)]), "{:?}", args);
    }
}

#[test]
fn test_WB_PROMPT_17_a_run_changes_nothing_under_the_repository() {
    let folder = set_up();
    put(&folder.path, ".watchbill/project/all.md", "Everyone: use make.\n");
    let before = snapshot(&folder.path);

    for args in [
        vec!["prompt", "build", "--task", "1", "--requirements", "R"],
        vec!["prompt", "build", "--parts", "--task", "1", "--requirements", "R"],
        vec!["prompt", "build"],
        vec!["prompt", "nobody"],
    ] {
        wb(&folder.path, &args, "");
        assert_eq!(snapshot(&folder.path), before, "{:?}", args);
    }
}
