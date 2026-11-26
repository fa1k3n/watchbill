//! `wb plan list`, `show`, `edit`, `accept` and `remove` (#290): a plan is managed as git manages a remote or a note.
//! A plan is a job of `wb plan add` (#234), the ref `refs/watchbill/<name>`; its issue's file is
//! `.watchbill/issues/<name>.yml`, accepted as point 13 has it, and its acceptance is void after a source added or a
//! planner round after it.
//!
//! `wb plan add` is run in the test process, through `watchbill::plan`, with the organisation's file of the test. The
//! other subcommands need no organisation's file, and are run as the program, as a person runs them. The patterns of
//! `watchbill.yml` are stand-ins that read files of a folder of the test: never the network.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const ORG: &str = "levels: [public, internal, confidential]
runners:
  stand-in:
    tools: this machine
    model: a stand-in
    agreement: none
    up_to: internal
default: stand-in
";

const YML: &str = ".watchbill/issues/54.yml";
const ACCEPTED: &str = ".watchbill/issues/54.accepted";

struct Bench {
    repo: Folder,
    log: Folder,
    org: PathBuf,
    _org_folder: Folder,
}

/// What `wb plan add` ended with and printed.
struct Ran {
    code: i32,
    out: String,
    err: String,
}

fn script(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\n{}", body)).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn git_text(folder: &Path, args: &[&str]) -> String {
    let run = Command::new("git").arg("-C").arg(folder).args(args).env_clear().envs(env()).stderr(Stdio::null()).output().expect("git could not be started");
    String::from_utf8_lossy(&run.stdout).to_string()
}

/// The time now, as `wb` writes it in a stamp and an acceptance.
fn now() -> String {
    let run = Command::new("date").args(["-u", "+%Y-%m-%dT%H:%M:%SZ"]).output().unwrap();
    text(&run.stdout).trim().to_string()
}

/// Waits until the clock has passed into the next second, so that what is written after is later to the second.
fn next_second() {
    std::thread::sleep(std::time::Duration::from_millis(1100));
}

/// A repository set up by `wb init --yes` and committed on `main`, which is also `origin/main`, at the level
/// `internal`, with `issue_comments` and `issue_comment_bodies` reading `<issue>.comments` and `<issue>.bodies` of
/// the log folder; the organisation's file of `ORG`.
fn bench() -> Bench {
    let repo = repository(None);
    let log = folder_named("log");
    let l = log.path.to_str().unwrap().to_string();
    script(&log.path.join("comments.sh"), &format!("cat {l}/$1.comments 2>/dev/null\nexit 0\n"));
    script(&log.path.join("bodies.sh"), &format!("cat {l}/$1.bodies 2>/dev/null\nexit 0\n"));
    git(&repo.path, &["config", "user.name", "Ada Lovelace"]);
    git(&repo.path, &["config", "user.email", "ada@example.org"]);
    git(&repo.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&repo.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let mut yml = read(&config(&repo.path));
    for line in [
        format!("issue_comments: '{}/comments.sh {{issue}}'", l),
        format!("issue_comment_bodies: '{}/bodies.sh {{issue}}'", l),
        "confidentiality: internal".to_string(),
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
    let org_folder = folder_named("org");
    let org = org_folder.path.join("organisation.yml");
    fs::write(&org, ORG).unwrap();
    Bench { repo, log, org, _org_folder: org_folder }
}

impl Bench {
    /// `wb plan add` with the words after `plan`, in the test process.
    fn add(&self, args: &[&str]) -> Ran {
        let words: Vec<OsString> = args.iter().map(OsString::from).collect();
        let mut input = std::io::Cursor::new(Vec::new());
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = watchbill::plan(&words, &self.repo.path, &self.org, &mut input, &mut out, &mut err);
        Ran { code, out: text(&out), err: text(&err) }
    }

    /// `wb plan add` that is taken.
    fn added(&self, args: &[&str]) {
        let ran = self.add(args);
        assert_eq!(ran.code, 0, "{}{}", ran.out, ran.err);
    }

    /// `wb` as the program, in the repository.
    fn wb(&self, args: &[&str]) -> Output {
        wb(&self.repo.path, args, "")
    }

    fn git(&self, args: &[&str]) -> String {
        git_text(&self.repo.path, args)
    }

    fn at(&self, name: &str) -> PathBuf {
        self.log.path.join(name)
    }

    /// The comments of issue 54: their first lines for `issue_comments`, and the whole of each for `issue_comment_bodies`.
    fn comments(&self, bodies: &[&str]) {
        let firsts: String = bodies.iter().map(|body| format!("{}\n", body.lines().next().unwrap_or(""))).collect();
        fs::write(self.at("54.comments"), firsts).unwrap();
        let whole: String = bodies.iter().map(|body| format!("{}\0", body)).collect();
        fs::write(self.at("54.bodies"), whole).unwrap();
        // Since #236 the stamps are commits on the job's ref, after the plan's, and `wb` reads no `[stamp]` comment.
        let lines: Vec<&str> = bodies.iter().map(|body| body.lines().next().unwrap_or("")).collect();
        stamps_from_lines(&self.repo.path, "54", &lines, bodies);
    }

    /// The branch `issue-54` from `main` with the issue's file committed alone, checked out. A planner's round lasts
    /// more than a second, and a source of the acceptance's own second voids it: the round starts in the next second.
    fn planned(&self) {
        next_second();
        git(&self.repo.path, &["checkout", "-q", "-b", "issue-54", "main"]);
        put(&self.repo.path, YML, &BARE_ISSUE.replace("{issue}", "54"));
        git(&self.repo.path, &["add", "--", YML]);
        git(&self.repo.path, &["commit", "-q", "-m", "The issue's file (54)"]);
        git(&self.repo.path, &["update-ref", "refs/remotes/origin/issue-54", "HEAD"]);
    }

    /// `wb plan accept 54`, taken.
    fn accept(&self) {
        let run = self.wb(&["plan", "accept", "54"]);
        assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    }

    /// What `wb plan show 54` printed, as it ends with 0.
    fn show(&self, name: &str) -> String {
        let run = self.wb(&["plan", "show", name]);
        assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
        text(&run.stdout)
    }

    /// The line `state:` of `wb plan show`.
    fn state(&self, name: &str) -> String {
        let shown = self.show(name);
        shown.lines().find(|line| line.starts_with("state: ")).unwrap_or_else(|| panic!("no state: {}", shown)).to_string()
    }

    fn commit_of(&self, name: &str) -> String {
        self.git(&["rev-parse", &format!("refs/watchbill/{}", name)]).trim().to_string()
    }

    fn count(&self) -> usize {
        self.git(&["rev-list", "--count", "HEAD"]).trim().parse().unwrap()
    }
}

/// `wb` as the program, with the editor's variables of the test only: `VISUAL` and `EDITOR` of the person who runs the
/// tests are taken out, and `GIT_*` with them (`env`); git reads no global file of theirs, so no `core.editor` of it.
fn wb_editing(folder: &Path, args: &[&str], vars: &[(&str, &str)]) -> Output {
    let base: Vec<(String, String)> = env().into_iter().filter(|(name, _)| name != "VISUAL" && name != "EDITOR").collect();
    Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(args)
        .current_dir(folder)
        .env_clear()
        .envs(base)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .envs(vars.iter().map(|(name, value)| (name.to_string(), value.to_string())))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("wb could not be started")
}

/// An editor that writes its name in the log folder and changes the interface of the file it is given.
fn editor(bench: &Bench, name: &str) -> String {
    let path = bench.at(&format!("{}.sh", name));
    script(&path, &format!("echo {name} >> {}/editors\nsed -i 's/^  none$/  fn brake()/' \"$1\"\n", bench.log.path.display()));
    path.display().to_string()
}

// ---- WB-JOB-27: wb plan list prints one plan to a line and nothing else ----

#[test]
fn test_WB_JOB_27_list_prints_one_plan_to_a_line_and_nothing_else() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.added(&["add", "github/55", "-m", "The lights blink."]);
    // A second name of the same plan, and a second source of the first: still one line each.
    bench.added(&["add", "gitlab/56", "-m", "The lights blink."]);
    bench.added(&["add", "54", "-m", "On a slope too."]);

    let run = bench.wb(&["plan", "list"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "54\ngithub/55\n");
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_JOB_27_list_of_no_plan_prints_nothing_and_ends_with_0() {
    let bench = bench();

    let run = bench.wb(&["plan", "list"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "");
}

// ---- WB-JOB-28: a ref that is not a job, and a plan removed, are not listed ----

#[test]
fn test_WB_JOB_28_list_passes_over_a_ref_that_is_not_a_job_and_says_it_once_and_leaves_out_a_plan_removed() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.added(&["add", "57", "-m", "The doors close."]);
    let run = bench.wb(&["plan", "remove", "57"]);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let head = bench.git(&["rev-parse", "HEAD"]);
    git(&bench.repo.path, &["update-ref", "refs/watchbill/junk", head.trim()]);

    let run = bench.wb(&["plan", "list"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "54\n");
    let said = text(&run.stderr);
    assert_eq!(said.matches("refs/watchbill/junk").count(), 1, "{}", said);
}

// ---- WB-JOB-29: wb plan show names the plan's names and each source with its hash and where it came from ----

#[test]
fn test_WB_JOB_29_show_lists_every_name_and_each_source_with_its_hash_and_where_it_came_from() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold.", "--source", "https://example.invalid/issues/54"]);
    bench.added(&["add", "54", "-m", "On a slope too."]);
    bench.added(&["add", "other", "-m", "On a slope too."]);
    let first = bench.git(&["rev-parse", "refs/watchbill/54:sources"]);
    assert!(!first.is_empty());
    let hashes: Vec<String> = bench.git(&["ls-tree", "--name-only", "refs/watchbill/54:sources"]).lines().map(str::to_string).collect();
    assert_eq!(hashes.len(), 2);

    let shown = bench.show("54");

    assert!(shown.starts_with("plan 54\nnames: 54, other\n"), "{}", shown);
    let sources: Vec<&str> = shown.lines().filter(|line| line.starts_with("source ")).collect();
    assert_eq!(sources.len(), 2, "{}", shown);
    assert!(sources[0].starts_with("source 1: ") && sources[0].contains("from https://example.invalid/issues/54"), "{}", shown);
    assert!(sources[1].starts_with("source 2: ") && sources[1].contains("from none"), "{}", shown);
    for hash in &hashes {
        assert!(sources.iter().any(|line| line.contains(hash.as_str())), "{}: {}", hash, shown);
    }
    assert!(sources.iter().all(|line| line.contains("kept") && line.contains("level internal") && line.contains("by Ada Lovelace <ada@example.org>")), "{}", shown);
}

// ---- WB-JOB-30: wb plan show says the issue's file and its state ----

#[test]
fn test_WB_JOB_30_show_before_the_planner_after_it_and_after_the_acceptance() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);

    assert_eq!(bench.state("54"), "state: not planned (.watchbill/issues/54.yml is not there)");

    bench.planned();
    let shown = bench.show("54");
    assert!(shown.contains("\nissue's file: .watchbill/issues/54.yml, read from refs/heads/issue-54\nstate: not accepted\n\n"), "{}", shown);
    assert!(shown.ends_with(&format!("\n\n{}", BARE_ISSUE.replace("{issue}", "54"))), "{}", shown);

    bench.accept();
    let state = bench.state("54");
    assert!(state.starts_with("state: accepted by Ada Lovelace <ada@example.org> at 20"), "{}", state);
}

#[test]
fn test_WB_JOB_30_show_reads_the_issue_s_file_where_wb_status_does_on_the_issue_s_branch_and_says_which_it_read() {
    let bench = bench();
    bench.comments(&[]);
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    bench.accept();
    git(&bench.repo.path, &["checkout", "-q", "main"]);
    assert!(!bench.repo.path.join(YML).exists());

    let shown = bench.show("54");

    assert!(shown.contains("\nissue's file: .watchbill/issues/54.yml, read from refs/heads/issue-54\n"), "{}", shown);
    assert!(bench.state("54").starts_with("state: accepted by "), "{}", shown);
    assert!(text(&bench.wb(&["status"]).stdout).starts_with("Jobs in the flow:\n\t54  issue-54"));
}

#[test]
fn test_WB_JOB_30_show_with_no_branch_of_the_plan_reads_the_working_tree_and_says_so() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    put(&bench.repo.path, YML, &BARE_ISSUE.replace("{issue}", "54"));

    let shown = bench.show("54");

    assert!(shown.contains("\nissue's file: .watchbill/issues/54.yml, read from the working tree\nstate: not accepted\n"), "{}", shown);
}

// ---- WB-JOB-31: a name that is no plan ----

#[test]
fn test_WB_JOB_31_show_edit_and_remove_of_a_name_that_is_no_plan_end_with_1_naming_list() {
    let bench = bench();
    bench.planned();

    for args in [&["plan", "show", "54"][..], &["plan", "edit", "54"][..], &["plan", "remove", "54"][..]] {
        let run = bench.wb(args);

        assert_eq!(code(&run), Some(1), "{:?}: {}", args, text(&run.stderr));
        assert!(text(&run.stderr).contains("wb plan list"), "{:?}: {}", args, text(&run.stderr));
    }
    assert!(!bench.repo.path.join(ACCEPTED).exists());
}

// ---- WB-JOB-32: wb plan edit opens the file in git's editor and commits the edit alone ----

#[test]
fn test_WB_JOB_32_edit_opens_the_editor_git_would_use_and_commits_the_edit_alone() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    let (a, b, c) = (editor(&bench, "a"), editor(&bench, "b"), editor(&bench, "c"));
    let before = bench.count();

    // VISUAL before EDITOR, as git has it: on a terminal that is not `dumb`, as git reads VISUAL only there.
    let run = wb_editing(&bench.repo.path, &["plan", "edit", "54"], &[("TERM", "xterm"), ("EDITOR", &a), ("VISUAL", &b)]);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    assert_eq!(read(&bench.at("editors")), "b\n");
    assert_eq!(bench.count(), before + 1);
    assert_eq!(bench.git(&["log", "-1", "--format=%s"]), "Edit the issue's file (54)\n");
    assert_eq!(bench.git(&["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"]), format!("{}\n", YML));
    assert!(bench.git(&["show", &format!("HEAD:{}", YML)]).contains("  fn brake()\n"));

    // GIT_EDITOR before them both.
    git(&bench.repo.path, &["checkout", "-q", "HEAD~1", "--", YML]);
    git(&bench.repo.path, &["commit", "-q", "-m", "Back"]);
    let run = wb_editing(&bench.repo.path, &["plan", "edit", "54"], &[("TERM", "xterm"), ("EDITOR", &a), ("VISUAL", &b), ("GIT_EDITOR", &c)]);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(read(&bench.at("editors")), "b\nc\n");
}

#[test]
fn test_WB_JOB_32_edit_leaves_what_is_staged_out_of_its_commit() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    put(&bench.repo.path, "staged.txt", "staged\n");
    git(&bench.repo.path, &["add", "--", "staged.txt"]);
    let a = editor(&bench, "a");

    let run = wb_editing(&bench.repo.path, &["plan", "edit", "54"], &[("EDITOR", &a)]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(bench.git(&["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"]), format!("{}\n", YML));
    assert_eq!(bench.git(&["diff", "--cached", "--name-only"]), "staged.txt\n");
}

// ---- WB-JOB-33: an edit that changes nothing commits nothing ----

#[test]
fn test_WB_JOB_33_an_edit_that_changes_nothing_commits_nothing_and_ends_with_0() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    let before = bench.count();

    let run = wb_editing(&bench.repo.path, &["plan", "edit", "54"], &[("EDITOR", "true")]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(bench.count(), before);
    assert!(text(&run.stdout).contains("nothing changed"), "{}", text(&run.stdout));
}

// ---- WB-JOB-34: an edit is refused before the editor where it cannot be committed, and after it where it fails ----

#[test]
fn test_WB_JOB_34_edit_off_the_issue_s_branch_or_of_a_file_not_as_head_has_it_is_refused_before_the_editor() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    let a = editor(&bench, "a");

    put(&bench.repo.path, YML, &format!("{}# a change\n", BARE_ISSUE.replace("{issue}", "54")));
    let run = wb_editing(&bench.repo.path, &["plan", "edit", "54"], &[("EDITOR", &a)]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).starts_with("wb plan edit: "), "{}", text(&run.stderr));
    git(&bench.repo.path, &["checkout", "-q", "--", YML]);

    git(&bench.repo.path, &["checkout", "-q", "main"]);
    git(&bench.repo.path, &["checkout", "-q", "issue-54", "--", YML]);
    git(&bench.repo.path, &["commit", "-q", "-m", "The file on main"]);
    let run = wb_editing(&bench.repo.path, &["plan", "edit", "54"], &[("EDITOR", &a)]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("issue-54"), "{}", text(&run.stderr));

    assert!(!bench.at("editors").exists());
}

#[test]
fn test_WB_JOB_34_an_editor_that_fails_or_a_file_that_no_longer_holds_its_form_commits_nothing_and_ends_with_1() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    let before = bench.count();
    let failing = bench.at("failing.sh");
    script(&failing, "echo '# touched' >> \"$1\"\nexit 3\n");
    let breaking = bench.at("breaking.sh");
    script(&breaking, "echo 'nonsense: [' >> \"$1\"\n");

    let run = wb_editing(&bench.repo.path, &["plan", "edit", "54"], &[("EDITOR", failing.to_str().unwrap())]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert_eq!(bench.count(), before);
    assert!(read(&bench.repo.path.join(YML)).ends_with("# touched\n"));
    git(&bench.repo.path, &["checkout", "-q", "--", YML]);

    let run = wb_editing(&bench.repo.path, &["plan", "edit", "54"], &[("EDITOR", breaking.to_str().unwrap())]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert_eq!(bench.count(), before);
    // The edit is left in the working tree.
    assert!(read(&bench.repo.path.join(YML)).ends_with("nonsense: [\n"));
}

#[test]
fn test_WB_JOB_34_an_edit_whose_issue_no_longer_names_the_plan_commits_nothing_and_ends_with_1() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    let before = bench.count();
    let renaming = bench.at("renaming.sh");
    script(&renaming, "sed -i \"s/^issue: '54'$/issue: '55'/\" \"$1\"\n");

    let run = wb_editing(&bench.repo.path, &["plan", "edit", "54"], &[("EDITOR", renaming.to_str().unwrap())]);

    assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
    assert!(text(&run.stderr).contains("not 54"), "{}", text(&run.stderr));
    assert_eq!(bench.count(), before);
    assert!(read(&bench.repo.path.join(YML)).starts_with("issue: '55'\n"));
}

// ---- WB-JOB-35: an edit voids the acceptance ----

#[test]
fn test_WB_JOB_35_an_edit_after_the_acceptance_voids_it() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    bench.accept();
    let a = editor(&bench, "a");

    let run = wb_editing(&bench.repo.path, &["plan", "edit", "54"], &[("EDITOR", &a)]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(bench.state("54"), "state: not accepted (edited after the acceptance)");
    assert!(text(&run.stdout).contains("An acceptance of the file before holds no longer."), "{}", text(&run.stdout));
}

#[test]
fn test_WB_JOB_35_an_edit_of_a_file_not_accepted_says_nothing_of_an_acceptance() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    let a = editor(&bench, "a");

    let run = wb_editing(&bench.repo.path, &["plan", "edit", "54"], &[("EDITOR", &a)]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(!text(&run.stdout).contains("acceptance"), "{}", text(&run.stdout));
}

// ---- WB-JOB-36: wb plan accept is the acceptance of the issue's file ----

#[test]
fn test_WB_JOB_36_plan_accept_writes_and_commits_what_wb_accept_wrote() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    let before = bench.count();

    let run = bench.wb(&["plan", "accept", "54"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let body = read(&bench.repo.path.join(ACCEPTED));
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines.len(), 3, "{}", body);
    assert_eq!(lines[0], format!("sha256: {}", sha256_of(&bench.repo.path.join(YML))));
    assert_eq!(lines[1], "by: Ada Lovelace <ada@example.org>");
    assert!(lines[2].starts_with("time: 20") && lines[2].ends_with('Z'), "{}", body);
    assert_eq!(bench.count(), before + 1);
    assert_eq!(bench.git(&["log", "-1", "--format=%s"]), "Accept the issue's file (54)\n");
    assert_eq!(bench.git(&["diff-tree", "--no-commit-id", "--name-only", "-r", "HEAD"]), format!("{}\n", ACCEPTED));
    let out = text(&run.stdout);
    assert!(out.lines().all(|line| !line.starts_with("wb accept")), "{}", out);
    assert!(out.contains("wb plan accept: will write .watchbill/issues/54.accepted"), "{}", out);
}

#[test]
fn test_WB_JOB_36_plan_accept_no_commit_writes_it_and_commits_nothing() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    let before = bench.count();

    let run = bench.wb(&["plan", "accept", "54", "--no-commit"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(bench.repo.path.join(ACCEPTED).exists());
    assert_eq!(bench.count(), before);
}

#[test]
fn test_WB_JOB_36_plan_accept_refuses_a_file_with_a_question_under_its_own_name() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    let asked = BARE_ISSUE.replace("{issue}", "54").replace("questions: []", "questions:\n  - '54.q1: Which brake?'");
    put(&bench.repo.path, YML, &asked);
    git(&bench.repo.path, &["commit", "-q", "-a", "-m", "A question"]);

    let run = bench.wb(&["plan", "accept", "54"]);

    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stderr).starts_with("wb plan accept: "), "{}", text(&run.stderr));
    assert!(!bench.repo.path.join(ACCEPTED).exists());
}

fn sha256_of(path: &Path) -> String {
    let run = Command::new("sha256sum").arg(path).output().unwrap();
    text(&run.stdout).split_whitespace().next().unwrap().to_string()
}

// ---- WB-JOB-37: a plan whose name is not an issue's name has no issue's file ----

#[test]
fn test_WB_JOB_37_a_plan_named_with_a_slash_has_no_issue_s_file_and_edit_and_accept_end_with_1() {
    let bench = bench();
    bench.added(&["add", "github/55", "-m", "The lights blink."]);

    let shown = bench.show("github/55");
    assert!(shown.contains("\nissue's file: none"), "{}", shown);
    assert!(shown.contains("\nstate: not planned"), "{}", shown);

    for args in [&["plan", "accept", "github/55"][..], &["plan", "edit", "github/55"][..]] {
        let run = wb_editing(&bench.repo.path, args, &[("EDITOR", "true")]);
        assert_eq!(code(&run), Some(1), "{:?}: {}", args, text(&run.stderr));
        assert!(text(&run.stderr).contains("issue's name"), "{:?}: {}", args, text(&run.stderr));
    }
}

// ---- WB-JOB-38: wb plan remove adds a marker and rewrites nothing ----

#[test]
fn test_WB_JOB_38_remove_adds_a_commit_with_a_marker_after_the_plan_s_and_moves_every_name() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.added(&["add", "other", "-m", "The brakes hold."]);
    let before = bench.commit_of("54");

    let run = bench.wb(&["plan", "remove", "54"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let after = bench.commit_of("54");
    assert_ne!(after, before);
    assert_eq!(bench.commit_of("other"), after);
    assert_eq!(bench.git(&["rev-parse", &format!("{}^", after)]).trim(), before);
    assert_eq!(bench.git(&["log", "-1", "--format=%s", &after]), "plan remove 54\n");
    let tree = bench.git(&["ls-tree", "--name-only", &after]);
    assert_eq!(tree, "job.yml\nremoved\nsources\n");
    let marker = bench.git(&["cat-file", "blob", &format!("{}:removed", after)]);
    assert!(marker.contains("by: Ada Lovelace <ada@example.org>\n") && marker.contains("time: 20"), "{}", marker);
    // The job of the commit before is as it was.
    assert_eq!(bench.git(&["rev-parse", &format!("{}:job.yml", after)]), bench.git(&["rev-parse", &format!("{}:job.yml", before)]));
    // It says what it writes before it writes it.
    let out = text(&run.stdout);
    let says = out.find("It writes:").expect(&out);
    let wrote = out.find("Wrote ").expect(&out);
    assert!(says < wrote, "{}", out);
    // A plan removed is no plan.
    assert_eq!(code(&bench.wb(&["plan", "show", "54"])), Some(1));
    assert_eq!(code(&bench.wb(&["plan", "remove", "other"])), Some(1));
}

// ---- WB-JOB-39: wb plan remove is refused while a role's work on the plan is not done ----

#[test]
fn test_WB_JOB_39_remove_is_refused_while_a_branch_of_the_plan_holds_work_not_on_the_default_branch() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    let before = bench.commit_of("54");

    let run = bench.wb(&["plan", "remove", "54"]);

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("issue-54"), "{}", text(&run.stderr));
    assert_eq!(bench.commit_of("54"), before);
}

#[test]
fn test_WB_JOB_39_remove_is_refused_while_a_branch_of_another_name_of_the_plan_holds_work() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.added(&["add", "other", "-m", "The brakes hold."]);
    git(&bench.repo.path, &["checkout", "-q", "-b", "issue-other", "main"]);
    git(&bench.repo.path, &["commit", "-q", "--allow-empty", "-m", "Work"]);
    git(&bench.repo.path, &["checkout", "-q", "main"]);
    let before = bench.commit_of("54");

    let run = bench.wb(&["plan", "remove", "54"]);

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("refs/heads/issue-other"), "{}", text(&run.stderr));
    assert_eq!(bench.commit_of("54"), before);
    assert_eq!(bench.commit_of("other"), before);
}

// ---- WB-JOB-40: a removed plan's name stays taken ----

#[test]
fn test_WB_JOB_40_add_under_a_removed_plan_s_name_ends_with_1_and_its_text_is_not_taken_as_that_plan_s() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    assert_eq!(code(&bench.wb(&["plan", "remove", "54"])), Some(0));
    let removed = bench.commit_of("54");

    let ran = bench.add(&["add", "54", "-m", "Other text."]);
    assert_eq!(ran.code, 1, "{}{}", ran.out, ran.err);
    assert!(ran.err.contains("removed"), "{}", ran.err);
    assert_eq!(bench.commit_of("54"), removed);

    bench.added(&["add", "fresh", "-m", "The brakes hold."]);
    assert_eq!(bench.commit_of("54"), removed);
    let job = bench.git(&["cat-file", "blob", "refs/watchbill/fresh:job.yml"]);
    assert!(job.contains("names:\n  - fresh\nsources:"), "{}", job);
}

// ---- WB-JOB-41: a source added after the acceptance voids it ----

#[test]
fn test_WB_JOB_41_a_source_added_after_the_acceptance_voids_it_in_show_status_and_the_gate() {
    let bench = bench();
    bench.comments(&[]);
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    bench.accept();
    assert!(bench.state("54").starts_with("state: accepted by "));
    let run = bench.wb(&["status"]);
    assert!(text(&run.stdout).starts_with("Jobs in the flow:\n\t54  issue-54"), "{}", text(&run.stdout));
    next_second();

    bench.added(&["add", "54", "-m", "On a slope too."]);

    assert_eq!(bench.state("54"), "state: not accepted (a source added after the acceptance)");
    let run = bench.wb(&["status"]);
    assert!(text(&run.stdout).starts_with("Jobs waiting for you:\n") && text(&run.stdout).contains("\t54  issue-54") && text(&run.stdout).contains("  planned"), "{}", text(&run.stdout));
    let run = bench.wb(&["view", "54", "tester", "--in"]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    assert!(text(&run.stderr).contains("a source added after"), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("wb plan accept 54"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_JOB_41_a_source_added_in_the_same_second_as_the_acceptance_voids_it() {
    let bench = bench();
    bench.comments(&[]);
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    bench.accept();

    // No wait: the source is of the acceptance's own second, or of the next.
    bench.added(&["add", "54", "-m", "On a slope too."]);

    assert_eq!(bench.state("54"), "state: not accepted (a source added after the acceptance)");
    let run = bench.wb(&["status"]);
    assert!(text(&run.stdout).starts_with("Jobs waiting for you:\n") && text(&run.stdout).contains("\t54  issue-54") && text(&run.stdout).contains("  planned"), "{}", text(&run.stdout));
}

// ---- WB-JOB-42: a planner round after the acceptance voids it, even where the plan comes out the same ----

#[test]
fn test_WB_JOB_42_a_planner_stamp_after_the_acceptance_voids_it_and_accepting_again_restores_it() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    // A planner round before the acceptance counts for nothing against it.
    bench.comments(&["[stamp] planner round 1\nrole: planner\ntime: 2000-01-01T00:00:00Z\n"]);
    bench.accept();
    assert!(bench.state("54").starts_with("state: accepted by "));
    assert!(text(&bench.wb(&["status"]).stdout).starts_with("Jobs in the flow:\n\t54  issue-54"));

    // A second round, which changed nothing: its stamp is of this second, the same as or after the acceptance.
    bench.comments(&[
        "[stamp] planner round 1\nrole: planner\ntime: 2000-01-01T00:00:00Z\n",
        &format!("[stamp] planner round 2\nrole: planner\ntime: {}\n", now()),
    ]);

    assert_eq!(bench.state("54"), "state: not accepted (planned again after the acceptance)");
    let run = bench.wb(&["status"]);
    assert!(text(&run.stdout).starts_with("Jobs waiting for you:\n") && text(&run.stdout).contains("\t54  issue-54") && text(&run.stdout).contains("  planned"), "{}", text(&run.stdout));

    // The person's word again.
    next_second();
    git(&bench.repo.path, &["rm", "-q", "--", ACCEPTED]);
    git(&bench.repo.path, &["commit", "-q", "-m", "The acceptance taken out"]);
    bench.accept();

    assert!(bench.state("54").starts_with("state: accepted by "), "{}", bench.show("54"));
    assert!(text(&bench.wb(&["status"]).stdout).starts_with("Jobs in the flow:\n\t54  issue-54"));
}

#[test]
fn test_WB_JOB_42_a_planner_stamp_taken_back_by_a_revert_voids_nothing_in_show_or_status() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    bench.comments(&[]);
    bench.accept();

    // A build round, a planner round after the acceptance, then the build round taken back with all after it.
    bench.comments(&[
        "[stamp] build round 1\nrole: build\ntime: 2000-01-01T00:00:00Z\n",
        &format!("[stamp] planner round 2\nrole: planner\ntime: {}\n", now()),
        "[revert] build round 1\n",
    ]);

    assert!(bench.state("54").starts_with("state: accepted by "), "{}", bench.show("54"));
    let run = bench.wb(&["status"]);
    assert!(text(&run.stdout).starts_with("Jobs in the flow:\n\t54  issue-54"), "{}", text(&run.stdout));
}

// ---- WB-JOB-43: wb accept of a plan not accepted names wb plan accept ----

#[test]
fn test_WB_JOB_43_wb_accept_of_a_plan_not_accepted_ends_with_2_naming_wb_plan_accept() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();

    let run = bench.wb(&["accept", "54"]);

    assert_eq!(code(&run), Some(2), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("wb plan accept 54"), "{}", text(&run.stderr));
    assert!(!bench.repo.path.join(ACCEPTED).exists());
}

#[test]
fn test_WB_JOB_43_wb_accept_of_a_name_with_no_plan_and_a_file_not_accepted_ends_with_2_and_wb_plan_accept_accepts_it() {
    let bench = bench();
    bench.planned();

    let run = bench.wb(&["accept", "54"]);

    assert_eq!(code(&run), Some(2), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("wb plan accept 54"), "{}", text(&run.stderr));
    assert!(!bench.repo.path.join(ACCEPTED).exists());
    let run = bench.wb(&["plan", "accept", "54"]);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(bench.repo.path.join(ACCEPTED).exists());
}

#[test]
fn test_WB_JOB_43_wb_accept_of_a_plan_whose_acceptance_a_planner_round_voided_ends_with_2_naming_wb_plan_accept() {
    let bench = bench();
    bench.added(&["add", "54", "-m", "The brakes hold."]);
    bench.planned();
    bench.comments(&[]);
    bench.accept();
    bench.comments(&[&format!("[stamp] planner round 1\nrole: planner\ntime: {}\n", now())]);

    let run = bench.wb(&["accept", "54"]);

    assert_eq!(code(&run), Some(2), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("wb plan accept 54"), "{}", text(&run.stderr));
}
