//! Flags take git's short forms (#285): `-n` for `--dry-run` of `wb upgrade` and `--no-commit` of `wb accept` and
//! `wb plan accept`, and `-q` / `--quiet` for every command, which leaves out the lines that only inform and never a
//! refusal, what a command says before it writes, or the output a script reads.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const GOOD: &str = "issue: '54'
from: 3f0c1d9e
requirements:
  - id: '54.1'
    text: The tool reads the file.
    source: the goal
interface: |
  fn read(path: &str) -> String
changes: [src/issue.rs]
reads:
  tester: []
  build: []
  review: []
  fix: []
existing_tests_changed: []
questions: []
";

const YML: &str = ".watchbill/issues/54.yml";
const ACCEPTED: &str = ".watchbill/issues/54.accepted";
const ORG: &str = "levels: [public, internal, confidential]
runners:
  stand-in:
    tools: this machine
    model: a stand-in
    agreement: none
    up_to: internal
default: stand-in
";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn out_of(run: &Output) -> String {
    text(&run.stdout)
}

fn err_of(run: &Output) -> String {
    String::from_utf8_lossy(&run.stderr).to_string()
}

fn say(folder: &Path, args: &[&str]) -> String {
    let run = Command::new("git").arg("-C").arg(folder).args(args).env_clear().envs(env()).stderr(Stdio::null()).output().expect("git could not be started");
    text(&run.stdout)
}

fn count(folder: &Path) -> usize {
    say(folder, &["rev-list", "--count", "HEAD"]).trim().parse().unwrap()
}

/// A repository set up by `wb init --yes` and committed, with a git identity.
fn set_up() -> Folder {
    let folder = repository(None);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    git(&folder.path, &["add", "--all", "--", ".watchbill"]);
    git(&folder.path, &["commit", "-q", "-m", "Watchbill"]);
    folder
}

/// A set-up repository on the branch `issue-54`, with the issue's file committed alone.
fn on_the_branch() -> Folder {
    let folder = set_up();
    git(&folder.path, &["checkout", "-q", "-b", "issue-54"]);
    put(&folder.path, YML, GOOD);
    git(&folder.path, &["add", "--", YML]);
    git(&folder.path, &["commit", "-q", "-m", "The issue's file (54)"]);
    folder
}

/// A set-up repository where a file of the revision is gone and committed, so that `wb upgrade` has a write to say.
fn upgradable() -> Folder {
    let folder = set_up();
    fs::remove_file(folder.path.join(".watchbill/revision/roles/fix.md")).unwrap();
    git(&folder.path, &["add", "--all", "--", ".watchbill"]);
    git(&folder.path, &["commit", "-q", "-m", "A file of the revision is gone"]);
    folder
}

fn files(folder: &Path) -> BTreeSet<String> {
    let found = say(folder, &["ls-files", "--others", "--cached", "--modified", "--deleted"]);
    found.lines().map(str::to_string).collect()
}

// ---- WB-CMD-37: wb upgrade -n ----

#[test]
fn test_WB_CMD_37_upgrade_n_does_what_dry_run_does() {
    let folder = upgradable();
    let before = files(&folder.path);

    let long = wb(&folder.path, &["upgrade", "--dry-run"], "");
    let short = wb(&folder.path, &["upgrade", "-n"], "");

    assert_eq!(code(&long), Some(0), "{}", err_of(&long));
    assert_eq!(code(&short), Some(0), "{}", err_of(&short));
    assert_eq!(out_of(&short), out_of(&long));
    assert!(out_of(&short).contains("add .watchbill/revision/roles/fix.md"), "{}", out_of(&short));
    assert!(out_of(&short).contains("--dry-run: Nothing written."), "{}", out_of(&short));
    assert!(!folder.path.join(".watchbill/revision/roles/fix.md").exists());
    assert_eq!(files(&folder.path), before);
}

#[test]
fn test_WB_CMD_37_upgrade_n_and_dry_run_together_are_given_twice() {
    let folder = upgradable();

    for args in [&["upgrade", "-n", "--dry-run"][..], &["upgrade", "--dry-run", "-n"][..], &["upgrade", "-n", "-n"][..]] {
        let run = wb(&folder.path, args, "");
        assert_eq!(code(&run), Some(2), "{:?}", args);
        assert!(err_of(&run).contains("is given twice"), "{:?}: {}", args, err_of(&run));
    }
    assert!(!folder.path.join(".watchbill/revision/roles/fix.md").exists());
}

// ---- WB-CMD-38: wb accept -n and wb plan accept -n ----

#[test]
fn test_WB_CMD_38_accept_n_does_what_no_commit_does() {
    let folder = on_the_branch();
    let before = count(&folder.path);

    let run = wb(&folder.path, &["accept", "-n", "54"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(folder.path.join(ACCEPTED).exists());
    assert_eq!(count(&folder.path), before);
    assert!(out_of(&run).contains("commit the acceptance alone"), "{}", out_of(&run));
}

#[test]
fn test_WB_CMD_38_accept_n_passes_the_commit_checks_as_no_commit_does_and_no_other() {
    // As `--no-commit`, it passes the checks of the commit alone: a file staged before does not stop it.
    let folder = on_the_branch();
    put(&folder.path, "other.txt", "staged\n");
    git(&folder.path, &["add", "--", "other.txt"]);

    let long = wb(&folder.path, &["accept", "--no-commit", "54"], "");
    let long_out = out_of(&long);
    fs::remove_file(folder.path.join(ACCEPTED)).unwrap();
    let short = wb(&folder.path, &["accept", "-n", "54"], "");

    // The time of the acceptance is the one thing that differs between two runs.
    let untimed = |out: &str| out.lines().filter(|line| !line.starts_with("time: ")).collect::<Vec<_>>().join("\n");
    assert_eq!(code(&long), code(&short));
    assert_eq!(untimed(&long_out), untimed(&out_of(&short)));
}

/// A repository for `wb plan add`, in the test process, as `tests/plans.rs` makes it.
struct Bench {
    repo: Folder,
    org: PathBuf,
    _org_folder: Folder,
}

fn bench() -> Bench {
    let repo = repository(None);
    git(&repo.path, &["config", "user.name", "Ada Lovelace"]);
    git(&repo.path, &["config", "user.email", "ada@example.org"]);
    git(&repo.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&repo.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let mut yml = read(&config(&repo.path));
    yml.push_str("confidentiality: internal\n");
    fs::write(config(&repo.path), yml).unwrap();
    git(&repo.path, &["add", "--", ".watchbill"]);
    git(&repo.path, &["commit", "-q", "-m", "Watchbill"]);
    git(&repo.path, &["branch", "-M", "main"]);
    git(&repo.path, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(&repo.path, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"]);
    let org_folder = folder_named("org");
    let org = org_folder.path.join("organisation.yml");
    fs::write(&org, ORG).unwrap();
    Bench { repo, org, _org_folder: org_folder }
}

impl Bench {
    /// `wb plan` with the words after `plan`, in the test process: its status, standard output and standard error.
    fn plan(&self, args: &[&str]) -> (i32, String, String) {
        let words: Vec<OsString> = args.iter().map(OsString::from).collect();
        let mut input = std::io::Cursor::new(Vec::new());
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let status = watchbill::plan(&words, &self.repo.path, &self.org, &mut input, &mut out, &mut err);
        (status, text(&out), text(&err))
    }

    /// The branch `issue-54` from `main` with the issue's file committed alone, checked out. A planner's round lasts
    /// more than a second: the round starts in the next second.
    fn planned(&self) {
        std::thread::sleep(std::time::Duration::from_millis(1100));
        git(&self.repo.path, &["checkout", "-q", "-b", "issue-54", "main"]);
        put(&self.repo.path, YML, &BARE_ISSUE.replace("{issue}", "54"));
        git(&self.repo.path, &["add", "--", YML]);
        git(&self.repo.path, &["commit", "-q", "-m", "The issue's file (54)"]);
        git(&self.repo.path, &["update-ref", "refs/remotes/origin/issue-54", "HEAD"]);
    }
}

#[test]
fn test_WB_CMD_38_plan_accept_n_does_what_no_commit_does() {
    let bench = bench();
    let (status, out, err) = bench.plan(&["add", "54", "-m", "The brakes hold."]);
    assert_eq!(status, 0, "{}{}", out, err);
    bench.planned();
    let before = count(&bench.repo.path);

    let run = wb(&bench.repo.path, &["plan", "accept", "-n", "54"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(bench.repo.path.join(ACCEPTED).exists());
    assert_eq!(count(&bench.repo.path), before);
}

// ---- WB-CMD-39: -q / --quiet is taken by every command, by one reading ----

/// Each command with the words it needs, run in a folder that is no repository.
const COMMANDS: [&[&str]; 20] = [
    &["init"],
    &["check"],
    &["upgrade"],
    &["plan", "add", "-m", "x"],
    &["plan", "list"],
    &["plan", "show", "54"],
    &["plan", "edit", "54"],
    &["plan", "accept", "54"],
    &["plan", "remove", "54"],
    &["accept", "54"],
    &["start", "build", "54"],
    &["start", "--list"],
    &["status"],
    &["view", "54", "build"],
    &["answer", "54"],
    &["revert", "54", "build"],
    &["clean"],
    &["prompt", "build", "54"],
    &["revision"],
    &["compare", "a", "b"],
];

#[test]
fn test_WB_CMD_39_each_command_takes_q_and_quiet() {
    let folder = folder();
    for flag in ["-q", "--quiet"] {
        for words in COMMANDS {
            let mut args: Vec<&str> = words.to_vec();
            args.push(flag);
            let run = wb(&folder.path, &args, "");
            let said = err_of(&run);
            assert!(!said.contains("is not known") && !said.contains("is not taken") && !said.contains("not a role"), "{:?}: {}", args, said);
            assert!(!said.contains("a fault in wb"), "{:?}: {}", args, said);
            // Used rightly, or refused for another reason: not a word of the command line that was wrong.
            assert!(matches!(code(&run), Some(0..=2)), "{:?}", args);
        }
    }
}

#[test]
fn test_WB_CMD_39_q_given_twice_or_with_a_value_ends_with_2_and_the_usage() {
    let folder = folder();
    for words in COMMANDS {
        for twice in [["-q", "-q"], ["-q", "--quiet"], ["--quiet", "-q"], ["--quiet", "--quiet"]] {
            let mut args: Vec<&str> = words.to_vec();
            args.extend(twice);
            let run = wb(&folder.path, &args, "");
            assert_eq!(code(&run), Some(2), "{:?}", args);
            assert!(err_of(&run).contains("'--quiet' is given twice"), "{:?}: {}", args, err_of(&run));
            assert!(err_of(&run).contains("usage:"), "{:?}", args);
        }
        let mut args: Vec<&str> = words.to_vec();
        args.push("--quiet=yes");
        let run = wb(&folder.path, &args, "");
        assert_eq!(code(&run), Some(2), "{:?}", args);
        assert!(err_of(&run).contains("--quiet=yes"), "{:?}: {}", args, err_of(&run));
    }
}

#[test]
fn test_WB_CMD_39_q_stands_with_the_other_flags_in_any_place() {
    let folder = upgradable();

    let before = wb(&folder.path, &["upgrade", "-q", "-n"], "");
    let after = wb(&folder.path, &["upgrade", "-n", "-q"], "");

    assert_eq!(code(&before), Some(0), "{}", err_of(&before));
    assert_eq!(out_of(&before), out_of(&after));
}

#[test]
fn test_WB_CMD_39_the_short_form_is_read_once_in_the_source() {
    // One parser of -q for all commands: the word is matched in one place of `src/lib.rs`.
    let source = fs::read_to_string(root().join("src/lib.rs")).unwrap();
    let places = source.lines().filter(|line| line.contains("Word::Short('q')") || line.contains("option == \"quiet\"")).count();
    assert!(places <= 2, "{} places read -q", places);
    assert!(places >= 1);
}

// ---- WB-CMD-40: -q leaves out the lines that only inform ----

#[test]
fn test_WB_CMD_40_init_q_leaves_out_what_it_wrote_and_says_what_it_will_write() {
    let folder = repository(None);

    let plain = wb(&folder.path, &["init", "--yes"], "");
    let again = repository(None);
    let quiet = wb(&again.path, &["init", "--yes", "-q"], "");

    assert_eq!(code(&plain), Some(0));
    assert_eq!(code(&quiet), Some(0), "{}", err_of(&quiet));
    assert!(out_of(&plain).contains("Wrote ") && out_of(&plain).contains("Keep them under version control"), "{}", out_of(&plain));
    assert!(out_of(&quiet).contains("About to write"), "{}", out_of(&quiet));
    assert!(!out_of(&quiet).contains("Wrote ") && !out_of(&quiet).contains("Keep them"), "{}", out_of(&quiet));
    assert!(config(&again.path).exists());
}

#[test]
fn test_WB_CMD_40_check_q_says_nothing_where_it_holds_and_everything_where_it_does_not() {
    let folder = set_up();

    let holds = wb(&folder.path, &["check", "-q"], "");

    assert_eq!(code(&holds), Some(0), "{}", out_of(&holds));
    assert_eq!(out_of(&holds), "");
    fs::write(folder.path.join(".watchbill/revision/roles/fix.md"), "changed\n").unwrap();
    let plain = wb(&folder.path, &["check"], "");
    let quiet = wb(&folder.path, &["check", "-q"], "");
    assert_eq!(code(&quiet), Some(1));
    assert_eq!(out_of(&quiet), out_of(&plain));
}

#[test]
fn test_WB_CMD_40_upgrade_q_says_each_write_first_and_leaves_out_what_follows() {
    let folder = upgradable();

    let run = wb(&folder.path, &["upgrade", "-q"], "");

    assert_eq!(code(&run), Some(0), "{}{}", out_of(&run), err_of(&run));
    let out = out_of(&run);
    assert!(out.contains("It writes these, in this order:") && out.contains("add .watchbill/revision/roles/fix.md"), "{}", out);
    assert!(!out.contains("Written.") && !out.contains("The check of wb check") && !out.contains("Nothing is committed"), "{}", out);
    assert!(folder.path.join(".watchbill/revision/roles/fix.md").exists());
}

#[test]
fn test_WB_CMD_40_upgrade_q_leaves_out_nothing_to_do() {
    let folder = set_up();

    let plain = wb(&folder.path, &["upgrade"], "");
    let quiet = wb(&folder.path, &["upgrade", "-q"], "");

    assert!(out_of(&plain).contains("Nothing to do"), "{}", out_of(&plain));
    assert_eq!(code(&quiet), Some(0));
    assert_eq!(out_of(&quiet), "");
}

#[test]
fn test_WB_CMD_40_a_refusal_is_said_with_q() {
    let folder = upgradable();
    put(&folder.path, ".watchbill/revision/models.yml", "changed, not committed\n");

    let run = wb(&folder.path, &["upgrade", "-q"], "");

    assert_eq!(code(&run), Some(1));
    assert!(err_of(&run).contains("wb upgrade: limit: "), "{}", err_of(&run));
    let outside = wb(&self::folder().path, &["check", "-q"], "");
    assert_eq!(code(&outside), Some(2));
    assert!(err_of(&outside).contains("not in a git repository"), "{}", err_of(&outside));
}

#[test]
fn test_WB_CMD_40_accept_q_says_what_it_will_write_and_leaves_out_the_rest() {
    let folder = on_the_branch();

    let run = wb(&folder.path, &["accept", "-q", "54"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    let out = out_of(&run);
    assert!(out.contains("will write .watchbill/issues/54.accepted") && out.contains("will commit"), "{}", out);
    assert!(!out.contains("is accepted.") && !out.contains("committed .watchbill") && !out.contains("wb pushes nothing"), "{}", out);
    assert!(folder.path.join(ACCEPTED).exists());
    assert_eq!(say(&folder.path, &["log", "-1", "--format=%s"]), "Accept the issue's file (54)\n");
}

#[test]
fn test_WB_CMD_40_accept_n_q_leaves_out_the_hint_to_commit() {
    let folder = on_the_branch();

    let run = wb(&folder.path, &["accept", "-n", "-q", "54"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(out_of(&run).contains("will write"), "{}", out_of(&run));
    assert!(!out_of(&run).contains("commit the acceptance alone"), "{}", out_of(&run));
}

#[test]
fn test_WB_CMD_40_accept_q_of_a_file_that_is_not_there_is_refused_and_said() {
    let folder = set_up();

    let run = wb(&folder.path, &["accept", "-q", "54"], "");

    assert_ne!(code(&run), Some(0));
    assert!(!err_of(&run).is_empty());
    assert!(!err_of(&run).contains("is not known"), "{}", err_of(&run));
}

#[test]
fn test_WB_CMD_40_plan_add_q_says_what_it_writes_and_leaves_out_what_it_wrote() {
    let bench = bench();

    let (status, out, err) = bench.plan(&["add", "54", "-m", "The brakes hold.", "-q"]);

    assert_eq!(status, 0, "{}{}", out, err);
    assert!(out.contains("It writes:") && out.contains("the ref refs/watchbill/54 made at it"), "{}", out);
    assert!(!out.contains("Wrote ") && !out.contains("the text is "), "{}", out);
    assert!(!say(&bench.repo.path, &["rev-parse", "refs/watchbill/54"]).is_empty());
    let (status, out, _) = bench.plan(&["add", "54", "-m", "The brakes hold.", "-q"]);
    assert_eq!(status, 0);
    assert_eq!(out, "");
}

#[test]
fn test_WB_CMD_40_plan_remove_q_says_what_it_writes_and_leaves_out_what_it_wrote() {
    let bench = bench();
    let (status, out, err) = bench.plan(&["add", "54", "-m", "The brakes hold."]);
    assert_eq!(status, 0, "{}{}", out, err);

    let run = wb(&bench.repo.path, &["plan", "remove", "-q", "54"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(out_of(&run).contains("It writes:"), "{}", out_of(&run));
    assert!(!out_of(&run).contains("Wrote "), "{}", out_of(&run));
}

#[test]
fn test_WB_CMD_40_plan_accept_q_leaves_out_what_follows_the_write() {
    let bench = bench();
    let (status, out, err) = bench.plan(&["add", "54", "-m", "The brakes hold."]);
    assert_eq!(status, 0, "{}{}", out, err);
    bench.planned();

    let run = wb(&bench.repo.path, &["plan", "accept", "54", "-q"], "");

    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(out_of(&run).contains("wb plan accept: will write"), "{}", out_of(&run));
    assert!(!out_of(&run).contains("is accepted.") && !out_of(&run).contains("committed .watchbill"), "{}", out_of(&run));
}

// ---- WB-CMD-41: where a command prints nothing that only informs, -q changes nothing ----

#[test]
fn test_WB_CMD_41_where_nothing_only_informs_q_changes_nothing() {
    let bench = bench();
    let (status, out, err) = bench.plan(&["add", "54", "-m", "The brakes hold."]);
    assert_eq!(status, 0, "{}{}", out, err);
    for words in [
        &["status"][..],
        &["plan", "list"][..],
        &["plan", "show", "54"][..],
        &["view", "--kept"][..],
        &["start", "--list"][..],
        &["revision"][..],
        &["prompt", "build", "54", "--parts"][..],
    ] {
        let plain = wb(&bench.repo.path, words, "");
        let mut args = words.to_vec();
        args.push("-q");
        let quiet = wb(&bench.repo.path, &args, "");
        assert_eq!(code(&quiet), code(&plain), "{:?}", words);
        assert_eq!(out_of(&quiet), out_of(&plain), "{:?}", words);
        assert_eq!(err_of(&quiet), err_of(&plain), "{:?}", words);
    }
}

// ---- WB-CMD-42, 43 and 44: the usage, the man pages and the completion name each ----

fn built() -> Vec<String> {
    watchbill::layout::COMMANDS
        .iter()
        .filter(|command| watchbill::layout::planned(command.name).is_none())
        .map(|command| command.name.to_string())
        .collect()
}

/// The commands whose usage names `-n`, with the flag of each.
const WITH_N: [&str; 4] = ["upgrade", "accept", "plan", "clean"];

fn help(command: &str) -> String {
    watchbill::layout::help_of(command, watchbill::USAGE).unwrap()
}

#[test]
fn test_WB_CMD_42_the_usage_and_the_help_of_each_command_name_q_and_quiet() {
    assert_eq!(built().len(), 14);
    for command in built() {
        let words: Vec<String> = help(&command).split_whitespace().map(String::from).collect();
        assert!(words.iter().any(|word| word == "-q"), "{}", command);
        assert!(words.iter().any(|word| word == "--quiet" || word == "--quiet]"), "{}", command);
    }
}

#[test]
fn test_WB_CMD_42_the_usage_names_n_for_dry_run_and_no_commit() {
    for command in WITH_N {
        let words: Vec<String> = help(command).split_whitespace().map(String::from).collect();
        assert!(words.iter().any(|word| word == "-n"), "{}: {}", command, help(command));
    }
    assert!(help("upgrade").contains("[--dry-run | -n]"), "{}", help("upgrade"));
    assert!(help("accept").contains("[--no-commit | -n]"), "{}", help("accept"));
    assert!(help("plan").contains("accept <name> [--no-commit | -n]"), "{}", help("plan"));
    assert!(help("clean").contains("[--dry-run | -n]"), "{}", help("clean"));
}

#[test]
fn test_WB_CMD_42_each_command_says_in_its_paragraph_what_q_leaves_out() {
    for command in built() {
        let paragraph = help(&command).split("\n\n").nth(1).unwrap().split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(paragraph.contains("-q (--quiet)"), "{}: {}", command, paragraph);
    }
}

/// A flag as roff has it.
fn roff(flag: &str) -> String {
    flag.replace('-', "\\-")
}

#[test]
fn test_WB_CMD_43_each_man_page_names_q_and_quiet_and_n_where_the_usage_does() {
    for command in built() {
        let text = fs::read_to_string(root().join(format!("packaging/man/wb-{}.1", command))).unwrap();
        assert!(text.contains(&roff("-q")) && text.contains(&roff("--quiet")), "wb-{}.1", command);
        let options: Vec<&str> = text.lines().filter(|line| line.starts_with(".B ")).collect();
        assert!(options.iter().any(|line| line.contains(&roff("--quiet")) && line.contains(&roff("-q"))), "wb-{}.1: {:?}", command, options);
        if WITH_N.contains(&command.as_str()) {
            assert!(text.contains(&roff("-n")), "wb-{}.1", command);
        }
    }
    let upgrade = fs::read_to_string(root().join("packaging/man/wb-upgrade.1")).unwrap();
    assert!(upgrade.lines().any(|line| line.starts_with(".B ") && line.contains(&roff("--dry-run")) && line.contains(&roff("-n"))));
    let accept = fs::read_to_string(root().join("packaging/man/wb-accept.1")).unwrap();
    assert!(accept.lines().any(|line| line.starts_with(".B ") && line.contains(&roff("--no-commit")) && line.contains(&roff("-n"))));
}

fn completion() -> String {
    fs::read_to_string(root().join("packaging/wb.bash")).unwrap()
}

fn words_of(text: &str, name: &str) -> BTreeSet<String> {
    let start = format!("{}=\"", name);
    match text.lines().find(|line| line.starts_with(&start)) {
        Some(line) => line[start.len()..].trim_end_matches('"').split_whitespace().map(String::from).collect(),
        None => BTreeSet::new(),
    }
}

/// The short flags that `USAGE` names for a command: a word of `-` and one letter, in its lines.
fn usage_shorts(command: &str) -> BTreeSet<String> {
    let block = watchbill::USAGE.split("\n\n").next().unwrap();
    let mut found = BTreeSet::new();
    let mut inside = false;
    let mut needs_prompt = false;
    for line in block.lines().skip(1) {
        if line.starts_with("  wb ") {
            inside = line.split_whitespace().nth(1) == Some(command);
        }
        if inside {
            needs_prompt |= line.contains("the options of prompt");
            for word in line.split(|c: char| c.is_whitespace() || "[]()|<>".contains(c)) {
                if word.len() == 2 && word.starts_with('-') && !word.starts_with("--") {
                    found.insert(word.to_string());
                }
            }
        }
    }
    if needs_prompt {
        found.extend(usage_shorts("prompt"));
    }
    found
}

#[test]
fn test_WB_CMD_44_the_completion_offers_quiet_for_each_command() {
    let text = completion();
    for command in built() {
        assert!(words_of(&text, &format!("_wb_flags_{}", command)).contains("--quiet"), "{}", command);
    }
}

#[test]
fn test_WB_CMD_44_the_completion_offers_the_short_forms_the_usage_names_and_no_other() {
    let text = completion();
    for command in built() {
        let found = words_of(&text, &format!("_wb_short_{}", command));
        assert_eq!(found, usage_shorts(&command), "{}", command);
        assert!(found.contains("-q"), "{}", command);
    }
    assert!(usage_shorts("upgrade").contains("-n") && usage_shorts("clean").contains("-n"));
}

#[test]
fn test_WB_CMD_44_a_single_dash_word_is_completed_from_the_short_forms() {
    let text = completion();
    assert!(text.contains("_wb_short_$command"), "the completion does not read the short forms of the command");
    assert!(text.lines().any(|line| line.contains("$cur == -*")), "{}", text);
}
