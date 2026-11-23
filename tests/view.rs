//! The view (#126): `wb start <role>` runs a session of an issue with an accepted file in bubblewrap, in a new
//! repository that holds only the paths of `changes` and of the role's `reads`, and takes its commits back.
//!
//! The session is a stand-in: a script the test writes in its own folder, which the organisation's file of the
//! test binds into the view writable, so that the script can say what it found there. Nothing reaches the
//! network. A test that needs bubblewrap to run checks first that it can: in a container without user
//! namespaces it says so on standard error and checks nothing more. What `wb` does with no bubblewrap, or with
//! user namespaces off, is tested with a stand-in for `bwrap`, and holds everywhere.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Whether bubblewrap can make a sandbox on this machine, by the check `wb` makes itself. A test that needs
/// it says on standard error that it checked nothing, and why.
fn sandbox(test: &str) -> bool {
    match watchbill::view::works(Path::new(watchbill::view::BWRAP)) {
        Ok(()) => true,
        Err(why) => {
            eprintln!("{}: nothing checked, bubblewrap cannot make a sandbox here: {}", test, why);
            false
        }
    }
}

/// A repository made for a test, the folder of its stand-ins, the organisation's file in a folder of its own,
/// a home folder, a folder bound read-only, and the state folder.
struct Bench {
    repo: Folder,
    log: Folder,
    org: Folder,
    home: Folder,
    ro: Folder,
    state: Folder,
}

fn script(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\n{}", body)).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// The organisation's file: `stand-in` starts `session.sh` of the log folder, bound writable, with the folder
/// `ro` bound read-only and `~/.runner` bound from the home folder; it is cleared for the breakdown.
const ORG: &str = "levels: [public, internal]
default: stand-in
runners:
  stand-in:
    tools: this machine
    model: a stand-in
    agreement: none
    up_to: internal
    breakdown:
      up_to: public
      model: claude-opus-5-5
      effort: high
    start: ['{log}/session.sh', '{model}', '{effort}', '{prompt}']
    binds: [{writable: '{log}'}, '{ro}', '~/.runner']
";

/// The issue's file of issue 54: it may change `src/brake.c`, `tests/test_brake.c` and `src/new.c`, which is
/// not there yet; the tester and the build read `src/brake.h` and the folder `include`.
const ISSUE_54: &str = "issue: '54'
from: 3f0c1d9e
requirements: []
interface: |
  none
changes: [src/brake.c, tests/test_brake.c, src/new.c]
reads:
  tester: [src/brake.h, include]
  build: [src/brake.h, include]
  review: [src/brake.h]
  fix: [src/brake.h]
existing_tests_changed: []
questions: []
";

/// The list of the build of issue 54, as the stamp hashes it: `changes`, then the role's `reads`, one to a line.
const BUILD_LIST: &str = "src/brake.c\ntests/test_brake.c\nsrc/new.c\nsrc/brake.h\ninclude\n";

/// A repository set up by `wb init --yes`, with the patterns stand-ins, `build_files: [build]`, the code
/// committed in two commits (the second changes `src/secret.c`, so that the history holds an older one), and
/// the accepted file of issue 54 committed on the branch `issue-54`, which is checked out.
fn bench() -> Bench {
    bench_for(ISSUE_54)
}

/// The same, with `issue_54` as the text of the issue's file of 54.
fn bench_for(issue_54: &str) -> Bench {
    let repo = repository(Some("ssh://example.invalid/brakes/controller.git"));
    let log = folder_named("log");
    let org = folder_named("org");
    let home = folder_named("home");
    let ro = folder_named("ro");
    let state = folder_named("state");
    let l = log.path.to_str().unwrap();
    script(&log.path.join("post.sh"), &format!("echo \"post $1 $2\" >> {l}/calls\ncat > {l}/posted-$1-$2\nexit 0\n"));
    script(&log.path.join("comments.sh"), &format!("echo \"comments $1 $2\" >> {l}/calls\nexit 0\n"));
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
        "issue_address: 'tracker {issue}'".to_string(),
        "change_address: 'server {change}'".to_string(),
        "confidentiality: public".to_string(),
        "build_files: [build]".to_string(),
    ] {
        yml.push_str(&format!("{}\n", line));
    }
    fs::write(config(&repo.path), yml).unwrap();
    put(&repo.path, "src/brake.c", "int brake(void) { return 1; }\n");
    put(&repo.path, "src/brake.h", "int brake(void);\n");
    put(&repo.path, "src/secret.c", "the first secret\n");
    put(&repo.path, "include/a.h", "#define A 1\n");
    put(&repo.path, "include/b.h", "#define B 2\n");
    put(&repo.path, "tests/test_brake.c", "int main(void) { return 0; }\n");
    put(&repo.path, "build/Makefile", "all:\n");
    git(&repo.path, &["add", "--", ".watchbill", "src", "include", "tests", "build"]);
    git(&repo.path, &["commit", "-q", "-m", "Watchbill and the code"]);
    put(&repo.path, "src/secret.c", "the second secret\n");
    git(&repo.path, &["commit", "-q", "-a", "-m", "The secret, again"]);
    git(&repo.path, &["branch", "-M", "main"]);
    git(&repo.path, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(&repo.path, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"]);
    // Issue 99 has no file: its branch is there for a role started alone.
    git(&repo.path, &["branch", "issue-99"]);
    git(&repo.path, &["checkout", "-q", "-b", "issue-54"]);
    put(&repo.path, ".watchbill/issues/54.yml", issue_54);
    let run = wb(&repo.path, &["accept", "54", "--no-commit"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    git(&repo.path, &["add", "--", ".watchbill/issues"]);
    git(&repo.path, &["commit", "-q", "-m", "The issue's file of 54"]);
    put(&ro.path, "kept.txt", "read me\n");
    put(&home.path, "secret.txt", "the home's own\n");
    put(&home.path, ".runner/settings", "the runner's own\n");
    let text = ORG.replace("{log}", l).replace("{ro}", ro.path.to_str().unwrap());
    fs::write(org.path.join("organisation.yml"), text).unwrap();
    Bench { repo, log, org, home, ro, state }
}

impl Bench {
    fn root(&self) -> String {
        fs::canonicalize(&self.repo.path).unwrap().to_str().unwrap().to_string()
    }

    fn at(&self, name: &str) -> PathBuf {
        self.log.path.join(name)
    }

    fn organisation(&self) -> PathBuf {
        self.org.path.join("organisation.yml")
    }

    /// What a stand-in left in the log folder, or an empty text.
    fn left(&self, name: &str) -> String {
        fs::read_to_string(self.at(name)).unwrap_or_default()
    }

    fn calls(&self) -> Vec<String> {
        self.left("calls").lines().map(str::to_string).collect()
    }

    fn started(&self) -> bool {
        self.calls().iter().any(|call| call == "start")
    }

    fn any_posted(&self) -> bool {
        self.calls().iter().any(|call| call.starts_with("post "))
    }

    fn posted(&self) -> String {
        self.left("posted-issue-54")
    }

    /// The session: a script run in the view, after it notes that it started. `{log}` is the log folder.
    fn session(&self, body: &str) {
        let l = self.log.path.to_str().unwrap();
        let s = self.state.path.to_str().unwrap();
        script(&self.at("session.sh"), &format!("echo start >> {l}/calls\n{}", body.replace("{log}", l).replace("{state}", s)));
    }

    fn start_by(&self, sandbox: &Path, args: &[&str]) -> Output {
        use std::os::unix::process::ExitStatusExt;
        let args: Vec<std::ffi::OsString> = args.iter().map(std::ffi::OsString::from).collect();
        let organisation = self.organisation();
        let machine = watchbill::Machine {
            organisation: &organisation,
            terminal: true,
            state: Some(&self.state.path),
            sandbox,
            home: Some(&self.home.path),
        };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let status = watchbill::start_on(&args, &self.repo.path, &machine, &mut out, &mut err);
        Output { status: std::process::ExitStatus::from_raw(status << 8), stdout: out, stderr: err }
    }

    fn start(&self, args: &[&str]) -> Output {
        self.start_by(Path::new(watchbill::view::BWRAP), args)
    }

    fn git_text(&self, args: &[&str]) -> String {
        let run = Command::new("git").arg("-C").arg(&self.repo.path).args(args).env_clear().envs(env()).output().unwrap();
        assert!(run.status.success(), "git {:?}: {}", args, text(&run.stderr));
        text(&run.stdout)
    }

    fn head(&self) -> String {
        self.git_text(&["rev-parse", "HEAD"]).trim().to_string()
    }

    /// The folders below `<state>/watchbill/<kind>`, by name.
    fn below(&self, kind: &str) -> Vec<PathBuf> {
        let Ok(found) = fs::read_dir(self.state.path.join("watchbill").join(kind)) else { return Vec::new() };
        let mut found: Vec<PathBuf> = found.map(|entry| entry.unwrap().path()).collect();
        found.sort();
        found
    }

    /// The one folder of the lines of a start.
    fn lines(&self) -> PathBuf {
        let found = self.below("sessions");
        assert_eq!(found.len(), 1, "{:?}", found);
        found[0].clone()
    }
}

fn ok(run: &Output) {
    assert_eq!(code(run), Some(0), "{}", text(&run.stderr));
}

/// The SHA-256 of a text, by `sha256sum`.
fn sha256(text: &str) -> String {
    let mut child = Command::new("sha256sum")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::io::Write::write_all(&mut child.stdin.take().unwrap(), text.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    text_of(&out.stdout).split_whitespace().next().unwrap().to_string()
}

fn text_of(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

/// The value of a line `name: value` of a stamp.
fn line_of(stamp: &str, name: &str) -> Option<String> {
    stamp.lines().find_map(|line| line.strip_prefix(&format!("{}: ", name)).map(str::to_string))
}

// ---- WB-VIEW-1: the view holds only the listed paths, as in HEAD, in one commit ----

#[test]
fn test_WB_VIEW_1_the_view_holds_only_the_listed_paths_as_in_head_in_one_commit_and_no_other_object() {
    if !sandbox("test_WB_VIEW_1") {
        return;
    }
    let bench = bench();
    // A change in the checkout that is not committed is not in the view: the view is HEAD's. A change under
    // `reads` is let by `--ignore-local` (#172); one under `changes` stops the start.
    put(&bench.repo.path, "src/brake.h", "int brake(int);\n");
    bench.session(
        "git ls-files > {log}/files\n\
         find . -path ./.git -prune -o -type f -print | sort > {log}/found\n\
         cat src/brake.h > {log}/brake\n\
         git rev-list --all --count > {log}/count\n\
         git remote > {log}/remotes\n\
         git cat-file --batch-all-objects --batch-check='%(objecttype) %(objectname)' > {log}/objects\n",
    );
    let before: Vec<String> = ["src/brake.c", "src/brake.h", "include/a.h", "include/b.h", "tests/test_brake.c", "src/secret.c"]
        .iter()
        .map(|path| bench.git_text(&["rev-parse", &format!("HEAD:{}", path)]).trim().to_string())
        .collect();
    let older = bench.git_text(&["rev-parse", "HEAD~1:src/secret.c"]).trim().to_string();
    let commits = bench.git_text(&["rev-list", "--all"]);

    ok(&bench.start(&["start", "build", "54", "--ignore-local"]));

    let listed = "include/a.h\ninclude/b.h\nsrc/brake.c\nsrc/brake.h\ntests/test_brake.c\n";
    assert_eq!(bench.left("files"), listed);
    let found: String = listed.lines().map(|path| format!("./{}\n", path)).collect();
    assert_eq!(bench.left("found"), found);
    assert_eq!(bench.left("brake"), "int brake(void);\n");
    assert_eq!(bench.left("count").trim(), "1");
    assert_eq!(bench.left("remotes"), "");
    let objects = bench.left("objects");
    let of = |kind: &str| -> Vec<String> {
        objects.lines().filter_map(|line| line.strip_prefix(&format!("{} ", kind))).map(str::to_string).collect()
    };
    let mut blobs = of("blob");
    blobs.sort();
    let mut wanted: Vec<String> = before[..5].to_vec();
    wanted.sort();
    assert_eq!(blobs, wanted, "{}", objects);
    assert_eq!(of("commit").len(), 1, "{}", objects);
    assert!(of("tag").is_empty(), "{}", objects);
    for hidden in [&before[5], &older] {
        assert!(!objects.contains(hidden.as_str()), "{} in {}", hidden, objects);
    }
    for commit in commits.lines() {
        assert!(!objects.contains(commit), "{} in {}", commit, objects);
    }
}

// ---- WB-VIEW-2: the session runs in bubblewrap, in the view at the path of the checkout ----

#[test]
fn test_WB_VIEW_2_the_session_runs_in_the_view_at_the_checkout_with_the_system_read_only_and_no_other_home_folder() {
    if !sandbox("test_WB_VIEW_2") {
        return;
    }
    let bench = bench();
    let home = bench.home.path.to_str().unwrap().to_string();
    bench.session(&format!(
        "pwd > {{log}}/pwd\n\
         echo here > written.txt && echo yes > {{log}}/view-written && rm written.txt\n\
         touch /usr/wb-test-written 2> /dev/null && echo yes > {{log}}/usr-written\n\
         touch /etc/wb-test-written 2> /dev/null && echo yes > {{log}}/etc-written\n\
         test -e /usr/bin/git && echo yes > {{log}}/usr-read\n\
         cat '{home}/secret.txt' > {{log}}/home-secret 2> /dev/null\n\
         ls -a '{home}' > {{log}}/home-list 2> /dev/null\n"
    ));

    ok(&bench.start(&["start", "build", "54"]));

    assert_eq!(bench.left("pwd").trim(), bench.root());
    assert_eq!(bench.left("view-written"), "yes\n");
    assert_eq!(bench.left("usr-written"), "");
    assert_eq!(bench.left("etc-written"), "");
    assert_eq!(bench.left("usr-read"), "yes\n");
    assert_eq!(bench.left("home-secret"), "");
    // Only what the organisation's file binds from the home folder is in it.
    let names: Vec<String> = bench.left("home-list").lines().filter(|name| *name != "." && *name != "..").map(str::to_string).collect();
    assert_eq!(names, [".runner"], "{:?}", names);
    assert!(!Path::new("/usr/wb-test-written").exists() && !Path::new("/etc/wb-test-written").exists());
}

// ---- WB-VIEW-3: a path that is not listed cannot be read, as a file or from the history ----

#[test]
fn test_WB_VIEW_3_a_path_that_is_not_listed_cannot_be_read_as_a_file_or_from_the_history() {
    if !sandbox("test_WB_VIEW_3") {
        return;
    }
    let bench = bench();
    bench.session(
        "cat src/secret.c > {log}/cat 2>&1; echo $? > {log}/cat.status\n\
         git show HEAD~1:src/secret.c > {log}/older 2>&1; echo $? > {log}/older.status\n\
         git show HEAD:src/secret.c > {log}/head 2>&1; echo $? > {log}/head.status\n\
         git log --all --format=%s > {log}/log 2>&1\n",
    );

    ok(&bench.start(&["start", "build", "54"]));

    for name in ["cat", "older", "head"] {
        assert_ne!(bench.left(&format!("{}.status", name)).trim(), "0", "{}: {}", name, bench.left(name));
        assert!(!bench.left(name).contains("secret\n"), "{}: {}", name, bench.left(name));
    }
    assert_eq!(bench.left("log").lines().count(), 1, "{}", bench.left("log"));
}

// ---- WB-VIEW-4: the checkout's .git, its .watchbill/ and the organisation's file are not inside ----

#[test]
fn test_WB_VIEW_4_the_checkout_s_git_its_watchbill_and_the_organisation_s_file_are_not_inside() {
    if !sandbox("test_WB_VIEW_4") {
        return;
    }
    let bench = bench();
    let head = bench.head();
    bench.session(&format!(
        "test -e .watchbill && echo yes > {{log}}/watchbill\n\
         git cat-file -e {head} 2> /dev/null && echo yes > {{log}}/head\n\
         cat '{org}' > {{log}}/org 2> /dev/null\n\
         ls '{folder}' > {{log}}/org-folder 2> /dev/null\n",
        head = head,
        org = bench.organisation().display(),
        folder = bench.org.path.display(),
    ));

    ok(&bench.start(&["start", "build", "54"]));

    assert!(bench.started());
    for name in ["watchbill", "head", "org", "org-folder"] {
        assert_eq!(bench.left(name), "", "{}", name);
    }
}

#[test]
fn test_WB_VIEW_4_the_folder_of_the_organisation_s_file_is_covered_after_the_system_is_bound() {
    let plan = watchbill::view::Plan {
        view: Path::new("/var/views/1"),
        checkout: Path::new("/work/brakes"),
        binds: vec![],
        organisation: Some(Path::new("/etc/watchbill")),
        home: Some(Path::new("/home/ada")),
        new_session: true,
        words: &["runner".to_string()],
    };
    let words: Vec<String> = watchbill::view::arguments(&plan).iter().map(|word| word.to_str().unwrap().to_string()).collect();
    let find = |pair: [&str; 3]| words.windows(3).position(|three| three == pair);
    let etc = find(["--ro-bind", "/etc", "/etc"]).expect("/etc is not bound");
    let covered = words.windows(2).position(|two| two == ["--tmpfs", "/etc/watchbill"]).expect("the organisation's folder is not covered");
    assert!(etc < covered, "{:?}", words);
    let view = find(["--bind", "/var/views/1", "/work/brakes"]).expect("the view is not bound at the checkout");
    assert!(covered < view, "{:?}", words);
    let rest: Vec<&str> = words.iter().skip_while(|word| *word != "--").map(String::as_str).collect();
    assert_eq!(rest, ["--", "runner"]);
}

#[test]
fn test_WB_VIEW_4_a_bind_that_holds_the_checkout_the_organisation_s_folder_or_the_state_folder_is_refused() {
    let bench = bench();
    let parent = bench.repo.path.parent().unwrap().to_str().unwrap().to_string();
    let org_folder = bench.org.path.to_str().unwrap().to_string();
    let state = bench.state.path.join("watchbill").to_str().unwrap().to_string();
    let good = format!("binds: [{{writable: '{}'}}, '{}', '~/.runner']", bench.log.path.display(), bench.ro.path.display());
    for bad in [&parent, &bench.root(), &format!("{}/src", bench.root()), &org_folder, &state] {
        let text = read(&bench.organisation()).replace(&good, &format!("binds: ['{}']", bad));
        fs::write(bench.organisation(), text).unwrap();
        let run = bench.start_by(Path::new("/nonexistent/no-bwrap"), &["start", "build", "54"]);
        assert_eq!(code(&run), Some(1), "{}: {}", bad, text_of(&run.stderr));
        assert!(text_of(&run.stderr).contains(bad.as_str()), "{}: {}", bad, text_of(&run.stderr));
        assert!(!bench.any_posted() && !bench.started(), "{}: {:?}", bad, bench.calls());
        let text = read(&bench.organisation()).replace(&format!("binds: ['{}']", bad), &good);
        fs::write(bench.organisation(), text).unwrap();
    }
}

// ---- WB-CONF-25: a runner's `binds` ----

#[test]
fn test_WB_CONF_25_a_runner_s_binds_are_inside_read_only_unless_marked_writable() {
    if !sandbox("test_WB_CONF_25") {
        return;
    }
    let bench = bench();
    let ro = bench.ro.path.to_str().unwrap().to_string();
    let home = bench.home.path.to_str().unwrap().to_string();
    bench.session(&format!(
        "cat '{ro}/kept.txt' > {{log}}/ro-read\n\
         touch '{ro}/new.txt' 2> /dev/null && echo yes > {{log}}/ro-written\n\
         cat '{home}/.runner/settings' > {{log}}/home-read\n\
         touch '{home}/.runner/new' 2> /dev/null && echo yes > {{log}}/home-written\n\
         touch {{log}}/rw-written && echo yes > {{log}}/rw-said\n"
    ));

    ok(&bench.start(&["start", "build", "54"]));

    assert_eq!(bench.left("ro-read"), "read me\n");
    assert_eq!(bench.left("ro-written"), "");
    assert!(!bench.ro.path.join("new.txt").exists());
    assert_eq!(bench.left("home-read"), "the runner's own\n");
    assert_eq!(bench.left("home-written"), "");
    assert_eq!(bench.left("rw-said"), "yes\n");
}

#[test]
fn test_WB_CONF_25_binds_are_read_as_paths_each_read_only_unless_marked_writable() {
    let folder = folder_named("org");
    let path = folder.path.join("organisation.yml");
    let base = ORG.replace("{log}", "/opt/log").replace("{ro}", "/opt/ro");
    fs::write(&path, &base).unwrap();
    let found = watchbill::confidentiality::read(&path).unwrap();
    let binds: Vec<(String, bool)> = found.runners[0].binds.iter().map(|bind| (bind.path.clone(), bind.writable)).collect();
    assert_eq!(binds, [("/opt/log".to_string(), true), ("/opt/ro".to_string(), false), ("~/.runner".to_string(), false)]);
    // A runner with no `binds` has none.
    let none = base.lines().filter(|line| !line.contains("binds:")).map(|line| format!("{}\n", line)).collect::<String>();
    fs::write(&path, none).unwrap();
    assert!(watchbill::confidentiality::read(&path).unwrap().runners[0].binds.is_empty());
}

#[test]
fn test_WB_CONF_25_binds_that_are_not_a_list_of_paths_are_refused_with_the_file_and_the_place() {
    let folder = folder_named("org");
    let path = folder.path.join("organisation.yml");
    let line = "    binds: [{writable: '/opt/log'}, '/opt/ro', '~/.runner']";
    let base = ORG.replace("{log}", "/opt/log").replace("{ro}", "/opt/ro");
    assert!(base.contains(line));
    for bad in [
        "binds: /opt/tool",
        "binds: [opt/tool]",
        "binds: ['/opt/../etc']",
        "binds: ['/']",
        "binds: ['~']",
        "binds: ['~ada/x']",
        "binds: ['']",
        "binds: [{writable: /opt/a, other: /opt/b}]",
        "binds: [{read: /opt/a}]",
        "binds: [{writable: [/opt/a]}]",
        "binds: [[/opt/a]]",
        "binds: {writable: /opt/a}",
    ] {
        fs::write(&path, base.replace(line, &format!("    {}", bad))).unwrap();
        let why = watchbill::confidentiality::read(&path).expect_err(bad);
        assert!(why.contains(path.to_str().unwrap()), "{}: {}", bad, why);
        assert!(why.contains("binds"), "{}: {}", bad, why);
    }
}

// ---- WB-VIEW-5: a commit outside `changes`, under .watchbill/ or under `build_files` is refused ----

/// Refused with 1, each of `named` said, the branch where it was, and the view kept and named.
fn refused_back(bench: &Bench, run: &Output, head: &str, named: &[&str]) {
    assert_eq!(code(run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    for name in named {
        assert!(err.contains(name), "{} not in: {}", name, err);
    }
    assert_eq!(bench.head(), head, "a commit was put on the branch");
    let views = bench.below("views");
    assert_eq!(views.len(), 1, "{:?}", views);
    assert!(err.contains(views[0].to_str().unwrap()), "the view is not named: {}", err);
}

#[test]
fn test_WB_VIEW_5_a_commit_that_touches_a_path_outside_changes_is_refused_with_1_named_and_nothing_put_on_the_branch() {
    if !sandbox("test_WB_VIEW_5") {
        return;
    }
    let bench = bench();
    let head = bench.head();
    bench.session("echo 'int brake(int);' > src/brake.h && git commit -q -a -m 'Change a read'\n");

    let run = bench.start(&["start", "build", "54"]);

    refused_back(&bench, &run, &head, &["src/brake.h", "Change a read"]);
}

#[test]
fn test_WB_VIEW_5_a_commit_under_watchbill_or_under_build_files_is_refused_in_the_same_way() {
    if !sandbox("test_WB_VIEW_5") {
        return;
    }
    let bench = bench();
    let head = bench.head();
    bench.session("mkdir -p .watchbill && echo x > .watchbill/x && git add .watchbill/x && git commit -q -m 'Under watchbill'\n");
    let run = bench.start(&["start", "build", "54"]);
    refused_back(&bench, &run, &head, &[".watchbill/x", "Under watchbill"]);

    // A path of `changes` that `build_files` names since the acceptance is refused for it.
    let bench = bench_with_build_files("[src/new.c]");
    let head = bench.head();
    bench.session("echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A file of the build'\n");
    let run = bench.start(&["start", "build", "54"]);
    refused_back(&bench, &run, &head, &["src/new.c", "build_files", "A file of the build"]);
}

/// The bench with `build_files` of watchbill.yml changed and committed after the acceptance.
fn bench_with_build_files(list: &str) -> Bench {
    let bench = bench();
    let yml = read(&config(&bench.repo.path)).replace("build_files: [build]", &format!("build_files: {}", list));
    fs::write(config(&bench.repo.path), yml).unwrap();
    git(&bench.repo.path, &["commit", "-q", "-a", "-m", "The files of the build"]);
    bench
}

#[test]
fn test_WB_VIEW_5_the_commits_before_a_refused_one_are_taken_and_none_after_it() {
    if !sandbox("test_WB_VIEW_5") {
        return;
    }
    let bench = bench();
    let head = bench.head();
    bench.session(
        "echo 'int brake(void) { return 2; }' > src/brake.c && git commit -q -a -m 'First, kept'\n\
         echo 'int brake(int);' > src/brake.h && git commit -q -a -m 'Second, refused'\n\
         echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'Third, after it'\n",
    );

    let run = bench.start(&["start", "build", "54"]);

    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(err.contains("Second, refused") && err.contains("src/brake.h") && err.contains("Third, after it"), "{}", err);
    assert_eq!(bench.git_text(&["log", "--format=%s", &format!("{}..HEAD", head)]), "First, kept\n");
    assert!(!bench.repo.path.join("src/new.c").exists());
    assert_eq!(bench.below("views").len(), 1);
}

// ---- WB-VIEW-6: the commits that keep to `changes` are put on the branch ----

#[test]
fn test_WB_VIEW_6_commits_inside_the_list_are_put_on_the_branch_in_order_with_their_messages_and_authors() {
    if !sandbox("test_WB_VIEW_6") {
        return;
    }
    let bench = bench();
    let head = bench.head();
    let origin = bench.git_text(&["rev-parse", "refs/remotes/origin/main"]);
    bench.session(
        "echo 'int brake(void) { return 2; }' > src/brake.c\n\
         git add src/brake.c && GIT_AUTHOR_NAME='Grace Hopper' GIT_AUTHOR_EMAIL=grace@example.org git commit -q -m 'First: brake returns 2'\n\
         echo 'int fresh;' > src/new.c\n\
         git add src/new.c && git commit -q -m 'Second: a new file' -m 'With a body.'\n",
    );

    ok(&bench.start(&["start", "build", "54"]));

    let log = bench.git_text(&["log", "--format=%an <%ae>|%B|", &format!("{}..HEAD", head)]);
    assert_eq!(
        log,
        "Ada Lovelace <ada@example.org>|Second: a new file\n\nWith a body.\n|\nGrace Hopper <grace@example.org>|First: brake returns 2\n|\n"
    );
    assert_eq!(bench.git_text(&["rev-parse", "--abbrev-ref", "HEAD"]).trim(), "issue-54");
    assert_eq!(read(&bench.repo.path.join("src/brake.c")), "int brake(void) { return 2; }\n");
    assert_eq!(read(&bench.repo.path.join("src/new.c")), "int fresh;\n");
    assert_eq!(bench.git_text(&["status", "--porcelain"]), "");
    // Nothing was pushed.
    assert_eq!(bench.git_text(&["rev-parse", "refs/remotes/origin/main"]), origin);
    assert!(!bench.at("network").exists());
}

// ---- WB-VIEW-7: work left uncommitted is named and not taken ----

#[test]
fn test_WB_VIEW_7_work_left_uncommitted_in_the_view_is_named_and_not_taken() {
    if !sandbox("test_WB_VIEW_7") {
        return;
    }
    let bench = bench();
    let head = bench.head();
    bench.session("echo 'int brake(void) { return 3; }' > src/brake.c\necho 'int fresh;' > src/new.c\n");

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let err = text_of(&run.stderr);
    assert!(err.contains("src/brake.c") && err.contains("src/new.c"), "{}", err);
    assert!(err.contains("uncommitted"), "{}", err);
    assert_eq!(bench.head(), head);
    assert_eq!(read(&bench.repo.path.join("src/brake.c")), "int brake(void) { return 1; }\n");
    assert!(!bench.repo.path.join("src/new.c").exists());
}

// ---- WB-VIEW-8: the view is removed when all was taken, and kept and named when not ----

#[test]
fn test_WB_VIEW_8_the_view_is_removed_when_all_was_taken_and_kept_and_named_when_anything_was_not() {
    if !sandbox("test_WB_VIEW_8") {
        return;
    }
    let bench = bench();
    bench.session("echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\n");
    ok(&bench.start(&["start", "build", "54"]));
    assert!(bench.below("views").is_empty(), "{:?}", bench.below("views"));

    bench.session("echo 'int brake(int);' > src/brake.h && git commit -q -a -m 'Change a read'\n");
    let run = bench.start(&["start", "build", "54"]);
    assert_eq!(code(&run), Some(1));
    let views = bench.below("views");
    assert_eq!(views.len(), 1, "{:?}", views);
    assert!(text_of(&run.stderr).contains(views[0].to_str().unwrap()), "{}", text_of(&run.stderr));
    assert_eq!(read(&views[0].join("src/brake.h")), "int brake(int);\n");
}

// ---- WB-VIEW-9: the stamp's line `view:` ----

#[test]
fn test_WB_VIEW_9_the_stamp_s_view_is_the_sha256_of_the_list_and_none_for_the_breakdown_and_a_role_with_no_file() {
    let bench = bench();
    let stamp = |words: &[&str]| -> String {
        let run = prompt_with(&bench.repo.path, &bench.organisation(), words);
        ok(&run);
        text_of(&run.stdout)
    };
    let build = stamp(&["build", "54", "--round", "1", "--stamp"]);
    assert_eq!(line_of(&build, "view"), Some(sha256(BUILD_LIST)), "{}", build);
    // It stands after the time.
    let names: Vec<&str> = build.lines().map(|line| line.split(':').next().unwrap()).collect();
    let time = names.iter().position(|name| *name == "time").unwrap();
    assert_eq!(names[time + 1], "view", "{}", build);
    let review = stamp(&["review", "54", "--change", "174", "--round", "1", "--stamp"]);
    assert_eq!(line_of(&review, "view"), Some(sha256("src/brake.c\ntests/test_brake.c\nsrc/new.c\nsrc/brake.h\n")), "{}", review);
    let breakdown = stamp(&["breakdown", "54", "--runner", "stand-in", "--round", "1", "--stamp"]);
    assert_eq!(line_of(&breakdown, "view").as_deref(), Some("none"), "{}", breakdown);
    let alone = stamp(&["build", "99", "--round", "1", "--stamp"]);
    assert_eq!(line_of(&alone, "view").as_deref(), Some("none"), "{}", alone);
}

#[test]
fn test_WB_VIEW_9_the_stamp_posted_has_the_view_of_the_list_the_session_ran_in() {
    if !sandbox("test_WB_VIEW_9") {
        return;
    }
    let bench = bench();
    ok(&bench.start(&["start", "build", "54"]));
    let posted = bench.posted();
    assert!(posted.starts_with("[stamp] build round 1\n"), "{}", posted);
    assert_eq!(line_of(&posted, "view"), Some(sha256(BUILD_LIST)), "{}", posted);
    assert_eq!(read(&bench.lines().join("view")), BUILD_LIST);
}

// ---- WB-VIEW-10: a role with no issue's file runs without a view, and says so ----

#[test]
fn test_WB_VIEW_10_a_role_started_without_an_issue_s_file_runs_without_a_view_and_says_so_first() {
    let bench = bench();
    bench.session("pwd > {log}/pwd\ntest -d .watchbill && echo yes > {log}/watchbill\n");

    // No bubblewrap is needed for it.
    let run = bench.start_by(Path::new("/nonexistent/no-bwrap"), &["start", "build", "99"]);

    ok(&run);
    let err = text_of(&run.stderr);
    let said = err.lines().position(|line| line.contains("without a view")).unwrap_or_else(|| panic!("not said: {}", err));
    let starts = err.lines().position(|line| line.starts_with("wb start build: starts build")).unwrap();
    assert!(said < starts, "{}", err);
    assert_eq!(bench.left("pwd").trim(), bench.root());
    assert_eq!(bench.left("watchbill"), "yes\n");
    assert_eq!(line_of(&bench.left("posted-issue-99"), "view").as_deref(), Some("none"));
    assert!(!bench.lines().join("view").exists());
    assert!(bench.below("views").is_empty());
}

// ---- WB-VIEW-11: no bwrap, or user namespaces off ----

#[test]
fn test_WB_VIEW_11_with_no_bwrap_it_is_refused_with_1_saying_why_before_anything_is_posted() {
    let bench = bench();
    let run = bench.start_by(Path::new("/nonexistent/no-bwrap"), &["start", "build", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(err.contains("/nonexistent/no-bwrap") && err.contains("bubblewrap"), "{}", err);
    assert!(!bench.any_posted() && !bench.started(), "{:?}", bench.calls());
    let outcome = read(&bench.lines().join("outcome"));
    assert!(outcome.starts_with("outcome: refused\ncheck: view\nstatus: 1\n"), "{}", outcome);
    assert!(bench.below("views").is_empty());
}

#[test]
fn test_WB_VIEW_11_with_user_namespaces_off_it_is_refused_with_1_saying_why_before_anything_is_posted() {
    let bench = bench();
    let stand_in = bench.at("bwrap");
    script(&stand_in, "echo 'bwrap: No permissions to create a new namespace, likely because the kernel does not allow non-privileged user namespaces.' >&2\nexit 1\n");
    let run = bench.start_by(&stand_in, &["start", "build", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(err.contains("No permissions to create a new namespace"), "{}", err);
    assert!(!bench.any_posted() && !bench.started(), "{:?}", bench.calls());
    assert!(read(&bench.lines().join("outcome")).starts_with("outcome: refused\ncheck: view\n"));
}

// ---- WB-VIEW-12: the package requires bubblewrap ----

#[test]
fn test_WB_VIEW_12_the_package_requires_bubblewrap() {
    let spec = read(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("packaging/watchbill.spec"));
    assert!(
        spec.lines().any(|line| line.starts_with("Requires:") && line.split_whitespace().last() == Some("bubblewrap")),
        "{}",
        spec
    );
}

// ---- WB-MON-7: the folder of a session in a view holds `view` and `back` ----

#[test]
fn test_WB_MON_7_the_folder_of_a_session_in_a_view_holds_view_and_back() {
    if !sandbox("test_WB_MON_7") {
        return;
    }
    let bench = bench();
    bench.session("echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\n");
    ok(&bench.start(&["start", "build", "54"]));
    let folder = bench.lines();
    assert_eq!(read(&folder.join("view")), BUILD_LIST);
    let back = read(&folder.join("back"));
    let taken = bench.head();
    assert!(back.lines().any(|line| line.starts_with("taken: ") && line.contains(&taken[..12]) && line.contains("A new file")), "{}", back);
    assert!(back.lines().any(|line| line == "view: removed"), "{}", back);

    let bench = self::bench();
    bench.session("echo 'int brake(int);' > src/brake.h && git commit -q -a -m 'Change a read'\n");
    assert_eq!(code(&bench.start(&["start", "build", "54"])), Some(1));
    let back = read(&bench.lines().join("back"));
    assert!(back.lines().any(|line| line.starts_with("refused: ") && line.contains("Change a read") && line.contains("src/brake.h")), "{}", back);
    assert!(back.lines().any(|line| line.starts_with("view: kept ")), "{}", back);
}

// ---- WB-VIEW-14 to 16: the commands that push the branch and open the change are said, not run (#145) ----

/// Adds a line to `watchbill.yml` of the bench.
fn with_line(bench: &Bench, line: &str) {
    let mut yml = read(&config(&bench.repo.path));
    yml.push_str(line);
    yml.push('\n');
    fs::write(config(&bench.repo.path), yml).unwrap();
    commit_config_keeping_refs(&bench.repo.path);
}

#[test]
fn test_WB_VIEW_14_after_the_commits_are_taken_back_the_push_command_is_said_and_not_run() {
    if !sandbox("test_WB_VIEW_14") {
        return;
    }
    let bench = bench();
    bench.session("echo 'int main(void) { return 1; }' > tests/test_brake.c && git commit -q -a -m 'A test'\n");

    let run = bench.start(&["start", "tester", "54"]);

    ok(&run);
    let err = text_of(&run.stderr);
    let taken = err.find("taken onto issue-54").unwrap_or_else(|| panic!("{}", err));
    let push = err.find("git push origin issue-54").unwrap_or_else(|| panic!("{}", err));
    assert!(taken < push, "{}", err);
    // Not run: nothing reached the remote, and no branch of it was made.
    assert!(!bench.at("network").exists());
    assert_eq!(bench.git_text(&["for-each-ref", "--format=%(refname)", "refs/remotes/origin/issue-54"]), "");
    // The tester opens no change.
    assert!(!err.contains("draft_change") && !err.contains("as a draft"), "{}", err);

    // A session that commits nothing gets no push command.
    let bench = self::bench();
    let run = bench.start(&["start", "tester", "54"]);
    ok(&run);
    assert!(!text_of(&run.stderr).contains("git push"), "{}", text_of(&run.stderr));
}

#[test]
fn test_WB_VIEW_15_after_a_build_the_command_that_opens_the_change_as_a_draft_is_said_and_not_run() {
    if !sandbox("test_WB_VIEW_15") {
        return;
    }
    let bench = bench();
    with_line(&bench, "draft_change: 'draft-stand-in --head {branch}'");
    bench.session("echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\n");

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let err = text_of(&run.stderr);
    let push = err.find("git push origin issue-54").unwrap_or_else(|| panic!("{}", err));
    let draft = err.find("draft-stand-in --head issue-54").unwrap_or_else(|| panic!("{}", err));
    assert!(push < draft, "{}", err);
    // A command that is not there would have failed the start, had it been run.
    assert!(!err.contains("not found"), "{}", err);
}

#[test]
fn test_WB_VIEW_15_without_draft_change_it_says_the_line_is_not_there() {
    if !sandbox("test_WB_VIEW_15") {
        return;
    }
    let bench = bench();
    bench.session("echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\n");

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let err = text_of(&run.stderr);
    assert!(err.contains("git push origin issue-54"), "{}", err);
    assert!(err.contains("`draft_change`"), "{}", err);
}

#[test]
fn test_WB_VIEW_16_without_a_view_new_commits_on_the_branch_get_the_same_commands() {
    let bench = bench();
    git(&bench.repo.path, &["checkout", "-q", "issue-99"]);
    with_line(&bench, "draft_change: 'draft-stand-in --head {branch}'");
    bench.session("echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\n");

    let run = bench.start_by(Path::new("/nonexistent/no-bwrap"), &["start", "build", "99"]);

    ok(&run);
    let err = text_of(&run.stderr);
    assert!(err.contains("git push origin issue-99"), "{}", err);
    assert!(err.contains("draft-stand-in --head issue-99"), "{}", err);
    assert!(!bench.at("network").exists());

    // A session that commits nothing gets no command.
    let bench = self::bench();
    git(&bench.repo.path, &["checkout", "-q", "issue-99"]);
    let run = bench.start_by(Path::new("/nonexistent/no-bwrap"), &["start", "build", "99"]);
    ok(&run);
    assert!(!text_of(&run.stderr).contains("git push"), "{}", text_of(&run.stderr));
}

#[test]
fn test_WB_BUILD_39_a_draft_change_the_session_wrote_and_did_not_commit_is_not_said() {
    let bench = bench();
    git(&bench.repo.path, &["checkout", "-q", "issue-99"]);
    with_line(&bench, "draft_change: 'draft-stand-in --head {branch}'");
    bench.session("echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file' && echo \"draft_change: 'evil {branch}'\" >> .watchbill/watchbill.yml\n");

    let run = bench.start_by(Path::new("/nonexistent/no-bwrap"), &["start", "build", "99"]);

    let err = text_of(&run.stderr);
    assert!(err.contains("git push origin issue-99"), "{}", err);
    assert!(err.contains("a person reads the change and commits it, or takes it back with git, before the next command"), "{}", err);
    assert!(!err.contains("evil") && !err.contains("draft-stand-in"), "{}", err);
}


// ---- WB-VIEW-17 to 19: the ignore rules of the checkout, and a long list left ----

/// Commits `text` as `path` on the branch of the bench, before a view is made from it.
fn commit_file(bench: &Bench, path: &str, text: &str) {
    let full = bench.repo.path.join(path);
    fs::create_dir_all(full.parent().unwrap()).unwrap();
    fs::write(&full, text).unwrap();
    git(&bench.repo.path, &["add", path]);
    git(&bench.repo.path, &["commit", "-q", "-m", &format!("Add {}", path)]);
}

#[test]
fn test_WB_VIEW_17_a_path_the_checkout_ignores_is_not_work_left_and_the_view_is_removed() {
    if !sandbox("test_WB_VIEW_17") {
        return;
    }
    let bench = bench();
    commit_file(&bench, ".gitignore", "node_modules/\n*.o\n");
    bench.session("mkdir -p src/node_modules/pkg && echo x > src/node_modules/pkg/index.js && echo y > src/brake.o\n");

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let err = text_of(&run.stderr);
    assert!(!err.contains("node_modules") && !err.contains("brake.o") && !err.contains("uncommitted"), "{}", err);
    assert!(bench.below("views").is_empty(), "{:?}", bench.below("views"));
    let back = read(&bench.lines().join("back"));
    assert!(!back.contains("uncommitted") && back.lines().any(|line| line == "view: removed"), "{}", back);
}

#[test]
fn test_WB_VIEW_18_a_gitignore_written_in_the_view_does_not_hide_a_path_the_checkout_does_not_ignore() {
    if !sandbox("test_WB_VIEW_18") {
        return;
    }
    let bench = bench();
    commit_file(&bench, ".gitignore", "node_modules/\n");
    bench.session("echo '*' > .gitignore\necho '*' > src/.gitignore\necho 'int fresh;' > src/new.c\nmkdir src/node_modules && echo x > src/node_modules/a\n");

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let err = text_of(&run.stderr);
    assert!(err.contains("src/new.c") && err.contains("uncommitted"), "{}", err);
    assert!(!err.contains("node_modules"), "{}", err);
    assert_eq!(bench.below("views").len(), 1);
}

#[test]
fn test_WB_VIEW_18_the_exclude_file_of_the_checkout_is_read_too() {
    if !sandbox("test_WB_VIEW_18") {
        return;
    }
    let bench = bench();
    fs::write(bench.repo.path.join(".git/info/exclude"), "scratch/\n").unwrap();
    bench.session("mkdir -p src/scratch && echo x > src/scratch/a\necho y > src/kept.txt\n");

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let err = text_of(&run.stderr);
    assert!(err.contains("src/kept.txt") && !err.contains("scratch"), "{}", err);
}

#[test]
fn test_WB_VIEW_19_more_than_20_paths_left_are_said_as_a_count_20_paths_and_the_place_of_the_list() {
    if !sandbox("test_WB_VIEW_19") {
        return;
    }
    let bench = bench();
    bench.session("mkdir src/many\ni=1\nwhile [ $i -le 25 ]; do echo $i > src/many/f$(printf %02d $i); i=$((i+1)); done\n");

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let err = text_of(&run.stderr);
    assert!(err.contains("25 paths"), "{}", err);
    assert!(err.contains("src/many/f01") && err.contains("src/many/f20"), "{}", err);
    assert!(!err.contains("src/many/f21") && !err.contains("src/many/f25"), "{}", err);
    let back = bench.lines().join("back");
    assert!(err.contains(back.to_str().unwrap()), "{}", err);
    let all = read(&back);
    assert_eq!(all.lines().filter(|line| line.starts_with("uncommitted: ")).count(), 25, "{}", all);
    assert!(all.contains("uncommitted: src/many/f25"), "{}", all);
}

// ---- WB-VIEW-20: how to look at the commits taken ----

#[test]
fn test_WB_VIEW_20_after_the_commits_are_taken_back_git_show_stat_is_said_for_each() {
    if !sandbox("test_WB_VIEW_20") {
        return;
    }
    let bench = bench();
    bench.session(
        "echo 'int a;' > src/brake.c && git commit -q -a -m 'First'\n\
         echo 'int b;' > src/new.c && git add src/new.c && git commit -q -m 'Second'\n",
    );

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let err = text_of(&run.stderr);
    let made = bench.git_text(&["rev-list", "--reverse", "HEAD~2..HEAD"]);
    let made: Vec<&str> = made.lines().collect();
    assert_eq!(made.len(), 2);
    for commit in made {
        assert!(err.contains(&format!("git show --stat {}", commit)), "{}", err);
    }
    let show = err.find("git show --stat").unwrap();
    let push = err.find("git push origin issue-54").unwrap();
    assert!(show < push, "{}", err);

    // A session that commits nothing gets none.
    let bench = self::bench();
    let run = bench.start(&["start", "build", "54"]);
    ok(&run);
    assert!(!text_of(&run.stderr).contains("git show"), "{}", text_of(&run.stderr));
}

// ---- WB-VIEW-18 (review of #166): rules only from the checkout's HEAD and exclude, and a notice ----

#[test]
fn test_WB_VIEW_18_the_view_s_own_exclude_and_excludesfile_do_not_hide_a_path() {
    if !sandbox("test_WB_VIEW_18") {
        return;
    }
    let bench = bench();
    bench.session(
        "echo 'int fresh;' > src/new.c\n\
         echo '*' > /tmp/all\n\
         echo 'src/new.c' >> .git/info/exclude\n\
         git config core.excludesFile /tmp/all\n",
    );

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let err = text_of(&run.stderr);
    assert!(err.contains("src/new.c") && err.contains("uncommitted"), "{}", err);
}

#[test]
fn test_WB_VIEW_18_the_person_s_global_ignore_file_is_not_read() {
    if !sandbox("test_WB_VIEW_18") {
        return;
    }
    let name = "test_WB_VIEW_18_the_person_s_global_ignore_file_is_not_read";
    // The variables of a process are the process's: the check runs in a child of this test binary, so that
    // no other test of it sees the person's files.
    let Some(person) = std::env::var_os("WB_TEST_GLOBAL_IGNORE") else {
        let person = folder_named("person");
        fs::create_dir_all(person.path.join(".config/git")).unwrap();
        fs::write(person.path.join(".config/git/ignore"), "*\n").unwrap();
        fs::write(person.path.join(".gitconfig"), "[core]\n\texcludesFile = ~/ignore-all\n").unwrap();
        fs::write(person.path.join("ignore-all"), "*\n").unwrap();
        let run = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name, "--nocapture", "--test-threads=1"])
            .env("WB_TEST_GLOBAL_IGNORE", &person.path)
            .output()
            .unwrap();
        assert!(run.status.success(), "{}{}", text(&run.stdout), text(&run.stderr));
        return;
    };
    let bench = bench();
    bench.session("echo 'int fresh;' > src/new.c\n");
    std::env::set_var("HOME", &person);
    std::env::set_var("XDG_CONFIG_HOME", Path::new(&person).join(".config"));

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    assert!(text_of(&run.stderr).contains("src/new.c"), "{}", text_of(&run.stderr));
}

#[test]
fn test_WB_VIEW_18_the_rules_are_those_of_head_not_of_the_working_tree_of_the_checkout() {
    if !sandbox("test_WB_VIEW_18") {
        return;
    }
    let bench = bench();
    commit_file(&bench, ".gitignore", "node_modules/\n");
    fs::write(bench.repo.path.join(".gitignore"), "*\n").unwrap();
    bench.session("echo 'int fresh;' > src/new.c\nmkdir src/node_modules && echo x > src/node_modules/a\n");

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let err = text_of(&run.stderr);
    assert!(err.contains("src/new.c") && !err.contains("node_modules"), "{}", err);
}

#[test]
fn test_WB_VIEW_18_an_exclude_file_that_is_passed_over_is_said_and_why() {
    if !sandbox("test_WB_VIEW_18") {
        return;
    }
    let bench = bench();
    let exclude = bench.repo.path.join(".git/info/exclude");
    fs::write(bench.repo.path.join(".git/elsewhere"), "scratch/\n").unwrap();
    let _ = fs::remove_file(&exclude);
    std::os::unix::fs::symlink("../elsewhere", &exclude).unwrap();
    bench.session("mkdir -p src/scratch && echo x > src/scratch/a\n");

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let err = text_of(&run.stderr);
    assert!(err.contains("info/exclude") && err.contains("not read") && err.contains("link"), "{}", err);

    // An exclude file that is not there is not said.
    let bench = self::bench();
    let _ = fs::remove_file(bench.repo.path.join(".git/info/exclude"));
    bench.session("echo x > src/kept.txt\n");
    let run = bench.start(&["start", "build", "54"]);
    ok(&run);
    assert!(!text_of(&run.stderr).contains("info/exclude"), "{}", text_of(&run.stderr));
}

// ---- WB-VIEW-21 and 22: a path from a view is said and written escaped (#167) ----

#[test]
fn test_WB_VIEW_22_a_path_with_a_line_break_is_one_escaped_line_of_back_and_the_last_line_stays() {
    if !sandbox("test_WB_VIEW_22") {
        return;
    }
    let bench = bench();
    bench.session("printf x > \"$(printf 'src/a\\nview: removed')\"\n");

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let back = read(&bench.lines().join("back"));
    assert!(back.lines().all(|line| line != "view: removed"), "{}", back);
    assert_eq!(back.lines().filter(|line| line.starts_with("uncommitted: ")).count(), 1, "{}", back);
    assert!(back.lines().any(|line| line == "uncommitted: src/a\\nview: removed"), "{}", back);
    assert!(back.lines().last().unwrap().starts_with("view: kept "), "{}", back);
}

#[test]
fn test_WB_VIEW_21_a_path_with_an_escape_code_is_said_on_standard_error_escaped() {
    if !sandbox("test_WB_VIEW_21") {
        return;
    }
    let bench = bench();
    bench.session("printf x > \"$(printf 'src/e\\033[31mred')\"\n");

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let err = text_of(&run.stderr);
    assert!(!err.contains('\u{1b}'), "{:?}", err);
    assert!(err.contains("src/e\\u{1b}[31mred"), "{}", err);
    let back = read(&bench.lines().join("back"));
    assert!(!back.contains('\u{1b}') && back.contains("uncommitted: src/e\\u{1b}[31mred"), "{:?}", back);
}

#[test]
fn test_WB_VIEW_21_a_commit_refused_for_a_path_with_a_line_break_names_the_path_escaped() {
    if !sandbox("test_WB_VIEW_21") {
        return;
    }
    let bench = bench();
    bench.session("mkdir docs\nf=\"$(printf 'docs/a\\nrefused: forged')\"\nprintf x > \"$f\" && git add -A && git commit -q -m 'Outside'\n");

    let run = bench.start(&["start", "build", "54"]);

    assert_eq!(run.status.code(), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(err.contains("docs/a\\nrefused: forged is outside `changes`"), "{}", err);
    assert!(err.lines().all(|line| !line.starts_with("refused: forged")), "{}", err);
    let back = read(&bench.lines().join("back"));
    assert!(back.contains("docs/a\\nrefused: forged is outside `changes`"), "{}", back);
    assert!(back.lines().all(|line| !line.starts_with("refused: forged")), "{}", back);
}

// ---- #167, after the review: more to escape, and a test for the subject ----

#[test]
fn test_WB_VIEW_21_a_line_separator_and_a_bidi_control_in_a_path_are_escaped() {
    if !sandbox("test_WB_VIEW_21") {
        return;
    }
    let bench = bench();
    bench.session("printf x > \"$(printf 'src/s\\342\\200\\250view: removed')\"\nprintf x > \"$(printf 'src/b\\342\\200\\256gpj.c')\"\n");

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let err = text_of(&run.stderr);
    let back = read(&bench.lines().join("back"));
    for text in [&err, &back] {
        assert!(!text.contains('\u{2028}') && !text.contains('\u{202e}'), "{:?}", text);
        assert!(text.contains("src/s\\u{2028}view: removed") && text.contains("src/b\\u{202e}gpj.c"), "{:?}", text);
    }
}

#[test]
fn test_WB_VIEW_21_a_backslash_in_a_path_is_escaped_so_it_differs_from_a_line_break() {
    if !sandbox("test_WB_VIEW_21") {
        return;
    }
    let bench = bench();
    bench.session("printf x > 'src/c\\nview'\nprintf x > \"$(printf 'src/c\\nview')\"\n");

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let back = read(&bench.lines().join("back"));
    assert!(back.lines().any(|line| line == "uncommitted: src/c\\\\nview"), "{}", back);
    assert!(back.lines().any(|line| line == "uncommitted: src/c\\nview"), "{}", back);
}

#[test]
fn test_WB_VIEW_21_the_failed_line_is_written_escaped() {
    let back = watchbill::view::Back { failed: Some("it said\u{1b}[2J\rview: removed\u{2028}x".to_string()), ..Default::default() };
    let text = back.text(None);
    assert!(text.lines().any(|line| line == "failed: it said\\u{1b}[2J\\rview: removed\\u{2028}x"), "{:?}", text);
    assert_eq!(text.lines().filter(|line| *line == "view: removed").count(), 1, "{:?}", text);
}

#[test]
fn test_WB_VIEW_21_a_commit_subject_with_control_characters_is_escaped_taken_and_refused() {
    if !sandbox("test_WB_VIEW_21") {
        return;
    }
    let bench = bench();
    bench.session("echo 'int a;' > src/brake.c && git commit -q -a -m \"$(printf 'Sub\\033[2J\\rview: removed')\"\n");
    let run = bench.start(&["start", "build", "54"]);
    ok(&run);
    let err = text_of(&run.stderr);
    let back = read(&bench.lines().join("back"));
    for text in [&err, &back] {
        assert!(!text.contains('\u{1b}') && !text.contains('\r'), "{:?}", text);
        assert!(text.contains("Sub\\u{1b}[2J\\rview: removed"), "{:?}", text);
    }

    let bench = self::bench();
    bench.session("echo 'int b;' > src/brake.h && git commit -q -a -m \"$(printf 'Ref\\033[31m\\rX')\"\n");
    let run = bench.start(&["start", "build", "54"]);
    assert_eq!(run.status.code(), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    let back = read(&bench.lines().join("back"));
    for text in [&err, &back] {
        assert!(!text.contains('\u{1b}') && !text.contains('\r'), "{:?}", text);
        assert!(text.contains("Ref\\u{1b}[31m\\rX"), "{:?}", text);
    }
}

// ---- WB-BUILD-30 to 35: a local change to a file of the view stops the start (#172) ----

/// What a start refused for a local change said, and that nothing was posted, started or made.
fn guarded(bench: &Bench, run: &Output) -> String {
    assert_eq!(code(run), Some(1), "{}", text_of(&run.stderr));
    assert!(!bench.any_posted() && !bench.started(), "{:?}", bench.calls());
    assert!(bench.below("views").is_empty());
    text_of(&run.stderr)
}

/// The guard comes before the view, so that it is made where no bubblewrap is.
fn start_unsandboxed(bench: &Bench, args: &[&str]) -> Output {
    bench.start_by(Path::new("/nonexistent/no-bwrap"), args)
}

#[test]
fn test_WB_BUILD_30_a_changed_file_under_reads_is_refused_with_1_named_and_nothing_is_posted() {
    let bench = bench();
    put(&bench.repo.path, "src/brake.h", "int brake(int);\n");
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "build", "54"]));
    assert!(err.contains("src/brake.h") && err.contains("guard"), "{}", err);
    assert!(!err.contains("bubblewrap"), "{}", err);
    // A file in a folder of `reads` is named as well.
    let bench = self::bench();
    put(&bench.repo.path, "include/a.h", "#define A 3\n");
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "build", "54"]));
    assert!(err.contains("include/a.h"), "{}", err);
}

#[test]
fn test_WB_BUILD_30_a_staged_change_and_a_deleted_file_are_each_named() {
    let bench = bench();
    put(&bench.repo.path, "src/brake.h", "int brake(int);\n");
    git(&bench.repo.path, &["add", "--", "src/brake.h"]);
    fs::remove_file(bench.repo.path.join("include/b.h")).unwrap();
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "build", "54"]));
    assert!(err.contains("src/brake.h") && err.contains("include/b.h"), "{}", err);
}

#[test]
fn test_WB_BUILD_30_a_file_that_is_not_of_the_view_is_no_local_change_and_neither_is_a_role_with_no_view() {
    let bench = bench();
    put(&bench.repo.path, "src/secret.c", "changed here\n");
    put(&bench.repo.path, "include/untracked.h", "#define U 1\n");
    // The start goes on to the view, which is where no bubblewrap is.
    let err = text_of(&start_unsandboxed(&bench, &["start", "build", "54", "--ignore-local"]).stderr);
    assert!(err.contains("bubblewrap") && !err.contains("guard") && !err.contains("src/secret.c"), "{}", err);
    // A role with no issue's file runs without a view, and is not checked.
    put(&bench.repo.path, "src/brake.h", "int brake(int);\n");
    ok(&start_unsandboxed(&bench, &["start", "build", "99"]));
    assert!(bench.started());
}

#[test]
fn test_WB_BUILD_31_an_untracked_file_under_a_path_of_changes_is_refused_with_1_and_named() {
    let bench = bench();
    put(&bench.repo.path, "src/new.c", "int fresh;\n");
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "build", "54"]));
    assert!(err.contains("src/new.c"), "{}", err);
    // Under a folder of `changes`, and not under a folder of `reads`.
    let file = ISSUE_54.replace("changes: [src/brake.c, tests/test_brake.c, src/new.c]", "changes: [src/brake.c, tests]");
    let bench = bench_for(&file);
    put(&bench.repo.path, "tests/test_new.c", "int main(void) { return 1; }\n");
    put(&bench.repo.path, "include/untracked.h", "#define U 1\n");
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "build", "54", "--ignore-local"]));
    assert!(err.contains("tests/test_new.c") && !err.contains("untracked.h"), "{}", err);
}

#[test]
fn test_WB_PROMPT_64_the_level_and_the_view_of_the_stamp_are_those_the_start_posts_for_the_same_head() {
    if !sandbox("test_WB_PROMPT_64") {
        return;
    }
    let bench = bench();
    let before = bench.head();
    let printed = prompt_with(&bench.repo.path, &bench.organisation(), &["build", "54", "--round", "1", "--stamp"]);
    ok(&printed);
    let printed = text(&printed.stdout);
    ok(&bench.start(&["start", "build", "54"]));
    assert!(bench.started() && bench.any_posted());
    let posted = bench.posted();
    assert_eq!(bench.head(), before);
    assert!(line_of(&printed, "level").is_some() && line_of(&printed, "view").is_some(), "{}", printed);
    assert_eq!(line_of(&posted, "level"), line_of(&printed, "level"), "{}\n{}", posted, printed);
    assert_eq!(line_of(&posted, "view"), line_of(&printed, "view"), "{}\n{}", posted, printed);
}

#[test]
fn test_WB_BUILD_32_ignore_local_lets_a_change_under_reads_go_on_and_the_stamp_and_the_lines_say_so() {
    if !sandbox("test_WB_BUILD_32") {
        return;
    }
    let bench = bench();
    put(&bench.repo.path, "src/brake.h", "int brake(int);\n");
    ok(&bench.start(&["start", "build", "54", "--ignore-local"]));
    assert!(bench.started() && bench.any_posted());
    let posted = bench.posted();
    assert_eq!(line_of(&posted, "local").as_deref(), Some("ignored 1"), "{}", posted);
    let names: Vec<&str> = posted.lines().map(|line| line.split(':').next().unwrap()).collect();
    let view = names.iter().position(|name| *name == "view").unwrap();
    // The line `level:` follows the view (#209), and `local:` follows it.
    assert_eq!(names[view + 1], "level", "{}", posted);
    assert_eq!(names[view + 2], "local", "{}", posted);
    // The view's list is the same: the flag changes nothing of what the session sees.
    assert_eq!(line_of(&posted, "view"), Some(sha256(BUILD_LIST)), "{}", posted);
    assert_eq!(read(&bench.lines().join("local")), "src/brake.h\n");
}

#[test]
fn test_WB_BUILD_34_with_no_local_change_the_stamp_has_no_local_line_and_the_lines_no_local_file() {
    if !sandbox("test_WB_BUILD_34") {
        return;
    }
    let bench = bench();
    ok(&bench.start(&["start", "build", "54"]));
    assert_eq!(line_of(&bench.posted(), "local"), None, "{}", bench.posted());
    assert!(!bench.lines().join("local").exists());
    // The flag with nothing to ignore changes nothing.
    let bench = self::bench();
    ok(&bench.start(&["start", "build", "54", "--ignore-local"]));
    assert_eq!(line_of(&bench.posted(), "local"), None, "{}", bench.posted());
    assert!(!bench.lines().join("local").exists());
}

#[test]
fn test_WB_BUILD_33_a_change_under_changes_is_refused_also_with_ignore_local_saying_it_cannot_be_overridden() {
    let bench = bench();
    put(&bench.repo.path, "src/brake.c", "int brake(void) { return 2; }\n");
    for words in [&["start", "build", "54"][..], &["start", "build", "54", "--ignore-local"][..]] {
        let err = guarded(&bench, &start_unsandboxed(&bench, words));
        assert!(err.contains("src/brake.c"), "{}", err);
        if words.len() > 3 {
            assert!(err.contains("cannot be overridden"), "{}", err);
        }
    }
    // With a change under `reads` as well, the flag passes neither.
    put(&bench.repo.path, "src/brake.h", "int brake(int);\n");
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "build", "54", "--ignore-local"]));
    assert!(err.contains("src/brake.c") && err.contains("src/brake.h") && err.contains("cannot be overridden"), "{}", err);
}

#[test]
fn test_WB_BUILD_35_the_refusal_says_it_is_a_guard_names_the_paths_with_their_list_and_the_flag() {
    let bench = bench();
    put(&bench.repo.path, "src/brake.h", "int brake(int);\n");
    put(&bench.repo.path, "src/brake.c", "int brake(void) { return 2; }\n");
    let run = start_unsandboxed(&bench, &["start", "build", "54"]);
    let err = guarded(&bench, &run);
    assert!(err.contains("guard") && err.contains("--ignore-local"), "{}", err);
    assert!(err.lines().any(|line| line.contains("changes") && line.contains("src/brake.c")), "{}", err);
    assert!(err.lines().any(|line| line.contains("reads") && line.contains("src/brake.h")), "{}", err);
    // The outcome of the lines is a refusal at a check of its own.
    let outcome = read(&bench.lines().join("outcome"));
    assert!(outcome.contains("outcome: refused\ncheck: local\nstatus: 1"), "{}", outcome);
}

#[test]
fn test_WB_BUILD_35_a_path_with_a_line_break_is_named_escaped() {
    let bench = bench();
    put(&bench.repo.path, "include/a\nb.h", "#define A 3\n");
    git(&bench.repo.path, &["add", "--", "include/a\nb.h"]);
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "build", "54"]));
    assert!(err.contains("include/a\\nb.h") && !err.contains("a\nb.h"), "{}", err);
}

// ---- #192: a commit is taken only as it was made ----

/// A session that commits, by git's plumbing, a tree made of `HEAD`'s with the folder `parent/<name>` holding
/// the file `x`: `git add` would not take such a path, and a session need not use it.
fn plumbed(parent: &str, name: &str, message: &str) -> String {
    let nested = if parent.is_empty() {
        format!("root=$(git ls-tree HEAD; printf '040000 tree %s\\t{name}\\n' \"$g\" | cat)\n")
    } else {
        format!(
            "top=$(git ls-tree HEAD:{parent}; printf '040000 tree %s\\t{name}\\n' \"$g\" | cat)\n\
             t=$(printf '%s\\n' \"$top\" | git mktree)\n\
             root=$(git ls-tree HEAD | sed '/\\t{parent}$/d'; printf '040000 tree %s\\t{parent}\\n' \"$t\")\n"
        )
    };
    format!(
        "b=$(echo x | git hash-object -w --stdin)\n\
         g=$(printf '100644 blob %s\\tx\\n' \"$b\" | git mktree)\n\
         {nested}\
         tree=$(printf '%s\\n' \"$root\" | git mktree)\n\
         c=$(git commit-tree \"$tree\" -p HEAD -m '{message}')\n\
         git update-ref refs/heads/issue-54 \"$c\"\n"
    )
}

#[test]
fn test_WB_VIEW_33_a_path_under_a_file_entry_of_changes_is_refused_named_and_nothing_is_put_on_the_branch() {
    if !sandbox("test_WB_VIEW_33") {
        return;
    }
    // `src/new.c` is not in HEAD and does not end with `/`: a file.
    let bench = bench();
    let head = bench.head();
    bench.session("mkdir src/new.c && echo x > src/new.c/x && git add src/new.c/x && git commit -q -m 'Under a new file'\n");
    let run = bench.start(&["start", "build", "54"]);
    refused_back(&bench, &run, &head, &["src/new.c/x", "Under a new file", "outside `changes`"]);

    // `src/brake.c` is a file in HEAD: it is taken out and a folder put in its place.
    let bench = self::bench();
    let head = bench.head();
    bench.session("git rm -q src/brake.c && mkdir src/brake.c && echo x > src/brake.c/x && git add src/brake.c/x && git commit -q -m 'A folder for a file'\n");
    let run = bench.start(&["start", "build", "54"]);
    refused_back(&bench, &run, &head, &["src/brake.c/x", "A folder for a file"]);
}

const FOLDERS: &str = "issue: '54'
from: 3f0c1d9e
requirements: []
interface: |
  none
changes: [src, tests/test_brake.c, docs/, notes]
reads:
  tester: [src/brake.h, include]
  build: [src/brake.h, include]
  review: [src/brake.h]
  fix: [src/brake.h]
existing_tests_changed: []
questions: []
";

#[test]
fn test_WB_VIEW_33_a_folder_in_head_and_a_new_path_ending_in_a_slash_cover_the_paths_under_them() {
    if !sandbox("test_WB_VIEW_33") {
        return;
    }
    let bench = bench_for(FOLDERS);
    let head = bench.head();
    bench.session(
        "mkdir -p src/a docs/deep && echo 'int b;' > src/a/b.c && echo x > docs/deep/y.md\n\
         git add src/a/b.c docs/deep/y.md && git commit -q -m 'Under folders'\n",
    );
    ok(&bench.start(&["start", "build", "54"]));
    assert_eq!(bench.git_text(&["log", "--format=%s", &format!("{}..HEAD", head)]), "Under folders\n");
    assert_eq!(read(&bench.repo.path.join("src/a/b.c")), "int b;\n");
    assert_eq!(read(&bench.repo.path.join("docs/deep/y.md")), "x\n");
}

#[test]
fn test_WB_VIEW_33_a_new_path_without_a_slash_is_a_file_and_covers_itself_only() {
    if !sandbox("test_WB_VIEW_33") {
        return;
    }
    let bench = bench_for(FOLDERS);
    let head = bench.head();
    bench.session("echo x > notes && git add notes && git commit -q -m 'The file'\nmkdir -p other && echo y > other/z && git add other/z && git commit -q -m 'Elsewhere'\n");
    let run = bench.start(&["start", "build", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert_eq!(bench.git_text(&["log", "--format=%s", &format!("{}..HEAD", head)]), "The file\n");

    let bench = bench_for(FOLDERS);
    let head = bench.head();
    bench.session("rm -f notes && mkdir notes && echo x > notes/z && git add notes/z && git commit -q -m 'Below a new file'\n");
    let run = bench.start(&["start", "build", "54"]);
    refused_back(&bench, &run, &head, &["notes/z", "Below a new file"]);
}

#[test]
fn test_WB_VIEW_34_a_path_with_a_part_dot_git_in_any_case_is_refused_named_and_nothing_is_put_on_the_branch() {
    if !sandbox("test_WB_VIEW_34") {
        return;
    }
    for (parent, name, path) in [("src", "new.c", "src/new.c/.git/x"), ("", ".GIT", ".GIT/x"), ("src", ".Git", "src/.Git/x")] {
        let bench = bench_for(FOLDERS);
        let head = bench.head();
        let body = if parent == "src" && name == "new.c" {
            // `.git` below the new file `src/new.c`, built by hand: a folder `src/new.c` with a folder `.git`.
            "b=$(echo x | git hash-object -w --stdin)\n\
             g=$(printf '100644 blob %s\\tx\\n' \"$b\" | git mktree)\n\
             n=$(printf '040000 tree %s\\t.git\\n' \"$g\" | git mktree)\n\
             top=$(git ls-tree HEAD:src; printf '040000 tree %s\\tnew.c\\n' \"$n\")\n\
             t=$(printf '%s\\n' \"$top\" | git mktree)\n\
             root=$(git ls-tree HEAD | sed '/\\tsrc$/d'; printf '040000 tree %s\\tsrc\\n' \"$t\")\n\
             tree=$(printf '%s\\n' \"$root\" | git mktree)\n\
             c=$(git commit-tree \"$tree\" -p HEAD -m 'A git folder')\n\
             git update-ref refs/heads/issue-54 \"$c\"\n"
                .to_string()
        } else {
            plumbed(parent, name, "A git folder")
        };
        bench.session(&body);
        let run = bench.start(&["start", "build", "54"]);
        refused_back(&bench, &run, &head, &[path, "A git folder", "git does not take"]);
    }
}

#[test]
fn test_WB_VIEW_34_a_path_git_refuses_is_said_escaped_and_the_rest_is_decided_by_one_rule() {
    use watchbill::view::refused_by_git as refused;
    for path in [
        ".git",
        "a/.git",
        "a/.git/b",
        ".GIT/x",
        "a/.Git/x",
        "a/.gIT",
        // NTFS forms: trailing dots and spaces, the short name, a stream, a backslash as a separator.
        "a/.git./x",
        "a/.git /x",
        "a/.git. ./x",
        "a/git~1/x",
        "a/GIT~1",
        "a/.git::$INDEX_ALLOCATION/x",
        "a\\.git\\x",
        // HFS forms: ignorable code points inside the name.
        "a/.g\u{200c}it/x",
        "a/.git\u{feff}/x",
        "a/.\u{202e}git/x",
        // Not a path at all.
        "a//b",
        "/a",
        "a/",
        "a/./b",
        "a/../b",
        "..",
    ] {
        assert!(refused(path, "100644").is_some(), "{:?} is taken", path);
    }
    for path in ["a/.gitignore", "a/.github/x", "a/git/x", "a/git~2/x", "a/.gi/x", "a/x.git/y", "a/b.git", ".gitattributes", "a/.gitmodules", "a/.git-keep", "a/gitmod~1/x"] {
        assert!(refused(path, "100644").is_none(), "{:?} is refused", path);
    }
    // `.gitmodules` as a link, in the forms git knows.
    for path in [".gitmodules", "a/.GITMODULES", "a/gitmod~1", "a/GI7EBA~1", "a/.gitmodules ", "a/.git\u{200d}modules"] {
        assert!(refused(path, "120000").is_some(), "{:?} is taken as a link", path);
        assert!(refused(path, "100644").is_none() || path.contains('\u{200d}'), "{:?} is refused as a file", path);
    }
    assert!(refused("a/.gitmodules", "100644").is_none());
    assert!(refused("a/.gitmodules", "120000").is_some());
}

#[test]
fn test_WB_VIEW_35_a_commit_whose_taken_tree_differs_from_the_view_s_is_refused_named_and_not_put_on_the_branch() {
    if !sandbox("test_WB_VIEW_35") {
        return;
    }
    // A tree that names `src/new.c` twice, as git allows in a tree and never in an index: put on the branch,
    // the commit would hold one of the two, and so not be the one made.
    let odd = "b=$(echo 'int fresh;' | git hash-object -w --stdin)\n\
               d=$(echo 'int other;' | git hash-object -w --stdin)\n\
               one=$(git ls-tree HEAD:src; printf '100644 blob %s\\tnew.c\\n' \"$b\")\n\
               one=$(printf '%s\\n' \"$one\" | git mktree)\n\
               two=$(printf '100644 blob %s\\tnew.c\\n' \"$d\" | git mktree)\n\
               s=$( { git cat-file tree \"$one\"; git cat-file tree \"$two\"; } | git hash-object -t tree -w --literally --stdin)\n\
               root=$(git ls-tree HEAD | sed '/\\tsrc$/d'; printf '040000 tree %s\\tsrc\\n' \"$s\")\n\
               tree=$(printf '%s\\n' \"$root\" | git mktree)\n\
               c=$(git commit-tree \"$tree\" -p HEAD -m 'A name twice')\n\
               git update-ref refs/heads/issue-54 \"$c\"\n";
    let bench = bench();
    let head = bench.head();
    bench.session(odd);
    let run = bench.start(&["start", "build", "54"]);
    refused_back(&bench, &run, &head, &["A name twice"]);

    // The commits before it are taken, as before a commit refused for its paths.
    let bench = self::bench();
    let head = bench.head();
    bench.session(&format!("echo 'int a;' > src/brake.c && git commit -q -a -m 'First, kept'\n{}", odd));
    let run = bench.start(&["start", "build", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("A name twice"), "{}", text_of(&run.stderr));
    assert_eq!(bench.git_text(&["log", "--format=%s", &format!("{}..HEAD", head)]), "First, kept\n");
}

// ---- #192, after the review ----

const SLASHED: &str = "issue: '54'
from: 3f0c1d9e
requirements: []
interface: |
  none
changes: [src/brake.c/, tests/test_brake.c]
reads:
  tester: [src/brake.h, include]
  build: [src/brake.h, include]
  review: [src/brake.h]
  fix: [src/brake.h]
existing_tests_changed: []
questions: []
";

#[test]
fn test_WB_VIEW_33_an_entry_that_is_a_file_in_head_covers_that_path_only_also_when_written_with_a_slash() {
    if !sandbox("test_WB_VIEW_33") {
        return;
    }
    let bench = bench_for(SLASHED);
    let head = bench.head();
    bench.session("git rm -q src/brake.c && mkdir src/brake.c && echo x > src/brake.c/x && git add src/brake.c/x && git commit -q -m 'A folder for a file'\n");
    let run = bench.start(&["start", "build", "54"]);
    refused_back(&bench, &run, &head, &["src/brake.c/x", "A folder for a file"]);
}

/// A session that commits the tree of `HEAD` with the tree `src` made by `body` (a shell text that sets `s`).
fn odd_src(body: &str, message: &str) -> String {
    format!(
        "{body}root=$(git ls-tree HEAD | sed '/\\tsrc$/d'; printf '040000 tree %s\\tsrc\\n' \"$s\")\n\
         tree=$(printf '%s\\n' \"$root\" | git mktree)\n\
         c=$(git commit-tree \"$tree\" -p HEAD -m '{message}')\n\
         git update-ref refs/heads/issue-54 \"$c\"\n"
    )
}

#[test]
fn test_WB_VIEW_35_a_tree_with_a_name_twice_one_copy_as_in_head_is_refused_named_and_not_put_on_the_branch() {
    if !sandbox("test_WB_VIEW_35") {
        return;
    }
    // `brake.c` twice: the first as it is in HEAD, the second new. `diff-tree` shows one change.
    let body = "b=$(echo 'int other;' | git hash-object -w --stdin)\n\
                one=$(git ls-tree HEAD:src)\n\
                one=$(printf '%s\\n' \"$one\" | git mktree)\n\
                two=$(printf '100644 blob %s\\tbrake.c\\n' \"$b\" | git mktree)\n\
                s=$( { git cat-file tree \"$one\"; git cat-file tree \"$two\"; } | git hash-object -t tree -w --literally --stdin)\n";
    let bench = bench();
    let head = bench.head();
    bench.session(&odd_src(body, "Brake twice"));
    let run = bench.start(&["start", "build", "54"]);
    refused_back(&bench, &run, &head, &["Brake twice"]);
}

#[test]
fn test_WB_VIEW_35_a_tree_with_a_mode_git_rewrites_is_refused_named_and_not_put_on_the_branch() {
    if !sandbox("test_WB_VIEW_35") {
        return;
    }
    let body = "b=$(echo 'int fresh;' | git hash-object -w --stdin)\n\
                s=$(git ls-tree HEAD:src; printf '100644 blob %s\\tnew.c\\n' \"$b\")\n\
                s=$(printf '%s\\n' \"$s\" | git mktree)\n\
                s=$(git cat-file tree \"$s\" | LC_ALL=C sed 's/100644 new\\.c/100664 new.c/' | git hash-object -t tree -w --literally --stdin)\n";
    let bench = bench();
    let head = bench.head();
    bench.session(&odd_src(body, "A mode git rewrites"));
    let run = bench.start(&["start", "build", "54"]);
    refused_back(&bench, &run, &head, &["A mode git rewrites"]);
}

#[test]
fn test_WB_VIEW_34_the_short_forms_ntfs_gives_gitmodules_are_refused_as_a_link() {
    use watchbill::view::refused_by_git as refused;
    for name in [
        "gitmod~1", "GITMOD~2", "gitmod~3", "gitmod~4", "gitmod~1 ", "gitmod~1.", "gitmod~1::$DATA",
        // The fall-back short names, 8 characters with the prefix `gi7eba`.
        "gi7eba~1", "gi7eba~9", "gi7eb~12", "gi7e~123", "gi7~1234", "gi~12345", "g~123456", "~1234567", "GI7EB~12", "g~123456 ",
    ] {
        let path = format!("a/{}", name);
        assert!(refused(&path, "120000").is_some(), "{:?} is taken as a link", path);
        assert!(refused(&path, "100644").is_none(), "{:?} is refused as a file", path);
    }
    for name in ["gitmod~5", "gitmod~0", "gitmod~10", "gi7eba~", "gi7eba~a", "gi7ebb~1", "gi7eb~1a", "g~12345", "g~1234567", "~12345678x", "gitmo~12", "gi7eba~12"] {
        let path = format!("a/{}", name);
        assert!(refused(&path, "120000").is_none(), "{:?} is refused as a link", path);
    }
}

// ---- #172, after the review: renames, the index, ignored files, assume-unchanged ----

#[test]
fn test_WB_BUILD_33_a_rename_from_changes_to_reads_names_both_paths_and_is_refused_also_with_ignore_local() {
    let bench = bench();
    git(&bench.repo.path, &["mv", "src/brake.c", "include/c.h"]);
    for words in [&["start", "build", "54"][..], &["start", "build", "54", "--ignore-local"][..]] {
        let err = guarded(&bench, &start_unsandboxed(&bench, words));
        assert!(err.contains("src/brake.c") && err.contains("include/c.h"), "{}", err);
        assert!(err.lines().any(|line| line.contains("changes") && line.contains("src/brake.c")), "{}", err);
        assert!(err.contains("cannot be overridden"), "{}", err);
    }
}

#[test]
fn test_WB_BUILD_30_a_difference_in_the_index_only_is_named() {
    // A file staged and then deleted from the working tree, and a change staged and then put back.
    let bench = bench();
    put(&bench.repo.path, "include/x.h", "#define X 1\n");
    git(&bench.repo.path, &["add", "--", "include/x.h"]);
    fs::remove_file(bench.repo.path.join("include/x.h")).unwrap();
    put(&bench.repo.path, "src/brake.h", "int brake(int);\n");
    git(&bench.repo.path, &["add", "--", "src/brake.h"]);
    put(&bench.repo.path, "src/brake.h", "int brake(void);\n");
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "build", "54"]));
    assert!(err.contains("include/x.h") && err.contains("src/brake.h"), "{}", err);
}

#[test]
fn test_WB_BUILD_31_an_ignored_file_under_changes_is_named_by_the_rules_of_the_checkout_and_of_the_person() {
    // Ignored by the checkout's own `.git/info/exclude`.
    let bench = bench();
    put(&bench.repo.path, ".git/info/exclude", "src/new.c\n");
    put(&bench.repo.path, "src/new.c", "int fresh;\n");
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "build", "54"]));
    assert!(err.contains("src/new.c"), "{}", err);
    // Ignored by the person's global file only: git would write over it too.
    let bench = self::bench();
    let global = bench.at("global-ignore");
    fs::write(&global, "src/new.c\n").unwrap();
    git(&bench.repo.path, &["config", "core.excludesFile", global.to_str().unwrap()]);
    put(&bench.repo.path, "src/new.c", "int fresh;\n");
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "build", "54", "--ignore-local"]));
    assert!(err.contains("src/new.c"), "{}", err);
    // An ignored file under `reads` is not in the view, and is not named.
    let bench = self::bench();
    put(&bench.repo.path, ".git/info/exclude", "include/o.h\n");
    put(&bench.repo.path, "include/o.h", "#define O 1\n");
    let err = text_of(&start_unsandboxed(&bench, &["start", "build", "54"]).stderr);
    assert!(!err.contains("guard") && err.contains("bubblewrap"), "{}", err);
}

#[test]
fn test_WB_BUILD_30_a_changed_file_marked_assume_unchanged_or_skip_worktree_is_named_and_an_unchanged_one_is_not() {
    let bench = bench();
    git(&bench.repo.path, &["update-index", "--assume-unchanged", "src/brake.h"]);
    git(&bench.repo.path, &["update-index", "--skip-worktree", "src/brake.c"]);
    // Marked and not changed: nothing to name.
    let err = text_of(&start_unsandboxed(&bench, &["start", "build", "54"]).stderr);
    assert!(!err.contains("guard") && err.contains("bubblewrap"), "{}", err);
    put(&bench.repo.path, "src/brake.h", "int brake(int);\n");
    put(&bench.repo.path, "src/brake.c", "int brake(void) { return 2; }\n");
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "build", "54", "--ignore-local"]));
    assert!(err.lines().any(|line| line.contains("changes") && line.contains("src/brake.c")), "{}", err);
    assert!(err.contains("src/brake.h") && err.contains("cannot be overridden"), "{}", err);
}

#[test]
fn test_WB_BUILD_35_a_long_list_is_said_as_the_first_20_and_a_count() {
    let bench = bench();
    for number in 0..25 {
        put(&bench.repo.path, &format!("include/f{:02}.h", number), "#define F 1\n");
        git(&bench.repo.path, &["add", "--", &format!("include/f{:02}.h", number)]);
    }
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "build", "54"]));
    assert!(err.contains("include/f19.h") && !err.contains("include/f20.h") && err.contains("5 more"), "{}", err);
}

// ---- #172, second look: an empty view has no paths to guard ----

#[test]
fn test_WB_BUILD_30_a_view_with_no_paths_checks_nothing_and_a_changed_file_elsewhere_does_not_stop_it() {
    let file = ISSUE_54
        .replace("changes: [src/brake.c, tests/test_brake.c, src/new.c]", "changes: []")
        .replace("  tester: [src/brake.h, include]\n  build: [src/brake.h, include]\n  review: [src/brake.h]\n  fix: [src/brake.h]\n", "  tester: []\n  build: []\n  review: []\n  fix: []\n");
    let bench = bench_for(&file);
    put(&bench.repo.path, "src/secret.c", "changed here\n");
    put(&bench.repo.path, "src/untracked.c", "int u;\n");
    // It goes on to the view, which is where no bubblewrap is; with the flag too, and nothing is counted.
    for words in [&["start", "build", "54"][..], &["start", "build", "54", "--ignore-local"][..]] {
        let err = text_of(&start_unsandboxed(&bench, words).stderr);
        assert!(!err.contains("guard") && !err.contains("src/secret.c") && err.contains("bubblewrap"), "{}", err);
    }
}

// ---- WB-BUILD-54 to 56, WB-MON-9: a hangup or a termination while the session runs (#221) ----
//
// A signal is sent to a process the test started, by its number. `wb` is run as the test binary itself, run
// again with only `a_helper_that_runs_wb_for_the_signal_tests_and_checks_nothing` in it, so that it can be signalled without ending this
// one. The `bwrap` is a stand-in that runs the session in the view, so these tests hold where bubblewrap cannot
// make a sandbox, and as root.

extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
    fn signal(signal: i32, handler: usize) -> usize;
}

const SIGHUP: i32 = 1;
const SIGTERM: i32 = 15;
const CHILD_ENV: &str = "WB_SIGNAL_CHILD_FOLDER";
const CHILD_SESSION: &str = "WB_SIGNAL_CHILD_SESSION";

/// A `bwrap` that makes no sandbox: it answers the probe, and else runs the words in the view.
fn stand_in_bwrap(bench: &Bench) -> PathBuf {
    let path = bench.at("bwrap");
    script(
        &path,
        "view=\nwhile [ $# -gt 0 ]; do\n  case \"$1\" in\n    --bind) view=$2; shift 3;;\n    --) shift; break;;\n    *) shift;;\n  esac\ndone\n[ \"$1\" = /usr/bin/true ] && exit 0\ncd \"$view\" && exec \"$@\"\n",
    );
    path
}

/// What the session of the signal tests does: it commits a file in its view, says its number, and waits.
const WAITS: &str = "echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\necho $$ > {log}/pid\nexec sleep 30\n";

/// The same, but the session ends at a termination and says that it got it.
const TRAPS: &str = "echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\ntrap 'echo got > {log}/got; kill $!; exit 0' TERM\nsleep 30 &\necho $$ > {log}/pid\nwait\n";

/// A session that commits, then sends terminations to `wb` every few milliseconds from a process of its own,
/// and ends at the first one it gets: the rest come while `wb` takes the commit back (#221, WB-BUILD-57). The
/// sender stops at 1000, once the lines have an `outcome`, or once the process is no longer the test's helper, so
/// it never signals a process of anyone else.
const STORM: &str = "echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\nwb=$PPID\ntrap 'exit 0' TERM\n(n=0; while [ $n -lt 1000 ] && ! ls {state}/watchbill/sessions/*/outcome >/dev/null 2>&1 && grep -qa a_helper_that_runs_wb /proc/$wb/cmdline 2>/dev/null && kill -TERM $wb 2>/dev/null; do n=$((n+1)); sleep 0.005; done) >/dev/null 2>&1 &\necho $$ > {log}/pid\nsleep 5 &\nwait\n";

// ---- #257: in a view the signal reaches the session inside bubblewrap ----
//
// These sessions run in the real bubblewrap. Each one that ends at a signal takes a second before it writes
// `got`, as a session does that ends cleanly: a session that is killed under it writes nothing.

/// Which `bwrap` the helper uses: `real`, `run` (the real one, through `view::run` with no controlling
/// terminal), or else the stand-in.
const CHILD_SANDBOX: &str = "WB_SIGNAL_CHILD_SANDBOX";

/// A session that commits, and at a termination takes a second, writes `got` and ends by the termination.
const CLEANLY_TERM: &str = "echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\ntrap 'sleep 1; echo got > {log}/got; kill $!; trap - TERM; kill -TERM $$' TERM\nsleep 30 &\necho $$ > {log}/pid\nwait\n";

/// The same at a hangup.
const CLEANLY_HUP: &str = "echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\ntrap 'sleep 1; echo got > {log}/got; kill $!; trap - HUP; kill -HUP $$' HUP\nsleep 30 &\necho $$ > {log}/pid\nwait\n";

/// A session with no repository, run by `view::run`: at a hangup it takes a second, writes `got` and ends.
const HANGS_UP_ALONE: &str = "trap 'sleep 1; echo got > {log}/got; kill $!; exit 0' HUP\nsleep 10 &\necho $$ > {log}/pid\nwait\n";

/// A session that commits and ends by a termination it sends itself, with no one signalling `wb`.
const ENDS_ITSELF: &str = "echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\necho $$ > {log}/pid\nsleep 0.5\nkill -TERM $$\n";

/// A session that writes the signals it started with ignored, as `/proc` gives them.
const SAYS_IGNORED: &str = "grep SigIgn /proc/self/status > {log}/ignored\necho $$ > {log}/pid\n";

/// Not a test of a row: it does nothing unless a signal test started it. It is the part that runs `wb` in the process the signal tests start: it writes the folders it made, writes the
/// status when `wb` ends, and waits for the parent to be done with the folders.
#[test]
fn a_helper_that_runs_wb_for_the_signal_tests_and_checks_nothing() {
    let Some(folder) = std::env::var_os(CHILD_ENV).map(PathBuf::from) else { return };
    let bench = bench();
    bench.session(match std::env::var("WB_SIGNAL_CHILD_SESSION").as_deref() {
        Ok("traps") => TRAPS,
        Ok("storm") => STORM,
        Ok("cleanly-term") => CLEANLY_TERM,
        Ok("cleanly-hup") => CLEANLY_HUP,
        Ok("hangs-up-alone") => HANGS_UP_ALONE,
        Ok("ends-itself") => ENDS_ITSELF,
        Ok("says-ignored") => SAYS_IGNORED,
        _ => WAITS,
    });
    let sandbox = match std::env::var(CHILD_SANDBOX).as_deref() {
        Ok("real") | Ok("run") => PathBuf::from(watchbill::view::BWRAP),
        _ => stand_in_bwrap(&bench),
    };
    let paths = format!("{}\n{}\n{}\n", bench.repo.path.display(), bench.log.path.display(), bench.state.path.display());
    fs::write(folder.join("paths.tmp"), paths).unwrap();
    fs::rename(folder.join("paths.tmp"), folder.join("paths")).unwrap();
    let status = if std::env::var(CHILD_SANDBOX).as_deref() == Ok("run") {
        // The session in bubblewrap with no controlling terminal, as where `dev.tty.legacy_tiocsti` is not 0 (#257).
        let view = folder_named("view");
        let words = [bench.at("session.sh").to_str().unwrap().to_string()];
        let plan = watchbill::view::Plan {
            view: &view.path,
            checkout: &bench.repo.path,
            binds: vec![(bench.log.path.clone(), true)],
            organisation: None,
            home: Some(&bench.home.path),
            new_session: true,
            words: &words,
        };
        match watchbill::view::run(&sandbox, &plan) {
            Ok(status) => status.code().unwrap_or(-3),
            Err(_) => -1,
        }
    } else {
        code(&bench.start_by(&sandbox, &["start", "build", "54"])).unwrap_or(-1)
    };
    fs::write(folder.join("status.tmp"), format!("{}\n", status)).unwrap();
    fs::rename(folder.join("status.tmp"), folder.join("status")).unwrap();
    for _ in 0..600 {
        if folder.join("release").exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
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

/// What the run of `wb` with a signal left.
struct Signalled {
    /// The status of `wb`, or -2 when it had not ended after a minute.
    status: i32,
    outcome: String,
    back: String,
    subjects: String,
    /// What a session that traps the termination wrote when it got it.
    got: String,
    /// The line `SigIgn` of a session that wrote it.
    ignored: String,
}

/// Starts `wb start build 54`, sends `to_wb` to it when the session waits, then, after a moment, `to_session` to
/// the session when there is one, and gathers what was left. Nothing waits longer than a minute.
fn signalled(session_is: &str, to_wb: Option<i32>, to_session: Option<i32>) -> Signalled {
    signalled_how(How { session_is, sandbox: "stand-in", to_wb, to_group: false, to_session, hangup_ignored: false })
}

/// How `signalled_how` runs `wb` and signals it.
struct How<'a> {
    session_is: &'a str,
    /// `stand-in`, `real` or `run`, as `CHILD_SANDBOX` says.
    sandbox: &'a str,
    to_wb: Option<i32>,
    /// Whether `to_wb` goes to the whole process group of `wb`, as a terminal sends a hangup, and not to `wb` alone.
    to_group: bool,
    to_session: Option<i32>,
    /// Whether `wb` is started with SIGHUP ignored, as by `nohup`.
    hangup_ignored: bool,
}

fn signalled_how(how: How) -> Signalled {
    use std::os::unix::process::CommandExt;
    let How { session_is, sandbox, to_wb, to_group, to_session, hangup_ignored } = how;
    let parent = folder_named("signalled");
    let folder = parent.path.clone();
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "a_helper_that_runs_wb_for_the_signal_tests_and_checks_nothing", "--nocapture", "--test-threads=1"])
        .env(CHILD_ENV, &folder)
        .env(CHILD_SESSION, session_is)
        .env(CHILD_SANDBOX, sandbox)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if to_group {
        // A group of its own, so that a signal to the group reaches no process but those of this run.
        command.process_group(0);
    }
    if hangup_ignored {
        // Only `signal(2)`, which may be called between fork and exec.
        unsafe {
            command.pre_exec(|| {
                signal(SIGHUP, 1);
                Ok(())
            });
        }
    }
    let mut child = command.spawn().expect("the test binary could not be started");
    let found = (|| {
        if !wait_for(60, || folder.join("paths").exists()) {
            return Err("the folders were not made".to_string());
        }
        let paths = read(&folder.join("paths"));
        let mut at = paths.lines().map(PathBuf::from);
        let (repo, log, state) = (at.next().unwrap(), at.next().unwrap(), at.next().unwrap());
        if !wait_for(60, || read_if(&log.join("pid")).ends_with('\n')) {
            return Err("the session did not start".to_string());
        }
        let session: i32 = read(&log.join("pid")).trim().parse().unwrap();
        if let Some(signal) = to_wb {
            let to = if to_group { -(child.id() as i32) } else { child.id() as i32 };
            if unsafe { kill(to, signal) } != 0 {
                return Err("the signal could not be sent to wb".to_string());
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
        if let Some(signal) = to_session {
            if unsafe { kill(session, signal) } != 0 {
                return Err("the signal could not be sent to the session".to_string());
            }
        }
        let status = if wait_for(60, || folder.join("status").exists() || child.try_wait().ok().flatten().is_some()) && folder.join("status").exists() { read(&folder.join("status")).trim().parse().unwrap() } else { -2 };
        let one = fs::read_dir(state.join("watchbill/sessions")).ok().and_then(|found| found.map(|entry| entry.unwrap().path()).next());
        let (outcome, back) = match one {
            Some(one) => (read_if(&one.join("outcome")), read_if(&one.join("back"))),
            None => (String::new(), String::new()),
        };
        let log_of = Command::new("git").arg("-C").arg(&repo).args(["log", "--format=%s", "issue-54"]).env_clear().envs(env()).output();
        Ok(Signalled {
            status,
            outcome,
            back,
            subjects: log_of.map(|run| text_of(&run.stdout)).unwrap_or_default(),
            got: read_if(&log.join("got")),
            ignored: read_if(&log.join("ignored")),
        })
    })();
    // The session of a run that went wrong is ended too, by its number.
    let _ = fs::write(folder.join("release"), "");
    if !wait_for(10, || child.try_wait().ok().flatten().is_some()) {
        let _ = child.kill();
        let _ = child.wait();
    }
    found.unwrap_or_else(|why| panic!("{}", why))
}

fn read_if(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn test_WB_BUILD_54_a_hangup_sent_to_wb_while_the_session_runs_does_not_end_it_and_the_commit_is_taken_back() {
    let found = signalled("waits", Some(SIGHUP), Some(SIGHUP));
    assert_eq!(found.status, 1, "wb did not wait for the session and give its status");
    assert_eq!(found.subjects.lines().next(), Some("A new file"), "{}", found.subjects);
    assert!(found.back.lines().any(|line| line.starts_with("taken: ") && line.contains("A new file")), "{}", found.back);
}

#[test]
fn test_WB_BUILD_54_a_termination_sent_to_wb_while_the_session_runs_does_not_end_it_before_the_commit_is_taken_back() {
    let found = signalled("waits", Some(SIGTERM), None);
    assert_eq!(found.status, 1);
    assert_eq!(found.subjects.lines().next(), Some("A new file"), "{}", found.subjects);
    assert!(found.back.lines().any(|line| line.starts_with("taken: ")), "{}", found.back);
}

#[test]
fn test_WB_BUILD_55_a_termination_sent_to_wb_alone_ends_the_session() {
    let found = signalled("traps", Some(SIGTERM), None);
    // The test never signals the session: it got the termination from `wb`.
    assert_eq!(found.got, "got\n", "the session was not sent the termination");
    assert_ne!(found.status, -2, "the session was not ended");
    assert!(found.outcome.starts_with("outcome: ended\n"), "{}", found.outcome);
}

#[test]
fn test_WB_BUILD_57_terminations_sent_while_the_commits_are_taken_back_do_not_end_wb_before_the_lines_are_written() {
    // What `wb` wrote is the proof: a termination that ends `wb` during the take-back leaves no `outcome`. The
    // helper's own status is not asserted: a last termination may come after the lines are written and the hold
    // is let go, and end the helper then, which is right.
    let found = signalled("storm", None, None);
    assert!(found.back.lines().any(|line| line.starts_with("taken: ") && line.contains("A new file")), "{}", found.back);
    assert!(found.outcome.starts_with("outcome: ended\nstatus: 0\n"), "{}", found.outcome);
}

#[test]
fn test_WB_MON_9_outcome_names_the_signal_that_ended_the_session() {
    let found = signalled("waits", Some(SIGHUP), Some(SIGHUP));
    assert!(found.outcome.lines().any(|line| line == "signal: SIGHUP"), "{}", found.outcome);
    let found = signalled("waits", Some(SIGTERM), None);
    assert!(found.outcome.lines().any(|line| line == "signal: SIGTERM"), "{}", found.outcome);
}

/// What the run left with `how`, the real bubblewrap and the session `session_is`, or none where bubblewrap
/// cannot make a sandbox here, which is said.
fn in_bubblewrap(test: &str, how: How) -> Option<Signalled> {
    if !sandbox(test) {
        return None;
    }
    Some(signalled_how(how))
}

#[test]
fn test_WB_BUILD_55_in_a_view_a_termination_sent_to_wb_reaches_the_session_inside_bubblewrap() {
    let how = How { session_is: "cleanly-term", sandbox: "real", to_wb: Some(SIGTERM), to_group: false, to_session: None, hangup_ignored: false };
    let Some(found) = in_bubblewrap("test_WB_BUILD_55_in_a_view", how) else { return };
    // The test never signals the session: it got the termination from `wb`, and was not killed under it.
    assert_eq!(found.got, "got\n", "the session did not end by itself: {}", found.outcome);
    assert_eq!(found.subjects.lines().next(), Some("A new file"), "{}", found.subjects);
    assert!(found.back.lines().any(|line| line.starts_with("taken: ") && line.contains("A new file")), "{}", found.back);
    assert!(found.outcome.lines().any(|line| line == "signal: SIGTERM"), "{}", found.outcome);
    assert_eq!(found.status, 1);
}

#[test]
fn test_WB_VIEW_57_a_hangup_to_the_group_of_wb_ends_neither_bubblewrap_nor_the_session_under_it() {
    // A terminal sends its hangup to the whole foreground group: `wb`, bubblewrap and, with no new terminal
    // session, the session too. Where the session has no controlling terminal, `wb` sends it on (WB-BUILD-85).
    let how = How { session_is: "cleanly-hup", sandbox: "real", to_wb: Some(SIGHUP), to_group: true, to_session: None, hangup_ignored: false };
    let Some(found) = in_bubblewrap("test_WB_VIEW_57", how) else { return };
    assert_eq!(found.got, "got\n", "the session did not end by itself: {}", found.outcome);
    assert_eq!(found.subjects.lines().next(), Some("A new file"), "{}", found.subjects);
    assert!(found.back.lines().any(|line| line.starts_with("taken: ") && line.contains("A new file")), "{}", found.back);
    assert!(found.outcome.lines().any(|line| line == "signal: SIGHUP"), "{}", found.outcome);
    assert_eq!(found.status, 1);
}

#[test]
fn test_WB_VIEW_57_with_no_controlling_terminal_a_hangup_to_the_group_ends_neither_bubblewrap_nor_the_session() {
    // The session is in a terminal session of its own: the group's hangup reaches `wb` and bubblewrap only, and
    // `wb` sends it on (WB-BUILD-85).
    let how = How { session_is: "hangs-up-alone", sandbox: "run", to_wb: Some(SIGHUP), to_group: true, to_session: None, hangup_ignored: false };
    let Some(found) = in_bubblewrap("test_WB_VIEW_57_with_no_controlling_terminal", how) else { return };
    assert_eq!(found.got, "got\n", "the session did not end by itself");
    assert_eq!(found.status, 0);
}

#[test]
fn test_WB_BUILD_87_where_bubblewrap_names_no_session_wb_says_a_termination_will_not_reach_it() {
    // The stand-in writes nothing on the info fd, and the session runs on past the 2 seconds `wb` waits.
    let slow = bench();
    slow.session("sleep 3\nexit 0\n");
    let sandbox = stand_in_bwrap(&slow);
    let run = slow.start_by(&sandbox, &["start", "build", "54"]);
    let err = text_of(&run.stderr);
    let said: Vec<&str> = err.lines().filter(|line| line.contains("a termination of wb goes to bubblewrap")).collect();
    assert_eq!(said.len(), 1, "{}", err);
    assert!(said[0].starts_with("wb start build: the session was not found in bubblewrap"), "{}", err);
    // A session that ends at once is not said to be lost.
    let quick = bench();
    quick.session("exit 0\n");
    let sandbox = stand_in_bwrap(&quick);
    let run = quick.start_by(&sandbox, &["start", "build", "54"]);
    assert!(!text_of(&run.stderr).contains("was not found in bubblewrap"), "{}", text_of(&run.stderr));
}

#[test]
fn test_WB_BUILD_85_with_no_controlling_terminal_a_hangup_sent_to_wb_is_sent_on_to_the_session() {
    let how = How { session_is: "hangs-up-alone", sandbox: "run", to_wb: Some(SIGHUP), to_group: false, to_session: None, hangup_ignored: false };
    let Some(found) = in_bubblewrap("test_WB_BUILD_85", how) else { return };
    assert_eq!(found.got, "got\n", "the session was not sent the hangup");
    assert_eq!(found.status, 0);
}

#[test]
fn test_WB_MON_15_a_session_in_a_view_ended_by_a_signal_is_named_by_the_signal_in_outcome() {
    let how = How { session_is: "ends-itself", sandbox: "real", to_wb: None, to_group: false, to_session: None, hangup_ignored: false };
    let Some(found) = in_bubblewrap("test_WB_MON_15", how) else { return };
    assert!(found.outcome.starts_with("outcome: ended\nsignal: SIGTERM\n"), "{}", found.outcome);
    assert!(!found.outcome.lines().any(|line| line.starts_with("status: ")), "{}", found.outcome);
    assert!(found.back.lines().any(|line| line.starts_with("taken: ") && line.contains("A new file")), "{}", found.back);
}

/// Whether signal `number` is in the mask of a line `SigIgn:` of `/proc/<pid>/status`.
fn ignores(line: &str, number: u32) -> bool {
    let mask = line.trim().strip_prefix("SigIgn:").map(str::trim).unwrap_or_else(|| panic!("no SigIgn: {:?}", line));
    u64::from_str_radix(mask, 16).unwrap() & (1 << (number - 1)) != 0
}

#[test]
fn test_WB_VIEW_58_the_session_in_a_view_starts_with_the_hangup_and_the_termination_as_wb_found_them() {
    let how = How { session_is: "says-ignored", sandbox: "real", to_wb: None, to_group: false, to_session: None, hangup_ignored: false };
    let Some(found) = in_bubblewrap("test_WB_VIEW_58", how) else { return };
    assert!(!ignores(&found.ignored, 1) && !ignores(&found.ignored, 15), "{}", found.ignored);
    // As by `nohup`: the hangup stays ignored, the termination does not.
    let how = How { session_is: "says-ignored", sandbox: "real", to_wb: None, to_group: false, to_session: None, hangup_ignored: true };
    let found = signalled_how(how);
    assert!(ignores(&found.ignored, 1) && !ignores(&found.ignored, 15), "{}", found.ignored);
}

/// A terminal that is gone: every write fails once the session has started.
struct Gone {
    calls: PathBuf,
}

impl std::io::Write for Gone {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if read_if(&self.calls).lines().any(|call| call == "start") {
            return Err(std::io::Error::from_raw_os_error(5));
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_WB_BUILD_56_a_write_that_fails_after_the_session_changes_neither_the_status_nor_the_lines() {
    let bench = bench();
    bench.session("echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\n");
    let sandbox = stand_in_bwrap(&bench);
    let organisation = bench.organisation();
    let machine = watchbill::Machine { organisation: &organisation, terminal: true, state: Some(&bench.state.path), sandbox: &sandbox, home: Some(&bench.home.path) };
    let args: Vec<std::ffi::OsString> = ["start", "build", "54"].iter().map(std::ffi::OsString::from).collect();
    let mut out = Gone { calls: bench.at("calls") };
    let mut err = Gone { calls: bench.at("calls") };
    let status = watchbill::start_on(&args, &bench.repo.path, &machine, &mut out, &mut err);
    assert_eq!(status, 0);
    let folder = bench.lines();
    assert!(read(&folder.join("back")).lines().any(|line| line.starts_with("taken: ")));
    assert!(read(&folder.join("outcome")).starts_with("outcome: ended\nstatus: 0\n"));
}

// ---- #222: a view kept with work not taken back is seen ----

/// What a kept view is made with in a test.
struct Keep<'a> {
    /// The name of the session's folder, `<time>-<role>-<issue>`.
    session: &'a str,
    role: &'a str,
    round: u32,
    /// The subjects of the commits made in the view.
    commits: &'a [&'a str],
    /// The paths left uncommitted in the view.
    uncommitted: &'a [&'a str],
    /// The text of `outcome`.
    outcome: &'a str,
}

/// A view of the bench made as `wb` makes it, with what `keep` says in it, and the session's folder as a start
/// leaves it, with no `back`: the `wb` that ran it was ended.
fn keep(bench: &Bench, keep: &Keep) -> (PathBuf, PathBuf) {
    let below = bench.state.path.join("watchbill").join("views");
    let made = watchbill::view::make(&bench.root(), &["src/brake.c".to_string()], &below, keep.session).unwrap();
    for (number, subject) in keep.commits.iter().enumerate() {
        let path = format!("made-{}.c", number);
        put(&made.folder, &path, "int made;\n");
        git(&made.folder, &["add", "--", &path]);
        git(
            &made.folder,
            &["-c", "user.name=Ada Lovelace", "-c", "user.email=ada@example.org", "-c", "commit.gpgsign=false", "commit", "-q", "-m", subject],
        );
    }
    for path in keep.uncommitted {
        put(&made.folder, path, "left\n");
    }
    let session = bench.state.path.join("watchbill").join("sessions").join(keep.session);
    fs::create_dir_all(&session).unwrap();
    fs::write(session.join("stamp"), format!("[stamp] {} round {}\nview: none\n", keep.role, keep.round)).unwrap();
    fs::write(session.join("outcome"), keep.outcome).unwrap();
    fs::write(
        session.join("folder"),
        format!("folder: {}\nbase: {}\nhead: {}\nbranch: {}\nissue: {}\n", made.folder.display(), made.base, made.head, made.branch, keep.session.rsplit('-').next().unwrap()),
    )
    .unwrap();
    (made.folder, session)
}

const RAN: &str = "outcome: ended\nstatus: 0\nseconds: 5\n";

/// The start of the build of 54, as a person runs it.
fn start_54(bench: &Bench, more: &[&str]) -> Output {
    let mut args = vec!["start", "build", "54"];
    args.extend_from_slice(more);
    bench.start(&args)
}

/// The text of the `outcome` of the newest folder of the lines of a start.
fn newest_outcome(bench: &Bench) -> String {
    let found = bench.below("sessions");
    read(&found.last().unwrap().join("outcome"))
}

/// A process ID that is gone: a child that ended and was waited for.
fn gone_pid() -> u32 {
    let mut child = Command::new("true").spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    pid
}

#[test]
fn test_WB_BUILD_66_a_start_is_refused_naming_the_view_the_role_the_round_and_the_commit_not_taken() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["The tests of 54"], uncommitted: &[], outcome: RAN });
    let run = start_54(&bench, &[]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(err.contains(&folder.display().to_string()), "{}", err);
    assert!(err.contains("tester round 2"), "{}", err);
    assert!(err.contains("The tests of 54"), "{}", err);
    assert!(!bench.any_posted(), "{}", bench.left("calls"));
    assert!(!bench.started());
    // The refusal is before any view: only the kept one is there.
    assert_eq!(bench.below("views"), vec![folder]);
}

#[test]
fn test_WB_BUILD_66_a_view_kept_by_a_refused_commit_is_named_by_the_next_start() {
    if !sandbox("test_WB_BUILD_66") {
        return;
    }
    let bench = bench();
    bench.session("echo 'int brake(int);' > src/brake.h && git commit -q -a -m 'Change a read'\n");
    assert_eq!(code(&bench.start(&["start", "build", "54"])), Some(1));
    let run = start_54(&bench, &[]);
    assert_eq!(code(&run), Some(1));
    let err = text_of(&run.stderr);
    assert!(err.contains("is kept") && err.contains("build round 1") && err.contains("Change a read"), "{}", err);
}

#[test]
fn test_WB_BUILD_67_a_start_is_refused_naming_the_paths_left_uncommitted_in_a_kept_view() {
    let bench = bench();
    keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 1, commits: &[], uncommitted: &["tests/new_test.c"], outcome: RAN });
    let run = start_54(&bench, &[]);
    assert_eq!(code(&run), Some(1));
    let err = text_of(&run.stderr);
    assert!(err.contains("uncommitted: tests/new_test.c"), "{}", err);
}

#[test]
fn test_WB_BUILD_82_the_refusal_names_the_folder_to_remove_and_the_flag_that_ignores_it() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 1, commits: &["Tests"], uncommitted: &[], outcome: RAN });
    let err = text_of(&start_54(&bench, &[]).stderr);
    assert!(err.contains("--take-back"), "{}", err);
    assert!(err.contains(&format!("remove {}", folder.display())), "{}", err);
    assert!(err.contains("--ignore-kept"), "{}", err);
}

#[test]
fn test_WB_BUILD_69_ignore_kept_lets_the_start_go_on_and_the_stamp_says_so() {
    if !sandbox("test_WB_BUILD_69") {
        return;
    }
    let bench = bench();
    keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &[], outcome: RAN });
    let run = start_54(&bench, &["--ignore-kept"]);
    ok(&run);
    assert!(bench.started());
    let posted = bench.posted();
    assert!(posted.lines().any(|line| line == "kept: ignored 1"), "{}", posted);
    let stamp = read(&bench.below("sessions").last().unwrap().join("stamp"));
    assert!(stamp.lines().any(|line| line == "kept: ignored 1"), "{}", stamp);
}

#[test]
fn test_WB_BUILD_70_the_line_comes_after_local_and_a_start_with_none_kept_has_no_such_line() {
    if !sandbox("test_WB_BUILD_70") {
        return;
    }
    let bench = bench();
    keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &["x.c"], outcome: RAN });
    put(&bench.repo.path, "src/brake.h", "int brake(int);\n");
    ok(&start_54(&bench, &["--ignore-kept", "--ignore-local"]));
    let lines: Vec<String> = bench.posted().lines().map(str::to_string).collect();
    let local = lines.iter().position(|line| line == "local: ignored 1").unwrap_or_else(|| panic!("{:?}", lines));
    assert_eq!(lines[local + 1], "kept: ignored 1", "{:?}", lines);

    let bench = self::bench();
    ok(&start_54(&bench, &["--ignore-kept"]));
    assert!(!bench.posted().contains("kept:"), "{}", bench.posted());
}

#[test]
fn test_WB_BUILD_71_a_view_kept_with_nothing_in_it_is_not_named_and_does_not_stop_a_start() {
    let bench = bench();
    keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 1, commits: &[], uncommitted: &[], outcome: RAN });
    let run = start_54(&bench, &[]);
    assert!(!text_of(&run.stderr).contains("is kept"), "{}", text_of(&run.stderr));
    assert!(!newest_outcome(&bench).contains("check: kept"), "{}", newest_outcome(&bench));
}

#[test]
fn test_WB_BUILD_72_a_view_kept_for_another_issue_does_not_stop_a_start() {
    let bench = bench();
    keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-55", role: "tester", round: 1, commits: &["Tests"], uncommitted: &[], outcome: RAN });
    let run = start_54(&bench, &[]);
    assert!(!text_of(&run.stderr).contains("is kept"), "{}", text_of(&run.stderr));
    assert!(!newest_outcome(&bench).contains("check: kept"));
}

#[test]
fn test_WB_BUILD_73_a_view_kept_by_any_role_stops_a_start() {
    for role in ["tester", "build", "review", "fix"] {
        let bench = bench();
        let name = format!("2026-11-22T100000Z-{}-54", role);
        keep(&bench, &Keep { session: &name, role, round: 3, commits: &["Work"], uncommitted: &[], outcome: RAN });
        let run = start_54(&bench, &[]);
        assert_eq!(code(&run), Some(1), "{}: {}", role, text_of(&run.stderr));
        assert!(text_of(&run.stderr).contains(&format!("{} round 3", role)), "{}", text_of(&run.stderr));
    }
}

#[test]
fn test_WB_BUILD_74_a_start_refused_for_a_kept_view_leaves_a_refused_outcome_of_the_check_kept() {
    let bench = bench();
    keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 1, commits: &["Tests"], uncommitted: &[], outcome: RAN });
    assert_eq!(code(&start_54(&bench, &[])), Some(1));
    let found = bench.below("sessions");
    let outcome = read(&found.iter().find(|folder| !folder.ends_with("2026-11-22T100000Z-tester-54")).unwrap().join("outcome"));
    assert!(outcome.starts_with("outcome: refused\ncheck: kept\nstatus: 1\n"), "{}", outcome);
    assert!(!bench.started());
}

#[test]
fn test_WB_BUILD_75_ignore_kept_passes_the_kept_views_alone() {
    let bench = bench();
    keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 1, commits: &["Tests"], uncommitted: &[], outcome: RAN });
    put(&bench.repo.path, "src/brake.c", "int brake(void) { return 2; }\n");
    let run = start_54(&bench, &["--ignore-kept", "--ignore-local"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("cannot be overridden") || newest_outcome(&bench).contains("check: local"), "{}", text_of(&run.stderr));
    assert!(!bench.any_posted());
}

#[test]
fn test_WB_MON_10_outcome_says_running_with_the_process_id_while_the_session_runs_and_ended_after() {
    let bench = bench();
    bench.session("cat {state}/watchbill/sessions/*/outcome > {log}/during\n");
    let run = bench.start_by(Path::new("/nonexistent/no-bwrap"), &["start", "build", "99"]);
    ok(&run);
    let during = bench.left("during");
    let mut lines = during.lines();
    assert_eq!(lines.next(), Some("outcome: running"), "{}", during);
    assert_eq!(lines.next().map(|line| line.starts_with("pid: ") && line[5..].parse::<u32>().is_ok()), Some(true), "{}", during);
    assert!(lines.next().map_or(false, |line| line.starts_with("started: ")), "{}", during);
    // WB-MON-11: replaced when the start ends.
    let after = read(&bench.lines().join("outcome"));
    assert!(after.starts_with("outcome: ended\nstatus: 0\nseconds: "), "{}", after);
    assert!(!after.contains("pid:") && !after.contains("running"), "{}", after);
    assert!(!bench.lines().join("outcome.new").exists());
}

#[test]
fn test_WB_MON_12_the_folder_of_a_session_in_a_view_holds_folder_before_the_session_starts() {
    if !sandbox("test_WB_MON_12") {
        return;
    }
    let bench = bench();
    bench.session("echo 'int brake(int);' > src/brake.h && git commit -q -a -m 'Change a read'\n");
    assert_eq!(code(&bench.start(&["start", "build", "54"])), Some(1));
    let file = read(&bench.lines().join("folder"));
    let views = bench.below("views");
    assert_eq!(views.len(), 1);
    let mut lines = file.lines();
    assert_eq!(lines.next(), Some(format!("folder: {}", views[0].display()).as_str()), "{}", file);
    assert!(lines.next().map_or(false, |line| line.starts_with("base: ") && line.len() >= 46), "{}", file);
    assert_eq!(lines.next(), Some(format!("head: {}", bench.head()).as_str()), "{}", file);
    assert_eq!(lines.next(), Some("branch: refs/heads/issue-54"), "{}", file);
    assert_eq!(lines.next(), Some("issue: 54"), "{}", file);
}

/// `wb` run as a person runs it in the bench's repository, with the state folder of the bench.
fn wb_state(bench: &Bench, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(args)
        .current_dir(&bench.repo.path)
        .env_clear()
        .envs(env())
        .env("XDG_STATE_HOME", &bench.state.path)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap()
}

#[test]
fn test_WB_STATUS_58_status_of_an_issue_prints_a_kept_line_for_each_view_with_work_not_taken() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &[], outcome: RAN });
    let (second, _) = keep(&bench, &Keep { session: "2026-11-22T110000Z-build-54", role: "build", round: 1, commits: &["One", "Two"], uncommitted: &["a.c", "b.c"], outcome: RAN });
    keep(&bench, &Keep { session: "2026-11-22T120000Z-fix-54", role: "fix", round: 1, commits: &[], uncommitted: &[], outcome: RAN });
    let run = wb_state(&bench, &["status", "54"]);
    let out = text_of(&run.stdout);
    assert!(out.lines().any(|line| line == format!("kept: tester round 2, 1 commit not taken, {}", folder.display())), "{}", out);
    assert!(out.lines().any(|line| line == format!("kept: build round 1, 2 commits not taken, 2 paths uncommitted, {}", second.display())), "{}", out);
    assert_eq!(out.matches("kept: ").count(), 2, "{}", out);
}

#[test]
fn test_WB_STATUS_59_status_with_no_issue_says_the_count_once_on_standard_error() {
    let bench = bench();
    let run = wb_state(&bench, &["status"]);
    assert!(!text_of(&run.stderr).contains("kept view"), "{}", text_of(&run.stderr));
    keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &[], outcome: RAN });
    keep(&bench, &Keep { session: "2026-11-22T110000Z-build-55", role: "build", round: 1, commits: &[], uncommitted: &["a.c"], outcome: RAN });
    keep(&bench, &Keep { session: "2026-11-22T120000Z-fix-56", role: "fix", round: 1, commits: &[], uncommitted: &[], outcome: RAN });
    let run = wb_state(&bench, &["status"]);
    let err = text_of(&run.stderr);
    assert_eq!(err.matches("2 kept views").count(), 1, "{}", err);
    assert!(!text_of(&run.stdout).contains("kept"), "{}", text_of(&run.stdout));
}

#[test]
fn test_WB_STATUS_60_a_session_whose_outcome_says_running_and_whose_wb_is_gone_is_cut_off() {
    let bench = bench();
    let session = bench.state.path.join("watchbill").join("sessions").join("2026-11-22T100000Z-tester-54");
    fs::create_dir_all(&session).unwrap();
    fs::write(session.join("stamp"), "[stamp] tester round 2\n").unwrap();
    fs::write(session.join("outcome"), format!("outcome: running\npid: {}\n", gone_pid())).unwrap();
    let out = text_of(&wb_state(&bench, &["status", "54"]).stdout);
    assert!(out.lines().any(|line| line == format!("cut off: tester round 2, {}", session.display())), "{}", out);
    // A process that is alive and is not `wb` is no `wb`: process IDs are reused.
    let mut other = Command::new("sleep").arg("30").spawn().unwrap();
    fs::write(session.join("outcome"), format!("outcome: running\npid: {}\n", other.id())).unwrap();
    let out = text_of(&wb_state(&bench, &["status", "54"]).stdout);
    other.kill().unwrap();
    other.wait().unwrap();
    assert!(out.contains("cut off: tester round 2"), "{}", out);
    // A start that ended is not cut off.
    fs::write(session.join("outcome"), RAN).unwrap();
    assert!(!text_of(&wb_state(&bench, &["status", "54"]).stdout).contains("cut off"));
}

#[test]
fn test_WB_STATUS_61_a_session_is_not_cut_off_while_its_wb_is_alive() {
    let bench = bench();
    let session = bench.state.path.join("watchbill").join("sessions").join("2026-11-22T100000Z-tester-54");
    fs::create_dir_all(&session).unwrap();
    fs::write(session.join("stamp"), "[stamp] tester round 2\n").unwrap();
    // A `wb` that waits for what a person types.
    let lonely = repository(None);
    let mut child = Command::new(env!("CARGO_BIN_EXE_wb"))
        .arg("init")
        .current_dir(&lonely.path)
        .env_clear()
        .envs(env())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let pid = child.id();
    let is_wb = || fs::read_link(format!("/proc/{}/exe", pid)).ok().and_then(|exe| exe.file_name().map(|name| name == "wb")).unwrap_or(false);
    for _ in 0..500 {
        if is_wb() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(is_wb());
    fs::write(session.join("outcome"), format!("outcome: running\npid: {}\n", pid)).unwrap();
    let out = text_of(&wb_state(&bench, &["status", "54"]).stdout);
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(!out.contains("cut off"), "{}", out);
}

#[test]
fn test_WB_STATUS_62_status_reads_a_kept_view_through_a_repository_of_its_own_and_writes_nothing() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &["a.c"], outcome: "outcome: running\npid: 1\n" });
    // The view's own settings and hooks must never run.
    let marker = bench.log.path.join("ran");
    let hook = bench.log.path.join("hook.sh");
    script(&hook, &format!("echo ran >> {}\nexit 0\n", marker.display()));
    let config = folder.join(".git/config");
    let mut text = read(&config);
    text.push_str(&format!(
        "[core]\n\tfsmonitor = {h}\n\thooksPath = {l}\n\tpager = {h}\n[diff]\n\texternal = {h}\n[include]\n\tpath = {h}\n",
        h = hook.display(),
        l = bench.log.path.display()
    ));
    fs::write(&config, text).unwrap();
    fs::write(folder.join(".gitattributes"), "* filter=hostile diff=hostile\n").unwrap();
    let listing = |root: &Path| -> Vec<String> {
        use std::os::unix::fs::MetadataExt;
        let mut found = Vec::new();
        let mut open = vec![root.to_path_buf()];
        while let Some(at) = open.pop() {
            for entry in fs::read_dir(&at).unwrap() {
                let path = entry.unwrap().path();
                let meta = fs::symlink_metadata(&path).unwrap();
                found.push(format!("{} {} {}", path.display(), meta.len(), meta.mtime_nsec()));
                if meta.is_dir() {
                    open.push(path);
                }
            }
        }
        found.sort();
        found
    };
    let before = (listing(&folder), listing(&bench.state.path));
    let run = wb_state(&bench, &["status", "54"]);
    let out = text_of(&run.stdout);
    assert!(out.contains("kept: tester round 2, 1 commit not taken, 2 paths uncommitted"), "{}", out);
    assert!(!marker.exists(), "a hook or setting of the view ran");
    assert_eq!(before, (listing(&folder), listing(&bench.state.path)));
}

#[test]
fn test_WB_STATUS_63_the_work_of_a_view_with_no_back_is_found_from_its_objects() {
    let bench = bench();
    let (_, session) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 1, commits: &["One", "Two"], uncommitted: &[], outcome: "outcome: running\npid: 1\n" });
    let out = text_of(&wb_state(&bench, &["status", "54"]).stdout);
    assert!(out.contains("kept: tester round 1, 2 commits not taken,"), "{}", out);
    assert!(out.contains("cut off: tester round 1,"), "{}", out);
    // The start names each commit of it.
    let err = text_of(&start_54(&bench, &[]).stderr);
    assert!(err.contains("One") && err.contains("Two"), "{}", err);
    assert!(session.exists());
}

// ---- #222, the review of #264: a kept view cannot hang wb, a live session is not kept ----

/// Runs a command and waits for it at most `seconds`; the child is killed by its PID when the time is up, and the
/// result is none then.
fn bounded(mut command: Command, seconds: u64) -> Option<(Output, std::time::Duration)> {
    use std::io::Read;
    let began = std::time::Instant::now();
    let mut child = command.stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().unwrap();
    let (mut out, mut err) = (child.stdout.take().unwrap(), child.stderr.take().unwrap());
    let reading = std::thread::spawn(move || {
        let (mut a, mut b) = (Vec::new(), Vec::new());
        let _ = out.read_to_end(&mut a);
        let _ = err.read_to_end(&mut b);
        (a, b)
    });
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            let (stdout, stderr) = reading.join().unwrap();
            return Some((Output { status, stdout, stderr }, began.elapsed()));
        }
        if began.elapsed() > std::time::Duration::from_secs(seconds) {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// `wb` as `wb_state` runs it, with a bound.
fn wb_state_bounded(bench: &Bench, args: &[&str], seconds: u64) -> Option<Output> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wb"));
    command.args(args).current_dir(&bench.repo.path).env_clear().envs(env()).env("XDG_STATE_HOME", &bench.state.path);
    bounded(command, seconds).map(|(run, _)| run)
}

/// `wb start build 54 …` in a thread of its own, as `Bench::start` runs it, with a bound.
fn start_bounded(bench: &Bench, more: &[&str], seconds: u64) -> Option<Output> {
    use std::os::unix::process::ExitStatusExt;
    let repo = bench.repo.path.clone();
    let organisation = bench.organisation();
    let (state, home) = (bench.state.path.clone(), bench.home.path.clone());
    let mut args: Vec<std::ffi::OsString> = ["start", "build", "54"].iter().map(std::ffi::OsString::from).collect();
    args.extend(more.iter().map(std::ffi::OsString::from));
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let machine = watchbill::Machine { organisation: &organisation, terminal: true, state: Some(&state), sandbox: Path::new(watchbill::view::BWRAP), home: Some(&home) };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let status = watchbill::start_on(&args, &repo, &machine, &mut out, &mut err);
        let _ = sender.send(Output { status: std::process::ExitStatus::from_raw(status << 8), stdout: out, stderr: err });
    });
    receiver.recv_timeout(std::time::Duration::from_secs(seconds)).ok()
}

/// Puts a FIFO in place of the loose object of the view's newest commit: git blocks for ever opening it.
fn fifo_for_head(view: &Path) {
    let head = {
        let run = Command::new("git").arg("-C").arg(view).args(["rev-parse", "HEAD"]).env_clear().envs(env()).output().unwrap();
        text(&run.stdout).trim().to_string()
    };
    let object = view.join(".git/objects").join(&head[..2]).join(&head[2..]);
    fs::remove_file(&object).unwrap();
    assert!(Command::new("mkfifo").arg(&object).status().unwrap().success());
}

#[test]
fn test_WB_STATUS_64_a_fifo_for_an_object_of_a_kept_view_does_not_hang_status_or_a_start() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &[], outcome: "outcome: running\npid: 1\n" });
    fifo_for_head(&folder);
    let run = wb_state_bounded(&bench, &["status", "54"], 10).expect("wb status did not end in 10 seconds");
    let out = text_of(&run.stdout);
    assert!(out.lines().any(|line| line.starts_with("kept: tester round 2, not read: ")), "{}", out);
    // A start is refused, with the view named as not read, in time.
    let run = start_bounded(&bench, &[], 10).expect("wb start did not end in 10 seconds");
    assert_eq!(code(&run), Some(1));
    assert!(text_of(&run.stderr).contains("not read"), "{}", text_of(&run.stderr));
}

#[test]
fn test_WB_BUILD_76_ignore_kept_counts_the_views_and_runs_no_git_on_them() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &[], outcome: RAN });
    fifo_for_head(&folder);
    // The terminal check ends the start after the stamp is made, and the stamp is kept in the lines.
    let run = start_bounded(&bench, &["--ignore-kept"], 10).expect("wb start --ignore-kept did not end in 10 seconds");
    let _ = run;
    let found = bench.below("sessions");
    let newest = found.iter().find(|one| !one.ends_with("2026-11-22T100000Z-tester-54")).unwrap();
    assert!(read(&newest.join("stamp")).lines().any(|line| line == "kept: ignored 1"), "{}", read(&newest.join("stamp")));
}

#[test]
fn test_WB_BUILD_77_a_session_that_runs_with_a_live_wb_is_not_kept_for_a_start() {
    let bench = bench();
    let (folder, session) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &[], outcome: "outcome: running\n" });
    let lonely = repository(None);
    let mut child = Command::new(env!("CARGO_BIN_EXE_wb"))
        .arg("init")
        .current_dir(&lonely.path)
        .env_clear()
        .envs(env())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let pid = child.id();
    let is_wb = || fs::read_link(format!("/proc/{}/exe", pid)).ok().and_then(|exe| exe.file_name().map(|name| name == "wb")).unwrap_or(false);
    for _ in 0..500 {
        if is_wb() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(is_wb());
    fs::write(session.join("outcome"), format!("outcome: running\npid: {}\n", pid)).unwrap();
    // Its view is never read: a FIFO where git would read would hang.
    fifo_for_head(&folder);
    let run = start_bounded(&bench, &[], 10);
    let status = wb_state_bounded(&bench, &["status", "54"], 10);
    child.kill().unwrap();
    child.wait().unwrap();
    let run = run.expect("the start did not end in 10 seconds");
    assert!(!text_of(&run.stderr).contains("is kept"), "{}", text_of(&run.stderr));
    let out = text_of(&status.expect("status did not end").stdout);
    assert!(!out.contains("kept:") && !out.contains("cut off"), "{}", out);
}

#[test]
fn test_WB_STATUS_65_a_session_that_runs_with_a_live_wb_is_skipped_by_status() {
    let bench = bench();
    let (_, session) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &[], outcome: "outcome: running\n" });
    let lonely = repository(None);
    let mut child = Command::new(env!("CARGO_BIN_EXE_wb"))
        .arg("init")
        .current_dir(&lonely.path)
        .env_clear()
        .envs(env())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let pid = child.id();
    let is_wb = || fs::read_link(format!("/proc/{}/exe", pid)).ok().and_then(|exe| exe.file_name().map(|name| name == "wb")).unwrap_or(false);
    for _ in 0..500 {
        if is_wb() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(is_wb());
    fs::write(session.join("outcome"), format!("outcome: running\npid: {}\n", pid)).unwrap();
    let out = text_of(&wb_state(&bench, &["status", "54"]).stdout);
    let err = text_of(&wb_state(&bench, &["status"]).stderr);
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(!out.contains("kept:") && !out.contains("cut off"), "{}", out);
    assert!(!err.contains("kept view"), "{}", err);
}

#[test]
fn test_WB_STATUS_66_a_commit_message_of_many_mebibytes_is_not_read_whole_or_printed() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &[], uncommitted: &[], outcome: RAN });
    let message = bench.log.path.join("message");
    fs::write(&message, format!("{}\n", "x".repeat(5 * 1024 * 1024))).unwrap();
    put(&folder, "huge.c", "int huge;\n");
    git(&folder, &["add", "--", "huge.c"]);
    git(&folder, &["-c", "user.name=Ada Lovelace", "-c", "user.email=ada@example.org", "-c", "commit.gpgsign=false", "commit", "-q", "-F", message.to_str().unwrap()]);
    let run = start_bounded(&bench, &[], 10).expect("the start did not end");
    assert_eq!(code(&run), Some(1));
    assert!(run.stderr.len() < 64 * 1024, "{} bytes", run.stderr.len());
    let out = wb_state_bounded(&bench, &["status", "54"], 10).expect("status did not end").stdout;
    assert!(out.len() < 64 * 1024 && text_of(&out).contains("1 commit not taken"), "{} bytes", out.len());
}

#[test]
fn test_WB_STATUS_67_a_filter_driver_of_the_global_config_is_not_run_on_the_files_of_a_view() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &[], outcome: "outcome: running\npid: 1\n" });
    let marker = bench.log.path.join("filtered");
    let driver = bench.log.path.join("driver.sh");
    script(&driver, &format!("echo ran >> {}\ncat\n", marker.display()));
    let global = bench.home.path.join(".gitconfig");
    fs::write(&global, format!("[filter \"hostile\"]\n\tclean = {d}\n\tsmudge = {d}\n[diff \"hostile\"]\n\ttextconv = {d}\n", d = driver.display())).unwrap();
    fs::write(folder.join(".gitattributes"), "* filter=hostile diff=hostile\n").unwrap();
    // A file of the view that is in its commit and changed, so that git must look at its content.
    fs::write(folder.join("src/brake.c"), "int changed;\n").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_wb"));
    command
        .args(["status", "54"])
        .current_dir(&bench.repo.path)
        .env_clear()
        .envs(env())
        .env("XDG_STATE_HOME", &bench.state.path)
        .env("HOME", &bench.home.path);
    let (run, _) = bounded(command, 10).expect("status did not end");
    assert!(text_of(&run.stdout).contains("kept: tester round 2"), "{}", text_of(&run.stdout));
    assert!(!marker.exists(), "a filter driver of the global config ran on a file of the view");
}

#[test]
fn test_WB_MON_13_a_file_of_the_lines_that_is_not_a_plain_file_is_not_read_and_not_waited_for() {
    let bench = bench();
    let (_, session) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &[], outcome: RAN });
    for name in ["back", "stamp"] {
        let _ = fs::remove_file(session.join(name));
        assert!(Command::new("mkfifo").arg(session.join(name)).status().unwrap().success());
    }
    assert!(wb_state_bounded(&bench, &["status", "54"], 10).is_some(), "status did not end");
    fs::remove_file(session.join("outcome")).unwrap();
    assert!(Command::new("mkfifo").arg(session.join("outcome")).status().unwrap().success());
    assert!(wb_state_bounded(&bench, &["status", "54"], 10).is_some(), "status did not end");
}

#[test]
fn test_WB_STATUS_68_status_leaves_nothing_in_the_temporary_folder() {
    let bench = bench();
    keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &["a.c"], outcome: "outcome: running\npid: 1\n" });
    let temporary = folder_named("tmp");
    let mut command = Command::new(env!("CARGO_BIN_EXE_wb"));
    command.args(["status", "54"]).current_dir(&bench.repo.path).env_clear().envs(env()).env("XDG_STATE_HOME", &bench.state.path).env("TMPDIR", &temporary.path);
    let (run, _) = bounded(command, 10).expect("status did not end");
    assert!(text_of(&run.stdout).contains("kept: tester round 2, 1 commit not taken, 1 paths uncommitted"), "{}", text_of(&run.stdout));
    assert_eq!(fs::read_dir(&temporary.path).unwrap().count(), 0);
}

// ---- #223: a kept view is taken back with `wb start <role> <issue> --take-back` ----

/// A commit made in a kept view, as a session makes it.
fn commit_in(view: &Path, path: &str, text: &str, subject: &str) {
    put(view, path, text);
    git(view, &["add", "--", path]);
    git(view, &["-c", "user.name=Ada Lovelace", "-c", "user.email=ada@example.org", "-c", "commit.gpgsign=false", "commit", "-q", "-m", subject]);
}

/// A kept view of the tester for 54, with no commit yet.
fn kept_tester(bench: &Bench) -> (PathBuf, PathBuf) {
    keep(bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 1, commits: &[], uncommitted: &[], outcome: RAN })
}

/// `wb start tester 54 --take-back` with more words.
fn take_back(bench: &Bench, more: &[&str]) -> Output {
    let mut args = vec!["start", "tester", "54", "--take-back"];
    args.extend_from_slice(more);
    bench.start(&args)
}

/// The same in a thread of its own, with a bound.
fn take_back_bounded(bench: &Bench, seconds: u64) -> Option<Output> {
    use std::os::unix::process::ExitStatusExt;
    let repo = bench.repo.path.clone();
    let organisation = bench.organisation();
    let (state, home) = (bench.state.path.clone(), bench.home.path.clone());
    let args: Vec<std::ffi::OsString> = ["start", "tester", "54", "--take-back"].iter().map(std::ffi::OsString::from).collect();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let machine = watchbill::Machine { organisation: &organisation, terminal: true, state: Some(&state), sandbox: Path::new(watchbill::view::BWRAP), home: Some(&home) };
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let status = watchbill::start_on(&args, &repo, &machine, &mut out, &mut err);
        let _ = sender.send(Output { status: std::process::ExitStatus::from_raw(status << 8), stdout: out, stderr: err });
    });
    receiver.recv_timeout(std::time::Duration::from_secs(seconds)).ok()
}

fn subjects(bench: &Bench, count: usize) -> Vec<String> {
    bench.git_text(&["log", &format!("-{}", count), "--format=%s"]).lines().map(str::to_string).collect()
}

#[test]
fn test_WB_BUILD_78_take_back_puts_the_commit_of_a_kept_view_on_the_branch_and_starts_and_posts_nothing() {
    let bench = bench();
    let (folder, session) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 2; }\n", "The tests of 54");
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(0), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["The tests of 54"]);
    assert_eq!(bench.git_text(&["show", "HEAD:src/brake.c"]), "int brake(void) { return 2; }\n");
    assert!(read(&session.join("back")).lines().any(|line| line.starts_with("taken: ")), "{}", read(&session.join("back")));
    assert!(read(&session.join("back")).lines().last() == Some("view: removed"));
    assert!(!folder.exists());
    assert!(!bench.any_posted() && !bench.started(), "{}", bench.left("calls"));
    // The lines of no new start are written.
    assert_eq!(bench.below("sessions").len(), 1);
}

#[test]
fn test_WB_BUILD_79_take_back_with_no_kept_view_of_the_role_and_issue_ends_with_1_saying_so() {
    let bench = bench();
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("no kept view of the tester for issue 54"), "{}", text_of(&run.stderr));
    // A view of another role is not it.
    let (folder, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-build-54", role: "build", round: 1, commits: &[], uncommitted: &[], outcome: RAN });
    commit_in(&folder, "src/brake.c", "int brake(void) { return 3; }\n", "Build");
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(folder.exists());
}

#[test]
fn test_WB_BUILD_79_take_back_takes_the_newest_view_of_the_role_and_issue() {
    let bench = bench();
    let (old, _) = keep(&bench, &Keep { session: "2026-11-22T090000Z-tester-54", role: "tester", round: 1, commits: &[], uncommitted: &[], outcome: RAN });
    commit_in(&old, "src/brake.c", "int brake(void) { return 4; }\n", "Old");
    let (new, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &[], uncommitted: &[], outcome: RAN });
    commit_in(&new, "src/brake.c", "int brake(void) { return 5; }\n", "New");
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(0), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["New"]);
    assert!(old.exists() && !new.exists());
}

#[test]
fn test_WB_BUILD_80_take_back_with_other_options_of_start_or_with_no_issue_ends_with_2() {
    let bench = bench();
    let (folder, _) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 6; }\n", "Kept");
    for more in [&["--prompt"][..], &["--stamp"], &["--parts"], &["--runner", "x"], &["--ignore-local"], &["--ignore-kept"], &["--round", "2"]] {
        let run = take_back(&bench, more);
        assert_eq!(code(&run), Some(2), "{:?}: {}", more, text_of(&run.stderr));
        assert!(text_of(&run.stderr).contains("used with the role and the issue alone"), "{:?}: {}", more, text_of(&run.stderr));
    }
    let run = take_back(&bench, &["--take-back"]);
    assert_eq!(code(&run), Some(2), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("'--take-back' is given twice"), "{}", text_of(&run.stderr));
    let run = bench.start(&["start", "review", "--change", "5", "--take-back"]);
    assert_eq!(code(&run), Some(2), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("used with the role and the issue alone"), "{}", text_of(&run.stderr));
    let run = bench.start(&["start", "tester", "--take-back"]);
    assert_eq!(code(&run), Some(2), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("the tester is handed out for an issue"), "{}", text_of(&run.stderr));
    assert!(folder.exists());
    assert_eq!(subjects(&bench, 1), vec!["The issue's file of 54"]);
}

#[test]
fn test_WB_BUILD_81_take_back_says_first_the_view_the_role_the_round_and_each_commit_with_its_subject() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 3, commits: &[], uncommitted: &[], outcome: RAN });
    commit_in(&folder, "src/brake.c", "int brake(void) { return 7; }\n", "First of two");
    commit_in(&folder, "tests/test_brake.c", "int main(void) { return 1; }\n", "Second of two");
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(0), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    let will = err.find(&folder.display().to_string()).expect(&err);
    assert!(err[will..].contains("tester round 3"), "{}", err);
    let (one, two) = (err.find("First of two").unwrap(), err.find("Second of two").unwrap());
    let taken = err.find("taken onto").expect(&err);
    assert!(will < one && one < two && two < taken, "{}", err);
    assert_eq!(subjects(&bench, 2), vec!["Second of two", "First of two"]);
}

#[test]
fn test_WB_BUILD_82_the_refusal_of_a_kept_view_names_how_to_take_it_back() {
    let bench = bench();
    keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 1, commits: &["Tests"], uncommitted: &[], outcome: RAN });
    let err = text_of(&start_54(&bench, &[]).stderr);
    assert!(err.contains("wb start tester 54 --take-back"), "{}", err);
    assert!(!err.contains("not built yet"), "{}", err);
}

#[test]
fn test_WB_VIEW_47_a_commit_that_touches_watchbill_is_refused_and_nothing_of_it_or_after_it_is_taken_and_the_view_is_kept() {
    let bench = bench();
    let (folder, session) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 8; }\n", "Good");
    commit_in(&folder, ".watchbill/rules.yml", "evil: true\n", "Evil");
    commit_in(&folder, "tests/test_brake.c", "int main(void) { return 2; }\n", "After");
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains(".watchbill"), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["Good"]);
    assert!(bench.git_text(&["log", "--format=%s"]).lines().all(|subject| subject != "Evil" && subject != "After"));
    assert!(folder.exists());
    let back = read(&session.join("back"));
    assert!(back.lines().any(|line| line.starts_with("refused: ")) && back.lines().any(|line| line.starts_with("not taken: ")), "{}", back);
    assert!(back.lines().last().unwrap().starts_with("view: kept "), "{}", back);
}

#[test]
fn test_WB_VIEW_47_a_commit_that_touches_a_build_file_or_another_path_is_refused() {
    for (path, text) in [("build/Makefile", "evil:\n"), ("src/other.c", "int other;\n")] {
        let bench = bench();
        let (folder, _) = kept_tester(&bench);
        commit_in(&folder, path, text, "Odd");
        let run = take_back(&bench, &[]);
        assert_eq!(code(&run), Some(1), "{}: {}", path, text_of(&run.stderr));
        assert_eq!(subjects(&bench, 1), vec!["The issue's file of 54"], "{}", path);
        assert!(folder.exists());
    }
}

#[test]
fn test_WB_VIEW_48_a_branch_moved_on_with_a_commit_to_the_same_path_ends_with_1_naming_it_and_takes_nothing() {
    let bench = bench();
    let (folder, session) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 9; }\n", "Tests");
    put(&bench.repo.path, "src/brake.c", "int brake(void) { return 10; }\n");
    git(&bench.repo.path, &["commit", "-q", "-a", "-m", "Moved on"]);
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(err.contains("src/brake.c") && err.contains("moved on"), "{}", err);
    assert_eq!(subjects(&bench, 1), vec!["Moved on"]);
    assert!(folder.exists());
    assert!(read(&session.join("back")).lines().any(|line| line.starts_with("not taken: ")));
}

#[test]
fn test_WB_VIEW_49_a_branch_moved_on_with_changes_elsewhere_is_a_descendant_and_the_commits_go_on_top() {
    let bench = bench();
    let (folder, _) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 11; }\n", "Tests");
    put(&bench.repo.path, "docs/note.txt", "a note\n");
    git(&bench.repo.path, &["add", "--", "docs/note.txt"]);
    git(&bench.repo.path, &["commit", "-q", "-m", "A note"]);
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(0), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 2), vec!["Tests", "A note"]);
    assert_eq!(bench.git_text(&["show", "HEAD:docs/note.txt"]), "a note\n");
    assert!(!folder.exists());
}

#[test]
fn test_WB_VIEW_50_a_checkout_on_another_branch_than_the_views_ends_with_1_naming_the_branch() {
    let bench = bench();
    let (folder, _) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 12; }\n", "Tests");
    git(&bench.repo.path, &["checkout", "-q", "-b", "other"]);
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("issue-54"), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["The issue's file of 54"]);
    assert!(folder.exists());
}

#[test]
fn test_WB_VIEW_51_nothing_of_the_views_settings_or_hooks_runs_in_a_take_back() {
    let bench = bench();
    let (folder, _) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 13; }\n", "Tests");
    let marks = folder_named("marks");
    let m = marks.path.to_str().unwrap();
    for name in ["post-commit", "reference-transaction", "post-merge", "fsmonitor"] {
        script(&marks.path.join(name), &format!("echo {name} >> {m}/ran\nexit 0\n"));
    }
    let config = folder.join(".git/config");
    let mut text = read(&config);
    text.push_str(&format!("[core]\n\thooksPath = {m}\n\tfsmonitor = {m}/fsmonitor\n[filter \"x\"]\n\tclean = {m}/fsmonitor\n\tsmudge = {m}/fsmonitor\n"));
    fs::write(&config, text).unwrap();
    fs::write(folder.join(".gitattributes"), "* filter=x\n").unwrap();
    put(&folder, "left.txt", "uncommitted\n");
    let run = take_back(&bench, &[]);
    assert!(fs::read_to_string(marks.path.join("ran")).unwrap_or_default().is_empty(), "{}", fs::read_to_string(marks.path.join("ran")).unwrap_or_default());
    assert_eq!(subjects(&bench, 1), vec!["Tests"], "{}", text_of(&run.stderr));
}

#[test]
fn test_WB_VIEW_52_a_fifo_for_an_object_of_the_view_ends_a_take_back_within_a_bound_and_takes_nothing() {
    let bench = bench();
    let (folder, _) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 14; }\n", "Tests");
    fifo_for_head(&folder);
    let run = take_back_bounded(&bench, 20).expect("the take-back did not end in 20 seconds");
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["The issue's file of 54"]);
    assert!(folder.exists());
    let prefix = format!("wb-view-{}-", std::process::id());
    // Other tests of this process make and remove their own stores at the same time: it is gone when none is there.
    let mut temporary = Vec::new();
    for _ in 0..300 {
        temporary = fs::read_dir(std::env::temp_dir()).unwrap().filter_map(|entry| entry.ok()).filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix)).collect::<Vec<_>>();
        if temporary.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(temporary.is_empty(), "{:?}", temporary);
}

#[test]
fn test_WB_VIEW_53_paths_left_uncommitted_are_named_and_not_taken_and_the_view_is_kept() {
    let bench = bench();
    let (folder, session) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 15; }\n", "Tests");
    put(&folder, "tests/new_test.c", "left\n");
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("tests/new_test.c"), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["Tests"]);
    assert!(folder.join("tests/new_test.c").exists());
    let back = read(&session.join("back"));
    assert!(back.lines().any(|line| line == "uncommitted: tests/new_test.c") && back.lines().any(|line| line.starts_with("taken: ")), "{}", back);
}

#[test]
fn test_WB_VIEW_54_a_session_that_still_runs_is_not_taken_back() {
    let bench = bench();
    let (folder, session) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 1, commits: &[], uncommitted: &[], outcome: "outcome: running\n" });
    commit_in(&folder, "src/brake.c", "int brake(void) { return 16; }\n", "Tests");
    let lonely = repository(None);
    let mut child = Command::new(env!("CARGO_BIN_EXE_wb"))
        .arg("init")
        .current_dir(&lonely.path)
        .env_clear()
        .envs(env())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let pid = child.id();
    let is_wb = || fs::read_link(format!("/proc/{}/exe", pid)).ok().and_then(|exe| exe.file_name().map(|name| name == "wb")).unwrap_or(false);
    for _ in 0..500 {
        if is_wb() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(is_wb());
    fs::write(session.join("outcome"), format!("outcome: running\npid: {}\n", pid)).unwrap();
    let run = take_back(&bench, &[]);
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("still runs"), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["The issue's file of 54"]);
    assert!(folder.exists());
}

#[test]
fn test_WB_MON_14_what_a_take_back_does_is_appended_to_back_and_status_names_only_what_is_left() {
    let bench = bench();
    let (folder, session) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 17; }\n", "Good");
    commit_in(&folder, ".watchbill/x.yml", "x: 1\n", "Evil");
    fs::write(session.join("back"), "refused: 0123456789abcdef0123 Old: its parent is not\nview: kept ".to_string() + &folder.display().to_string() + "\n").unwrap();
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let back = read(&session.join("back"));
    assert!(back.starts_with("refused: 0123456789abcdef0123 Old"), "{}", back);
    assert!(back.lines().any(|line| line == "take-back") && back.lines().any(|line| line.starts_with("taken: ")), "{}", back);
    let run = wb_state(&bench, &["status", "54"]);
    let out = text_of(&run.stdout);
    assert!(out.lines().any(|line| line.starts_with("kept: tester round 1, 1 commit not taken, ")), "{}", out);
}

// ---- #223, the review of #266: a hostile blob fails fast, what is said is what is taken ----

/// Puts a FIFO in place of the loose object `object` of the view.
fn fifo_for_object(view: &Path, object: &str) {
    let path = view.join(".git/objects").join(&object[..2]).join(&object[2..]);
    fs::remove_file(&path).unwrap();
    assert!(Command::new("mkfifo").arg(&path).status().unwrap().success());
}

#[test]
fn test_WB_VIEW_55_a_fifo_for_a_blob_of_the_view_ends_a_take_back_within_10_seconds_and_takes_nothing() {
    let bench = bench();
    let (folder, _) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 18; }\n", "Tests");
    let blob = {
        let run = Command::new("git").arg("-C").arg(&folder).args(["rev-parse", "HEAD:src/brake.c"]).env_clear().envs(env()).output().unwrap();
        text(&run.stdout).trim().to_string()
    };
    fifo_for_object(&folder, &blob);
    let began = std::time::Instant::now();
    let run = take_back_bounded(&bench, 30).expect("the take-back did not end in 30 seconds");
    assert!(began.elapsed() < std::time::Duration::from_secs(10), "{:?}", began.elapsed());
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["The issue's file of 54"]);
    assert!(folder.exists());
}

/// A view with `count` commits to `src/brake.c`, made in one shell.
fn many_commits(folder: &Path, count: usize) {
    let body = format!(
        "for i in $(seq 1 {count}); do echo \"int brake$i;\" > src/brake.c; git -c user.name=Ada -c user.email=ada@example.org -c commit.gpgsign=false commit -q -a -m \"Commit $i\"; done"
    );
    let run = Command::new("sh").arg("-c").arg(body).current_dir(folder).env_clear().envs(env()).output().unwrap();
    assert!(run.status.success(), "{}", text(&run.stderr));
}

#[test]
fn test_WB_BUILD_83_take_back_says_the_first_200_commits_and_how_many_more_and_takes_all_of_them() {
    let bench = bench();
    let (folder, _) = kept_tester(&bench);
    many_commits(&folder, 201);
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(0), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    let said = err.split("taken onto").next().unwrap();
    assert!(said.contains("Commit 1\n") && said.contains("Commit 200\n") && !said.contains("Commit 201\n") && said.contains("and 1 more"), "{}", err);
    assert_eq!(subjects(&bench, 1), vec!["Commit 201"]);
    assert!(!folder.exists());
}

#[test]
fn test_WB_BUILD_83_take_back_takes_nothing_when_the_count_found_at_the_take_differs_from_the_one_said() {
    let bench = bench();
    let (folder, session) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 19; }\n", "One");
    commit_in(&folder, "src/brake.c", "int brake(void) { return 20; }\n", "Two");
    let file = read(&session.join("folder"));
    let get = |name: &str| file.lines().find_map(|line| line.strip_prefix(&format!("{}: ", name))).unwrap().to_string();
    let view = watchbill::view::View { folder: PathBuf::from(get("folder")), base: get("base"), head: get("head"), branch: get("branch") };
    let mut err = Vec::new();
    let back = watchbill::view::take_back_later("wb start tester", &bench.root(), &view, &["src/brake.c".to_string()], &[], 1, None, &mut err);
    assert!(back.failed.is_some() && back.taken.is_empty(), "{}", text_of(&err));
    assert_eq!(subjects(&bench, 1), vec!["The issue's file of 54"]);
    assert!(folder.exists());
}

#[test]
fn test_WB_BUILD_84_take_back_ends_with_1_where_its_lines_cannot_be_written_and_says_the_commits_are_on_the_branch() {
    let bench = bench();
    let (folder, session) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 21; }\n", "Tests");
    std::os::unix::fs::symlink("/dev/zero", session.join("back")).unwrap();
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("the commits are on the branch, but the lines of this take-back could not be written"), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["Tests"]);
}

#[test]
fn test_WB_VIEW_56_no_setting_of_the_person_or_the_system_is_read_by_a_git_run_in_the_temporary_store() {
    let bench = bench();
    let (folder, _) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 22; }\n", "Tests");
    let home = folder_named("home2");
    let marks = folder_named("marks2");
    let m = marks.path.to_str().unwrap();
    script(&marks.path.join("reference-transaction"), &format!("pwd >> {m}/ran\nexit 0\n"));
    fs::write(home.path.join(".gitconfig"), format!("[core]\n\thooksPath = {m}\n[user]\n\tname = Ada Lovelace\n\temail = ada@example.org\n")).unwrap();
    let run = Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(["start", "tester", "54", "--take-back"])
        .current_dir(&bench.repo.path)
        .env_clear()
        .envs(env())
        .env("HOME", &home.path)
        .env("XDG_STATE_HOME", &bench.state.path)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert_eq!(code(&run), Some(0), "{}", text_of(&run.stderr));
    let ran = fs::read_to_string(marks.path.join("ran")).unwrap_or_default();
    assert!(!ran.lines().any(|line| line.contains("wb-view-")), "{}", ran);
}
