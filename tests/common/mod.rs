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
/// its own, given for it alone, so that a test of a missing identity is not spoilt. Since #323 a repository set up
/// with no `.watchbill/signers` refuses every role and every acceptance, so the repository also signs with Ada's key
/// of the test and has her line in the list (`signing`), where the default branch holds none.
pub fn commit_config(folder: &Path) {
    commit_config_alone(folder);
    signing(folder);
}

/// As `commit_config`, with no list and no signing: `.watchbill/watchbill.yml` alone.
pub fn commit_config_alone(folder: &Path) {
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
/// alone in one commit, so that a role may be handed out for the issue (#97). Gives git an identity. Since #323
/// nothing that needs a signature counts without `.watchbill/signers`: the repository signs with Ada's key of the
/// test (`signing`), and each acceptance is the signed tag that `wb plan accept` makes.
pub fn issues_ready(folder: &Path, issues: &[&str]) {
    git(folder, &["config", "user.name", "Ada Lovelace"]);
    git(folder, &["config", "user.email", "ada@example.org"]);
    git(folder, &["config", "commit.gpgsign", "false"]);
    commit_config(folder);
    signing(folder);
    let mut added = Vec::new();
    for issue in issues {
        put(folder, &format!(".watchbill/issues/{}.yml", issue), &BARE_ISSUE.replace("{issue}", issue));
        let run = wb(folder, &["plan", "accept", "--no-commit", "--", issue], "");
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

/// A stand-in for a running `wb`: the program at `source` copied to `program`, written, closed and synced, then
/// started with `args` and no input. Where another thread of the test has forked while the file was open for
/// writing, the start fails with `ETXTBSY` ("text file busy"); it is tried again, a short wait between.
pub fn stand_in(source: &str, program: &Path, args: &[&str]) -> std::process::Child {
    fs::copy(source, program).expect("the stand-in could not be copied");
    fs::File::open(program).expect("the stand-in could not be opened").sync_all().expect("the stand-in could not be synced");
    for attempt in 0..100 {
        match Command::new(program).args(args).stdin(Stdio::null()).spawn() {
            Ok(child) => return child,
            Err(error) if error.raw_os_error() == Some(26) && attempt < 99 => std::thread::sleep(std::time::Duration::from_millis(50)),
            Err(error) => panic!("the stand-in for wb could not be started: {}", error),
        }
    }
    unreachable!()
}

/// Writes each of `messages` as a commit on the job's ref of `issue`, `refs/watchbill/<issue>`, after the one before,
/// with the tree of the commit before (empty for the first), signed with the key git signs with: a stamp or a marker as
/// `wb start` and `wb revert` write one (#236). Since #323 an unsigned one counts for nothing, so the repository signs
/// with Ada's key of the test, and has her line in `.watchbill/signers` where it has no list (`signing`). A message is a
/// first line such as `[stamp] review round 1` and any lines after it. The ref is moved with no check of what it was.
pub fn stamps_on_ref(folder: &Path, issue: &str, messages: &[&str]) {
    signing(folder);
    commits_on_ref(folder, issue, messages, true);
}

/// As `stamps_on_ref`, and each commit unsigned: a stamp that counts for nothing (WB-BUILD-48, WB-TASK-58).
pub fn unsigned_stamps_on_ref(folder: &Path, issue: &str, messages: &[&str]) {
    commits_on_ref(folder, issue, messages, false);
}

fn commits_on_ref(folder: &Path, issue: &str, messages: &[&str], signed: bool) {
    let full = format!("refs/watchbill/{}", issue);
    let run = |args: &[&str], input: &[u8]| -> String {
        let mut child = Command::new("git")
            .arg("-C")
            .arg(folder)
            .args(args)
            .env_clear()
            .envs(env())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("git could not be started");
        child.stdin.take().unwrap().write_all(input).unwrap();
        let output = child.wait_with_output().expect("git could not be waited for");
        assert!(output.status.success(), "git {:?}: {}", args, String::from_utf8_lossy(&output.stderr));
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };
    let parent = Command::new("git").arg("-C").arg(folder).args(["rev-parse", "--verify", "-q", &full]).env_clear().envs(env()).stderr(Stdio::null()).output().expect("git could not be started");
    let mut parent = Some(String::from_utf8_lossy(&parent.stdout).trim().to_string()).filter(|commit| !commit.is_empty());
    for message in messages {
        let tree = match &parent {
            Some(commit) => run(&["rev-parse", &format!("{}^{{tree}}", commit)], b""),
            None => run(&["mktree"], b""),
        };
        let sign = if signed { "-S" } else { "--no-gpg-sign" };
        let mut args = vec!["-c", "user.name=Ada Lovelace", "-c", "user.email=ada@example.org", "commit-tree", sign, tree.as_str()];
        if let Some(commit) = &parent {
            args.push("-p");
            args.push(commit);
        }
        let mut body = if message.ends_with('\n') { message.to_string() } else { format!("{}\n", message) };
        // Every stamp and marker names its issue, as `wb` writes it: a commit counts only on the ref of its issue.
        let named = format!("issue: {}", issue);
        if !body.lines().any(|line| line == named) {
            let (first, rest) = body.split_once('\n').unwrap_or((body.as_str(), ""));
            body = format!("{}\n{}\n{}", first, named, rest);
        }
        let commit = run(&args, body.as_bytes());
        run(&["update-ref", &full, &commit], b"");
        parent = Some(commit);
    }
}

/// Sends the pushes of `origin` to a bare repository beside the checkout `repo`, in the folder of the test, while its
/// fetch address stays as it is: `wb start` pushes the stamp to `origin` before the session (#236), and no push of a
/// test reaches the network. The bare repository's path.
pub fn push_beside(repo: &Path) -> PathBuf {
    let bare = repo.parent().expect("the checkout is in a folder of the test").join("origin.git");
    git(repo, &["init", "-q", "--bare", bare.to_str().unwrap()]);
    git(repo, &["config", "remote.origin.pushurl", bare.to_str().unwrap()]);
    bare
}

/// The stamps and the markers among `lines`, first lines of comments as a test of an earlier revision gave them, put
/// where `wb` reads them since #236: as the commits on the job's ref of `issue`, in their order, in place of what the ref
/// held. A line of another kind is left to the comments. `bodies` gives a whole message for a first line where it has
/// one (a planner's `time:`); a line with none is given a time long past, as a stamp whose time is not read stood.
pub fn stamps_from_lines(folder: &Path, issue: &str, lines: &[&str], bodies: &[&str]) {
    let full = format!("refs/watchbill/{}", issue);
    // The stamps and markers on the ref go; a plan's commits under them stay.
    let read = |args: &[&str]| {
        let run = Command::new("git").arg("-C").arg(folder).args(args).env_clear().envs(env()).stderr(Stdio::null()).output().expect("git could not be started");
        Some(String::from_utf8_lossy(&run.stdout).trim().to_string()).filter(|out| run.status.success() && !out.is_empty())
    };
    while let Some(tip) = read(&["rev-parse", "--verify", "-q", &full]) {
        let first = read(&["log", "-1", "--format=%s", &tip]).unwrap_or_default();
        if !first.starts_with("[stamp] ") && !first.starts_with("[revert] ") {
            break;
        }
        match read(&["rev-parse", "--verify", "-q", &format!("{}^", tip)]) {
            Some(parent) => git(folder, &["update-ref", &full, &parent]),
            None => git(folder, &["update-ref", "-d", &full]),
        }
    }
    let is_entry = |line: &str| line.starts_with("[stamp] ") || line.starts_with("[revert] ");
    let messages: Vec<String> = lines
        .iter()
        .filter(|line| is_entry(line))
        .map(|line| bodies.iter().find(|body| body.split('\n').next() == Some(*line)).map_or_else(|| format!("{}\ntime: 2000-01-01T00:00:00Z\n", line), |body| body.to_string()))
        .collect();
    let messages: Vec<&str> = messages.iter().map(String::as_str).collect();
    stamps_on_ref(folder, issue, &messages);
}

/// The output of git in `folder` with `envs` beside those of `env`, and `input` on standard input.
fn git_with(folder: &Path, args: &[&str], envs: &[(&str, &str)], input: &[u8]) -> Output {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(args)
        .env_clear()
        .envs(env())
        .envs(envs.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("git could not be started");
    let _ = child.stdin.take().unwrap().write_all(input);
    child.wait_with_output().expect("git could not be waited for")
}

/// What git printed in `folder`, trimmed, where it ended with 0.
fn git_text(folder: &Path, args: &[&str]) -> Option<String> {
    let run = git_with(folder, args, &[], b"");
    run.status.success().then(|| String::from_utf8_lossy(&run.stdout).trim().to_string())
}

/// The folder of the keys of a test of the repository `folder`: in its `.git`, so that it goes with the repository and
/// is never committed. Beside the keys, a home of its own, which every run of `ssh-keygen` for them is given.
fn keys_folder(folder: &Path) -> PathBuf {
    let git_dir = git_text(folder, &["rev-parse", "--absolute-git-dir"]).expect("the repository has no git folder");
    let keys = PathBuf::from(git_dir).join("wb-test-keys");
    fs::create_dir_all(keys.join("home")).expect("the folder of the keys could not be made");
    keys
}

/// The private key `name` of a test of the repository `folder`, made the first time it is asked for, by an
/// `ssh-keygen` with a home of its own and no ssh agent: no person's key, agent or config is read or touched.
pub fn test_key(folder: &Path, name: &str) -> PathBuf {
    let keys = keys_folder(folder);
    let path = keys.join(name);
    if !path.exists() {
        let home = keys.join("home");
        let run = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-C", name, "-f"])
            .arg(&path)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", &home)
            .env("XDG_CONFIG_HOME", home.join(".config"))
            .stdin(Stdio::null())
            .output()
            .expect("ssh-keygen could not be started");
        assert!(run.status.success(), "ssh-keygen: {}", String::from_utf8_lossy(&run.stderr));
    }
    path
}

/// The public key of `test_key`, as `<type> <base64>`.
pub fn test_public(folder: &Path, name: &str) -> String {
    let key = test_key(folder, name);
    let line = read(&PathBuf::from(format!("{}.pub", key.display())));
    line.split_whitespace().take(2).collect::<Vec<_>>().join(" ")
}

/// Ada's line of `.watchbill/signers`, with the roles `accept` and `start`.
pub fn ada_line(folder: &Path) -> String {
    format!("ada@example.org roles=\"accept,start\" {}\n", test_public(folder, "ada"))
}

/// Writes `.watchbill/signers` with Ada's line in the working tree alone, as a person writes it after `wb init`:
/// `wb check` holds only where it is there (#323).
pub fn put_signers(folder: &Path) {
    put(folder, ".watchbill/signers", &ada_line(folder));
}

/// The default branch as `wb` finds it: `refs/remotes/origin/HEAD`, else `origin/main`, `main`, `origin/master`, `master`.
pub fn default_branch(folder: &Path) -> Option<String> {
    let exists = |full: &str| git_text(folder, &["rev-parse", "--verify", "-q", &format!("{}^{{commit}}", full)]).is_some();
    if let Some(target) = git_text(folder, &["symbolic-ref", "-q", "refs/remotes/origin/HEAD"]).filter(|target| !target.is_empty() && exists(target)) {
        return Some(target);
    }
    ["refs/remotes/origin/main", "refs/heads/main", "refs/remotes/origin/master", "refs/heads/master"].iter().find(|full| exists(full)).map(|full| full.to_string())
}

/// Signs with Ada's key of the test from now on, as git's config of the repository: `gpg.format` ssh, `user.signingkey`
/// her key, and `gpg.ssh.program` a wrapper that runs `ssh-keygen` with the home of the keys and no ssh agent; and,
/// as the default branch holds no `.watchbill/signers` (#323), commits one there with Ada's line alone: in the checkout
/// where that branch is checked out (or no branch is there yet), else onto the branch's ref, the checkout left as it
/// is. Where the default branch holds a list, it is the test's own, and nothing is done.
pub fn signing(folder: &Path) {
    let key = test_key(folder, "ada");
    let keys = keys_folder(folder);
    let program = keys.join("ssh-keygen");
    if !program.exists() {
        let home = keys.join("home");
        let quoted = |path: &Path| format!("'{}'", path.display().to_string().replace('\'', "'\\''"));
        let body = format!(
            "#!/bin/sh\nunset SSH_AUTH_SOCK SSH_AGENT_PID\nHOME={}\nXDG_CONFIG_HOME={}\nexport HOME XDG_CONFIG_HOME\nexec ssh-keygen \"$@\"\n",
            quoted(&home),
            quoted(&home.join(".config"))
        );
        fs::write(&program, body).expect("the wrapper of ssh-keygen could not be written");
        fs::set_permissions(&program, std::os::unix::fs::PermissionsExt::from_mode(0o755)).expect("the wrapper could not be made a program");
    }
    // The repository signs with a key of the test in any case (the review of #325, note 5), never by the person's
    // config: with Ada's key here, unless the test set a key of its own in the repository's config.
    if git_text(folder, &["config", "--local", "--get", "user.signingkey"]).is_none() {
        git(folder, &["config", "gpg.format", "ssh"]);
        git(folder, &["config", "user.signingkey", key.to_str().unwrap()]);
        git(folder, &["config", "gpg.ssh.program", program.to_str().unwrap()]);
    }
    // A list that is there is the test's own: it is not changed.
    let default = default_branch(folder);
    if let Some(default) = &default {
        if git_text(folder, &["cat-file", "-e", &format!("{}:.watchbill/signers", default)]).is_some() {
            return;
        }
    }
    let line = ada_line(folder);
    let head = git_text(folder, &["symbolic-ref", "-q", "HEAD"]);
    let identity = ["-c", "user.name=Ada Lovelace", "-c", "user.email=ada@example.org", "-c", "commit.gpgsign=false"];
    // The checkout's commit where it is that of the default branch (as after a clone): the list is committed in the
    // checkout, and the default branch moved along with it.
    let commit_of = |name: &str| git_text(folder, &["rev-parse", "--verify", "-q", &format!("{}^{{commit}}", name)]);
    let along = match (&default, &head) {
        (Some(default), Some(head)) if default != head && commit_of(default).is_some() && commit_of(default) == commit_of(head) => Some(default.clone()),
        _ => None,
    };
    if let Some(default) = along {
        let old = commit_of(&default).unwrap();
        put(folder, ".watchbill/signers", &line);
        git(folder, &["add", "--", ".watchbill/signers"]);
        let mut args: Vec<&str> = identity.to_vec();
        args.extend(["commit", "-q", "-m", "The project's keys", "--", ".watchbill/signers"]);
        git(folder, &args);
        let new = commit_of("HEAD").unwrap();
        git(folder, &["update-ref", &default, &new, &old]);
        return;
    }
    match default {
        Some(default) if head.as_deref() != Some(default.as_str()) => {
            let blob = String::from_utf8_lossy(&git_with(folder, &["hash-object", "-w", "--stdin"], &[], line.as_bytes()).stdout).trim().to_string();
            let index = keys.join("index");
            let index = index.to_str().unwrap();
            let with_index = |args: &[&str]| {
                let run = git_with(folder, args, &[("GIT_INDEX_FILE", index)], b"");
                assert!(run.status.success(), "git {:?}: {}", args, String::from_utf8_lossy(&run.stderr));
                String::from_utf8_lossy(&run.stdout).trim().to_string()
            };
            with_index(&["read-tree", &default]);
            with_index(&["update-index", "--add", "--cacheinfo", &format!("100644,{},.watchbill/signers", blob)]);
            let tree = with_index(&["write-tree"]);
            let _ = fs::remove_file(index);
            let mut args: Vec<&str> = identity.to_vec();
            args.extend(["commit-tree", "--no-gpg-sign", tree.as_str(), "-p", default.as_str(), "-m", "The project's keys"]);
            let commit = git_text(folder, &args).expect("the commit of the keys could not be made");
            git(folder, &["update-ref", &default, &commit]);
        }
        _ => {
            put(folder, ".watchbill/signers", &line);
            git(folder, &["add", "--", ".watchbill/signers"]);
            let mut args: Vec<&str> = identity.to_vec();
            args.extend(["commit", "-q", "-m", "The project's keys", "--", ".watchbill/signers"]);
            git(folder, &args);
        }
    }
}

/// The signed tag of the acceptance `.watchbill/issues/<issue>.accepted` that a test wrote by hand, as `wb plan accept`
/// makes it for an issue with no plan (#235): on the blob of the issue's file, signed with the key git signs with
/// (`signing`, which this runs first). Since #323 an acceptance with no tag counts for nothing.
pub fn sign_acceptance(folder: &Path, issue: &str) {
    signing(folder);
    let file = fs::read(folder.join(format!(".watchbill/issues/{}.yml", issue))).expect("the issue's file could not be read");
    let body = read(&folder.join(format!(".watchbill/issues/{}.accepted", issue)));
    let blob = String::from_utf8_lossy(&git_with(folder, &["hash-object", "-w", "--no-filters", "--stdin"], &[], &file).stdout).trim().to_string();
    let tag = watchbill::signers::tag_of(watchbill::signers::ACCEPT_TAGS, issue, body.as_bytes());
    let message = watchbill::signers::file_message(issue, &body, &[]);
    let short = tag.strip_prefix("refs/tags/").unwrap();
    let run = git_with(folder, &["tag", "-s", "-f", "--cleanup=verbatim", "-F", "-", short, &blob], &[], message.as_bytes());
    assert!(run.status.success(), "git tag: {}", String::from_utf8_lossy(&run.stderr));
}
