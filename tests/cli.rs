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
  wb prompt <role> [<issue>] [--issue <text>] [--change <text>]
            [--requirements <text>] [--round <text>] [--checkout <text>]
            [--branch <text>] [--issue_address <text>]
            [--change_address <text>] [--runner <name>] [--parts] [--stamp]
  wb start <role> [<issue>] [--runner <name>] [--prompt] [the options of prompt]
  wb revision
  wb accept <issue> [--no-commit]
  wb status [<issue>] [--stage <heading>]
  wb revert <issue> <role> [--round <n>]
  wb revert --change <n> <role> [--round <n>]
  wb answer <issue> [--suggested | <question> <answer>]
  wb answer --change <n> [--suggested | <question> <answer>]
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
            revision against the installed wb, and the project text for a
            placeholder the revision does not know. Ends with 0 when all of it
            holds, 1 when something differs. Writes nothing
  prompt    print the prompt of a role on standard output and nothing else: the
            role's text of the pinned revision with each placeholder filled
            from the option of its name, such as --issue for {issue}, then the
            project's own text. The issue is the word after the role, or
            --issue. What no option gives is taken from the repository and
            watchbill.yml: the checkout, the branch, the two addresses and
            the requirements; an option always wins. It runs the check of wb
            check first and prints nothing when that fails. --parts prints
            the path and the SHA-256 of each part, in the form sha256sum
            prints, in place of the text.
            --stamp prints, in place of the text, what the assignment is: the
            role, its model and effort, the runner, the version and revision,
            the issue, the change, the round, the branch, the checkout, the
            time, and each part as --parts gives it. Not with --parts.
            --runner names the runner the prompt is handed to. The breakdown
            needs it, and is handed out only to a runner that
            /etc/watchbill/organisation.yml clears for the breakdown up to the
            level of the repository, the line confidentiality: of
            watchbill.yml. Its stamp then has the model and effort of that
            file, and --parts adds the level and the runner. Ends with 0 when
            the prompt is printed, 1 when it is refused, 2 when it was used
            wrongly or the repository is not set up. Writes nothing
  start     start a session of one role, breakdown, tester, build, review or
            fix: the role is the word after start. The issue is needed by
            all but the review and the fix, which may be given --change
            and no issue. It checks what prompt checks, that a fix
            follows a review, that no classed code is in its way (classed: of
            watchbill.yml), that the runner has start in the organisation's
            file, and that standard input and output are a terminal; then say
            and post the stamp by issue_comment of watchbill.yml (by
            change_comment for a review or a fix of a change with no issue),
            and run the runner's start in the root of the checkout. The runner
            is --runner, or the organisation's default. The round and the
            change are worked out from issue_comments, change_comments and
            change_of_branch; --round and --change win. --prompt, --stamp and
            --parts print as prompt does, and post and start nothing. Ends with
            the session: 0 when it ended with 0, else 1; 1 when it is refused,
            2 when it was used wrongly or the repository is not set up
  revision  print the version of the package, the hash of its revision and the
            manifest: the hash of every file the revision holds. Writes nothing
  accept    accept the issue's file .watchbill/issues/<issue>.yml: read it,
            hold it to its form, refuse it while it holds a question, say what
            will be written, and write .watchbill/issues/<issue>.accepted with
            the SHA-256 of the file, the git user.name and user.email, and the
            time in UTC. An acceptance that is there is never written over.
            Then commit it alone, on the issue's branch, with nothing else:
            it refuses first when the checkout is not on that branch or the
            issue's file is not committed as it is. --no-commit only writes
            it and says the commit. Ends with 0 when it is accepted, 1 when it
            is refused, 2 when it was used wrongly or the repository is not
            set up
  status    list the issues in the flow under headings, as git status lists
            files: each issue under the last step it has done (Not broken
            down, Broken down, Accepted, Tested, Built, Reviewed, Fixed), and
            last the issues that have a question no [answers] or [answer] line
            has closed. An issue is in the flow when its branch, local or at
            origin, holds a commit that the default branch of origin does not:
            git alone says which, and fetches nothing. Its line shows the
            branch, the open change as #<n> (change_of_branch) and how many
            commits are not pushed. The comments come from issue_comments.
            <issue> prints that issue, in the flow or not, with its title
            (issue_address) and what it was worked out from; --stage <heading>
            prints that heading only, whatever its case. Ends with 0 when
            every issue could be read, 1 when a pattern failed for one (it is
            listed under Not read), 2 when it was used wrongly or the
            repository is not set up. Writes nothing
  revert    take an issue back to before a step, deleting nothing:
            wb revert <issue> <role> [--round <n>] posts, by issue_comment of
            watchbill.yml, one comment whose first line is
            [revert] <role> round <n>, for the last stamp of the role, or round
            <n>. From that stamp up to the marker no stamp, question or answer
            counts, for wb start <role>, wb prompt and wb status. wb revert
            --change <n> <role> posts it on a change with no issue, by
            change_comment.
            The branch, the issue's file and its acceptance, and the change are
            not touched: taking commits back is git's. The breakdown is taken
            back with git, not by a marker. Ends with 0 when the marker is
            posted, 1 when it was refused or the posting failed, 2 when it was
            used wrongly or the repository is not set up
  answer    answer the open questions of an issue: wb answer <issue> reads
            the whole comments by issue_comment_bodies of watchbill.yml, shows
            each open question in full, one at a time, and asks: Enter takes
            the suggestion, a number takes that choice, other text is the
            answer in words; a number that is not a choice is asked again.
            Then it says what it will post and posts one [answers] comment per
            role by issue_comment. Nothing is posted before every question is
            answered, and nothing after Ctrl-C. --suggested takes every
            suggestion and names the questions with none. <question> <answer>,
            as tester.q1 2 (q1 for the breakdown), answers one question without
            asking. --change <n> answers on a change with no issue, by
            change_comment_bodies and change_comment. It never changes the
            issue's file: where an answer does, it says so. Ends with 0 when
            the answers are posted or no question is open, 1 when it was
            refused, the input ended or a pattern failed, 2 when it was used
            wrongly or the repository is not set up
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
    assert_eq!(text(&run.stdout), text(&wb(&["help"]).stdout));
    assert!(text(&run.stdout).starts_with("usage: wb <command>"), "{}", text(&run.stdout));
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_INIT_35_help_and_h_do_as_dash_dash_help() {
    for word in ["help", "-h"] {
        let run = wb(&[word]);

        assert_eq!(run.status.code(), Some(0), "{}", word);
        assert_eq!(text(&run.stdout), text(&wb(&["help"]).stdout), "{}", word);
        assert!(text(&run.stdout).starts_with("usage: wb <command>"), "{}", word);
        assert_eq!(text(&run.stderr), "", "{}", word);
    }
}

#[test]
fn test_WB_INIT_25_no_command_ends_with_2_and_the_usage() {
    let run = wb(&[]);

    assert_eq!(run.status.code(), Some(2));
    assert_eq!(text(&run.stdout), text(&wb(&["help"]).stdout));
    assert!(text(&run.stdout).starts_with("usage: wb <command>"), "{}", text(&run.stdout));
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_INIT_25_a_command_that_is_not_known_ends_with_2_and_the_usage_on_standard_error() {
    for word in ["begin", "--force", "", "Help", "--version=1"] {
        let run = wb(&[word]);

        assert_eq!(run.status.code(), Some(2), "{:?}", word);
        assert_eq!(text(&run.stdout), "", "{:?}", word);
        assert_eq!(text(&run.stderr), format!("wb: no command '{}'\n\n{}", word, USAGE), "{:?}", word);
    }
}

#[test]
fn test_WB_INIT_25_only_the_first_word_is_the_command() {
    let run = wb(&["begin", "--version"]);

    assert_eq!(run.status.code(), Some(2));
    assert_eq!(text(&run.stderr), format!("wb: no command 'begin'\n\n{}", USAGE));

    // Words after a known option change nothing, as in the Python `wb`.
    let run = wb(&["--version", "begin"]);

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
