//! A runner's hooks in a view (#160): before a session in a view starts, `wb start <role>` reads the settings of a
//! runner whose program is `claude` (the person's `~/.claude/settings.json`, and the project's `.claude/settings.json`
//! and `.claude/settings.local.json` as `HEAD` has them), names each hook command whose program the sandbox does not
//! hold, and starts the session all the same.
//!
//! The start runs with a stand-in for bubblewrap that runs the session on the host, so that these tests hold where
//! user namespaces are off. A settings file that is a FIFO, a link to `/dev/zero` or over the cap is read by a start
//! in a child: this test binary, run again under a cap of memory, and ended after a time.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use watchbill::hooks::{unheld, Sandbox};

fn script(path: &Path, body: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, format!("#!/bin/sh\n{}", body)).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// The organisation's file: `claude-code` starts `claude` of the log folder, bound writable, with `~/.claude` bound
/// read-only; `other` starts `session.sh` of the same folder.
const ORG: &str = "levels: [public, internal]
default: claude-code
runners:
  claude-code:
    tools: this machine
    model: a stand-in
    agreement: none
    up_to: internal
    start: ['{log}/claude', '{model}', '{effort}', '{prompt}']
    binds: [{writable: '{log}'}, '~/.claude']
  other:
    tools: this machine
    model: a stand-in
    agreement: none
    up_to: internal
    start: ['{log}/session.sh', '{model}', '{effort}', '{prompt}']
    binds: [{writable: '{log}'}, '~/.claude']
";

/// The issue's file of issue 54: it may change `src/brake.c`; the build reads `src/brake.h`.
const ISSUE_54: &str = "issue: '54'
from: 3f0c1d9e
requirements: []
interface: |
  none
changes: [src/brake.c]
reads:
  tester: [src/brake.h]
  build: [src/brake.h]
  review: [src/brake.h]
  fix: [src/brake.h]
existing_tests_changed: []
questions: []
";

/// A repository set up by `wb init --yes`, with the patterns stand-ins, the code committed, the accepted file of
/// issue 54 on the branch `issue-54`, and the branch `issue-99`, with no issue's file, checked out; the folder of the
/// stand-ins, the organisation's file, a home folder and the state folder.
struct Bench {
    repo: Folder,
    log: Folder,
    org: Folder,
    home: Folder,
    state: Folder,
}

fn bench() -> Bench {
    let repo = repository(Some("ssh://example.invalid/brakes/controller.git"));
    let log = folder_named("log");
    let org = folder_named("org");
    let home = folder_named("home");
    let state = folder_named("state");
    let l = log.path.to_str().unwrap();
    script(&log.path.join("post.sh"), &format!("echo \"post $1 $2\" >> {l}/calls\ncat > {l}/posted-$1-$2\nexit 0\n"));
    script(&log.path.join("comments.sh"), &format!("cat {l}/comments-$1-$2 2>/dev/null\nexit 0\n"));
    script(&log.path.join("text.sh"), "echo \"The text of issue $1.\"\nexit 0\n");
    script(&log.path.join("address.sh"), "echo \"The text of issue $1.\"\nexit 0\n");
    script(&log.path.join("bodies.sh"), "exit 0\n");
    script(&log.path.join("ctext.sh"), "echo \"The description of change $1.\"\nexit 0\n");
    script(&log.path.join("cbodies.sh"), "exit 0\n");
    script(&log.path.join("changes.sh"), "echo 174\nexit 0\n");
    script(&log.path.join("ssh.sh"), "exit 1\n");
    let session = format!("echo start >> {l}/calls\nexit $(cat {l}/session.status 2>/dev/null || echo 0)\n");
    script(&log.path.join("claude"), &session);
    script(&log.path.join("session.sh"), &session);
    git(&repo.path, &["config", "user.name", "Ada Lovelace"]);
    git(&repo.path, &["config", "user.email", "ada@example.org"]);
    git(&repo.path, &["config", "commit.gpgsign", "false"]);
    git(&repo.path, &["config", "core.sshCommand", &format!("{}/ssh.sh", l)]);
    let run = wb(&repo.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let mut yml = read(&config(&repo.path));
    for line in [
        format!("issue_comment: '{}/post.sh issue {{issue}}'", l),
        format!("issue_comments: '{}/comments.sh issue {{issue}}'", l),
        format!("change_comment: '{}/post.sh change {{change}}'", l),
        format!("change_comments: '{}/comments.sh change {{change}}'", l),
        format!("change_of_branch: '{}/changes.sh {{branch}}'", l),
        format!("issue_address: '{}/address.sh {{issue}}'", l),
        format!("issue_text: '{}/text.sh {{issue}}'", l),
        format!("issue_comment_bodies: '{}/bodies.sh {{issue}}'", l),
        format!("change_text: '{}/ctext.sh {{change}}'", l),
        format!("change_comment_bodies: '{}/cbodies.sh {{change}}'", l),
        "change_address: 'server {change}'".to_string(),
        "confidentiality: public".to_string(),
    ] {
        yml.push_str(&format!("{}\n", line));
    }
    fs::write(config(&repo.path), yml).unwrap();
    put(&repo.path, "src/brake.c", "int brake(void) { return 1; }\n");
    put(&repo.path, "src/brake.h", "int brake(void);\n");
    git(&repo.path, &["add", "--", ".watchbill", "src"]);
    git(&repo.path, &["commit", "-q", "-m", "Watchbill and the code"]);
    git(&repo.path, &["branch", "-M", "main"]);
    git(&repo.path, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(&repo.path, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"]);
    git(&repo.path, &["branch", "issue-99"]);
    git(&repo.path, &["checkout", "-q", "-b", "issue-54"]);
    put(&repo.path, ".watchbill/issues/54.yml", ISSUE_54);
    let run = wb(&repo.path, &["accept", "54", "--no-commit"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    git(&repo.path, &["add", "--", ".watchbill/issues"]);
    git(&repo.path, &["commit", "-q", "-m", "The issue's file of 54"]);
    git(&repo.path, &["checkout", "-q", "issue-99"]);
    fs::create_dir_all(home.path.join(".claude")).unwrap();
    fs::write(org.path.join("organisation.yml"), ORG.replace("{log}", l)).unwrap();
    // The stand-in for bubblewrap: it runs the session on the host, in the checkout.
    script(&log.path.join("bwrap"), "while [ $# -gt 0 ]; do\n  case \"$1\" in\n    --) shift; break ;;\n    *) shift ;;\n  esac\ndone\nexec \"$@\"\n");
    Bench { repo, log, org, home, state }
}

impl Bench {
    fn at(&self, name: &str) -> PathBuf {
        self.log.path.join(name)
    }

    fn organisation(&self) -> PathBuf {
        self.org.path.join("organisation.yml")
    }

    fn started(&self) -> bool {
        fs::read_to_string(self.at("calls")).unwrap_or_default().lines().any(|call| call == "start")
    }

    /// The person's settings, `~/.claude/settings.json`.
    fn user_settings(&self, json: &str) {
        fs::write(self.home.path.join(".claude/settings.json"), json).unwrap();
    }

    /// Commits a file on the branch checked out.
    fn commit(&self, path: &str, text: &str) {
        put(&self.repo.path, path, text);
        git(&self.repo.path, &["add", "--", path]);
        git(&self.repo.path, &["commit", "-q", "-m", &format!("Add {}", path)]);
    }

    fn start(&self, args: &[&str]) -> Output {
        use std::os::unix::process::ExitStatusExt;
        let args: Vec<std::ffi::OsString> = args.iter().map(std::ffi::OsString::from).collect();
        let organisation = self.organisation();
        let sandbox = self.at("bwrap");
        let machine = watchbill::Machine {
            organisation: &organisation,
            terminal: true,
            state: Some(&self.state.path),
            sandbox: &sandbox,
            home: Some(&self.home.path),
        };
        let mut out = Vec::new();
        let mut err = Vec::new();
        let status = watchbill::start_on(&args, &self.repo.path, &machine, &mut out, &mut err);
        Output { status: std::process::ExitStatus::from_raw(status << 8), stdout: out, stderr: err }
    }
}

fn said(run: &Output) -> String {
    String::from_utf8_lossy(&run.stderr).to_string()
}

/// A settings file with one command hook on each of `events`.
fn settings(command: &str, events: &[&str]) -> String {
    let each: Vec<String> = events
        .iter()
        .map(|event| format!("    \"{}\": [{{\"matcher\": \"\", \"hooks\": [{{\"type\": \"command\", \"command\": \"{}\", \"timeout\": 30}}]}}]", event, command.replace('\\', "\\\\").replace('"', "\\\"")))
        .collect();
    format!("{{\n  \"model\": \"opus\",\n  \"hooks\": {{\n{}\n  }}\n}}\n", each.join(",\n"))
}

const FAILS: &str = "will fail in the view";
const ALL_THE_SAME: &str = "the session starts all the same";

// ---- WB-VIEW-135: a hook whose program the sandbox does not hold is named before the session, which starts ----

#[test]
fn test_WB_VIEW_135_a_hook_whose_program_the_sandbox_does_not_hold_is_named_before_the_session_and_it_starts() {
    let bench = bench();
    let tools = folder_named("tools");
    let recorder = tools.path.join("recorder");
    script(&recorder, "exit 0\n");
    let command = format!("{} hook", recorder.display());
    bench.user_settings(&settings(&command, &["PreToolUse", "Stop"]));

    let run = bench.start(&["start", "build", "99"]);

    assert_eq!(code(&run), Some(0), "{}", said(&run));
    assert!(bench.started());
    let err = said(&run);
    let line = err.lines().find(|line| line.contains(FAILS)).unwrap_or_else(|| panic!("no hook named: {}", err));
    assert!(line.starts_with("wb start build: "), "{}", line);
    assert!(line.contains("~/.claude/settings.json") && line.contains("PreToolUse, Stop") && line.contains(&command), "{}", line);
    assert!(line.contains(&recorder.display().to_string()), "{}", line);
    let after = err.lines().find(|line| line.contains(ALL_THE_SAME)).unwrap_or_else(|| panic!("{}", err));
    assert!(after.contains("binds") && after.contains("'claude-code'") && after.contains(&bench.organisation().display().to_string()), "{}", after);
    let named = err.find(FAILS).unwrap();
    let starts = err.find("starts build on the runner").unwrap_or_else(|| panic!("{}", err));
    assert!(named < starts && err.find(ALL_THE_SAME).unwrap() < starts, "{}", err);
}

#[test]
fn test_WB_VIEW_135_a_hook_whose_program_the_sandbox_holds_is_not_named() {
    let bench = bench();
    let held = format!("{}/claude", bench.log.path.display());
    let json = format!(
        "{{\"hooks\": {{\"PreToolUse\": [{{\"hooks\": [{{\"type\": \"command\", \"command\": \"/usr/bin/true\"}}, {{\"type\": \"command\", \"command\": \"sh -c 'exit 0'\"}}, {{\"type\": \"command\", \"command\": \"{} --version\"}}, {{\"type\": \"command\", \"command\": \"exit 0\"}}]}}]}}}}",
        held
    );
    bench.user_settings(&json);
    bench.commit(".claude/hooks/ok.sh", "#!/bin/sh\nexit 0\n");
    fs::set_permissions(bench.repo.path.join(".claude/hooks/ok.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    git(&bench.repo.path, &["add", "--chmod=+x", "--", ".claude/hooks/ok.sh"]);
    git(&bench.repo.path, &["commit", "-q", "-m", "Executable"]);
    bench.commit(".claude/settings.json", &settings("\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/ok.sh", &["Stop"]));

    let run = bench.start(&["start", "build", "99"]);

    assert_eq!(code(&run), Some(0), "{}", said(&run));
    assert!(bench.started());
    assert!(!said(&run).contains(FAILS) && !said(&run).contains(ALL_THE_SAME), "{}", said(&run));
}

#[test]
fn test_WB_VIEW_135_the_project_s_settings_and_local_settings_of_head_are_read() {
    let bench = bench();
    bench.commit(".claude/settings.json", &settings("/nonexistent-wb-test/recorder hook", &["SessionStart"]));
    bench.commit(".claude/settings.local.json", &settings("/nonexistent-wb-test/local hook", &["SessionEnd"]));
    // The working tree is not read: a hook written there and not committed is not named.
    fs::write(bench.repo.path.join(".claude/settings.json"), settings("/nonexistent-wb-test/uncommitted", &["Stop"])).unwrap();
    git(&bench.repo.path, &["update-index", "--assume-unchanged", ".claude/settings.json"]);

    let run = bench.start(&["start", "build", "99", "--ignore-local"]);

    assert_eq!(code(&run), Some(0), "{}", said(&run));
    let err = said(&run);
    let lines: Vec<&str> = err.lines().filter(|line| line.contains(FAILS)).collect();
    assert_eq!(lines.len(), 2, "{}", err);
    assert!(lines[0].contains(".claude/settings.json") && lines[0].contains("SessionStart") && lines[0].contains("/nonexistent-wb-test/recorder hook"), "{}", err);
    assert!(lines[1].contains(".claude/settings.local.json") && lines[1].contains("SessionEnd"), "{}", err);
    assert!(!err.contains("uncommitted"), "{}", err);
}

#[test]
fn test_WB_VIEW_135_a_runner_whose_program_is_not_claude_has_no_settings_read() {
    let bench = bench();
    bench.user_settings(&settings("/nonexistent-wb-test/recorder hook", &["Stop"]));
    bench.commit(".claude/settings.json", "{\"hooks\": [");

    let run = bench.start(&["start", "build", "99", "--runner", "other"]);

    assert_eq!(code(&run), Some(0), "{}", said(&run));
    assert!(bench.started());
    let err = said(&run);
    assert!(!err.contains(FAILS) && !err.contains(ALL_THE_SAME) && !err.contains("settings.json"), "{}", err);
}

// ---- WB-VIEW-136: what a program the sandbox holds is ----

/// Folders for the reading of a command: one bound, one free, the checkout and its view, and a home folder.
struct Place {
    _all: Folder,
    bound: PathBuf,
    free: PathBuf,
    checkout: PathBuf,
    view: PathBuf,
    home: PathBuf,
}

fn place() -> Place {
    let all = folder_named("place");
    let root = fs::canonicalize(&all.path).unwrap();
    let made = |name: &str| {
        let path = root.join(name);
        fs::create_dir_all(&path).unwrap();
        path
    };
    let (bound, free, checkout, view, home) = (made("bound"), made("free"), made("checkout"), made("view"), made("home"));
    Place { bound, free, checkout, view, home, _all: all }
}

impl Place {
    fn unheld(&self, command: &str, binds: &[PathBuf], path: &str) -> Option<String> {
        let binds: Vec<(PathBuf, bool)> = binds.iter().map(|bind| (bind.clone(), false)).collect();
        let path = std::ffi::OsString::from(path);
        let sandbox = Sandbox { checkout: &self.checkout, view: &self.view, home: Some(&self.home), binds: &binds, path: Some(&path) };
        unheld(command, &sandbox)
    }
}

#[test]
fn test_WB_VIEW_136_a_bare_word_is_looked_for_on_the_folders_of_path_as_the_sandbox_has_them() {
    let p = place();
    script(&p.free.join("rec"), "exit 0\n");
    script(&p.free.join("both"), "exit 0\n");
    script(&p.bound.join("both"), "exit 0\n");
    let path = format!("relative:{}:{}:/usr/bin", p.free.display(), p.bound.display());
    let binds = [p.bound.clone()];

    let why = p.unheld("rec hook --event", &binds, &path).expect("a program in a folder the sandbox does not hold is named");
    assert!(why.contains(&p.free.join("rec").display().to_string()), "{}", why);
    assert_eq!(p.unheld("both hook", &binds, &path), None, "the sandbox finds it in the bound folder, after the free one");
    assert_eq!(p.unheld("sh -c 'exit 0'", &binds, &path), None);
    let why = p.unheld("wb-test-no-such-program", &binds, &path).expect("a program on no folder is named");
    assert!(why.contains("wb-test-no-such-program") && why.contains("on no folder of PATH"), "{}", why);
    // A relative folder of `PATH` is not looked in.
    script(&p.checkout.join("relative/rel"), "exit 0\n");
    script(&p.view.join("relative/rel"), "exit 0\n");
    assert!(p.unheld("rel", &binds, &path).is_some());
}

#[test]
fn test_WB_VIEW_136_a_path_is_held_under_the_system_or_a_bind_as_written_and_as_resolved() {
    let p = place();
    script(&p.free.join("rec"), "exit 0\n");
    script(&p.bound.join("rec"), "exit 0\n");
    fs::write(p.bound.join("plain"), "not a program\n").unwrap();
    std::os::unix::fs::symlink(p.free.join("rec"), p.bound.join("link")).unwrap();
    script(&p.home.join("bin/x"), "exit 0\n");
    let binds = [p.bound.clone()];
    let path = "/usr/bin";

    assert_eq!(p.unheld("/usr/bin/env -S x", &binds, path), None);
    assert_eq!(p.unheld(&format!("{}/rec --flag", p.bound.display()), &binds, path), None);
    assert_eq!(p.unheld(&format!("'{}/rec' --flag", p.bound.display()), &binds, path), None);
    let free = p.free.join("rec").display().to_string();
    let why = p.unheld(&format!("{} hook", free), &binds, path).unwrap();
    assert!(why.contains(&free) && why.contains("lies outside what the sandbox holds"), "{}", why);
    let why = p.unheld(&format!("{}/missing", p.bound.display()), &binds, path).unwrap();
    assert!(why.contains("is not there"), "{}", why);
    let why = p.unheld(&format!("{}/plain", p.bound.display()), &binds, path).unwrap();
    assert!(why.contains("is not an executable file"), "{}", why);
    let why = p.unheld(&format!("{}/link", p.bound.display()), &binds, path).unwrap();
    assert!(why.contains(&free), "a link out of a bind is named by where it leads: {}", why);
    assert_eq!(p.unheld("~/bin/x", &[p.home.join("bin")], path), None);
    assert!(p.unheld("~/bin/x", &binds, path).is_some());
}

#[test]
fn test_WB_VIEW_136_a_path_in_the_checkout_is_held_only_where_the_view_holds_it() {
    let p = place();
    script(&p.view.join("hooks/ok.sh"), "exit 0\n");
    script(&p.checkout.join("hooks/gone.sh"), "exit 0\n");
    fs::create_dir_all(p.view.join("hooks")).unwrap();
    fs::write(p.view.join("hooks/plain.sh"), "exit 0\n").unwrap();
    let binds = [p.bound.clone()];

    for command in ["$CLAUDE_PROJECT_DIR/hooks/ok.sh", "\"${CLAUDE_PROJECT_DIR}\"/hooks/ok.sh arg", "\"$CLAUDE_PROJECT_DIR/hooks/ok.sh\"", "./hooks/ok.sh", "hooks/ok.sh"] {
        assert_eq!(p.unheld(command, &binds, "/usr/bin"), None, "{}", command);
    }
    let why = p.unheld("$CLAUDE_PROJECT_DIR/hooks/gone.sh", &binds, "/usr/bin").unwrap();
    assert!(why.contains("hooks/gone.sh") && why.contains("is in the checkout"), "{}", why);
    assert!(p.unheld("$CLAUDE_PROJECT_DIR/hooks/plain.sh", &binds, "/usr/bin").is_some());
    let checkout = format!("{}/hooks/ok.sh", p.checkout.display());
    assert_eq!(p.unheld(&checkout, &binds, "/usr/bin"), None, "an absolute path in the checkout is looked for in the view");
}

#[test]
fn test_WB_VIEW_136_a_builtin_is_held_and_a_word_that_cannot_be_told_is_named() {
    let p = place();
    let binds = [p.bound.clone()];
    for command in ["exit 0", "echo hi >&2", ":", "true", "  printf '%s' x", "test -f x"] {
        assert_eq!(p.unheld(command, &binds, "/usr/bin"), None, "{}", command);
    }
    for command in ["$HOME/bin/rec", "`which rec`", "$(which rec)", "A=1 rec", "\"unclosed", "'unclosed", "exec rec", "eval rec", "source x", ". x", "command rec", "(rec)", "{ rec; }", "", "   ", "~other/rec", "re*c", "\\rec"] {
        let why = p.unheld(command, &binds, "/usr/bin").unwrap_or_else(|| panic!("{:?} was held", command));
        assert!(why.contains("cannot tell its program"), "{:?}: {}", command, why);
    }
}

// ---- WB-VIEW-137: the settings are read bounded, never through a link ----

const CHILD: &str = "WB_HOOKS_CHILD";
const CHILD_OUT: &str = "WB_HOOKS_OUT";

/// Not a test of its own: run again by `test_WB_VIEW_137_a_fifo_…` in a child, it starts a session with the person's
/// settings as the case of `WB_HOOKS_CHILD` makes them, and writes what the start said, its status and its time.
#[test]
fn a_helper_that_starts_on_hostile_settings_and_checks_nothing() {
    let (Some(case), Some(out)) = (std::env::var_os(CHILD), std::env::var_os(CHILD_OUT)) else { return };
    let out = PathBuf::from(out);
    let bench = bench();
    let settings = bench.home.path.join(".claude/settings.json");
    match case.to_str().unwrap() {
        "fifo" => assert!(Command::new("mkfifo").arg(&settings).status().unwrap().success()),
        "zero" => std::os::unix::fs::symlink("/dev/zero", &settings).unwrap(),
        "big" => fs::write(&settings, format!("{{\"model\": \"{}\"}}", "x".repeat(2 * 1024 * 1024))).unwrap(),
        other => panic!("no case {}", other),
    }
    let began = std::time::Instant::now();
    let run = bench.start(&["start", "build", "99"]);
    let seconds = began.elapsed().as_secs();
    fs::write(out.join("err"), &run.stderr).unwrap();
    fs::write(out.join("code"), format!("{}", code(&run).unwrap_or(-1))).unwrap();
    fs::write(out.join("seconds"), format!("{}", seconds)).unwrap();
    fs::write(out.join("started"), format!("{}", bench.started())).unwrap();
}

#[test]
fn test_WB_VIEW_137_a_fifo_a_link_and_a_file_over_the_cap_are_named_as_not_read_within_seconds() {
    for (case, why) in [("fifo", "not a plain file"), ("zero", "not a plain file"), ("big", "over 1 MiB")] {
        let out = folder_named(format!("child-{}", case));
        let mut child = Command::new("prlimit")
            .arg("--as=2147483648")
            .arg("--")
            .arg(std::env::current_exe().unwrap())
            .args(["--exact", "a_helper_that_starts_on_hostile_settings_and_checks_nothing", "--test-threads=1"])
            .env(CHILD, case)
            .env(CHILD_OUT, &out.path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("the test binary could not be started again");
        let began = std::time::Instant::now();
        let ended = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break Some(status);
            }
            if began.elapsed() > std::time::Duration::from_secs(60) {
                // Only the child this test started is ended.
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        };
        let status = ended.unwrap_or_else(|| panic!("{}: the start did not end within 60 seconds", case));
        assert!(status.success(), "{}: the child ended with {}", case, status);
        let err = read(&out.path.join("err"));
        assert_eq!(read(&out.path.join("code")), "0", "{}: {}", case, err);
        assert_eq!(read(&out.path.join("started")), "true", "{}: {}", case, err);
        let seconds: u64 = read(&out.path.join("seconds")).parse().unwrap();
        assert!(seconds < 20, "{}: the start took {} seconds", case, seconds);
        let line = err.lines().find(|line| line.contains("~/.claude/settings.json was not read")).unwrap_or_else(|| panic!("{}: {}", case, err));
        assert!(line.contains(why) && line.contains("its hooks are not looked at"), "{}: {}", case, line);
    }
}

#[test]
fn test_WB_VIEW_137_settings_the_loader_refuses_are_named_as_not_read_and_the_session_starts() {
    for json in ["{\"hooks\": {}, \"hooks\": {}}", "{\n\t\"hooks\": {}\n}", "{\"hooks\": [", "\u{0}\u{1}"] {
        let bench = bench();
        bench.user_settings(json);

        let run = bench.start(&["start", "build", "99"]);

        assert_eq!(code(&run), Some(0), "{:?}: {}", json, said(&run));
        assert!(bench.started());
        let err = said(&run);
        assert!(err.lines().any(|line| line.contains("~/.claude/settings.json was not read") && line.contains("its hooks are not looked at")), "{:?}: {}", json, err);
    }
}

#[test]
fn test_WB_VIEW_137_settings_that_are_not_there_say_nothing() {
    let bench = bench();
    fs::remove_dir(bench.home.path.join(".claude")).unwrap();

    let run = bench.start(&["start", "build", "99"]);

    assert_eq!(code(&run), Some(0), "{}", said(&run));
    assert!(!said(&run).contains("settings"), "{}", said(&run));
}

#[test]
fn test_WB_VIEW_137_a_link_or_a_file_over_the_cap_in_head_is_named_as_not_read() {
    let bench = bench();
    fs::create_dir_all(bench.repo.path.join(".claude")).unwrap();
    std::os::unix::fs::symlink("/dev/zero", bench.repo.path.join(".claude/settings.json")).unwrap();
    git(&bench.repo.path, &["add", "--", ".claude/settings.json"]);
    git(&bench.repo.path, &["commit", "-q", "-m", "A link"]);
    bench.commit(".claude/settings.local.json", &format!("{{\"model\": \"{}\"}}", "x".repeat(2 * 1024 * 1024)));

    let run = bench.start(&["start", "build", "99"]);

    assert_eq!(code(&run), Some(0), "{}", said(&run));
    let err = said(&run);
    let link = err.lines().find(|line| line.contains(".claude/settings.json of HEAD was not read")).unwrap_or_else(|| panic!("{}", err));
    assert!(link.contains("not a plain file"), "{}", link);
    let big = err.lines().find(|line| line.contains(".claude/settings.local.json of HEAD was not read")).unwrap_or_else(|| panic!("{}", err));
    assert!(big.contains("over 1 MiB"), "{}", big);
}

// ---- WB-VIEW-138: settings the sandbox does not show ----

#[test]
fn test_WB_VIEW_138_the_person_s_settings_that_no_bind_holds_are_said_not_to_run() {
    let bench = bench();
    let org = read(&bench.organisation()).replace(", '~/.claude']", "]");
    fs::write(bench.organisation(), org).unwrap();
    bench.user_settings(&settings("/usr/bin/true", &["PreToolUse", "Stop"]));

    let run = bench.start(&["start", "build", "99"]);

    assert_eq!(code(&run), Some(0), "{}", said(&run));
    let err = said(&run);
    let line = err.lines().find(|line| line.contains("~/.claude/settings.json is not in the runner's `binds`")).unwrap_or_else(|| panic!("{}", err));
    assert!(line.contains("none of its 2 hooks runs in the view"), "{}", line);
}

#[test]
fn test_WB_VIEW_138_the_project_s_settings_of_head_that_the_view_does_not_hold_are_said_not_to_run() {
    let bench = bench();
    git(&bench.repo.path, &["checkout", "-q", "issue-54"]);
    bench.commit(".claude/settings.json", &settings("/usr/bin/true", &["Stop"]));

    let run = bench.start(&["start", "build", "54"]);

    assert_eq!(code(&run), Some(0), "{}", said(&run));
    let err = said(&run);
    let line = err.lines().find(|line| line.contains(".claude/settings.json of HEAD is not in the view")).unwrap_or_else(|| panic!("{}", err));
    assert!(line.contains("none of its 1 hooks runs in the view"), "{}", line);
}

// ---- WB-VIEW-139: the check reads only ----

/// Each path below `folder`, with its size and the time it was changed.
fn listing(folder: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut left = vec![folder.to_path_buf()];
    while let Some(at) = left.pop() {
        for entry in fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            let meta = fs::symlink_metadata(&path).unwrap();
            if meta.is_dir() {
                left.push(path.clone());
            }
            found.push(format!("{} {} {:?}", path.display(), meta.len(), meta.modified().unwrap()));
        }
    }
    found.sort();
    found
}

#[test]
fn test_WB_VIEW_139_the_check_writes_nothing_and_the_status_is_the_session_s() {
    let bench = bench();
    bench.user_settings(&settings("/nonexistent-wb-test/recorder hook", &["Stop"]));
    bench.commit(".claude/settings.json", &settings("/nonexistent-wb-test/project hook", &["Stop"]));
    let home = listing(&bench.home.path);
    let head = Command::new("git").arg("-C").arg(&bench.repo.path).args(["rev-parse", "HEAD"]).output().unwrap().stdout;
    let before = Command::new("git").arg("-C").arg(&bench.repo.path).args(["status", "--porcelain", "--ignored"]).output().unwrap().stdout;
    fs::write(bench.at("session.status"), "3").unwrap();

    let run = bench.start(&["start", "build", "99"]);

    assert_eq!(code(&run), Some(1), "{}", said(&run));
    assert!(said(&run).contains(FAILS) && said(&run).contains("the session did not end with 0"), "{}", said(&run));
    assert_eq!(listing(&bench.home.path), home);
    let status = Command::new("git").arg("-C").arg(&bench.repo.path).args(["status", "--porcelain", "--ignored"]).output().unwrap().stdout;
    assert_eq!(text(&status), text(&before));
    assert_eq!(Command::new("git").arg("-C").arg(&bench.repo.path).args(["rev-parse", "HEAD"]).output().unwrap().stdout, head);
}

// ---- WB-VIEW-140: what is said ----

#[test]
fn test_WB_VIEW_140_a_command_is_named_once_for_a_file_with_its_events_in_order() {
    let bench = bench();
    bench.user_settings(&format!(
        "{{\"hooks\": {{\"Stop\": [{{\"hooks\": [{{\"type\": \"command\", \"command\": \"/nonexistent-wb-test/a\"}}]}}], \"PreToolUse\": [{{\"matcher\": \"Bash\", \"hooks\": [{{\"type\": \"command\", \"command\": \"/nonexistent-wb-test/a\"}}, {{\"type\": \"command\", \"command\": \"/nonexistent-wb-test/b\"}}]}}, {{\"matcher\": \"Edit\", \"hooks\": [{{\"type\": \"command\", \"command\": \"/nonexistent-wb-test/a\"}}]}}]}}}}"
    ));

    let run = bench.start(&["start", "build", "99"]);

    assert_eq!(code(&run), Some(0), "{}", said(&run));
    let err = said(&run);
    let lines: Vec<&str> = err.lines().filter(|line| line.contains(FAILS)).collect();
    assert_eq!(lines.len(), 2, "{}", err);
    assert!(lines[0].contains("`/nonexistent-wb-test/a`") && lines[0].contains("(Stop, PreToolUse)"), "{}", err);
    assert!(lines[1].contains("`/nonexistent-wb-test/b`") && lines[1].contains("(PreToolUse)"), "{}", err);
    assert_eq!(err.matches(ALL_THE_SAME).count(), 1, "{}", err);
}

#[test]
fn test_WB_VIEW_140_more_than_20_commands_are_said_as_20_and_a_count_and_a_long_command_is_cut() {
    let bench = bench();
    let mut hooks: Vec<String> = (0..25).map(|n| format!("{{\"type\": \"command\", \"command\": \"/nonexistent-wb-test/h{}\"}}", n)).collect();
    hooks[0] = format!("{{\"type\": \"command\", \"command\": \"/nonexistent-wb-test/long\\u001b[31m {}\"}}", "y".repeat(400));
    bench.user_settings(&format!("{{\"hooks\": {{\"Stop\": [{{\"hooks\": [{}]}}]}}}}", hooks.join(", ")));

    let run = bench.start(&["start", "build", "99"]);

    assert_eq!(code(&run), Some(0), "{}", said(&run));
    let err = said(&run);
    let lines: Vec<&str> = err.lines().filter(|line| line.contains(FAILS)).collect();
    assert_eq!(lines.len(), 20, "{}", err);
    assert!(err.lines().any(|line| line.ends_with("and 5 more hooks that will fail in the view")), "{}", err);
    assert!(!err.contains('\u{1b}') && lines[0].contains("\\u{1b}"), "{}", lines[0]);
    assert!(lines[0].contains(&format!("{}…`", "y".repeat(10))) && !lines[0].contains(&"y".repeat(200)), "{}", lines[0]);
}

#[test]
fn test_WB_VIEW_140_hooks_not_in_the_form_read_are_said_so_and_other_types_are_not_looked_at() {
    let bench = bench();
    bench.user_settings("{\"hooks\": [\"/nonexistent-wb-test/a\"]}");
    bench.commit(".claude/settings.json", "{\"hooks\": {\"Stop\": [{\"hooks\": [{\"type\": \"prompt\", \"prompt\": \"Is it done?\"}]}]}}");

    let run = bench.start(&["start", "build", "99"]);

    assert_eq!(code(&run), Some(0), "{}", said(&run));
    let err = said(&run);
    assert!(err.lines().any(|line| line.contains("the hooks of ~/.claude/settings.json are not in the form wb reads")), "{}", err);
    assert!(!err.contains(FAILS), "{}", err);
}

// ---- WB-VIEW-141: the README ----

#[test]
fn test_WB_VIEW_141_the_readme_says_how_a_runner_s_hooks_work_in_a_view() {
    let readme = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"));
    let start = readme.find("### A runner's hooks").expect("the README has no section on a runner's hooks");
    let section = &readme[start..];
    let section = &section[..section[4..].find("\n#").map(|end| end + 4).unwrap_or(section.len())];
    for words in ["`binds`", "read-only", "writable", "the session's claim", "cannot be bound", "outside the checkout", "the session starts all the same", "network"] {
        assert!(section.contains(words), "the section does not say {:?}:\n{}", words, section);
    }
    let lower = readme.to_lowercase();
    for name in ["howl", "handover ledger", "handover-ledger"] {
        assert!(!lower.contains(name), "the README names {}", name);
    }
}

// ---- WB-VIEW-142: a program in the checkout ----

#[test]
fn test_WB_VIEW_142_a_program_in_the_checkout_that_the_view_does_not_hold_is_said_to_be_installed_outside() {
    let bench = bench();
    git(&bench.repo.path, &["checkout", "-q", "issue-54"]);
    bench.commit("tools/rec.sh", "#!/bin/sh\nexit 0\n");
    git(&bench.repo.path, &["add", "--chmod=+x", "--", "tools/rec.sh"]);
    git(&bench.repo.path, &["commit", "-q", "-m", "Executable"]);
    bench.user_settings(&settings("$CLAUDE_PROJECT_DIR/tools/rec.sh hook", &["Stop"]));

    let run = bench.start(&["start", "build", "54"]);

    assert_eq!(code(&run), Some(0), "{}", said(&run));
    let err = said(&run);
    let line = err.lines().find(|line| line.contains(FAILS)).unwrap_or_else(|| panic!("{}", err));
    assert!(line.contains("tools/rec.sh") && line.contains("is in the checkout") && line.contains("installed outside the checkout"), "{}", line);
}
