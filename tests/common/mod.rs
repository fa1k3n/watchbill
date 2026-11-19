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

/// The environment of the tests. A hook of git hands its children GIT_DIR and the like. With them,
/// the `git init` of a test would act on the repository the tests are run from, so they never reach a child.
pub fn env() -> Vec<(String, String)> {
    std::env::vars().filter(|(name, _)| !name.starts_with("GIT_")).collect()
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

/// An issue's file with no requirement, as a breakdown might leave it.
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

/// Gives each of the issues a file with no requirement, accepted, and commits the files and the acceptances
/// alone in one commit, so that a role may be handed out for the issue (#97). Gives git an identity.
pub fn issues_ready(folder: &Path, issues: &[&str]) {
    git(folder, &["config", "user.name", "Ada Lovelace"]);
    git(folder, &["config", "user.email", "ada@example.org"]);
    git(folder, &["config", "commit.gpgsign", "false"]);
    let mut added = Vec::new();
    for issue in issues {
        put(folder, &format!(".watchbill/issues/{}.yml", issue), &BARE_ISSUE.replace("{issue}", issue));
        let run = wb(folder, &["accept", "--", issue], "");
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

/// Runs `wb <role>` in the test process, through `watchbill::start`, with the organisation's file at
/// `organisation` (#93) and `terminal` as the answer to whether standard input and output are a terminal
/// (#84). `args` are the role and the words after it. The result is as the program gives it.
pub fn start_with(folder: &Path, organisation: &Path, terminal: bool, args: &[&str]) -> Output {
    use std::os::unix::process::ExitStatusExt;
    let args: Vec<std::ffi::OsString> = args.iter().map(std::ffi::OsString::from).collect();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = watchbill::start(&args, folder, organisation, terminal, &mut out, &mut err);
    Output { status: std::process::ExitStatus::from_raw(status << 8), stdout: out, stderr: err }
}

/// An organisation's file that clears the runner `claude-code` for the breakdown up to `public`.
pub const ORGANISATION: &str = "levels: [public, internal, confidential]
runners:
  claude-code:
    tools: this machine
    model: Anthropic, under the organisation's agreement
    agreement: EU, no retention, no training
    up_to: internal
    breakdown:
      up_to: public
      model: claude-opus-5-5
      effort: high
";

/// Writes `ORGANISATION` in a folder of its own, and gives `.watchbill/watchbill.yml` of the repository
/// the level `public`, so that the breakdown is handed out to `claude-code`. The folder is kept by the caller.
pub fn cleared(repository: &Path) -> (Folder, PathBuf) {
    let place = folder_named("organisation");
    let path = place.path.join("organisation.yml");
    fs::write(&path, ORGANISATION).expect("the organisation's file could not be written");
    let mut yml = read(&config(repository));
    yml.push_str("confidentiality: public\n");
    fs::write(config(repository), yml).expect("watchbill.yml could not be written");
    (place, path)
}
