//! How `wb` reads its command line: `--`, short options written together, an option given a
//! value it does not take, and words that are not UTF-8. The decisions are in #34.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::{Command, Output};
use watchbill::USAGE;

/// Runs `wb` in the folder with words that may not be UTF-8, and nothing on standard input.
fn wb_os(folder: &Path, args: &[&[u8]]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(args.iter().map(|arg| OsStr::from_bytes(arg)))
        .current_dir(folder)
        .env_clear()
        .envs(env())
        .stdin(std::process::Stdio::null())
        .output()
        .expect("wb could not be started")
}

fn not_known(word: &str) -> String {
    format!("wb init: '{}' is not known\n\n{}", word, USAGE)
}

fn no_command(word: &str) -> String {
    format!("wb: no command '{}'\n\n{}", word, USAGE)
}

/// The run was refused as it was used wrongly: 2, nothing on standard output, and nothing written.
fn assert_refused(run: &Output, folder: &Path, message: &str) {
    assert_eq!(code(run), Some(2), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "");
    assert_eq!(text(&run.stderr), message);
    assert!(!folder.join(".watchbill").exists());
}

/// The run wrote the file of `wb init --yes` and ended with 0.
fn assert_written(run: &Output, folder: &Path) {
    assert_eq!(code(run), Some(0), "{}", text(&run.stderr));
    assert!(config(folder).is_file());
}

// WB-INIT-39: `--` ends the options.

#[test]
fn test_WB_INIT_39_dash_dash_before_the_command_does_as_the_command() {
    // With no answers, `wb init` ends at the question whether it is OK and writes nothing.
    let folder = repository(None);
    let plain = wb(&folder.path, &["init"], "");
    let run = wb(&folder.path, &["--", "init"], "");
    assert_eq!(code(&run), code(&plain));
    assert_eq!(text(&run.stdout), text(&plain.stdout));
    assert_eq!(text(&run.stderr), text(&plain.stderr));
    assert!(text(&run.stdout).contains("project"), "{}", text(&run.stdout));

    // After `--` every word is a word, also those after the command.
    let run = wb(&folder.path, &["--", "init", "--yes"], "");
    assert_refused(&run, &folder.path, &not_known("--yes"));
}

#[test]
fn test_WB_INIT_39_dash_dash_after_init_does_as_init() {
    let folder = repository(None);
    let run = wb(&folder.path, &["init", "--yes", "--"], "");
    assert_written(&run, &folder.path);
}

#[test]
fn test_WB_INIT_39_a_word_after_dash_dash_is_not_an_option() {
    let folder = repository(None);
    let run = wb(&folder.path, &["init", "--", "--yes"], "");
    assert_refused(&run, &folder.path, &not_known("--yes"));

    let run = wb(&folder.path, &["--", "--help"], "");
    assert_refused(&run, &folder.path, &no_command("--help"));
}

#[test]
fn test_WB_INIT_39_dash_dash_alone_does_as_no_command() {
    let folder = folder();
    let run = wb(&folder.path, &["--"], "");
    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stdout), USAGE);
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_INIT_39_dash_dash_after_revision_does_as_revision() {
    let folder = folder();
    let plain = wb(&folder.path, &["revision"], "");
    let run = wb(&folder.path, &["revision", "--"], "");
    assert_eq!(code(&run), code(&plain));
    assert_eq!(text(&run.stdout), text(&plain.stdout));
    assert_eq!(text(&run.stderr), text(&plain.stderr));

    let run = wb(&folder.path, &["revision", "--", "--x"], "");
    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stderr), format!("wb revision: '--x' is not known\n\n{}", USAGE));
}

// WB-INIT-40: short options written together.

#[test]
fn test_WB_INIT_40_dash_yy_does_as_dash_y() {
    let folder = repository(None);
    let run = wb(&folder.path, &["init", "-yy"], "");
    assert_written(&run, &folder.path);
}

#[test]
fn test_WB_INIT_40_yes_given_twice_does_as_yes() {
    let folder = repository(None);
    let run = wb(&folder.path, &["init", "--yes", "-y"], "");
    assert_written(&run, &folder.path);
}

#[test]
fn test_WB_INIT_25_dash_h_and_dash_v_are_taken_only_alone() {
    let folder = folder();
    for word in ["-hv", "-vh", "-hh", "-vx"] {
        let run = wb(&folder.path, &[word], "");
        assert_refused(&run, &folder.path, &no_command(word));
    }
}

// WB-INIT-41: an option given a value it does not take.

#[test]
fn test_WB_INIT_41_yes_with_a_value_is_refused_and_named_with_it() {
    let folder = repository(None);
    for word in ["--yes=x", "--yes=", "-y=x"] {
        let run = wb(&folder.path, &["init", word], "");
        assert_refused(&run, &folder.path, &not_known(word));
    }
}

#[test]
fn test_WB_INIT_41_an_unknown_long_option_is_named_with_its_value() {
    let folder = repository(None);
    let run = wb(&folder.path, &["init", "--force=1"], "");
    assert_refused(&run, &folder.path, &not_known("--force=1"));
}

#[test]
fn test_WB_INIT_41_version_and_help_with_a_value_are_not_commands() {
    let folder = folder();
    for word in ["--version=1", "--help=x", "-h=x", "-v1"] {
        let run = wb(&folder.path, &[word], "");
        assert_refused(&run, &folder.path, &no_command(word));
    }
}

#[test]
fn test_WB_INIT_41_revision_refuses_an_option_with_its_value() {
    let folder = folder();
    let run = wb(&folder.path, &["revision", "--all=yes"], "");
    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stdout), "");
    assert_eq!(text(&run.stderr), format!("wb revision: '--all=yes' is not known\n\n{}", USAGE));
}

// WB-INIT-42: an option that is not known is named alone.

#[test]
fn test_WB_INIT_42_an_option_written_with_others_is_named_alone() {
    let folder = repository(None);
    let run = wb(&folder.path, &["init", "-yx"], "");
    assert_refused(&run, &folder.path, &not_known("-x"));

    let run = wb(&folder.path, &["init", "-xy"], "");
    assert_refused(&run, &folder.path, &not_known("-x"));
}

#[test]
fn test_WB_INIT_42_an_option_name_that_is_not_utf_8_is_written_with_the_replacement_character() {
    let folder = repository(None);
    let run = wb_os(&folder.path, &[b"init", b"-\xff"]);
    assert_refused(&run, &folder.path, &not_known("-\u{FFFD}"));

    let run = wb_os(&folder.path, &[b"init", b"--a\xffb"]);
    assert_refused(&run, &folder.path, &not_known("--a\u{FFFD}b"));
}

#[test]
fn test_WB_INIT_25_a_word_after_init_that_is_not_utf_8_is_written_as_python_writes_it() {
    let folder = repository(None);
    let run = wb_os(&folder.path, &[b"init", b"a\xffb"]);
    assert_refused(&run, &folder.path, &not_known("a\\udcffb"));

    // The value of an option keeps its bytes too.
    let run = wb_os(&folder.path, &[b"init", b"--yes=\xfe"]);
    assert_refused(&run, &folder.path, &not_known("--yes=\\udcfe"));
}

#[test]
fn test_WB_INIT_25_an_option_at_the_top_that_is_not_utf_8_is_written_as_python_writes_it() {
    let folder = folder();
    let run = wb_os(&folder.path, &[b"-\xff"]);
    assert_refused(&run, &folder.path, &no_command("-\\udcff"));
}

// Round 1 of the review of #41: the same naming in every command, and a short option keeps its value.

#[test]
fn test_WB_INIT_42_revision_names_an_option_written_with_others_alone() {
    let folder = folder();
    let run = wb(&folder.path, &["revision", "-ab"], "");
    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stdout), "");
    assert_eq!(text(&run.stderr), format!("wb revision: '-a' is not known\n\n{}", USAGE));
}

#[test]
fn test_WB_INIT_41_an_unknown_short_option_is_named_with_its_value() {
    let folder = repository(None);
    let run = wb(&folder.path, &["init", "-x=1"], "");
    assert_refused(&run, &folder.path, &not_known("-x=1"));

    let run = wb(&folder.path, &["init", "-yx=1"], "");
    assert_refused(&run, &folder.path, &not_known("-x=1"));

    let run = wb(&folder.path, &["revision", "-y=x"], "");
    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stderr), format!("wb revision: '-y=x' is not known\n\n{}", USAGE));
}
