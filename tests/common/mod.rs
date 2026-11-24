//! What the tests of `wb init` share: folders made for the test, repositories, and the program run as a person runs it.

#![allow(dead_code)]

use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The version of the package, which `wb init` writes as the line `watchbill:`.
pub const REVISION: &str = env!("CARGO_PKG_VERSION");

pub const HEAD: [&str; 2] = [
    "# How Watchbill is set up in this repository. Written by `wb init`.",
    "# `watchbill` is the version of the package the repository is set up for.",
];

static COUNT: AtomicUsize = AtomicUsize::new(0);

/// A folder made for one test, removed when the test is over.
pub struct Folder {
    base: PathBuf,
    pub path: PathBuf,
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

/// A folder of the given name in a folder of its own.
pub fn folder_named(name: impl AsRef<OsStr>) -> Folder {
    let base = std::env::temp_dir().join(format!(
        "wb-test-{}-{}",
        std::process::id(),
        COUNT.fetch_add(1, Ordering::SeqCst)
    ));
    let path = base.join(name.as_ref());
    fs::create_dir_all(&path).expect("the folder could not be made");
    Folder { base, path }
}

pub fn folder() -> Folder {
    folder_named("brake-controller")
}

/// A state folder that cannot be made, below a file that is not a folder: a test that runs the program
/// writes no lines of a start (#124) in the real home folder. A test of those lines gives its own.
pub const NO_STATE: &str = "/dev/null/wb-test-state";

/// The environment of the tests. A hook of git hands its children GIT_DIR and the like. With them,
/// the `git init` of a test would act on the repository the tests are run from, so they never reach a child.
/// `XDG_STATE_HOME` is `NO_STATE`.
pub fn env() -> Vec<(String, String)> {
    let mut vars: Vec<(String, String)> =
        std::env::vars().filter(|(name, _)| !name.starts_with("GIT_") && name != "XDG_STATE_HOME").collect();
    vars.push(("XDG_STATE_HOME".to_string(), NO_STATE.to_string()));
    vars
}

pub fn git(folder: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(args)
        .env_clear()
        .envs(env())
        .stdout(Stdio::null())
        .status()
        .expect("git could not be started");
    assert!(status.success(), "git {:?} failed", args);
}

/// A repository with `origin` at the given address, when one is given.
pub fn repository_named(name: impl AsRef<OsStr>, origin: Option<&str>) -> Folder {
    let folder = folder_named(name);
    git(&folder.path, &["init", "-q"]);
    // A commit may start `git maintenance run --auto` in the background, which makes and removes
    // `.git/objects/maintenance.lock` while a test compares the files of the repository (#135).
    git(&folder.path, &["config", "gc.auto", "0"]);
    git(&folder.path, &["config", "maintenance.auto", "false"]);
    if let Some(origin) = origin {
        git(&folder.path, &["remote", "add", "origin", origin]);
    }
    folder
}

pub fn repository(origin: Option<&str>) -> Folder {
    repository_named("brake-controller", origin)
}

/// Puts a file in a folder, and the folders above it.
pub fn put(folder: &Path, path: &str, text: &str) {
    let file = folder.join(path);
    fs::create_dir_all(file.parent().unwrap()).expect("the folder could not be made");
    fs::write(file, text).expect("the file could not be written");
}

pub fn config(folder: &Path) -> PathBuf {
    folder.join(".watchbill").join("watchbill.yml")
}

pub fn read(path: &Path) -> String {
    fs::read_to_string(path).expect("the file could not be read")
}

pub fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("wb wrote text that is not UTF-8")
}

/// What a person would type, one answer per line.
pub fn lines(answers: &[&str]) -> String {
    answers.iter().map(|answer| format!("{}\n", answer)).collect()
}

/// Runs `wb` in the folder with the given bytes on standard input.
pub fn wb_bytes(folder: &Path, args: &[&str], typed: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(args)
        .current_dir(folder)
        .env_clear()
        .envs(env())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("wb could not be started");
    // The program may end before it has read all of it.
    let _ = child.stdin.take().unwrap().write_all(typed);
    child.wait_with_output().expect("wb could not be waited for")
}

/// Runs `wb` in the folder; `typed` is what a person would type, one answer per line.
pub fn wb(folder: &Path, args: &[&str], typed: &str) -> Output {
    wb_bytes(folder, args, typed.as_bytes())
}

pub fn code(run: &Output) -> Option<i32> {
    run.status.code()
}

/// An issue's file with no requirement, as a planner might leave it.
pub const BARE_ISSUE: &str = "issue: '{issue}'
from: 3f0c1d9e
requirements: []
interface: |
  none
changes: []
reads:
  tester: []
  build: []
  review: []
  fix: []
existing_tests_changed: []
questions: []
";

/// Commits `.watchbill/watchbill.yml` as it is in the working tree, when `HEAD` has it otherwise or has none:
/// `wb` runs a pattern of the file only while it is as `HEAD` has it (#186). The commit has an identity of
/// its own, given for it alone, so that a test of a missing identity is not spoilt.
pub fn commit_config(folder: &Path) {
    let held = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(["show", "HEAD:.watchbill/watchbill.yml"])
        .env_clear()
        .envs(env())
        .stderr(Stdio::null())
        .output()
        .expect("git could not be started");
    if held.status.success() && held.stdout == fs::read(config(folder)).expect("watchbill.yml could not be read") {
        return;
    }
    git(folder, &["add", "--", ".watchbill/watchbill.yml"]);
    git(folder, &["-c", "user.name=Ada Lovelace", "-c", "user.email=ada@example.org", "-c", "commit.gpgsign=false", "commit", "-q", "-m", "The patterns"]);
}

/// As `commit_config`, and every other branch that stood at `HEAD` moves with it, so that the branches stand
/// as they did: the one a test checked out is the only one that has the commit.
pub fn commit_config_keeping_refs(folder: &Path) {
    let text_of = |args: &[&str]| {
        let run = Command::new("git").arg("-C").arg(folder).args(args).env_clear().envs(env()).stderr(Stdio::null()).output().expect("git could not be started");
        String::from_utf8_lossy(&run.stdout).to_string()
    };
    let old = text_of(&["rev-parse", "HEAD"]).trim().to_string();
    let current = text_of(&["symbolic-ref", "-q", "HEAD"]).trim().to_string();
    commit_config(folder);
    let new = text_of(&["rev-parse", "HEAD"]).trim().to_string();
    if old == new || old.is_empty() {
        return;
    }
    for name in text_of(&["for-each-ref", "--points-at", &old, "--format=%(refname)"]).lines() {
        if name != current && !name.ends_with("/HEAD") {
            git(folder, &["update-ref", name, &new, &old]);
        }
    }
}

/// Gives each of the issues a file with no requirement, accepted, and commits the files and the acceptances
/// alone in one commit, so that a role may be handed out for the issue (#97). Gives git an identity.
pub fn issues_ready(folder: &Path, issues: &[&str]) {
    git(folder, &["config", "user.name", "Ada Lovelace"]);
    git(folder, &["config", "user.email", "ada@example.org"]);
    git(folder, &["config", "commit.gpgsign", "false"]);
    commit_config(folder);
    let mut added = Vec::new();
    for issue in issues {
        put(folder, &format!(".watchbill/issues/{}.yml", issue), &BARE_ISSUE.replace("{issue}", issue));
        let run = wb(folder, &["accept", "--no-commit", "--", issue], "");
        assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
        added.push(format!(".watchbill/issues/{}.yml", issue));
        added.push(format!(".watchbill/issues/{}.accepted", issue));
    }
    let mut add = vec!["add", "--"];
    add.extend(added.iter().map(|path| path.as_str()));
    git(folder, &add);
    git(folder, &["commit", "-q", "-m", "The issues' files and their acceptances"]);
}

/// Runs `wb prompt` in the test process, through `watchbill::prompt`, with the organisation's file at
/// `organisation` in place of the path fixed in `wb` (#93), so that no test reads the real `/etc`.
/// `args` are the words after `prompt`. The result is as the program gives it.
pub fn prompt_with(folder: &Path, organisation: &Path, args: &[&str]) -> Output {
    use std::os::unix::process::ExitStatusExt;
    let args: Vec<std::ffi::OsString> = args.iter().map(std::ffi::OsString::from).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = watchbill::prompt(&args, folder, organisation, &mut out, &mut err);
    Output { status: std::process::ExitStatus::from_raw(status << 8), stdout: out, stderr: err }
}

/// Runs `wb start <role>` in the test process, through `watchbill::start`, with the organisation's file at
/// `organisation` (#93) and `terminal` as the answer to whether standard input and output are a terminal
/// (#84). `args` are the role and the words after it. The result is as the program gives it. The lines of
/// the start (#124) are written in a state folder of the call's own, removed after it.
pub fn start_with(folder: &Path, organisation: &Path, terminal: bool, args: &[&str]) -> Output {
    let state = folder_named("state");
    start_in(folder, organisation, terminal, Some(&state.path), args)
}

/// Runs `wb start <role>` as `start_with` does, with `state` the state folder the lines of the start are written
/// below (#124), or none.
///
/// Every session runs in a view, in bubblewrap (#202). Where bubblewrap cannot make a sandbox here, as in a
/// container, the session is run by a stand-in for it that makes none and runs the session in the checkout, and
/// the test says so on standard error: these tests are of the starter, and `tests/view.rs` those of the sandbox.
pub fn start_in(folder: &Path, organisation: &Path, terminal: bool, state: Option<&Path>, args: &[&str]) -> Output {
    use std::os::unix::process::ExitStatusExt;
    let args: Vec<std::ffi::OsString> = args.iter().map(std::ffi::OsString::from).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let stand_in = folder_named("bwrap");
    let sandbox = match watchbill::view::works(Path::new(watchbill::view::BWRAP)) {
        Ok(()) => PathBuf::from(watchbill::view::BWRAP),
        // For the container and release checks (#202): no stand-in, a failure.
        Err(why) if std::env::var_os("WB_TEST_REQUIRE_SANDBOX").is_some_and(|value| value == "1") => {
            panic!("WB_TEST_REQUIRE_SANDBOX=1, and bubblewrap cannot make a sandbox here: {}", why)
        }
        Err(why) => {
            eprintln!("the session runs by a stand-in for bubblewrap, which cannot make a sandbox here: {}", why);
            let path = stand_in.path.join("bwrap");
            fs::write(&path, "#!/bin/sh\nwhile [ $# -gt 0 ] && [ \"$1\" != -- ]; do shift; done\nshift\nexec \"$@\"\n").expect("the stand-in could not be written");
            fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o755)).expect("the stand-in could not be made a program");
            path
        }
    };
    let home = std::env::var_os("HOME").map(PathBuf::from).filter(|path| path.is_absolute());
    let machine = watchbill::Machine { organisation, terminal, state, sandbox: &sandbox, home: home.as_deref() };
    let status = watchbill::start_on(&args, folder, &machine, &mut out, &mut err);
    Output { status: std::process::ExitStatus::from_raw(status << 8), stdout: out, stderr: err }
}

/// An organisation's file that clears the runner `claude-code` for the planner up to `public`.
pub const ORGANISATION: &str = "levels: [public, internal, confidential]
runners:
  claude-code:
    tools: this machine
    model: Anthropic, under the organisation's agreement
    agreement: EU, no retention, no training
    up_to: internal
    planner:
      up_to: public
      model: claude-opus-5-5
      effort: high
";

/// Writes `ORGANISATION` in a folder of its own, and gives `.watchbill/watchbill.yml` of the repository
/// the level `public`, so that the planner is handed out to `claude-code`. The folder is kept by the caller.
pub fn cleared(repository: &Path) -> (Folder, PathBuf) {
    let place = folder_named("organisation");
    let path = place.path.join("organisation.yml");
    fs::write(&path, ORGANISATION).expect("the organisation's file could not be written");
    let mut yml = read(&config(repository));
    yml.push_str("confidentiality: public\n");
    fs::write(config(repository), yml).expect("watchbill.yml could not be written");
    commit_config(repository);
    (place, path)
}

/// As `put`, and the file is committed alone: `wb prompt` and `wb start` take the project's text as `HEAD` has it
/// (#211). The commit has an identity of its own, given for it alone.
pub fn put_committed(folder: &Path, path: &str, text: &str) {
    put(folder, path, text);
    commit_path(folder, path);
}

/// Commits `path` alone as it is in the working tree: added, changed or, where the file is gone, removed.
pub fn commit_path(folder: &Path, path: &str) {
    git(folder, &["add", "-A", "--", path]);
    git(folder, &["-c", "user.name=Ada Lovelace", "-c", "user.email=ada@example.org", "-c", "commit.gpgsign=false", "commit", "-q", "-m", "The project's text"]);
}
