//! The rules of the layout (#180): each refusal of each command says whether it is a limit or a guard, a guard
//! names the flag that passes it, and the plumbing's standard output is fixed. Run as a person runs `wb`, in a
//! repository made for the test; the patterns are small commands over files of a folder `.tracker/` that git
//! does not see: stand-ins, never the network.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Output;

/// The stand-in that posts: each comment is kept whole in `.tracker/posted-<on>-<n>`.
const POST: &str = "n=$(ls .tracker | grep -c \"^posted-$1-\")\ncat > \".tracker/posted-$1-$n\"\n";

/// A repository set up by `wb init --yes`, with the stand-in patterns, issue 174 accepted and committed, and a
/// folder `.tracker/` that git ignores.
fn set_up() -> Folder {
    let folder = repository(None);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let patterns = "issue_comments: \"cat .tracker/{issue}.comments\"
issue_comment_bodies: \"cat .tracker/{issue}.bodies\"
issue_comment: \"sh .tracker/post.sh {issue}\"
";
    let mut file = fs::OpenOptions::new().append(true).open(config(&folder.path)).unwrap();
    file.write_all(patterns.as_bytes()).unwrap();
    put(&folder.path, ".git/info/exclude", ".tracker/\n");
    put(&folder.path, ".tracker/post.sh", POST);
    git(&folder.path, &["add", "--", ".watchbill"]);
    git(&folder.path, &["commit", "-q", "-m", "Set up"]);
    issues_ready(&folder.path, &["174"]);
    folder
}

/// The comments posted on an issue, in the order they were posted.
fn posted(folder: &Path, on: &str) -> Vec<String> {
    (0..).map_while(|n| fs::read_to_string(folder.join(".tracker").join(format!("posted-{}-{}", on, n))).ok()).collect()
}

/// The first line of standard error that starts with `<name>: limit: ` or `<name>: guard: `, if there is one.
fn labelled<'a>(err: &'a str, name: &str) -> Option<&'a str> {
    let (limit, guard) = (format!("{}: limit: ", name), format!("{}: guard: ", name));
    err.lines().find(|line| line.starts_with(&limit) || line.starts_with(&guard))
}

/// The run ended with 1, and its standard error holds a refusal of `name` that starts with `kind`.
fn refused_as(run: &Output, name: &str, kind: &str) -> String {
    let err = text(&run.stderr);
    assert_eq!(code(run), Some(1), "{}", err);
    let line = labelled(&err, name).unwrap_or_else(|| panic!("no line of {} starts with limit: or guard:\n{}", name, err));
    assert!(line.starts_with(&format!("{}: {}: ", name, kind)), "{}", err);
    err
}

// ---- WB-CMD-27: each refusal of each command holds limit: or guard: ----

#[test]
fn test_WB_CMD_27_a_refusal_of_each_command_starts_with_limit_after_the_command_s_name() {
    let folder = set_up();
    let at = &folder.path;
    // init: the files are there already.
    refused_as(&wb(at, &["init", "--yes"], ""), "wb init", "limit");
    // prompt and start: a role the revision does not have, and a role started out of a person's sight.
    refused_as(&wb(at, &["prompt", "captain", "174"], ""), "wb prompt", "limit");
    let organisation = folder_named("organisation");
    let org = organisation.path.join("organisation.yml");
    fs::write(&org, "levels: [public]\nrunners:\n  stand-in:\n    tools: this machine\n    model: none\n    agreement: none\n    up_to: public\n    start: [/bin/true]\n").unwrap();
    refused_as(&start_with(at, &org, false, &["start", "build", "174", "--runner", "stand-in"]), "wb start build", "limit");
    // accept: no issue's file.
    refused_as(&wb(at, &["accept", "999"], ""), "wb accept", "limit");
    // view: an issue with no file.
    refused_as(&wb(at, &["view", "999", "build"], ""), "wb view", "limit");
    // answer: no open question of that name.
    put(at, ".tracker/174.bodies", &format!("{}\0", BREAKDOWN));
    refused_as(&wb(at, &["answer", "174", "q9", "1"], ""), "wb answer", "limit");
    // revert: the breakdown is taken back by git.
    refused_as(&wb(at, &["revert", "174", "breakdown"], ""), "wb revert", "limit");
    // upgrade: the repository is pinned to a version newer than the installed one.
    let yml = read(&config(at)).replace(&format!("watchbill: {}", REVISION), "watchbill: 999.0.0");
    fs::write(config(at), yml).unwrap();
    refused_as(&wb(at, &["upgrade"], ""), "wb upgrade", "limit");
    // status and view: watchbill.yml over its bound in HEAD (#261).
    let mut big = read(&config(at));
    big.push_str(&format!("# {}\n", "x".repeat(2 * 1024 * 1024)));
    fs::write(config(at), big).unwrap();
    commit_config(at);
    refused_as(&wb(at, &["status"], ""), "wb status", "limit");
    refused_as(&wb(at, &["view", "174", "build"], ""), "wb view", "limit");
    // Exit 2, used wrongly or not set up, is no refusal and has no label.
    for args in [&["accept"][..], &["view", "174"], &["revert", "174"], &["prompt"], &["start"]] {
        let run = wb(at, args, "");
        assert_eq!(code(&run), Some(2), "{:?}: {}", args, text(&run.stderr));
        let err = text(&run.stderr);
        assert!(!err.contains(": limit: ") && !err.contains(": guard: "), "{:?}: {}", args, err);
    }
}

#[test]
fn test_WB_CMD_27_a_refusal_of_wb_revision_starts_with_limit() {
    let empty = folder();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = watchbill::revision(&[], &empty.path, &mut out, &mut err);
    assert_eq!(status, 1);
    let err = text(&err);
    assert!(labelled(&err, "wb revision").is_some_and(|line| line.starts_with("wb revision: limit: ")), "{}", err);
}

#[test]
fn test_WB_CMD_27_a_refusal_of_several_lines_carries_the_label_on_its_first() {
    let folder = set_up();
    put(&folder.path, ".watchbill/issues/175.yml", &BARE_ISSUE.replace("{issue}", "175").replace("questions: []\n", "questions:\n  - Which name?\n  - Which value?\n"));
    let err = refused_as(&wb(&folder.path, &["accept", "175", "--no-commit"], ""), "wb accept", "limit");
    let first = err.lines().position(|line| line.starts_with("wb accept: limit: ")).unwrap();
    assert!(err.lines().nth(first).unwrap().contains("holds 2 questions"), "{}", err);
    assert!(err.lines().nth(first + 1).unwrap().starts_with("  - "), "{}", err);
}

// ---- WB-CMD-28 and WB-CMD-29: a guard names its flag, and the flag passes it ----

/// A repository set up as `set_up` makes it, on `main`, with the issue's file of 54 there and not committed.
fn not_on_the_branch() -> Folder {
    let folder = set_up();
    put(&folder.path, ".watchbill/issues/54.yml", &BARE_ISSUE.replace("{issue}", "54"));
    folder
}

#[test]
fn test_WB_CMD_28_the_guard_of_wb_accept_names_no_commit_a_flag_of_wb_accept() {
    let folder = not_on_the_branch();
    let err = refused_as(&wb(&folder.path, &["accept", "54"], ""), "wb accept", "guard");
    assert!(err.contains("--no-commit"), "{}", err);
    let help = wb(&folder.path, &["help", "accept"], "");
    assert!(text(&help.stdout).contains("--no-commit"), "{}", text(&help.stdout));
}

#[test]
fn test_WB_CMD_29_no_commit_passes_the_guard_of_wb_accept() {
    assert!(watchbill::refusal::GUARDS.contains(&("accept", "--no-commit")));
    let folder = not_on_the_branch();
    let run = wb(&folder.path, &["accept", "54", "--no-commit"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(labelled(&text(&run.stderr), "wb accept").is_none(), "{}", text(&run.stderr));
    assert!(folder.path.join(".watchbill/issues/54.accepted").is_file());
}

#[test]
fn test_WB_CMD_28_each_flag_that_passes_a_guard_is_a_flag_of_its_command() {
    let folder = folder();
    for (command, flag) in watchbill::refusal::GUARDS {
        let help = wb(&folder.path, &["help", command], "");
        assert_eq!(code(&help), Some(0), "{}", text(&help.stderr));
        assert!(text(&help.stdout).contains(flag), "{} is not in wb help {}:\n{}", flag, command, text(&help.stdout));
    }
    // No general `--force`: no command has it.
    assert!(!watchbill::USAGE.contains("--force"));
}

// ---- WB-CMD-30: the plumbing's standard output is fixed ----

#[test]
fn test_WB_CMD_30_a_refusal_of_the_plumbing_is_said_on_standard_error_only() {
    let folder = set_up();
    // prompt: nothing on standard output, the label on standard error.
    let run = wb(&folder.path, &["prompt", "captain", "174"], "");
    assert_eq!(text(&run.stdout), "");
    refused_as(&run, "wb prompt", "limit");
    // revision: the same.
    let empty = self::folder();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    assert_eq!(watchbill::revision(&[], &empty.path, &mut out, &mut err), 1);
    assert!(out.is_empty());
    assert!(text(&err).starts_with("wb revision: limit: "), "{}", text(&err));
    // check: a difference is an answer, not a refusal. Its lines stay as they were, with no label on either stream.
    let file: PathBuf = folder.path.join(".watchbill/revision/models.yml");
    let mut bytes = fs::read(&file).unwrap_or_default();
    bytes.extend_from_slice(b"\n# changed\n");
    fs::write(&file, bytes).unwrap();
    let run = wb(&folder.path, &["check"], "");
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let (out, err) = (text(&run.stdout), text(&run.stderr));
    assert!(out.lines().last().is_some_and(|line| line.starts_with("1 difference. ")), "{}", out);
    assert!(!out.contains("limit:") && !out.contains("guard:") && !err.contains("limit:") && !err.contains("guard:"), "{}{}", out, err);
}

// ---- WB-CMD-31: a refusal written without a label fails a test ----

/// The lines of `src/` that end a command with 1 by a shape a refusal takes: `return 1;`, `Err(1)`, `code = 1;`,
/// `=> 1,` or a line `1`. Each is allowed only with a comment `// not a refusal:` on it or on one of the three lines
/// above: every refusal goes through `refusal::limit`, `refusal::guard` or `refusal::say`, which write the label.
#[test]
fn test_WB_CMD_31_every_site_that_ends_with_1_writes_through_refusal_or_says_why_it_is_none() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut bare = Vec::new();
    let mut files: Vec<PathBuf> = fs::read_dir(&src).unwrap().map(|entry| entry.unwrap().path()).filter(|path| path.extension().is_some_and(|ext| ext == "rs")).collect();
    files.sort();
    assert!(files.iter().any(|path| path.ends_with("refusal.rs")), "src/refusal.rs is not there");
    for path in files.iter().filter(|path| !path.ends_with("refusal.rs") && !path.ends_with("main.rs")) {
        let text = fs::read_to_string(path).unwrap();
        // The tests inside a file are no command.
        let code = text.split("#[cfg(test)]").next().unwrap_or_default();
        let lines: Vec<&str> = code.lines().collect();
        for (at, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let ends = trimmed.contains("return 1;") || trimmed.contains("Err(1)") || trimmed.contains("code = 1;") || trimmed.ends_with("=> 1,") || trimmed == "1";
            if !ends {
                continue;
            }
            let said = lines[at.saturating_sub(3)..=at].iter().any(|line| line.contains("// not a refusal:"));
            if !said {
                bare.push(format!("{}:{}: {}", path.file_name().unwrap().to_string_lossy(), at + 1, trimmed));
            }
        }
    }
    assert!(bare.is_empty(), "these end with 1 and neither write through refusal nor say why they are no refusal:\n{}", bare.join("\n"));
}

#[test]
fn test_WB_CMD_31_the_refusal_functions_write_the_label_and_give_1() {
    let mut err = Vec::new();
    assert_eq!(watchbill::refusal::limit(&mut err, "wb x", "it is not there."), 1);
    assert_eq!(text(&err), "wb x: limit: it is not there.\n");
    let mut err = Vec::new();
    assert_eq!(watchbill::refusal::guard(&mut err, "wb x", "--ignore-y", "a y is there; --ignore-y passes it."), 1);
    assert_eq!(text(&err), "wb x: guard: a y is there; --ignore-y passes it.\n");
}

// ---- WB-CMD-32: a command that posts checks every part before the first post ----

const BREAKDOWN: &str = "[questions] 174 q1
q1: Which name does the event use?
<details>

1. `task_from`.
2. Another name.

Suggested: 1, because it is the issue's own example.

changes the issue's file: no

</details>
";

const TESTER: &str = "[questions] 174.tester q1
q1: Is a clause with no test left without one?
<details>

1. Let the tester read the two files of tests.
2. Leave the clause without a test.

Suggested: 1, because the clause needs a check.

changes the issue's file: no

</details>
";

#[test]
fn test_WB_CMD_32_wb_answer_with_one_answer_wrong_posts_none_and_says_it_as_a_limit() {
    let folder = set_up();
    put(&folder.path, ".tracker/174.bodies", &format!("{}\0{}\0", BREAKDOWN, TESTER));
    // The answer to the breakdown is right, and would be posted first; the tester's is no choice.
    let run = wb(&folder.path, &["answer", "174", "q1", "1", "tester.q1", "9"], "");
    refused_as(&run, "wb answer", "limit");
    assert!(posted(&folder.path, "174").is_empty(), "{:?}", posted(&folder.path, "174"));
}
