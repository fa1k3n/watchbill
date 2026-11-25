//! A stamp is a signed commit on the job's ref, and its push is the lock (#236; `docs/revision.md`, points 12 and 15).
//!
//! Each test makes its repository, its stand-ins and, where it signs, its own SSH keys, in folders of its own: `HOME`,
//! `XDG_CONFIG_HOME` and `GIT_CONFIG_GLOBAL` point at a home folder of the test, and no ssh agent is handed on, so no
//! person's key, agent or git config is ever read or touched. `wb start`, `wb prompt` and `wb plan add`, which `wb` as a
//! program runs only with the organisation's file of `/etc` and at a terminal, run in a child of this test binary with
//! that same environment. The remote, where there is one, is a bare repository in a folder of the test: never the
//! network. The patterns of `watchbill.yml` are stand-ins that write what they were given in a folder of the test.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const YML: &str = ".watchbill/issues/54.yml";
const SIGNERS: &str = ".watchbill/signers";
const JOB: &str = "refs/watchbill/54";

/// The options `wb prompt` of a role needs beside the issue.
const REST: [&str; 8] = ["--branch", "issue-54", "--issue_address", "tracker 54", "--change", "34", "--change_address", "server 34"];

/// The organisation's file: `stand-in` has `start`, with the folder of the stand-ins bound writable.
const ORG: &str = "levels: [public, internal, confidential]
runners:
  stand-in:
    tools: this machine
    model: a stand-in
    agreement: none
    up_to: internal
    start: ['{log}/start.sh', '{model}', '{effort}', '{prompt}']
    binds: [{writable: '{log}'}]
default: stand-in
";

/// A repository of a test, the folder of its stand-ins, the home folder its git and `wb` run in, and the
/// organisation's file.
struct Bench {
    repo: Folder,
    log: Folder,
    home: Folder,
    org: PathBuf,
    _org: Folder,
    _origin: Option<Folder>,
}

/// The environment of a test: that of `common::env`, with the home, git's config and the ssh agent of the test's own.
fn env_in(home: &Path) -> Vec<(String, String)> {
    let own = ["HOME", "XDG_CONFIG_HOME", "SSH_AUTH_SOCK", "SSH_AGENT_PID", "GIT_CONFIG_GLOBAL", "GIT_CONFIG_NOSYSTEM", "GNUPGHOME"];
    let mut vars: Vec<(String, String)> = env().into_iter().filter(|(name, _)| !own.contains(&name.as_str())).collect();
    let at = |below: &str| home.join(below).display().to_string();
    vars.push(("HOME".to_string(), home.display().to_string()));
    vars.push(("XDG_CONFIG_HOME".to_string(), at(".config")));
    vars.push(("GIT_CONFIG_GLOBAL".to_string(), at(".gitconfig")));
    vars.push(("GIT_CONFIG_NOSYSTEM".to_string(), "1".to_string()));
    vars.push(("GNUPGHOME".to_string(), at(".gnupg")));
    vars
}

fn script(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\n{}", body)).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// The variables that make this test binary, run as a child, the runner of one command of `wb` (`Bench::child`).
const CHILD_COMMAND: &str = "WB_STAMPED_CHILD_COMMAND";
const CHILD_WORDS: &str = "WB_STAMPED_CHILD_WORDS";
const CHILD_ORG: &str = "WB_STAMPED_CHILD_ORG";
const CHILD_SAID: &str = "WB_STAMPED_CHILD_SAID";
const CHILD_TEST: &str = "a_helper_that_runs_wb_for_the_stamp_tests_and_checks_nothing";

/// The child: one command of `wb` in the folder it was started in, what it wrote in the file `CHILD_SAID` (standard
/// output, a line `--`, standard error), and its status as the status of the process. Run alone, it checks nothing.
#[test]
fn a_helper_that_runs_wb_for_the_stamp_tests_and_checks_nothing() {
    let (Ok(command), Ok(words), Ok(org), Ok(said)) = (std::env::var(CHILD_COMMAND), std::env::var(CHILD_WORDS), std::env::var(CHILD_ORG), std::env::var(CHILD_SAID)) else {
        return;
    };
    let words: Vec<String> = words.split('\u{1f}').map(str::to_string).collect();
    let folder = std::env::current_dir().unwrap();
    let org = PathBuf::from(org);
    let run = match command.as_str() {
        "start" => {
            let state = folder_named("state");
            let args: Vec<&str> = std::iter::once("start").chain(words.iter().map(String::as_str)).collect();
            start_in(&folder, &org, true, Some(&state.path), &args)
        }
        "prompt" => {
            let args: Vec<&str> = words.iter().map(String::as_str).collect();
            prompt_with(&folder, &org, &args)
        }
        _ => {
            use std::os::unix::process::ExitStatusExt;
            let words: Vec<OsString> = words.iter().map(OsString::from).collect();
            let mut input = std::io::Cursor::new(Vec::new());
            let (mut out, mut err) = (Vec::new(), Vec::new());
            let code = watchbill::plan(&words, &folder, &org, &mut input, &mut out, &mut err);
            Output { status: std::process::ExitStatus::from_raw(code << 8), stdout: out, stderr: err }
        }
    };
    fs::write(said, format!("{}--\n{}", text(&run.stdout), text(&run.stderr))).unwrap();
    std::process::exit(run.status.code().unwrap_or(-1));
}

/// What a child said: its status, its standard output and its standard error.
struct Said {
    code: i32,
    out: String,
    err: String,
}

impl Bench {
    fn run_in(&self, program: &str, args: &[&str], dir: &Path) -> Output {
        Command::new(program).args(args).current_dir(dir).env_clear().envs(env_in(&self.home.path)).stdin(Stdio::null()).output().expect("a program could not be started")
    }

    /// git in `dir`, which must end with 0; what it printed.
    fn git_in(&self, dir: &Path, args: &[&str]) -> String {
        let run = self.run_in("git", args, dir);
        assert!(run.status.success(), "git {:?}: {}", args, text(&run.stderr));
        text(&run.stdout)
    }

    fn git(&self, args: &[&str]) -> String {
        self.git_in(&self.repo.path, args)
    }

    /// `wb` as the program, in the repository.
    fn wb(&self, args: &[&str]) -> Output {
        self.run_in(env!("CARGO_BIN_EXE_wb"), args, &self.repo.path)
    }

    /// One command of `wb` in a child of this test binary, in `dir`, with the bench's home and organisation's file.
    fn child_in(&self, dir: &Path, command: &str, args: &[&str]) -> Said {
        let said = self.home.path.join(format!("said-{}", std::process::id()));
        let _ = fs::remove_file(&said);
        let run = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", CHILD_TEST, "--test-threads=1", "--quiet", "--nocapture"])
            .current_dir(dir)
            .env_clear()
            .envs(env_in(&self.home.path))
            .env(CHILD_COMMAND, command)
            .env(CHILD_WORDS, args.join("\u{1f}"))
            .env(CHILD_ORG, &self.org)
            .env(CHILD_SAID, &said)
            .stdin(Stdio::null())
            .output()
            .expect("the test binary could not be started");
        let written = fs::read_to_string(&said).unwrap_or_else(|_| format!("--\nthe child said nothing: {}{}", text(&run.stdout), text(&run.stderr)));
        let (out, err) = written.split_once("--\n").unwrap_or(("", &written));
        Said { code: run.status.code().unwrap_or(-1), out: out.to_string(), err: err.to_string() }
    }

    fn start(&self, args: &[&str]) -> Said {
        self.child_in(&self.repo.path, "start", args)
    }

    fn prompt(&self, args: &[&str]) -> Said {
        self.child_in(&self.repo.path, "prompt", args)
    }

    /// `wb plan add 54 -m <text>`, which must end with 0.
    fn plan(&self, text: &str) {
        let said = self.child_in(&self.repo.path, "plan", &["add", "54", "-m", text]);
        assert_eq!(said.code, 0, "{}", said.err);
    }

    /// What the stand-ins were run with, one line for each run, in order.
    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.log.path.join("calls")).unwrap_or_default().lines().map(str::to_string).collect()
    }

    fn sessions(&self) -> usize {
        self.calls().iter().filter(|call| *call == "start").count()
    }

    fn posted(&self) -> Option<String> {
        fs::read_to_string(self.log.path.join("posted-issue-54")).ok()
    }

    /// The comments on issue 54, as `issue_comments` prints them.
    fn comments(&self, lines: &[&str]) {
        fs::write(self.log.path.join("comments-issue-54"), lines.iter().map(|line| format!("{}\n", line)).collect::<String>()).unwrap();
    }

    /// The commits of the job's ref, newest first.
    fn on_ref(&self) -> Vec<String> {
        let run = self.run_in("git", &["rev-list", "--first-parent", JOB], &self.repo.path);
        text(&run.stdout).lines().map(str::to_string).collect()
    }

    /// The message of a commit.
    fn message(&self, commit: &str) -> String {
        self.git(&["cat-file", "commit", commit]).split_once("\n\n").map(|(_, message)| message.to_string()).unwrap_or_default()
    }

    fn head(&self) -> String {
        self.git(&["rev-parse", "HEAD"]).trim().to_string()
    }

    /// The private key of `name`, made the first time it is asked for.
    fn key(&self, name: &str) -> PathBuf {
        let folder = self.home.path.join("keys");
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join(name);
        if !path.exists() {
            let run = self.run_in("ssh-keygen", &["-q", "-t", "ed25519", "-N", "", "-C", name, "-f", path.to_str().unwrap()], &self.home.path);
            assert!(run.status.success(), "ssh-keygen: {}", text(&run.stderr));
        }
        path
    }

    /// The public key of `name`, as `<type> <base64>`.
    fn public(&self, name: &str) -> String {
        let key = self.key(name);
        let line = read(&PathBuf::from(format!("{}.pub", key.display())));
        line.split_whitespace().take(2).collect::<Vec<_>>().join(" ")
    }

    /// Signs with the key of `name` from now on, as git's config of the repository.
    fn sign_with(&self, name: &str) {
        let key = self.key(name);
        self.git(&["config", "gpg.format", "ssh"]);
        self.git(&["config", "user.signingkey", key.to_str().unwrap()]);
    }

    /// Commits `.watchbill/signers` on `main` with the line of `name` and `roles`, and comes back to `issue-54`.
    fn signers(&self, name: &str, roles: &str) {
        self.git(&["checkout", "-q", "main"]);
        put(&self.repo.path, SIGNERS, &format!("{}@example.org roles=\"{}\" {}\n", name, roles, self.public(name)));
        self.git(&["add", "--", SIGNERS]);
        self.git(&["commit", "-q", "-m", "The project's keys"]);
        self.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        self.git(&["checkout", "-q", "issue-54"]);
        self.git(&["merge", "-q", "--ff-only", "main"]);
    }

    /// An issue's file of 54, accepted and committed with its acceptance on `issue-54`.
    fn issue_file(&self) {
        put(&self.repo.path, YML, &BARE_ISSUE.replace("{issue}", "54").replace("  tester: []", "  tester: [src/brake.c]"));
        let run = self.wb(&["plan", "accept", "--no-commit", "54"]);
        assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
        self.git(&["add", "--", ".watchbill/issues"]);
        self.git(&["commit", "-q", "-m", "The issue's file"]);
    }

    /// `wb status 54`: what it printed.
    fn status(&self) -> String {
        let run = self.wb(&["status", "54"]);
        text(&run.stdout)
    }
}

/// A repository set up by `wb init --yes` (which writes `classed: []`), with the level `public`, the patterns of the
/// issue's text and its whole comments as stand-ins, and `tracker` the stamp's copy and the comments too; committed on
/// `main`, with `origin/HEAD` naming `main`, and the branch `issue-54` checked out. `origin` is a bare repository of the
/// test where `remote` is true, and there is none where it is false. Since #323 nothing that needs a signature counts
/// without `.watchbill/signers`: `main` holds Ada's line, with `accept` and `start`, and git signs with her key.
fn bench(tracker: bool, remote: bool) -> Bench {
    bench_listed(tracker, remote, true)
}

/// As `bench`, with no `.watchbill/signers` on `main` where `listed` is false: the list with Ada's line is then on the
/// branch `issue-54` alone, where it is not trusted, so that `wb check` of the checkout holds. Git signs with Ada's key.
fn bench_listed(tracker: bool, remote: bool, listed: bool) -> Bench {
    let repo = repository(None);
    let log = folder_named("log");
    let home = folder_named("home");
    fs::write(home.path.join(".gitconfig"), "").unwrap();
    let l = log.path.to_str().unwrap().to_string();
    let l = l.as_str();
    script(&log.path.join("post.sh"), &format!("echo \"post $1 $2\" >> {l}/calls\ncat > {l}/posted-$1-$2\nexit 0\n"));
    script(&log.path.join("comments.sh"), &format!("echo \"comments $1 $2\" >> {l}/calls\ncat {l}/comments-$1-$2 2>/dev/null\nexit 0\n"));
    script(&log.path.join("text.sh"), &format!("echo \"text $1\" >> {l}/calls\necho \"The text of issue $1.\"\nexit 0\n"));
    script(&log.path.join("bodies.sh"), &format!("echo \"bodies $1\" >> {l}/calls\nexit 0\n"));
    script(&log.path.join("start.sh"), &format!("echo start >> {l}/calls\nexit 0\n"));
    let org_folder = folder_named("org");
    let org = org_folder.path.join("organisation.yml");
    fs::write(&org, ORG.replace("{log}", l)).unwrap();
    let bench = Bench { repo, log, home, org, _org: org_folder, _origin: None };
    for (key, value) in [("user.name", "Ada Lovelace"), ("user.email", "ada@example.org"), ("commit.gpgsign", "false"), ("tag.gpgsign", "false")] {
        bench.git(&["config", key, value]);
    }
    let run = bench.wb(&["init", "--yes"]);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let mut yml = read(&config(&bench.repo.path));
    yml.push_str(&format!("issue_text: '{}/text.sh {{issue}}'\nissue_comment_bodies: '{}/bodies.sh {{issue}}'\nconfidentiality: public\n", l, l));
    if tracker {
        yml.push_str(&format!("issue_comment: '{}/post.sh issue {{issue}}'\nissue_comments: '{}/comments.sh issue {{issue}}'\n", l, l));
    }
    fs::write(config(&bench.repo.path), yml).unwrap();
    put(&bench.repo.path, "src/brake.c", "int brake(void) { return 1; }\n");
    if listed {
        put(&bench.repo.path, SIGNERS, &format!("ada@example.org roles=\"accept,start\" {}\n", bench.public("ada")));
    }
    bench.sign_with("ada");
    bench.git(&["add", "--", ".watchbill", "src/brake.c"]);
    bench.git(&["commit", "-q", "-m", "Watchbill"]);
    bench.git(&["branch", "-M", "main"]);
    bench.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    bench.git(&["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"]);
    bench.git(&["checkout", "-q", "-b", "issue-54"]);
    if !listed {
        put(&bench.repo.path, SIGNERS, &format!("ada@example.org roles=\"accept,start\" {}\n", bench.public("ada")));
        bench.git(&["add", "--", SIGNERS]);
        bench.git(&["commit", "-q", "-m", "The project's keys, on the branch alone"]);
    }
    if !remote {
        return bench;
    }
    let origin = folder_named("origin.git");
    bench.git_in(&origin.path, &["init", "-q", "--bare"]);
    bench.git(&["remote", "add", "origin", origin.path.to_str().unwrap()]);
    bench.git(&["push", "-q", "origin", "main", "issue-54"]);
    Bench { _origin: Some(origin), ..bench }
}

/// The hash of the text of a source, as `wb plan add` stores it.
fn blob(bench: &Bench, text: &str) -> String {
    let path = bench.home.path.join("source");
    fs::write(&path, text).unwrap();
    bench.git(&["hash-object", "-t", "blob", path.to_str().unwrap()]).trim().to_string()
}

fn ok(said: &Said) {
    assert_eq!(said.code, 0, "{}", said.err);
}

// ---- WB-BUILD-43: the stamp is a commit on the job's ref, after the one before ----

#[test]
fn test_WB_BUILD_43_a_start_writes_one_commit_on_the_jobs_ref_with_the_stamp_job_and_base_after_the_plan() {
    let bench = bench(false, true);
    bench.plan("The brakes hold.");
    let plan = bench.on_ref()[0].clone();
    let base = bench.head();

    ok(&bench.start(&["tester", "54"]));

    let commits = bench.on_ref();
    assert_eq!(commits.len(), 2, "{:?}", commits);
    assert_eq!(bench.git(&["rev-parse", &format!("{}^", commits[0])]).trim(), plan);
    let message = bench.message(&commits[0]);
    assert!(message.starts_with("[stamp] tester round 1\nrole: tester\n"), "{}", message);
    assert!(message.contains(&format!("\njob: {}\n", blob(&bench, "The brakes hold.\n"))), "{}", message);
    assert!(message.contains(&format!("\nbase: {}\n", base)), "{}", message);
    assert_eq!(bench.sessions(), 1);
}

#[test]
fn test_WB_BUILD_43_a_second_start_names_the_first_stamp_before_it() {
    let bench = bench(false, false);
    ok(&bench.start(&["tester", "54"]));
    let first = bench.on_ref()[0].clone();
    ok(&bench.start(&["tester", "54"]));
    let commits = bench.on_ref();
    assert_eq!(commits.len(), 2, "{:?}", commits);
    assert_eq!(bench.git(&["rev-parse", &format!("{}^", commits[0])]).trim(), first);
    assert!(bench.message(&commits[0]).contains("\njob: none\n"), "{}", bench.message(&commits[0]));
}

// ---- WB-BUILD-44: the tree of a stamp holds the job's tree, the view's list and the issue's file ----

#[test]
fn test_WB_BUILD_44_the_stamps_tree_holds_the_jobs_tree_the_views_list_and_the_issues_file() {
    let bench = bench(false, false);
    bench.plan("The brakes hold.");
    // A source of the acceptance's own second counts as after it (#290): the acceptance comes in the next second.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    bench.issue_file();

    ok(&bench.start(&["tester", "54"]));

    let tree = bench.git(&["ls-tree", "-r", "--name-only", JOB]);
    let names: Vec<&str> = tree.lines().collect();
    let source = format!("sources/{}", blob(&bench, "The brakes hold.\n"));
    assert_eq!(names, ["issue.yml", "job.yml", source.as_str(), "view"], "{}", tree);
    assert_eq!(bench.git(&["cat-file", "blob", &format!("{}:issue.yml", JOB)]), read(&bench.repo.path.join(YML)));
    assert!(bench.git(&["cat-file", "blob", &format!("{}:view", JOB)]).contains("src/brake.c"), "{}", bench.git(&["cat-file", "blob", &format!("{}:view", JOB)]));
}

// ---- WB-BUILD-45: the push is the lock ----

#[test]
fn test_WB_BUILD_45_a_second_start_from_a_clone_that_did_not_fetch_the_first_is_refused_with_1_and_starts_nothing() {
    let bench = bench(false, true);
    let origin = bench._origin.as_ref().unwrap().path.clone();
    let other = folder_named("other");
    let clone = other.path.join("brake-controller");
    bench.git_in(&other.path, &["clone", "-q", "-b", "issue-54", origin.to_str().unwrap(), clone.to_str().unwrap()]);
    // The clone signs its stamp with Ada's key, which the list holds with `start` (#323).
    let key = bench.key("ada");
    for (key, value) in [("user.name", "Grace Hopper"), ("user.email", "grace@example.org"), ("commit.gpgsign", "false"), ("gpg.format", "ssh"), ("user.signingkey", key.to_str().unwrap())] {
        bench.git_in(&clone, &["config", key, value]);
    }

    let first = bench.child_in(&clone, "start", &["tester", "54"]);
    ok(&first);
    assert_eq!(bench.sessions(), 1);
    let pushed = bench.git_in(&origin, &["rev-parse", JOB]).trim().to_string();

    let second = bench.start(&["tester", "54"]);

    assert_eq!(second.code, 1, "{}", second.err);
    assert!(second.err.contains("another start went first"), "{}", second.err);
    assert!(second.err.contains("Nothing is started"), "{}", second.err);
    assert_eq!(bench.sessions(), 1, "{:?}", bench.calls());
    // Neither the remote's ref nor the local one moved.
    assert_eq!(bench.git_in(&origin, &["rev-parse", JOB]).trim(), pushed);
    assert!(bench.on_ref().is_empty(), "{:?}", bench.on_ref());
}

#[test]
fn test_WB_BUILD_45_a_start_pushes_its_stamp_to_origin_before_the_session() {
    let bench = bench(false, true);
    let origin = bench._origin.as_ref().unwrap().path.clone();
    let said = bench.start(&["tester", "54"]);
    ok(&said);
    assert_eq!(bench.git_in(&origin, &["rev-parse", JOB]).trim(), bench.on_ref()[0]);
    let pushed = said.err.find("pushed it to origin").unwrap_or_else(|| panic!("{}", said.err));
    let started = said.err.find("starts tester on the runner").unwrap_or_else(|| panic!("{}", said.err));
    assert!(pushed < started, "{}", said.err);
}

// ---- WB-BUILD-46: with no remote the stamp is local ----

#[test]
fn test_WB_BUILD_46_with_no_remote_the_stamp_is_local_the_start_goes_on_and_says_so() {
    let bench = bench(false, false);
    let said = bench.start(&["tester", "54"]);
    ok(&said);
    assert_eq!(bench.on_ref().len(), 1);
    assert!(said.err.contains("there is no remote origin, so the stamp"), "{}", said.err);
    assert!(said.err.contains(&format!("is on the local {} alone", JOB)), "{}", said.err);
    assert_eq!(bench.sessions(), 1);
}

// ---- WB-BUILD-47: the round is counted from the stamp commits ----

#[test]
fn test_WB_BUILD_47_two_stamps_of_the_tester_give_round_2_with_no_tracker_set_up() {
    let bench = bench(false, false);
    ok(&bench.start(&["tester", "54"]));
    let second = bench.start(&["tester", "54"]);
    ok(&second);
    assert!(bench.message(&bench.on_ref()[0]).starts_with("[stamp] tester round 2\n"), "{}", bench.message(&bench.on_ref()[0]));
    assert!(second.err.contains(&format!("round 2 (1 tester stamp on {})", JOB)), "{}", second.err);
    assert!(second.err.contains("so no copy of the stamp is posted"), "{}", second.err);
    assert!(!bench.calls().iter().any(|call| call.starts_with("post ") || call.starts_with("comments ")), "{:?}", bench.calls());
}

// ---- WB-BUILD-48: a stamp that does not check is not a stamp ----

#[test]
fn test_WB_BUILD_48_an_unsigned_stamp_commit_is_not_counted_and_status_says_so() {
    let bench = bench(false, false);
    bench.signers("ada", "start");
    bench.sign_with("ada");
    unsigned_stamps_on_ref(&bench.repo.path, "54", &["[stamp] tester round 1\nrole: tester\n", "[stamp] review round 1\nrole: review\n"]);
    let before = bench.git(&["for-each-ref"]);

    let said = bench.start(&["tester", "54"]);

    // Since #327 an unsigned commit makes the job's ref not as `wb` writes it: the start is refused, and writes nothing.
    assert_eq!(said.code, 1, "{}", said.err);
    assert!(said.err.contains("is not as wb writes it"), "{}", said.err);
    assert_eq!(bench.git(&["for-each-ref"]), before);
    assert_eq!(bench.sessions(), 0, "{:?}", bench.calls());
    let status = bench.status();
    assert!(!status.contains("Tested:") && !status.contains("Reviewed:"), "{}", status);
    assert!(status.contains("[stamp] tester round 1  (on refs/watchbill/54, not counted: unsigned"), "{}", status);
}

#[test]
fn test_WB_BUILD_48_a_stamp_signed_by_a_key_not_in_the_list_with_start_is_not_counted() {
    let bench = bench(false, false);
    bench.signers("ada", "start");
    // Bob's key is in no list; Ada signs the next start.
    bench.sign_with("bob");
    let tree = bench.git(&["write-tree"]).trim().to_string();
    let forged = bench.git(&["commit-tree", "-S", "-m", "[stamp] tester round 1", &tree]).trim().to_string();
    bench.git(&["update-ref", JOB, &forged]);
    bench.sign_with("ada");
    let before = bench.git(&["for-each-ref"]);

    let said = bench.start(&["tester", "54"]);

    // Since #327 a commit signed by a key not in the list makes the job's ref not as `wb` writes it: the start is
    // refused, and writes nothing.
    assert_eq!(said.code, 1, "{}", said.err);
    assert!(said.err.contains("is not as wb writes it") && said.err.contains("signed by a key that is not in"), "{}", said.err);
    assert_eq!(bench.git(&["for-each-ref"]), before);
    assert_eq!(bench.sessions(), 0, "{:?}", bench.calls());
    assert!(!bench.status().contains("Tested:"), "{}", bench.status());
    assert!(bench.status().contains("not counted: signed by a key that is not in .watchbill/signers of origin/main"), "{}", bench.status());
}

// ---- WB-BUILD-49: the comment is a copy, never read back ----

#[test]
fn test_WB_BUILD_49_the_stamp_comment_is_posted_as_a_copy_of_the_commit() {
    let bench = bench(true, false);
    ok(&bench.start(&["tester", "54"]));
    let posted = bench.posted().expect("no copy was posted");
    assert_eq!(posted, bench.message(&bench.on_ref()[0]));
}

#[test]
fn test_WB_BUILD_49_a_forged_stamp_comment_changes_no_round() {
    let bench = bench(true, false);
    bench.comments(&["[stamp] tester round 1", "[stamp] tester round 2", "[stamp] review round 1"]);

    let said = bench.start(&["tester", "54"]);

    ok(&said);
    assert!(bench.message(&bench.on_ref()[0]).starts_with("[stamp] tester round 1\n"), "{}", bench.message(&bench.on_ref()[0]));
    assert!(!bench.calls().iter().any(|call| call.starts_with("comments ")), "{:?}", bench.calls());
    assert!(!bench.status().contains("Reviewed:"), "{}", bench.status());
}

// ---- WB-BUILD-50: the stamp is signed by the starter ----

#[test]
fn test_WB_BUILD_50_with_the_list_the_stamp_is_signed_by_the_starters_key_and_counts() {
    let bench = bench(false, false);
    bench.signers("ada", "start");
    bench.sign_with("ada");

    ok(&bench.start(&["tester", "54"]));

    let commit = bench.on_ref()[0].clone();
    let allowed = bench.home.path.join("allowed");
    fs::write(&allowed, format!("ada@example.org {}\n", bench.public("ada"))).unwrap();
    let setting = format!("gpg.ssh.allowedSignersFile={}", allowed.display());
    let verified = bench.run_in("git", &["-c", &setting, "verify-commit", &commit], &bench.repo.path);
    assert!(verified.status.success(), "{}", text(&verified.stderr));
    assert!(bench.status().contains("[stamp] tester round 1  (on refs/watchbill/54, signed by ada@example.org)"), "{}", bench.status());
    let second = bench.start(&["tester", "54"]);
    ok(&second);
    assert!(bench.message(&bench.on_ref()[0]).starts_with("[stamp] tester round 2\n"));
}

// ---- WB-TASK-56, WB-TASK-58 and WB-TASK-59: with no list, no stamp is written and none counts (#323) ----

#[test]
fn test_WB_TASK_56_with_no_list_the_start_is_refused_with_1_writes_nothing_and_names_the_file_and_how_to_add_a_key() {
    // Git signs with Ada's key: only the list on the default branch is missing. The branch's own is not trusted.
    let bench = bench_listed(true, false, false);
    let before = bench.git(&["for-each-ref"]);

    let said = bench.start(&["tester", "54"]);

    assert_eq!(said.code, 1, "{}", said.err);
    assert!(said.err.contains("no .watchbill/signers on origin/main"), "{}", said.err);
    assert!(said.err.contains("roles=\"start\""), "{}", said.err);
    assert_eq!(bench.git(&["for-each-ref"]), before);
    assert!(bench.on_ref().is_empty(), "{:?}", bench.on_ref());
    assert!(bench.calls().iter().all(|call| !call.starts_with("post ") && call != "start"), "{:?}", bench.calls());

    // With the list in no tree at all, the check of `wb check` refuses it first, naming the list.
    bench.git(&["rm", "-q", "--", SIGNERS]);
    bench.git(&["commit", "-q", "-m", "No keys"]);
    let before = bench.git(&["for-each-ref"]);
    let said = bench.start(&["tester", "54"]);
    assert_eq!(said.code, 1, "{}", said.err);
    assert!(said.err.contains("missing: .watchbill/signers"), "{}", said.err);
    assert_eq!(bench.git(&["for-each-ref"]), before);
    assert!(bench.calls().iter().all(|call| !call.starts_with("post ") && call != "start"), "{:?}", bench.calls());
}

#[test]
fn test_WB_TASK_56_with_no_list_revert_of_an_issue_is_refused_with_1_and_writes_no_marker() {
    let bench = bench_listed(true, false, false);
    unsigned_stamps_on_ref(&bench.repo.path, "54", &["[stamp] review round 1\nrole: review\n"]);
    let before = bench.git(&["for-each-ref"]);

    let run = bench.wb(&["revert", "54", "review"]);

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("no .watchbill/signers on origin/main"), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("roles=\"start\""), "{}", text(&run.stderr));
    assert_eq!(bench.git(&["for-each-ref"]), before);
    assert!(bench.posted().is_none());
}

#[test]
fn test_WB_TASK_58_with_no_list_a_stamp_commit_signed_or_not_counts_for_nothing() {
    let bench = bench_listed(false, false, false);
    // One stamp signed by Ada's key, which no list holds, and one unsigned after it.
    let tree = bench.git(&["write-tree"]).trim().to_string();
    let signed = bench.git(&["commit-tree", "-S", "-m", "[stamp] tester round 1\nissue: 54\nrole: tester", &tree]).trim().to_string();
    bench.git(&["update-ref", JOB, &signed]);
    unsigned_stamps_on_ref(&bench.repo.path, "54", &["[stamp] review round 1\nrole: review\n"]);

    let status = bench.status();
    assert!(!status.contains("Tested:") && !status.contains("Reviewed:"), "{}", status);
    assert!(status.contains("[stamp] tester round 1  (on refs/watchbill/54, not counted"), "{}", status);
    assert!(status.contains("[stamp] review round 1  (on refs/watchbill/54, not counted"), "{}", status);
    assert_eq!(status.matches("no .watchbill/signers on origin/main: nothing is signed, so nothing counts").count(), 1, "{}", status);

    // No round and no next role comes from them: a fix finds no review stamp.
    let mut args = vec!["fix", "54"];
    args.extend(REST);
    let said = bench.prompt(&args);
    assert_eq!(said.code, 1, "{}", said.err);
    assert!(said.err.contains("no review stamp on refs/watchbill/54"), "{}", said.err);
}

#[test]
fn test_WB_TASK_58_with_no_list_a_marker_takes_nothing_back() {
    let bench = bench(false, false);
    stamps_on_ref(&bench.repo.path, "54", &["[stamp] review round 1\nrole: review\n"]);
    let run = bench.wb(&["revert", "54", "review"]);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    // With the list gone from `main`, the stamp and the marker both count for nothing.
    bench.git(&["checkout", "-q", "main"]);
    bench.git(&["rm", "-q", "--", SIGNERS]);
    bench.git(&["commit", "-q", "-m", "No keys"]);
    bench.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    bench.git(&["checkout", "-q", "issue-54"]);
    let status = bench.status();
    assert!(status.contains("[revert] review round 1  (on refs/watchbill/54, not counted"), "{}", status);
    assert!(status.contains("[stamp] review round 1  (on refs/watchbill/54, not counted"), "{}", status);
}

// ---- WB-BUILD-52 and WB-BUILD-53: refused before anything is written ----

#[test]
fn test_WB_BUILD_52_with_the_list_and_no_signing_key_the_start_is_refused_with_1_naming_the_setting() {
    let bench = bench(true, false);
    bench.signers("ada", "start");
    bench.git(&["config", "--unset", "user.signingkey"]);
    let before = bench.git(&["for-each-ref"]);

    let said = bench.start(&["tester", "54"]);

    assert_eq!(said.code, 1, "{}", said.err);
    assert!(said.err.contains("user.signingkey"), "{}", said.err);
    assert_eq!(bench.git(&["for-each-ref"]), before);
    assert!(bench.calls().iter().all(|call| !call.starts_with("post ") && call != "start"), "{:?}", bench.calls());
}

#[test]
fn test_WB_BUILD_53_with_the_list_and_a_key_not_in_it_with_start_the_start_is_refused_with_1_naming_the_key() {
    let bench = bench(true, false);
    bench.signers("ada", "accept");
    let before = bench.git(&["for-each-ref"]);

    bench.sign_with("ada");
    let said = bench.start(&["tester", "54"]);
    assert_eq!(said.code, 1, "{}", said.err);
    assert!(said.err.contains(&bench.public("ada")) && said.err.contains("without the role start"), "{}", said.err);

    bench.sign_with("bob");
    let said = bench.start(&["tester", "54"]);
    assert_eq!(said.code, 1, "{}", said.err);
    assert!(said.err.contains(&bench.public("bob")) && said.err.contains("is not in .watchbill/signers"), "{}", said.err);

    assert_eq!(bench.git(&["for-each-ref"]), before);
    assert!(bench.calls().iter().all(|call| !call.starts_with("post ") && call != "start"), "{:?}", bench.calls());
}

// ---- WB-PROMPT-58 and WB-PROMPT-59: `job:` and `base:` ----

#[test]
fn test_WB_PROMPT_58_stamp_prints_the_job_none_without_a_plan_and_its_hash_with_one() {
    let bench = bench(false, false);
    let mut args = vec!["tester", "54", "--stamp", "--round", "1"];
    args.extend(REST);
    let said = bench.prompt(&args);
    ok(&said);
    assert!(said.out.contains("\njob: none\n"), "{}", said.out);

    bench.plan("The brakes hold.");
    let said = bench.prompt(&args);
    ok(&said);
    assert!(said.out.contains(&format!("\njob: {}\n", blob(&bench, "The brakes hold.\n"))), "{}", said.out);
}

#[test]
fn test_WB_PROMPT_59_stamp_prints_the_base_the_commit_of_head() {
    let bench = bench(false, false);
    let mut args = vec!["tester", "54", "--stamp", "--round", "1"];
    args.extend(REST);
    let said = bench.prompt(&args);
    ok(&said);
    assert!(said.out.contains(&format!("\nbase: {}\n", bench.head())), "{}", said.out);
    let job = said.out.find("\njob: ").unwrap();
    let base = said.out.find("\nbase: ").unwrap();
    let part = said.out.find("\npart: ").unwrap_or(said.out.len());
    assert!(job < base && base < part, "{}", said.out);
}

// ---- WB-JOB-42: a planner round after the acceptance, read from the job's ref ----

#[test]
fn test_WB_JOB_42_a_planner_round_after_the_acceptance_voids_it_for_prompt_of_an_issue_with_no_plan() {
    let bench = bench(false, false);
    bench.issue_file();
    stamps_on_ref(&bench.repo.path, "54", &["[stamp] planner round 2\nrole: planner\ntime: 2999-01-01T00:00:00Z\n"]);

    let mut args = vec!["build", "54", "--round", "1"];
    args.extend(REST);
    let said = bench.prompt(&args);

    assert_eq!(said.code, 1, "{}", said.err);
    assert!(said.out.is_empty(), "{}", said.out);
    assert!(said.err.contains("wb plan accept 54"), "{}", said.err);
}

// ---- WB-BUILD-48: a stamp of another job is not a stamp of this one (the review of PR #322) ----

#[test]
fn test_WB_BUILD_48_a_signed_stamp_of_another_issue_under_an_unsigned_child_is_not_counted() {
    let bench = bench(false, false);
    bench.signers("ada", "start");
    bench.sign_with("ada");
    // Ada's own review stamp of issue 99, signed, on its job's ref.
    let tree = bench.git(&["write-tree"]).trim().to_string();
    let stamp = bench.git(&["commit-tree", "-S", "-m", "[stamp] review round 1\nrole: review\nissue: 99", &tree]).trim().to_string();
    bench.git(&["update-ref", "refs/watchbill/99", &stamp]);
    // Someone with no key points 54 at an unsigned child of it.
    let child = bench.git(&["commit-tree", "--no-gpg-sign", "-p", &stamp, "-m", "a commit", &tree]).trim().to_string();
    bench.git(&["update-ref", JOB, &child]);

    let status = bench.status();
    assert!(!status.contains("Reviewed:"), "{}", status);
    assert!(status.contains("[stamp] review round 1  (on refs/watchbill/54, not counted: a stamp of another job"), "{}", status);

    let mut args = vec!["fix", "54"];
    args.extend(REST);
    let said = bench.prompt(&args);
    assert_eq!(said.code, 1, "{}", said.err);
    assert!(said.err.contains("no review stamp on refs/watchbill/54"), "{}", said.err);
}

#[test]
fn test_WB_BUILD_48_the_marker_of_wb_revert_names_its_issue_and_job() {
    let bench = bench(false, false);
    stamps_on_ref(&bench.repo.path, "54", &["[stamp] review round 1\nrole: review\nissue: 54\n"]);

    let run = bench.wb(&["revert", "54", "review"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let marker = bench.message(&bench.on_ref()[0]);
    assert!(marker.starts_with("[revert] review round 1\n"), "{}", marker);
    assert!(marker.contains("\nissue: 54\n") && marker.contains("\njob: none\n"), "{}", marker);
}

// ---- WB-BUILD-46: a remote that is not `origin` (the review of PR #322) ----

#[test]
fn test_WB_BUILD_46_with_remotes_and_none_named_origin_the_start_is_refused_with_1_naming_origin() {
    let bench = bench(false, false);
    let other = folder_named("upstream.git");
    bench.git_in(&other.path, &["init", "-q", "--bare"]);
    bench.git(&["remote", "add", "upstream", other.path.to_str().unwrap()]);
    let before = bench.git(&["for-each-ref"]);

    let said = bench.start(&["tester", "54"]);

    assert_eq!(said.code, 1, "{}", said.err);
    assert!(said.err.contains("origin") && said.err.contains("upstream"), "{}", said.err);
    assert_eq!(bench.git(&["for-each-ref"]), before);
    assert_eq!(bench.sessions(), 0, "{:?}", bench.calls());
}

// ---- the loose ends of #236 (#324) ----

/// A commit of `message` as it is, with the tree `tree` and the parents `parents`, signed by Ada's key where `signed`
/// is true. No ref is moved.
fn raw_commit(bench: &Bench, tree: &str, parents: &[&str], message: &str, signed: bool) -> String {
    use std::io::Write;
    let mut args = vec!["commit-tree", if signed { "-S" } else { "--no-gpg-sign" }, tree];
    for parent in parents {
        args.extend(["-p", parent]);
    }
    let mut child = Command::new("git")
        .args(&args)
        .current_dir(&bench.repo.path)
        .env_clear()
        .envs(env_in(&bench.home.path))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(message.as_bytes()).unwrap();
    let run = child.wait_with_output().unwrap();
    assert!(run.status.success(), "git commit-tree: {}", text(&run.stderr));
    text(&run.stdout).trim().to_string()
}

/// The tip of the job's ref of 54, or empty.
fn tip_of(bench: &Bench) -> String {
    text(&bench.run_in("git", &["rev-parse", "--verify", "-q", JOB], &bench.repo.path).stdout).trim().to_string()
}

/// A signed commit on the job's ref of 54 with `message` as it is, after the tip of the ref, with the tree of the tip.
/// The ref is moved to it. Ada's key signs it, and `.watchbill/signers` holds her line with `start`.
fn raw_stamp(bench: &Bench, message: &str) -> String {
    let tip = tip_of(bench);
    let tree = if tip.is_empty() { bench.git(&["mktree"]).trim().to_string() } else { bench.git(&["rev-parse", &format!("{}^{{tree}}", tip)]).trim().to_string() };
    let parents: Vec<&str> = if tip.is_empty() { vec![] } else { vec![tip.as_str()] };
    let commit = raw_commit(bench, &tree, &parents, message, true);
    bench.git(&["update-ref", JOB, &commit]);
    commit
}

/// A bench with Ada's line, with `accept` and `start`, on `main`, and git signing with her key: since #327 a commit of
/// `wb plan` is signed by a key with `accept`, so her line keeps `accept` as `bench` writes it.
fn listed() -> Bench {
    bench(false, false)
}

/// The line of `wb status 54` that holds `first`.
fn status_of(bench: &Bench, first: &str) -> String {
    bench.status().lines().find(|line| line.contains(first)).unwrap_or_default().to_string()
}

#[test]
fn test_WB_BUILD_161_a_stamp_with_a_space_after_its_issue_is_not_counted_and_the_text_is_not_a_riddle() {
    let bench = listed();
    let base = bench.head();
    raw_stamp(&bench, &format!("[stamp] tester round 1\nrole: tester\nissue: 54 \njob: none\nbase: {}\n", base));

    let line = status_of(&bench, "[stamp] tester round 1");

    assert!(line.contains("not counted:"), "{}", bench.status());
    assert!(!line.contains("names issue 54, not 54"), "{}", line);
    assert!(line.contains("issue: 54"), "{}", line);
    ok(&bench.start(&["tester", "54"]));
    assert!(bench.message(&bench.on_ref()[0]).starts_with("[stamp] tester round 1\n"), "{}", bench.message(&bench.on_ref()[0]));
}

#[test]
fn test_WB_BUILD_161_a_stamp_with_two_issue_lines_is_not_counted() {
    let bench = listed();
    let base = bench.head();
    raw_stamp(&bench, &format!("[stamp] tester round 1\nrole: tester\nissue: 99\nissue: 54\njob: none\nbase: {}\n", base));
    raw_stamp(&bench, &format!("[stamp] review round 1\nrole: review\nissue: 54\nissue: 54\njob: none\nbase: {}\n", base));

    let status = bench.status();

    assert!(status_of(&bench, "[stamp] tester round 1").contains("not counted:"), "{}", status);
    assert!(status_of(&bench, "[stamp] review round 1").contains("not counted:"), "{}", status);
    assert!(!status.contains("Reviewed:"), "{}", status);
}

#[test]
fn test_WB_BUILD_162_a_stamp_commit_over_64_kib_is_not_counted_and_status_names_it() {
    let bench = listed();
    let base = bench.head();
    let padding = "x".repeat(70 * 1024);
    let commit = raw_stamp(&bench, &format!("[stamp] tester round 1\nrole: tester\nissue: 54\njob: none\nbase: {}\n\n{}\n", base, padding));

    let line = status_of(&bench, &commit);

    assert!(line.contains("(on refs/watchbill/54, not counted: larger than 64 KiB ("), "{}", bench.status());
    assert!(line.contains("so it is not read)"), "{}", line);
}

#[test]
fn test_WB_BUILD_163_a_stamp_whose_base_is_no_commit_of_this_repository_is_not_counted_as_one_of_another() {
    let bench = listed();
    let blob = blob(&bench, "not a commit\n");
    // A commit this repository does not have, a blob it has, a base that is no hash, and none.
    let bases = ["base: 1234567890123456789012345678901234567890\n".to_string(), format!("base: {}\n", blob), "base: not-a-hash\n".to_string(), String::new()];
    for (round, base) in bases.iter().enumerate() {
        raw_stamp(&bench, &format!("[stamp] tester round {}\nrole: tester\nissue: 54\njob: none\n{}", round + 1, base));
    }

    let status = bench.status();

    for round in 1..=4 {
        let line = status_of(&bench, &format!("[stamp] tester round {}", round));
        assert!(line.contains("not counted: a stamp of another repository"), "{}", status);
        assert!(line.contains("base"), "{}", line);
    }
    ok(&bench.start(&["tester", "54"]));
    assert!(bench.message(&bench.on_ref()[0]).starts_with("[stamp] tester round 1\n"), "{}", bench.message(&bench.on_ref()[0]));
}

#[test]
fn test_WB_BUILD_163_a_stamp_with_two_base_lines_is_not_counted_and_one_with_this_repositorys_commit_is() {
    let bench = listed();
    let base = bench.head();
    raw_stamp(&bench, &format!("[stamp] tester round 1\nrole: tester\nissue: 54\njob: none\nbase: {}\nbase: {}\n", base, base));
    raw_stamp(&bench, &format!("[stamp] review round 1\nrole: review\nissue: 54\njob: none\nbase: {}\n", base));

    assert!(status_of(&bench, "[stamp] tester round 1").contains("not counted: a stamp of another repository"), "{}", bench.status());
    assert!(status_of(&bench, "[stamp] review round 1").contains("signed by"), "{}", bench.status());
}

#[test]
fn test_WB_BUILD_163_the_marker_of_wb_revert_holds_the_base_and_counts() {
    let bench = listed();
    ok(&bench.start(&["tester", "54"]));
    let base = bench.head();

    let run = bench.wb(&["revert", "54", "tester"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let marker = bench.message(&bench.on_ref()[0]);
    assert!(marker.starts_with("[revert] tester round 1\n") && marker.contains(&format!("\nbase: {}\n", base)), "{}", marker);
    assert!(bench.status().contains("taken back by a [revert] marker"), "{}", bench.status());
}

#[test]
fn test_WB_BUILD_164_a_stamp_of_a_job_this_repository_has_no_plan_for_is_not_counted_as_one_of_another() {
    let bench = listed();
    let base = bench.head();
    raw_stamp(&bench, &format!("[stamp] tester round 1\nrole: tester\nissue: 54\njob: {}\nbase: {}\n", blob(&bench, "A plan of another repository.\n"), base));

    let line = status_of(&bench, "[stamp] tester round 1");

    assert!(line.contains("not counted: a stamp of another repository"), "{}", bench.status());
    assert!(line.contains("job"), "{}", line);
}

#[test]
fn test_WB_BUILD_164_a_stamp_with_job_none_where_this_repository_has_a_plan_is_not_counted() {
    let bench = listed();
    bench.plan("The brakes hold.");
    let base = bench.head();
    raw_stamp(&bench, &format!("[stamp] tester round 1\nrole: tester\nissue: 54\njob: none\nbase: {}\n", base));
    raw_stamp(&bench, &format!("[stamp] review round 1\nrole: review\nissue: 54\njob: {}\nbase: {}\n", blob(&bench, "The brakes hold.\n"), base));

    let status = bench.status();

    assert!(status_of(&bench, "[stamp] tester round 1").contains("not counted: a stamp of another repository"), "{}", status);
    assert!(status_of(&bench, "[stamp] review round 1").contains("signed by"), "{}", status);
}

#[test]
fn test_WB_BUILD_164_a_stamp_stays_counted_after_a_source_is_added_to_the_plan_later() {
    let bench = listed();
    bench.plan("The brakes hold.");
    ok(&bench.start(&["tester", "54"]));
    bench.plan("And they hold in the wet.");

    let line = status_of(&bench, "[stamp] tester round 1");

    assert!(line.contains("signed by"), "{}", bench.status());
}

#[test]
fn test_WB_BUILD_164_the_stamp_of_a_start_after_a_plan_names_its_job_and_counts() {
    let bench = listed();
    bench.plan("The brakes hold.");
    ok(&bench.start(&["tester", "54"]));
    ok(&bench.start(&["tester", "54"]));
    assert!(bench.message(&bench.on_ref()[0]).starts_with("[stamp] tester round 2\n"), "{}", bench.message(&bench.on_ref()[0]));
}

/// A signed stamp of 54 with `job: none` on a chain of its own, which the repository's own ref does not hold: what a
/// stamp of another repository looks like. Its commit.
fn foreign_stamp(bench: &Bench) -> String {
    let empty = bench.git(&["mktree"]).trim().to_string();
    let root = raw_commit(bench, &empty, &[], "A commit of another repository\n", false);
    let message = format!("[stamp] review round 1\nrole: review\nissue: 54\njob: none\nbase: {}\n", bench.head());
    raw_commit(bench, &empty, &[root.as_str()], &message, true)
}

#[test]
fn test_WB_BUILD_164_a_signed_job_none_stamp_spliced_in_by_an_unsigned_merge_is_not_counted_where_the_tip_holds_a_plan() {
    let bench = listed();
    bench.plan("The brakes hold.");
    ok(&bench.start(&["tester", "54"]));
    let old = tip_of(&bench);
    let tree = bench.git(&["rev-parse", &format!("{}^{{tree}}", old)]).trim().to_string();
    let foreign = foreign_stamp(&bench);
    let merge = raw_commit(&bench, &tree, &[foreign.as_str(), old.as_str()], "Merge\n", false);
    bench.git(&["update-ref", JOB, &merge]);

    let status = bench.status();

    assert!(!status.contains("Reviewed:"), "{}", status);
    assert!(status_of(&bench, "[stamp] review round 1").contains("not counted: a stamp of another repository"), "{}", status);
}

#[test]
fn test_WB_BUILD_164_the_same_stamp_under_one_unsigned_commit_with_the_plan_on_top_is_not_counted() {
    let bench = listed();
    bench.plan("The brakes hold.");
    ok(&bench.start(&["tester", "54"]));
    let tree = bench.git(&["rev-parse", &format!("{}^{{tree}}", tip_of(&bench))]).trim().to_string();
    let foreign = foreign_stamp(&bench);
    let top = raw_commit(&bench, &tree, &[foreign.as_str()], "A commit with no stamp\n", false);
    bench.git(&["update-ref", JOB, &top]);

    let status = bench.status();

    assert!(!status.contains("Reviewed:"), "{}", status);
    assert!(status_of(&bench, "[stamp] review round 1").contains("not counted: a stamp of another repository"), "{}", status);
}

#[test]
fn test_WB_BUILD_164_the_jobs_of_a_stamp_are_a_prefix_of_the_sources_at_the_tip_and_none_only_where_there_is_no_plan() {
    let bench = listed();
    bench.plan("The brakes hold.");
    bench.plan("And they hold in the wet.");
    let (first, second) = (blob(&bench, "The brakes hold.\n"), blob(&bench, "And they hold in the wet.\n"));
    let base = bench.head();
    let stamp = |round: u32, jobs: &str| raw_stamp(&bench, &format!("[stamp] tester round {}\nrole: tester\nissue: 54\n{}base: {}\n", round, jobs, base));
    stamp(1, &format!("job: {}\n", first));
    stamp(2, &format!("job: {}\njob: {}\n", first, second));
    stamp(3, &format!("job: {}\n", second));
    stamp(4, &format!("job: {}\njob: {}\n", second, first));
    stamp(5, &format!("job: {}\njob: {}\njob: {}\n", first, second, first));
    stamp(6, "job: none\n");

    let status = bench.status();

    for (round, counts) in [(1, true), (2, true), (3, false), (4, false), (5, false), (6, false)] {
        let line = status_of(&bench, &format!("[stamp] tester round {} ", round));
        assert_eq!(line.contains("signed by"), counts, "round {}: {}", round, status);
    }
}

#[test]
fn test_WB_BUILD_164_where_the_plan_is_removed_only_job_none_counts() {
    let bench = listed();
    bench.plan("The brakes hold.");
    let base = bench.head();
    let said = bench.child_in(&bench.repo.path, "plan", &["remove", "54"]);
    assert_eq!(said.code, 0, "{}", said.err);
    raw_stamp(&bench, &format!("[stamp] tester round 1\nrole: tester\nissue: 54\njob: {}\nbase: {}\n", blob(&bench, "The brakes hold.\n"), base));
    raw_stamp(&bench, &format!("[stamp] review round 1\nrole: review\nissue: 54\njob: none\nbase: {}\n", base));

    let status = bench.status();

    assert!(status_of(&bench, "[stamp] tester round 1").contains("not counted: a stamp of another repository"), "{}", status);
    assert!(status_of(&bench, "[stamp] review round 1").contains("signed by"), "{}", status);
}

// ---- WB-BUILD-163: no commit at HEAD, no stamp (the review of #326) ----

#[test]
fn test_WB_BUILD_163_a_start_with_no_commit_at_head_is_refused_with_1_before_anything_is_written() {
    let bench = listed();
    bench.git(&["checkout", "-q", "--orphan", "lone"]);
    let before = bench.git(&["for-each-ref"]);

    let said = bench.start(&["tester", "54", "--ignore-branch"]);

    assert_eq!(said.code, 1, "{}", said.err);
    assert!(said.err.contains("HEAD"), "{}", said.err);
    assert_eq!(bench.git(&["for-each-ref"]), before);
    assert!(tip_of(&bench).is_empty());
    assert_eq!(bench.sessions(), 0, "{:?}", bench.calls());
}

#[test]
fn test_WB_BUILD_163_a_revert_with_no_commit_at_head_is_refused_with_1_and_writes_no_marker() {
    let bench = listed();
    ok(&bench.start(&["tester", "54"]));
    let tip = tip_of(&bench);
    bench.git(&["checkout", "-q", "--orphan", "lone"]);

    let run = bench.wb(&["revert", "54", "tester"]);

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("HEAD"), "{}", text(&run.stderr));
    assert_eq!(tip_of(&bench), tip);
}

// ---- WB-BUILD-165: one line, naming what fails (the review of #326) ----

#[test]
fn test_WB_BUILD_165_a_stamp_of_another_repository_is_one_line_that_names_the_line_that_fails() {
    let bench = listed();
    let base = bench.head();
    raw_stamp(&bench, &format!("[stamp] tester round 1\nrole: tester\nissue: 54\njob: none\nbase: {}\n", "1".repeat(40)));
    raw_stamp(&bench, &format!("[stamp] review round 1\nrole: review\nissue: 54\njob: {}\nbase: {}\n", "2".repeat(40), base));

    let status = bench.status();

    let tester = status_of(&bench, "[stamp] tester round 1");
    let review = status_of(&bench, "[stamp] review round 1");
    assert!(tester.contains("not counted: a stamp of another repository") && tester.contains("base"), "{}", status);
    assert!(review.contains("not counted: a stamp of another repository") && review.contains("`job:`"), "{}", status);
    assert!(!tester.contains("`job:`") && !review.contains("its base"), "{}", status);
    // One line each: the text of the reason holds no line break.
    assert_eq!(status.matches("a stamp of another repository").count(), 2, "{}", status);
}

// ---- WB-BUILD-163: a base is a commit object, not a tag (the review of #326) ----

#[test]
fn test_WB_BUILD_163_a_base_that_is_a_tag_object_is_not_counted() {
    let bench = listed();
    bench.git(&["tag", "-a", "-m", "A tag", "tagged", "HEAD"]);
    let tag = bench.git(&["rev-parse", "refs/tags/tagged"]).trim().to_string();
    raw_stamp(&bench, &format!("[stamp] tester round 1\nrole: tester\nissue: 54\njob: none\nbase: {}\n", tag));

    let line = status_of(&bench, "[stamp] tester round 1");

    assert!(line.contains("not counted: a stamp of another repository") && line.contains("base"), "{}", bench.status());
}

// ---- WB-BUILD-166 to WB-BUILD-170: the job's ref as `wb` writes it (#327) ----

/// git in the repository with `input` on standard input, which must end with 0; what it printed, trimmed.
fn git_input(bench: &Bench, args: &[&str], input: &[u8]) -> String {
    use std::io::Write;
    let mut child = Command::new("git")
        .args(args)
        .current_dir(&bench.repo.path)
        .env_clear()
        .envs(env_in(&bench.home.path))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    let run = child.wait_with_output().unwrap();
    assert!(run.status.success(), "git {:?}: {}", args, text(&run.stderr));
    text(&run.stdout).trim().to_string()
}

/// The lines of `wb status 54` that say the job's ref is not as `wb` writes it.
fn not_as_written(bench: &Bench) -> Vec<String> {
    bench.status().lines().filter(|line| line.contains("is not as wb writes it")).map(str::to_string).collect()
}

/// `wb start tester 54` refused with 1 because the job's ref is not as `wb` writes it, with nothing written: no ref
/// moves and no session starts.
fn start_refused(bench: &Bench) {
    let before = bench.git(&["for-each-ref"]);
    let sessions = bench.sessions();
    let said = bench.start(&["tester", "54"]);
    assert_eq!(said.code, 1, "{}", said.err);
    assert!(said.err.contains("is not as wb writes it"), "{}", said.err);
    assert_eq!(bench.git(&["for-each-ref"]), before);
    assert_eq!(bench.sessions(), sessions, "{:?}", bench.calls());
}

/// A plan of `name` with `text`, by `wb plan add`, which must end with 0.
fn plan_named(bench: &Bench, name: &str, text: &str) {
    let said = bench.child_in(&bench.repo.path, "plan", &["add", name, "-m", text]);
    assert_eq!(said.code, 0, "{}", said.err);
}

#[test]
fn test_WB_BUILD_166_a_signed_stamp_spliced_in_by_a_signed_merge_counts_for_nothing_and_the_ref_is_refused() {
    let bench = bench(false, false);
    ok(&bench.start(&["tester", "54"]));
    let old = tip_of(&bench);
    let tree = bench.git(&["rev-parse", &format!("{}^{{tree}}", old)]).trim().to_string();
    let base = bench.head();
    // Every commit signed by a key with `start`: a stamp on a chain of its own, and a merge of it that is a stamp too.
    let empty = bench.git(&["mktree"]).trim().to_string();
    let other = raw_commit(&bench, &empty, &[], &format!("[stamp] build round 1\nrole: build\nissue: 54\njob: none\nbase: {}\n", base), true);
    let merge = raw_commit(&bench, &tree, &[old.as_str(), other.as_str()], &format!("[stamp] review round 1\nrole: review\nissue: 54\njob: none\nbase: {}\n", base), true);
    bench.git(&["update-ref", JOB, &merge]);

    let status = bench.status();

    assert!(!status.contains("Tested:") && !status.contains("Built:") && !status.contains("Reviewed:"), "{}", status);
    let lines = not_as_written(&bench);
    assert_eq!(lines.len(), 1, "{}", status);
    assert!(lines[0].contains(&merge) && lines[0].contains("merge"), "{}", lines[0]);
    start_refused(&bench);
}

#[test]
fn test_WB_BUILD_166_an_unsigned_commit_on_top_of_signed_stamps_counts_for_nothing_and_start_and_revert_are_refused() {
    let bench = bench(false, false);
    ok(&bench.start(&["tester", "54"]));
    unsigned_stamps_on_ref(&bench.repo.path, "54", &["[stamp] review round 1\nrole: review\n", "[stamp] fix round 1\nrole: fix\n"]);
    let commits = bench.on_ref();
    let (newest, first_bad) = (commits[0].clone(), commits[1].clone());

    let status = bench.status();

    assert!(!status.contains("Tested:"), "{}", status);
    // One line, naming the first commit that is not as `wb` writes it.
    let lines = not_as_written(&bench);
    assert_eq!(lines.len(), 1, "{}", status);
    assert!(lines[0].contains(&first_bad) && lines[0].contains("unsigned"), "{}", lines[0]);
    assert!(!lines[0].contains(&newest), "{}", lines[0]);
    start_refused(&bench);
    let run = bench.wb(&["revert", "54", "tester"]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("is not as wb writes it"), "{}", text(&run.stderr));
    assert_eq!(tip_of(&bench), newest);
}

#[test]
fn test_WB_BUILD_166_an_unsigned_tip_that_sets_the_plan_lets_no_stamp_count() {
    let bench = bench(false, false);
    ok(&bench.start(&["tester", "54"]));
    // The tree of a plan with this text, from a plan of another name.
    plan_named(&bench, "other", "The brakes hold.");
    let job = blob(&bench, "The brakes hold.\n");
    let base = bench.head();
    // A signed stamp that names that job, and an unsigned commit on top whose tree holds the plan.
    let stamp = raw_stamp(&bench, &format!("[stamp] review round 1\nrole: review\nissue: 54\njob: {}\nbase: {}\n", job, base));
    let tree = bench.git(&["rev-parse", "refs/watchbill/other^{tree}"]).trim().to_string();
    let top = raw_commit(&bench, &tree, &[stamp.as_str()], &format!("plan add 54\n\nsource {}\n", job), false);
    bench.git(&["update-ref", JOB, &top]);

    let status = bench.status();

    assert!(!status.contains("Tested:") && !status.contains("Reviewed:"), "{}", status);
    let lines = not_as_written(&bench);
    assert_eq!(lines.len(), 1, "{}", status);
    assert!(lines[0].contains(&top) && lines[0].contains("unsigned"), "{}", lines[0]);
}

#[test]
fn test_WB_BUILD_166_an_unsigned_tip_with_a_removed_plan_lets_no_stamp_count() {
    let bench = bench(false, false);
    bench.plan("The brakes hold.");
    ok(&bench.start(&["tester", "54"]));
    let base = bench.head();
    let stamp = raw_stamp(&bench, &format!("[stamp] review round 1\nrole: review\nissue: 54\njob: none\nbase: {}\n", base));
    // An unsigned commit on top whose tree holds the marker of a plan removed.
    let marker = git_input(&bench, &["hash-object", "-w", "--stdin"], b"by: Ada Lovelace <ada@example.org>\ntime: 2026-11-25T10:00:00Z\n");
    let mut entries = bench.git(&["ls-tree", &format!("{}^{{tree}}", stamp)]);
    entries.push_str(&format!("100644 blob {}\tremoved\n", marker));
    let tree = git_input(&bench, &["mktree"], entries.as_bytes());
    let top = raw_commit(&bench, &tree, &[stamp.as_str()], "plan remove 54\n", false);
    bench.git(&["update-ref", JOB, &top]);

    let status = bench.status();

    assert!(!status.contains("Tested:") && !status.contains("Reviewed:"), "{}", status);
    let lines = not_as_written(&bench);
    assert_eq!(lines.len(), 1, "{}", status);
    assert!(lines[0].contains(&top), "{}", lines[0]);
}

#[test]
fn test_WB_BUILD_169_a_plan_commit_signed_by_a_key_without_accept_makes_the_ref_not_as_wb_writes_it() {
    let bench = bench(false, false);
    bench.plan("The brakes hold.");
    let plan = bench.on_ref()[0].clone();
    ok(&bench.start(&["tester", "54"]));
    // The list now holds Ada's key with `start` alone.
    bench.signers("ada", "start");

    let status = bench.status();

    assert!(!status.contains("Tested:"), "{}", status);
    let lines = not_as_written(&bench);
    assert_eq!(lines.len(), 1, "{}", status);
    assert!(lines[0].contains(&plan) && lines[0].contains("without the role accept"), "{}", lines[0]);
    start_refused(&bench);
}

#[test]
fn test_WB_BUILD_166_a_line_of_commits_as_wb_writes_them_counts_as_before_and_plan_commits_are_signed() {
    let bench = bench(false, false);
    bench.plan("The brakes hold.");
    ok(&bench.start(&["tester", "54"]));
    // It says how the commit is signed before it writes it (WB-JOB-14).
    let added = bench.child_in(&bench.repo.path, "plan", &["add", "54", "-m", "And they hold in the wet."]);
    assert_eq!(added.code, 0, "{}", added.err);
    assert!(added.out.contains("signed by ada@example.org, with the role accept in .watchbill/signers"), "{}", added.out);
    ok(&bench.start(&["tester", "54"]));
    assert!(bench.message(&bench.on_ref()[0]).starts_with("[stamp] tester round 2\n"), "{}", bench.message(&bench.on_ref()[0]));

    let status = bench.status();

    assert!(status.contains("Tested:"), "{}", status);
    assert!(not_as_written(&bench).is_empty(), "{}", status);
    // The commits of `wb plan add` are signed by Ada's key, as git checks them.
    let allowed = bench.home.path.join("allowed");
    fs::write(&allowed, format!("ada@example.org {}\n", bench.public("ada"))).unwrap();
    let setting = format!("gpg.ssh.allowedSignersFile={}", allowed.display());
    let commits = bench.on_ref();
    for plan in [&commits[1], &commits[3]] {
        assert!(bench.message(plan).starts_with("plan add 54\n"), "{}", bench.message(plan));
        let verified = bench.run_in("git", &["-c", &setting, "verify-commit", plan], &bench.repo.path);
        assert!(verified.status.success(), "{}", text(&verified.stderr));
    }
    // A plan removed by `wb plan remove`, and a stamp after it: the line still counts. The stamps of the plan no longer
    // do (WB-BUILD-164), so the new stamp is round 1 again, and counts.
    let said = bench.child_in(&bench.repo.path, "plan", &["remove", "54"]);
    assert_eq!(said.code, 0, "{}", said.err);
    assert!(said.out.contains("signed by ada@example.org, with the role accept in .watchbill/signers"), "{}", said.out);
    let removed = bench.on_ref()[0].clone();
    let verified = bench.run_in("git", &["-c", &setting, "verify-commit", &removed], &bench.repo.path);
    assert!(verified.status.success(), "{}", text(&verified.stderr));
    ok(&bench.start(&["tester", "54"]));
    let status = bench.status();
    assert!(not_as_written(&bench).is_empty(), "{}", status);
    assert!(status.contains("[stamp] tester round 1  (on refs/watchbill/54, signed by ada@example.org)"), "{}", status);
    assert!(status.contains("Tested:"), "{}", status);
}

#[test]
fn test_WB_BUILD_168_plan_add_and_remove_on_a_ref_not_as_wb_writes_it_are_refused_with_1_before_writing() {
    let bench = bench(false, false);
    bench.plan("The brakes hold.");
    unsigned_stamps_on_ref(&bench.repo.path, "54", &["[stamp] review round 1\nrole: review\n"]);
    let before = bench.git(&["for-each-ref"]);

    let added = bench.child_in(&bench.repo.path, "plan", &["add", "54", "-m", "And they hold in the wet."]);
    let removed = bench.child_in(&bench.repo.path, "plan", &["remove", "54"]);

    for said in [&added, &removed] {
        assert_eq!(said.code, 1, "{}", said.err);
        assert!(said.err.contains("is not as wb writes it"), "{}", said.err);
        assert!(!said.out.contains("It writes:"), "{}", said.out);
    }
    assert_eq!(bench.git(&["for-each-ref"]), before);
}

#[test]
fn test_WB_BUILD_169_plan_add_and_remove_are_refused_with_1_before_writing_without_a_key_with_accept() {
    let bench = bench(false, false);
    bench.plan("The brakes hold.");
    bench.signers("ada", "start");
    let before = bench.git(&["for-each-ref"]);

    let added = bench.child_in(&bench.repo.path, "plan", &["add", "54", "-m", "And they hold in the wet."]);
    let removed = bench.child_in(&bench.repo.path, "plan", &["remove", "54"]);
    let other = bench.child_in(&bench.repo.path, "plan", &["add", "55", "-m", "The doors close."]);

    for said in [&added, &removed, &other] {
        assert_eq!(said.code, 1, "{}", said.err);
        assert!(said.err.contains(&bench.public("ada")) && said.err.contains("without the role accept"), "{}", said.err);
        assert!(!said.out.contains("It writes:"), "{}", said.out);
    }
    assert_eq!(bench.git(&["for-each-ref"]), before);

    // With no signing key set, the setting is named.
    bench.git(&["config", "--unset", "user.signingkey"]);
    let said = bench.child_in(&bench.repo.path, "plan", &["add", "55", "-m", "The doors close."]);
    assert_eq!(said.code, 1, "{}", said.err);
    assert!(said.err.contains("user.signingkey"), "{}", said.err);
    assert_eq!(bench.git(&["for-each-ref"]), before);
}

#[test]
fn test_WB_BUILD_169_with_no_list_plan_add_is_refused_with_1_naming_the_file_and_the_role_accept() {
    let bench = bench_listed(false, false, false);
    let before = bench.git(&["for-each-ref"]);

    let said = bench.child_in(&bench.repo.path, "plan", &["add", "54", "-m", "The brakes hold."]);

    assert_eq!(said.code, 1, "{}", said.err);
    assert!(said.err.contains("no .watchbill/signers on origin/main"), "{}", said.err);
    assert!(said.err.contains("roles=\"accept\""), "{}", said.err);
    assert_eq!(bench.git(&["for-each-ref"]), before);
}

#[test]
fn test_WB_TASK_56_with_no_list_plan_add_and_remove_are_refused_with_1_and_write_nothing() {
    let bench = bench(false, false);
    bench.plan("The brakes hold.");
    // The list gone from `main`.
    bench.git(&["checkout", "-q", "main"]);
    bench.git(&["rm", "-q", "--", SIGNERS]);
    bench.git(&["commit", "-q", "-m", "No keys"]);
    bench.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
    bench.git(&["checkout", "-q", "issue-54"]);
    let before = bench.git(&["for-each-ref"]);

    let added = bench.child_in(&bench.repo.path, "plan", &["add", "54", "-m", "And they hold in the wet."]);
    let removed = bench.child_in(&bench.repo.path, "plan", &["remove", "54"]);

    for said in [&added, &removed] {
        assert_eq!(said.code, 1, "{}", said.err);
        assert!(said.err.contains("no .watchbill/signers on origin/main") && said.err.contains("roles=\"accept\""), "{}", said.err);
        assert!(!said.out.contains("It writes:"), "{}", said.out);
    }
    assert_eq!(bench.git(&["for-each-ref"]), before);
}

#[test]
fn test_WB_BUILD_170_plan_show_says_the_ref_is_not_as_wb_writes_it() {
    let bench = bench(false, false);
    bench.plan("The brakes hold.");
    unsigned_stamps_on_ref(&bench.repo.path, "54", &["[stamp] review round 1\nrole: review\n"]);
    let top = tip_of(&bench);

    let run = bench.wb(&["plan", "show", "54"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    let lines: Vec<&str> = out.lines().filter(|line| line.contains("is not as wb writes it")).collect();
    assert_eq!(lines.len(), 1, "{}", out);
    assert!(lines[0].contains(&top), "{}", out);
}

#[test]
fn test_WB_BUILD_170_the_readme_says_what_the_ref_as_wb_writes_it_is_and_how_a_person_repairs_it() {
    let readme = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"));
    assert!(readme.contains("as `wb` writes it"), "README.md");
    assert!(readme.contains("Repairing the job's ref"), "README.md");
    assert!(readme.contains("git push --force-with-lease origin"), "README.md");
    assert!(readme.contains("holds no plan, or a removed one,"), "README.md");
}

// ---- The review of PR #328 ----

/// The tree of `commit` with an entry `name` more, a blob of `text`.
fn tree_with(bench: &Bench, commit: &str, name: &str, text: &[u8]) -> String {
    let blob = git_input(bench, &["hash-object", "-w", "--stdin"], text);
    let mut entries = bench.git(&["ls-tree", &format!("{}^{{tree}}", commit)]);
    entries.push_str(&format!("100644 blob {}\t{}\n", blob, name));
    git_input(bench, &["mktree"], entries.as_bytes())
}

/// The tree of `commit`.
fn tree_of(bench: &Bench, commit: &str) -> String {
    bench.git(&["rev-parse", &format!("{}^{{tree}}", commit)]).trim().to_string()
}

#[test]
fn test_WB_BUILD_168_a_bad_ref_voids_the_acceptance_a_planner_round_voided_and_prompt_is_refused() {
    let bench = bench(false, false);
    bench.issue_file();
    // A signed planner round after the acceptance voids it.
    stamps_on_ref(&bench.repo.path, "54", &["[stamp] planner round 1\nrole: planner\ntime: 2999-01-01T00:00:00Z\n"]);
    let mut with_round = vec!["build", "54", "--round", "1"];
    with_round.extend(REST);
    let mut without = vec!["build", "54"];
    without.extend(REST);
    assert_eq!(bench.prompt(&with_round).code, 1);
    assert!(!bench.status().contains("Accepted:"), "{}", bench.status());

    // One unsigned commit on top: nothing on the ref counts, and the void stays.
    unsigned_stamps_on_ref(&bench.repo.path, "54", &["[stamp] tester round 1\nrole: tester\n"]);
    let top = tip_of(&bench);

    for args in [&with_round, &without] {
        let said = bench.prompt(args);
        assert_eq!(said.code, 1, "{}", said.err);
        assert!(said.out.is_empty(), "{}", said.out);
        assert!(said.err.contains("is not as wb writes it") && said.err.contains(&top), "{}", said.err);
    }
    let status = bench.status();
    assert!(!status.contains("Accepted:"), "{}", status);
    assert_eq!(not_as_written(&bench).len(), 1, "{}", status);
}

#[test]
fn test_WB_BUILD_168_plan_accept_on_a_bad_ref_is_refused_with_1_and_writes_nothing() {
    let bench = bench(false, false);
    bench.plan("The brakes hold.");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    put(&bench.repo.path, YML, &BARE_ISSUE.replace("{issue}", "54"));
    unsigned_stamps_on_ref(&bench.repo.path, "54", &["[stamp] review round 1\nrole: review\n"]);
    let before = bench.git(&["for-each-ref"]);

    let run = bench.wb(&["plan", "accept", "--no-commit", "54"]);

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("is not as wb writes it"), "{}", text(&run.stderr));
    assert!(!bench.repo.path.join(".watchbill/issues/54.accepted").exists());
    assert_eq!(bench.git(&["for-each-ref"]), before);
}

#[test]
fn test_WB_BUILD_166_a_signed_stamp_whose_tree_marks_the_plan_removed_makes_the_ref_not_as_wb_writes_it() {
    let bench = bench(false, false);
    bench.plan("The brakes hold.");
    ok(&bench.start(&["tester", "54"]));
    let tip = tip_of(&bench);
    // A stamp signed by a key with `start`, whose tree adds the marker of `wb plan remove`.
    let tree = tree_with(&bench, &tip, "removed", b"by: Ada Lovelace <ada@example.org>\ntime: 2026-11-26T10:00:00Z\n");
    let stamp = raw_commit(&bench, &tree, &[tip.as_str()], &format!("[stamp] review round 1\nrole: review\nissue: 54\njob: none\nbase: {}\n", bench.head()), true);
    bench.git(&["update-ref", JOB, &stamp]);

    let status = bench.status();

    assert!(!status.contains("Tested:") && !status.contains("Reviewed:"), "{}", status);
    let lines = not_as_written(&bench);
    assert_eq!(lines.len(), 1, "{}", status);
    assert!(lines[0].contains(&stamp) && lines[0].contains("tree"), "{}", lines[0]);
    start_refused(&bench);
}

#[test]
fn test_WB_BUILD_166_a_signed_marker_whose_tree_is_not_its_parents_makes_the_ref_not_as_wb_writes_it() {
    let bench = bench(false, false);
    ok(&bench.start(&["tester", "54"]));
    ok(&bench.start(&["tester", "54"]));
    let tip = tip_of(&bench);
    let tree = tree_with(&bench, &tip, "extra", b"more\n");
    let marker = raw_commit(&bench, &tree, &[tip.as_str()], &format!("[revert] tester round 2\nissue: 54\njob: none\nbase: {}\n", bench.head()), true);
    bench.git(&["update-ref", JOB, &marker]);

    let lines = not_as_written(&bench);

    assert_eq!(lines.len(), 1, "{}", bench.status());
    assert!(lines[0].contains(&marker) && lines[0].contains("tree"), "{}", lines[0]);
}

#[test]
fn test_WB_BUILD_167_a_side_chain_is_named_by_its_merge_and_the_commit_before_it_keeps_every_genuine_stamp() {
    let bench = bench(false, false);
    ok(&bench.start(&["tester", "54"]));
    ok(&bench.start(&["tester", "54"]));
    let (a2, a1) = (bench.on_ref()[0].clone(), bench.on_ref()[1].clone());
    let base = bench.head();
    // An unsigned side commit after a1, and a signed merge of a2 and it.
    let side = raw_commit(&bench, &tree_of(&bench, &a1), &[a1.as_str()], &format!("[stamp] review round 1\nrole: review\nissue: 54\njob: none\nbase: {}\n", base), false);
    let merge = raw_commit(&bench, &tree_of(&bench, &a2), &[a2.as_str(), side.as_str()], &format!("[stamp] build round 1\nrole: build\nissue: 54\njob: none\nbase: {}\n", base), true);
    bench.git(&["update-ref", JOB, &merge]);

    let lines = not_as_written(&bench);

    assert_eq!(lines.len(), 1, "{}", bench.status());
    assert!(lines[0].contains(&merge) && !lines[0].contains(&side), "{}", lines[0]);
    // The repair the README gives: the ref moved to the commit before the one named holds both rounds.
    bench.git(&["update-ref", JOB, &format!("{}^", merge)]);
    let status = bench.status();
    assert!(not_as_written(&bench).is_empty(), "{}", status);
    assert!(status.contains("[stamp] tester round 2  (on refs/watchbill/54, signed by ada@example.org)"), "{}", status);
}

#[test]
fn test_WB_BUILD_167_a_side_chain_with_its_own_root_is_named_by_its_merge() {
    let bench = bench(false, false);
    ok(&bench.start(&["tester", "54"]));
    let tip = tip_of(&bench);
    let empty = bench.git(&["mktree"]).trim().to_string();
    let root = raw_commit(&bench, &empty, &[], "A commit of another chain\n", false);
    let merge = raw_commit(&bench, &tree_of(&bench, &tip), &[tip.as_str(), root.as_str()], &format!("[stamp] review round 1\nrole: review\nissue: 54\njob: none\nbase: {}\n", bench.head()), true);
    bench.git(&["update-ref", JOB, &merge]);

    let lines = not_as_written(&bench);

    assert_eq!(lines.len(), 1, "{}", bench.status());
    assert!(lines[0].contains(&merge) && !lines[0].contains(&root), "{}", lines[0]);
    bench.git(&["update-ref", JOB, &format!("{}^", merge)]);
    assert!(not_as_written(&bench).is_empty(), "{}", bench.status());
    assert!(bench.status().contains("Tested:"), "{}", bench.status());
}

#[test]
fn test_WB_BUILD_166_a_graft_of_this_machine_hides_no_stamp() {
    let bench = bench(false, false);
    ok(&bench.start(&["tester", "54"]));
    ok(&bench.start(&["tester", "54"]));
    // A graft that makes the tip a commit with no parent, on this machine alone.
    let git_dir = bench.git(&["rev-parse", "--absolute-git-dir"]).trim().to_string();
    fs::create_dir_all(Path::new(&git_dir).join("info")).unwrap();
    fs::write(Path::new(&git_dir).join("info/grafts"), format!("{}\n", tip_of(&bench))).unwrap();

    let status = bench.status();

    assert!(status.contains("[stamp] tester round 1  (on refs/watchbill/54, signed by ada@example.org)"), "{}", status);
    ok(&bench.start(&["tester", "54"]));
    assert!(bench.message(&tip_of(&bench)).starts_with("[stamp] tester round 3\n"), "{}", bench.message(&tip_of(&bench)));
}

#[test]
fn test_WB_BUILD_166_in_a_shallow_repository_the_ref_is_not_as_wb_writes_it() {
    let bench = bench(false, false);
    ok(&bench.start(&["tester", "54"]));
    ok(&bench.start(&["tester", "54"]));
    let git_dir = bench.git(&["rev-parse", "--absolute-git-dir"]).trim().to_string();
    fs::write(Path::new(&git_dir).join("shallow"), format!("{}\n", tip_of(&bench))).unwrap();

    let lines = not_as_written(&bench);

    assert_eq!(lines.len(), 1, "{}", bench.status());
    assert!(lines[0].contains("shallow"), "{}", lines[0]);
    assert!(!bench.status().contains("Tested:"), "{}", bench.status());
    start_refused(&bench);
}

#[test]
fn test_WB_BUILD_166_a_ref_at_an_annotated_tag_or_a_tree_is_not_as_wb_writes_it() {
    let bench = bench(false, false);
    ok(&bench.start(&["tester", "54"]));
    let tip = tip_of(&bench);
    bench.git(&["tag", "-a", "-m", "A tag", "on-the-ref", &tip]);
    let tag = bench.git(&["rev-parse", "refs/tags/on-the-ref"]).trim().to_string();
    bench.git(&["update-ref", JOB, &tag]);

    let lines = not_as_written(&bench);

    assert_eq!(lines.len(), 1, "{}", bench.status());
    assert!(lines[0].contains("tag"), "{}", lines[0]);
    assert!(!bench.status().contains("Tested:"), "{}", bench.status());
    start_refused(&bench);

    bench.git(&["update-ref", JOB, &tree_of(&bench, &tip)]);
    let lines = not_as_written(&bench);
    assert_eq!(lines.len(), 1, "{}", bench.status());
    assert!(lines[0].contains("tree"), "{}", lines[0]);
}

// The cases WB-BUILD-166 lists that 3b3d8ca built with no test of their own (the review of #328, note 10).

#[test]
fn test_WB_BUILD_166_a_signed_commit_of_another_kind_makes_the_ref_not_as_wb_writes_it() {
    let bench = bench(false, false);
    ok(&bench.start(&["tester", "54"]));
    let tip = tip_of(&bench);
    let other = raw_commit(&bench, &tree_of(&bench, &tip), &[tip.as_str()], "A note on the ref\n", true);
    bench.git(&["update-ref", JOB, &other]);

    let lines = not_as_written(&bench);

    assert_eq!(lines.len(), 1, "{}", bench.status());
    assert!(lines[0].contains(&other) && lines[0].contains("neither a stamp"), "{}", lines[0]);
    assert!(!bench.status().contains("Tested:"), "{}", bench.status());
}

#[test]
fn test_WB_BUILD_166_a_commit_over_64_kib_makes_the_ref_not_as_wb_writes_it() {
    let bench = bench(false, false);
    ok(&bench.start(&["tester", "54"]));
    let padding = "x".repeat(70 * 1024);
    let big = raw_stamp(&bench, &format!("[stamp] review round 1\nrole: review\nissue: 54\njob: none\nbase: {}\n\n{}\n", bench.head(), padding));

    let lines = not_as_written(&bench);

    assert_eq!(lines.len(), 1, "{}", bench.status());
    assert!(lines[0].contains(&big) && lines[0].contains("larger than 64 KiB"), "{}", lines[0]);
    assert!(!bench.status().contains("Tested:"), "{}", bench.status());
}

#[test]
fn test_WB_BUILD_166_a_stamp_signed_by_a_key_with_accept_alone_makes_the_ref_not_as_wb_writes_it() {
    let bench = bench(false, false);
    ok(&bench.start(&["tester", "54"]));
    let stamp = tip_of(&bench);
    bench.signers("ada", "accept");

    let lines = not_as_written(&bench);

    assert_eq!(lines.len(), 1, "{}", bench.status());
    assert!(lines[0].contains(&stamp) && lines[0].contains("without the role start"), "{}", lines[0]);
}

#[test]
fn test_WB_BUILD_166_an_octopus_merge_makes_the_ref_not_as_wb_writes_it() {
    let bench = bench(false, false);
    ok(&bench.start(&["tester", "54"]));
    let tip = tip_of(&bench);
    let base = bench.head();
    let empty = bench.git(&["mktree"]).trim().to_string();
    let stamp = |role: &str| format!("[stamp] {} round 1\nrole: {}\nissue: 54\njob: none\nbase: {}\n", role, role, base);
    let one = raw_commit(&bench, &empty, &[], &stamp("build"), true);
    let two = raw_commit(&bench, &empty, &[], &stamp("review"), true);
    let octopus = raw_commit(&bench, &tree_of(&bench, &tip), &[tip.as_str(), one.as_str(), two.as_str()], &stamp("fix"), true);
    bench.git(&["update-ref", JOB, &octopus]);

    let lines = not_as_written(&bench);

    assert_eq!(lines.len(), 1, "{}", bench.status());
    assert!(lines[0].contains(&octopus) && lines[0].contains("merge of 3 commits"), "{}", lines[0]);
}

#[test]
fn test_WB_BUILD_170_the_readme_says_a_forced_push_to_a_chain_all_signed_is_not_seen() {
    let readme = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"));
    assert!(readme.contains("A forced push to another chain"), "README.md");
    assert!(readme.contains("only the holders of the keys"), "README.md");
}
