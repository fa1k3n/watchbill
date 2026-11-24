//! The watchdog (#267): a session that runs long, or has done nothing for long, is seen, and ended past a hard limit.
//!
//! The limits are in the rule table, `quiet_minutes` and `max_minutes` for each role, and `watchbill.yml` may set them
//! lower, never higher. The `wb` that waits for the session is the watchdog: past `quiet_minutes` with nothing changed
//! in the view it says so, and past `max_minutes` it ends the session as a termination of `wb` does (#221). `wb status
//! <issue>` shows a session that runs with how long it has run and how long it has been quiet.
//!
//! A minute lasts a second in these tests: `WB_TEST_MINUTE`, which only a build with debug assertions reads, is set
//! in a process of its own, the test binary run again with only the helper in it, so that no other test sees it and
//! no test sleeps through real minutes. The `bwrap` is a stand-in that runs the session in the view, so these tests
//! hold where bubblewrap cannot make a sandbox.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn script(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\n{}", body)).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// The organisation's file: `stand-in` starts `session.sh` of the log folder, bound writable.
const ORG: &str = "levels: [public, internal]
default: stand-in
runners:
  stand-in:
    tools: this machine
    model: a stand-in
    agreement: none
    up_to: internal
    start: ['{log}/session.sh', '{model}', '{effort}', '{prompt}']
    binds: [{writable: '{log}'}]
";

/// The issue's file of issue 54.
const ISSUE_54: &str = "issue: '54'
from: 3f0c1d9e
requirements: []
interface: |
  none
changes: [src/brake.c, tests/test_brake.c, src/new.c]
reads:
  tester: [src/brake.h]
  build: [src/brake.h]
  review: [src/brake.h]
  fix: [src/brake.h]
existing_tests_changed: []
questions: []
";

/// A repository set up by `wb init --yes`, with the patterns stand-ins, the code committed, and the accepted file of
/// issue 54 committed on the branch `issue-54`, which is checked out; the organisation's file, and the state folder.
struct Bench {
    repo: Folder,
    log: Folder,
    org: Folder,
    home: Folder,
    state: Folder,
}

/// The bench, with `limits` added to `watchbill.yml` and committed, where it is not empty.
fn bench_with(limits: &str) -> Bench {
    let repo = repository(Some("ssh://example.invalid/brakes/controller.git"));
    let log = folder_named("log");
    let org = folder_named("org");
    let home = folder_named("home");
    let state = folder_named("state");
    let l = log.path.to_str().unwrap();
    script(&log.path.join("post.sh"), &format!("echo \"post $1 $2\" >> {l}/calls\ncat > {l}/posted-$1-$2\nexit 0\n"));
    script(&log.path.join("comments.sh"), &format!("echo \"comments $1 $2\" >> {l}/calls\ncat {l}/comments-$1-$2 2>/dev/null\nexit 0\n"));
    script(&log.path.join("text.sh"), &format!("echo \"text $1\" >> {l}/calls\necho \"The text of issue $1.\"\nexit 0\n"));
    script(&log.path.join("address.sh"), &format!("echo \"address $1\" >> {l}/calls\necho \"The text of issue $1.\"\nexit 0\n"));
    script(&log.path.join("bodies.sh"), &format!("echo \"bodies $1\" >> {l}/calls\nexit 0\n"));
    script(&log.path.join("ctext.sh"), &format!("echo \"ctext $1\" >> {l}/calls\necho \"The description of change $1.\"\nexit 0\n"));
    script(&log.path.join("cbodies.sh"), &format!("echo \"cbodies $1\" >> {l}/calls\nexit 0\n"));
    script(&log.path.join("changes.sh"), &format!("echo \"changes $1\" >> {l}/calls\necho 174\nexit 0\n"));
    script(&log.path.join("ssh.sh"), &format!("echo ssh >> {l}/network\nexit 1\n"));
    script(&log.path.join("session.sh"), &format!("echo start >> {l}/calls\nexit 0\n"));
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
        "build_files: [build]".to_string(),
    ] {
        yml.push_str(&format!("{}\n", line));
    }
    yml.push_str(limits);
    fs::write(config(&repo.path), yml).unwrap();
    put(&repo.path, "src/brake.c", "int brake(void) { return 1; }\n");
    put(&repo.path, "src/brake.h", "int brake(void);\n");
    put(&repo.path, "tests/test_brake.c", "int main(void) { return 0; }\n");
    put(&repo.path, "build/Makefile", "all:\n");
    git(&repo.path, &["add", "--", ".watchbill", "src", "tests", "build"]);
    git(&repo.path, &["commit", "-q", "-m", "Watchbill and the code"]);
    git(&repo.path, &["branch", "-M", "main"]);
    git(&repo.path, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(&repo.path, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"]);
    git(&repo.path, &["checkout", "-q", "-b", "issue-54"]);
    put(&repo.path, ".watchbill/issues/54.yml", ISSUE_54);
    let run = wb(&repo.path, &["accept", "54", "--no-commit"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    git(&repo.path, &["add", "--", ".watchbill/issues"]);
    git(&repo.path, &["commit", "-q", "-m", "The issue's file of 54"]);
    let text = ORG.replace("{log}", l);
    fs::write(org.path.join("organisation.yml"), text).unwrap();
    Bench { repo, log, org, home, state }
}

fn bench() -> Bench {
    bench_with("")
}

impl Bench {
    fn at(&self, name: &str) -> PathBuf {
        self.log.path.join(name)
    }

    fn left(&self, name: &str) -> String {
        fs::read_to_string(self.at(name)).unwrap_or_default()
    }

    fn calls(&self) -> Vec<String> {
        self.left("calls").lines().map(str::to_string).collect()
    }

    /// The session: a script run in the view, after it notes that it started. `{log}` is the log folder, `{state}`
    /// the state folder.
    fn session(&self, body: &str) {
        let l = self.log.path.to_str().unwrap();
        let s = self.state.path.to_str().unwrap();
        script(&self.at("session.sh"), &format!("echo start >> {l}/calls\n{}", body.replace("{log}", l).replace("{state}", s)));
    }

    /// `wb start` in this process, with the stand-in `bwrap`, standard error written to `err` as it is said.
    fn start_saying(&self, args: &[&str], err: &mut dyn std::io::Write) -> i32 {
        let args: Vec<std::ffi::OsString> = args.iter().map(std::ffi::OsString::from).collect();
        let organisation = self.org.path.join("organisation.yml");
        let sandbox = stand_in_bwrap(self);
        let machine = watchbill::Machine { organisation: &organisation, terminal: true, state: Some(&self.state.path), sandbox: &sandbox, home: Some(&self.home.path) };
        let mut out = Vec::new();
        watchbill::start_on(&args, &self.repo.path, &machine, &mut out, err)
    }
}

/// A `bwrap` that makes no sandbox: it answers the probe, and else runs the words in the view.
fn stand_in_bwrap(bench: &Bench) -> PathBuf {
    let path = bench.at("bwrap");
    script(
        &path,
        "view=\nwhile [ $# -gt 0 ]; do\n  case \"$1\" in\n    --bind) view=$2; shift 3;;\n    --) shift; break;;\n    *) shift;;\n  esac\ndone\n[ \"$1\" = /usr/bin/true ] && exit 0\ncd \"$view\" && exec \"$@\"\n",
    );
    path
}

// ---- The helper: `wb start build 54` in a process of its own, with a minute of one second ----

const CHILD_FOLDER: &str = "WB_WATCHDOG_CHILD_FOLDER";
const CHILD_SESSION: &str = "WB_WATCHDOG_CHILD_SESSION";
const CHILD_LIMITS: &str = "WB_WATCHDOG_CHILD_LIMITS";

/// Not a test of a row: it does nothing unless a watchdog test started it. It makes the bench with the limits and the
/// session it is given, runs `wb start build 54`, writes what `wb` said, the folders and the status, and waits for the
/// parent to be done with the folders.
#[test]
fn a_helper_that_runs_wb_for_the_watchdog_tests_and_checks_nothing() {
    let Some(folder) = std::env::var_os(CHILD_FOLDER).map(PathBuf::from) else { return };
    let bench = bench_with(&std::env::var(CHILD_LIMITS).unwrap_or_default());
    bench.session(&std::env::var(CHILD_SESSION).unwrap_or_default());
    let paths = format!("{}\n{}\n{}\n", bench.repo.path.display(), bench.log.path.display(), bench.state.path.display());
    fs::write(folder.join("paths.tmp"), paths).unwrap();
    fs::rename(folder.join("paths.tmp"), folder.join("paths")).unwrap();
    let mut said = fs::File::create(folder.join("said")).unwrap();
    let status = bench.start_saying(&["start", "build", "54"], &mut said);
    fs::write(folder.join("status.tmp"), format!("{}\n", status)).unwrap();
    fs::rename(folder.join("status.tmp"), folder.join("status")).unwrap();
    for _ in 0..600 {
        if folder.join("release").exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// What the run of the helper left.
struct Watched {
    /// The status of `wb`, or -2 where it had not ended within the bound.
    status: i32,
    /// What `wb` said on standard error.
    said: String,
    outcome: String,
    back: String,
    /// The file `quiet` of the lines of the start.
    quiet: String,
    /// The subjects of the branch `issue-54`, newest first.
    subjects: String,
    /// How long `wb` ran.
    took: std::time::Duration,
    /// What the session left in the log folder, by name.
    log: PathBuf,
    _parent: Folder,
}

impl Watched {
    fn left(&self, name: &str) -> String {
        fs::read_to_string(self.log.join(name)).unwrap_or_default()
    }

    fn quiet_lines(&self) -> Vec<&str> {
        self.said.lines().filter(|line| line.starts_with("wb start build: quiet: ")).collect()
    }
}

/// Runs the helper with `session`, the lines of `limits` in `watchbill.yml` and `vars` (a minute of one second unless
/// they say another), under a cap of 2 GiB of memory, and gathers what it left. Nothing waits longer than `seconds`.
fn watched(session: &str, limits: &str, vars: &[(&str, &str)], seconds: u64) -> Watched {
    let parent = folder_named("watched");
    let folder = parent.path.clone();
    let mut command = Command::new("prlimit");
    command
        .arg("--as=2147483648")
        .arg("--")
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", "a_helper_that_runs_wb_for_the_watchdog_tests_and_checks_nothing", "--nocapture", "--test-threads=1"])
        .env(CHILD_FOLDER, &folder)
        .env(CHILD_SESSION, session)
        .env(CHILD_LIMITS, limits)
        .env("WB_TEST_MINUTE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    for (name, value) in vars {
        command.env(name, value);
    }
    let mut child = command.spawn().expect("the test binary could not be started");
    let mut paths = String::new();
    let end = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while paths.is_empty() && std::time::Instant::now() < end {
        paths = fs::read_to_string(folder.join("paths")).unwrap_or_default();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let began = std::time::Instant::now();
    let end = began + std::time::Duration::from_secs(seconds);
    while !folder.join("status").exists() && std::time::Instant::now() < end && child.try_wait().ok().flatten().is_none() {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let took = began.elapsed();
    let status = fs::read_to_string(folder.join("status")).ok().and_then(|text| text.trim().parse().ok()).unwrap_or(-2);
    let mut at = paths.lines().map(PathBuf::from);
    let (repo, log, state) = (at.next().unwrap_or_default(), at.next().unwrap_or_default(), at.next().unwrap_or_default());
    let one = fs::read_dir(state.join("watchbill/sessions")).ok().and_then(|found| found.map(|entry| entry.unwrap().path()).next());
    let file = |name: &str| one.as_ref().and_then(|one| fs::read_to_string(one.join(name)).ok()).unwrap_or_default();
    let (outcome, back, quiet) = (file("outcome"), file("back"), file("quiet"));
    let subjects = Command::new("git").arg("-C").arg(&repo).args(["log", "--format=%s", "issue-54"]).env_clear().envs(env()).output().map(|run| text(&run.stdout)).unwrap_or_default();
    let said = fs::read_to_string(folder.join("said")).unwrap_or_default();
    // The log folder goes with the bench when the helper ends: it is copied first.
    let copied = parent.path.join("log-copy");
    let _ = fs::create_dir_all(&copied);
    if let Ok(entries) = fs::read_dir(&log) {
        for entry in entries.flatten() {
            if entry.file_type().map(|kind| kind.is_file()).unwrap_or(false) {
                let _ = fs::copy(entry.path(), copied.join(entry.file_name()));
            }
        }
    }
    let _ = fs::write(folder.join("release"), "");
    if !wait_for(10, || child.try_wait().ok().flatten().is_some()) {
        let _ = child.kill();
        let _ = child.wait();
    }
    Watched { status, said, outcome, back, quiet, subjects, took, log: copied, _parent: parent }
}

/// Waits at most `seconds` for `ready` to be true.
fn wait_for(seconds: u64, mut ready: impl FnMut() -> bool) -> bool {
    let end = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    while std::time::Instant::now() < end {
        if ready() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    ready()
}

fn limits(quiet: u64, max: u64) -> String {
    format!("session_limits:\n  build:\n    quiet_minutes: {}\n    max_minutes: {}\n", quiet, max)
}

/// The seconds since 1970 now.
fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
}

// ---- WB-RULE-55 to 60: the limits in the rule table and in watchbill.yml ----

fn first_revision() -> watchbill::rules::Rules {
    watchbill::rules::load_revision().expect("the rule table of the package could not be read")
}

/// The rule table of the package with `from` written as `to`, read as `wb` reads a table, with the model table of
/// the package.
fn table_with(from: &str, to: &str) -> Result<watchbill::rules::Rules, String> {
    let shipped = read(Path::new(watchbill::rules::RULES_FILE));
    assert!(shipped.contains(from), "the table holds no {:?}", from);
    let folder = folder_named("table");
    let path = folder.path.join("rules.yml");
    fs::write(&path, shipped.replacen(from, to, 1)).unwrap();
    watchbill::rules::load(&path, watchbill::rules::MODELS_FILE).map_err(|why| why.to_string())
}

#[test]
fn test_WB_RULE_55_every_role_of_the_table_has_a_quiet_of_30_and_a_most_of_240_minutes() {
    let table = first_revision();
    for role in watchbill::rules::ROLES {
        assert_eq!(table.roles[role].quiet_minutes, 30, "{}", role);
        assert_eq!(table.roles[role].max_minutes, 240, "{}", role);
    }
    // They are written in the table itself, the planner's row too, not only given where none is written.
    let shipped = read(Path::new(watchbill::rules::RULES_FILE));
    assert_eq!(shipped.matches("    quiet_minutes: 30\n").count(), watchbill::rules::ROLES.len(), "{}", shipped);
    assert_eq!(shipped.matches("    max_minutes: 240\n").count(), watchbill::rules::ROLES.len(), "{}", shipped);
}

#[test]
fn test_WB_RULE_56_a_limit_that_is_not_a_whole_number_of_at_least_1_is_refused_naming_the_place() {
    for (from, to) in [("    quiet_minutes: 30\n", "    quiet_minutes: 0\n"), ("    max_minutes: 240\n", "    max_minutes: soon\n"), ("    max_minutes: 240\n", "    max_minutes: -5\n")] {
        let why = table_with(from, to).expect_err(to);
        assert!(why.contains("roles.planner.") && why.contains(to.trim().split(':').next().unwrap()), "{}", why);
    }
}

#[test]
fn test_WB_RULE_56_a_quiet_longer_than_the_most_is_refused_naming_the_role() {
    let why = table_with("    quiet_minutes: 30\n", "    quiet_minutes: 300\n").expect_err("a quiet over the most");
    assert!(why.contains("roles.planner.quiet_minutes") && why.contains("max_minutes"), "{}", why);
}

#[test]
fn test_WB_RULE_57_a_row_with_no_limits_takes_30_and_240() {
    let table = table_with("    quiet_minutes: 30\n    max_minutes: 240\n", "").expect("a row with no limits is read");
    assert_eq!(table.roles["planner"].quiet_minutes, 30);
    assert_eq!(table.roles["planner"].max_minutes, 240);
}

/// A repository set up by `wb init --yes` with issue 1 ready, and `limits` added to `watchbill.yml`.
fn checked(limits: &str) -> (Folder, Output) {
    let folder = repository(None);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    issues_ready(&folder.path, &["1"]);
    let mut yml = read(&config(&folder.path));
    yml.push_str(limits);
    fs::write(config(&folder.path), yml).unwrap();
    let run = wb(&folder.path, &["check"], "");
    (folder, run)
}

#[test]
fn test_WB_RULE_58_watchbill_yml_may_set_a_lower_limit_for_a_role_and_check_passes() {
    let (_folder, run) = checked("session_limits:\n  build:\n    quiet_minutes: 10\n    max_minutes: 60\n  planner:\n    max_minutes: 240\n");
    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
}

#[test]
fn test_WB_RULE_58_a_higher_limit_in_watchbill_yml_is_refused_by_check_with_1_naming_the_key() {
    let (_folder, run) = checked("session_limits:\n  build:\n    max_minutes: 300\n");
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains("session_limits.build.max_minutes") && out.contains("240"), "{}", out);
    let (_folder, run) = checked("session_limits:\n  review:\n    quiet_minutes: 31\n");
    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stdout).contains("session_limits.review.quiet_minutes"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_RULE_58_a_role_or_a_key_that_is_not_known_or_a_value_that_is_no_whole_number_is_refused_by_check() {
    for (limits, place) in [
        ("session_limits:\n  builder:\n    max_minutes: 10\n", "session_limits.builder"),
        ("session_limits:\n  build:\n    most_minutes: 10\n", "session_limits.build.most_minutes"),
        ("session_limits:\n  build:\n    max_minutes: 0\n", "session_limits.build.max_minutes"),
        ("session_limits:\n  build:\n    quiet_minutes: '10'\n", "session_limits.build.quiet_minutes"),
    ] {
        let (_folder, run) = checked(limits);
        assert_eq!(code(&run), Some(1), "{}", limits);
        assert!(text(&run.stdout).contains(place), "{}: {}", limits, text(&run.stdout));
    }
}

#[test]
fn test_WB_RULE_58_the_page_of_check_says_a_role_or_a_key_it_does_not_know_is_refused() {
    let page = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("packaging/man/wb-check.1"));
    assert!(page.contains("a role or a key it does not know"), "wb-check.1 does not say it");
}

#[test]
fn test_WB_RULE_59_start_refuses_a_higher_limit_with_1_naming_the_key_before_anything_is_posted() {
    let bench = bench_with(&limits(30, 241));
    let mut err = Vec::new();
    let status = bench.start_saying(&["start", "build", "54"], &mut err);
    let err = text(&err);
    assert_eq!(status, 1, "{}", err);
    assert!(err.contains("limit: ") && err.contains("session_limits.build.max_minutes"), "{}", err);
    assert!(!bench.calls().iter().any(|call| call.starts_with("post ") || call == "start"), "{:?}", bench.calls());
}

#[test]
fn test_WB_RULE_60_the_page_of_the_revision_says_the_two_limits_and_that_watchbill_yml_sets_them_lower_only() {
    let page = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/revision.md"));
    let point = &page[page.find("## 3. What the rule table says").unwrap()..page.find("## 4.").unwrap()];
    assert!(point.contains("`quiet_minutes`") && point.contains("`max_minutes`"), "{}", point);
    assert!(point.contains("lower") && point.contains("never higher"), "{}", point);
}

// ---- WB-BUILD-153 to 160: the wb that waits is the watchdog ----

/// Whether this build reads `WB_TEST_MINUTE`: only one with debug assertions does. The RPM's `%check` runs the tests
/// in the release profile, where a test that needs a minute of one second checks nothing and says so.
fn minute_is_read() -> bool {
    if !cfg!(debug_assertions) {
        eprintln!("skipped: a build without debug assertions does not read WB_TEST_MINUTE (WB-BUILD-158)");
    }
    cfg!(debug_assertions)
}

/// A session that does nothing for `seconds`, after it says the outcome it found and its number.
fn idle(seconds: &str) -> String {
    format!("cat {{state}}/watchbill/sessions/*/outcome > {{log}}/outcome-running\necho $$ > {{log}}/pid\nsleep {}\n", seconds)
}

#[test]
fn test_WB_BUILD_153_past_quiet_minutes_one_line_is_said_and_written_and_the_session_goes_on() {
    if !minute_is_read() {
        return;
    }
    let found = watched(&idle("3"), &limits(2, 30), &[], 60);
    assert_eq!(found.status, 0, "the session went on to its end: {}", found.said);
    assert_eq!(found.quiet_lines().len(), 1, "{}", found.said);
    assert!(found.quiet_lines()[0].contains("2 minutes"), "{}", found.said);
    assert_eq!(found.quiet, "quiet: 2\n");
    assert!(found.outcome.starts_with("outcome: ended\nstatus: 0\n"), "{}", found.outcome);
    assert!(!found.outcome.contains("limit:"), "{}", found.outcome);
}

#[test]
fn test_WB_BUILD_154_the_line_is_said_again_each_time_the_quiet_doubles() {
    if !minute_is_read() {
        return;
    }
    let found = watched(&idle("5.5"), &limits(1, 30), &[], 60);
    assert_eq!(found.status, 0, "{}", found.said);
    assert_eq!(found.quiet, "quiet: 1\nquiet: 2\nquiet: 4\n", "{}", found.said);
    assert_eq!(found.quiet_lines().len(), 3, "{}", found.said);
}

/// A session that commits a file in its view, says its number and waits.
const WAITS: &str = "echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\necho $$ > {log}/pid\nexec sleep 30\n";

#[test]
fn test_WB_BUILD_155_past_max_minutes_the_session_is_ended_its_commit_taken_back_and_wb_ends_with_1() {
    if !minute_is_read() {
        return;
    }
    let found = watched(WAITS, &limits(1, 2), &[], 60);
    assert_eq!(found.status, 1, "{}", found.said);
    assert!(found.took < std::time::Duration::from_secs(20), "{:?}", found.took);
    assert!(found.outcome.starts_with("outcome: ended\n"), "{}", found.outcome);
    assert!(found.outcome.lines().any(|line| line == "signal: SIGTERM"), "{}", found.outcome);
    assert!(found.outcome.lines().any(|line| line == "limit: max_minutes"), "{}", found.outcome);
    assert!(found.back.lines().any(|line| line.starts_with("taken: ") && line.contains("A new file")), "{}", found.back);
    assert_eq!(found.subjects.lines().next(), Some("A new file"), "{}", found.subjects);
    assert!(found.said.lines().any(|line| line.starts_with("wb start build: limit: ") && line.contains("max_minutes")), "{}", found.said);
}

#[test]
fn test_WB_BUILD_155_a_session_that_ignores_the_termination_is_killed_after_the_grace() {
    if !minute_is_read() {
        return;
    }
    let session = format!("trap '' TERM\n{}", WAITS);
    let found = watched(&session, &limits(1, 2), &[("WB_TEST_GRACE", "1")], 60);
    assert_eq!(found.status, 1, "{}", found.said);
    assert!(found.took < std::time::Duration::from_secs(20), "{:?}", found.took);
    assert!(found.outcome.lines().any(|line| line == "signal: SIGKILL"), "{}", found.outcome);
    assert!(found.outcome.lines().any(|line| line == "limit: max_minutes"), "{}", found.outcome);
    assert_eq!(found.subjects.lines().next(), Some("A new file"), "{}", found.subjects);
}

#[test]
fn test_WB_BUILD_155_a_session_that_ends_with_0_at_the_termination_still_ends_wb_with_1() {
    if !minute_is_read() {
        return;
    }
    let session = "echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\ntrap 'kill $!; exit 0' TERM\nsleep 30 &\necho $$ > {log}/pid\nwait\n";
    let found = watched(session, &limits(1, 2), &[], 60);
    assert_eq!(found.status, 1, "{}", found.said);
    assert!(found.outcome.starts_with("outcome: ended\nstatus: 0\n"), "{}", found.outcome);
    assert!(found.outcome.lines().any(|line| line == "limit: max_minutes"), "{}", found.outcome);
}

#[test]
fn test_WB_BUILD_156_a_session_that_commits_now_and_then_is_not_called_quiet() {
    let session = "i=0\nwhile [ $i -lt 14 ]; do echo \"int step$i;\" > src/new.c && git add src/new.c && git commit -q -m \"Step $i\"; sleep 0.3; i=$((i+1)); done\necho $$ > {log}/pid\n";
    let found = watched(session, &limits(2, 30), &[], 60);
    assert_eq!(found.status, 0, "{}", found.said);
    assert!(found.quiet_lines().is_empty(), "{}", found.said);
    assert_eq!(found.quiet, "");
    assert_eq!(found.subjects.lines().next(), Some("Step 13"), "{}", found.subjects);
}

#[test]
fn test_WB_BUILD_157_a_fifo_or_a_link_to_dev_zero_in_the_view_does_not_make_the_quiet_check_wait() {
    if !minute_is_read() {
        return;
    }
    // The files of the view are looked at by their metadata alone: none is opened, and no link is followed. Its HEAD
    // is a FIFO too, which no read of `wb` opens.
    let session = "rm -f src/new.c && mkfifo src/new.c\nrm -f tests/test_brake.c && ln -s /dev/zero tests/test_brake.c\nmkdir -p src/deep && ln -s /dev/zero src/deep/zero\nrm -f .git/HEAD && mkfifo .git/HEAD\necho $$ > {log}/pid\nsleep 5\n";
    let found = watched(session, &limits(1, 30), &[], 60);
    assert_ne!(found.status, -2, "wb did not end: {}", found.said);
    assert!(found.took < std::time::Duration::from_secs(30), "{:?}", found.took);
    assert!(!found.quiet_lines().is_empty(), "the quiet check went on: {}", found.said);
    assert!(found.quiet.starts_with("quiet: 1\n"), "{}", found.quiet);
}

/// A session that makes `tests/test_brake.c` a tree of 25000 entries, a chain of 50 folders with 500 entries each,
/// and then writes `src/brake.c` now and then. The tree is the last path of the list but one, and is walked first.
const COMB: &str = "rm -f tests/test_brake.c && mkdir tests/test_brake.c\nd=tests/test_brake.c\ni=0\nwhile [ $i -lt 50 ]; do d=$d/d; mkdir $d && (cd $d && seq 499 | xargs touch); i=$((i+1)); done\ni=0\nwhile [ $i -lt 18 ]; do echo \"int step$i;\" > src/brake.c; sleep 0.3; i=$((i+1)); done\nrm -rf tests/test_brake.c\necho $$ > {log}/pid\n";

#[test]
fn test_WB_BUILD_157_a_tree_of_25000_entries_is_read_to_the_bound_and_every_listed_path_is_still_looked_at() {
    // What a look reads and holds is bounded by the entries read: each path of the list is looked at first, so a big
    // tree under one of them does not hide another, and the look ends fast.
    let found = watched(COMB, &limits(2, 30), &[], 60);
    assert_eq!(found.status, 0, "{}", found.said);
    assert!(found.took < std::time::Duration::from_secs(30), "{:?}", found.took);
    assert!(found.quiet_lines().is_empty(), "the writes to src/brake.c were not seen: {}", found.said);
}

#[test]
fn test_WB_BUILD_157_a_link_in_any_part_of_a_listed_path_is_not_followed() {
    if !minute_is_read() {
        return;
    }
    // `src` is made a link to a folder outside the view whose `brake.c` changes: the look does not leave the view, so
    // nothing it sees changes, and the quiet is said.
    let session = "mkdir {log}/outside && cp src/brake.c {log}/outside/brake.c\nrm -rf src && ln -s {log}/outside src\ni=0\nwhile [ $i -lt 18 ]; do echo \"int step$i;\" > {log}/outside/brake.c; sleep 0.3; i=$((i+1)); done\necho $$ > {log}/pid\n";
    let found = watched(session, &limits(2, 30), &[], 60);
    assert_ne!(found.status, -2, "wb did not end: {}", found.said);
    assert!(found.took < std::time::Duration::from_secs(30), "{:?}", found.took);
    assert!(found.quiet.starts_with("quiet: 2\n"), "a change outside the view was seen through the link: {}\n{}", found.quiet, found.said);
}

#[test]
fn test_WB_BUILD_158_a_build_without_debug_assertions_ignores_the_test_minute() {
    if cfg!(debug_assertions) {
        eprintln!("WB-BUILD-158 is not tested by a debug build: only a release run tests it (cargo test --release, or the RPM's %check)");
        return;
    }
    // With a minute of one second, a quiet of 1 would be said within 3 seconds. `wb` as installed counts real minutes.
    let found = watched(&idle("3"), &limits(1, 2), &[], 60);
    assert_eq!(found.status, 0, "{}", found.said);
    assert!(found.quiet_lines().is_empty(), "{}", found.said);
    assert!(!found.outcome.contains("limit:"), "{}", found.outcome);
}

#[test]
fn test_WB_BUILD_159_a_time_the_session_writes_on_a_file_or_a_commit_does_not_hold_it_busy() {
    if !minute_is_read() {
        return;
    }
    // The quiet is counted by `wb`'s own clock from the last look that found the view changed: a file or a commit
    // dated an hour ahead is one change, not an hour of work.
    let session = "touch -d '+1 hour' src/brake.c\necho 'int fresh;' > src/new.c && git add src/new.c && GIT_COMMITTER_DATE=\"$(date -d '+1 hour' -R)\" git commit -q -m 'Ahead'\ntouch -d '+1 hour' src/new.c\necho $$ > {log}/pid\nsleep 6\n";
    let found = watched(session, &limits(1, 30), &[], 60);
    assert_eq!(found.status, 0, "{}", found.said);
    assert!(found.quiet.starts_with("quiet: 1\nquiet: 2\n"), "{}\n{}", found.quiet, found.said);
}

#[test]
fn test_WB_BUILD_159_outcome_running_says_since_when_in_seconds_since_1970() {
    let before = now();
    let found = watched(&idle("0.2"), &limits(1, 30), &[], 60);
    let after = now();
    let running = found.left("outcome-running");
    assert!(running.starts_with("outcome: running\npid: "), "{}", running);
    let since: u64 = running.lines().find_map(|line| line.strip_prefix("since: ")).unwrap_or_else(|| panic!("{}", running)).parse().unwrap();
    assert!(before <= since && since <= after, "{} {} {}", before, since, after);
}

#[test]
fn test_WB_BUILD_160_the_readme_and_the_pages_say_the_watchdog() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let readme = read(&root.join("README.md"));
    for word in ["quiet_minutes", "max_minutes", "limit: max_minutes", "WB_TEST_MINUTE", "session_limits"] {
        assert!(readme.contains(word), "the README says nothing of {}", word);
    }
    let start = read(&root.join("packaging/man/wb-start.1"));
    assert!(start.contains("max_minutes") && start.contains("quiet_minutes"), "wb-start.1");
    let status = read(&root.join("packaging/man/wb-status.1"));
    assert!(status.contains("running:"), "wb-status.1");
}

// ---- WB-STATUS-85 to 88: `wb status <issue>` shows a session that runs ----

/// A process that is a `wb` to `wb` (`wb`, by the name of its program) and waits.
fn running_wb() -> (Folder, std::process::Child) {
    let place = folder_named("running");
    let program = place.path.join("wb");
    let child = stand_in("/bin/sh", &program, &["-c", "while :; do sleep 1; done"]);
    (place, child)
}

/// A session of the build, round 2, of issue 54 that runs: its view made as `wb start` makes it, with `src/brake.c`
/// in it, its lines with `outcome: running` of the live `wb` `pid`, `since` seconds since 1970 where one is given.
fn running_session(bench: &Bench, pid: u32, since: Option<u64>) -> (PathBuf, PathBuf) {
    let root = fs::canonicalize(&bench.repo.path).unwrap();
    let below = bench.state.path.join("watchbill").join("views");
    let name = "2026-11-24T100000Z-build-54";
    let made = watchbill::view::make(root.to_str().unwrap(), &["src/brake.c".to_string()], &below, name).unwrap();
    let session = bench.state.path.join("watchbill").join("sessions").join(name);
    fs::create_dir_all(&session).unwrap();
    fs::write(session.join("stamp"), "[stamp] build round 2\n").unwrap();
    let started = watchbill::kept::start_time(pid).unwrap();
    let since = since.map(|since| format!("since: {}\n", since)).unwrap_or_default();
    fs::write(session.join("outcome"), format!("outcome: running\npid: {}\nstarted: {}\n{}", pid, started, since)).unwrap();
    fs::write(session.join("view"), "src/brake.c\n").unwrap();
    fs::write(session.join("folder"), format!("folder: {}\nbase: {}\nhead: {}\nbranch: {}\nissue: 54\n", made.folder.display(), made.base, made.head, made.branch)).unwrap();
    (made.folder, session)
}

/// Sets the time of the file at `path` (a link itself, not what it names) to `seconds` since 1970.
fn touch(path: &Path, seconds: u64) {
    assert!(Command::new("touch").arg("-h").arg("-d").arg(format!("@{}", seconds)).arg(path).status().unwrap().success());
}

/// `wb status` in the bench's repository with its state folder, ended after `seconds`; none where it did not end.
fn status_within(bench: &Bench, args: &[&str], seconds: u64) -> Option<Output> {
    use std::io::Read;
    let mut child = Command::new("prlimit")
        .arg("--as=1073741824")
        .arg("--")
        .arg(env!("CARGO_BIN_EXE_wb"))
        .args(args)
        .current_dir(&bench.repo.path)
        .env_clear()
        .envs(env())
        .env("XDG_STATE_HOME", &bench.state.path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let (mut out, mut err) = (child.stdout.take().unwrap(), child.stderr.take().unwrap());
    let reading = std::thread::spawn(move || {
        let (mut a, mut b) = (Vec::new(), Vec::new());
        let _ = out.read_to_end(&mut a);
        let _ = err.read_to_end(&mut b);
        (a, b)
    });
    let end = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            let (stdout, stderr) = reading.join().unwrap();
            return Some(Output { status, stdout, stderr });
        }
        if std::time::Instant::now() > end {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn running_line(out: &str) -> Vec<&str> {
    out.lines().filter(|line| line.starts_with("running: ")).collect()
}

#[test]
fn test_WB_STATUS_85_a_session_that_runs_is_shown_with_how_long_it_has_run_and_been_quiet() {
    let bench = bench();
    let (_place, mut child) = running_wb();
    let at = now();
    let (view, _) = running_session(&bench, child.id(), Some(at - (7 * 3600 + 12 * 60 + 20)));
    touch(&view.join("src/brake.c"), at - (6 * 3600 + 50 * 60 + 20));
    let run = status_within(&bench, &["status", "54"], 20);
    let _ = child.kill();
    let _ = child.wait();
    let run = run.expect("wb status did not end in 20 seconds");
    let out = text(&run.stdout);
    assert_eq!(running_line(&out), vec!["running: build round 2, 7h12m, quiet 6h50m"], "{}", out);
    // Its view is the session's own: it is not named as kept, nor the session as cut off.
    assert!(!out.contains("kept:") && !out.contains("cut off"), "{}", out);
}

#[test]
fn test_WB_STATUS_85_under_an_hour_the_times_are_minutes_and_the_minutes_of_an_hour_have_two_digits() {
    let bench = bench();
    let (_place, mut child) = running_wb();
    let at = now();
    let (view, _) = running_session(&bench, child.id(), Some(at - (3600 + 5 * 60 + 20)));
    touch(&view.join("src/brake.c"), at - (12 * 60 + 20));
    let run = status_within(&bench, &["status", "54"], 20);
    let _ = child.kill();
    let _ = child.wait();
    let out = text(&run.expect("wb status did not end").stdout);
    assert_eq!(running_line(&out), vec!["running: build round 2, 1h05m, quiet 12m"], "{}", out);
}

#[test]
fn test_WB_STATUS_86_the_quiet_is_from_the_newest_of_the_start_the_commit_of_the_view_and_its_files() {
    let bench = bench();
    let (_place, mut child) = running_wb();
    let at = now();
    let (view, _) = running_session(&bench, child.id(), Some(at - 5 * 3600));
    put(&view, "src/brake.c", "int brake(void) { return 2; }\n");
    let date = format!("@{} +0000", at - 3600 - 20);
    let committed = Command::new("git")
        .arg("-C")
        .arg(&view)
        .args(["-c", "user.name=Ada Lovelace", "-c", "user.email=ada@example.org", "-c", "commit.gpgsign=false", "commit", "-q", "-a", "-m", "An hour ago"])
        .env_clear()
        .envs(env())
        .env("GIT_COMMITTER_DATE", &date)
        .env("GIT_AUTHOR_DATE", &date)
        .status()
        .unwrap();
    assert!(committed.success());
    touch(&view.join("src/brake.c"), at - 3 * 3600);
    let run = status_within(&bench, &["status", "54"], 20);
    let _ = child.kill();
    let _ = child.wait();
    let out = text(&run.expect("wb status did not end").stdout);
    assert_eq!(running_line(&out), vec!["running: build round 2, 5h00m, quiet 1h00m"], "{}", out);
}

#[test]
fn test_WB_STATUS_86_a_time_after_now_in_the_view_is_not_read() {
    let bench = bench();
    let (_place, mut child) = running_wb();
    let at = now();
    let (view, _) = running_session(&bench, child.id(), Some(at - 600));
    touch(&view.join("src/brake.c"), at + 3600);
    let run = status_within(&bench, &["status", "54"], 20);
    let _ = child.kill();
    let _ = child.wait();
    let out = text(&run.expect("wb status did not end").stdout);
    assert_eq!(running_line(&out), vec!["running: build round 2, 10m, quiet not read"], "{}", out);
}

#[test]
fn test_WB_STATUS_86_a_session_whose_outcome_says_no_since_is_shown_without_times() {
    let bench = bench();
    let (_place, mut child) = running_wb();
    running_session(&bench, child.id(), None);
    let run = status_within(&bench, &["status", "54"], 20);
    let _ = child.kill();
    let _ = child.wait();
    let out = text(&run.expect("wb status did not end").stdout);
    assert_eq!(running_line(&out), vec!["running: build round 2"], "{}", out);
}

/// A FIFO in the place of the loose object of the view's newest commit: git blocks for ever opening it.
fn fifo_for_head(view: &Path) {
    let run = Command::new("git").arg("-C").arg(view).args(["rev-parse", "HEAD"]).env_clear().envs(env()).output().unwrap();
    let head = text(&run.stdout).trim().to_string();
    let object = view.join(".git/objects").join(&head[..2]).join(&head[2..]);
    fs::remove_file(&object).unwrap();
    assert!(Command::new("mkfifo").arg(&object).status().unwrap().success());
}

#[test]
fn test_WB_STATUS_87_a_fifo_or_a_link_to_dev_zero_in_a_running_view_does_not_make_status_wait() {
    let bench = bench();
    let (_place, mut child) = running_wb();
    let at = now();
    let (view, _) = running_session(&bench, child.id(), Some(at - 600));
    put(&view, "src/brake.c", "int brake(void) { return 2; }\n");
    git(&view, &["-c", "user.name=Ada Lovelace", "-c", "user.email=ada@example.org", "-c", "commit.gpgsign=false", "commit", "-q", "-a", "-m", "A commit"]);
    fifo_for_head(&view);
    fs::remove_file(view.join("src/brake.c")).unwrap();
    std::os::unix::fs::symlink("/dev/zero", view.join("src/brake.c")).unwrap();
    let run = status_within(&bench, &["status", "54"], 15);
    let _ = child.kill();
    let _ = child.wait();
    let out = text(&run.expect("wb status did not end in 15 seconds").stdout);
    assert_eq!(running_line(&out), vec!["running: build round 2, 10m, quiet not read"], "{}", out);
}

/// A chain of `depth` folders under `at`, each with `width` entries (the next folder among them), every entry dated
/// `seconds` since 1970.
fn comb(at: &Path, depth: usize, width: usize, seconds: u64) {
    let time = std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds);
    let mut folders = vec![at.to_path_buf()];
    let mut here = at.to_path_buf();
    fs::create_dir_all(&here).unwrap();
    for _ in 0..depth {
        here = here.join("d");
        fs::create_dir(&here).unwrap();
        for n in 1..width {
            fs::File::create(here.join(n.to_string())).unwrap().set_modified(time).unwrap();
        }
        folders.push(here.clone());
    }
    for folder in folders.iter().rev() {
        fs::File::open(folder).unwrap().set_modified(time).unwrap();
    }
}

#[test]
fn test_WB_STATUS_87_a_tree_of_25000_entries_in_a_running_view_is_read_to_the_bound_and_every_listed_path_is_looked_at() {
    let bench = bench();
    let (_place, mut child) = running_wb();
    let at = now();
    let (view, session) = running_session(&bench, child.id(), Some(at - 3600));
    fs::write(session.join("view"), "src/brake.c\ntests\n").unwrap();
    comb(&view.join("tests"), 50, 500, at - 3 * 3600);
    touch(&view.join("src/brake.c"), at - (12 * 60 + 20));
    let began = std::time::Instant::now();
    let run = status_within(&bench, &["status", "54"], 20);
    let took = began.elapsed();
    let _ = child.kill();
    let _ = child.wait();
    let out = text(&run.expect("wb status did not end in 20 seconds").stdout);
    assert!(took < std::time::Duration::from_secs(10), "{:?}", took);
    assert_eq!(running_line(&out), vec!["running: build round 2, 1h00m, quiet 12m"], "{}", out);
}

#[test]
fn test_WB_STATUS_87_a_link_in_any_part_of_a_listed_path_of_a_running_view_is_not_followed() {
    let bench = bench();
    let (_place, mut child) = running_wb();
    let at = now();
    let (view, session) = running_session(&bench, child.id(), Some(at - 600));
    let outside = folder_named("outside");
    put(&outside.path, "brake.c", "int brake(void) { return 3; }\n");
    fs::remove_dir_all(view.join("src")).unwrap();
    std::os::unix::fs::symlink(&outside.path, view.join("src")).unwrap();
    let run = status_within(&bench, &["status", "54"], 20);
    // A link to `/`, with a folder of the system under it listed: the look does not leave the view, and ends at once.
    fs::remove_file(view.join("src")).unwrap();
    std::os::unix::fs::symlink("/", view.join("src")).unwrap();
    fs::write(session.join("view"), "src/brake.c\nsrc/usr\n").unwrap();
    let began = std::time::Instant::now();
    let root = status_within(&bench, &["status", "54"], 20);
    let took = began.elapsed();
    let _ = child.kill();
    let _ = child.wait();
    let out = text(&run.expect("wb status did not end").stdout);
    assert_eq!(running_line(&out), vec!["running: build round 2, 10m, quiet 10m"], "{}", out);
    let out = text(&root.expect("wb status did not end").stdout);
    assert_eq!(running_line(&out), vec!["running: build round 2, 10m, quiet 10m"], "{}", out);
    assert!(took < std::time::Duration::from_secs(10), "{:?}", took);
}

#[test]
fn test_WB_STATUS_88_status_with_no_issue_and_a_session_cut_off_say_no_running_line() {
    let bench = bench();
    let (_place, mut child) = running_wb();
    running_session(&bench, child.id(), Some(now() - 60));
    let all = status_within(&bench, &["status"], 20);
    let _ = child.kill();
    let _ = child.wait();
    let all = all.expect("wb status did not end");
    assert!(!text(&all.stdout).contains("running:") && !text(&all.stderr).contains("running:"), "{}{}", text(&all.stdout), text(&all.stderr));
    // Its `wb` is gone now: it was cut off, and is not said to run.
    let one = status_within(&bench, &["status", "54"], 20).expect("wb status did not end");
    let out = text(&one.stdout);
    assert!(running_line(&out).is_empty() && out.contains("cut off: build round 2"), "{}", out);
}
