//! `wb` runs a pattern of `watchbill.yml` only while the file is as `HEAD` has it (#186). Each command that runs
//! a pattern, or reads `needs` or `build_files`, is refused with 1 where the file differs from `HEAD` or is not
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
        ("wb status", |b| wb(&b.repo.path, &["status", "54"], "")),
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
        ("wb status 54 --stage accepted", wb(&bench.repo.path, &["status", "54", "--stage", "accepted"], "")),
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
    let run = wb(&bench.repo.path, &["status"], "");
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
