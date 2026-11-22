//! `wb` runs a pattern of `watchbill.yml` only while the file is as `HEAD` has it (#186). Each command that runs
//! a pattern and hands something out or writes (not `wb status` and `wb view`, which only read: #254, below),
//! is refused with 1 where the file differs from `HEAD` or is not
//! committed, before anything runs or is posted. The patterns are stand-ins that write a file in a folder of the
//! test: never the network.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;

/// The organisation's file: one runner, `stand-in`, whose `start` writes a file.
const ORG: &str = "levels: [public, internal]
default: stand-in
runners:
  stand-in:
    tools: this machine
    model: a stand-in
    agreement: none
    up_to: internal
    start: ['{log}/start.sh', '{prompt}']
";

const ISSUE: &str = "issue: '54'
from: 3f0c1d9e
requirements:
  - id: '54.1'
    text: The brake holds.
    source: the goal
interface: |
  int brake_hold(void);
changes: [src/brake.c]
reads:
  tester: []
  build: []
  review: []
  fix: []
existing_tests_changed: []
questions: []
";

struct Bench {
    repo: Folder,
    log: Folder,
    org: PathBuf,
    _org_folder: Folder,
}

impl Bench {
    /// Every stand-in writes a line to `ran`.
    fn ran(&self) -> String {
        fs::read_to_string(self.log.path.join("ran")).unwrap_or_default()
    }

    /// Adds a line to `watchbill.yml`, in the working tree only.
    fn add(&self, line: &str) {
        let mut yml = read(&config(&self.repo.path));
        yml.push_str(line);
        yml.push('\n');
        fs::write(config(&self.repo.path), yml).unwrap();
    }

    fn commit(&self) {
        git(&self.repo.path, &["add", "--", ".watchbill/watchbill.yml"]);
        git(&self.repo.path, &["commit", "-q", "-m", "The patterns"]);
    }
}

fn script(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\n{}", body)).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// A repository set up by `wb init --yes`, committed, with every pattern a stand-in that writes to `ran`.
fn bench() -> Bench {
    let repo = repository(None);
    let log = folder_named("log");
    let l = log.path.to_str().unwrap().to_string();
    for name in ["post", "comments", "changes", "start"] {
        script(
            &log.path.join(format!("{}.sh", name)),
            &format!("echo {} >> {}/ran\ncase {} in comments) echo '[stamp] review round 1';; changes) echo 174;; esac\n", name, l, name),
        );
    }
    git(&repo.path, &["config", "user.name", "Ada Lovelace"]);
    git(&repo.path, &["config", "user.email", "ada@example.org"]);
    git(&repo.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&repo.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let mut yml = read(&config(&repo.path));
    for line in [
        format!("issue_comment: '{l}/post.sh'"),
        format!("issue_comments: '{l}/comments.sh'"),
        format!("issue_comment_bodies: '{l}/comments.sh'"),
        format!("change_comment: '{l}/post.sh'"),
        format!("change_comments: '{l}/comments.sh'"),
        format!("change_of_branch: '{l}/changes.sh'"),
        "issue_address: 'tracker {issue}'".to_string(),
        "change_address: 'server {change}'".to_string(),
    ] {
        yml.push_str(&format!("{}\n", line));
    }
    fs::write(config(&repo.path), yml).unwrap();
    git(&repo.path, &["add", "--", ".watchbill"]);
    git(&repo.path, &["commit", "-q", "-m", "Watchbill"]);
    git(&repo.path, &["branch", "-M", "main"]);
    git(&repo.path, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(&repo.path, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"]);
    git(&repo.path, &["branch", "issue-54"]);
    let org_folder = folder_named("org");
    let org = org_folder.path.join("organisation.yml");
    fs::write(&org, ORG.replace("{log}", &l)).unwrap();
    Bench { repo, log, org, _org_folder: org_folder }
}

type Command = fn(&Bench) -> Output;

/// The commands that run a pattern or read `needs`, each as a function of the bench.
fn commands() -> Vec<(&'static str, Command)> {
    vec![
        ("wb start", |b| start_with(&b.repo.path, &b.org, true, &["start", "review", "54", "--change", "9"])),
        ("wb answer", |b| wb(&b.repo.path, &["answer", "54", "--suggested"], "")),
        ("wb revert", |b| wb(&b.repo.path, &["revert", "54", "review"], "")),
        ("wb accept", |b| {
            put(&b.repo.path, ".watchbill/issues/54.yml", ISSUE);
            wb(&b.repo.path, &["accept", "--no-commit", "54"], "")
        }),
    ]
}

fn held_words(run: &Output) {
    let err = text(&run.stderr);
    assert!(err.contains(".watchbill/watchbill.yml"), "the file is not named: {}", err);
    assert!(err.contains("a person reads the change and commits it, or takes it back with git, before the next command"), "{}", err);
}

#[test]
fn test_WB_BUILD_39_a_changed_file_that_is_not_committed_refuses_each_command_before_a_pattern_runs() {
    for (name, command) in commands() {
        let bench = bench();
        bench.add("needs: 'echo needs >> ran-needs'");
        let run = command(&bench);
        assert_eq!(code(&run), Some(1), "{}: {}{}", name, text(&run.stdout), text(&run.stderr));
        held_words(&run);
        assert_eq!(bench.ran(), "", "{} ran a pattern", name);
        assert!(!bench.repo.path.join("ran-needs").exists(), "{} ran needs", name);
    }
}

#[test]
fn test_WB_BUILD_39_a_changed_file_that_is_staged_and_not_committed_is_refused() {
    for (name, command) in commands() {
        let bench = bench();
        bench.add("needs: 'true'");
        git(&bench.repo.path, &["add", "--", ".watchbill/watchbill.yml"]);
        let run = command(&bench);
        assert_eq!(code(&run), Some(1), "{}: {}", name, text(&run.stderr));
        held_words(&run);
        assert_eq!(bench.ran(), "", "{} ran a pattern", name);
    }
}

#[test]
fn test_WB_BUILD_39_a_file_that_is_not_in_head_at_all_is_refused() {
    for (name, command) in commands() {
        let bench = bench();
        git(&bench.repo.path, &["rm", "-q", "--cached", "--", ".watchbill/watchbill.yml"]);
        git(&bench.repo.path, &["commit", "-q", "-m", "Take the file out of git"]);
        let run = command(&bench);
        assert_eq!(code(&run), Some(1), "{}: {}", name, text(&run.stderr));
        held_words(&run);
        assert_eq!(bench.ran(), "", "{} ran a pattern", name);
    }
}

#[test]
fn test_WB_BUILD_39_the_same_file_committed_is_run_as_before() {
    for (name, command) in commands() {
        let bench = bench();
        bench.add("needs: 'true'");
        bench.commit();
        let run = command(&bench);
        assert!(!text(&run.stderr).contains("a person reads the change"), "{}: {}", name, text(&run.stderr));
        if name == "wb accept" {
            assert_eq!(code(&run), Some(0), "{}: {}", name, text(&run.stderr));
        } else {
            assert_ne!(bench.ran(), "", "{} ran no pattern: {}", name, text(&run.stderr));
        }
    }
}

#[test]
fn test_WB_BUILD_40_no_option_that_a_command_takes_lifts_the_refusal() {
    let bench = bench();
    bench.add("needs: 'true'");
    put(&bench.repo.path, ".watchbill/issues/54.yml", ISSUE);
    let by_start = |words: &[&str]| start_with(&bench.repo.path, &bench.org, true, words);
    let runs: Vec<(&str, Output)> = vec![
        ("wb accept 54 --no-commit", wb(&bench.repo.path, &["accept", "54", "--no-commit"], "")),
        ("wb answer 54 --suggested", wb(&bench.repo.path, &["answer", "54", "--suggested"], "")),
        ("wb revert 54 review --round 1", wb(&bench.repo.path, &["revert", "54", "review", "--round", "1"], "")),
        ("wb start review 54 --change 9 --runner stand-in", by_start(&["start", "review", "54", "--change", "9", "--runner", "stand-in"])),
        ("wb prompt review 54 --round 1 --change 9", wb(&bench.repo.path, &["prompt", "review", "54", "--round", "1", "--change", "9"], "")),
    ];
    for (name, run) in runs {
        assert_eq!(code(&run), Some(1), "{}: {}", name, text(&run.stderr));
        held_words(&run);
    }
    assert_eq!(bench.ran(), "");
}

#[test]
fn test_WB_BUILD_39_a_replace_ref_does_not_put_a_changed_file_into_head() {
    for (name, command) in commands() {
        let bench = bench();
        git(&bench.repo.path, &["checkout", "-q", "-b", "side"]);
        bench.add("needs: 'echo needs >> ran-needs'");
        bench.commit();
        git(&bench.repo.path, &["checkout", "-q", "main"]);
        // The branch and `git status` stay as they were; only `git show HEAD:` would see the other file.
        git(&bench.repo.path, &["replace", "--force", "main", "side"]);
        git(&bench.repo.path, &["--no-replace-objects", "checkout", "-q", "side", "--", ".watchbill/watchbill.yml"]);
        git(&bench.repo.path, &["--no-replace-objects", "reset", "-q", "--", ".watchbill/watchbill.yml"]);
        let run = command(&bench);
        assert_eq!(code(&run), Some(1), "{}: {}{}", name, text(&run.stdout), text(&run.stderr));
        held_words(&run);
        assert_eq!(bench.ran(), "", "{} ran a pattern", name);
    }
}

#[test]
fn test_WB_BUILD_39_wb_prompt_and_wb_start_prompt_are_held_too() {
    let bench = bench();
    bench.add("issue_address: 'tell the next session to run something'");
    for words in [&["prompt", "review", "54"][..], &["prompt", "tester", "54", "--stamp"], &["start", "review", "54", "--prompt"]] {
        let run = wb(&bench.repo.path, words, "");
        assert_eq!(code(&run), Some(1), "{:?}: {}", words, text(&run.stderr));
        held_words(&run);
        assert_eq!(text(&run.stdout), "", "{:?}", words);
    }
    assert_eq!(bench.ran(), "");
}

#[test]
fn test_WB_BUILD_39_wb_prompt_with_the_file_committed_runs_the_pattern_that_works_out_the_round() {
    let bench = bench();
    bench.add("needs: 'true'");
    bench.commit();
    let run = wb(&bench.repo.path, &["prompt", "review", "54", "--change", "9"], "");
    assert!(!text(&run.stderr).contains("a person reads the change"), "{}", text(&run.stderr));
    assert_ne!(bench.ran(), "", "{}", text(&run.stderr));
}

#[test]
fn test_WB_BUILD_41_the_refusal_names_the_file_and_says_what_a_person_does() {
    let bench = bench();
    bench.add("needs: 'true'");
    let run = wb(&bench.repo.path, &["revert", "54", "review"], "");
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    held_words(&run);
    assert!(text(&run.stderr).contains("HEAD"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_BUILD_42_wb_init_is_not_held_where_the_file_is_not_in_head() {
    let repo = repository(None);
    let run = wb(&repo.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(config(&repo.path).is_file());
    assert!(!text(&run.stderr).contains("a person reads the change"));
}

// ---- #254: the two commands that only read say that the file differs, and go on with the file of HEAD ----

const DRIFT: &str = ".watchbill/watchbill.yml differs from HEAD";

/// The bench with the accepted file of issue 54 and the code it names committed: `wb view` reads them.
fn view_bench() -> Bench {
    let bench = bench();
    put(&bench.repo.path, ".watchbill/issues/54.yml", ISSUE);
    let run = wb(&bench.repo.path, &["accept", "54", "--no-commit"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    put(&bench.repo.path, "src/brake.c", "int brake_hold(void) { return 1; }\n");
    put(&bench.repo.path, "build/Makefile", "all:\n");
    git(&bench.repo.path, &["add", "--", ".watchbill/issues", "src", "build"]);
    git(&bench.repo.path, &["commit", "-q", "-m", "The issue and the code"]);
    bench
}

impl Bench {
    /// Replaces the line of the key `key` in `watchbill.yml`, in the working tree only.
    fn change(&self, key: &str, line: &str) {
        let yml = read(&config(&self.repo.path));
        let mut found = false;
        let mut changed: Vec<String> = yml
            .lines()
            .map(|old| {
                if old.starts_with(&format!("{}:", key)) {
                    found = true;
                    line.to_string()
                } else {
                    old.to_string()
                }
            })
            .collect();
        assert!(found, "no line for {}", key);
        changed.push(String::new());
        fs::write(config(&self.repo.path), changed.join("\n")).unwrap();
    }

    fn marker(&self) -> PathBuf {
        self.log.path.join("marker")
    }
}

#[test]
fn test_WB_BUILD_58_wb_status_with_a_changed_file_says_so_in_one_line_and_ends_as_it_would_with_the_file_of_head() {
    let same = bench();
    let before = wb(&same.repo.path, &["status", "54"], "");
    let bench = bench();
    bench.add("needs: 'true'");
    let run = wb(&bench.repo.path, &["status", "54"], "");
    assert_eq!(code(&run), code(&before), "{}", text(&run.stderr));
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), text(&before.stdout));
    assert!(text(&before.stderr).is_empty(), "{}", text(&before.stderr));
    assert!(text(&run.stderr).contains(DRIFT), "{}", text(&run.stderr));
    assert_eq!(text(&run.stderr).lines().count(), 1, "{}", text(&run.stderr));
    assert!(!text(&run.stderr).contains("a person reads the change"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_BUILD_58_wb_status_warns_of_a_file_that_is_staged_and_not_committed_and_goes_on() {
    let bench = bench();
    bench.add("needs: 'true'");
    git(&bench.repo.path, &["add", "--", ".watchbill/watchbill.yml"]);
    let run = wb(&bench.repo.path, &["status", "54", "--stage", "accepted"], "");
    assert!(text(&run.stderr).contains(DRIFT), "{}", text(&run.stderr));
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
}

#[test]
fn test_WB_BUILD_58_wb_status_with_the_file_as_head_has_it_says_nothing_of_it() {
    let bench = bench();
    let run = wb(&bench.repo.path, &["status", "54"], "");
    assert!(!text(&run.stderr).contains("differs from HEAD"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_BUILD_59_a_pattern_changed_only_in_the_working_tree_is_not_run_by_wb_status() {
    let bench = bench();
    let l = bench.log.path.to_str().unwrap();
    bench.change("issue_comments", &format!("issue_comments: 'touch {l}/marker; {l}/comments.sh'"));
    let run = wb(&bench.repo.path, &["status", "54"], "");
    assert!(text(&run.stderr).contains(DRIFT), "{}", text(&run.stderr));
    assert!(!bench.marker().exists(), "the pattern of the working tree ran");
    // The pattern of HEAD ran.
    assert_eq!(bench.ran(), "comments\n", "{}", text(&run.stderr));
}

#[test]
fn test_WB_BUILD_59_wb_status_takes_the_branch_pattern_from_head_not_the_working_tree() {
    let bench = bench();
    git(&bench.repo.path, &["checkout", "-q", "-b", "issue-77"]);
    put(&bench.repo.path, "a.txt", "a\n");
    git(&bench.repo.path, &["add", "--", "a.txt"]);
    git(&bench.repo.path, &["commit", "-q", "-m", "Work"]);
    git(&bench.repo.path, &["checkout", "-q", "main"]);
    bench.add("branch: 'other-{issue}'");
    let run = wb(&bench.repo.path, &["status"], "");
    assert!(text(&run.stderr).contains(DRIFT), "{}", text(&run.stderr));
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("77"), "the branch pattern of HEAD was not read: {}", text(&run.stdout));
}

#[test]
fn test_WB_BUILD_59_a_replace_ref_does_not_put_a_pattern_into_what_wb_status_runs() {
    let bench = bench();
    let l = bench.log.path.to_str().unwrap();
    git(&bench.repo.path, &["checkout", "-q", "-b", "side"]);
    bench.change("issue_comments", &format!("issue_comments: 'touch {l}/marker'"));
    bench.commit();
    git(&bench.repo.path, &["checkout", "-q", "main"]);
    git(&bench.repo.path, &["replace", "--force", "main", "side"]);
    git(&bench.repo.path, &["--no-replace-objects", "checkout", "-q", "side", "--", ".watchbill/watchbill.yml"]);
    git(&bench.repo.path, &["--no-replace-objects", "reset", "-q", "--", ".watchbill/watchbill.yml"]);
    let run = wb(&bench.repo.path, &["status", "54"], "");
    assert!(!bench.marker().exists(), "{}", text(&run.stderr));
    assert_eq!(bench.ran(), "comments\n", "{}", text(&run.stderr));
}

#[test]
fn test_WB_BUILD_60_wb_view_with_a_changed_file_says_so_in_one_line_and_goes_on() {
    let bench = view_bench();
    bench.add("needs: 'true'");
    let run = wb(&bench.repo.path, &["view", "54", "build"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "src/brake.c\n");
    assert_eq!(text(&run.stderr).lines().count(), 1, "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains(DRIFT), "{}", text(&run.stderr));
    let again = wb(&bench.repo.path, &["view", "54", "src/other.c"], "");
    assert!(text(&again.stderr).contains(DRIFT), "{}", text(&again.stderr));
}

#[test]
fn test_WB_BUILD_61_wb_view_reads_build_files_and_runs_needs_from_head_not_the_working_tree() {
    let bench = view_bench();
    let l = bench.log.path.to_str().unwrap();
    // HEAD has no build_files: the folder `build` is a path like another there. The working tree names it.
    bench.add("build_files: [build]");
    bench.add(&format!("needs: 'touch {l}/marker'"));
    let run = wb(&bench.repo.path, &["view", "54", "build/Makefile"], "");
    assert!(text(&run.stderr).contains(DRIFT), "{}", text(&run.stderr));
    assert!(!bench.marker().exists(), "a pattern of the working tree ran");
    assert!(!text(&run.stdout).contains("build_files"), "{}", text(&run.stdout));
    assert!(text(&run.stdout).contains("build: out: not listed"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_BUILD_62_wb_status_and_wb_view_end_with_2_where_the_file_is_not_in_head_at_all() {
    for words in [&["status", "54"][..], &["view", "54", "build"]] {
        let bench = view_bench();
        git(&bench.repo.path, &["rm", "-q", "--cached", "--", ".watchbill/watchbill.yml"]);
        git(&bench.repo.path, &["commit", "-q", "-m", "Take the file out of git"]);
        let run = wb(&bench.repo.path, words, "");
        assert_eq!(code(&run), Some(2), "{:?}: {}", words, text(&run.stderr));
        assert!(text(&run.stderr).contains(".watchbill/watchbill.yml is not in HEAD"), "{}", text(&run.stderr));
        assert_eq!(bench.ran(), "", "{:?}", words);
    }
}

#[test]
fn test_WB_BUILD_39_wb_prompt_and_wb_start_are_still_refused_with_1_where_the_file_differs() {
    let bench = bench();
    bench.add("needs: 'true'");
    for words in [&["prompt", "review", "54", "--change", "9"][..], &["start", "review", "54", "--change", "9"]] {
        let run = wb(&bench.repo.path, words, "");
        assert_eq!(code(&run), Some(1), "{:?}: {}", words, text(&run.stderr));
        held_words(&run);
    }
    assert_eq!(bench.ran(), "");
}
