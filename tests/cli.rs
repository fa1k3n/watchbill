//! The Rust `wb`, run as the command a person types: a process, its text and its exit status.
//! The words are those of the first `wb`, which was written in Python.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::{Command, Output};

const USAGE: &str = "usage:
  wb init [--yes]
  wb check
  wb prompt <role> [<task>] [--task <text>] [--change <text>]
            [--requirements <text>] [--round <text>] [--checkout <text>]
            [--branch <text>] [--task_address <text>]
            [--change_address <text>] [--parts] [--stamp]
  wb revision
  wb --version

  init      set Watchbill up in the git repository of this folder: propose what
            the repository says about itself (its name, the git server of
            origin, a folder of requirements, the CI driver), ask one line per
            answer, show the file and write .watchbill/watchbill.yml at the
            root of the repository. --yes (-y) takes what it found and asks
            nothing. A file that is already there is never changed
  check     say whether the repository still is at the revision it is pinned to:
            each file under .watchbill/revision/ against the manifest, the
            manifest against the revision line of watchbill.yml, and that
            revision against the installed wb. Ends with 0 when all of it
            holds, 1 when something differs. Writes nothing
  prompt    print the prompt of a role on standard output and nothing else: the
            role's text of the pinned revision with each placeholder filled
            from the option of its name, such as --task for {task}, then the
            project's own text. The task is the word after the role, or
            --task. What no option gives is taken from the repository and
            watchbill.yml: the checkout, the branch, the two addresses and
            the requirements; an option always wins. It runs the check of wb
            check first and prints nothing when that fails. --parts prints the path and the SHA-256 of
            each part, in the form sha256sum prints, in place of the text.
            --stamp prints, in place of the text, what the assignment is: the
            role, its model and effort, the version and revision, the task, the
            change, the round, the branch, the checkout, the time, and each
            part as --parts gives it. Not with --parts. Ends with 0 when the
            prompt is printed, 1 when it is refused, 2 when it was used
            wrongly or the repository is not set up. Writes nothing
  revision  print the version of the package, the hash of its revision and the
            manifest: the hash of every file the revision holds. Writes nothing
";

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn wb(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(args)
        .output()
        .expect("wb could not be started")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("wb wrote text that is not UTF-8")
}

/// The version of the crate: the `version` line of the `[package]` table.
fn crate_version() -> String {
    let file = fs::read_to_string(root().join("Cargo.toml")).expect("Cargo.toml is not there");
    let mut in_package = false;
    for line in file.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
        } else if in_package {
            if let Some(rest) = line.strip_prefix("version") {
                let value = rest.trim_start().strip_prefix('=').expect("version has no =").trim();
                return value.trim_matches('"').to_string();
            }
        }
    }
    panic!("Cargo.toml has no version in [package]");
}

#[test]
fn test_WB_INIT_26_version_prints_the_version_of_the_package() {
    let run = wb(&["--version"]);

    assert_eq!(run.status.code(), Some(0));
    assert_eq!(text(&run.stdout), format!("{}\n", crate_version()));
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_INIT_36_dash_v_does_as_dash_dash_version() {
    let run = wb(&["-v"]);

    assert_eq!(run.status.code(), Some(0));
    assert_eq!(text(&run.stdout), format!("{}\n", crate_version()));
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_INIT_27_help_prints_the_usage_and_ends_with_0() {
    let run = wb(&["--help"]);

    assert_eq!(run.status.code(), Some(0));
    assert_eq!(text(&run.stdout), USAGE);
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_INIT_35_help_and_h_do_as_dash_dash_help() {
    for word in ["help", "-h"] {
        let run = wb(&[word]);

        assert_eq!(run.status.code(), Some(0), "{}", word);
        assert_eq!(text(&run.stdout), USAGE, "{}", word);
        assert_eq!(text(&run.stderr), "", "{}", word);
    }
}

#[test]
fn test_WB_INIT_25_no_command_ends_with_2_and_the_usage() {
    let run = wb(&[]);

    assert_eq!(run.status.code(), Some(2));
    assert_eq!(text(&run.stdout), USAGE);
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_INIT_25_a_command_that_is_not_known_ends_with_2_and_the_usage_on_standard_error() {
    for word in ["start", "--force", "", "Help", "--version=1"] {
        let run = wb(&[word]);

        assert_eq!(run.status.code(), Some(2), "{:?}", word);
        assert_eq!(text(&run.stdout), "", "{:?}", word);
        assert_eq!(text(&run.stderr), format!("wb: no command '{}'\n\n{}", word, USAGE), "{:?}", word);
    }
}

#[test]
fn test_WB_INIT_25_only_the_first_word_is_the_command() {
    let run = wb(&["start", "--version"]);

    assert_eq!(run.status.code(), Some(2));
    assert_eq!(text(&run.stderr), format!("wb: no command 'start'\n\n{}", USAGE));

    // Words after a known option change nothing, as in the Python `wb`.
    let run = wb(&["--version", "start"]);

    assert_eq!(run.status.code(), Some(0));
    assert_eq!(text(&run.stdout), format!("{}\n", crate_version()));
}

#[test]
fn test_WB_INIT_25_the_commands_that_are_not_built_yet_are_not_known() {
    // `init` is built (#22), `revision` is built (#24) and `check` is built (#8): none is left.
    for word in [] as [&str; 0] {
        let run = wb(&[word]);

        assert_eq!(run.status.code(), Some(2), "{}", word);
        assert_eq!(text(&run.stdout), "", "{}", word);
        assert_eq!(text(&run.stderr), format!("wb: no command '{}'\n\n{}", word, USAGE), "{}", word);
    }
}

#[test]
fn test_WB_PKG_9_wb_version_prints_the_version_in_cargo_toml() {
    let run = wb(&["--version"]);

    assert_eq!(run.status.code(), Some(0));
    assert_eq!(text(&run.stdout), format!("{}\n", crate_version()));
    assert_ne!(crate_version(), "");
}

#[test]
fn test_WB_PKG_6_the_source_of_crates_has_the_name_the_macros_give_it() {
    // `packaging/rpm.sh` builds in a folder that may be inside the checkout. `%cargo_prep` then
    // writes a config that cargo merges with this one, and two names for the one folder are refused.
    let text = fs::read_to_string(root().join(".cargo/config.toml")).expect(".cargo/config.toml is not there");
    assert!(text.contains("[source.local-registry]"), "the source is not named local-registry");
    assert!(text.contains("directory = \"/usr/share/cargo/registry\""));
    assert!(text.contains("replace-with = \"local-registry\""));
}

#[test]
fn test_WB_INIT_25_a_word_that_is_not_utf_8_is_written_as_python_writes_it() {
    let run = Command::new(env!("CARGO_BIN_EXE_wb"))
        .arg(OsStr::from_bytes(b"a\xffb"))
        .output()
        .expect("wb could not be started");

    assert_eq!(run.status.code(), Some(2));
    assert_eq!(text(&run.stdout), "");
    assert_eq!(text(&run.stderr), format!("wb: no command 'a\\udcffb'\n\n{}", USAGE));
}

// A fault in `wb` itself: the panic hook is quiet, and the command ends with 1 and one line.

const CHILD: &str = "WB_TEST_PANIC_CHILD";

/// Runs only in the child process of the test below: it makes the program panic through
/// `guarded`, which is how `main` runs a command, and prints what `guarded` returned.
#[test]
fn test_WB_PKG_10_helper_a_panic_in_the_command_is_caught() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    let mut err = std::io::stderr();
    let code = watchbill::guarded(|| panic!("a fault in wb"), &mut err);
    println!("code {}", code);
}

#[test]
fn test_WB_PKG_10_a_panic_in_the_command_ends_with_1_and_one_line_and_no_panic_message() {
    let run = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "test_WB_PKG_10_helper_a_panic_in_the_command_is_caught", "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .output()
        .expect("the test program could not be started");
    let (out, err) = (text(&run.stdout), text(&run.stderr));

    assert!(out.contains("code 1\n"), "{}", out);
    assert!(err.contains("wb: an internal error. This is a fault in wb, not in what it was given.\n"), "{}", err);
    assert!(!err.contains("panicked"), "{}", err);
    assert!(!err.contains("a fault in wb\n"), "{}", err);
}

#[test]
fn test_WB_PKG_10_main_runs_the_command_guarded() {
    let main = fs::read_to_string(root().join("src/main.rs")).unwrap();
    assert!(main.contains("watchbill::guarded("));
}
