//! The Rust `wb`, run as the command a person types: a process, its text and its exit status.
//! The words are those of the Python `wb`, which is still the one that is installed.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const USAGE: &str = "usage:
  wb init [--yes]
  wb revision
  wb --version

  init      set Watchbill up in the git repository of this folder: propose what
            the repository says about itself (its name, the git server of
            origin, a folder of requirements, the CI driver), ask one line per
            answer, show the file and write .watchbill/watchbill.yml at the
            root of the repository. --yes (-y) takes what it found and asks
            nothing. A file that is already there is never changed
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

/// The version the package has: the one line of `__init__.py` that sets it.
fn python_version() -> Option<String> {
    let file = fs::read_to_string(root().join("src/watchbill/__init__.py")).ok()?;
    file.lines()
        .find_map(|line| line.strip_prefix("__version__ = \"")?.strip_suffix('"').map(String::from))
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
    // `init` and `revision` are in the Python `wb`. This program does not have them yet (#22 to #24).
    for word in ["init", "revision"] {
        let run = wb(&[word]);

        assert_eq!(run.status.code(), Some(2), "{}", word);
        assert_eq!(text(&run.stdout), "", "{}", word);
        assert_eq!(text(&run.stderr), format!("wb: no command '{}'\n\n{}", word, USAGE), "{}", word);
    }
}

#[test]
fn test_WB_PKG_8_the_version_of_the_crate_is_the_version_of_the_package() {
    // As long as both are there: the Python package is removed by #25.
    if let Some(package) = python_version() {
        assert_eq!(crate_version(), package);
    }
}
