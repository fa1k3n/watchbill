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

/// The issues the tests of this file hand a role out for. Each has a file with no requirement, accepted
/// and committed (#97), as no role is handed out without. `a..b` has none since #323: an acceptance is a signed tag
/// (#235), which git cannot name for it, so it cannot be accepted; with no file it is handed out as it is (WB-TASK-17).
const ISSUES: [&str; 8] = ["1", "7", "12", "42", "54", "T-1", "T-1.a_b", "-x"];

/// A repository set up by `wb init --yes`, with the files of `ISSUES` accepted and committed.
fn set_up() -> Folder {
    let folder = repository(None);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    issues_ready(&folder.path, &ISSUES);
    folder
}

/// The text of a role of the package with each placeholder filled in.
fn filled(role: &str, values: &[(&str, &str)]) -> String {
    let mut text = fs::read_to_string(package().join("revision/roles").join(format!("{}.md", role))).unwrap();
    for (name, value) in values {
        text = text.replace(&format!("{{{}}}", name), value);
    }
    // A repository of `wb init --yes` has `classed: []`: the two placeholders of classed code are `none` (#227).
    text = text.replace("{classed}", "none").replace("{classed_touched}", "none");
    // `{description}` is the file of the change's description in the sandbox, where no option gives it (#151).
    text = text.replace("{description}", "/run/watchbill/description");
    // What `wb` brings in, and the files the session writes for it to post, where no option gives them (#153).
    text = text
        .replace("{brought}", "/run/watchbill/in")
        .replace("{questions}", "/run/watchbill/questions")
        .replace("{change_comment}", "/run/watchbill/change_comment")
        .replace("{needs_person}", "/run/watchbill/needs_person");
    // The four roles after the planner get the part of the issue's file after their text (#97).
    if ["tester", "build", "review", "fix"].contains(&role) {
        text = format!("{}\n{}", text, BARE_PART);
    }
    text
}

/// What the file of `BARE_ISSUE` adds to a prompt: the interface, and what the role may change and read.
const BARE_PART: &str = "## The interface\n\nnone\n\n## What you may change and read\n\nYou may change:\n\n- none\n\nYou may read, besides those:\n\n- none\n";

const ALL: [&str; 16] = [
    "--issue",
    "T-1",
    "--change",
    "34",
    "--requirements",
    "WB-X-1",
    "--round",
    "1",
    "--checkout",
    "/work/brakes",
    "--branch",
    "issue-1",
    "--issue_address",
    "tracker T-1",
    "--change_address",
    "server 34",
];
const ALL_VALUES: [(&str, &str); 8] = [
    ("issue", "T-1"),
    ("change", "34"),
    ("requirements", "WB-X-1"),
    ("round", "1"),
    ("checkout", "/work/brakes"),
    ("branch", "issue-1"),
    ("issue_address", "tracker T-1"),
    ("change_address", "server 34"),
];

/// Where the work is (#43): every role prompt names the checkout, the branch and the issue's address.
const WHERE: [&str; 6] = ["--checkout", "/work/brakes", "--branch", "issue-1", "--issue_address", "tracker T-1"];
const WHERE_VALUES: [(&str, &str); 3] = [("checkout", "/work/brakes"), ("branch", "issue-1"), ("issue_address", "tracker T-1")];

/// The words of a run with the options of where the work is added at the end.
fn with_where<'a>(args: &[&'a str]) -> Vec<&'a str> {
    args.iter().copied().chain(WHERE).collect()
}

/// A role of the package filled with the values given and those of where the work is.
fn filled_where(role: &str, values: &[(&str, &str)]) -> String {
    let all: Vec<(&str, &str)> = values.iter().copied().chain(WHERE_VALUES).collect();
    filled(role, &all)
}

/// Every plain file under a folder, with its bytes, sorted by path.
fn snapshot(folder: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &Path, at: &Path, found: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(base, &path, found);
            } else {
                let bytes = fs::read(&path).unwrap_or_else(|error| panic!("{} could not be read: {}", path.display(), error));
                found.push((path.strip_prefix(base).unwrap().to_str().unwrap().to_string(), bytes));
            }
        }
    }
    let mut found = Vec::new();
    walk(folder, folder, &mut found);
    found.sort();
    found
}

/// Fails, naming each path that was added, removed or changed between two snapshots, and how.
#[track_caller]
fn assert_same_snapshot(before: &[(String, Vec<u8>)], after: &[(String, Vec<u8>)], context: &str) {
    let mut differences = Vec::new();
    for (path, bytes) in before {
        match after.iter().find(|(other, _)| other == path) {
            None => differences.push(format!("removed: {}", path)),
            Some((_, now)) if now != bytes => {
                differences.push(format!("changed: {} ({} bytes before, {} after)", path, bytes.len(), now.len()))
            }
            Some(_) => {}
        }
    }
    for (path, _) in after {
        if !before.iter().any(|(other, _)| other == path) {
            differences.push(format!("added: {}", path));
        }
    }
    assert!(differences.is_empty(), "{}: the repository differs:\n{}", context, differences.join("\n"));
}

#[test]
fn test_WB_PROMPT_10_a_fresh_repository_prints_the_build_prompt_with_the_issue_filled_in() {
    let folder = set_up();

    let run = wb(&folder.path, &with_where(&["prompt", "build", "--issue", "12", "--requirements", "WB-X-1"]), "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), filled_where("build", &[("issue", "12"), ("requirements", "WB-X-1")]));
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_PROMPT_10_with_project_text_the_output_is_the_three_parts_in_order() {
    let folder = set_up();
    put_committed(&folder.path, ".watchbill/project/all.md", "Everyone: use make.\n");
    put_committed(&folder.path, ".watchbill/project/build.md", "Builders: run make test.\n");

    let run = wb(&folder.path, &with_where(&["prompt", "build", "--issue", "T-1", "--requirements", "WB-X-1"]), "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let role = filled_where("build", &[("issue", "T-1"), ("requirements", "WB-X-1")]);
    assert_eq!(
        text(&run.stdout),
        format!("{}\n{}\n\nEveryone: use make.\n\nBuilders: run make test.\n", role, FIXED_LINE)
    );
}

#[test]
fn test_WB_PROMPT_10_an_option_may_come_before_the_role() {
    let folder = set_up();

    let run = wb(&folder.path, &with_where(&["prompt", "--issue", "7", "tester", "--requirements", "R"]), "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), filled_where("tester", &[("issue", "7"), ("requirements", "R")]));
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
    commit_config(&folder.path);

    let run = wb(&folder.path, &["prompt", "build", "--issue", "1", "--requirements", "R"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    let err = text(&run.stderr);
    assert!(err.contains("revision/roles/build.md"), "{}", err);
}

#[test]
fn test_WB_PROMPT_12_a_changed_file_of_another_role_fails_the_check_too() {
    // The role asked for is as it was shipped: only the check of `wb check` can refuse this.
    let folder = set_up();
    let file = folder.path.join(".watchbill/revision/roles/review.md");
    let mut bytes = fs::read(&file).unwrap();
    bytes.insert(0, b'x');
    fs::write(&file, bytes).unwrap();
    commit_config(&folder.path);

    let run = wb(&folder.path, &["prompt", "build", "--issue", "1", "--requirements", "R"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("revision/roles/review.md"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_6_a_role_that_is_not_utf_8_is_a_role_the_revision_does_not_have() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let folder = set_up();

    let run = Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(["prompt"])
        .arg(OsStr::from_bytes(b"b\xffd"))
        .current_dir(&folder.path)
        .output()
        .unwrap();

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    let err = text(&run.stderr);
    for role in ["build", "fix", "review", "tester"] {
        assert!(err.contains(role), "{}: {}", role, err);
    }
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
    commit_config(&folder.path);

    let run = wb(&folder.path, &["prompt", "build", "--issue", "1", "--requirements", "R"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("0.0.1"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_13_no_watchbill_folder_ends_with_2_and_prints_nothing() {
    let folder = repository(None);

    let run = wb(&folder.path, &["prompt", "build", "--issue", "1"], "");

    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("not set up"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_13_a_folder_that_is_not_in_a_git_repository_ends_with_2() {
    let folder = folder();

    let run = wb(&folder.path, &["prompt", "build", "--issue", "1"], "");

    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).starts_with("wb prompt: "), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("git repository"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_14_a_placeholder_the_prompt_uses_with_no_option_is_refused_by_name() {
    let folder = set_up();
    // No role's text names an address since #153: the project's own text of the build does here.
    put_committed(&folder.path, ".watchbill/project/build.md", "Read the issue at {issue_address}.\n");

    // `requirements` has its default text (#72), and this repository has no pattern for the address.
    let run = wb(&folder.path, &["prompt", "build", "--issue", "12", "--branch", "issue-12"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("{issue_address}"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_14_an_issue_and_a_branch_alone_are_refused_for_a_placeholder_that_is_used() {
    // Was test_WB_PROMPT_14_no_option_at_all_is_refused_for_a_placeholder_that_is_used. Since #116 the
    // build with no issue is used wrongly; with one, this repository has no pattern for its address.
    let folder = set_up();
    // No role's text names an address since #153: the project's own text of the build does here.
    put_committed(&folder.path, ".watchbill/project/build.md", "Read the issue at {issue_address}.\n");

    let run = wb(&folder.path, &["prompt", "build", "1", "--branch", "b"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("got no value"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_6_a_role_the_revision_does_not_have_is_refused_with_the_roles_it_has() {
    let folder = set_up();

    let run = wb(&folder.path, &["prompt", "captain", "--issue", "1"], "");

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
        vec!["prompt", "build", "--colour", "x"],
        vec!["prompt", "build", "--colour=x"],
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
fn test_WB_PROMPT_15_a_missing_role_a_third_word_and_a_repeated_option_end_with_2_and_the_usage() {
    let folder = set_up();

    for args in [
        vec!["prompt"],
        vec!["prompt", "--issue", "1"],
        vec!["prompt", "build", "1", "2"],
        vec!["prompt", "build", "--issue", "1", "--issue", "2"],
        vec!["prompt", "build", "--issue"],
        vec!["prompt", "build", "--parts=yes"],
    ] {
        let run = wb(&folder.path, &args, "");

        assert_eq!(code(&run), Some(2), "{:?}", args);
        assert_eq!(text(&run.stdout), "", "{:?}", args);
        assert!(text(&run.stderr).starts_with("wb prompt: "), "{:?}: {}", args, text(&run.stderr));
        assert!(text(&run.stderr).ends_with(&format!("\n\n{}", USAGE)), "{:?}: {}", args, text(&run.stderr));
    }
}

#[test]
fn test_WB_PROMPT_15_an_empty_value_is_refused_by_name_and_ends_with_2() {
    let folder = set_up();

    for (args, name) in [
        (vec!["prompt", "build", "--issue=", "--requirements", "R"], "--issue"),
        (vec!["prompt", "build", "--issue", "", "--requirements", "R"], "--issue"),
        (vec!["prompt", "build", "--issue", "1", "--requirements", ""], "--requirements"),
    ] {
        let run = wb(&folder.path, &args, "");

        assert_eq!(code(&run), Some(2), "{:?}", args);
        assert_eq!(text(&run.stdout), "", "{:?}", args);
        let err = text(&run.stderr);
        assert!(err.starts_with("wb prompt: ") && err.contains(&format!("'{}'", name)), "{:?}: {}", args, err);
        assert!(err.ends_with(&format!("\n\n{}", USAGE)), "{:?}: {}", args, err);
    }
}

#[test]
fn test_WB_PROMPT_15_the_usage_names_wb_prompt() {
    let run = wb(&folder().path, &["help", "prompt"], "");

    assert!(text(&run.stdout).contains("wb prompt <role>"), "{}", text(&run.stdout));
    assert!(text(&run.stdout).contains("--parts"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_PROMPT_11_parts_prints_one_line_per_part_and_sha256sum_passes_at_the_root() {
    let folder = set_up();
    put_committed(&folder.path, ".watchbill/project/all.md", "Everyone: use make.\n");
    put_committed(&folder.path, ".watchbill/project/build.md", "Builders: run make test.\n");

    let mut args = vec!["prompt", "build", "--parts"];
    args.extend(ALL);
    let run = wb(&folder.path, &args, "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    let paths: Vec<&str> = out.lines().map(|line| line.split_once("  ").unwrap().1).collect();
    assert_eq!(
        paths,
        [".watchbill/revision/roles/build.md", ".watchbill/issues/T-1.yml", ".watchbill/project/all.md", ".watchbill/project/build.md"]
    );
    assert!(!out.contains("Everyone"), "{}", out);
    let sums = folder.path.join("sums.txt");
    fs::write(&sums, &out).unwrap();
    let check = Command::new("sha256sum").arg("-c").arg(&sums).current_dir(&folder.path).output().unwrap();
    assert!(check.status.success(), "{}{}", text(&check.stdout), text(&check.stderr));
}

#[test]
fn test_WB_PROMPT_11_with_no_project_text_parts_prints_the_line_of_the_role_and_of_the_issues_file() {
    let folder = set_up();

    // The tester with no issue is used wrongly (#116): refused, nothing printed.
    let run = wb(&folder.path, &["prompt", "tester", "--parts"], "");
    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stdout), "");

    let mut args = vec!["prompt", "tester", "--parts"];
    args.extend(ALL);
    let run = wb(&folder.path, &args, "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    // The role's text, then the issue's file (#97): no role is handed out without one.
    assert_eq!(text(&run.stdout).lines().count(), 2);
}

#[test]
fn test_WB_PROMPT_16_a_value_that_starts_with_a_dash_or_holds_a_line_break_arrives_as_given() {
    let folder = set_up();

    for (args, issue, requirements) in [
        (vec!["prompt", "build", "--issue", "-x", "--requirements", "R"], "-x", "R"),
        (vec!["prompt", "build", "--issue=-x", "--requirements=-y"], "-x", "-y"),
        (vec!["prompt", "build", "--issue", "1", "--requirements", "a\nb\n\nc"], "1", "a\nb\n\nc"),
        (vec!["prompt", "build", "--issue", "1", "--requirements", "{round}{issue}"], "1", "{round}{issue}"),
    ] {
        let run = wb(&folder.path, &with_where(&args), "");

        assert_eq!(code(&run), Some(0), "{:?}: {}", args, text(&run.stderr));
        assert_eq!(text(&run.stdout), filled_where("build", &[("issue", issue), ("requirements", requirements)]), "{:?}", args);
    }
    // A value that is no name of an issue has no file. From #97 no role was handed out for it; since #116
    // an issue with no file is handed out, and the value arrives as it was given, with no part of a file.
    for issue in ["--", "{change}", "one\ntwo"] {
        let args = with_where(&["prompt", "build", "--issue", issue, "--requirements", "R"]);
        let run = wb(&folder.path, &args, "");

        assert_eq!(code(&run), Some(0), "{:?}: {}", args, text(&run.stderr));
        let whole = filled_where("build", &[("issue", issue), ("requirements", "R")]);
        let expected = whole.strip_suffix(&format!("\n{}", BARE_PART)).unwrap();
        assert_eq!(text(&run.stdout), expected, "{:?}", args);
    }
}

#[test]
fn test_WB_PROMPT_17_a_run_changes_nothing_under_the_repository() {
    let folder = set_up();
    put_committed(&folder.path, ".watchbill/project/all.md", "Everyone: use make.\n");
    let before = snapshot(&folder.path);

    for (args, status) in [
        (with_where(&["prompt", "build", "--issue", "1", "--requirements", "R"]), 0),
        (with_where(&["prompt", "build", "--parts", "--issue", "1", "--requirements", "R"]), 0),
        // The build with no issue is used wrongly since #116.
        (vec!["prompt", "build"], 2),
        (vec!["prompt", "nobody"], 1),
    ] {
        let run = wb(&folder.path, &args, "");
        assert_eq!(code(&run), Some(status), "{:?}: {}", args, text(&run.stderr));
        assert_same_snapshot(&before, &snapshot(&folder.path), &format!("{:?}", args));
    }
}

/// The placeholders of the list of the package, `- `{name}`: ` lines, in order.
fn listed() -> Vec<String> {
    let text = fs::read_to_string(package().join("revision/roles/placeholders.md")).unwrap();
    text.lines()
        .filter_map(|line| line.strip_prefix("- `{"))
        .filter_map(|rest| rest.split_once("}`").map(|(name, _)| name.to_string()))
        .collect()
}

#[test]
fn test_WB_PROMPT_18_the_options_are_the_placeholders_of_the_list() {
    let mut options: Vec<String> = watchbill::prompt::PLACEHOLDERS.iter().map(|name| name.to_string()).collect();
    let mut names = listed();
    options.sort();
    names.sort();
    assert_eq!(options, names);
}

#[test]
fn test_WB_PROMPT_18_an_option_of_where_the_work_is_is_taken_and_filled_in() {
    let folder = set_up();
    // No role's text names an address since #153: the project's own text of the review does here.
    put_committed(&folder.path, ".watchbill/project/review.md", "The issue is at {issue_address}, the change at {change_address}.\n");

    for name in ["checkout", "branch", "issue_address", "change_address"] {
        // Every option of ALL, with a value of its own for the one under test.
        let option = format!("--{}", name);
        let value = format!("the value of {}", name);
        let mut args = vec!["prompt", "review"];
        for pair in ALL.chunks(2) {
            args.push(pair[0]);
            args.push(if pair[0] == option { value.as_str() } else { pair[1] });
        }

        let run = wb(&folder.path, &args, "");

        assert_eq!(code(&run), Some(0), "{}: {}", name, text(&run.stderr));
        assert!(text(&run.stdout).contains(&value), "{}: {}", name, text(&run.stdout));
    }
}

// ---- `wb prompt <role> --stamp` (#68) ----

// The line `ran:` follows `model:` (#336).
const STAMP_NAMES: [&str; 14] =
    ["role", "model", "ran", "effort", "runner", "version", "revision", "issue", "change", "round", "branch", "checkout", "time", "view"];

const ROLES: [&str; 4] = ["build", "fix", "review", "tester"];

/// Seconds since 1970 of a time in the form `2026-11-17T18:04:05Z`, or `None` when it has another form.
fn seconds_of(time: &str) -> Option<u64> {
    let b = time.as_bytes();
    let shape = "dddd-dd-ddTdd:dd:ddZ";
    if b.len() != shape.len() || !shape.bytes().zip(b).all(|(s, c)| if s == b'd' { c.is_ascii_digit() } else { s == *c }) {
        return None;
    }
    let n = |from: usize, to: usize| time[from..to].parse::<i64>().unwrap();
    let (y, m, d) = (n(0, 4), n(5, 7), n(8, 10));
    // Days from the civil date, as Howard Hinnant gives them.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some((days * 86400 + n(11, 13) * 3600 + n(14, 16) * 60 + n(17, 19)) as u64)
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

/// The lines of a stamp as (name, value): split at the first `: `, or a `:` that ends the line.
fn fields(stamp: &str) -> Vec<(String, String)> {
    stamp
        .lines()
        .map(|line| match line.split_once(": ") {
            Some((name, value)) => (name.to_string(), value.to_string()),
            None => (line.strip_suffix(':').expect(line).to_string(), String::new()),
        })
        .collect()
}

fn stamp_of(folder: &Path, role: &str, extra: &[&str]) -> std::process::Output {
    let mut args = vec!["prompt", role, "--stamp"];
    args.extend(extra);
    wb(folder, &args, "")
}

fn value<'a>(found: &'a [(String, String)], name: &str) -> &'a str {
    &found.iter().find(|(given, _)| given == name).unwrap().1
}

#[test]
fn test_WB_PROMPT_19_a_stamp_is_the_thirteen_lines_in_order_and_the_parts_for_each_role() {
    let folder = set_up();
    put_committed(&folder.path, ".watchbill/project/all.md", "Everyone: use make.\n");

    for role in ROLES {
        let run = stamp_of(&folder.path, role, &ALL);

        assert_eq!(code(&run), Some(0), "{}: {}", role, text(&run.stderr));
        assert_eq!(text(&run.stderr), "", "{}", role);
        let out = text(&run.stdout);
        let mut found = fields(&out);
        // The line `level:` follows `view:` for a role that runs in a view (#209); the lines after it are as before.
        let has_level = ["tester", "build", "review", "fix"].contains(&role);
        assert_eq!(found.iter().any(|(name, _)| name == "level"), has_level, "{}: {}", role, out);
        found.retain(|(name, _)| name != "level");
        let names: Vec<&str> = found.iter().take(14).map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, STAMP_NAMES, "{}", role);
        assert!(found.len() > 14, "{}: {}", role, out);
        // The line `requirements: given` stands before the parts, as the file of the issue is used (#97).
        assert_eq!(found[14], ("requirements".to_string(), "given".to_string()), "{}: {}", role, out);
        // The lines `job:` and `base:` follow it, before the parts (#236).
        assert_eq!(found[15].0, "job", "{}: {}", role, out);
        assert_eq!(found[16].0, "base", "{}: {}", role, out);
        assert!(found.iter().skip(17).all(|(name, _)| name == "part"), "{}: {}", role, out);
        assert!(out.ends_with('\n'), "{}", role);
        assert_eq!(value(&found, "role"), role);
        // A value the prompt does not hold is written empty: it was not handed out.
        let role_text = fs::read_to_string(package().join("revision/roles").join(format!("{}.md", role))).unwrap();
        for name in ["issue", "change", "round", "branch", "checkout"] {
            let given = ALL_VALUES.iter().find(|(n, _)| *n == name).unwrap().1;
            // The round is written where it is given, whatever the text holds (#136).
            let expected = if name == "round" || role_text.contains(&format!("{{{}}}", name)) { given } else { "" };
            assert_eq!(value(&found, name), expected, "{} {}", role, name);
        }
        assert_eq!(value(&found, "runner"), "", "{}", role);
        // With no runner given, what ran is not known (#336).
        assert_eq!(value(&found, "ran"), "unknown", "{}", role);
        // The prompt itself is not in the stamp.
        assert!(!out.contains("Everyone"), "{}", role);
    }
}

#[test]
fn test_WB_PROMPT_19_version_and_revision_are_those_of_the_pinned_revision() {
    let folder = set_up();
    let shown = text(&wb(&folder.path, &["revision"], "").stdout);
    let hash = shown.lines().nth(1).unwrap().strip_prefix("revision ").unwrap().to_string();

    let run = stamp_of(&folder.path, "build", &ALL);

    let found = fields(&text(&run.stdout));
    assert_eq!(value(&found, "version"), env!("CARGO_PKG_VERSION"));
    assert_eq!(value(&found, "revision"), hash);
}

#[test]
fn test_WB_PROMPT_19_time_is_utc_in_its_form_and_between_before_and_after_the_run() {
    let folder = set_up();

    let before = now();
    let run = stamp_of(&folder.path, "build", &ALL);
    let after = now();

    let found = fields(&text(&run.stdout));
    let time = value(&found, "time");
    let seconds = seconds_of(time).unwrap_or_else(|| panic!("not of the form YYYY-MM-DDTHH:MM:SSZ: {}", time));
    assert!(before <= seconds && seconds <= after, "{} not between {} and {}", time, before, after);
}

#[test]
fn test_WB_PROMPT_19_a_stamped_value_that_got_no_option_is_written_empty_with_no_trailing_space() {
    let folder = set_up();

    // The build prompt uses {issue}, {requirements} and the places of the work; nothing else is given.
    let mut args = vec!["prompt", "build", "--stamp"];
    args.extend(with_where(&["--issue", "12", "--requirements", "R"]));
    let run = wb(&folder.path, &args, "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    for name in ["runner", "change", "round"] {
        assert!(out.lines().any(|line| line == format!("{}:", name)), "{}: {}", name, out);
    }
    assert!(out.lines().all(|line| !line.ends_with(' ')), "{}", out);
}

#[test]
fn test_WB_PROMPT_14_a_stamp_is_refused_for_a_placeholder_the_prompt_uses_with_no_value() {
    let folder = set_up();
    // No role's text names an address since #153: the project's own text of the build does here.
    put_committed(&folder.path, ".watchbill/project/build.md", "Read the issue at {issue_address}.\n");

    // Since #116 the build with no issue is used wrongly, so the issue is given and the address is not.
    let run = stamp_of(&folder.path, "build", &["1", "--branch", "b"]);

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("got no value"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_20_model_and_effort_are_those_of_the_table_read_through_the_loader() {
    let folder = set_up();
    let table =
        watchbill::rules::load(package().join("revision/rules.yml"), package().join("revision/models.yml")).unwrap();

    for role in ROLES {
        let run = stamp_of(&folder.path, role, &ALL);

        assert_eq!(code(&run), Some(0), "{}: {}", role, text(&run.stderr));
        let found = fields(&text(&run.stdout));
        let row = &table.models[role];
        assert_eq!(value(&found, "model"), row.model, "{}", role);
        assert_eq!(value(&found, "effort"), row.effort, "{}", role);
    }
}

#[test]
fn test_WB_PROMPT_21_the_part_lines_are_the_lines_of_parts_in_order() {
    let folder = set_up();
    put_committed(&folder.path, ".watchbill/project/all.md", "Everyone: use make.\n");
    put_committed(&folder.path, ".watchbill/project/review.md", "Reviewers: read twice.\n");
    let mut args = vec!["prompt", "review", "--parts"];
    args.extend(ALL);
    let parts = text(&wb(&folder.path, &args, "").stdout);

    let run = stamp_of(&folder.path, "review", &ALL);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    let lines: Vec<String> = out.lines().filter(|line| line.starts_with("part: ")).map(|line| line.strip_prefix("part: ").unwrap().to_string()).collect();
    let expected: Vec<String> = parts.lines().map(|line| line.to_string()).collect();
    assert_eq!(lines.len(), 4);
    assert_eq!(lines, expected);
    // `sha256sum` takes them with `part: ` taken off.
    let sums = folder.path.join("sums.txt");
    fs::write(&sums, lines.join("\n") + "\n").unwrap();
    let check = Command::new("sha256sum").arg("-c").arg(&sums).current_dir(&folder.path).output().unwrap();
    assert!(check.status.success(), "{}{}", text(&check.stdout), text(&check.stderr));
}

#[test]
fn test_WB_PROMPT_22_a_stamped_value_with_a_line_break_is_refused_by_name_with_2() {
    let folder = set_up();

    for name in ["issue", "change", "round", "branch", "checkout"] {
        for bad in [
            "a\nb", "a\rb", "a\x0bb", "a\x0cb", "a\u{85}b", "a\u{2028}b", "a\u{2029}b", "a\x1bb", "a\tb", " a",
            "a ", "   ", "\u{a0}a", "a\u{3000}",
        ] {
            let mut args = vec!["prompt", "build", "--stamp"];
            for pair in ALL.chunks(2) {
                args.push(pair[0]);
                args.push(if pair[0] == format!("--{}", name) { bad } else { pair[1] });
            }

            let run = wb(&folder.path, &args, "");

            assert_eq!(code(&run), Some(2), "{} {:?}", name, bad);
            assert_eq!(text(&run.stdout), "", "{} {:?}", name, bad);
            assert!(text(&run.stderr).contains(&format!("--{}", name)), "{}: {}", name, text(&run.stderr));
            assert!(text(&run.stderr).contains("one line"), "{}: {}", name, text(&run.stderr));
        }
    }
}

#[test]
fn test_WB_PROMPT_22_a_value_the_stamp_does_not_print_may_still_hold_a_line_break() {
    let folder = set_up();
    let mut args = vec!["prompt", "build", "--stamp"];
    args.extend(with_where(&["--issue", "1", "--requirements", "a\nb"]));

    let run = wb(&folder.path, &args, "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(!text(&run.stdout).contains("a\nb"));
}

#[test]
fn test_WB_PROMPT_23_stamp_with_parts_ends_with_2_and_prints_nothing() {
    let folder = set_up();

    for first in [["--stamp", "--parts"], ["--parts", "--stamp"]] {
        let mut args = vec!["prompt", "build"];
        args.extend(first);
        args.extend(ALL);

        let run = wb(&folder.path, &args, "");

        assert_eq!(code(&run), Some(2), "{:?}", args);
        assert_eq!(text(&run.stdout), "", "{:?}", args);
        assert!(text(&run.stderr).contains("--stamp"), "{}", text(&run.stderr));
        assert!(!text(&run.stderr).contains("is not known"), "{}", text(&run.stderr));
        assert!(text(&run.stderr).contains("--parts"), "{}", text(&run.stderr));
    }
}

#[test]
fn test_WB_PROMPT_24_one_changed_byte_of_a_prompt_ends_with_1_prints_nothing_and_names_the_file() {
    let folder = set_up();
    let path = folder.path.join(".watchbill/revision/roles/build.md");
    let mut bytes = fs::read(&path).unwrap();
    bytes.push(b'x');
    fs::write(&path, bytes).unwrap();

    let run = stamp_of(&folder.path, "build", &ALL);

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("roles/build.md"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_24_a_repository_that_is_not_set_up_ends_with_2_and_prints_nothing() {
    let folder = repository(None);

    let run = stamp_of(&folder.path, "build", &ALL);

    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("not set up"), "{}", text(&run.stderr));
    assert!(!text(&run.stderr).contains("is not known"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_17_a_stamp_changes_nothing_under_the_repository() {
    let folder = set_up();
    let before = snapshot(&folder.path);

    let run = stamp_of(&folder.path, "build", &ALL);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_same_snapshot(&before, &snapshot(&folder.path), "a stamp");
}

#[test]
fn test_WB_PROMPT_19_the_usage_names_stamp() {
    let run = wb(&folder().path, &["help", "prompt"], "");

    assert!(text(&run.stdout).contains("[--stamp]"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_PROMPT_19_a_value_the_prompt_does_not_use_is_written_empty_but_the_round() {
    let folder = set_up();
    let mut args = vec!["prompt", "build", "--stamp", "--change", "5", "--round", "2"];
    args.extend(with_where(&["--issue", "1", "--requirements", "R"]));

    let run = wb(&folder.path, &args, "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.lines().any(|line| line == "change:"), "{}", out);
    assert!(out.lines().any(|line| line == "round: 2"), "{}", out);
}

#[test]
fn test_WB_PROMPT_81_with_no_round_given_or_worked_out_the_round_line_is_empty() {
    let folder = set_up();
    let mut args = vec!["prompt", "build", "--stamp"];
    args.extend(with_where(&["--issue", "1", "--requirements", "R"]));

    let run = wb(&folder.path, &args, "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).lines().any(|line| line == "round:"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_PROMPT_22_a_stamped_value_may_hold_white_space_inside() {
    let folder = set_up();
    let mut args = vec!["prompt", "build", "--stamp", "--requirements", "R", "--checkout", "/work/my brakes"];
    args.extend(["--issue", "T-1", "--branch", "issue 1", "--issue_address", "x"]);

    let run = wb(&folder.path, &args, "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("\nbranch: issue 1\n"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_PROMPT_23_stamp_or_parts_given_twice_ends_with_2_and_prints_nothing() {
    let folder = set_up();

    for flag in ["--stamp", "--parts"] {
        let mut args = vec!["prompt", "build", flag, flag];
        args.extend(ALL);

        let run = wb(&folder.path, &args, "");

        assert_eq!(code(&run), Some(2), "{}", flag);
        assert_eq!(text(&run.stdout), "", "{}", flag);
        assert!(text(&run.stderr).contains(&format!("'{}' is given twice", flag)), "{}", text(&run.stderr));
    }
}

// ---- what `wb prompt` asks only for what it cannot know (#72) ----

const GH: &str = "git@github.com:example/brake-controller.git";

/// A repository with a GitHub `origin`, set up by `wb init --yes`.
fn set_up_github() -> Folder {
    let folder = repository(Some(GH));
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    issues_ready(&folder.path, &ISSUES);
    folder
}

/// Local branches of the given names, on one empty commit.
fn branches(folder: &Path, names: &[&str]) {
    git(folder, &["-c", "user.name=t", "-c", "user.email=t@example.org", "commit", "-q", "--allow-empty", "-m", "x"]);
    for name in names {
        git(folder, &["branch", name]);
    }
}

fn root_of(folder: &Path) -> String {
    fs::canonicalize(folder).unwrap().to_str().unwrap().to_string()
}

const ISSUE_ADDRESS: &str = "gh issue view 54 --repo example/brake-controller; gh issue view 54 --repo example/brake-controller --comments";
const CHANGE_ADDRESS: &str = "gh pr view 168 --repo example/brake-controller; gh pr view 168 --repo example/brake-controller --comments";
const REQUIREMENTS: &str = "none: the issue's file holds no requirement";

/// The values of `wb prompt review 54 --change 168 --round 1` with one local branch `issue-54-serve`.
fn derived(folder: &Path) -> Vec<(&'static str, String)> {
    vec![
        ("issue", "54".to_string()),
        ("change", "168".to_string()),
        ("round", "1".to_string()),
        ("requirements", REQUIREMENTS.to_string()),
        ("checkout", root_of(folder)),
        ("branch", "issue-54-serve".to_string()),
        ("issue_address", ISSUE_ADDRESS.to_string()),
        ("change_address", CHANGE_ADDRESS.to_string()),
    ]
}

fn filled_with(role: &str, values: &[(&str, String)]) -> String {
    let all: Vec<(&str, &str)> = values.iter().map(|(name, value)| (*name, value.as_str())).collect();
    filled(role, &all)
}

#[test]
fn test_WB_PROMPT_31_the_issue_is_the_word_after_the_role_and_the_tester_needs_no_other_option() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-54"]);

    let run = wb(&folder.path, &["prompt", "tester", "54"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let values = vec![
        ("issue", "54".to_string()),
        ("requirements", REQUIREMENTS.to_string()),
        ("checkout", root_of(&folder.path)),
        ("branch", "issue-54".to_string()),
        ("issue_address", ISSUE_ADDRESS.to_string()),
    ];
    assert_eq!(text(&run.stdout), filled_with("tester", &values));
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_PROMPT_31_the_issue_word_may_stand_after_an_option() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-54-serve"]);

    let run = wb(&folder.path, &["prompt", "review", "--change", "168", "54", "--round", "1"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), filled_with("review", &derived(&folder.path)));
}

#[test]
fn test_WB_PROMPT_31_the_option_issue_still_works_and_is_the_same_as_the_word() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-54-serve"]);

    let by_word = wb(&folder.path, &["prompt", "review", "54", "--change", "168", "--round", "1"], "");
    let by_option = wb(&folder.path, &["prompt", "review", "--issue", "54", "--change", "168", "--round", "1"], "");

    assert_eq!(code(&by_option), Some(0), "{}", text(&by_option.stderr));
    assert_eq!(text(&by_option.stdout), text(&by_word.stdout));
}

#[test]
fn test_WB_PROMPT_31_an_issue_word_with_the_option_issue_ends_with_2_and_prints_nothing() {
    let folder = set_up_github();

    for args in [
        vec!["prompt", "tester", "54", "--issue", "54"],
        vec!["prompt", "tester", "54", "--issue", "55"],
        vec!["prompt", "tester", "--issue", "54", "54"],
    ] {
        let run = wb(&folder.path, &args, "");

        assert_eq!(code(&run), Some(2), "{:?}", args);
        assert_eq!(text(&run.stdout), "", "{:?}", args);
        assert!(text(&run.stderr).contains("--issue"), "{:?}: {}", args, text(&run.stderr));
        assert!(text(&run.stderr).ends_with(&format!("\n\n{}", USAGE)), "{:?}", args);
    }
}

#[test]
fn test_WB_PROMPT_31_an_empty_issue_word_ends_with_2() {
    let folder = set_up_github();

    let run = wb(&folder.path, &["prompt", "tester", ""], "");

    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stdout), "");
}

#[test]
fn test_WB_PROMPT_32_to_35_review_with_one_local_branch_fills_the_branch_the_addresses_and_the_rest() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-54-serve"]);

    let run = wb(&folder.path, &["prompt", "review", "54", "--change", "168", "--round", "1"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), filled_with("review", &derived(&folder.path)));
}

#[test]
fn test_WB_PROMPT_32_the_checkout_is_the_root_when_run_in_a_folder_inside() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-54"]);
    fs::create_dir_all(folder.path.join("src/deep")).unwrap();

    let run = wb(&folder.path.join("src/deep"), &["prompt", "tester", "54"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains(&format!("the checkout {},", root_of(&folder.path))), "{}", text(&run.stdout));
}

#[test]
fn test_WB_PROMPT_49_a_branch_that_is_the_pattern_alone_is_found_and_issue_540_is_not_issue_54() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-54", "issue-540", "issue-5"]);

    let run = wb(&folder.path, &["prompt", "build", "54"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("on the branch issue-54."), "{}", text(&run.stdout));
}

#[test]
fn test_WB_PROMPT_49_two_local_branches_are_refused_with_both_named_and_nothing_on_standard_output() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-54-a", "issue-54-b"]);

    for role in ["build", "review", "fix"] {
        let run = wb(&folder.path, &["prompt", role, "54", "--change", "168", "--round", "1"], "");

        assert_eq!(code(&run), Some(1), "{}", role);
        assert_eq!(text(&run.stdout), "", "{}", role);
        let err = text(&run.stderr);
        assert!(err.contains("issue-54-a") && err.contains("issue-54-b") && err.contains("--branch"), "{}: {}", role, err);
    }
}

#[test]
fn test_WB_PROMPT_49_no_local_branch_is_refused_the_same_way() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-6-other"]);

    let run = wb(&folder.path, &["prompt", "build", "54"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    let err = text(&run.stderr);
    assert!(err.contains("issue-54") && err.contains("--branch"), "{}", err);
}

#[test]
fn test_WB_PROMPT_25_requirements_with_no_option_are_those_of_the_issues_file() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-54-x"]);

    let run = wb(&folder.path, &["prompt", "build", "54"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("the requirements none: the issue's file holds no requirement"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_PROMPT_36_an_option_given_wins_over_each_derived_value() {
    let folder = set_up_github();
    // Two branches match, so the branch is only found when it is given.
    branches(&folder.path, &["issue-54-serve", "issue-54-other"]);

    for name in ["requirements", "checkout", "branch", "issue_address", "change_address"] {
        let value = format!("the value of {}", name);
        let option = format!("--{}", name);
        let mut values = derived(&folder.path);
        for (given, own) in values.iter_mut() {
            if *given == name {
                *own = value.clone();
            }
        }
        let mut args = vec!["prompt", "review", "54", "--change", "168", "--round", "1"];
        if name != "branch" {
            args.extend(["--branch", "issue-54-serve"]);
        }
        args.push(option.as_str());
        args.push(value.as_str());

        let run = wb(&folder.path, &args, "");

        assert_eq!(code(&run), Some(0), "{}: {}", name, text(&run.stderr));
        assert_eq!(text(&run.stdout), filled_with("review", &values), "{}", name);
    }
}

#[test]
fn test_WB_PROMPT_36_an_issue_option_in_place_of_the_word_and_a_given_change_are_used_as_given() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-54-serve"]);
    // No role's text names an address since #153: the project's own text of the review does here.
    put_committed(&folder.path, ".watchbill/project/review.md", "The change is at {change_address}.\n");

    let run = wb(&folder.path, &["prompt", "review", "--issue", "54", "--change", "9", "--round", "2"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("gh pr view 9 --repo example/brake-controller; gh pr view 9 --repo example/brake-controller --comments"), "{}", text(&run.stdout));
}

/// A repository set up before #72: the lines of the patterns are not in `watchbill.yml`.
fn without_patterns() -> Folder {
    let folder = set_up_github();
    let file = config(&folder.path);
    let kept: String = read(&file)
        .lines()
        .filter(|line| !["issue_address:", "change_address:", "branch:"].iter().any(|key| line.starts_with(key)))
        .map(|line| format!("{}\n", line))
        .collect();
    fs::write(&file, kept).unwrap();
    commit_config(&folder.path);
    folder
}

#[test]
fn test_WB_PROMPT_37_a_watchbill_yml_with_no_patterns_needs_the_options_as_before_and_wb_check_holds() {
    let folder = without_patterns();
    branches(&folder.path, &["issue-54-serve"]);

    assert_eq!(code(&wb(&folder.path, &["check"], "")), Some(0));
    let refused = wb(&folder.path, &["prompt", "tester", "54"], "");
    assert_eq!(code(&refused), Some(1));
    assert_eq!(text(&refused.stdout), "");
    assert!(text(&refused.stderr).contains("got no value"), "{}", text(&refused.stderr));

    let run = wb(&folder.path, &["prompt", "tester", "54", "--branch", "b-1", "--issue_address", "at 54"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let values = vec![
        ("issue", "54".to_string()),
        ("requirements", REQUIREMENTS.to_string()),
        ("checkout", root_of(&folder.path)),
        ("branch", "b-1".to_string()),
        ("issue_address", "at 54".to_string()),
    ];
    assert_eq!(text(&run.stdout), filled_with("tester", &values));
}

#[test]
fn test_WB_PROMPT_37_a_pattern_that_is_not_a_text_is_refused_by_the_loader() {
    let folder = set_up_github();
    let file = config(&folder.path);
    let changed = read(&file).replace("branch: 'issue-{issue}'", "branch:\n  - a");
    fs::write(&file, changed).unwrap();
    commit_config(&folder.path);

    let run = wb(&folder.path, &["prompt", "tester", "54"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("branch"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_38_the_stamp_holds_the_derived_values() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-54-serve"]);

    let run = wb(&folder.path, &["prompt", "review", "54", "--change", "168", "--round", "1", "--stamp"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let found = fields(&text(&run.stdout));
    assert_eq!(value(&found, "issue"), "54");
    assert_eq!(value(&found, "change"), "168");
    assert_eq!(value(&found, "round"), "1");
    assert_eq!(value(&found, "branch"), "issue-54-serve");
    assert_eq!(value(&found, "checkout"), root_of(&folder.path));
}

#[test]
fn test_WB_PROMPT_38_the_stamp_is_refused_with_1_for_a_derived_checkout_that_is_not_one_plain_line() {
    let folder = repository_named("brakes\nx", Some(GH));
    assert_eq!(code(&wb(&folder.path, &["init", "--yes"], "")), Some(0));
    issues_ready(&folder.path, &["54"]);
    branches(&folder.path, &["issue-54"]);

    let run = wb(&folder.path, &["prompt", "tester", "54", "--stamp"], "");

    // Nothing was used wrongly, and the caller gave no option for it: refused, with where it came from.
    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    let err = text(&run.stderr);
    assert!(err.contains("checkout") && err.contains("root of the repository"), "{}", err);
    assert!(!err.contains("'--checkout'"), "{}", err);
}

/// Values that carry shell, a path or a pattern of a glob, or that git or `gh` would take as more than a name.
const NOT_PLAIN: [&str; 12] =
    ["54; rm -rf ~", "$(x)", "`x`", "5 && x", "..", "5/6", "5*", "a..b", ".hidden", "5.", "-x", "a.lock"];

#[test]
fn test_WB_PROMPT_39_an_issue_that_fills_a_pattern_must_be_plain_or_it_is_refused_by_name_with_2() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-54-serve"]);

    for bad in NOT_PLAIN {
        let as_option = format!("--issue={}", bad);
        for args in [vec!["prompt", "tester", bad], vec!["prompt", "tester", as_option.as_str()]] {
            if bad.starts_with('-') && args[2] == bad {
                continue; // a word that starts with `-` is an option, not the issue word
            }
            let run = wb(&folder.path, &args, "");

            assert_eq!(code(&run), Some(2), "{:?}", args);
            assert_eq!(text(&run.stdout), "", "{:?}", args);
            let err = text(&run.stderr);
            assert!(err.starts_with("wb prompt: ") && err.contains("'--issue'"), "{:?}: {}", args, err);
        }
    }
}

#[test]
fn test_WB_PROMPT_39_a_change_that_fills_a_pattern_must_be_plain_or_it_is_refused_by_name_with_2() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-54-serve"]);

    for bad in NOT_PLAIN {
        let change = format!("--change={}", bad);
        let run = wb(&folder.path, &["prompt", "review", "54", change.as_str(), "--round", "1"], "");

        assert_eq!(code(&run), Some(2), "{}", bad);
        assert_eq!(text(&run.stdout), "", "{}", bad);
        assert!(text(&run.stderr).contains("'--change'"), "{}: {}", bad, text(&run.stderr));
    }
}

#[test]
fn test_WB_PROMPT_39_plain_values_are_filled_and_a_value_that_fills_no_pattern_is_taken_as_given() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-T-1.a_b"]);

    let plain = wb(&folder.path, &["prompt", "tester", "T-1.a_b"], "");
    assert_eq!(code(&plain), Some(0), "{}", text(&plain.stderr));
    assert!(text(&plain.stdout).contains("issue-T-1.a_b"), "{}", text(&plain.stdout));

    // Given the branch and the address, the issue fills no pattern: it is not held to the rule.
    let run = wb(&folder.path, &["prompt", "tester", "a..b", "--branch", "b", "--issue_address", "a"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("issue a..b"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_PROMPT_17_a_run_that_derives_writes_nothing_under_the_repository() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-54-serve"]);
    let before = snapshot(&folder.path);

    let run = wb(&folder.path, &["prompt", "review", "54", "--change", "168", "--round", "1"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_same_snapshot(&before, &snapshot(&folder.path), "a run that derives");
}

#[test]
fn test_WB_PROMPT_31_the_usage_names_the_issue_word() {
    let run = wb(&folder().path, &["help", "prompt"], "");

    assert!(text(&run.stdout).contains("wb prompt <role> [<issue>]"), "{}", text(&run.stdout));
}

// ---- the thing at the tracker is an issue (#78) ----

#[test]
fn test_WB_PROMPT_45_the_options_task_and_task_address_are_not_known_and_end_with_2_and_the_usage() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-54-serve"]);

    for args in [
        vec!["prompt", "tester", "--task", "54"],
        vec!["prompt", "tester", "--task=54"],
        vec!["prompt", "tester", "54", "--task_address", "at 54"],
        vec!["prompt", "tester", "54", "--task_address=at 54"],
    ] {
        let run = wb(&folder.path, &args, "");

        assert_eq!(code(&run), Some(2), "{:?}", args);
        assert_eq!(text(&run.stdout), "", "{:?}", args);
        let err = text(&run.stderr);
        assert!(err.starts_with("wb prompt: '--task"), "{:?}: {}", args, err);
        assert!(err.contains("is not known"), "{:?}: {}", args, err);
        assert!(err.ends_with(&format!("\n\n{}", USAGE)), "{:?}: {}", args, err);
    }
}

#[test]
fn test_WB_PROMPT_45_the_usage_names_no_task_option() {
    assert!(!USAGE.contains("--task"), "{}", USAGE);
    assert!(USAGE.contains("[--issue <text>]") && USAGE.contains("[--issue_address <text>]"), "{}", USAGE);
}

/// `wb prompt tester 54` in a repository whose `watchbill.yml` has `old` in the place of `new`.
fn with_old_config(old: &str, new: &str) -> std::process::Output {
    let folder = set_up_github();
    let file = config(&folder.path);
    let before = read(&file);
    assert!(before.contains(new), "{}", before);
    fs::write(&file, before.replace(new, old)).unwrap();
    commit_config(&folder.path);
    wb(&folder.path, &["prompt", "tester", "54"], "")
}

#[test]
fn test_WB_PROMPT_46_a_watchbill_yml_with_the_key_task_address_is_refused_by_name_with_what_to_write() {
    let run = with_old_config("task_address: 'gh issue view {issue}", "issue_address: 'gh issue view {issue}");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "");
    let err = text(&run.stderr);
    assert!(err.starts_with("wb prompt: "), "{}", err);
    assert!(err.contains("watchbill.yml") && err.contains("`task_address`"), "{}", err);
    assert!(err.contains("write `issue_address` in its place"), "{}", err);
}

#[test]
fn test_WB_PROMPT_46_the_key_task_address_is_refused_also_when_the_address_is_given() {
    let folder = set_up_github();
    let file = config(&folder.path);
    fs::write(&file, format!("{}task_address: 'x {{issue}}'\n", read(&file))).unwrap();
    commit_config(&folder.path);

    let run = wb(&folder.path, &["prompt", "tester", "54", "--issue_address", "at 54"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("`task_address`"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_47_a_pattern_with_task_in_braces_is_refused_by_name_with_what_to_write() {
    for (old, new, key) in [
        ("branch: 'issue-{task}'", "branch: 'issue-{issue}'", "branch"),
        ("issue_address: 'gh issue view {task}", "issue_address: 'gh issue view {issue}", "issue_address"),
        ("change_address: 'gh pr view {change} {task}", "change_address: 'gh pr view {change}", "change_address"),
    ] {
        let run = with_old_config(old, new);

        assert_eq!(code(&run), Some(1), "{}: {}", key, text(&run.stderr));
        assert_eq!(text(&run.stdout), "", "{}", key);
        let err = text(&run.stderr);
        assert!(err.starts_with("wb prompt: "), "{}: {}", key, err);
        assert!(err.contains("watchbill.yml") && err.contains(&format!("({})", key)), "{}: {}", key, err);
        assert!(err.contains("`{task}`") && err.contains("write `{issue}` in its place"), "{}: {}", key, err);
    }
}

#[test]
fn test_WB_PROMPT_48_the_stamp_names_the_issue_on_the_line_issue_and_has_no_line_task() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-54"]);

    let run = wb(&folder.path, &["prompt", "tester", "54", "--stamp"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.lines().any(|line| line == "issue: 54"), "{}", out);
    assert!(!out.lines().any(|line| line.starts_with("task:")), "{}", out);
}

#[test]
fn test_WB_PROMPT_49_the_planner_with_no_branch_there_is_given_the_pattern_filled() {
    let folder = set_up_github();
    // The planner is handed out only to a runner the organisation cleared for it (#93).
    let (_place, organisation) = cleared(&folder.path);

    let run = prompt_with(&folder.path, &organisation, &["planner", "42", "--runner", "claude-code"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("on the branch issue-42."), "{}", text(&run.stdout));
}

#[test]
fn test_WB_PROMPT_52_the_planner_is_handed_out_with_no_file_of_the_issue_to_a_cleared_runner() {
    let folder = set_up_github();
    let (_place, organisation) = cleared(&folder.path);
    assert!(!folder.path.join(".watchbill/issues").join("77.yml").exists());

    let run = prompt_with(&folder.path, &organisation, &["planner", "77", "--runner", "claude-code"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("You are the planner of issue 77."), "{}", text(&run.stdout));
}

#[test]
fn test_WB_PROMPT_49_the_tester_on_a_longer_branch_names_it_in_the_prompt_and_the_stamp() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-42-one-round-count"]);

    let run = wb(&folder.path, &["prompt", "tester", "42"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("on the branch issue-42-one-round-count."), "{}", text(&run.stdout));

    let stamp = wb(&folder.path, &["prompt", "tester", "42", "--stamp"], "");
    assert_eq!(code(&stamp), Some(0), "{}", text(&stamp.stderr));
    assert!(text(&stamp.stdout).contains("branch: issue-42-one-round-count\n"), "{}", text(&stamp.stdout));
}

#[test]
fn test_WB_PROMPT_49_the_tester_with_no_branch_there_is_refused_with_1_and_nothing_on_standard_output() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-6-other"]);

    let run = wb(&folder.path, &["prompt", "tester", "42"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    let err = text(&run.stderr);
    assert!(err.contains("issue-42") && err.contains("--branch"), "{}", err);
}

#[test]
fn test_WB_PROMPT_49_the_tester_with_two_branches_is_refused_with_both_named() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-42-a", "issue-42-b"]);

    let run = wb(&folder.path, &["prompt", "tester", "42"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    let err = text(&run.stderr);
    assert!(err.contains("issue-42-a") && err.contains("issue-42-b") && err.contains("--branch"), "{}", err);
}

#[test]
fn test_WB_PROMPT_49_branch_given_to_the_tester_and_the_planner_wins() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-42-a", "issue-42-b"]);

    // The planner is handed out only to a runner the organisation cleared for it (#93).
    let (_place, organisation) = cleared(&folder.path);
    for role in ["tester", "planner"] {
        let run = match role {
            "planner" => prompt_with(&folder.path, &organisation, &[role, "42", "--branch", "mine", "--runner", "claude-code"]),
            _ => wb(&folder.path, &["prompt", role, "42", "--branch", "mine"], ""),
        };

        assert_eq!(code(&run), Some(0), "{}: {}", role, text(&run.stderr));
        assert!(text(&run.stdout).contains("on the branch mine."), "{}: {}", role, text(&run.stdout));
    }
}

#[test]
fn test_WB_PROMPT_49_the_build_the_review_and_the_fix_take_the_one_branch_as_before() {
    let folder = set_up_github();
    branches(&folder.path, &["issue-42-x"]);

    for role in ["build", "review", "fix"] {
        let run = wb(&folder.path, &["prompt", role, "42", "--change", "7", "--round", "1"], "");

        assert_eq!(code(&run), Some(0), "{}: {}", role, text(&run.stderr));
        assert!(text(&run.stdout).contains("on the branch issue-42-x."), "{}: {}", role, text(&run.stdout));
    }
}

// ---- `{classed}` and `{classed_touched}` (#227) ----

/// The words of a review with every placeholder given but the two of classed code.
fn review_words() -> Vec<&'static str> {
    ["prompt", "review"].into_iter().chain(ALL).collect()
}

/// What the review is printed as, for these values of the two placeholders.
fn review_filled(classed: &str, touched: &str) -> String {
    // The two stand in a list item of the text, after two spaces, and a value of several lines keeps them.
    let (classed, touched) = (classed.replace('\n', "\n  "), touched.replace('\n', "\n  "));
    let all: Vec<(&str, &str)> = ALL_VALUES.iter().copied().chain([("classed", classed.as_str()), ("classed_touched", touched.as_str())]).collect();
    filled("review", &all)
}

/// The review printed holds both values, and is the text of the role with them filled in.
#[track_caller]
fn assert_review(run: &std::process::Output, classed: &str, touched: &str) {
    let printed = text(&run.stdout);
    let indented = |value: &str| format!("\n  {}\n", value.replace('\n', "\n  "));
    assert!(printed.contains(&indented(classed)), "{{classed}} is not in it: {}", printed);
    assert!(printed.contains(&indented(touched)), "{{classed_touched}} is not in it: {}", printed);
    assert_eq!(printed, review_filled(classed, touched));
}

/// A repository set up as `set_up` does, with `classed:` as given (no line for `None`), the default branch
/// `main` and the branch `issue-1` with one commit that writes each of `changed`, left checked out.
fn classed_repository(classed: Option<&str>, changed: &[&str]) -> Folder {
    let folder = set_up();
    let yml: String = read(&config(&folder.path))
        .lines()
        .filter(|line| !line.starts_with("classed:"))
        .chain(classed)
        .map(|line| format!("{}\n", line))
        .collect();
    put(&folder.path, ".watchbill/watchbill.yml", &yml);
    commit_config(&folder.path);
    git(&folder.path, &["branch", "-M", "main"]);
    git(&folder.path, &["checkout", "-q", "-b", "issue-1"]);
    for path in changed {
        put(&folder.path, path, "changed\n");
        git(&folder.path, &["add", "--", path]);
    }
    git(&folder.path, &["commit", "-q", "-m", "The change"]);
    folder
}

#[test]
fn test_WB_PROMPT_65_classed_is_the_paths_of_watchbill_yml_one_to_a_line_and_none_for_an_empty_list() {
    let folder = classed_repository(Some("classed: [src/safety, firmware/boot.c]"), &["src/other.rs"]);
    let run = wb(&folder.path, &review_words(), "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_review(&run, "src/safety\nfirmware/boot.c", "none");

    let folder = classed_repository(Some("classed: []"), &["src/safety/limits.rs"]);
    let run = wb(&folder.path, &review_words(), "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_review(&run, "none", "none");
}

#[test]
fn test_WB_PROMPT_66_classed_says_every_path_is_classed_where_watchbill_yml_has_no_classed() {
    let folder = classed_repository(None, &["src/a.rs", "src/b.rs"]);
    let run = wb(&folder.path, &review_words(), "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_review(&run, "every path (watchbill.yml has no `classed`)", "src/a.rs\nsrc/b.rs");
}

#[test]
fn test_WB_PROMPT_67_classed_touched_is_the_paths_of_the_change_under_a_classed_path_or_none() {
    let folder = classed_repository(Some("classed: [src/safety]"), &["src/safety/limits.rs", "src/other.rs"]);
    let run = wb(&folder.path, &review_words(), "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_review(&run, "src/safety", "src/safety/limits.rs");

    let folder = classed_repository(Some("classed: [src/safety]"), &["src/other.rs", "src/safety-not/x.rs"]);
    let run = wb(&folder.path, &review_words(), "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_review(&run, "src/safety", "none");
}

#[test]
fn test_WB_PROMPT_68_classed_touched_says_it_cannot_be_told_and_why_where_the_diff_is_not_there() {
    // The branch of the change is not there.
    let folder = classed_repository(Some("classed: [src/safety]"), &["src/safety/limits.rs"]);
    let mut words = review_words();
    let at = words.iter().position(|word| *word == "issue-1").unwrap();
    words[at] = "nosuch";
    let run = wb(&folder.path, &words, "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let told = "cannot be told: the branch nosuch is not there";
    assert!(text(&run.stdout).contains(told), "{}", text(&run.stdout));
    assert_eq!(text(&run.stdout), review_filled("src/safety", told).replace("issue-1", "nosuch"));

    // No default branch to compare with.
    let folder = classed_repository(Some("classed: [src/safety]"), &["src/safety/limits.rs"]);
    git(&folder.path, &["branch", "-m", "main", "trunk"]);
    // Since #323 no default branch is as no `.watchbill/signers`: the acceptance of T-1 counts for nothing.
    let run = wb(&folder.path, &review_words(), "");
    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    assert!(text(&run.stderr).contains("no default branch"), "{}", text(&run.stderr));
    // An issue with no file needs no acceptance (WB-TASK-17): T-1's file taken out, the review is handed out.
    git(&folder.path, &["rm", "-q", "--", ".watchbill/issues/T-1.yml", ".watchbill/issues/T-1.accepted"]);
    git(&folder.path, &["commit", "-q", "-m", "No file of T-1"]);
    let run = wb(&folder.path, &review_words(), "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    // The review as before, without the parts that come of the issue's file: the interface and the lists.
    let told = "cannot be told: no default branch to compare with";
    let printed = text(&run.stdout);
    assert!(printed.contains("\n  src/safety\n") && printed.contains(&format!("\n  {}\n", told)), "{}", printed);
    let full = review_filled("src/safety", told);
    assert_eq!(printed, full[..full.find("\n## The interface").unwrap()]);
}

#[test]
fn test_WB_PROMPT_69_the_two_placeholders_are_on_the_list_and_wb_check_takes_them_in_the_projects_text() {
    for name in ["classed", "classed_touched"] {
        assert!(listed().contains(&name.to_string()), "{} is not on the list", name);
        assert!(watchbill::prompt::PLACEHOLDERS.contains(&name), "{} is not known to wb", name);
    }
    let folder = set_up();
    put(&folder.path, ".watchbill/project/review.md", "Classed: {classed}. Touched: {classed_touched}.\n");

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_67_only_the_files_of_this_issue_are_left_out_of_classed_touched() {
    // With no `classed` every path is classed. The files of issue 1 are on its branch by design and are left
    // out; those of issue 9 are not, and a folder of the same name deeper down is not the issue's file.
    let folder = classed_repository(None, &["src/a.rs", ".watchbill/issues/9.yml", ".watchbill/issues/9.accepted", ".watchbill/issues/x/1.yml"]);
    git(&folder.path, &["rm", "-q", "--", ".watchbill/issues/1.yml", ".watchbill/issues/1.accepted"]);
    git(&folder.path, &["commit", "-q", "-m", "The issue's files go"]);
    let mut words = review_words();
    let at = words.iter().position(|word| *word == "T-1").unwrap();
    words[at] = "1";
    let run = wb(&folder.path, &words, "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let listed = "`none`:\n\n  .watchbill/issues/9.accepted\n  .watchbill/issues/9.yml\n  .watchbill/issues/x/1.yml\n  src/a.rs\n\n  Say first";
    assert!(text(&run.stdout).contains(listed), "{}", text(&run.stdout));

    // With no issue nothing is left out.
    use watchbill::prompt::touched_text;
    let touched = touched_text(folder.path.to_str().unwrap(), &None, Some("issue-1"), None);
    assert!(touched.contains(".watchbill/issues/1.yml") && touched.contains(".watchbill/issues/1.accepted"), "{}", touched);
}

#[test]
fn test_WB_PROMPT_68_each_reason_that_the_diff_cannot_be_had_is_said() {
    use watchbill::prompt::touched_text;
    let folder = classed_repository(Some("classed: [src/safety]"), &["src/safety/limits.rs"]);
    let root = folder.path.to_str().unwrap();
    let list = Some(vec!["src/safety".to_string()]);

    // The branch of the change is not known.
    assert_eq!(touched_text(root, &list, None, None), "cannot be told: the branch of the change is not known");
    // The branch is not there.
    assert_eq!(touched_text(root, &list, Some("nosuch"), None), "cannot be told: the branch nosuch is not there");
    // The branch has no history in common with the default branch.
    git(&folder.path, &["checkout", "-q", "--orphan", "other"]);
    git(&folder.path, &["commit", "-q", "-m", "Alone"]);
    let alone = touched_text(root, &list, Some("other"), None);
    assert!(alone.starts_with("cannot be told: the branch other has no merge base with "), "{}", alone);
    // No default branch is there.
    git(&folder.path, &["branch", "-m", "main", "trunk"]);
    assert_eq!(touched_text(root, &list, Some("issue-1"), None), "cannot be told: no default branch to compare with");
}

#[test]
fn test_WB_PROMPT_70_a_value_of_several_lines_of_another_placeholder_is_put_in_as_it_is_given() {
    let folder = set_up();
    put_committed(&folder.path, ".watchbill/project/review.md", "Where:\n  {issue_address}\n");
    let mut words = review_words();
    let at = words.iter().position(|word| *word == "tracker T-1").unwrap();
    words[at] = "one\ntwo";

    let run = wb(&folder.path, &words, "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("Where:\n  one\ntwo\n"), "{}", text(&run.stdout));
    assert!(!text(&run.stdout).contains("one\n  two"), "{}", text(&run.stdout));
}
