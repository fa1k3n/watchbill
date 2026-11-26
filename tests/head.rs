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
    // Nothing that needs a signature counts without `.watchbill/signers` on the default branch (#323).
    signing(&repo.path);
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
        // The change is worked out by `change_of_branch`: since #236 the round runs no pattern.
        ("wb start", |b| start_with(&b.repo.path, &b.org, true, &["start", "review", "54"])),
        ("wb answer", |b| wb(&b.repo.path, &["answer", "54", "--suggested"], "")),
        // Since #236 the stamp a marker takes back is a commit on the job's ref; the marker's copy is posted by `issue_comment`.
        ("wb revert", |b| {
            stamps_on_ref(&b.repo.path, "54", &["[stamp] review round 1"]);
            wb(&b.repo.path, &["revert", "54", "review"], "")
        }),
        ("wb plan accept", |b| {
            put(&b.repo.path, ".watchbill/issues/54.yml", ISSUE);
            wb(&b.repo.path, &["plan", "accept", "--no-commit", "54"], "")
        }),
        // The acceptance of the work, for a file that is accepted (written by hand here, as `wb plan accept` writes it).
        ("wb accept", |b| {
            put(&b.repo.path, ".watchbill/issues/54.yml", ISSUE);
            let hash = watchbill::manifest::hash_bytes(ISSUE.as_bytes());
            put(&b.repo.path, ".watchbill/issues/54.accepted", &format!("sha256: {}\nby: Ada <ada@example.org>\ntime: 2026-11-24T10:00:00Z\n", hash));
            // Since #323 an acceptance counts only by its signed tag.
            sign_acceptance(&b.repo.path, "54");
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
        if name == "wb plan accept" {
            assert_eq!(code(&run), Some(0), "{}: {}", name, text(&run.stderr));
        } else if name == "wb accept" {
            // The work is not accepted here: no `issue_comments` says which roles ran. It is not the file's drift.
            assert_eq!(code(&run), Some(1), "{}: {}", name, text(&run.stderr));
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
        ("wb plan accept 54 --no-commit", wb(&bench.repo.path, &["plan", "accept", "54", "--no-commit"], "")),
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
    // The change is worked out by `change_of_branch`: since #236 the round runs no pattern.
    let run = wb(&bench.repo.path, &["prompt", "review", "54"], "");
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
    let run = wb(&bench.repo.path, &["plan", "accept", "54", "--no-commit"], "");
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
    assert_eq!(text(&run.stdout), text(&before.stdout));
    // The same as with the file of HEAD (the stand-in tracker is not there, so it ends with 1 either way), and one line more.
    let (said, rest) = text(&run.stderr).split_once('\n').map(|(a, b)| (a.to_string(), b.to_string())).unwrap();
    assert!(said.contains(DRIFT), "{}", text(&run.stderr));
    assert_eq!(rest, text(&before.stderr));
    assert!(!text(&run.stderr).contains("a person reads the change"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_BUILD_58_wb_status_warns_of_a_file_that_is_staged_and_not_committed_and_goes_on() {
    let bench = bench();
    bench.add("needs: 'true'");
    git(&bench.repo.path, &["add", "--", ".watchbill/watchbill.yml"]);
    let run = wb(&bench.repo.path, &["status"], "");
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
    // The issue's file is on the branch: a job not broken down has no line in the list (#284).
    put(&bench.repo.path, ".watchbill/issues/77.yml", "issue: '77'\n");
    git(&bench.repo.path, &["add", "--", "a.txt", ".watchbill/issues/77.yml"]);
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
fn test_WB_BUILD_61_wb_view_reads_build_files_from_head_not_the_working_tree_and_runs_no_pattern() {
    let bench = view_bench();
    let l = bench.log.path.to_str().unwrap();
    // HEAD has `build_files: [build]`; the working tree has another entry, which would change the listing.
    bench.add("build_files: [build]");
    bench.commit();
    bench.change("build_files", "build_files: [docs]");
    bench.add(&format!("needs: 'touch {l}/marker'"));
    let run = wb(&bench.repo.path, &["view", "54", "build/Makefile"], "");
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains(DRIFT), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("build: out: under `build_files` of watchbill.yml (build)"), "{}", text(&run.stdout));
    assert!(!bench.marker().exists(), "a pattern ran");
    assert_eq!(bench.ran(), "", "wb view ran a pattern");
    let other = wb(&bench.repo.path, &["view", "54", "docs/a.md"], "");
    assert!(!text(&other.stdout).contains("`build_files`"), "the working tree's build_files was read: {}", text(&other.stdout));
}

#[test]
fn test_WB_BUILD_63_a_file_missing_from_the_working_tree_but_in_head_counts_as_differing_and_the_commands_go_on() {
    let bench = view_bench();
    fs::remove_file(config(&bench.repo.path)).unwrap();
    let status = wb(&bench.repo.path, &["status"], "");
    assert_eq!(code(&status), Some(0), "{}", text(&status.stderr));
    assert!(text(&status.stderr).contains(DRIFT), "{}", text(&status.stderr));
    let view = wb(&bench.repo.path, &["view", "54", "build"], "");
    assert_eq!(code(&view), Some(0), "{}", text(&view.stderr));
    assert_eq!(text(&view.stdout), "src/brake.c\n");
    assert!(text(&view.stderr).contains(DRIFT), "{}", text(&view.stderr));
    // The pattern of HEAD ran for the status of an issue.
    let one = wb(&bench.repo.path, &["status", "54"], "");
    assert_eq!(bench.ran(), "comments\n", "{}", text(&one.stderr));
}

#[test]
fn test_WB_BUILD_63_neither_in_the_working_tree_nor_in_head_ends_with_2_as_not_set_up() {
    let repo = repository(None);
    for words in [&["status"][..], &["view", "54", "build"]] {
        let run = wb(&repo.path, words, "");
        assert_eq!(code(&run), Some(2), "{:?}: {}", words, text(&run.stderr));
        assert!(text(&run.stderr).contains("is not there. Watchbill is not set up"), "{}", text(&run.stderr));
    }
}

#[test]
fn test_WB_BUILD_64_the_read_only_mode_ends_with_the_guard_and_held_refuses_again_after_it() {
    let bench = bench();
    bench.add("needs: 'true'");
    let root = bench.repo.path.to_str().unwrap();
    assert!(watchbill::starter::held(root).is_err());
    {
        let (_guard, differs) = watchbill::starter::read_only(root).unwrap();
        assert!(differs);
        assert!(watchbill::starter::held(root).is_ok(), "the mode is not in force while the guard is held");
    }
    let why = watchbill::starter::held(root).unwrap_err();
    assert!(why.contains(".watchbill/watchbill.yml differs from HEAD"), "{}", why);
}

#[test]
fn test_WB_BUILD_58_the_warning_is_two_sentences_naming_the_file_and_the_command() {
    let bench = bench();
    bench.add("needs: 'true'");
    let run = wb(&bench.repo.path, &["status"], "");
    assert!(
        text(&run.stderr).starts_with("wb status: .watchbill/watchbill.yml differs from HEAD. It goes on with the file as HEAD has it: a pattern changed only in the working tree is not run."),
        "{}",
        text(&run.stderr)
    );
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
    let runs = [
        wb(&bench.repo.path, &["prompt", "review", "54", "--change", "9"], ""),
        start_with(&bench.repo.path, &bench.org, true, &["start", "review", "54", "--change", "9"]),
    ];
    for run in runs {
        assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
        held_words(&run);
    }
    assert_eq!(bench.ran(), "");
}

/// `wb` run for at most `seconds`: a command that hangs is killed and ends the test with a failure, not the suite.
fn wb_within(folder: &Path, args: &[&str], seconds: u64) -> Output {
    // wb is started under a cap of 1 GiB, so that a wb that reads without end
    // fails the test and does not take the memory of the machine.
    // prlimit executes wb in its own place, so the child's ID is wb's.
    let child = std::process::Command::new("prlimit")
        .arg("--as=1073741824")
        .arg("--")
        .arg(env!("CARGO_BIN_EXE_wb"))
        .args(args)
        .current_dir(folder)
        .env_clear()
        .envs(env())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("wb could not be started");
    let id = child.id();
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = send.send(child.wait_with_output());
    });
    match receive.recv_timeout(std::time::Duration::from_secs(seconds)) {
        Ok(done) => done.expect("wb could not be waited for"),
        Err(_) => {
            // Only the child this test started is ended.
            let _ = std::process::Command::new("kill").args(["-9", &id.to_string()]).status();
            panic!("wb {:?} did not end within {} seconds", args, seconds);
        }
    }
}

/// What `wb status` and `wb view` print and end with where the file in the working tree is not a plain file of
/// a size they read: the same as with the file of `HEAD`, and the warning.
fn assert_goes_on_as_with_head(bench: &Bench, what: &str) {
    for words in [&["status"][..], &["view", "54", "build"]] {
        let run = wb_within(&bench.repo.path, words, 10);
        assert_eq!(code(&run), Some(0), "{} {:?}: {}", what, words, text(&run.stderr));
        assert!(text(&run.stderr).contains(DRIFT), "{} {:?}: {}", what, words, text(&run.stderr));
    }
    let view = wb_within(&bench.repo.path, &["view", "54", "build"], 10);
    assert_eq!(text(&view.stdout), "src/brake.c\n", "{}", what);
}

#[test]
fn test_WB_BUILD_65_a_fifo_in_the_place_of_the_file_counts_as_differing_and_is_not_read() {
    let bench = view_bench();
    fs::remove_file(config(&bench.repo.path)).unwrap();
    let made = std::process::Command::new("mkfifo").arg(config(&bench.repo.path)).status().unwrap();
    assert!(made.success());
    assert_goes_on_as_with_head(&bench, "a FIFO");
}

#[test]
fn test_WB_BUILD_65_a_symlink_in_the_place_of_the_file_counts_as_differing_and_is_not_followed() {
    let bench = view_bench();
    fs::remove_file(config(&bench.repo.path)).unwrap();
    std::os::unix::fs::symlink("/dev/zero", config(&bench.repo.path)).unwrap();
    assert_goes_on_as_with_head(&bench, "a symlink to /dev/zero");
    // A wb that reads /dev/zero stops at the cap of wb_within and may still
    // count the file as differing, so the case above can pass without the
    // code. A symlink to a FIFO that no one writes to cannot: a wb that
    // follows the link waits for ever and the test fails on the time.
    let pipe = bench.log.path.join("pipe");
    let made = std::process::Command::new("mkfifo").arg(&pipe).status().unwrap();
    assert!(made.success());
    fs::remove_file(config(&bench.repo.path)).unwrap();
    std::os::unix::fs::symlink(&pipe, config(&bench.repo.path)).unwrap();
    assert_goes_on_as_with_head(&bench, "a symlink to a FIFO");
}

#[test]
fn test_WB_BUILD_65_a_file_over_1_mib_counts_as_differing_and_a_folder_does_too() {
    let bench = view_bench();
    let mut big = read(&config(&bench.repo.path));
    big.push_str(&format!("# {}\n", "x".repeat(2 * 1024 * 1024)));
    fs::write(config(&bench.repo.path), big).unwrap();
    assert_goes_on_as_with_head(&bench, "a file over 1 MiB");
    fs::remove_file(config(&bench.repo.path)).unwrap();
    fs::create_dir(config(&bench.repo.path)).unwrap();
    assert_goes_on_as_with_head(&bench, "a folder");
}

// ---- The project's text is as `HEAD` has it (#211) ----

/// A bench with issue 54 ready, and `project/all.md`, `project/build.md` and `project/review.md` committed.
fn text_bench() -> Bench {
    let bench = bench();
    issues_ready(&bench.repo.path, &["54"]);
    put(&bench.repo.path, ".watchbill/project/all.md", "Everyone: use make.\n");
    put(&bench.repo.path, ".watchbill/project/build.md", "Builders: run make test.\n");
    put(&bench.repo.path, ".watchbill/project/review.md", "Reviewers: read twice.\n");
    git(&bench.repo.path, &["add", "--", ".watchbill/project/all.md", ".watchbill/project/build.md", ".watchbill/project/review.md"]);
    git(&bench.repo.path, &["commit", "-q", "-m", "The project's text"]);
    bench
}

const BUILD_MD: &str = ".watchbill/project/build.md";

// `wb check` also refuses a FIFO or a link in the place of a project file; the check of the project's text comes
// before it, so the words of this check are the ones said, and the file is never opened.
/// `wb prompt build 54` and `wb start build 54` are both refused with 1, naming `file`, and nothing is posted or started.
fn assert_text_refused(bench: &Bench, file: &str, what: &str) {
    let prompt = wb_within(&bench.repo.path, &["prompt", "build", "54"], 10);
    assert_eq!(code(&prompt), Some(1), "{} prompt: {}", what, text(&prompt.stderr));
    assert_eq!(text(&prompt.stdout), "", "{}", what);
    assert!(text(&prompt.stderr).contains(file), "{}: the file is not named: {}", what, text(&prompt.stderr));
    assert!(text(&prompt.stderr).contains("commits it, or takes it back with git"), "{}: {}", what, text(&prompt.stderr));
    let stamp = wb_within(&bench.repo.path, &["prompt", "build", "54", "--stamp"], 10);
    assert_eq!(code(&stamp), Some(1), "{} stamp: {}", what, text(&stamp.stderr));
    assert_eq!(text(&stamp.stdout), "", "{}", what);
    // The binary, under the cap and the time of `wb_within` (#261): `start_with` runs in the test process.
    let start = wb_within(&bench.repo.path, &["start", "build", "54", "--runner", "any"], 10);
    assert_eq!(code(&start), Some(1), "{} start: {}{}", what, text(&start.stdout), text(&start.stderr));
    assert!(text(&start.stderr).contains(file), "{}: {}", what, text(&start.stderr));
    // The refusal comes before any pattern runs: not a read of the comments either.
    assert_eq!(bench.ran(), "", "{} ran something", what);
}

#[test]
fn test_WB_PROMPT_56_a_project_file_changed_and_not_committed_is_refused() {
    let bench = text_bench();
    fs::write(bench.repo.path.join(BUILD_MD), "Builders: skip the tests.\n").unwrap();
    assert_text_refused(&bench, BUILD_MD, "a changed file");
}

#[test]
fn test_WB_PROMPT_56_both_files_changed_are_both_named_and_no_pattern_runs() {
    let bench = text_bench();
    fs::write(bench.repo.path.join(BUILD_MD), "Builders: skip the tests.\n").unwrap();
    fs::write(bench.repo.path.join(".watchbill/project/all.md"), "Everyone: skip.\n").unwrap();
    let run = wb_within(&bench.repo.path, &["prompt", "build", "54"], 10);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains(".watchbill/project/all.md") && text(&run.stderr).contains(BUILD_MD), "{}", text(&run.stderr));
    let start = start_with(&bench.repo.path, &bench.org, true, &["start", "build", "54"]);
    assert_eq!(code(&start), Some(1));
    assert!(text(&start.stderr).contains(".watchbill/project/all.md") && text(&start.stderr).contains(BUILD_MD), "{}", text(&start.stderr));
    assert_eq!(bench.ran(), "");
}

#[test]
fn test_WB_PROMPT_56_a_project_file_untracked_is_refused() {
    let bench = text_bench();
    git(&bench.repo.path, &["rm", "-q", "--", ".watchbill/project/all.md"]);
    git(&bench.repo.path, &["commit", "-q", "-m", "Take the text out"]);
    put(&bench.repo.path, ".watchbill/project/all.md", "Everyone: use make.\n");
    assert_text_refused(&bench, ".watchbill/project/all.md", "an untracked file");
}

#[test]
fn test_WB_PROMPT_56_a_project_file_in_head_and_deleted_in_the_working_tree_is_refused() {
    let bench = text_bench();
    fs::remove_file(bench.repo.path.join(BUILD_MD)).unwrap();
    assert_text_refused(&bench, BUILD_MD, "a deleted file");
}

#[test]
fn test_WB_PROMPT_56_a_file_of_another_role_changed_is_not_refused() {
    let bench = text_bench();
    fs::write(bench.repo.path.join(".watchbill/project/review.md"), "Reviewers: skim.\n").unwrap();
    let run = wb_within(&bench.repo.path, &["prompt", "build", "54"], 10);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("Builders: run make test."));
    assert!(!text(&run.stdout).contains("Reviewers"));
}

#[test]
fn test_WB_PROMPT_56_each_part_hash_of_the_stamp_is_the_hash_of_the_file_in_head() {
    // By design this cannot be red on its own: a working tree that differs is refused, so where a stamp is
    // made the working tree's file and the file of `HEAD` are the same bytes. The refusals above hold it.
    use sha2::{Digest, Sha256};
    let bench = text_bench();
    let run = wb_within(&bench.repo.path, &["prompt", "build", "54", "--stamp"], 10);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    for path in [".watchbill/project/all.md", BUILD_MD] {
        let shown = std::process::Command::new("git")
            .arg("-C")
            .arg(&bench.repo.path)
            .args(["show", &format!("HEAD:{}", path)])
            .env_clear()
            .envs(env())
            .output()
            .unwrap();
        let hash: String = Sha256::digest(&shown.stdout).iter().map(|b| format!("{:02x}", b)).collect();
        assert!(out.lines().any(|l| l.starts_with("part: ") && l.contains(&hash) && l.ends_with(path)), "no part line for {} ({}) in {}", path, hash, out);
    }
}

#[test]
fn test_WB_PROMPT_56_wb_check_still_ends_with_0_with_a_project_file_changed() {
    let bench = text_bench();
    fs::write(bench.repo.path.join(BUILD_MD), "Builders: skip the tests.\n").unwrap();
    let run = wb_within(&bench.repo.path, &["check"], 10);
    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_56_a_fifo_in_the_place_of_a_project_file_is_refused_and_not_read() {
    let bench = text_bench();
    fs::remove_file(bench.repo.path.join(BUILD_MD)).unwrap();
    let made = std::process::Command::new("mkfifo").arg(bench.repo.path.join(BUILD_MD)).status().unwrap();
    assert!(made.success());
    assert_text_refused(&bench, BUILD_MD, "a FIFO");
}

#[test]
fn test_WB_PROMPT_56_a_symlink_in_the_place_of_a_project_file_is_refused_and_not_followed() {
    let bench = text_bench();
    fs::remove_file(bench.repo.path.join(BUILD_MD)).unwrap();
    std::os::unix::fs::symlink("/dev/zero", bench.repo.path.join(BUILD_MD)).unwrap();
    assert_text_refused(&bench, BUILD_MD, "a symlink to /dev/zero");
}

#[test]
fn test_WB_PROMPT_57_no_option_lifts_the_refusal() {
    let bench = text_bench();
    fs::write(bench.repo.path.join(BUILD_MD), "Builders: skip the tests.\n").unwrap();
    for flag in ["--force", "--ignore-local", "--no-check", "--head"] {
        let run = wb_within(&bench.repo.path, &["prompt", "build", "54", flag], 10);
        assert_ne!(code(&run), Some(0), "{}: {}", flag, text(&run.stdout));
        assert_eq!(text(&run.stdout), "", "{}", flag);
    }
    let run = wb_within(&bench.repo.path, &["prompt", "build", "54"], 10);
    assert_eq!(code(&run), Some(1));
}

/// `starter::project_text` run for at most 10 seconds: a call that waits on a FIFO fails the test.
fn project_text_within(root: &Path, place: &str) -> Result<Option<Vec<u8>>, String> {
    let (send, receive) = std::sync::mpsc::channel();
    let (root, place) = (root.to_path_buf(), place.to_string());
    std::thread::spawn(move || {
        let _ = send.send(watchbill::starter::project_text(&root, &place));
    });
    receive.recv_timeout(std::time::Duration::from_secs(10)).expect("project_text did not end within 10 seconds")
}

#[test]
fn test_WB_PROMPT_56_project_text_counts_a_fifo_a_link_and_a_file_over_the_cap_as_differing() {
    let repo = repository(None);
    put_committed(&repo.path, BUILD_MD, "Builders: run make test.\n");
    assert_eq!(project_text_within(&repo.path, BUILD_MD).unwrap().unwrap(), b"Builders: run make test.\n");
    let file = repo.path.join(BUILD_MD);
    fs::remove_file(&file).unwrap();
    let made = std::process::Command::new("mkfifo").arg(&file).status().unwrap();
    assert!(made.success());
    let why = project_text_within(&repo.path, BUILD_MD).unwrap_err();
    assert!(why.contains(BUILD_MD) && why.contains("differs from HEAD"), "{}", why);
    fs::remove_file(&file).unwrap();
    std::os::unix::fs::symlink("/dev/zero", &file).unwrap();
    let why = project_text_within(&repo.path, BUILD_MD).unwrap_err();
    assert!(why.contains("differs from HEAD"), "{}", why);
    fs::remove_file(&file).unwrap();
    fs::write(&file, format!("Builders: run make test.\n{}", "x".repeat(2 * 1024 * 1024))).unwrap();
    let why = project_text_within(&repo.path, BUILD_MD).unwrap_err();
    assert!(why.contains("differs from HEAD"), "{}", why);
}

#[test]
fn test_WB_PROMPT_71_a_file_over_1_mib_in_head_is_refused_as_too_big_before_it_is_read() {
    let repo = repository(None);
    put_committed(&repo.path, BUILD_MD, &format!("Builders: {}\n", "x".repeat(2 * 1024 * 1024)));
    let why = project_text_within(&repo.path, BUILD_MD).unwrap_err();
    assert!(why.contains(BUILD_MD) && why.contains("too big") && why.contains("1 MiB"), "{}", why);
    assert!(!why.contains("differs from HEAD"), "{}", why);
}

#[test]
fn test_WB_PROMPT_56_the_refusal_of_a_prompt_does_not_say_that_anything_is_posted() {
    let bench = text_bench();
    fs::write(bench.repo.path.join(BUILD_MD), "Builders: skip the tests.\n").unwrap();
    let run = wb_within(&bench.repo.path, &["prompt", "build", "54"], 10);
    assert_eq!(code(&run), Some(1));
    assert!(!text(&run.stderr).contains("posted"), "{}", text(&run.stderr));
}

// ---- One bounded reader for a file a session can write (#261) ----

/// `wb prompt build 54` and `wb start build 54` end with 1 within 10 seconds, naming `watchbill.yml`, and run nothing.
fn assert_config_refused(bench: &Bench, what: &str) {
    let prompt = wb_within(&bench.repo.path, &["prompt", "build", "54"], 10);
    assert_eq!(code(&prompt), Some(1), "{} prompt: {}", what, text(&prompt.stderr));
    assert_eq!(text(&prompt.stdout), "", "{}", what);
    assert!(text(&prompt.stderr).contains(".watchbill/watchbill.yml"), "{}: the file is not named: {}", what, text(&prompt.stderr));
    // The held check's words, not those of `wb check`, which also refuses such a file but reads it first.
    assert!(text(&prompt.stderr).contains("differs from HEAD"), "{}: {}", what, text(&prompt.stderr));
    // The binary, under the cap and the time of `wb_within`: `start_with` runs in the test process, with no bound.
    // A runner that is given skips the organisation's file, so the real path of a start reaches the held check on
    // a machine with no such file too.
    let start = wb_within(&bench.repo.path, &["start", "build", "54", "--runner", "any"], 10);
    assert_eq!(code(&start), Some(1), "{} start: {}{}", what, text(&start.stdout), text(&start.stderr));
    assert!(text(&start.stderr).contains(".watchbill/watchbill.yml"), "{}: {}", what, text(&start.stderr));
    assert!(text(&start.stderr).contains("differs from HEAD"), "{}: {}", what, text(&start.stderr));
    assert_eq!(bench.ran(), "", "{} ran something", what);
}

#[test]
fn test_WB_BUILD_100_a_fifo_in_the_place_of_watchbill_yml_is_refused_by_prompt_and_start() {
    let bench = text_bench();
    fs::remove_file(config(&bench.repo.path)).unwrap();
    let made = std::process::Command::new("mkfifo").arg(config(&bench.repo.path)).status().unwrap();
    assert!(made.success());
    assert_config_refused(&bench, "a FIFO");
}

#[test]
fn test_WB_BUILD_100_a_symlink_to_dev_zero_in_the_place_of_watchbill_yml_is_refused_by_prompt_and_start() {
    let bench = text_bench();
    fs::remove_file(config(&bench.repo.path)).unwrap();
    std::os::unix::fs::symlink("/dev/zero", config(&bench.repo.path)).unwrap();
    assert_config_refused(&bench, "a symlink to /dev/zero");
}

#[test]
fn test_WB_BUILD_100_a_folder_in_the_place_of_watchbill_yml_is_refused_by_prompt_and_start() {
    let bench = text_bench();
    fs::remove_file(config(&bench.repo.path)).unwrap();
    fs::create_dir(config(&bench.repo.path)).unwrap();
    assert_config_refused(&bench, "a folder");
}

#[test]
fn test_WB_BUILD_100_a_file_over_1_mib_in_the_place_of_watchbill_yml_is_refused_by_prompt_and_start() {
    let bench = text_bench();
    let mut big = read(&config(&bench.repo.path));
    big.push_str(&format!("# {}\n", "x".repeat(2 * 1024 * 1024)));
    fs::write(config(&bench.repo.path), big).unwrap();
    assert_config_refused(&bench, "a file over 1 MiB");
}

/// Puts a thing of the kind in the place of `watchbill.yml`: a FIFO, a folder or a link to `/dev/zero`.
fn put_thing(bench: &Bench, kind: &str) {
    fs::remove_file(config(&bench.repo.path)).unwrap();
    match kind {
        "a FIFO" => assert!(std::process::Command::new("mkfifo").arg(config(&bench.repo.path)).status().unwrap().success()),
        "a folder" => fs::create_dir(config(&bench.repo.path)).unwrap(),
        _ => std::os::unix::fs::symlink("/dev/zero", config(&bench.repo.path)).unwrap(),
    }
}

#[test]
fn test_WB_BUILD_106_accept_answer_and_revert_refuse_a_thing_in_the_place_of_watchbill_yml_with_1() {
    for kind in ["a FIFO", "a folder", "a link to /dev/zero"] {
        let bench = bench();
        put(&bench.repo.path, ".watchbill/issues/54.yml", ISSUE);
        // The work is accepted where the file is: the acceptance is written before the thing is put in place.
        assert_eq!(code(&wb_within(&bench.repo.path, &["plan", "accept", "--no-commit", "54"], 10)), Some(0));
        put_thing(&bench, kind);
        for words in [&["accept", "--no-commit", "54"][..], &["answer", "54", "--suggested"], &["revert", "54", "review"]] {
            let run = wb_within(&bench.repo.path, words, 10);
            assert_eq!(code(&run), Some(1), "{} {:?}: {}", kind, words, text(&run.stderr));
            assert!(text(&run.stderr).contains(".watchbill/watchbill.yml"), "{} {:?}: {}", kind, words, text(&run.stderr));
            assert!(!text(&run.stderr).contains("is not there"), "{} {:?}: {}", kind, words, text(&run.stderr));
        }
    }
}

#[test]
fn test_WB_BUILD_106_accept_answer_and_revert_still_end_with_2_where_watchbill_yml_is_not_there() {
    let bench = bench();
    put(&bench.repo.path, ".watchbill/issues/54.yml", ISSUE);
    fs::remove_file(config(&bench.repo.path)).unwrap();
    for words in [&["plan", "accept", "--no-commit", "54"][..], &["answer", "54", "--suggested"], &["revert", "54", "review"]] {
        let run = wb_within(&bench.repo.path, words, 10);
        assert_eq!(code(&run), Some(2), "{:?}: {}", words, text(&run.stderr));
        assert!(text(&run.stderr).contains("is not there"), "{:?}: {}", words, text(&run.stderr));
    }
}

#[test]
fn test_WB_BUILD_104_a_watchbill_yml_over_1_mib_in_head_is_refused_as_too_big() {
    let bench = bench();
    let mut big = read(&config(&bench.repo.path));
    big.push_str(&format!("# {}\n", "x".repeat(2 * 1024 * 1024)));
    fs::write(config(&bench.repo.path), big).unwrap();
    bench.commit();
    // A refusal ends with 1 in every command, `wb status` and `wb view` too (not 2, which is "not set up").
    for words in [&["prompt", "build", "54"][..], &["status"], &["view", "54", "build"]] {
        let run = wb_within(&bench.repo.path, words, 10);
        assert_eq!(code(&run), Some(1), "{:?}: {}{}", words, text(&run.stdout), text(&run.stderr));
        let err = text(&run.stderr);
        assert!(err.contains(".watchbill/watchbill.yml") && err.contains("over 1 MiB in HEAD"), "{:?}: {}", words, err);
    }
}
