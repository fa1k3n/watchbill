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
        // For the container and release checks (#202): a sandbox that cannot be made is a failure, not a skip.
        Err(why) if std::env::var_os("WB_TEST_REQUIRE_SANDBOX").is_some_and(|value| value == "1") => {
            panic!("{}: WB_TEST_REQUIRE_SANDBOX=1, and bubblewrap cannot make a sandbox here: {}", test, why)
        }
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
/// `ro` bound read-only and `~/.runner` bound from the home folder; it is cleared for the planner.
const ORG: &str = "levels: [public, internal]
default: stand-in
runners:
  stand-in:
    tools: this machine
    model: a stand-in
    agreement: none
    up_to: internal
    planner:
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
    // The stamp is pushed before the session (#236): to a bare repository of the test, never the network.
    push_beside(&repo.path);
    let log = folder_named("log");
    let org = folder_named("org");
    let home = folder_named("home");
    let ro = folder_named("ro");
    let state = folder_named("state");
    let l = log.path.to_str().unwrap();
    script(&log.path.join("post.sh"), &format!("echo \"post $1 $2\" >> {l}/calls\ncat > {l}/posted-$1-$2\nexit $(cat {l}/post.status 2>/dev/null || echo 0)\n"));
    script(&log.path.join("comments.sh"), &format!("echo \"comments $1 $2\" >> {l}/calls\ncat {l}/comments-$1-$2 2>/dev/null\nexit 0\n"));
    // The issue's text alone, by `issue_text`, and its whole comments, for the planner (#295): what the test leaves in
    // `text-<issue>` and `bodies-<issue>`, else a line of text and no comment. `issue_address` prints a line for a title.
    script(
        &log.path.join("text.sh"),
        &format!("echo \"text $1\" >> {l}/calls\nif [ -f {l}/text-$1 ]; then cat {l}/text-$1; else echo \"The text of issue $1.\"; fi\nexit $(cat {l}/text.status 2>/dev/null || echo 0)\n"),
    );
    // What the test leaves in `address-<issue>` and
    // `bodies-<issue>`, else a line of text and no comment.
    script(
        &log.path.join("address.sh"),
        &format!("echo \"address $1\" >> {l}/calls\nif [ -f {l}/address-$1 ]; then cat {l}/address-$1; else echo \"The text of issue $1.\"; fi\nexit $(cat {l}/address.status 2>/dev/null || echo 0)\n"),
    );
    script(&log.path.join("bodies.sh"), &format!("echo \"bodies $1\" >> {l}/calls\ncat {l}/bodies-$1 2>/dev/null\nexit $(cat {l}/bodies.status 2>/dev/null || echo 0)\n"));
    // The change's description alone, by `change_text`, and its whole comments, for the review and the fix (#153): what
    // the test leaves in `ctext-<change>` and `cbodies-<change>`, else a line of text and no comment.
    script(
        &log.path.join("ctext.sh"),
        &format!("echo \"ctext $1\" >> {l}/calls\nif [ -f {l}/ctext-$1 ]; then cat {l}/ctext-$1; else echo \"The description of change $1.\"; fi\nexit $(cat {l}/ctext.status 2>/dev/null || echo 0)\n"),
    );
    script(&log.path.join("cbodies.sh"), &format!("echo \"cbodies $1\" >> {l}/calls\ncat {l}/cbodies-$1 2>/dev/null\nexit $(cat {l}/cbodies.status 2>/dev/null || echo 0)\n"));
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
    // Nothing that needs a signature counts without `.watchbill/signers` on the default branch (#323). Ada also holds
    // the role `answer`, for the tests of `wb answer` after a start (#238).
    signing_as(&repo.path, "accept,start,answer");
    // Issue 99 has no file: its branch is there for a role started alone.
    git(&repo.path, &["branch", "issue-99"]);
    git(&repo.path, &["checkout", "-q", "-b", "issue-54"]);
    put(&repo.path, ".watchbill/issues/54.yml", issue_54);
    let run = wb(&repo.path, &["plan", "accept", "54", "--no-commit"], "");
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
        let mut err = Vec::new();
        let mut run = self.start_saying(sandbox, args, &mut err);
        run.stderr = err;
        run
    }

    /// The same, with standard error written to `err` as it is said, where a session can wait for it (#288); the
    /// `stderr` of the output is empty.
    fn start_saying(&self, sandbox: &Path, args: &[&str], err: &mut dyn std::io::Write) -> Output {
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
        let mut out = Vec::new();
        let status = watchbill::start_on(&args, &self.repo.path, &machine, &mut out, err);
        Output { status: std::process::ExitStatus::from_raw(status << 8), stdout: out, stderr: Vec::new() }
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
fn test_WB_VIEW_9_the_stamp_s_view_is_the_sha256_of_the_list_and_none_for_the_planner_and_a_role_with_no_file() {
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
    let planner = stamp(&["planner", "54", "--runner", "stand-in", "--round", "1", "--stamp"]);
    assert_eq!(line_of(&planner, "view").as_deref(), Some("none"), "{}", planner);
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

    // With both patterns `wb` pushes (#151): `--no-push` keeps the commands said and not run.
    let run = bench.start(&["start", "build", "54", "--no-push"]);

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
fn test_WB_BUILD_39_a_draft_change_the_session_wrote_and_did_not_commit_is_not_said() {
    let bench = bench();
    git(&bench.repo.path, &["checkout", "-q", "issue-99"]);
    with_line(&bench, "draft_change: 'draft-stand-in --head {branch}'");
    // The session commits in its view and changes the checkout's file, as only a session outside the sandbox could.
    bench.session("(cd \"$VIEW\" && echo 'int fresh;' > src/new.c && git add src/new.c && git -c commit.gpgsign=false commit -q -m 'A new file') && echo \"draft_change: 'evil {branch}'\" >> .watchbill/watchbill.yml\n");

    let run = bench.start_by(&pass_through(&bench), &["start", "build", "99"]);

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
    // A role with no issue's file runs in a view of the whole tree (#202): its guard is the whole tree, and `--ignore-local` passes it.
    put(&bench.repo.path, "src/brake.h", "int brake(int);\n");
    // On the branch of issue 99, which the commits come back onto (#296).
    git(&bench.repo.path, &["checkout", "-q", "issue-99"]);
    ok(&bench.start_by(&pass_through(&bench), &["start", "build", "99", "--ignore-local"]));
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
fn test_WB_BUILD_133_the_refusal_of_a_local_change_names_the_paths_with_their_list_and_the_flag() {
    let bench = bench();
    put(&bench.repo.path, "src/brake.h", "int brake(int);\n");
    put(&bench.repo.path, "src/brake.c", "int brake(void) { return 2; }\n");
    let run = start_unsandboxed(&bench, &["start", "build", "54"]);
    let err = guarded(&bench, &run);
    // A change under `changes` makes it a limit (#180): the flag is named for what it passes, a change under `reads`.
    assert!(err.lines().any(|line| line.starts_with("wb start build: limit: ")) && err.contains("--ignore-local"), "{}", err);
    assert!(err.lines().any(|line| line.contains("changes") && line.contains("src/brake.c")), "{}", err);
    assert!(err.lines().any(|line| line.contains("reads") && line.contains("src/brake.h")), "{}", err);
    // The outcome of the lines is a refusal at a check of its own.
    let outcome = read(&bench.lines().join("outcome"));
    assert!(outcome.contains("outcome: refused\ncheck: local\nstatus: 1"), "{}", outcome);
}

#[test]
fn test_WB_BUILD_133_a_path_with_a_line_break_is_named_escaped() {
    let bench = bench();
    put(&bench.repo.path, "include/a\nb.h", "#define A 3\n");
    git(&bench.repo.path, &["add", "--", "include/a\nb.h"]);
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "build", "54"]));
    assert!(err.contains("include/a\\nb.h") && !err.contains("a\nb.h"), "{}", err);
}

// ---- #180: each refusal of the start says limit or guard, and a guard names the flag that passes it ----

/// The first line of standard error that starts with `<name>: <kind>: `.
fn labelled_line(err: &str, name: &str, kind: &str) -> Option<String> {
    let start = format!("{}: {}: ", name, kind);
    err.lines().find(|line| line.starts_with(&start)).map(str::to_string)
}

/// The kept view of the tester for 54 that holds a commit not taken back.
fn kept_with_a_commit(bench: &Bench) {
    keep(bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 1, commits: &["Tests"], uncommitted: &[], outcome: RAN });
}

#[test]
fn test_WB_BUILD_130_a_kept_view_is_refused_as_a_guard_that_names_ignore_kept() {
    let bench = bench();
    kept_with_a_commit(&bench);
    let run = start_unsandboxed(&bench, &["start", "build", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    let line = labelled_line(&err, "wb start build", "guard").unwrap_or_else(|| panic!("no guard: {}", err));
    assert!(line.contains("is kept"), "{}", err);
    assert!(err.contains("--ignore-kept"), "{}", err);
    assert!(labelled_line(&err, "wb start build", "limit").is_none(), "{}", err);
}

#[test]
fn test_WB_CMD_29_ignore_kept_passes_the_guard_of_a_kept_view_and_no_other_refusal() {
    let bench = bench();
    kept_with_a_commit(&bench);
    // Past the guard, the start goes on to the view, where no bubblewrap is: a limit.
    let run = start_unsandboxed(&bench, &["start", "build", "54", "--ignore-kept"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(labelled_line(&err, "wb start build", "guard").is_none() && !err.contains("is kept"), "{}", err);
    assert!(labelled_line(&err, "wb start build", "limit").is_some(), "{}", err);
}

#[test]
fn test_WB_BUILD_131_a_local_change_under_reads_only_is_refused_as_a_guard_that_names_ignore_local() {
    let bench = bench();
    put(&bench.repo.path, "src/brake.h", "int brake(int);\n");
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "build", "54"]));
    let line = labelled_line(&err, "wb start build", "guard").unwrap_or_else(|| panic!("no guard: {}", err));
    assert!(line.contains("local change"), "{}", err);
    assert!(err.contains("--ignore-local"), "{}", err);
}

#[test]
fn test_WB_CMD_29_ignore_local_passes_the_guard_of_a_local_change_under_reads_and_no_other_refusal() {
    let bench = bench();
    put(&bench.repo.path, "src/brake.h", "int brake(int);\n");
    let run = start_unsandboxed(&bench, &["start", "build", "54", "--ignore-local"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(labelled_line(&err, "wb start build", "guard").is_none() && !err.contains("local change"), "{}", err);
    assert!(labelled_line(&err, "wb start build", "limit").is_some(), "{}", err);
}

#[test]
fn test_WB_BUILD_132_a_local_change_under_changes_is_refused_as_a_limit_also_with_ignore_local() {
    let bench = bench();
    put(&bench.repo.path, "src/brake.c", "int brake(void) { return 2; }\n");
    for words in [&["start", "build", "54"][..], &["start", "build", "54", "--ignore-local"][..]] {
        let err = guarded(&bench, &start_unsandboxed(&bench, words));
        let line = labelled_line(&err, "wb start build", "limit").unwrap_or_else(|| panic!("no limit: {}", err));
        assert!(line.contains("local change"), "{}", err);
        assert!(labelled_line(&err, "wb start build", "guard").is_none(), "{}", err);
    }
}

#[test]
fn test_WB_BUILD_134_a_commit_refused_at_the_take_back_is_said_as_a_limit() {
    let bench = bench();
    let (folder, _) = kept_tester(&bench);
    commit_in(&folder, "src/brake.h", "int brake(int);\n", "Change a read");
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    let line = labelled_line(&err, "wb start tester", "limit").unwrap_or_else(|| panic!("no limit: {}", err));
    assert!(line.contains("refused, and not taken") && line.contains("Change a read"), "{}", err);
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

/// A `bwrap` that makes no sandbox but names the session on `--info-fd` as bubblewrap does: its own child, a
/// reaper, has the session as its one child. It writes the info only once `wb` has taken the test's signal (`sent`
/// in `log`), so that the signal comes before the session is found (#271), or, at the latest, a second and a half
/// after it started, by the clock, within the 2 seconds `wb` looks for the session (#288).
fn slow_info_bwrap(bench: &Bench) -> PathBuf {
    let path = bench.at("bwrap-slow-info");
    let log = bench.log.path.to_str().unwrap();
    script(
        &path,
        &format!(
            "view=\nfd=\nwhile [ $# -gt 0 ]; do\n  case \"$1\" in\n    --info-fd) fd=$2; shift 2;;\n    --bind) view=$2; shift 3;;\n    --) shift; break;;\n    *) shift;;\n  esac\ndone\n[ \"$1\" = /usr/bin/true ] && exit 0\ncd \"$view\" || exit 1\n( \"$@\" & wait $! ) &\nreaper=$!\n(sleep 1.5; : > {log}/late) &\nwhile [ ! -e {log}/sent ] && [ ! -e {log}/late ]; do sleep 0.01; done\necho \"{{\\\"child-pid\\\": $reaper}}\" >&\"$fd\"\nwait $reaper\nexit $?\n"
        ),
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
const HANGS_UP_ALONE: &str = "trap 'sleep 1; echo got > {log}/got; kill $!; exit 0' HUP\nsleep 30 &\necho $$ > {log}/pid\nwait\n";

/// A session that commits and ends by a termination it sends itself, with no one signalling `wb`.
const ENDS_ITSELF: &str = "echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\necho $$ > {log}/pid\nsleep 0.5\nkill -TERM $$\n";

/// A session that writes the signals it started with ignored, as `/proc` gives them.
const SAYS_IGNORED: &str = "grep SigIgn /proc/self/status > {log}/ignored\necho $$ > {log}/pid\n";

// ---- #271: in a view Ctrl-C and Ctrl-\ reach the session inside bubblewrap ----

const SIGINT: i32 = 2;
const SIGQUIT: i32 = 3;

/// A session that commits, and at an interrupt takes a second, writes `got` and ends by the interrupt.
const CLEANLY_INT: &str = "echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\ntrap 'sleep 1; echo got > {log}/got; kill $!; trap - INT; kill -INT $$' INT\nsleep 30 &\necho $$ > {log}/pid\nwait\n";

/// The same at a quit, with no core written.
const CLEANLY_QUIT: &str = "ulimit -c 0\necho 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\ntrap 'sleep 1; echo got > {log}/got; kill $!; trap - QUIT; kill -QUIT $$' QUIT\nsleep 30 &\necho $$ > {log}/pid\nwait\n";

/// A session with no repository, run by `view::run`: at an interrupt it takes a second, writes `got` and ends by it.
const INT_ALONE: &str = "trap 'sleep 1; echo got > {log}/got; kill $!; trap - INT; kill -INT $$' INT\nsleep 30 &\necho $$ > {log}/pid\nwait\n";

/// The same at a quit.
const QUIT_ALONE: &str = "ulimit -c 0\ntrap 'sleep 1; echo got > {log}/got; kill $!; trap - QUIT; kill -QUIT $$' QUIT\nsleep 30 &\necho $$ > {log}/pid\nwait\n";

/// A session with no repository that writes a line `int` for each interrupt it gets, and ends by itself, writing
/// `got`, once `wb` has taken the test's signal (`sent`), so that one that `wb` sent on came before (#288). An
/// interrupt to the group kills the `sleep` of the moment, not the session. A minute is the ceiling.
const COUNTS_INTS: &str = "trap 'echo int >> {log}/ints' INT\necho $$ > {log}/pid\nn=0\nwhile [ ! -e {log}/sent ] && [ $n -lt 1200 ]; do sleep 0.05; n=$((n+1)); done\necho got > {log}/got\n";

/// A session with no repository that ends by itself, writing `got`, once `wb` has taken the test's signal (`sent`)
/// and has said that the session was not found (`said`), so that a kill of bubblewrap would have come before (#288).
/// A minute is the ceiling.
const ENDS_LATER: &str = "echo $$ > {log}/pid\nn=0\nwhile { [ ! -e {log}/sent ] || ! grep -q 'not found in bubblewrap' {log}/said 2>/dev/null; } && [ $n -lt 1200 ]; do sleep 0.05; n=$((n+1)); done\necho got > {log}/got\n";

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
        Ok("cleanly-int") => CLEANLY_INT,
        Ok("cleanly-quit") => CLEANLY_QUIT,
        Ok("int-alone") => INT_ALONE,
        Ok("quit-alone") => QUIT_ALONE,
        Ok("counts-ints") => COUNTS_INTS,
        Ok("ends-later") => ENDS_LATER,
        _ => WAITS,
    });
    let sandbox = match std::env::var(CHILD_SANDBOX).as_deref() {
        Ok("real") | Ok("run") | Ok("run-in-group") => PathBuf::from(watchbill::view::BWRAP),
        Ok("run-slow-info") => slow_info_bwrap(&bench),
        // The stand-in that writes nothing on the info fd, through `view::run`, with and without `--new-session`.
        Ok("run-stand-in") | Ok("run-stand-in-in-group") => stand_in_bwrap(&bench),
        _ => stand_in_bwrap(&bench),
    };
    let paths = format!("{}\n{}\n{}\n", bench.repo.path.display(), bench.log.path.display(), bench.state.path.display());
    fs::write(folder.join("paths.tmp"), paths).unwrap();
    fs::rename(folder.join("paths.tmp"), folder.join("paths")).unwrap();
    let through_run = std::env::var(CHILD_SANDBOX).ok().filter(|how| ["run", "run-in-group", "run-slow-info", "run-stand-in", "run-stand-in-in-group"].contains(&how.as_str()));
    let status = if let Some(how) = through_run {
        // The session in bubblewrap with no controlling terminal, as where `dev.tty.legacy_tiocsti` is not 0 (#257),
        // or, with `run-in-group`, in the process group of `wb` (#271).
        let view = folder_named("view");
        let words = [bench.at("session.sh").to_str().unwrap().to_string()];
        let plan = watchbill::view::Plan {
            view: &view.path,
            checkout: &bench.repo.path,
            binds: vec![(bench.log.path.clone(), true)],
            organisation: None,
            home: Some(&bench.home.path),
            new_session: !how.ends_with("in-group"),
            words: &words,
        };
        // What `wb` says goes to `said` in the log folder, where a session can wait for it (#288).
        let mut said = fs::File::create(bench.log.path.join("said")).unwrap();
        match watchbill::view::run_saying(&sandbox, &plan, "wb start", &mut said) {
            Ok(status) => {
                use std::os::unix::process::ExitStatusExt;
                // The signal that ended the session, as `wb start` names it in `outcome` (#271).
                if let Some(number) = status.signal() {
                    fs::write(folder.join("signal"), format!("{}\n", number)).unwrap();
                }
                status.code().unwrap_or(-3)
            }
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
    /// A line `int` for each interrupt a session that counts them got.
    ints: String,
    /// The signal that ended the session, by its number, where `view::run` ran it.
    signal: String,
}

/// Starts `wb start build 54`, sends `to_wb` to it when the session waits, then, once `wb` has taken it, `to_session` to
/// the session when there is one, and gathers what was left. Nothing waits longer than a minute.
fn signalled(session_is: &str, to_wb: Option<i32>, to_session: Option<i32>) -> Signalled {
    signalled_how(How { session_is, sandbox: "stand-in", to_wb, to_group: false, to_session, hangup_ignored: false })
}

/// How `signalled_how` runs `wb` and signals it.
struct How<'a> {
    session_is: &'a str,
    /// `stand-in`, `real`, `run`, `run-in-group`, `run-slow-info`, `run-stand-in` or `run-stand-in-in-group`, as
    /// `CHILD_SANDBOX` says.
    sandbox: &'a str,
    to_wb: Option<i32>,
    /// Whether `to_wb` goes to the whole process group of `wb`, as a terminal sends a hangup, and not to `wb` alone.
    to_group: bool,
    to_session: Option<i32>,
    /// Whether `wb` is started with SIGHUP ignored, as by `nohup`.
    hangup_ignored: bool,
}

fn signalled_how(how: How) -> Signalled {
    signalled_ignoring(how, &[])
}

/// The same, with `wb` started with the signals `ignored` ignored too, as by a shell that runs it in the
/// background (#271).
fn signalled_ignoring(how: How, ignored: &[i32]) -> Signalled {
    use std::os::unix::process::CommandExt;
    let How { session_is, sandbox, to_wb, to_group, to_session, hangup_ignored } = how;
    let mut ignored = ignored.to_vec();
    if hangup_ignored {
        ignored.push(SIGHUP);
    }
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
    // The held signals start at their default, whatever this test was started with (`nohup cargo test`, a
    // background job), and then those of `ignored` are ignored. Only `signal(2)`, which may be called between
    // fork and exec.
    unsafe {
        command.pre_exec(move || {
            for number in [SIGINT, SIGQUIT, SIGHUP, SIGTERM] {
                signal(number, 0);
            }
            for number in &ignored {
                signal(*number, 1);
            }
            Ok(())
        });
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
            // `wb` has taken it once its handler has run: then, and not after a time, the session is signalled, and
            // a session or a stand-in that waits for `sent` goes on (#288).
            if !wait_for(60, || taken(child.id() as i32, signal)) {
                return Err("wb did not take the signal within a minute".to_string());
            }
            fs::write(log.join("sent"), "").unwrap();
        }
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
            ints: read_if(&log.join("ints")),
            signal: read_if(&folder.join("signal")),
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

/// Whether process `pid` has no `signal` waiting for it, as `ShdPnd` of `/proc/<pid>/status` says: one sent to the
/// process is taken there until a thread of it takes it and runs its handler. A process that has ended has none.
fn taken(pid: i32, signal: i32) -> bool {
    let status = read_if(Path::new(&format!("/proc/{}/status", pid)));
    let Some(mask) = status.lines().find_map(|line| line.strip_prefix("ShdPnd:")) else { return true };
    u64::from_str_radix(mask.trim(), 16).map(|mask| mask & (1 << (signal - 1)) == 0).unwrap_or(false)
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
    // The stand-in writes nothing on the info fd, and the session runs until `wb` has said the line (#288).
    let slow = bench();
    slow.session(UNTIL_SAID);
    let sandbox = stand_in_bwrap(&slow);
    let said = slow.log.path.join("said");
    slow.start_saying(&sandbox, &["start", "build", "54"], &mut fs::File::create(&said).unwrap());
    let err = read(&said);
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
fn test_WB_BUILD_87_a_session_that_ends_as_the_info_fd_closes_is_not_said_to_be_lost() {
    // A process that ends closes its files before it can be waited for: `wb` sees the info end and the process still
    // there (#273). The stand-in closes the info fd and ends a moment later, as a loaded machine makes `exit 0` do.
    let bench = bench();
    bench.session("exit 0\n");
    let path = bench.at("bwrap-closing");
    script(&path, "fd=\nwhile [ $# -gt 0 ]; do\n  case \"$1\" in\n    --info-fd) fd=$2; shift 2;;\n    --) shift; break;;\n    *) shift;;\n  esac\ndone\n[ \"$1\" = /usr/bin/true ] && exit 0\neval \"exec $fd>&-\"\nsleep 0.1\nexit 0\n");
    let run = bench.start_by(&path, &["start", "build", "54"]);
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

#[test]
fn test_WB_VIEW_59_an_interrupt_to_the_group_of_wb_ends_neither_bubblewrap_nor_the_session_under_it() {
    // Ctrl-C: the terminal sends the interrupt to the whole foreground group. `wb start` gives the session a
    // controlling terminal or none as the kernel says; with none, `wb` sends it on (WB-BUILD-107).
    let how = How { session_is: "cleanly-int", sandbox: "real", to_wb: Some(SIGINT), to_group: true, to_session: None, hangup_ignored: false };
    let Some(found) = in_bubblewrap("test_WB_VIEW_59_an_interrupt", how) else { return };
    assert_eq!(found.got, "got\n", "the session did not end by itself: {}", found.outcome);
    assert_eq!(found.subjects.lines().next(), Some("A new file"), "{}", found.subjects);
    assert!(found.back.lines().any(|line| line.starts_with("taken: ") && line.contains("A new file")), "{}", found.back);
    assert!(found.outcome.lines().any(|line| line == "signal: SIGINT"), "{}", found.outcome);
    assert_eq!(found.status, 1);
}

#[test]
fn test_WB_VIEW_59_a_quit_to_the_group_of_wb_ends_neither_bubblewrap_nor_the_session_under_it() {
    // Ctrl-\, as Ctrl-C.
    let how = How { session_is: "cleanly-quit", sandbox: "real", to_wb: Some(SIGQUIT), to_group: true, to_session: None, hangup_ignored: false };
    let Some(found) = in_bubblewrap("test_WB_VIEW_59_a_quit", how) else { return };
    assert_eq!(found.got, "got\n", "the session did not end by itself: {}", found.outcome);
    assert_eq!(found.subjects.lines().next(), Some("A new file"), "{}", found.subjects);
    assert!(found.back.lines().any(|line| line.starts_with("taken: ") && line.contains("A new file")), "{}", found.back);
    assert!(found.outcome.lines().any(|line| line == "signal: SIGQUIT"), "{}", found.outcome);
    assert_eq!(found.status, 1);
}

#[test]
fn test_WB_BUILD_107_with_no_controlling_terminal_an_interrupt_to_the_group_reaches_the_session() {
    // The session is in a terminal session of its own (`--new-session`): the group's interrupt reaches `wb` and
    // bubblewrap only, and `wb` sends it on. The session ends by it, which `wb start` names in `outcome`.
    let how = How { session_is: "int-alone", sandbox: "run", to_wb: Some(SIGINT), to_group: true, to_session: None, hangup_ignored: false };
    let Some(found) = in_bubblewrap("test_WB_BUILD_107_an_interrupt", how) else { return };
    assert_eq!(found.got, "got\n", "the session was not sent the interrupt, or was killed under it");
    assert_eq!(found.signal, "2\n", "the session did not end by the interrupt (status {})", found.status);
}

#[test]
fn test_WB_BUILD_107_with_no_controlling_terminal_a_quit_to_the_group_reaches_the_session() {
    let how = How { session_is: "quit-alone", sandbox: "run", to_wb: Some(SIGQUIT), to_group: true, to_session: None, hangup_ignored: false };
    let Some(found) = in_bubblewrap("test_WB_BUILD_107_a_quit", how) else { return };
    assert_eq!(found.got, "got\n", "the session was not sent the quit, or was killed under it");
    assert_eq!(found.signal, "3\n", "the session did not end by the quit (status {})", found.status);
}

#[test]
fn test_WB_BUILD_108_with_a_controlling_terminal_the_session_gets_an_interrupt_to_the_group_once() {
    // No `--new-session`: the session is in the group, and gets the interrupt there. `wb` does not send it on.
    let how = How { session_is: "counts-ints", sandbox: "run-in-group", to_wb: Some(SIGINT), to_group: true, to_session: None, hangup_ignored: false };
    let Some(found) = in_bubblewrap("test_WB_BUILD_108", how) else { return };
    assert_eq!(found.got, "got\n", "the session did not end by itself");
    assert_eq!(found.ints, "int\n", "the session did not get the interrupt once");
    assert_eq!(found.status, 0);
}

#[test]
fn test_WB_BUILD_108_with_a_controlling_terminal_an_interrupt_to_wb_alone_is_not_sent_on() {
    // If `wb` sent it on, the session would get one; it gets none.
    let how = How { session_is: "counts-ints", sandbox: "run-in-group", to_wb: Some(SIGINT), to_group: false, to_session: None, hangup_ignored: false };
    let Some(found) = in_bubblewrap("test_WB_BUILD_108_to_wb_alone", how) else { return };
    assert_eq!(found.got, "got\n", "the session did not end by itself");
    assert_eq!(found.ints, "", "the interrupt was sent on to the session");
    assert_eq!(found.status, 0);
}

#[test]
fn test_WB_BUILD_109_where_bubblewrap_names_no_session_an_interrupt_to_wb_ends_bubblewrap_and_the_session() {
    // The stand-in writes nothing on the info fd and runs the session itself, which waits 30 seconds, with no
    // controlling terminal. The interrupt is sent to `wb` alone, so only `wb` can end it.
    let how = How { session_is: "waits", sandbox: "run-stand-in", to_wb: Some(SIGINT), to_group: false, to_session: None, hangup_ignored: false };
    let found = signalled_how(how);
    assert_ne!(found.status, -2, "wb had not ended after a minute");
    assert_eq!(found.signal, "9\n", "bubblewrap was not killed (status {})", found.status);
}

#[test]
fn test_WB_BUILD_109_where_bubblewrap_names_no_session_a_quit_to_wb_ends_bubblewrap_and_the_session() {
    let how = How { session_is: "waits", sandbox: "run-stand-in", to_wb: Some(SIGQUIT), to_group: false, to_session: None, hangup_ignored: false };
    let found = signalled_how(how);
    assert_ne!(found.status, -2, "wb had not ended after a minute");
    assert_eq!(found.signal, "9\n", "bubblewrap was not killed (status {})", found.status);
}

#[test]
fn test_WB_BUILD_109_with_a_controlling_terminal_an_interrupt_to_wb_does_not_kill_bubblewrap() {
    // No `--new-session`: the session would get Ctrl-C from the terminal itself, so bubblewrap is not killed
    // under it. The session runs on and ends by itself.
    let how = How { session_is: "ends-later", sandbox: "run-stand-in-in-group", to_wb: Some(SIGINT), to_group: false, to_session: None, hangup_ignored: false };
    let found = signalled_how(how);
    assert_eq!(found.got, "got\n", "bubblewrap was killed under the session (status {}, signal {:?})", found.status, found.signal);
    assert_eq!(found.status, 0);
}

/// A session that runs until `wb` has said, in `said` of the log folder, that it was not found, and then ends with
/// 0: it does not race the time `wb` looks for it (#288). A minute is the ceiling.
const UNTIL_SAID: &str = "n=0\nwhile ! grep -q 'not found in bubblewrap' {log}/said 2>/dev/null && [ $n -lt 1200 ]; do sleep 0.05; n=$((n+1)); done\nexit 0\n";

/// What `view::run_saying` says, with the stand-in that writes nothing on the info fd and a session that runs until
/// it is said (`UNTIL_SAID`), with `--new-session` or without it.
fn said_for_lost(new_session: bool) -> String {
    let slow = bench();
    slow.session(UNTIL_SAID);
    let sandbox = stand_in_bwrap(&slow);
    let view = folder_named("view");
    let words = [slow.at("session.sh").to_str().unwrap().to_string()];
    let plan = watchbill::view::Plan {
        view: &view.path,
        checkout: &slow.repo.path,
        binds: vec![(slow.log.path.clone(), true)],
        organisation: None,
        home: Some(&slow.home.path),
        new_session,
        words: &words,
    };
    let said = slow.log.path.join("said");
    let status = watchbill::view::run_saying(&sandbox, &plan, "wb start build", &mut fs::File::create(&said).unwrap()).unwrap();
    assert!(status.success(), "{:?}: {}", status, read(&said));
    read(&said)
}

#[test]
fn test_WB_BUILD_110_the_line_for_a_session_not_found_says_what_an_interrupt_and_a_hangup_do() {
    let err = said_for_lost(true);
    let said: Vec<&str> = err.lines().filter(|line| line.contains("was not found in bubblewrap")).collect();
    assert_eq!(said.len(), 1, "{}", err);
    assert!(said[0].contains("a hangup does not reach it either"), "{}", err);
    assert!(said[0].contains("an interrupt or a quit (Ctrl-C, Ctrl-\\) ends bubblewrap and the session with it"), "{}", err);
    // With a controlling terminal the session gets those from the terminal, and the line does not name them.
    let err = said_for_lost(false);
    let said: Vec<&str> = err.lines().filter(|line| line.contains("was not found in bubblewrap")).collect();
    assert_eq!(said.len(), 1, "{}", err);
    assert!(!said[0].contains("hangup") && !said[0].contains("interrupt"), "{}", err);
}

#[test]
fn test_WB_BUILD_111_an_interrupt_that_comes_before_the_session_is_found_is_sent_on_when_it_is() {
    // The stand-in names the session only once the interrupt has been sent to `wb` alone. With no controlling
    // terminal, `wb` sends it on when the session is found: the session ends by it, not killed.
    let how = How { session_is: "int-alone", sandbox: "run-slow-info", to_wb: Some(SIGINT), to_group: false, to_session: None, hangup_ignored: false };
    let found = signalled_how(how);
    assert_eq!(found.got, "got\n", "the session was not sent the interrupt, or was killed");
    assert_eq!(found.signal, "2\n", "the session did not end by the interrupt (status {})", found.status);
}

#[test]
fn test_WB_BUILD_111_a_quit_that_comes_before_the_session_is_found_is_sent_on_when_it_is() {
    let how = How { session_is: "quit-alone", sandbox: "run-slow-info", to_wb: Some(SIGQUIT), to_group: false, to_session: None, hangup_ignored: false };
    let found = signalled_how(how);
    assert_eq!(found.got, "got\n", "the session was not sent the quit, or was killed");
    assert_eq!(found.signal, "3\n", "the session did not end by the quit (status {})", found.status);
}

#[test]
fn test_WB_VIEW_60_the_session_in_a_view_starts_with_the_interrupt_and_the_quit_as_wb_found_them() {
    let how = How { session_is: "says-ignored", sandbox: "real", to_wb: None, to_group: false, to_session: None, hangup_ignored: false };
    let Some(found) = in_bubblewrap("test_WB_VIEW_60", how) else { return };
    assert!(!ignores(&found.ignored, 2) && !ignores(&found.ignored, 3), "{}", found.ignored);
    // As by a shell that runs `wb` in the background with no job control: the interrupt stays ignored, the quit
    // does not.
    let how = How { session_is: "says-ignored", sandbox: "real", to_wb: None, to_group: false, to_session: None, hangup_ignored: false };
    let found = signalled_ignoring(how, &[SIGINT]);
    assert!(ignores(&found.ignored, 2) && !ignores(&found.ignored, 3), "{}", found.ignored);
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
            &["-c", "user.name=Ada Lovelace", "-c", "user.email=ada@example.org", "-c", "commit.gpgsign=false", "-c", "maintenance.auto=false", "commit", "-q", "-m", subject],
        );
    }
    for path in keep.uncommitted {
        put(&made.folder, path, "left\n");
    }
    let session = bench.state.path.join("watchbill").join("sessions").join(keep.session);
    fs::create_dir_all(&session).unwrap();
    // The stamp's `view:`, as the start of that role would have posted it: a take-back decides by it (#202).
    let issue = keep.session.rsplit('-').next().unwrap();
    let view = view_line_of(bench, keep.role, issue);
    fs::write(session.join("stamp"), format!("[stamp] {} round {}\nview: {}\n", keep.role, keep.round, view)).unwrap();
    fs::write(session.join("outcome"), keep.outcome).unwrap();
    fs::write(
        session.join("folder"),
        format!("folder: {}\nbase: {}\nhead: {}\nbranch: {}\nissue: {}\n", made.folder.display(), made.base, made.head, made.branch, keep.session.rsplit('-').next().unwrap()),
    )
    .unwrap();
    (made.folder, session)
}

const RAN: &str = "outcome: ended\nstatus: 0\nseconds: 5\n";

/// The line `view:` of the stamp of `role` for `issue`, as `wb prompt --stamp` gives it, or `none`.
fn view_line_of(bench: &Bench, role: &str, issue: &str) -> String {
    let mut words = vec![role, issue, "--round", "1", "--stamp"];
    match role {
        "planner" => words.extend(["--runner", "stand-in"]),
        "review" | "fix" => words.extend(["--change", "174"]),
        _ => {}
    }
    let run = prompt_with(&bench.repo.path, &bench.organisation(), &words);
    line_of(&text_of(&run.stdout), "view").unwrap_or_else(|| "none".to_string())
}

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
    // On the branch of issue 99, which the commits come back onto (#296).
    git(&bench.repo.path, &["checkout", "-q", "issue-99"]);
    bench.session("cat {state}/watchbill/sessions/*/outcome > {log}/during\n");
    // The state folder is not in a sandbox: the stand-in makes none (#202).
    let run = bench.start_by(&pass_through(&bench), &["start", "build", "99"]);
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
fn test_WB_VIEW_132_outcome_says_running_before_folder_is_written_and_before_the_stamp_is_posted() {
    let bench = bench();
    git(&bench.repo.path, &["checkout", "-q", "issue-99"]);
    // The stand-in that posts the stamp looks at the folder of the lines when the post begins.
    let (l, st) = (bench.log.path.to_str().unwrap().to_string(), bench.state.path.to_str().unwrap().to_string());
    script(
        &bench.at("post.sh"),
        &format!("cat > {l}/posted-$1-$2\nfor d in {st}/watchbill/sessions/*; do ls $d > {l}/at-post; cat $d/outcome > {l}/outcome-at-post; done\nexit 0\n"),
    );
    bench.session("exit 0\n");
    let run = bench.start_by(&pass_through(&bench), &["start", "build", "99"]);
    ok(&run);
    let at_post = bench.left("at-post");
    let names: Vec<&str> = at_post.lines().collect();
    assert!(names.contains(&"outcome") && names.contains(&"folder"), "{}", at_post);
    let outcome = bench.left("outcome-at-post");
    let mut lines = outcome.lines();
    assert_eq!(lines.next(), Some("outcome: running"), "{}", outcome);
    assert!(lines.next().is_some_and(|line| line.starts_with("pid: ")), "{}", outcome);
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
    git(&folder, &["-c", "user.name=Ada Lovelace", "-c", "user.email=ada@example.org", "-c", "commit.gpgsign=false", "-c", "maintenance.auto=false", "commit", "-q", "-F", message.to_str().unwrap()]);
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
    assert!(text_of(&run.stdout).contains("kept: tester round 2, 1 commit not taken, 1 path uncommitted"), "{}", text_of(&run.stdout));
    assert_eq!(fs::read_dir(&temporary.path).unwrap().count(), 0);
}

// ---- #223: a kept view is taken back with `wb start <role> <issue> --take-back` ----

/// A commit made in a kept view, as a session makes it.
fn commit_in(view: &Path, path: &str, text: &str, subject: &str) {
    put(view, path, text);
    git(view, &["add", "--", path]);
    git(view, &["-c", "user.name=Ada Lovelace", "-c", "user.email=ada@example.org", "-c", "commit.gpgsign=false", "-c", "maintenance.auto=false", "commit", "-q", "-m", subject]);
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
fn test_WB_CMD_40_take_back_q_leaves_out_how_to_see_the_commit_and_keeps_what_is_not_pushed() {
    let bench = bench();
    let (folder, _) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 8; }\n", "The tests of 54");
    let run = take_back(&bench, &["-q"]);
    let err = text_of(&run.stderr);
    assert_eq!(code(&run), Some(0), "{}", err);
    assert!(err.contains("will take back") && err.contains("The tests of 54"), "{}", err);
    assert!(!err.contains("git show --stat"), "{}", err);
    // What is not pushed, and the command to push it, are said together, and stay.
    assert!(err.contains("nothing is pushed") && err.contains("to push the branch, run"), "{}", err);
    assert_eq!(subjects(&bench, 1), vec!["The tests of 54"]);
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
fn test_WB_VIEW_114_a_kept_view_is_taken_back_onto_its_stamp_s_branch_whichever_branch_is_checked_out() {
    // Changed by #296 from the test of WB-VIEW-50, which it replaces.
    let bench = bench();
    let (folder, _) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 12; }\n", "Tests");
    git(&bench.repo.path, &["checkout", "-q", "-b", "other"]);
    let other = bench.head();
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(0), "{}", text_of(&run.stderr));
    assert_eq!(bench.git_text(&["log", "-1", "--format=%s", "issue-54"]).trim(), "Tests");
    assert_eq!(bench.git_text(&["rev-parse", "issue-54~1"]).trim(), other);
    assert_eq!(bench.git_text(&["symbolic-ref", "HEAD"]).trim(), "refs/heads/other");
    assert_eq!(bench.head(), other);
    assert_eq!(read(&bench.repo.path.join("src/brake.c")), "int brake(void) { return 1; }\n");
    assert!(!folder.exists());
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
    // Other tests of this process make and remove stores of their own at the same time, under the same name and
    // process number. A store is this one's when its `alternates` names the objects of this view, and only such a
    // store is looked for.
    let objects = format!("{}\n", folder.join(".git/objects").display());
    let mut temporary = Vec::new();
    for _ in 0..300 {
        temporary = fs::read_dir(std::env::temp_dir())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
            .filter(|entry| fs::read_to_string(entry.path().join("objects/info/alternates")).is_ok_and(|text| text == objects))
            .collect::<Vec<_>>();
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

// ---- #265: what `wb` says of a kept view it could not read is true, and a view has one deadline ----

/// A folder with a `git` that runs `before` (a shell line, with the words of the run as `$@`) and then the real
/// one, and the `PATH` that finds it first. The folder is held until the test ends.
fn stand_in_git(before: &str) -> (Folder, String) {
    let folder = folder_named("stand-in");
    script(&folder.path.join("git"), &format!("{}\nexec /usr/bin/git \"$@\"\n", before));
    let path = format!("{}:{}", folder.path.display(), std::env::var("PATH").unwrap_or_default());
    (folder, path)
}

/// `wb status 54` with a `PATH`, with a bound, and how long it took.
fn status_with_path(bench: &Bench, path: &str, seconds: u64) -> Option<(Output, std::time::Duration)> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wb"));
    command.args(["status", "54"]).current_dir(&bench.repo.path).env_clear().envs(env()).env("XDG_STATE_HOME", &bench.state.path).env("PATH", path);
    bounded(command, seconds)
}

#[test]
fn test_WB_STATUS_70_a_time_out_of_merge_base_is_said_as_a_time_out_naming_it() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &[], outcome: "outcome: running\npid: 1\n" });
    fifo_for_head(&folder);
    let run = wb_state_bounded(&bench, &["status", "54"], 20).expect("wb status did not end in 20 seconds");
    let out = text_of(&run.stdout);
    let line = out.lines().find(|line| line.starts_with("kept: tester round 2, not read: ")).unwrap_or_else(|| panic!("{}", out));
    assert!(line.contains("`git merge-base` took more than 3 seconds"), "{}", line);
    assert!(!line.contains("does not come from"), "{}", line);
}

/// `wb status 54` with a `PATH` and the two values that only a build with debug assertions reads (#279): the deadline of
/// one view and the bound of one git run, in seconds.
fn status_with_bounds(bench: &Bench, path: &str, deadline: &str, run_bound: &str, seconds: u64) -> Option<(Output, std::time::Duration)> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wb"));
    command
        .args(["status", "54"])
        .current_dir(&bench.repo.path)
        .env_clear()
        .envs(env())
        .env("XDG_STATE_HOME", &bench.state.path)
        .env("PATH", path)
        .env("WB_TEST_VIEW_DEADLINE", deadline)
        .env("WB_TEST_RUN_BOUND", run_bound);
    bounded(command, seconds)
}

#[test]
fn test_WB_STATUS_71_a_view_slow_on_every_git_run_is_not_read_within_the_deadline_of_one_view() {
    if !cfg!(debug_assertions) {
        eprintln!("skipped: a build without debug assertions does not read the test values (WB-STATUS-76)");
        return;
    }
    let bench = bench();
    keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &["a.c"], outcome: "outcome: running\npid: 1\n" });
    // The deadline of the view is 2 seconds and the bound of a run 30: the stand-in waits on every run on the view
    // until it is ended, so that the deadline ends it, and no sleep is near a bound however slow the machine is (#279).
    let (_held, path) = stand_in_git(r#"case "$*" in *--git-dir=*|*wb-view-*) exec sleep 1000;; esac"#);
    let (run, took) = status_with_bounds(&bench, &path, "2", "30", 40).expect("wb status did not end in 40 seconds");
    let out = text_of(&run.stdout);
    assert!(out.lines().any(|line| line.starts_with("kept: tester round 2, not read: ") && line.contains("2 seconds in all")), "{}", out);
    assert!(took < std::time::Duration::from_secs(25), "{:?}", took);
}

#[test]
fn test_WB_STATUS_71_a_view_slow_on_every_git_run_has_the_deadline_of_20_seconds_where_no_deadline_is_set() {
    if !cfg!(debug_assertions) {
        eprintln!("skipped: a build without debug assertions does not read the test values (WB-STATUS-76)");
        return;
    }
    let bench = bench();
    keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &["a.c"], outcome: "outcome: running\npid: 1\n" });
    let (_held, path) = stand_in_git(r#"case "$*" in *--git-dir=*|*wb-view-*) exec sleep 1000;; esac"#);
    // Only the bound of a run is set, to 30 seconds: the deadline of the view is its own 20, and ends the run (#279).
    let mut command = Command::new(env!("CARGO_BIN_EXE_wb"));
    command.args(["status", "54"]).current_dir(&bench.repo.path).env_clear().envs(env()).env("XDG_STATE_HOME", &bench.state.path).env("PATH", &path).env("WB_TEST_RUN_BOUND", "30");
    let (run, took) = bounded(command, 40).expect("wb status did not end in 40 seconds");
    let out = text_of(&run.stdout);
    assert!(out.lines().any(|line| line.starts_with("kept: tester round 2, not read: ") && line.contains("20 seconds in all")), "{}", out);
    assert!(took < std::time::Duration::from_secs(25), "{:?}", took);
}

#[test]
fn test_WB_STATUS_76_a_build_without_debug_assertions_ignores_the_test_values_of_the_deadline_and_the_bound() {
    if cfg!(debug_assertions) {
        eprintln!("skipped: only a build without debug assertions, as the RPM's, shows that the values are ignored");
        return;
    }
    let bench = bench();
    keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &["a.c"], outcome: "outcome: running\npid: 1\n" });
    let (_held, path) = stand_in_git(r#"case "$*" in *--git-dir=*|*wb-view-*) exec sleep 1000;; esac"#);
    // A deadline of 1 second and a bound of 30 would end the run in 1 second, and say so. `wb` as installed ends it
    // after its own 3 seconds.
    let (run, took) = status_with_bounds(&bench, &path, "1", "30", 40).expect("wb status did not end in 40 seconds");
    let out = text_of(&run.stdout);
    let line = out.lines().find(|line| line.starts_with("kept: tester round 2, not read: ")).unwrap_or_else(|| panic!("{}", out));
    assert!(line.contains("took more than 3 seconds"), "{}", line);
    assert!(!line.contains("in all"), "{}", line);
    assert!(took >= std::time::Duration::from_secs(3), "{:?}", took);
}

#[test]
fn test_WB_STATUS_72_the_time_out_of_a_git_run_names_the_command_and_not_an_option() {
    let bench = bench();
    keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &["a.c"], outcome: "outcome: running\npid: 1\n" });
    // The paths left uncommitted are read by runs that start with `-c`.
    let (_held, path) = stand_in_git("case \"$*\" in *read-tree*) sleep 6;; esac");
    let (run, _) = status_with_path(&bench, &path, 40).expect("wb status did not end in 40 seconds");
    let out = text_of(&run.stdout);
    assert!(out.contains("`git read-tree` took more than 3 seconds"), "{}", out);
    assert!(!out.contains("`git -c`"), "{}", out);
}

#[test]
fn test_WB_VIEW_70_a_fifo_for_a_blob_that_the_checkout_also_holds_ends_a_take_back_in_about_3_seconds() {
    let bench = bench();
    let (folder, _) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 70; }\n", "Tests");
    let blob = {
        let run = Command::new("git").arg("-C").arg(&folder).args(["rev-parse", "HEAD:src/brake.c"]).env_clear().envs(env()).output().unwrap();
        text(&run.stdout).trim().to_string()
    };
    // The checkout holds the same blob, as a session can make it do by committing a file equal to one it has.
    let copy = bench.log.path.join("same.c");
    fs::write(&copy, "int brake(void) { return 70; }\n").unwrap();
    let held = Command::new("git").arg("-C").arg(&bench.repo.path).args(["hash-object", "-w"]).arg(&copy).env_clear().envs(env()).output().unwrap();
    assert_eq!(text(&held.stdout).trim(), blob);
    fifo_for_object(&folder, &blob);
    let began = std::time::Instant::now();
    let run = take_back_bounded(&bench, 30).expect("the take-back did not end in 30 seconds");
    assert!(began.elapsed() < std::time::Duration::from_secs(8), "{:?}", began.elapsed());
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["The issue's file of 54"]);
    assert!(folder.exists());
}

#[test]
fn test_WB_STATUS_69_a_view_of_more_than_200_commits_says_and_k_more_in_status_and_in_the_refusal() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 1, commits: &[], uncommitted: &[], outcome: "outcome: running\npid: 1\n" });
    many_commits(&folder, 205);
    let out = text_of(&wb_state(&bench, &["status", "54"]).stdout);
    assert!(out.lines().any(|line| line.starts_with("kept: tester round 1, 200 commits not taken and 5 more,")), "{}", out);
    let run = start_54(&bench, &[]);
    assert_eq!(code(&run), Some(1));
    let err = text_of(&run.stderr);
    assert!(err.contains("Commit 200\n    and 5 more\n"), "{}", err);
    assert!(!err.contains("Commit 201"), "{}", err);
}

#[test]
fn test_WB_STATUS_75_a_missing_head_commit_is_said_as_the_failure_of_merge_base_and_not_as_another_origin() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-tester-54", role: "tester", round: 2, commits: &["Tests"], uncommitted: &[], outcome: "outcome: running\npid: 1\n" });
    let head = {
        let run = Command::new("git").arg("-C").arg(&folder).args(["rev-parse", "HEAD"]).env_clear().envs(env()).output().unwrap();
        text(&run.stdout).trim().to_string()
    };
    fs::remove_file(folder.join(".git/objects").join(&head[..2]).join(&head[2..])).unwrap();
    let run = wb_state_bounded(&bench, &["status", "54"], 20).expect("wb status did not end in 20 seconds");
    let out = text_of(&run.stdout);
    let line = out.lines().find(|line| line.starts_with("kept: tester round 2, not read: ")).unwrap_or_else(|| panic!("{}", out));
    assert!(line.contains("`git merge-base` ended with 128"), "{}", line);
    assert!(!line.contains("does not come from"), "{}", line);
}

// ---- #202: every session that `wb start` starts runs in a sandbox ----

/// A stand-in for bubblewrap that makes no sandbox: it runs the words after `--` in the folder it was started in,
/// the checkout, with `VIEW` naming the folder bound at it, so that a session can change the checkout while it
/// commits in its view. For what only a session outside the sandbox could do.
fn pass_through(bench: &Bench) -> PathBuf {
    let stand_in = bench.at("bwrap");
    script(
        &stand_in,
        "here=$(pwd -P)\nVIEW=\nwhile [ $# -gt 0 ]; do\n  case \"$1\" in\n    --) shift; break ;;\n    --bind|--ro-bind)\n      if [ -z \"$VIEW\" ] && [ \"$3\" = \"$here\" ]; then VIEW=\"$2\"; fi\n      shift 3 ;;\n    *) shift ;;\n  esac\ndone\nexport VIEW\nexec \"$@\"\n",
    );
    stand_in
}

/// The bench with the branch of issue 99, which has no issue's file, checked out.
fn on_99() -> Bench {
    let bench = bench();
    git(&bench.repo.path, &["checkout", "-q", "issue-99"]);
    bench
}

/// The paths of `HEAD` of the checkout, one to a line, as `git ls-files` lists them in a view.
fn head_paths(bench: &Bench) -> String {
    bench.git_text(&["ls-tree", "-r", "--name-only", "HEAD"])
}

/// What the session of `GIT_WRITER` wrote in the `.git` it had did not reach the checkout's: no hook, no ref, no
/// replace ref, no setting, and `HEAD` where it was.
fn git_of_the_checkout_untouched(bench: &Bench, head: &str) {
    assert_eq!(bench.left("done"), "done\n", "the session did not run to its end");
    assert!(!bench.repo.path.join(".git/hooks/pre-commit").exists(), "a hook reached the checkout");
    assert_eq!(bench.git_text(&["for-each-ref", "refs/heads/evil", "refs/replace/"]), "");
    let hooks = Command::new("git").arg("-C").arg(&bench.repo.path).args(["config", "--get", "core.hooksPath"]).env_clear().envs(env()).output().unwrap();
    assert!(!hooks.status.success(), "{}", text_of(&hooks.stdout));
    assert_eq!(bench.head(), head);
}

/// A session that writes a hook, a ref, a replace ref and a setting in the `.git` it has.
const GIT_WRITER: &str = "mkdir -p .git/hooks && printf '#!/bin/sh\\necho hook >> {log}/hook\\n' > .git/hooks/pre-commit && chmod +x .git/hooks/pre-commit\n\
git update-ref refs/heads/evil HEAD\n\
other=$(git commit-tree -m evil 'HEAD^{tree}') && git replace HEAD \"$other\"\n\
git config core.hooksPath /tmp/evil\n\
echo done > {log}/done\n";

#[test]
fn test_WB_VIEW_73_the_planner_runs_in_a_view_of_every_file_of_head_with_no_history() {
    if !sandbox("test_WB_VIEW_73") {
        return;
    }
    let bench = on_99();
    bench.session("pwd > {log}/pwd\ngit ls-files > {log}/files\ngit rev-list --all --count > {log}/count\ngit remote > {log}/remotes\n");

    let run = bench.start(&["start", "planner", "99"]);

    ok(&run);
    assert_eq!(bench.left("pwd").trim(), bench.root());
    assert_eq!(bench.left("files"), head_paths(&bench));
    assert!(bench.left("files").contains("src/secret.c\n"), "{}", bench.left("files"));
    assert_eq!(bench.left("count").trim(), "1");
    assert_eq!(bench.left("remotes"), "");
    assert!(text_of(&run.stderr).contains("in bubblewrap"), "{}", text_of(&run.stderr));
}

#[test]
fn test_WB_VIEW_94_in_the_planner_s_view_a_build_may_write_outside_watchbill_and_it_stays_in_the_view() {
    if !sandbox("test_WB_VIEW_94") {
        return;
    }
    // A stand-in for `cargo test -- --list`: it writes `Cargo.lock` and `target/`, and changes a file of the code.
    let bench = on_99();
    bench.session(
        "echo lock > Cargo.lock && mkdir -p target/debug && echo built > target/debug/tests && echo yes > {log}/built\n\
         echo x > new.txt && echo x >> src/brake.c && echo yes > {log}/code\n",
    );

    let run = bench.start(&["start", "planner", "99"]);

    assert!(bench.started(), "{}", text_of(&run.stderr));
    assert_eq!(bench.left("built"), "yes\n");
    assert_eq!(bench.left("code"), "yes\n");
    for path in ["Cargo.lock", "target", "new.txt"] {
        assert!(!bench.repo.path.join(path).exists(), "{} reached the checkout", path);
    }
    assert_eq!(read(&bench.repo.path.join("src/brake.c")), "int brake(void) { return 1; }\n");
}

#[test]
fn test_WB_VIEW_95_in_the_planner_s_view_watchbill_is_read_only_but_for_its_issues_folder() {
    if !sandbox("test_WB_VIEW_95") {
        return;
    }
    let bench = on_99();
    let yml = read(&config(&bench.repo.path));
    bench.session(
        "echo x >> .watchbill/watchbill.yml 2> /dev/null && echo yes > {log}/yml\n\
         echo x >> .watchbill/revision/rules.yml 2> /dev/null && echo yes > {log}/revision\n\
         cat .watchbill/issue/99.md > {log}/read\n\
         echo x >> .watchbill/issue/99.md 2> /dev/null && echo yes > {log}/brought\n\
         mkdir .watchbill/other 2> /dev/null && echo yes > {log}/other\n\
         echo 'issue: x' > .watchbill/issues/99.yml && echo yes > {log}/issue\n",
    );

    let run = bench.start(&["start", "planner", "99"]);

    assert!(bench.started(), "{}", text_of(&run.stderr));
    for name in ["yml", "revision", "brought", "other"] {
        assert_eq!(bench.left(name), "", "{} was written", name);
    }
    assert_eq!(bench.left("issue"), "yes\n");
    // The issue's text the program brought in is there to read, and not to write.
    assert_eq!(bench.left("read"), "The text of issue 99.\n");
    assert_eq!(read(&config(&bench.repo.path)), yml);
}

#[test]
fn test_WB_VIEW_75_what_the_planner_writes_in_its_git_does_not_reach_the_checkout() {
    if !sandbox("test_WB_VIEW_75") {
        return;
    }
    let bench = on_99();
    let head = bench.head();
    bench.session(GIT_WRITER);

    ok(&bench.start(&["start", "planner", "99"]));

    git_of_the_checkout_untouched(&bench, &head);
}

#[test]
fn test_WB_VIEW_75_what_a_role_with_no_issue_s_file_writes_in_its_git_does_not_reach_the_checkout() {
    if !sandbox("test_WB_VIEW_75") {
        return;
    }
    let bench = on_99();
    let head = bench.head();
    bench.session(GIT_WRITER);

    ok(&bench.start(&["start", "build", "99"]));

    git_of_the_checkout_untouched(&bench, &head);
}

#[test]
fn test_WB_VIEW_76_the_planner_s_commit_of_its_issue_s_file_comes_back_onto_the_branch() {
    if !sandbox("test_WB_VIEW_76") {
        return;
    }
    let bench = on_99();
    let before = bench.head();
    bench.session("printf 'issue: 99\\n' > .watchbill/issues/99.yml && git add .watchbill/issues/99.yml && git commit -q -m 'The issue file of 99'\n");

    let run = bench.start(&["start", "planner", "99"]);

    ok(&run);
    assert_eq!(subjects(&bench, 1), vec!["The issue file of 99"]);
    assert_eq!(bench.git_text(&["show", "HEAD:.watchbill/issues/99.yml"]), "issue: 99\n");
    assert_eq!(bench.git_text(&["rev-parse", "HEAD~1"]).trim(), before);
    assert_eq!(bench.git_text(&["show", "--name-only", "--format=", "HEAD"]), ".watchbill/issues/99.yml\n");
    assert!(text_of(&run.stderr).contains("git push origin issue-99"), "{}", text_of(&run.stderr));
    assert!(bench.below("views").is_empty(), "{:?}", bench.below("views"));
}

#[test]
fn test_WB_VIEW_77_a_commit_of_the_planner_that_changes_any_other_path_is_refused_and_named() {
    if !sandbox("test_WB_VIEW_77") {
        return;
    }
    // One commit that changes the issue's file and, by git's plumbing, a file of the code.
    let bench = on_99();
    let before = bench.head();
    bench.session(
        "blob=$(echo evil | git hash-object -w --stdin) && git update-index --cacheinfo \"100644,$blob,src/brake.c\"\n\
         printf 'issue: 99\\n' > .watchbill/issues/99.yml && git add .watchbill/issues/99.yml && git commit -q -m 'Both'\n",
    );
    let run = bench.start(&["start", "planner", "99"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("src/brake.c"), "{}", text_of(&run.stderr));
    assert_eq!(bench.head(), before);

    // The issue's file first, then the file of another issue: the first comes back, the second is refused.
    let bench = on_99();
    let before = bench.head();
    bench.session(
        "printf 'issue: 99\\n' > .watchbill/issues/99.yml && git add .watchbill/issues/99.yml && git commit -q -m 'The file of 99'\n\
         printf 'issue: 54\\n' > .watchbill/issues/54.yml && git add .watchbill/issues/54.yml && git commit -q -m 'The file of 54'\n",
    );
    let run = bench.start(&["start", "planner", "99"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains(".watchbill/issues/54.yml"), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["The file of 99"]);
    assert_eq!(bench.git_text(&["rev-parse", "HEAD~1"]).trim(), before);
}

#[test]
fn test_WB_VIEW_78_a_local_change_or_an_untracked_file_anywhere_stops_the_planner() {
    // A file of the code changed, and not committed.
    let bench = on_99();
    put(&bench.repo.path, "src/secret.c", "changed here\n");
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "planner", "99"]));
    assert!(err.contains("src/secret.c") && err.contains("guard"), "{}", err);
    // A file staged, and one deleted.
    let bench = on_99();
    put(&bench.repo.path, "include/a.h", "#define A 2\n");
    git(&bench.repo.path, &["add", "--", "include/a.h"]);
    fs::remove_file(bench.repo.path.join("build/Makefile")).unwrap();
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "planner", "99"]));
    assert!(err.contains("include/a.h") && err.contains("build/Makefile"), "{}", err);
    // An untracked file that git does not ignore.
    let bench = on_99();
    put(&bench.repo.path, "notes.txt", "mine\n");
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "planner", "99"]));
    assert!(err.contains("notes.txt"), "{}", err);
}

#[test]
fn test_WB_VIEW_78_an_untracked_file_that_git_ignores_does_not_stop_the_planner() {
    let bench = on_99();
    commit_file(&bench, ".gitignore", "*.o\n");
    put(&bench.repo.path, "src/brake.o", "an object\n");
    // The start goes on to the view, which is where no bubblewrap is.
    let err = text_of(&start_unsandboxed(&bench, &["start", "planner", "99"]).stderr);
    assert!(err.contains("bubblewrap") && !err.contains("guard") && !err.contains("brake.o"), "{}", err);
}

#[test]
fn test_WB_VIEW_79_ignore_local_passes_a_change_that_is_not_the_issue_s_file() {
    let bench = on_99();
    put(&bench.repo.path, "src/secret.c", "changed here\n");
    put(&bench.repo.path, "notes.txt", "mine\n");
    let err = text_of(&start_unsandboxed(&bench, &["start", "planner", "99", "--ignore-local"]).stderr);
    assert!(err.contains("bubblewrap") && !err.contains("guard"), "{}", err);
}

#[test]
fn test_WB_VIEW_79_no_flag_passes_a_change_to_the_issue_s_file() {
    // Untracked.
    let bench = on_99();
    put(&bench.repo.path, ".watchbill/issues/99.yml", "issue: 99\n");
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "planner", "99", "--ignore-local"]));
    assert!(err.contains(".watchbill/issues/99.yml") && err.contains("cannot be overridden"), "{}", err);
    // Untracked, and ignored by the checkout.
    let bench = on_99();
    commit_file(&bench, ".gitignore", "*.yml\n");
    put(&bench.repo.path, ".watchbill/issues/99.yml", "issue: 99\n");
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "planner", "99", "--ignore-local"]));
    assert!(err.contains(".watchbill/issues/99.yml") && err.contains("cannot be overridden"), "{}", err);
    // Changed, where it is in HEAD: the planner of 54 again.
    let bench = self::bench();
    put(&bench.repo.path, ".watchbill/issues/54.yml", &format!("{}# changed\n", ISSUE_54));
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "planner", "54", "--ignore-local"]));
    assert!(err.contains(".watchbill/issues/54.yml") && err.contains("cannot be overridden"), "{}", err);
}

#[test]
fn test_WB_VIEW_79_the_stamp_of_a_planner_that_ignored_local_changes_says_how_many() {
    if !sandbox("test_WB_VIEW_79") {
        return;
    }
    let bench = on_99();
    put(&bench.repo.path, "src/secret.c", "changed here\n");
    put(&bench.repo.path, "notes.txt", "mine\n");
    ok(&bench.start(&["start", "planner", "99", "--ignore-local"]));
    let posted = bench.left("posted-issue-99");
    assert_eq!(line_of(&posted, "local").as_deref(), Some("ignored 2"), "{}", posted);
}

#[test]
fn test_WB_VIEW_80_a_role_with_no_issue_s_file_runs_in_a_writable_view_of_every_file_of_head() {
    if !sandbox("test_WB_VIEW_80") {
        return;
    }
    let bench = on_99();
    bench.session("pwd > {log}/pwd\ngit ls-files > {log}/files\ngit rev-list --all --count > {log}/count\necho x > src/written.c && echo yes > {log}/written\n");

    let run = bench.start(&["start", "build", "99"]);

    ok(&run);
    assert_eq!(bench.left("pwd").trim(), bench.root());
    assert_eq!(bench.left("files"), head_paths(&bench));
    assert_eq!(bench.left("count").trim(), "1");
    assert_eq!(bench.left("written"), "yes\n");
    assert!(!bench.repo.path.join("src/written.c").exists());
    assert!(text_of(&run.stderr).contains("in bubblewrap"), "{}", text_of(&run.stderr));
}

#[test]
fn test_WB_VIEW_80_a_review_of_a_change_with_no_issue_runs_in_a_view_of_every_file_of_head() {
    if !sandbox("test_WB_VIEW_80") {
        return;
    }
    let bench = bench();
    bench.session("pwd > {log}/pwd\ngit ls-files > {log}/files\ngit rev-list --all --count > {log}/count\n");

    let run = bench.start(&["start", "review", "--change", "174"]);

    ok(&run);
    assert_eq!(bench.left("pwd").trim(), bench.root());
    assert_eq!(bench.left("files"), head_paths(&bench));
    assert_eq!(bench.left("count").trim(), "1");
}

#[test]
fn test_WB_VIEW_81_in_the_view_of_a_role_with_no_issue_s_file_watchbill_cannot_be_written() {
    if !sandbox("test_WB_VIEW_81") {
        return;
    }
    let bench = on_99();
    let yml = read(&config(&bench.repo.path));
    bench.session(
        "echo x >> .watchbill/watchbill.yml 2> /dev/null && echo yes > {log}/yml\n\
         mkdir -p .watchbill/issues 2> /dev/null; echo x > .watchbill/issues/99.yml 2> /dev/null && echo yes > {log}/issue\n\
         test -f .watchbill/watchbill.yml && echo yes > {log}/there\n",
    );

    ok(&bench.start(&["start", "build", "99"]));

    assert_eq!(bench.left("there"), "yes\n");
    assert_eq!(bench.left("yml"), "");
    assert_eq!(bench.left("issue"), "");
    assert_eq!(read(&config(&bench.repo.path)), yml);
}

#[test]
fn test_WB_VIEW_82_the_commits_of_a_role_with_no_issue_s_file_come_back_with_the_same_commands() {
    if !sandbox("test_WB_VIEW_82") {
        return;
    }
    let bench = on_99();
    with_line(&bench, "draft_change: 'draft-stand-in --head {branch}'");
    bench.session("echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\n");

    // With both patterns `wb` pushes (#151): `--no-push` keeps the commands said and not run.
    let run = bench.start(&["start", "build", "99", "--no-push"]);

    ok(&run);
    assert_eq!(subjects(&bench, 1), vec!["A new file"]);
    assert_eq!(bench.git_text(&["show", "HEAD:src/new.c"]), "int fresh;\n");
    let err = text_of(&run.stderr);
    assert!(err.contains("git show --stat"), "{}", err);
    assert!(err.contains("git push origin issue-99"), "{}", err);
    assert!(err.contains("draft-stand-in --head issue-99"), "{}", err);
    assert!(!bench.at("network").exists());

    // A session that commits nothing gets no command.
    let bench = on_99();
    let run = bench.start(&["start", "build", "99"]);
    ok(&run);
    assert!(!text_of(&run.stderr).contains("git push"), "{}", text_of(&run.stderr));
}

#[test]
fn test_WB_VIEW_83_a_commit_of_a_role_with_no_issue_s_file_under_build_files_or_watchbill_is_refused() {
    if !sandbox("test_WB_VIEW_83") {
        return;
    }
    let bench = on_99();
    let before = bench.head();
    bench.session("echo 'all: evil' > build/Makefile && git commit -q -a -m 'The build'\n");
    let run = bench.start(&["start", "build", "99"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(err.contains("build/Makefile") && err.contains("build_files"), "{}", err);
    assert_eq!(bench.head(), before);

    // Under `.watchbill/`, by git's plumbing, after a commit that keeps to the rule.
    let bench = on_99();
    let before = bench.head();
    bench.session(
        "echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\n\
         blob=$(echo evil | git hash-object -w --stdin) && git update-index --cacheinfo \"100644,$blob,.watchbill/watchbill.yml\" && git commit -q -m 'Evil'\n",
    );
    let run = bench.start(&["start", "build", "99"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains(".watchbill/watchbill.yml"), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["A new file"]);
    assert_eq!(bench.git_text(&["rev-parse", "HEAD~1"]).trim(), before);
}

#[test]
fn test_WB_VIEW_84_before_a_view_of_the_whole_tree_starts_it_says_how_many_files_and_bytes() {
    if !sandbox("test_WB_VIEW_84") {
        return;
    }
    for role in ["build", "planner"] {
        let bench = on_99();
        let listed = bench.git_text(&["ls-tree", "-r", "-l", "HEAD"]);
        let files = listed.lines().count();
        let bytes: u64 = listed.lines().map(|line| line.split_whitespace().nth(3).unwrap().parse::<u64>().unwrap()).sum();

        let run = bench.start(&["start", role, "99"]);

        ok(&run);
        let err = text_of(&run.stderr);
        let said = format!("wb start {}: the view holds the whole tree of HEAD: {} files, {} bytes", role, files, bytes);
        let at = err.lines().position(|line| line == said).unwrap_or_else(|| panic!("{} not in: {}", said, err));
        let starts = err.lines().position(|line| line.starts_with(&format!("wb start {}: starts {}", role, role))).unwrap();
        assert!(at < starts, "{}", err);
    }
}

#[test]
fn test_WB_VIEW_88_take_back_takes_a_kept_view_of_the_planner_by_its_issue_s_file_alone() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-planner-54", role: "planner", round: 1, commits: &[], uncommitted: &[], outcome: RAN });
    commit_in(&folder, ".watchbill/issues/54.yml", "issue: '54'\n", "The file again");
    let run = bench.start(&["start", "planner", "54", "--take-back"]);
    assert_eq!(code(&run), Some(0), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["The file again"]);

    let bench = self::bench();
    let before = bench.head();
    let (folder, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-planner-54", role: "planner", round: 1, commits: &[], uncommitted: &[], outcome: RAN });
    commit_in(&folder, "src/brake.c", "int brake(void) { return 9; }\n", "The code");
    let run = bench.start(&["start", "planner", "54", "--take-back"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("src/brake.c"), "{}", text_of(&run.stderr));
    assert_eq!(bench.head(), before);
    assert!(folder.exists());
}

#[test]
fn test_WB_VIEW_88_take_back_takes_a_kept_view_of_a_role_with_no_issue_s_file_by_its_rule() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-build-99", role: "build", round: 1, commits: &[], uncommitted: &[], outcome: RAN });
    commit_in(&folder, "src/other.c", "int other;\n", "Another file");
    let run = bench.start(&["start", "build", "99", "--take-back"]);
    assert_eq!(code(&run), Some(0), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["Another file"]);

    let bench = self::bench();
    let before = bench.head();
    let (folder, _) = keep(&bench, &Keep { session: "2026-11-22T100000Z-build-99", role: "build", round: 1, commits: &[], uncommitted: &[], outcome: RAN });
    commit_in(&folder, "build/Makefile", "all: evil\n", "The build");
    let run = bench.start(&["start", "build", "99", "--take-back"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("build/Makefile"), "{}", text_of(&run.stderr));
    assert_eq!(bench.head(), before);
}

// ---- The review of #276 (#202) ----

#[test]
fn test_WB_VIEW_91_a_local_change_stops_a_role_with_no_issue_s_file_and_ignore_local_passes_it() {
    let bench = on_99();
    put(&bench.repo.path, "src/secret.c", "changed here\n");
    put(&bench.repo.path, "notes.txt", "mine\n");
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "build", "99"]));
    assert!(err.contains("src/secret.c") && err.contains("notes.txt") && err.contains("guard"), "{}", err);
    // `--ignore-local` passes every one: the start goes on to the view, which is where no bubblewrap is.
    let err = text_of(&start_unsandboxed(&bench, &["start", "build", "99", "--ignore-local"]).stderr);
    assert!(err.contains("bubblewrap") && !err.contains("guard"), "{}", err);
}

#[test]
fn test_WB_VIEW_92_a_commit_that_adds_a_path_untracked_in_the_checkout_is_refused_and_nothing_is_written_over() {
    if !sandbox("test_WB_VIEW_92") {
        return;
    }
    // Ignored in the checkout, so the guard leaves it out.
    let bench = on_99();
    commit_file(&bench, ".gitignore", "*.env\n");
    put(&bench.repo.path, "local.env", "the person's own\n");
    let before = bench.head();
    bench.session("echo session > local.env && git add -f local.env && git commit -q -m 'An env'\n");
    let run = bench.start(&["start", "build", "99"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("local.env"), "{}", text_of(&run.stderr));
    assert_eq!(read(&bench.repo.path.join("local.env")), "the person's own\n");
    assert_eq!(bench.head(), before);

    // An ignored file where the commit puts a folder: git takes it as expendable, and would put the folder there.
    let bench = on_99();
    commit_file(&bench, ".gitignore", "*.env\n");
    put(&bench.repo.path, "out.env", "the person's own\n");
    let before = bench.head();
    bench.session("mkdir out.env && echo session > out.env/a && git add -f out.env/a && git commit -q -m 'A folder'\n");
    let run = bench.start(&["start", "build", "99"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("out.env"), "{}", text_of(&run.stderr));
    assert_eq!(read(&bench.repo.path.join("out.env")), "the person's own\n");
    assert_eq!(bench.head(), before);

    // An ignored link where the commit puts a folder: the link is looked at, never followed.
    let bench = on_99();
    commit_file(&bench, ".gitignore", "*.env\n");
    let elsewhere = folder_named("elsewhere");
    std::os::unix::fs::symlink(&elsewhere.path, bench.repo.path.join("link.env")).unwrap();
    let before = bench.head();
    bench.session("mkdir link.env && echo session > link.env/a && git add -f link.env/a && git commit -q -m 'Through a link'\n");
    let run = bench.start(&["start", "build", "99"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("link.env"), "{}", text_of(&run.stderr));
    assert!(fs::symlink_metadata(bench.repo.path.join("link.env")).unwrap().file_type().is_symlink());
    assert_eq!(fs::read_dir(&elsewhere.path).unwrap().count(), 0);
    assert_eq!(bench.head(), before);

    // Not ignored, and passed by `--ignore-local`.
    let bench = on_99();
    put(&bench.repo.path, "notes.txt", "mine\n");
    let before = bench.head();
    bench.session("echo session > notes.txt && git add notes.txt && git commit -q -m 'Notes'\n");
    let run = bench.start(&["start", "build", "99", "--ignore-local"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("notes.txt"), "{}", text_of(&run.stderr));
    assert_eq!(read(&bench.repo.path.join("notes.txt")), "mine\n");
    assert_eq!(bench.head(), before);
}

#[test]
fn test_WB_VIEW_93_take_back_refuses_where_the_stamp_s_view_and_head_differ() {
    // The stamp says no list, and HEAD's issue's file gives one.
    let bench = bench();
    let before = bench.head();
    let (folder, session) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 7; }\n", "Tests");
    fs::write(session.join("stamp"), "[stamp] tester round 1\nview: none\n").unwrap();
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("view: none"), "{}", text_of(&run.stderr));
    assert_eq!(bench.head(), before);
    assert!(folder.exists());

    // The issue's file is taken out of HEAD after the view was made: the rule its session ran under is not known.
    let bench = self::bench();
    let (folder, _) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 8; }\n", "Tests");
    git(&bench.repo.path, &["rm", "-q", "--", ".watchbill/issues/54.yml", ".watchbill/issues/54.accepted"]);
    git(&bench.repo.path, &["commit", "-q", "-m", "No file"]);
    let before = bench.head();
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("view: none"), "{}", text_of(&run.stderr));
    assert_eq!(bench.head(), before);
}

#[test]
fn test_WB_PROMPT_72_the_planner_s_stamp_holds_the_level_after_the_view_line() {
    let bench = bench();
    let run = prompt_with(&bench.repo.path, &bench.organisation(), &["planner", "54", "--runner", "stand-in", "--round", "1", "--stamp"]);
    ok(&run);
    let stamp = text_of(&run.stdout);
    let names: Vec<&str> = stamp.lines().map(|line| line.split(':').next().unwrap()).collect();
    let view = names.iter().position(|name| *name == "view").unwrap();
    assert_eq!(names[view + 1], "level", "{}", stamp);
    assert_eq!(line_of(&stamp, "level").as_deref(), Some("public"), "{}", stamp);
}

#[test]
fn test_WB_PROMPT_72_the_posted_stamp_of_a_role_with_no_issue_s_file_holds_the_level() {
    if !sandbox("test_WB_PROMPT_72") {
        return;
    }
    let bench = on_99();
    ok(&bench.start(&["start", "build", "99"]));
    let posted = bench.left("posted-issue-99");
    assert_eq!(line_of(&posted, "level").as_deref(), Some("public"), "{}", posted);
}

// ---- #295: the planner reads its issue without a login, and `wb` posts its questions ----

/// The settings with the line of `key` taken out, committed so that `HEAD` has them.
fn without_line(bench: &Bench, key: &str) {
    let yml = read(&config(&bench.repo.path));
    let kept: String = yml.lines().filter(|line| !line.starts_with(&format!("{}:", key))).map(|line| format!("{}\n", line)).collect();
    fs::write(config(&bench.repo.path), kept).unwrap();
    commit_config_keeping_refs(&bench.repo.path);
}

/// Refused before anything is posted or started, with `named` on standard error as a limit, and no view left.
fn refused_before_posting(bench: &Bench, run: &Output, named: &[&str]) {
    assert_eq!(code(run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(err.contains("limit:"), "{}", err);
    for name in named {
        assert!(err.contains(name), "{} not in: {}", name, err);
    }
    assert!(!bench.any_posted() && !bench.started(), "{:?}", bench.calls());
    assert!(bench.below("views").is_empty(), "{:?}", bench.below("views"));
}

/// The issue's file of 54 as a planner writes it with two questions, each in full (#295).
const TWO_QUESTIONS: &str = "issue: '54'
from: 3f0c1d9e
requirements: []
interface: |
  none
changes: [src/brake.c]
reads:
  tester: []
  build: []
  review: []
  fix: []
existing_tests_changed: []
questions:
  - |
    54.q1: Which brake does the issue mean?
    Why it matters: the tests check one brake or the other.

    1. (planner) The front brake.
    2. (planner) The rear brake.

    Suggested: 1, because the issue names the front brake first.
  - |
    54.q2: Is a test of the timing asked for?
    Why it matters: a test of the timing needs the clock stubbed.

    1. (planner) No timing test.
    2. (planner) A timing test, with the clock stubbed.

    Suggested: 2, because the issue speaks of a deadline.
";

/// The comment of questions `wb` posts for `TWO_QUESTIONS`.
const TWO_POSTED: &str = "[questions] 54 q1 q2
q1: Which brake does the issue mean?
<details>

Why it matters: the tests check one brake or the other.

1. (planner) The front brake.
2. (planner) The rear brake.

Suggested: 1, because the issue names the front brake first.

</details>

q2: Is a test of the timing asked for?
<details>

Why it matters: a test of the timing needs the clock stubbed.

1. (planner) No timing test.
2. (planner) A timing test, with the clock stubbed.

Suggested: 2, because the issue speaks of a deadline.

</details>
";

/// A session of the planner of 54 that writes `file` as the issue's file and commits it alone, then runs `after`.
fn planner_writes_then(bench: &Bench, file: &str, after: &str) {
    bench.session(&format!(
        "cat > .watchbill/issues/54.yml <<'YML'\n{}YML\ngit add .watchbill/issues/54.yml && git commit -q -m 'The issue file of 54'\n{}",
        file, after
    ));
}

/// A session of the planner of 54 that writes `file` as the issue's file and commits it alone.
fn planner_writes(bench: &Bench, file: &str) {
    planner_writes_then(bench, file, "");
}

/// The calls that posted on issue 54.
fn posts_on_54(bench: &Bench) -> usize {
    bench.calls().iter().filter(|call| *call == "post issue 54").count()
}

#[test]
fn test_WB_BUILD_135_the_issue_s_text_is_brought_into_the_view_as_issue_text_printed_it() {
    if !sandbox("test_WB_BUILD_135") {
        return;
    }
    let bench = bench();
    fs::write(bench.at("text-54"), "Brakes: the front one.\nA second line.\n").unwrap();
    bench.session("cat .watchbill/issue/54.md > {log}/seen\n");

    let run = bench.start(&["start", "planner", "54"]);

    ok(&run);
    assert_eq!(bench.left("seen"), "Brakes: the front one.\nA second line.\n");
    // Run by `wb`, before the stamp is posted and the session started.
    let calls = bench.calls();
    let address = calls.iter().position(|call| call == "text 54").expect("issue_text was not run");
    let posted = calls.iter().position(|call| call == "post issue 54").unwrap();
    assert!(address < posted, "{:?}", calls);
    // It is not taken back: the checkout has no such folder.
    assert!(!bench.repo.path.join(".watchbill/issue").exists());
}

/// `wb plan add` with the words after `plan`, in the test process, with the organisation's file of the bench.
fn plan_add(bench: &Bench, args: &[&str]) {
    let words: Vec<std::ffi::OsString> = args.iter().map(std::ffi::OsString::from).collect();
    let mut input = std::io::Cursor::new(Vec::new());
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = watchbill::plan(&words, &bench.repo.path, &bench.organisation(), &mut input, &mut out, &mut err);
    assert_eq!(code, 0, "{}{}", text(&out), text(&err));
}

#[test]
fn test_WB_JOB_44_the_planner_of_a_plan_reads_every_source_of_it_and_no_issue_text() {
    if !sandbox("test_WB_JOB_44") {
        return;
    }
    let bench = bench();
    plan_add(&bench, &["add", "54", "-m", "Brakes: the front one."]);
    plan_add(&bench, &["add", "54", "-m", "And the rear one.", "--source", "https://example.invalid/54#2"]);
    bench.session("cat .watchbill/issue/54.md > {log}/seen\n");

    let run = bench.start(&["start", "planner", "54"]);

    ok(&run);
    let seen = bench.left("seen");
    let first = seen.find("Brakes: the front one.\n").expect(&seen);
    let second = seen.find("And the rear one.\n").expect(&seen);
    assert!(first < second, "{}", seen);
    assert!(seen.starts_with("--- source 1: "), "{}", seen);
    assert!(seen.contains(", from https://example.invalid/54#2 ---\n"), "{}", seen);
    assert!(!bench.calls().iter().any(|call| call == "text 54"), "{:?}", bench.calls());
}

/// Waits until the clock has passed into the next second, so that what is written after is later to the second.
fn next_second() {
    std::thread::sleep(std::time::Duration::from_millis(1100));
}

/// The acceptance of 54 taken out and committed, then `wb plan accept 54`: the person's word again.
fn plan_accept_again(bench: &Bench) {
    git(&bench.repo.path, &["rm", "-q", "--", ".watchbill/issues/54.accepted"]);
    git(&bench.repo.path, &["commit", "-q", "-m", "The acceptance taken out"]);
    let run = wb(&bench.repo.path, &["plan", "accept", "54"], "");
    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
}

#[test]
fn test_WB_JOB_42_a_planner_round_after_the_acceptance_voids_it_at_the_gate_and_accepting_again_lets_the_build_start() {
    if !sandbox("test_WB_JOB_42") {
        return;
    }
    let bench = bench();
    plan_add(&bench, &["add", "54", "-m", "Brakes: the front one."]);
    next_second();
    plan_accept_again(&bench);

    // A planner round whose session leaves the issue's file as it was; the tracker then holds its stamp.
    bench.session("true\n");
    ok(&bench.start(&["start", "planner", "54"]));
    let stamp = bench.posted();
    assert!(stamp.starts_with("[stamp] planner round 1\n") && stamp.contains("\ntime: "), "{}", stamp);
    fs::write(bench.at("comments-issue-54"), format!("{}\n", stamp.lines().next().unwrap())).unwrap();
    fs::write(bench.at("bodies-54"), format!("{}\0\n", stamp)).unwrap();
    assert_eq!(read(&bench.repo.path.join(".watchbill/issues/54.yml")), ISSUE_54);
    let starts = bench.calls().iter().filter(|call| *call == "start").count();

    let run = bench.start(&["start", "build", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("wb plan accept 54"), "{}", text_of(&run.stderr));
    assert_eq!(bench.calls().iter().filter(|call| *call == "start").count(), starts);

    let run = prompt_with(&bench.repo.path, &bench.organisation(), &["build", "54", "--round", "1"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert_eq!(text_of(&run.stdout), "");
    assert!(text_of(&run.stderr).contains("wb plan accept 54"), "{}", text_of(&run.stderr));

    // The person's word again.
    next_second();
    plan_accept_again(&bench);
    ok(&prompt_with(&bench.repo.path, &bench.organisation(), &["build", "54", "--round", "1"]));
    ok(&bench.start(&["start", "build", "54"]));
    assert_eq!(bench.calls().iter().filter(|call| *call == "start").count(), starts + 1);
}

#[test]
fn test_WB_JOB_39_remove_is_refused_while_this_machine_keeps_a_view_of_the_plan_with_work_not_taken_back() {
    let bench = bench();
    plan_add(&bench, &["add", "54", "-m", "Brakes: the front one."]);
    kept_with_a_commit(&bench);
    // The branch of 54 is on the default branch: only the kept view holds work.
    git(&bench.repo.path, &["update-ref", "refs/heads/main", "refs/heads/issue-54"]);
    git(&bench.repo.path, &["update-ref", "refs/remotes/origin/main", "refs/heads/issue-54"]);
    let before = bench.git_text(&["rev-parse", "refs/watchbill/54"]);

    let run = Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(["plan", "remove", "54"])
        .current_dir(&bench.repo.path)
        .env_clear()
        .envs(env())
        .env("XDG_STATE_HOME", &bench.state.path)
        .output()
        .unwrap();

    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(err.contains("keeps 1 view of it with work not taken back"), "{}", err);
    assert!(!err.contains("the branch"), "{}", err);
    assert_eq!(bench.git_text(&["rev-parse", "refs/watchbill/54"]), before);
}

#[test]
fn test_WB_PROMPT_73_the_planner_s_posted_stamp_holds_the_sha256_of_the_issue_s_text_before_the_parts() {
    if !sandbox("test_WB_PROMPT_73") {
        return;
    }
    let bench = bench();
    let issue = "Brakes: the front one.\n";
    fs::write(bench.at("text-54"), issue).unwrap();

    ok(&bench.start(&["start", "planner", "54"]));

    let posted = bench.posted();
    assert_eq!(line_of(&posted, "issue_text"), Some(sha256(issue)), "{}", posted);
    let names: Vec<&str> = posted.lines().map(|line| line.split(':').next().unwrap()).collect();
    let at = names.iter().position(|name| *name == "issue_text").unwrap();
    let level = names.iter().position(|name| *name == "level").unwrap();
    let part = names.iter().position(|name| *name == "part").unwrap();
    assert!(level < at && at < part, "{}", posted);
}

#[test]
fn test_WB_ROLE_104_the_planner_s_prompt_as_handed_out_names_the_issue_s_file_and_no_program_of_the_tracker() {
    let bench = bench();
    let run = prompt_with(&bench.repo.path, &bench.organisation(), &["planner", "54", "--runner", "stand-in", "--round", "1"]);
    ok(&run);
    let prompt = text_of(&run.stdout);
    assert!(prompt.contains(".watchbill/issue/54.md"), "{}", prompt);
    let words: Vec<&str> = prompt.split(|c: char| !c.is_ascii_alphanumeric()).collect();
    assert!(!prompt.contains("address.sh") && !words.contains(&"gh"), "{}", prompt);
}

#[test]
fn test_WB_BUILD_136_the_questions_and_answers_come_in_whole_and_no_stamp_or_other_comment() {
    if !sandbox("test_WB_BUILD_136") {
        return;
    }
    let bench = bench();
    let comments = [
        "[stamp] planner round 1\nrole: planner\nmodel: claude-opus-5-5",
        "[questions] 54 q1\nq1: Which brake?\n<details>\n\n1. (planner) The front brake.\n\nSuggested: 1, because it is first.\n\n</details>",
        "[answers] 54 q1>planner\nq1: 1",
        "[answers] 54.tester q1>tester\nq1: 2",
        "Thanks, that reads well.",
    ];
    let mut bodies = Vec::new();
    for comment in comments {
        bodies.extend_from_slice(comment.as_bytes());
        bodies.extend_from_slice(b"\0\n");
    }
    fs::write(bench.at("text-54"), "The front brake holds the car.\n").unwrap();
    fs::write(bench.at("bodies-54"), bodies).unwrap();
    bench.session("cat .watchbill/issue/54.md > {log}/text\ncat .watchbill/issue/54.questions > {log}/questions\n");

    ok(&bench.start(&["start", "planner", "54"]));

    let text = bench.left("text");
    assert!(text.contains("The front brake holds the car."), "{}", text);
    for never in ["[stamp]", "model: claude", "[questions]", "[answers]", "Thanks"] {
        assert!(!text.contains(never), "{}: {}", never, text);
    }
    let questions = bench.left("questions");
    assert!(questions.contains(comments[1]), "{}", questions);
    assert!(questions.contains(comments[2]), "{}", questions);
    for never in ["[stamp]", "54.tester", "Thanks"] {
        assert!(!questions.contains(never), "{}: {}", never, questions);
    }
}

#[test]
fn test_WB_BUILD_137_a_failing_issue_text_or_issue_comment_bodies_is_refused_as_a_limit_naming_it() {
    for (status, named) in [("text.status", "issue_text"), ("bodies.status", "issue_comment_bodies")] {
        let bench = on_99();
        fs::write(bench.at(status), "1\n").unwrap();
        let run = bench.start_by(&pass_through(&bench), &["start", "planner", "99"]);
        refused_before_posting(&bench, &run, &[named]);
    }
}

#[test]
fn test_WB_BUILD_137_without_issue_text_or_issue_comment_bodies_the_planner_is_refused_naming_it() {
    for key in ["issue_text", "issue_comment_bodies"] {
        let bench = on_99();
        without_line(&bench, key);
        let run = bench.start_by(&pass_through(&bench), &["start", "planner", "99"]);
        refused_before_posting(&bench, &run, &[key]);
    }
}

#[test]
fn test_WB_BUILD_138_more_than_1_mib_from_issue_text_or_issue_comment_bodies_is_refused_naming_it() {
    for (file, named) in [("text-99", "issue_text"), ("bodies-99", "issue_comment_bodies")] {
        let bench = on_99();
        fs::write(bench.at(file), "a".repeat(1024 * 1024 + 1)).unwrap();
        let run = bench.start_by(&pass_through(&bench), &["start", "planner", "99"]);
        refused_before_posting(&bench, &run, &[named, "1 MiB"]);
    }
}

#[test]
fn test_WB_BUILD_143_a_head_that_holds_watchbill_issue_is_refused_naming_it() {
    let bench = on_99();
    commit_file(&bench, ".watchbill/issue/99.md", "planted\n");
    let run = bench.start_by(&pass_through(&bench), &["start", "planner", "99"]);
    refused_before_posting(&bench, &run, &[".watchbill/issue"]);
}

#[test]
fn test_WB_BUILD_139_the_new_questions_of_the_file_are_posted_in_one_comment_by_issue_comment() {
    if !sandbox("test_WB_BUILD_139") {
        return;
    }
    let bench = bench();
    planner_writes(&bench, TWO_QUESTIONS);
    ok(&bench.start(&["start", "planner", "54"]));
    assert_eq!(posts_on_54(&bench), 2, "{:?}", bench.calls());
    assert_eq!(bench.posted(), TWO_POSTED);

    // A question that stands on the job's ref already is not posted again (#238): the tracker is not read for it.
    let bench = crate::bench();
    stamps_on_ref(&bench.repo.path, "54", &["[questions] 54 q1\n\nq1: Which brake does the issue mean?\n"]);
    planner_writes(&bench, TWO_QUESTIONS);
    ok(&bench.start(&["start", "planner", "54"]));
    assert_eq!(posts_on_54(&bench), 2, "{:?}", bench.calls());
    let q2 = TWO_POSTED.split_once("q2: ").unwrap().1;
    assert_eq!(bench.posted(), format!("[questions] 54 q2\nq2: {}", q2));
}

#[test]
fn test_WB_BUILD_140_the_comment_of_questions_is_said_before_it_is_posted() {
    if !sandbox("test_WB_BUILD_140") {
        return;
    }
    let bench = bench();
    planner_writes(&bench, TWO_QUESTIONS);
    let run = bench.start(&["start", "planner", "54"]);
    ok(&run);
    let err = text_of(&run.stderr);
    let said = format!("wb start planner: will post on issue 54, by `issue_comment` of watchbill.yml:\n{}", TWO_POSTED);
    assert!(err.contains(&said), "{}", err);
}

#[test]
fn test_WB_BUILD_141_a_file_with_no_question_or_none_new_posts_nothing_after_the_session() {
    if !sandbox("test_WB_BUILD_141") {
        return;
    }
    let bench = bench();
    let none = TWO_QUESTIONS.split_once("questions:\n").unwrap().0.to_string() + "questions: []\n";
    planner_writes(&bench, &none);
    ok(&bench.start(&["start", "planner", "54"]));
    assert_eq!(posts_on_54(&bench), 1, "{:?}", bench.calls());
    assert!(bench.posted().starts_with("[stamp] planner"), "{}", bench.posted());

    let bench = crate::bench();
    stamps_on_ref(&bench.repo.path, "54", &["[questions] 54 q1 q2\n\nq1: Which brake does the issue mean?\nq2: Is a test of the timing asked for?\n"]);
    planner_writes(&bench, TWO_QUESTIONS);
    ok(&bench.start(&["start", "planner", "54"]));
    assert_eq!(posts_on_54(&bench), 1, "{:?}", bench.calls());
    // The questions on the ref were found to be all of them: the tracker is not read for them (#238).
    let calls = bench.calls();
    let start = calls.iter().position(|call| call == "start").unwrap();
    assert!(!calls[start..].iter().any(|call| call == "comments issue 54"), "{:?}", calls);
}

#[test]
fn test_WB_BUILD_142_questions_that_cannot_be_posted_end_with_1_as_a_limit_saying_so() {
    if !sandbox("test_WB_BUILD_142") {
        return;
    }
    let bench = bench();
    // The stamp is posted; the comment of questions is not.
    planner_writes_then(&bench, TWO_QUESTIONS, "echo 1 > {log}/post.status\n");
    let run = bench.start(&["start", "planner", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(err.contains("limit:") && err.contains("not posted"), "{}", err);
    // The issue's file came back all the same.
    assert_eq!(subjects(&bench, 1), vec!["The issue file of 54"]);
}

/// The messages of the commits of the job's ref of 54, oldest first: what follows the header of each commit.
fn ref_messages(bench: &Bench) -> Vec<String> {
    let commits = bench.git_text(&["rev-list", "--reverse", "refs/watchbill/54"]);
    commits
        .lines()
        .map(|commit| bench.git_text(&["cat-file", "commit", commit]).split_once("\n\n").map(|(_, message)| message.to_string()).unwrap_or_default())
        .collect()
}

/// The bare repository the pushes of `origin` go to (`push_beside`).
fn bare_of(bench: &Bench) -> String {
    bench.repo.path.parent().unwrap().join("origin.git").to_str().unwrap().to_string()
}

/// The first line of a message and what follows its header: the form of the copy that is posted.
fn copy_of(message: &str) -> String {
    let (head, body) = message.split_once("\n\n").unwrap_or((message, ""));
    format!("{}\n{}", head.lines().next().unwrap_or(""), body)
}

#[test]
fn test_WB_ANSWER_21_the_planner_s_questions_are_a_commit_on_the_job_s_ref_after_its_stamp_signed_and_pushed() {
    if !sandbox("test_WB_ANSWER_21") {
        return;
    }
    let bench = bench();
    planner_writes(&bench, TWO_QUESTIONS);

    ok(&bench.start(&["start", "planner", "54"]));

    let messages = ref_messages(&bench);
    assert_eq!(messages.len(), 2, "{:?}", messages);
    assert!(messages[0].starts_with("[stamp] planner round 1\n"), "{}", messages[0]);
    assert!(messages[1].starts_with("[questions] 54 q1 q2\nissue: 54\njob: none\nbase: "), "{}", messages[1]);
    assert_eq!(copy_of(&messages[1]), TWO_POSTED);
    let tip = bench.git_text(&["rev-parse", "refs/watchbill/54"]).trim().to_string();
    assert!(bench.git_text(&["cat-file", "commit", &tip]).contains("\ngpgsig "));
    assert_eq!(bench.git_text(&["rev-parse", &format!("{}^{{tree}}", tip)]), bench.git_text(&["rev-parse", &format!("{}^^{{tree}}", tip)]));
    // Pushed to origin with the stamp, and the copy posted.
    assert_eq!(bench.git_text(&["--git-dir", &bare_of(&bench), "rev-parse", "refs/watchbill/54"]).trim(), tip);
    assert_eq!(bench.posted(), TWO_POSTED);
    // `wb status` shows them open from the ref, and runs no `issue_comments` for the issue.
    let before = called(&bench, "comments");
    let run = wb(&bench.repo.path, &["status", "54"], "");
    assert!(text_of(&run.stdout).contains("Questions open:\n  54  54.q1, 54.q2\n"), "{}{}", text_of(&run.stdout), text_of(&run.stderr));
    assert_eq!(called(&bench, "comments"), before, "{:?}", bench.calls());
}

#[test]
fn test_WB_ANSWER_21_the_questions_of_another_role_are_a_commit_named_for_the_role() {
    if !sandbox("test_WB_ANSWER_21") {
        return;
    }
    let bench = bench();
    writing_session(&bench, "questions", TESTER_QUESTIONS, 0);

    ok(&bench.start(&["start", "tester", "54"]));

    let messages = ref_messages(&bench);
    assert_eq!(messages.len(), 2, "{:?}", messages);
    assert!(messages[0].starts_with("[stamp] tester round 1\n"), "{}", messages[0]);
    assert!(messages[1].starts_with("[questions] 54.tester q1 q2\nissue: 54\n"), "{}", messages[1]);
    assert_eq!(copy_of(&messages[1]), TESTER_POSTED);
}

#[test]
fn test_WB_ANSWER_21_a_session_with_no_new_question_writes_no_commit() {
    if !sandbox("test_WB_ANSWER_21") {
        return;
    }
    let bench = bench();
    bench.session("true\n");

    ok(&bench.start(&["start", "tester", "54"]));

    assert_eq!(ref_messages(&bench).len(), 1);
}

#[test]
fn test_WB_ANSWER_21_the_commit_is_written_before_the_copy_and_a_copy_that_fails_leaves_it_on_the_ref() {
    if !sandbox("test_WB_ANSWER_21") {
        return;
    }
    let bench = bench();
    planner_writes_then(&bench, TWO_QUESTIONS, "echo 1 > {log}/post.status\n");

    let run = bench.start(&["start", "planner", "54"]);

    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(err.contains("not posted") && err.contains("refs/watchbill/54"), "{}", err);
    let messages = ref_messages(&bench);
    assert_eq!(messages.len(), 2, "{:?}", messages);
    assert!(messages[1].starts_with("[questions] 54 q1 q2\n"), "{}", messages[1]);
}

#[test]
fn test_WB_ANSWER_21_with_no_issue_comment_the_commit_is_written_and_no_copy_is_posted() {
    if !sandbox("test_WB_ANSWER_21") {
        return;
    }
    let bench = bench();
    without_line(&bench, "issue_comment");
    planner_writes(&bench, TWO_QUESTIONS);

    let run = bench.start(&["start", "planner", "54"]);

    ok(&run);
    assert!(!bench.any_posted(), "{:?}", bench.calls());
    assert!(text_of(&run.stderr).contains("no copy of the questions is posted"), "{}", text_of(&run.stderr));
    assert!(ref_messages(&bench)[1].starts_with("[questions] 54 q1 q2\n"));
}

#[test]
fn test_WB_ANSWER_21_a_ref_that_is_not_as_wb_writes_it_at_the_end_of_the_session_holds_back_the_questions_and_says_them() {
    if !sandbox("test_WB_ANSWER_21") {
        return;
    }
    let bench = bench();
    // The copy of the stamp is posted by a stand-in that, the first time it runs, puts a commit that no key signed on the
    // job's ref, which breaks it (#327), as it is posted before the session.
    let l = bench.log.path.to_str().unwrap();
    script(
        &bench.at("post.sh"),
        &format!(
            "cat > {l}/posted-$1-$2\necho \"post $1 $2\" >> {l}/calls\nif [ ! -f {l}/planted ]; then\n  tree=$(git rev-parse 'refs/watchbill/54^{{tree}}')\n  commit=$(git -c user.name=x -c user.email=x@example.org commit-tree --no-gpg-sign -p refs/watchbill/54 -m planted \"$tree\")\n  git update-ref refs/watchbill/54 \"$commit\"\n  echo \"$commit\" > {l}/planted\nfi\nexit 0\n"
        ),
    );
    planner_writes(&bench, TWO_QUESTIONS);

    let run = bench.start(&["start", "planner", "54"]);

    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(err.contains("is not as wb writes it") && err.contains("[questions] 54 q1 q2") && err.contains("Which brake does the issue mean?"), "{}", err);
    assert!(!ref_messages(&bench).iter().any(|message| message.starts_with("[questions] ")), "{:?}", ref_messages(&bench));
}

#[test]
fn test_WB_ANSWER_21_questions_too_large_for_a_commit_are_not_written_and_are_said() {
    if !sandbox("test_WB_ANSWER_21") {
        return;
    }
    let bench = bench();
    let long = "x".repeat(60 * 1024);
    let file = TWO_QUESTIONS.replace("the tests check one brake or the other.", &long);
    planner_writes(&bench, &file);

    let run = bench.start(&["start", "planner", "54"]);

    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(err.contains("48 KiB"), "{}", err);
    // The questions are said whole, so that they are not lost with the session's files.
    assert!(err.contains("[questions] 54 q1 q2") && err.contains("Which brake does the issue mean?") && err.contains("Is a test of the timing asked for?"), "{}", &err[..err.len().min(2000)]);
    assert!(err.contains(&"x".repeat(1000)), "the text of the question is not said");
    assert_eq!(ref_messages(&bench).len(), 1);
}

#[test]
fn test_WB_ANSWER_21_where_the_tracker_holds_open_questions_the_first_commit_says_they_are_not_read_any_more() {
    if !sandbox("test_WB_ANSWER_21") {
        return;
    }
    let bench = bench();
    fs::write(bench.at("comments-issue-54"), "[questions] 54.build q1\n").unwrap();
    planner_writes(&bench, TWO_QUESTIONS);

    let run = bench.start(&["start", "planner", "54"]);

    ok(&run);
    let err = text_of(&run.stderr);
    assert!(err.contains("the tracker holds open questions") && err.contains("54.build.q1") && err.contains("not read"), "{}", err);
}

#[test]
fn test_WB_ANSWER_21_the_first_commit_names_the_questions_it_asks_again_that_the_tracker_holds_and_says_its_answers_are_not_read() {
    if !sandbox("test_WB_ANSWER_21") {
        return;
    }
    let bench = bench();
    // q1 was asked and answered at the tracker, q2 asked and still open; the planner's file holds both.
    fs::write(bench.at("comments-issue-54"), "[questions] 54 q1 q2\n[answers] 54 q1>planner\n").unwrap();
    planner_writes(&bench, TWO_QUESTIONS);

    let run = bench.start(&["start", "planner", "54"]);

    ok(&run);
    let err = text_of(&run.stderr);
    assert!(err.contains("54.q1 (answered at the tracker)") && err.contains("54.q2 (open at the tracker)"), "{}", err);
    assert!(err.contains("asked again") && err.contains("the answers at the tracker are not read any more"), "{}", err);
    // Both are on the ref, asked again.
    assert!(ref_messages(&bench)[1].starts_with("[questions] 54 q1 q2\n"), "{:?}", ref_messages(&bench));
}

/// Makes the bare repository that `origin` pushes to refuse a commit whose message starts with `[questions]`.
fn refuse_questions_at_origin(bench: &Bench) {
    let hook = Path::new(&bare_of(bench)).join("hooks/pre-receive");
    script(&hook, "while read old new ref; do\n  if git log -1 --format=%B \"$new\" | head -n 1 | grep -q '^\\[questions\\]'; then echo 'no questions here' >&2; exit 1; fi\ndone\nexit 0\n");
}

#[test]
fn test_WB_ANSWER_21_a_questions_commit_that_cannot_be_pushed_says_how_to_get_the_questions_asked() {
    if !sandbox("test_WB_ANSWER_21") {
        return;
    }
    let bench = bench();
    refuse_questions_at_origin(&bench);
    planner_writes(&bench, TWO_QUESTIONS);

    let run = bench.start(&["start", "planner", "54"]);

    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    // The questions are said, and what to do.
    assert!(err.contains("[questions] 54 q1 q2") && err.contains("Which brake does the issue mean?"), "{}", err);
    let after = err.split("push it by hand: `git push origin ").nth(1).unwrap_or_else(|| panic!("no command to push by hand: {}", err));
    let (command, _) = after.split_once('`').unwrap();
    let (commit, full) = command.split_once(':').unwrap();
    assert_eq!(bench.git_text(&["cat-file", "-t", commit]).trim(), "commit", "{}", err);
    assert_eq!(full, "refs/watchbill/54", "{}", err);
    // What it says is true in both setups: where the fetch line of `wb init` is there the push moves the local ref too,
    // and where it is not the person fetches the ref by its name.
    assert!(err.contains("once it is pushed, the questions are on the ref and asked"), "{}", err);
    assert!(err.contains("`+refs/watchbill/*:refs/watchbill/*`") && err.contains("`git fetch origin refs/watchbill/54:refs/watchbill/54`"), "{}", err);
    assert!(!err.contains("The local ref is not moved") && !err.contains("behind origin") && !err.contains("refused as not a fast-forward until"), "{}", err);
    // The message reads right: one full stop, the questions not written said once, and no other role named.
    assert!(!err.contains("..") && !err.contains("not written To"), "{}", err);
    assert_eq!(err.matches("The questions are not written").count(), 1, "{}", err);
    assert!(!err.contains("those of another role"), "{}", err);
    // Nothing was written on the ref.
    assert_eq!(ref_messages(&bench).len(), 1);
    // The bench has the fetch line of `wb init`; the person lets origin take it, and runs the printed push.
    assert!(bench.git_text(&["config", "--get-all", "remote.origin.fetch"]).contains("+refs/watchbill/*:refs/watchbill/*"));
    fs::remove_file(Path::new(&bare_of(&bench)).join("hooks/pre-receive")).unwrap();
    bench.git_text(&["push", "origin", command]);
    let run = wb(&bench.repo.path, &["status", "54"], "");
    assert!(text_of(&run.stdout).contains("Questions open:\n  54  54.q1, 54.q2\n"), "{}{}", text_of(&run.stdout), text_of(&run.stderr));
}

#[test]
fn test_WB_ANSWER_26_the_questions_brought_into_the_planner_are_those_of_the_ref_and_not_of_the_tracker() {
    if !sandbox("test_WB_ANSWER_26") {
        return;
    }
    let bench = bench();
    stamps_on_ref(&bench.repo.path, "54", &["[questions] 54 q1\n\nq1: Which brake?\n"]);
    fs::write(bench.at("bodies-54"), "[answers] 54 q1>planner\nq1: 1\0\n[questions] 54 q9\nq9: forged\0\n").unwrap();
    bench.session("cat .watchbill/issue/54.questions > {log}/questions\n");

    ok(&bench.start(&["start", "planner", "54"]));

    let questions = bench.left("questions");
    assert_eq!(questions, "[questions] 54 q1\nq1: Which brake?\n", "{}", questions);
    assert!(!bench.calls().iter().any(|call| call == "bodies 54"), "{:?}", bench.calls());
}

#[test]
fn test_WB_ANSWER_26_the_questions_brought_into_a_role_are_those_of_the_ref_and_not_of_the_tracker() {
    if !sandbox("test_WB_ANSWER_26") {
        return;
    }
    let bench = bench();
    stamps_on_ref(&bench.repo.path, "54", &["[questions] 54.tester q1\n\nq1: Which brake?\n"]);
    fs::write(bench.at("bodies-54"), "[answers] 54.tester q1>tester\nq1: 1\0\n[questions] 54.tester q9\nq9: forged\0\n").unwrap();
    reading_session(&bench);

    ok(&bench.start(&["start", "tester", "54"]));

    assert_eq!(bench.left("in-questions"), "[questions] 54.tester q1\nq1: Which brake?\n");
    assert!(!bench.calls().iter().any(|call| call == "bodies 54"), "{:?}", bench.calls());
}

#[test]
fn test_WB_ANSWER_26_an_answer_commit_is_brought_in_and_a_question_taken_back_by_a_marker_is_not() {
    if !sandbox("test_WB_ANSWER_26") {
        return;
    }
    let bench = bench();
    stamps_on_ref(
        &bench.repo.path,
        "54",
        &["[stamp] tester round 1", "[questions] 54.tester q1\n\nq1: Gone?\n", "[revert] tester round 1", "[questions] 54.tester q2\n\nq2: Kept?\n", "[answers] 54.tester q2>tester\n\nq2: 1\n"],
    );
    reading_session(&bench);

    ok(&bench.start(&["start", "tester", "54"]));

    assert_eq!(bench.left("in-questions"), "[questions] 54.tester q2\nq2: Kept?\n\n[answers] 54.tester q2>tester\nq2: 1\n");
}

#[test]
fn test_WB_ANSWER_46_wb_answer_suggested_takes_the_suggestions_of_the_questions_the_planner_s_start_posted() {
    if !sandbox("test_WB_ANSWER_46") {
        return;
    }
    let bench = bench();
    planner_writes(&bench, TWO_QUESTIONS);
    ok(&bench.start(&["start", "planner", "54"]));
    let mut bodies = bench.posted().into_bytes();
    bodies.push(0);
    fs::write(bench.at("bodies-54"), bodies).unwrap();

    let run = wb(&bench.repo.path, &["answer", "54", "--suggested"], "");

    assert_eq!(code(&run), Some(0), "{}", text_of(&run.stderr));
    let answers = bench.posted();
    assert!(answers.starts_with("[answers] 54 q1>planner q2>planner\n"), "{}", answers);
    assert!(answers.contains("q1: 1\n") && answers.contains("q2: 2"), "{}", answers);
}

#[test]
fn test_WB_VIEW_96_a_path_the_planner_left_uncommitted_beside_its_file_is_not_said_and_does_not_keep_the_view() {
    if !sandbox("test_WB_VIEW_96") {
        return;
    }
    let bench = on_99();
    bench.session(
        "echo lock > Cargo.lock && echo x >> src/brake.c && echo yes > {log}/wrote\n\
         printf 'issue: 99\\n' > .watchbill/issues/99.yml && git add .watchbill/issues/99.yml && git commit -q -m 'The issue file of 99'\n",
    );
    let run = bench.start(&["start", "planner", "99"]);
    ok(&run);
    assert_eq!(bench.left("wrote"), "yes\n");
    assert!(!text_of(&run.stderr).contains("left uncommitted"), "{}", text_of(&run.stderr));
    assert!(bench.below("views").is_empty(), "{:?}", bench.below("views"));

    // Its issue's file, left uncommitted, is said, and keeps the view.
    let bench = on_99();
    bench.session("echo lock > Cargo.lock\nprintf 'issue: 99\\n' > .watchbill/issues/99.yml\n");
    let run = bench.start(&["start", "planner", "99"]);
    let err = text_of(&run.stderr);
    assert!(err.contains("left uncommitted") && err.contains(".watchbill/issues/99.yml") && !err.contains("Cargo.lock"), "{}", err);
    assert_eq!(bench.below("views").len(), 1, "{:?}", bench.below("views"));
}

#[test]
fn test_WB_VIEW_77_a_commit_of_cargo_lock_by_the_planner_is_refused_at_the_take_back() {
    if !sandbox("test_WB_VIEW_77") {
        return;
    }
    let bench = on_99();
    let before = bench.head();
    bench.session("echo lock > Cargo.lock && git add Cargo.lock && git commit -q -m 'The lock'\n");
    let run = bench.start(&["start", "planner", "99"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("Cargo.lock"), "{}", text_of(&run.stderr));
    assert_eq!(bench.head(), before);
}

// ---- The review of #297: only the comments at the end are cut, and nothing raw leaves the sandbox ----

/// The planner of 99 started with the stand-in for bubblewrap, its session copying the issue's text it was given.
fn text_brought(bench: &Bench, text: &[u8], bodies: &str) -> (Output, Vec<u8>) {
    fs::write(bench.at("text-99"), text).unwrap();
    fs::write(bench.at("bodies-99"), bodies).unwrap();
    bench.session("cat \"$VIEW/.watchbill/issue/99.md\" > {log}/text\n");
    let run = bench.start_by(&pass_through(bench), &["start", "planner", "99"]);
    let brought = fs::read(bench.at("text")).unwrap_or_default();
    (run, brought)
}

#[test]
fn test_WB_BUILD_144_the_issue_s_text_arrives_whole_byte_for_byte_whatever_the_comments() {
    // The cases of the second look at #297: comments that end the text, are pieces of it, or are all of it, and an
    // issue whose last lines look like a tracker's.
    let cases: [(&str, &[u8], &str); 7] = [
        ("A", b"Release the brake. Never", "Never\0"),
        ("B", b"Do not delete the log.", "log\0"),
        ("C", b"Do not delete the log.", "Do not\0delete the log.\0"),
        ("D", b"Do not delete the log.", "Do not delete the log.\0"),
        ("E", b"Do it.\nlevel:\tASIL-D\nnote:\tnever on Fridays\n", "ok\0"),
        ("F", b"Build it:\n```\nmake\n```\n", "ok\0"),
        ("bytes", b"Line one.\r\n\xff\xfe not UTF-8 --\n", "--\0Line one.\0"),
    ];
    for (case, text, bodies) in cases {
        let bench = on_99();
        let (run, brought) = text_brought(&bench, text, bodies);
        assert!(bench.started(), "{}: {}", case, text_of(&run.stderr));
        assert_eq!(brought, text.to_vec(), "{}", case);
        assert_eq!(line_of(&bench.left("posted-issue-99"), "issue_text"), Some(sha256_bytes(text)), "{}", case);
    }
}

/// The SHA-256 of bytes, by `sha256sum`.
fn sha256_bytes(bytes: &[u8]) -> String {
    let mut child = Command::new("sha256sum").stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).spawn().unwrap();
    std::io::Write::write_all(&mut child.stdin.take().unwrap(), bytes).unwrap();
    let out = child.wait_with_output().unwrap();
    text_of(&out.stdout).split_whitespace().next().unwrap().to_string()
}

#[test]
fn test_WB_BUILD_147_without_issue_text_the_planner_is_refused_naming_it_and_wb_upgrade() {
    let bench = on_99();
    without_line(&bench, "issue_text");
    let run = bench.start_by(&pass_through(&bench), &["start", "planner", "99"]);
    refused_before_posting(&bench, &run, &["`issue_text`", "`wb upgrade` adds the line `issue_text`"]);
}

/// The issue's file of 54 with one question whose line holds `odd`, as YAML writes it in double quotes.
fn odd_question(odd: &str) -> String {
    TWO_QUESTIONS.split_once("questions:\n").unwrap().0.to_string() + &format!("questions:\n  - \"54.q1: Which {} brake?\"\n", odd)
}

#[test]
fn test_WB_BUILD_145_a_question_with_an_escape_or_a_bidi_control_is_not_posted_and_is_said_escaped() {
    if !sandbox("test_WB_BUILD_145") {
        return;
    }
    for (odd, raw, escaped) in [("\\e]0;pwned\\a", '\u{1b}', "\\u{1b}"), ("\\u202E", '\u{202e}', "\\u{202e}")] {
        let bench = bench();
        planner_writes(&bench, &odd_question(odd));
        let run = bench.start(&["start", "planner", "54"]);
        assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
        let err = text_of(&run.stderr);
        assert!(err.contains("limit:") && err.contains("not posted") && err.contains(escaped), "{}", err);
        assert!(!err.contains(raw), "{:?}", err);
        assert_eq!(posts_on_54(&bench), 1, "{:?}", bench.calls());
        assert!(!bench.posted().contains(raw), "{:?}", bench.posted());
    }
}

#[test]
fn test_WB_BUILD_146_the_questions_are_read_from_the_commit_taken_back_not_the_working_tree() {
    let bench = bench();
    put(&bench.repo.path, ".watchbill/issues/54.yml", TWO_QUESTIONS);
    git(&bench.repo.path, &["commit", "-q", "-a", "-m", "The issue file of 54"]);
    let commit = bench.head();
    // The checkout's working tree holds the older file, with no question.
    git(&bench.repo.path, &["reset", "-q", "--hard", "HEAD~1"]);
    let mut err = Vec::new();
    // The questions are written as a commit on the job's ref (#238), signed as a start signs.
    let signing = watchbill::stamps::ready(&bench.root()).expect("the bench signs as the starter");

    let posted = watchbill::brought::post_questions("wb start planner", &bench.root(), "54", &commit, &signing, &mut err);

    assert_eq!(posted, Ok(()), "{}", text_of(&err));
    assert_eq!(posts_on_54(&bench), 1, "{:?}", bench.calls());
    assert_eq!(bench.posted(), TWO_POSTED);
}

// ---- #296: a take-back goes onto the stamp's branch, never onto the branch checked out ----

/// The bench with `main` checked out, where no branch of issue 7 is.
fn on_main() -> Bench {
    let bench = bench();
    git(&bench.repo.path, &["checkout", "-q", "main"]);
    bench
}

/// A planner of 7 that commits its issue's file in the view and notes the view's branch: in bubblewrap the
/// session is in the view, and under the stand-in `pass_through` the view is `$VIEW`. `before` runs first, in the
/// checkout under the stand-in.
fn planner_7(bench: &Bench, before: &str) {
    bench.session(&format!(
        "{}cd \"${{VIEW:-.}}\" && git symbolic-ref HEAD > {{log}}/branch && printf 'issue: 7\\n' > .watchbill/issues/7.yml && git add .watchbill/issues/7.yml && git -c commit.gpgsign=false commit -q -m 'The issue file of 7'\n",
        before
    ));
}

/// What `git status` says of the checkout, ignored files too: the index and the working tree.
fn tree_of(bench: &Bench) -> String {
    bench.git_text(&["status", "--porcelain=v1", "--ignored", "--untracked-files=all"])
}

#[test]
fn test_WB_VIEW_110_with_main_checked_out_the_planner_s_commit_makes_the_issue_s_branch_and_main_is_not_moved() {
    let bench = on_main();
    // `wb status` reads the issue's title by `issue_address` as `HEAD` has it: the bench's stand-in prints one (#295).
    let main = bench.head();
    let tree = tree_of(&bench);
    planner_7(&bench, "");

    let run = bench.start_by(&pass_through(&bench), &["start", "planner", "7"]);

    ok(&run);
    assert_eq!(bench.git_text(&["log", "-1", "--format=%s", "issue-7"]).trim(), "The issue file of 7");
    assert_eq!(bench.git_text(&["rev-parse", "issue-7~1"]).trim(), main);
    assert_eq!(bench.git_text(&["show", "issue-7:.watchbill/issues/7.yml"]), "issue: 7\n");
    assert_eq!(bench.head(), main);
    assert_eq!(bench.git_text(&["symbolic-ref", "HEAD"]).trim(), "refs/heads/main");
    assert_eq!(tree_of(&bench), tree);
    assert!(text_of(&run.stderr).contains("git push origin issue-7"), "{}", text_of(&run.stderr));
    let status = wb_state(&bench, &["status", "7"]);
    let out = text_of(&status.stdout);
    assert!(out.contains("Planned") && !out.contains("Not planned"), "{}{}", out, text_of(&status.stderr));
}

#[test]
fn test_WB_VIEW_110_in_bubblewrap_the_planner_with_main_checked_out_makes_the_issue_s_branch() {
    if !sandbox("test_WB_VIEW_110") {
        return;
    }
    let bench = on_main();
    let main = bench.head();
    planner_7(&bench, "");

    let run = bench.start(&["start", "planner", "7"]);

    ok(&run);
    assert_eq!(bench.git_text(&["log", "-1", "--format=%s", "issue-7"]).trim(), "The issue file of 7");
    assert_eq!(bench.git_text(&["rev-parse", "issue-7~1"]).trim(), main);
    assert_eq!(bench.head(), main);
    assert_eq!(bench.left("branch"), "refs/heads/issue-7\n");
}

#[test]
fn test_WB_VIEW_111_with_the_issue_s_branch_there_and_main_checked_out_the_commit_lands_on_it_and_the_checkout_is_untouched() {
    // The branch is there, at the checkout's HEAD; the checkout is on main, which the guard stops and
    // `--ignore-branch` passes (#298).
    let bench = on_main();
    git(&bench.repo.path, &["branch", "issue-7"]);
    let main = bench.head();
    put(&bench.repo.path, "build/out.o", "ignored\n");
    fs::write(bench.repo.path.join(".git/info/exclude"), "build/out.o\n").unwrap();
    let tree = tree_of(&bench);
    planner_7(&bench, "");

    let run = bench.start_by(&pass_through(&bench), &["start", "planner", "7", "--ignore-branch"]);

    ok(&run);
    assert_eq!(bench.git_text(&["log", "-1", "--format=%s", "issue-7"]), "The issue file of 7\n");
    assert_eq!(bench.git_text(&["rev-parse", "issue-7~1"]).trim(), main);
    assert_eq!(bench.head(), main);
    assert_eq!(bench.git_text(&["symbolic-ref", "HEAD"]).trim(), "refs/heads/main");
    assert_eq!(tree_of(&bench), tree);
    assert!(!bench.repo.path.join(".watchbill/issues/7.yml").exists());
    let err = text_of(&run.stderr);
    assert!(err.lines().any(|line| line.contains("issue-7") && line.contains("not checked out")), "{}", err);
}

#[test]
fn test_WB_VIEW_112_with_the_issue_s_branch_checked_out_the_commit_comes_by_a_fast_forward_of_the_checkout() {
    let bench = on_main();
    git(&bench.repo.path, &["checkout", "-q", "-b", "issue-7"]);
    let before = bench.head();
    planner_7(&bench, "");

    let run = bench.start_by(&pass_through(&bench), &["start", "planner", "7"]);

    ok(&run);
    assert_eq!(subjects(&bench, 1), vec!["The issue file of 7"]);
    assert_eq!(bench.git_text(&["rev-parse", "HEAD~1"]).trim(), before);
    assert_eq!(bench.git_text(&["symbolic-ref", "HEAD"]).trim(), "refs/heads/issue-7");
    assert_eq!(read(&bench.repo.path.join(".watchbill/issues/7.yml")), "issue: 7\n");
    assert_eq!(bench.git_text(&["rev-parse", "main"]).trim(), before);
    assert!(!text_of(&run.stderr).contains("not checked out"), "{}", text_of(&run.stderr));
}

#[test]
fn test_WB_VIEW_113_a_branch_made_or_moved_between_the_start_and_the_take_back_is_refused_and_nothing_is_moved() {
    // Made while the session runs, where it was not there when the view was made.
    let bench = on_main();
    let main = bench.head();
    planner_7(&bench, "git branch issue-7 HEAD~1 && ");
    let run = bench.start_by(&pass_through(&bench), &["start", "planner", "7"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(labelled_line(&err, "wb start planner", "limit").is_some_and(|line| line.contains("issue-7")), "{}", err);
    assert_eq!(bench.git_text(&["rev-parse", "issue-7"]), bench.git_text(&["rev-parse", "main~1"]));
    assert_eq!(bench.head(), main);
    assert_eq!(bench.below("views").len(), 1, "the view is kept");

    // Moved while the session runs.
    let bench = on_main();
    let main = bench.head();
    git(&bench.repo.path, &["branch", "issue-7"]);
    planner_7(&bench, "git branch -f issue-7 HEAD~1 && ");
    let run = bench.start_by(&pass_through(&bench), &["start", "planner", "7", "--ignore-branch"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(labelled_line(&err, "wb start planner", "limit").is_some_and(|line| line.contains("issue-7") && line.contains("moved")), "{}", err);
    assert_eq!(bench.git_text(&["rev-parse", "issue-7"]), bench.git_text(&["rev-parse", "main~1"]));
    assert_eq!(bench.head(), main);
    assert_eq!(bench.below("views").len(), 1, "the view is kept");

    // Deleted while the session runs.
    let bench = on_main();
    let main = bench.head();
    git(&bench.repo.path, &["branch", "issue-7"]);
    planner_7(&bench, "git branch -D issue-7 > /dev/null && ");
    let run = bench.start_by(&pass_through(&bench), &["start", "planner", "7", "--ignore-branch"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(labelled_line(&err, "wb start planner", "limit").is_some_and(|line| line.contains("issue-7") && line.contains("deleted")), "{}", err);
    assert!(bench.git_text(&["branch", "--list", "issue-7"]).is_empty());
    assert_eq!(bench.head(), main);
    assert_eq!(bench.below("views").len(), 1, "the view is kept");
}

#[test]
fn test_WB_VIEW_115_the_view_s_branch_has_the_name_of_the_stamp_s_branch() {
    let bench = on_main();
    planner_7(&bench, "");
    ok(&bench.start_by(&pass_through(&bench), &["start", "planner", "7"]));
    assert_eq!(bench.left("branch"), "refs/heads/issue-7\n");
    assert_eq!(line_of(&bench.left("posted-issue-7"), "branch").as_deref(), Some("issue-7"));
}

#[test]
fn test_WB_VIEW_116_a_branch_checked_out_in_another_worktree_is_refused_and_not_moved() {
    let bench = on_main();
    let other = folder_named("worktree");
    let tree = other.path.join("tree");
    git(&bench.repo.path, &["worktree", "add", "-q", "-b", "issue-7", tree.to_str().unwrap()]);
    let before = bench.git_text(&["rev-parse", "issue-7"]);
    planner_7(&bench, "");

    let run = bench.start_by(&pass_through(&bench), &["start", "planner", "7", "--ignore-branch"]);

    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    let shown = fs::canonicalize(&tree).unwrap();
    assert!(labelled_line(&err, "wb start planner", "limit").is_some_and(|line| line.contains(shown.to_str().unwrap())), "{}", err);
    assert_eq!(bench.git_text(&["rev-parse", "issue-7"]), before);
    assert!(!tree.join(".watchbill/issues/7.yml").exists());
    git(&bench.repo.path, &["worktree", "remove", "--force", tree.to_str().unwrap()]);
}

#[test]
fn test_WB_BUILD_150_a_checkout_on_another_branch_is_refused_as_a_guard_naming_it_the_stamp_s_branch_and_ignore_branch() {
    let bench = bench();
    git(&bench.repo.path, &["checkout", "-q", "-b", "other"]);
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "build", "54"]));
    let line = labelled_line(&err, "wb start build", "guard").unwrap_or_else(|| panic!("no guard: {}", err));
    assert!(line.contains("other") && line.contains("issue-54") && line.contains("--ignore-branch"), "{}", err);
    assert!(labelled_line(&err, "wb start build", "limit").is_none(), "{}", err);

    // On the issue's branch, or on the default branch, there is no such guard.
    for (branch, words) in [("issue-54", ["start", "build", "54"]), ("main", ["start", "planner", "7"])] {
        let bench = self::bench();
        git(&bench.repo.path, &["checkout", "-q", branch]);
        let run = start_unsandboxed(&bench, &words);
        let err = text_of(&run.stderr);
        assert!(!err.contains("--ignore-branch") && !err.contains(": guard: "), "{}: {}", branch, err);
    }
}

#[test]
fn test_WB_BUILD_151_ignore_branch_passes_the_guard_alone_and_the_commit_still_goes_onto_the_stamp_s_branch() {
    let bench = bench();
    git(&bench.repo.path, &["checkout", "-q", "-b", "other"]);
    // Past the guard, the start goes on to the view, where no bubblewrap is: a limit.
    let run = start_unsandboxed(&bench, &["start", "build", "54", "--ignore-branch"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(labelled_line(&err, "wb start build", "guard").is_none(), "{}", err);
    assert!(labelled_line(&err, "wb start build", "limit").is_some(), "{}", err);

    let bench = self::bench();
    git(&bench.repo.path, &["checkout", "-q", "-b", "other"]);
    let other = bench.head();
    bench.session("cd \"$VIEW\" && echo 'int brake(void) { return 3; }' > src/brake.c && git add src/brake.c && git -c commit.gpgsign=false commit -q -m 'The brake'\n");
    let run = bench.start_by(&pass_through(&bench), &["start", "build", "54", "--ignore-branch"]);
    ok(&run);
    assert_eq!(bench.git_text(&["log", "-1", "--format=%s", "issue-54"]).trim(), "The brake");
    assert_eq!(bench.git_text(&["rev-parse", "issue-54~1"]).trim(), other);
    assert_eq!(bench.head(), other);
    assert_eq!(bench.git_text(&["symbolic-ref", "HEAD"]).trim(), "refs/heads/other");
}

// ---- The review of #298: a session starts only on its branch's tree ----

#[test]
fn test_WB_BUILD_152_where_the_issue_s_branch_is_ahead_of_main_checked_out_the_start_is_refused_as_a_limit() {
    // issue-54 holds the accepted file of 54, a commit that main has not.
    let bench = bench();
    git(&bench.repo.path, &["checkout", "-q", "main"]);
    for words in [&["start", "build", "54"][..], &["start", "build", "54", "--ignore-branch"][..]] {
        let err = guarded(&bench, &start_unsandboxed(&bench, words));
        let line = labelled_line(&err, "wb start build", "limit").unwrap_or_else(|| panic!("no limit: {}", err));
        assert!(line.contains("main") && line.contains("issue-54") && line.contains("Check out issue-54"), "{}", err);
        assert!(labelled_line(&err, "wb start build", "guard").is_none(), "{}", err);
    }
}

#[test]
fn test_WB_BUILD_152_where_main_checked_out_has_moved_past_the_issue_s_branch_the_start_is_refused_as_a_limit() {
    let bench = bench();
    git(&bench.repo.path, &["checkout", "-q", "main"]);
    git(&bench.repo.path, &["merge", "-q", "--ff-only", "issue-54"]);
    put(&bench.repo.path, "docs/note.txt", "a note\n");
    git(&bench.repo.path, &["add", "--", "docs/note.txt"]);
    git(&bench.repo.path, &["commit", "-q", "-m", "Merged on"]);
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "build", "54"]));
    let line = labelled_line(&err, "wb start build", "limit").unwrap_or_else(|| panic!("no limit: {}", err));
    assert!(line.contains("main") && line.contains("issue-54") && line.contains("Check out issue-54"), "{}", err);
}

#[test]
fn test_WB_BUILD_150_on_main_with_the_issue_s_branch_there_at_the_same_commit_the_guard_stops_the_start() {
    let bench = on_main();
    git(&bench.repo.path, &["branch", "issue-7"]);
    let err = guarded(&bench, &start_unsandboxed(&bench, &["start", "planner", "7"]));
    let line = labelled_line(&err, "wb start planner", "guard").unwrap_or_else(|| panic!("no guard: {}", err));
    assert!(line.contains("main") && line.contains("issue-7") && line.contains("--ignore-branch"), "{}", err);
}

#[test]
fn test_WB_VIEW_117_a_take_back_onto_a_branch_behind_the_commit_the_view_was_made_from_says_it_is_behind() {
    let bench = bench();
    // A commit after the issue's file, so that the branch set one back still holds it.
    put(&bench.repo.path, "docs/note.txt", "a note\n");
    git(&bench.repo.path, &["add", "--", "docs/note.txt"]);
    git(&bench.repo.path, &["commit", "-q", "-m", "A note"]);
    let (folder, _) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 13; }\n", "Tests");
    let before = bench.git_text(&["rev-parse", "HEAD~1"]);
    git(&bench.repo.path, &["reset", "-q", "--hard", "HEAD~1"]);
    let run = take_back(&bench, &[]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    let line = labelled_line(&err, "wb start tester", "limit").unwrap_or_else(|| panic!("no limit: {}", err));
    assert!(line.contains("issue-54") && line.contains("behind") && !line.contains("moved on"), "{}", err);
    assert_eq!(bench.git_text(&["rev-parse", "issue-54"]), before);
    assert!(folder.exists());
}

// ---- #277: a commit taken back is not named again as not taken when its view is kept ----

fn git_text_in(folder: &Path, args: &[&str]) -> String {
    let out = Command::new("git").args(args).current_dir(folder).output().unwrap();
    text_of(&out.stdout)
}

/// A kept view of the build for 54 with one commit, `A taken commit`, and a path left uncommitted; taken back once.
/// The commit comes onto the branch, and the view is kept for the path.
fn taken_once(bench: &Bench) -> (PathBuf, PathBuf) {
    let (folder, session) = keep(bench, &Keep { session: "2026-11-22T100000Z-build-54", role: "build", round: 1, commits: &[], uncommitted: &[], outcome: RAN });
    commit_in(&folder, "src/new.c", "int taken;\n", "A taken commit");
    put(&folder, "left.txt", "uncommitted\n");
    let run = bench.start(&["start", "build", "54", "--take-back"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert_eq!(subjects(bench, 1), vec!["A taken commit"]);
    assert!(folder.exists());
    (folder, session)
}

#[test]
fn test_WB_VIEW_119_a_take_back_writes_for_each_commit_taken_a_line_took_with_the_view_s_commit_and_the_branch_s() {
    let bench = bench();
    let (folder, session) = taken_once(&bench);
    let back = read(&session.join("back"));
    let from = git_text_in(&folder, &["rev-parse", "HEAD"]);
    let onto = bench.git_text(&["rev-parse", "HEAD"]);
    let took: Vec<&str> = back.lines().filter(|line| line.starts_with("took: ")).collect();
    assert_eq!(took, vec![format!("took: {} {}", from.trim(), onto.trim())], "{}", back);
    // The line `taken:` is as it was.
    assert!(back.lines().any(|line| line.starts_with("taken: ") && line.contains("A taken commit")), "{}", back);
}

#[test]
fn test_WB_VIEW_120_a_commit_taken_is_not_named_again_as_not_taken_by_status_the_guard_or_a_take_back() {
    let bench = bench();
    let (_folder, _) = taken_once(&bench);
    let out = text_of(&wb_state(&bench, &["status", "54"]).stdout);
    assert!(!out.contains("not taken") && !out.contains("A taken commit"), "{}", out);
    let err = text_of(&bench.start(&["start", "build", "54"]).stderr);
    assert!(err.contains("left.txt") && !err.contains("A taken commit"), "{}", err);
    let before = bench.head();
    let run = bench.start(&["start", "build", "54", "--take-back"]);
    let err = text_of(&run.stderr);
    assert_eq!(code(&run), Some(1), "{}", err);
    assert!(!err.contains("A taken commit") && !err.contains("moved on"), "{}", err);
    assert_eq!(bench.head(), before);
}

#[test]
fn test_WB_VIEW_121_a_kept_view_whose_commits_were_all_taken_is_named_as_work_left_uncommitted() {
    let bench = bench();
    let (folder, _) = taken_once(&bench);
    let out = text_of(&wb_state(&bench, &["status", "54"]).stdout);
    assert!(out.lines().any(|line| line == format!("kept: build round 1, 1 path left uncommitted, {}", folder.display())), "{}", out);
    let err = text_of(&bench.start(&["start", "build", "54"]).stderr);
    assert!(err.contains("work left uncommitted: left.txt") && !err.contains("\n    uncommitted:"), "{}", err);
}

#[test]
fn test_WB_VIEW_122_a_take_back_of_a_view_with_only_uncommitted_work_left_takes_nothing_and_says_why() {
    let bench = bench();
    let (folder, _) = taken_once(&bench);
    let before = bench.head();
    let run = bench.start(&["start", "build", "54", "--take-back"]);
    let err = text_of(&run.stderr);
    assert_eq!(code(&run), Some(1), "{}", err);
    assert!(err.contains("every commit of the view was taken before") && err.contains("left.txt"), "{}", err);
    assert_eq!(bench.head(), before);
    assert!(folder.exists());
}

#[test]
fn test_WB_VIEW_123_a_commit_made_in_the_view_after_a_take_back_is_named_and_taken_by_the_next_one_and_the_one_taken_is_not() {
    let bench = bench();
    let (folder, _) = taken_once(&bench);
    commit_in(&folder, "tests/test_brake.c", "int second;\n", "A second commit");
    let out = text_of(&wb_state(&bench, &["status", "54"]).stdout);
    assert!(out.contains("kept: build round 1, 1 commit not taken, 1 path uncommitted"), "{}", out);
    let err = text_of(&bench.start(&["start", "build", "54"]).stderr);
    assert!(err.contains("A second commit") && !err.contains("A taken commit") && !err.contains("work left uncommitted"), "{}", err);
    let run = bench.start(&["start", "build", "54", "--take-back"]);
    let err = text_of(&run.stderr);
    assert_eq!(code(&run), Some(1), "{}", err);
    assert_eq!(subjects(&bench, 2), vec!["A second commit", "A taken commit"], "{}", err);
    assert!(!err.contains("moved on") && !err.contains("taken before"), "{}", err);
}

#[test]
fn test_WB_VIEW_120_after_a_take_back_that_took_and_a_later_one_that_failed_the_commit_taken_is_not_named() {
    let bench = bench();
    let (folder, _) = taken_once(&bench);
    // A second commit that changes the path the first one did: the next take-back is refused, as the branch has moved on.
    commit_in(&folder, "src/new.c", "int second;\n", "A second commit");
    let run = bench.start(&["start", "build", "54", "--take-back"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["A taken commit"]);
    let out = text_of(&wb_state(&bench, &["status", "54"]).stdout);
    assert!(out.contains("kept: build round 1, 1 commit not taken,"), "{}", out);
    let err = text_of(&bench.start(&["start", "build", "54"]).stderr);
    assert!(err.contains("A second commit") && !err.contains("A taken commit"), "{}", err);
}

#[test]
fn test_WB_VIEW_124_a_view_with_no_line_took_lists_every_commit_as_before() {
    let bench = bench();
    let (folder, session) = keep(&bench, &Keep { session: "2026-11-22T100000Z-build-54", role: "build", round: 1, commits: &["One"], uncommitted: &[], outcome: RAN });
    assert!(!session.join("back").exists());
    commit_in(&folder, "src/new.c", "int two;\n", "Two");
    let out = text_of(&wb_state(&bench, &["status", "54"]).stdout);
    assert!(out.contains("kept: build round 1, 2 commits not taken,"), "{}", out);
}

#[test]
fn test_WB_VIEW_121_a_kept_view_that_cannot_be_read_is_not_said_to_hold_only_uncommitted_work() {
    let bench = bench();
    let (folder, _) = taken_once(&bench);
    fifo_for_head(&folder);
    let run = wb_state_bounded(&bench, &["status", "54"], 30).expect("wb status did not end in 30 seconds");
    let out = text_of(&run.stdout);
    let line = out.lines().find(|line| line.starts_with("kept: build round 1, not read: ")).unwrap_or_else(|| panic!("{}", out));
    assert!(!line.contains("left uncommitted"), "{}", line);
    let run = bench.start(&["start", "build", "54"]);
    let err = text_of(&run.stderr);
    assert!(err.contains("not read: ") && err.contains("uncommitted: left.txt") && !err.contains("work left uncommitted"), "{}", err);
}

// ---- #151: wb pushes the branch and opens the change after a session; the session holds no login ----

/// The bench with `origin` a bare repository of its own, whose hook notes each ref it is given in `pushes` (and what
/// `wb` had said by then, from `err`, in `said-at-push`), and the patterns of the change: `change_of_branch` prints
/// what the test leaves in `open`; `draft_change` notes its call, keeps its standard input in `description`, and
/// leaves 175 in `open`, as a change opened.
fn pushing() -> (Bench, Folder) {
    let bench = bench();
    let origin = folder_named("origin");
    let l = bench.log.path.to_str().unwrap().to_string();
    git(&origin.path, &["init", "-q", "--bare"]);
    script(
        &origin.path.join("hooks").join("post-receive"),
        &format!("cat {l}/err > {l}/said-at-push 2>/dev/null\nwhile read old new ref; do case $ref in refs/watchbill/*) continue;; esac; echo \"$ref $new\" >> {l}/pushes; done\n"),
    );
    git(&bench.repo.path, &["remote", "set-url", "origin", origin.path.to_str().unwrap()]);
    git(&bench.repo.path, &["config", "remote.origin.pushurl", origin.path.to_str().unwrap()]);
    script(&bench.at("changes.sh"), &format!("echo \"changes $1\" >> {l}/calls\ncat {l}/open 2>/dev/null\nexit 0\n"));
    script(
        &bench.at("draft.sh"),
        &format!("echo \"draft $1\" >> {l}/calls\ncat > {l}/description\necho 175 > {l}/open\nexit $(cat {l}/draft.status 2>/dev/null || echo 0)\n"),
    );
    with_line(&bench, &format!("draft_change: '{}/draft.sh {{branch}}'", l));
    (bench, origin)
}

/// A build's session: a new file committed, and `description` written to the description's file, where given.
fn build_session(bench: &Bench, description: Option<&str>) {
    let write = match description {
        Some(text) => format!(" && printf '%s' '{}' > /run/watchbill/description", text),
        None => String::new(),
    };
    bench.session(&format!("echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'{}\n", write));
}

/// The refs of the bare `origin`, each as `<ref> <commit>`.
/// The refs of `origin`, but the jobs' refs, where `wb start` pushes the stamp before the session (#236; tests/stamped.rs).
fn refs_of(origin: &Folder) -> String {
    let run = Command::new("git").arg("-C").arg(&origin.path).args(["for-each-ref", "--format=%(refname) %(objectname)"]).env_clear().envs(env()).output().unwrap();
    text_of(&run.stdout).lines().filter(|line| !line.starts_with("refs/watchbill/")).map(|line| format!("{}\n", line)).collect()
}

/// A review stamp on issue 54: since #236 a commit on the job's ref, and the comment a copy that `wb` never reads.
fn review_stamp(bench: &Bench) {
    fs::write(bench.at("comments-issue-54"), "[stamp] review round 1\n").unwrap();
    stamps_on_ref(&bench.repo.path, "54", &["[stamp] review round 1"]);
}

/// The file `push` of the session's folder.
fn push_lines(bench: &Bench) -> String {
    read(&bench.lines().join("push"))
}

/// How many times a stand-in was called, by the first word of its call.
fn called(bench: &Bench, word: &str) -> usize {
    bench.calls().iter().filter(|call| call.split(' ').next() == Some(word)).count()
}

#[test]
fn test_WB_PUSH_1_after_a_build_the_issue_s_branch_is_pushed_once_to_origin() {
    if !sandbox("test_WB_PUSH_1") {
        return;
    }
    let (bench, _origin) = pushing();
    build_session(&bench, Some("What changed.\n"));

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let tip = bench.git_text(&["rev-parse", "issue-54"]).trim().to_string();
    // The notes ref goes after the branch, where the branch was pushed (#237, WB-VIEW-41).
    let notes = bench.git_text(&["rev-parse", "refs/notes/watchbill"]).trim().to_string();
    assert_eq!(bench.left("pushes"), format!("refs/heads/issue-54 {}\nrefs/notes/watchbill {}\n", tip, notes), "{}", text_of(&run.stderr));
}

#[test]
fn test_WB_PUSH_2_only_the_stamp_s_branch_is_pushed_at_the_last_commit_taken_back() {
    if !sandbox("test_WB_PUSH_2") {
        return;
    }
    let (bench, origin) = pushing();
    bench.session("echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file' && echo 'int more;' >> src/new.c && git commit -q -a -m 'More'\n");

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let tip = bench.git_text(&["rev-parse", "issue-54"]).trim().to_string();
    assert_eq!(subjects(&bench, 1), vec!["More"]);
    // The branch, and the notes ref of #237 after it, as the branch was pushed (WB-VIEW-41).
    let notes = bench.git_text(&["rev-parse", "refs/notes/watchbill"]).trim().to_string();
    assert_eq!(refs_of(&origin), format!("refs/heads/issue-54 {}\nrefs/notes/watchbill {}\n", tip, notes));
}

#[test]
fn test_WB_PUSH_2_a_branch_at_origin_that_has_moved_on_is_never_forced() {
    if !sandbox("test_WB_PUSH_2") {
        return;
    }
    let (bench, origin) = pushing();
    // `origin` holds the issue's branch at a commit the checkout's branch does not hold.
    let tree = bench.git_text(&["rev-parse", "main^{tree}"]).trim().to_string();
    let main = bench.git_text(&["rev-parse", "main"]).trim().to_string();
    let other = bench.git_text(&["commit-tree", &tree, "-p", &main, "-m", "Elsewhere"]).trim().to_string();
    git(&bench.repo.path, &["push", "-q", origin.path.to_str().unwrap(), &format!("{}:refs/heads/issue-54", other)]);
    fs::remove_file(bench.at("pushes")).unwrap();
    build_session(&bench, Some("What changed.\n"));

    let run = bench.start(&["start", "build", "54"]);

    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert_eq!(refs_of(&origin), format!("refs/heads/issue-54 {}\n", other));
    assert_eq!(bench.left("pushes"), "");
}

#[test]
fn test_WB_PUSH_3_what_is_pushed_is_said_before_the_push() {
    if !sandbox("test_WB_PUSH_3") {
        return;
    }
    let (bench, _origin) = pushing();
    build_session(&bench, Some("What changed.\n"));
    let mut said = fs::File::create(bench.at("err")).unwrap();

    let run = bench.start_saying(Path::new(watchbill::view::BWRAP), &["start", "build", "54"], &mut said);

    let err = bench.left("err");
    assert_eq!(code(&run), Some(0), "{}", err);
    let tip = bench.git_text(&["rev-parse", "issue-54"]).trim().to_string();
    let before = bench.left("said-at-push");
    let line = before.lines().find(|line| line.starts_with("wb start build: will push ")).unwrap_or_else(|| panic!("{}", before));
    assert!(line.contains(&tip) && line.contains("issue-54"), "{}", line);
    assert!(before.contains(&format!("git push origin {}:refs/heads/issue-54", tip)), "{}", before);
}

#[test]
fn test_WB_PUSH_4_a_commit_refused_at_the_taking_back_pushes_nothing_and_says_so() {
    if !sandbox("test_WB_PUSH_4") {
        return;
    }
    let (bench, origin) = pushing();
    bench.session("echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file' && echo 'int brake(int);' > src/brake.h && git commit -q -a -m 'Change a read'\n");

    let run = bench.start(&["start", "build", "54"]);

    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["A new file"]);
    assert_eq!(refs_of(&origin), "");
    assert_eq!(called(&bench, "draft"), 0);
    let err = text_of(&run.stderr);
    assert!(err.lines().any(|line| line.starts_with("wb start build: nothing is pushed") && line.contains("refused")), "{}", err);
    assert!(err.contains("git push origin issue-54"), "{}", err);
}

#[test]
fn test_WB_PUSH_4_work_left_uncommitted_pushes_nothing_and_says_so() {
    if !sandbox("test_WB_PUSH_4") {
        return;
    }
    let (bench, origin) = pushing();
    bench.session("echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file' && echo 'int left;' > tests/test_brake.c\n");

    let run = bench.start(&["start", "build", "54"]);

    let err = text_of(&run.stderr);
    assert_eq!(subjects(&bench, 1), vec!["A new file"], "{}", err);
    assert_eq!(refs_of(&origin), "");
    assert_eq!(called(&bench, "draft"), 0);
    assert!(err.lines().any(|line| line.starts_with("wb start build: nothing is pushed") && line.contains("uncommitted")), "{}", err);
    assert!(err.contains("git push origin issue-54"), "{}", err);
}

#[test]
fn test_WB_PUSH_5_a_push_git_refuses_ends_with_1_says_why_and_opens_no_change() {
    if !sandbox("test_WB_PUSH_5") {
        return;
    }
    let (bench, origin) = pushing();
    // The remote refuses every branch it is given; the stamp's push before the session (#236) goes through.
    script(
        &origin.path.join("hooks").join("pre-receive"),
        "while read old new ref; do case $ref in refs/heads/*) echo 'no pushes today' >&2; exit 1;; esac; done\nexit 0\n",
    );
    build_session(&bench, Some("What changed.\n"));

    let run = bench.start(&["start", "build", "54"]);

    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    let err = text_of(&run.stderr);
    assert!(err.contains("no pushes today"), "{}", err);
    let line = labelled_line(&err, "wb start build", "limit").unwrap_or_else(|| panic!("no limit: {}", err));
    assert!(line.contains("push") && line.contains("no change is opened"), "{}", err);
    assert_eq!(refs_of(&origin), "");
    assert_eq!(called(&bench, "draft"), 0);
    assert!(push_lines(&bench).starts_with("not pushed: "), "{}", push_lines(&bench));
}

#[test]
fn test_WB_PUSH_6_after_a_build_with_no_change_open_one_is_opened_as_a_draft_with_the_description_on_standard_input() {
    if !sandbox("test_WB_PUSH_6") {
        return;
    }
    let (bench, _origin) = pushing();
    build_session(&bench, Some("What changed, and why.\n"));

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let calls = bench.calls();
    let changes = calls.iter().position(|call| call == "changes issue-54").unwrap_or_else(|| panic!("{:?}", calls));
    let draft = calls.iter().position(|call| call == "draft issue-54").unwrap_or_else(|| panic!("{:?}", calls));
    assert!(changes < draft, "{:?}", calls);
    assert_eq!(called(&bench, "draft"), 1);
    assert_eq!(bench.left("description"), "What changed, and why.\n");
    let err = text_of(&run.stderr);
    let said = err.find("wb start build: will open the change of issue-54 as a draft").unwrap_or_else(|| panic!("{}", err));
    assert!(err[said..].contains(&format!("{}/draft.sh issue-54", bench.log.path.display())), "{}", err);
    assert!(err[said..].contains("What changed, and why."), "{}", err);
}

#[test]
fn test_WB_PUSH_6_a_change_open_for_the_branch_gets_the_push_only() {
    if !sandbox("test_WB_PUSH_6") {
        return;
    }
    let (bench, origin) = pushing();
    fs::write(bench.at("open"), "174\n").unwrap();
    build_session(&bench, Some("What changed.\n"));

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    assert_ne!(refs_of(&origin), "");
    assert_eq!(called(&bench, "draft"), 0);
    assert!(push_lines(&bench).contains("change: 174\n"), "{}", push_lines(&bench));

    // More than one open: none is opened either.
    let (bench, _origin) = pushing();
    fs::write(bench.at("open"), "174\n176\n").unwrap();
    build_session(&bench, Some("What changed.\n"));
    let run = bench.start(&["start", "build", "54"]);
    ok(&run);
    assert_eq!(called(&bench, "draft"), 0);
    assert!(push_lines(&bench).contains("change: not opened: "), "{}", push_lines(&bench));
}

#[test]
fn test_WB_PUSH_7_after_the_tester_and_the_fix_only_the_push() {
    if !sandbox("test_WB_PUSH_7") {
        return;
    }
    let (bench, origin) = pushing();
    bench.session("echo 'int main(void) { return 1; }' > tests/test_brake.c && git commit -q -a -m 'A test'\n");
    let run = bench.start(&["start", "tester", "54"]);
    ok(&run);
    assert_ne!(refs_of(&origin), "");
    assert_eq!(called(&bench, "changes"), 0, "{:?}", bench.calls());
    assert_eq!(called(&bench, "draft"), 0);

    let (bench, origin) = pushing();
    review_stamp(&bench);
    fs::write(bench.at("open"), "174\n").unwrap();
    bench.session("echo 'int brake(void) { return 2; }' > src/brake.c && git commit -q -a -m 'Fix finding 1 of round 1'\n");
    let run = bench.start(&["start", "fix", "54"]);
    ok(&run);
    assert_ne!(refs_of(&origin), "");
    // The one call of `change_of_branch` is the start's own, which works out the change.
    assert_eq!(called(&bench, "changes"), 1, "{:?}", bench.calls());
    assert_eq!(called(&bench, "draft"), 0);
}

#[test]
fn test_WB_PUSH_8_the_description_s_file_is_named_in_the_prompt_lies_outside_the_view_and_is_removed() {
    if !sandbox("test_WB_PUSH_8") {
        return;
    }
    let (bench, _origin) = pushing();
    bench.session(
        "printf '%s' \"$3\" > {log}/prompt\necho 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\nprintf 'Described.\\n' > /run/watchbill/description && git status --porcelain --untracked-files=all > {log}/status\n",
    );

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    assert!(bench.left("prompt").contains("/run/watchbill/description"), "{}", bench.left("prompt"));
    // Not in the view's tree: git sees nothing to commit after it is written.
    assert_eq!(bench.left("status"), "");
    assert_eq!(bench.left("description"), "Described.\n");
    // The folder `wb` made for it is gone.
    assert!(bench.below("out").is_empty(), "{:?}", bench.below("out"));
}

#[test]
fn test_WB_PUSH_9_with_no_description_the_change_names_the_issue_and_says_the_session_wrote_none() {
    if !sandbox("test_WB_PUSH_9") {
        return;
    }
    for written in [None, Some("  \n\n")] {
        let (bench, _origin) = pushing();
        build_session(&bench, written);

        let run = bench.start(&["start", "build", "54"]);

        ok(&run);
        let description = bench.left("description");
        assert!(description.contains("54") && description.contains("wrote no description"), "{:?}: {}", written, description);
    }
}

#[test]
fn test_WB_PUSH_10_without_draft_change_nothing_is_pushed_and_the_commands_are_said() {
    if !sandbox("test_WB_PUSH_10") {
        return;
    }
    let bench = bench();
    let origin = folder_named("origin");
    git(&origin.path, &["init", "-q", "--bare"]);
    git(&bench.repo.path, &["remote", "set-url", "origin", origin.path.to_str().unwrap()]);
    git(&bench.repo.path, &["config", "remote.origin.pushurl", origin.path.to_str().unwrap()]);
    build_session(&bench, Some("What changed.\n"));

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    assert_eq!(refs_of(&origin), "");
    let err = text_of(&run.stderr);
    assert!(err.lines().any(|line| line.starts_with("wb start build: nothing is pushed") && line.contains("`draft_change`")), "{}", err);
    assert!(err.contains("git push origin issue-54"), "{}", err);
    assert!(push_lines(&bench).starts_with("not pushed: "), "{}", push_lines(&bench));
}

#[test]
fn test_WB_PUSH_10_no_push_says_the_commands_and_runs_neither() {
    if !sandbox("test_WB_PUSH_10") {
        return;
    }
    let (bench, origin) = pushing();
    build_session(&bench, Some("What changed.\n"));

    let run = bench.start(&["start", "build", "54", "--no-push"]);

    ok(&run);
    assert_eq!(refs_of(&origin), "");
    assert_eq!(called(&bench, "draft"), 0);
    assert_eq!(called(&bench, "changes"), 0);
    let err = text_of(&run.stderr);
    assert!(err.lines().any(|line| line.starts_with("wb start build: nothing is pushed") && line.contains("--no-push")), "{}", err);
    let push = err.find("git push origin issue-54").unwrap_or_else(|| panic!("{}", err));
    let draft = err.find(&format!("{}/draft.sh issue-54", bench.log.path.display())).unwrap_or_else(|| panic!("{}", err));
    assert!(push < draft, "{}", err);
}

#[test]
fn test_WB_PUSH_11_the_session_s_folder_holds_pushed_and_change() {
    if !sandbox("test_WB_PUSH_11") {
        return;
    }
    let (bench, _origin) = pushing();
    build_session(&bench, Some("What changed.\n"));

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let tip = bench.git_text(&["rev-parse", "issue-54"]).trim().to_string();
    assert_eq!(push_lines(&bench), format!("pushed: {}\nchange: 175\n", tip));

    // Nothing pushed: why.
    let (bench, _origin) = pushing();
    build_session(&bench, Some("What changed.\n"));
    let run = bench.start(&["start", "build", "54", "--no-push"]);
    ok(&run);
    let lines = push_lines(&bench);
    assert!(lines.starts_with("not pushed: ") && lines.contains("--no-push") && !lines.contains("change:"), "{}", lines);
}

#[test]
fn test_WB_PUSH_12_a_runner_s_bind_that_holds_a_login_of_the_code_host_is_refused() {
    let bench = bench();
    let good = format!("binds: [{{writable: '{}'}}, '{}', '~/.runner']", bench.log.path.display(), bench.ro.path.display());
    for bad in ["~/.config/gh", "~/.config/glab-cli", "~/.ssh", "~/.git-credentials", "~/.config/git/credentials", "~/.netrc", "~/.config"] {
        let text = read(&bench.organisation()).replace(&good, &format!("binds: ['{}']", bad));
        fs::write(bench.organisation(), &text).unwrap();
        let run = start_unsandboxed(&bench, &["start", "build", "54"]);
        assert_eq!(code(&run), Some(1), "{}: {}", bad, text_of(&run.stderr));
        let err = text_of(&run.stderr);
        assert!(err.contains("login") && err.contains("code host"), "{}: {}", bad, err);
        assert!(!bench.any_posted() && !bench.started(), "{}: {:?}", bad, bench.calls());
        let back = read(&bench.organisation()).replace(&format!("binds: ['{}']", bad), &good);
        fs::write(bench.organisation(), back).unwrap();
    }
}

/// The login of the code host that a runner's bind `bind` meets, where the environment holds `vars` alone and the
/// person's home is `/home/ada`.
fn login_met(bind: &str, vars: &[(&str, &str)]) -> Option<PathBuf> {
    let vars: Vec<(String, String)> = vars.iter().map(|(name, value)| (name.to_string(), value.to_string())).collect();
    let lookup = move |name: &str| vars.iter().find(|(one, _)| one == name).map(|(_, value)| std::ffi::OsString::from(value));
    watchbill::push::login_of(Path::new(bind), Some(Path::new("/home/ada")), &lookup)
}

#[test]
fn test_WB_PUSH_12_a_bind_of_a_login_below_xdg_config_home_is_refused() {
    let vars = [("XDG_CONFIG_HOME", "/srv/ada/conf")];
    for bad in ["/srv/ada/conf/gh", "/srv/ada/conf/gh/hosts.yml", "/srv/ada/conf/glab-cli", "/srv/ada/conf/git/credentials", "/srv/ada/conf", "/srv/ada"] {
        assert!(login_met(bad, &vars).is_some(), "{} is let through", bad);
    }
    for good in ["/srv/ada/conf/nvim", "/srv/ada/conf/git/config", "/srv/other"] {
        assert_eq!(login_met(good, &vars), None, "{}", good);
    }
    // A folder that is not absolute names nothing.
    assert_eq!(login_met("/srv/ada/conf/gh", &[("XDG_CONFIG_HOME", "conf")]), None);
    // Below the home, as before.
    assert!(login_met("/home/ada/.config/gh", &[]).is_some());
}

#[test]
fn test_WB_PUSH_12_a_bind_of_the_runtime_folder_or_of_a_keyring_in_it_is_refused() {
    let vars = [("XDG_RUNTIME_DIR", "/run/user/1000")];
    for bad in [
        "/run/user/1000",
        "/run/user",
        "/run",
        "/run/user/1000/keyring",
        "/run/user/1000/keyring/ssh",
        "/run/user/1000/gnupg",
        "/run/user/1000/gcr",
        "/run/user/1000/gcr/ssh",
        "/run/user/1000/bus",
    ] {
        assert!(login_met(bad, &vars).is_some(), "{} is let through", bad);
    }
    for good in ["/run/user/1000/podman", "/run/user/1001", "/run/media"] {
        assert_eq!(login_met(good, &vars), None, "{}", good);
    }
}

#[test]
fn test_WB_PUSH_12_a_bind_of_the_folder_of_the_ssh_agent_s_socket_is_refused() {
    let vars = [("SSH_AUTH_SOCK", "/var/agents/ada/agent.4242")];
    for bad in ["/var/agents/ada", "/var/agents/ada/agent.4242", "/var/agents"] {
        assert!(login_met(bad, &vars).is_some(), "{} is let through", bad);
    }
    for good in ["/var/agents/bob", "/var/lib"] {
        assert_eq!(login_met(good, &vars), None, "{}", good);
    }
}

#[test]
fn test_WB_PUSH_15_a_session_that_did_not_end_with_0_pushes_nothing() {
    if !sandbox("test_WB_PUSH_15") {
        return;
    }
    let (bench, origin) = pushing();
    bench.session("echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\nexit 3\n");

    let run = bench.start(&["start", "build", "54"]);

    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["A new file"]);
    assert_eq!(refs_of(&origin), "");
    let err = text_of(&run.stderr);
    assert!(err.lines().any(|line| line.starts_with("wb start build: nothing is pushed") && line.contains("did not end with 0")), "{}", err);
}

#[test]
fn test_WB_PUSH_16_after_a_review_nothing_is_pushed_and_the_commands_are_said() {
    if !sandbox("test_WB_PUSH_16") {
        return;
    }
    let (bench, origin) = pushing();
    fs::write(bench.at("open"), "174\n").unwrap();
    bench.session("echo 'int brake(void) { return 3; }' > src/brake.c && git commit -q -a -m 'A review that commits'\n");

    let run = bench.start(&["start", "review", "54"]);

    let err = text_of(&run.stderr);
    assert_eq!(subjects(&bench, 1), vec!["A review that commits"], "{}", err);
    assert_eq!(refs_of(&origin), "");
    assert_eq!(called(&bench, "draft"), 0);
    assert!(err.lines().any(|line| line.starts_with("wb start review: nothing is pushed: ") && line.contains("review")), "{}", err);
    assert!(err.contains("git push origin issue-54"), "{}", err);
}

/// The session of a build that commits a new file and then runs `write`, which leaves something at the description's
/// file; then `wb start build 54`, where the description is refused: the branch is pushed, no change is opened, and
/// `wb` ends with 1, with a limit that names the description.
fn assert_description_refused(write: &str) {
    let (bench, origin) = pushing();
    bench.session(&format!("echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file' && {}\n", write));

    let run = bench.start(&["start", "build", "54"]);

    assert_eq!(code(&run), Some(1), "{}: {}", write, text_of(&run.stderr));
    // The branch is pushed; the change is not opened.
    assert_ne!(refs_of(&origin), "", "{}", write);
    assert_eq!(called(&bench, "draft"), 0, "{}", write);
    let err = text_of(&run.stderr);
    assert!(labelled_line(&err, "wb start build", "limit").is_some_and(|line| line.contains("description")), "{}: {}", write, err);
    assert!(!err.contains('\u{1b}'), "{:?}", err);
    assert!(push_lines(&bench).contains("change: not opened: "), "{}: {}", write, push_lines(&bench));
}

#[test]
fn test_WB_PUSH_17_a_description_that_cannot_be_taken_opens_no_change() {
    if !sandbox("test_WB_PUSH_17") {
        return;
    }
    assert_description_refused("ln -s /etc/hostname /run/watchbill/description");
    assert_description_refused("printf 'red \\033[31m here\\n' > /run/watchbill/description");
}

#[test]
fn test_WB_PUSH_17_a_description_over_64_kib_opens_no_change_and_one_of_64_kib_does() {
    if !sandbox("test_WB_PUSH_17") {
        return;
    }
    // One byte over the cap of 64 KiB, well below the 128 KiB that Linux holds one word of a command to.
    assert_description_refused("head -c 65537 /dev/zero | tr '\\0' a > /run/watchbill/description");
    let (bench, _origin) = pushing();
    bench.session("echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file' && head -c 65536 /dev/zero | tr '\\0' a > /run/watchbill/description\n");
    let run = bench.start(&["start", "build", "54"]);
    assert_eq!(code(&run), Some(0), "{}", text_of(&run.stderr));
    assert_eq!(called(&bench, "draft"), 1);
    assert_eq!(read(&bench.at("description")).len(), 65536);
}

#[test]
fn test_WB_PUSH_17_a_description_of_a_dash_alone_opens_no_change() {
    if !sandbox("test_WB_PUSH_17") {
        return;
    }
    // `-` would have `glab` open an editor, as the person.
    assert_description_refused("printf ' -\\n' > /run/watchbill/description");
    assert_description_refused("printf -- '-' > /run/watchbill/description");
}

#[test]
fn test_WB_PUSH_17_a_description_that_is_not_utf_8_opens_no_change() {
    if !sandbox("test_WB_PUSH_17") {
        return;
    }
    assert_description_refused("printf 'Issue 54 \\377\\376\\n' > /run/watchbill/description");
}

#[test]
fn test_WB_PUSH_17_a_fifo_as_the_description_opens_no_change_and_is_not_waited_for() {
    if !sandbox("test_WB_PUSH_17") {
        return;
    }
    // The binary cannot run a session with the bench's runner (the organisation's file is fixed when `wb` is built), so
    // `start_on` runs in a thread of its own, held to 60 seconds: a read that waits on the FIFO, which has no writer,
    // fails the test and not the suite.
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        assert_description_refused("mkfifo /run/watchbill/description");
        let _ = send.send(());
    });
    match receive.recv_timeout(std::time::Duration::from_secs(60)) {
        Ok(()) => {}
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => panic!("the case of the FIFO failed"),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => panic!("wb start waited on a FIFO as the description for 60 seconds"),
    }
}

#[test]
fn test_WB_PUSH_18_description_given_to_start_is_used_wrongly() {
    let bench = bench();
    let run = start_unsandboxed(&bench, &["start", "build", "54", "--description", "/tmp/elsewhere.md"]);
    assert_eq!(code(&run), Some(2), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("--description"), "{}", text_of(&run.stderr));
    assert!(!bench.any_posted() && !bench.started());
    // Printed only, it is taken as `wb prompt` takes it.
    let run = start_unsandboxed(&bench, &["start", "build", "54", "--prompt", "--description", "/tmp/elsewhere.md"]);
    assert_eq!(code(&run), Some(0), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stdout).contains("/tmp/elsewhere.md"), "{}", text_of(&run.stdout));
}

#[test]
fn test_WB_PUSH_20_take_back_pushes_nothing_and_says_the_commands() {
    let (bench, origin) = pushing();
    let (folder, _) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 2; }\n", "The tests of 54");

    let run = take_back(&bench, &[]);

    assert_eq!(code(&run), Some(0), "{}", text_of(&run.stderr));
    assert_eq!(refs_of(&origin), "");
    let err = text_of(&run.stderr);
    assert!(err.lines().any(|line| line.starts_with("wb start tester: nothing is pushed: ") && line.contains("--take-back")), "{}", err);
    assert!(err.contains("git push origin issue-54"), "{}", err);

    let run = take_back(&bench, &["--no-push"]);
    assert_eq!(code(&run), Some(2), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("'--take-back' is used with the role and the issue alone"), "{}", text_of(&run.stderr));
}

// ---- #153: a session in a view holds no login: wb brings in what it reads and posts what it writes ----

/// The whole comments as `issue_comment_bodies` prints them: each body and a NUL byte, and a line break after it.
fn bodies_of(comments: &[&str]) -> Vec<u8> {
    let mut bodies = Vec::new();
    for comment in comments {
        bodies.extend_from_slice(comment.as_bytes());
        bodies.extend_from_slice(b"\0\n");
    }
    bodies
}

/// A session that copies each file `wb` brought in to the log folder, as `in-<name>`, and notes whether it could
/// write in the folder, and whether the view saw a file of it.
fn reading_session(bench: &Bench) {
    bench.session(
        "for f in issue.text questions change.text change.comments change.diff; do [ -f /run/watchbill/in/$f ] && cat /run/watchbill/in/$f > {log}/in-$f; done\n\
         ( echo x > /run/watchbill/in/planted ) 2>/dev/null && echo wrote > {log}/wrote\n\
         git status --porcelain --untracked-files=all > {log}/status\n",
    );
}

#[test]
fn test_WB_PUSH_21_the_issue_and_the_role_s_own_questions_are_brought_in_read_only_outside_the_view() {
    if !sandbox("test_WB_PUSH_21") {
        return;
    }
    let bench = bench();
    let comments = [
        "[stamp] tester round 1\nrole: tester\nmodel: claude-opus-5-5",
        "[questions] 54.tester q1\nq1: Which brake?\n<details>\n\n1. (tester) The front brake.\n\n</details>",
        "[answers] 54.tester q1>tester\nq1: 1",
        "[questions] 54.build q1\nq1: Which build?",
        "[questions] 54 q1\nq1: The planner's own.",
        "Thanks, that reads well.",
    ];
    fs::write(bench.at("text-54"), "Brakes: the front one.\n").unwrap();
    fs::write(bench.at("bodies-54"), bodies_of(&comments)).unwrap();
    reading_session(&bench);

    ok(&bench.start(&["start", "tester", "54"]));

    assert_eq!(bench.left("in-issue.text"), "Brakes: the front one.\n");
    let questions = bench.left("in-questions");
    assert!(questions.contains(comments[1]) && questions.contains(comments[2]), "{}", questions);
    for never in ["[stamp]", "model: claude", "54.build", "The planner's own", "Thanks"] {
        assert!(!questions.contains(never), "{}: {}", never, questions);
    }
    // Read-only, and not in the view's tree.
    assert_eq!(bench.left("wrote"), "");
    assert_eq!(bench.left("status"), "");
    // Read by `wb` before the stamp is posted.
    let calls = bench.calls();
    let text = calls.iter().position(|call| call == "text 54").expect("issue_text was not run");
    let bodies = calls.iter().position(|call| call == "bodies 54").expect("issue_comment_bodies was not run");
    let posted = calls.iter().position(|call| call == "post issue 54").unwrap();
    assert!(text < posted && bodies < posted, "{:?}", calls);
    // The folder is gone after the session.
    assert!(bench.below("out").is_empty(), "{:?}", bench.below("out"));
}

#[test]
fn test_WB_PUSH_21_every_role_but_the_planner_gets_the_issue_s_text() {
    if !sandbox("test_WB_PUSH_21") {
        return;
    }
    for role in ["build", "review", "fix"] {
        let bench = bench();
        review_stamp(&bench);
        fs::write(bench.at("text-54"), "Brakes: the front one.\n").unwrap();
        reading_session(&bench);
        ok(&bench.start(&["start", role, "54"]));
        assert_eq!(bench.left("in-issue.text"), "Brakes: the front one.\n", "{}", role);
    }
}

#[test]
fn test_WB_PUSH_21_a_failing_or_missing_issue_text_or_issue_comment_bodies_refuses_the_tester_before_posting() {
    for (status, named) in [("text.status", "issue_text"), ("bodies.status", "issue_comment_bodies")] {
        let bench = bench();
        fs::write(bench.at(status), "1\n").unwrap();
        let run = bench.start_by(&pass_through(&bench), &["start", "tester", "54"]);
        refused_before_posting(&bench, &run, &[named]);
    }
    for key in ["issue_text", "issue_comment_bodies"] {
        let bench = bench();
        without_line(&bench, key);
        let run = bench.start_by(&pass_through(&bench), &["start", "build", "54"]);
        refused_before_posting(&bench, &run, &[key]);
    }
    let bench = bench();
    fs::write(bench.at("text-54"), "a".repeat(1024 * 1024 + 1)).unwrap();
    let run = bench.start_by(&pass_through(&bench), &["start", "tester", "54"]);
    refused_before_posting(&bench, &run, &["issue_text", "1 MiB"]);
}

#[test]
fn test_WB_PUSH_22_the_review_gets_the_change_s_description_and_the_comments_of_the_review_and_the_fix() {
    if !sandbox("test_WB_PUSH_22") {
        return;
    }
    let bench = bench();
    let comments = [
        "[stamp] review round 1\nrole: review",
        "[review] 54 round 1\n1. must src/brake.c:1 returns 1.",
        "[fix] 54 round 1\n1. fixed: Fix finding 1 of round 1",
        "[review] 55 round 1\nAnother issue's.",
        "Looks good to me.",
    ];
    fs::write(bench.at("ctext-174"), "Issue 54: the front brake.\n").unwrap();
    fs::write(bench.at("cbodies-174"), bodies_of(&comments)).unwrap();
    reading_session(&bench);

    ok(&bench.start(&["start", "review", "54"]));

    assert_eq!(bench.left("in-change.text"), "Issue 54: the front brake.\n");
    let seen = bench.left("in-change.comments");
    assert!(seen.contains(comments[1]) && seen.contains(comments[2]), "{}", seen);
    for never in ["[stamp]", "Another issue's", "Looks good"] {
        assert!(!seen.contains(never), "{}: {}", never, seen);
    }
    let calls = bench.calls();
    let posted = calls.iter().position(|call| call == "post issue 54").unwrap();
    assert!(calls.iter().position(|call| call == "ctext 174").is_some_and(|at| at < posted), "{:?}", calls);
    assert!(calls.iter().position(|call| call == "cbodies 174").is_some_and(|at| at < posted), "{:?}", calls);
}

#[test]
fn test_WB_PUSH_22_with_no_issue_the_questions_of_the_review_come_from_the_change() {
    if !sandbox("test_WB_PUSH_22") {
        return;
    }
    let bench = bench();
    let comments = ["[questions] 174.review q1\nq1: Which brake?", "[questions] 174.fix q1\nq1: Not the review's."];
    fs::write(bench.at("cbodies-174"), bodies_of(&comments)).unwrap();
    reading_session(&bench);

    ok(&bench.start(&["start", "review", "--change", "174"]));

    assert_eq!(bench.left("in-issue.text"), "", "no issue, no issue's text");
    let questions = bench.left("in-questions");
    assert!(questions.contains(comments[0]) && !questions.contains("Not the review's"), "{}", questions);
    assert!(!bench.calls().iter().any(|call| call.starts_with("text ")), "{:?}", bench.calls());
}

#[test]
fn test_WB_PUSH_22_a_failing_or_missing_change_text_refuses_the_review_before_posting() {
    let bench = bench();
    fs::write(bench.at("ctext.status"), "1\n").unwrap();
    let run = bench.start_by(&pass_through(&bench), &["start", "review", "54"]);
    refused_before_posting(&bench, &run, &["change_text"]);
    let bench = self::bench();
    fs::write(bench.at("cbodies.status"), "1\n").unwrap();
    let run = bench.start_by(&pass_through(&bench), &["start", "review", "54"]);
    refused_before_posting(&bench, &run, &["change_comment_bodies"]);
    let bench = self::bench();
    without_line(&bench, "change_text");
    let run = bench.start_by(&pass_through(&bench), &["start", "review", "54"]);
    refused_before_posting(&bench, &run, &["change_text", "wb upgrade"]);
}

#[test]
fn test_WB_PUSH_23_the_diff_holds_only_the_paths_of_the_role_s_lists() {
    if !sandbox("test_WB_PUSH_23") {
        return;
    }
    let bench = bench();
    commit_file(&bench, "src/brake.c", "int brake(void) { return 2; }\n");
    commit_file(&bench, "src/secret.c", "the third secret\n");
    reading_session(&bench);

    ok(&bench.start(&["start", "review", "54"]));

    let diff = bench.left("in-change.diff");
    assert!(diff.contains("src/brake.c") && diff.contains("+int brake(void) { return 2; }"), "{}", diff);
    for never in ["secret", ".watchbill/issues/54"] {
        assert!(!diff.contains(never), "{}: {}", never, diff);
    }
}

#[test]
fn test_WB_PUSH_24_without_change_text_the_fix_is_refused_naming_the_line_and_wb_upgrade() {
    let bench = bench();
    review_stamp(&bench);
    without_line(&bench, "change_text");
    let run = bench.start_by(&pass_through(&bench), &["start", "fix", "54"]);
    refused_before_posting(&bench, &run, &["change_text", "wb upgrade"]);
}

/// Two questions of the tester, in full, as the session writes them to `{questions}`.
const TESTER_QUESTIONS: &str = "q1: Which brake?
Why it matters: the test differs.

1. (tester) The front brake.
2. (person) Both.

Suggested: 1, because it is first.

q2: Which unit?
Why it matters: the bound differs.
";

/// The comment `wb` makes of them (#295's form).
const TESTER_POSTED: &str = "[questions] 54.tester q1 q2
q1: Which brake?
<details>

Why it matters: the test differs.

1. (tester) The front brake.
2. (person) Both.

Suggested: 1, because it is first.

</details>

q2: Which unit?
<details>

Why it matters: the bound differs.

</details>
";

/// A session that writes `text` to the file `name` of `/run/watchbill`, and ends with `status`.
fn writing_session(bench: &Bench, name: &str, text: &str, status: u8) {
    fs::write(bench.at(&format!("write-{}", name)), text).unwrap();
    bench.session(&format!("cat {{log}}/write-{name} > /run/watchbill/{name}\nexit {status}\n"));
}

#[test]
fn test_WB_PUSH_25_the_questions_written_are_posted_once_in_one_comment_after_the_session() {
    if !sandbox("test_WB_PUSH_25") {
        return;
    }
    let bench = bench();
    writing_session(&bench, "questions", TESTER_QUESTIONS, 0);

    let run = bench.start(&["start", "tester", "54"]);

    ok(&run);
    assert_eq!(posts_on_54(&bench), 2, "{:?}", bench.calls());
    assert_eq!(bench.posted(), TESTER_POSTED);
    let calls = bench.calls();
    let start = calls.iter().position(|call| call == "start").unwrap();
    assert_eq!(calls.iter().rposition(|call| call == "post issue 54").map(|at| at > start), Some(true), "{:?}", calls);
    let said = format!("wb start tester: will post on issue 54, by `issue_comment` of watchbill.yml:\n{}", TESTER_POSTED);
    assert!(text_of(&run.stderr).contains(&said), "{}", text_of(&run.stderr));

    // A question that stands on the job's ref already is not posted again (#238).
    let bench = crate::bench();
    stamps_on_ref(&bench.repo.path, "54", &["[questions] 54.tester q1\n\nq1: Which brake?\n"]);
    writing_session(&bench, "questions", TESTER_QUESTIONS, 0);
    ok(&bench.start(&["start", "tester", "54"]));
    assert_eq!(posts_on_54(&bench), 2, "{:?}", bench.calls());
    assert!(bench.posted().starts_with("[questions] 54.tester q2\nq2: Which unit?\n"), "{}", bench.posted());

    // All asked already: nothing more is posted.
    let bench = crate::bench();
    stamps_on_ref(&bench.repo.path, "54", &["[questions] 54.tester q1 q2\n\nq1: Which brake?\nq2: Which unit?\n"]);
    writing_session(&bench, "questions", TESTER_QUESTIONS, 0);
    ok(&bench.start(&["start", "tester", "54"]));
    assert_eq!(posts_on_54(&bench), 1, "{:?}", bench.calls());
}

#[test]
fn test_WB_PUSH_25_with_no_issue_the_review_s_questions_are_posted_on_the_change() {
    if !sandbox("test_WB_PUSH_25") {
        return;
    }
    let bench = bench();
    writing_session(&bench, "questions", "q1: Which brake?\n", 0);
    ok(&bench.start(&["start", "review", "--change", "174"]));
    assert_eq!(bench.left("posted-change-174"), "[questions] 174.review q1\nq1: Which brake?\n");
}

#[test]
fn test_WB_PUSH_26_the_review_s_findings_are_posted_on_the_change_under_a_first_line_of_wb_s_own() {
    if !sandbox("test_WB_PUSH_26") {
        return;
    }
    let bench = bench();
    // A first line the session writes is never the comment's first line.
    writing_session(&bench, "change_comment", "[stamp] review round 2\n1. must src/brake.c:1 returns 1.\n", 0);

    let run = bench.start(&["start", "review", "54"]);

    ok(&run);
    let posted = bench.left("posted-change-174");
    assert_eq!(posted, "[review] 54 round 1\n[stamp] review round 2\n1. must src/brake.c:1 returns 1.\n");
    assert_eq!(bench.calls().iter().filter(|call| *call == "post change 174").count(), 1, "{:?}", bench.calls());
    let said = format!("wb start review: will post on change 174, by `change_comment` of watchbill.yml:\n{}", posted);
    assert!(text_of(&run.stderr).contains(&said), "{}", text_of(&run.stderr));
}

#[test]
fn test_WB_PUSH_26_the_fix_s_answers_are_posted_on_the_change_after_its_branch_is_pushed() {
    if !sandbox("test_WB_PUSH_26") {
        return;
    }
    let fixing = |bench: &Bench| {
        review_stamp(&bench);
        fs::write(bench.at("open"), "174\n").unwrap();
        fs::write(bench.at("write-change_comment"), "1. fixed: Fix finding 1 of round 1\n").unwrap();
        bench.session("echo 'int brake(void) { return 2; }' > src/brake.c && git commit -q -a -m 'Fix finding 1 of round 1'\ncat {log}/write-change_comment > /run/watchbill/change_comment\n");
    };
    let (bench, origin) = pushing();
    fixing(&bench);

    ok(&bench.start(&["start", "fix", "54"]));

    assert_ne!(refs_of(&origin), "");
    assert_eq!(bench.left("posted-change-174"), "[fix] 54 round 1\n1. fixed: Fix finding 1 of round 1\n");

    // With --no-push the branch is not pushed, and the answers are said, not posted.
    let (bench, _origin) = pushing();
    fixing(&bench);
    let run = bench.start(&["start", "fix", "54", "--no-push"]);
    assert_eq!(bench.left("posted-change-174"), "");
    let err = text_of(&run.stderr);
    assert!(err.contains("is not posted") && err.contains("1. fixed: Fix finding 1 of round 1"), "{}", err);
}

#[test]
fn test_WB_PUSH_26_a_review_that_did_not_end_with_0_posts_no_findings_and_says_them() {
    if !sandbox("test_WB_PUSH_26") {
        return;
    }
    let bench = bench();
    writing_session(&bench, "change_comment", "1. must src/brake.c:1 returns 1.\n", 3);
    let run = bench.start(&["start", "review", "54"]);
    assert_eq!(code(&run), Some(1));
    assert_eq!(bench.left("posted-change-174"), "");
    let err = text_of(&run.stderr);
    assert!(err.contains("is not posted") && err.contains("did not end with 0") && err.contains("1. must src/brake.c:1 returns 1."), "{}", err);
}

#[test]
fn test_WB_PUSH_27_needs_person_is_posted_on_the_issue_whatever_the_session_ended_with() {
    if !sandbox("test_WB_PUSH_27") {
        return;
    }
    let bench = bench();
    writing_session(&bench, "needs_person", "The CRC part of build.rs, the person's step of 3.1, is not there.\n", 1);
    let run = bench.start(&["start", "build", "54"]);
    assert_eq!(code(&run), Some(1));
    assert_eq!(posts_on_54(&bench), 2, "{:?}", bench.calls());
    let posted = bench.posted();
    assert_eq!(posted, "[needs_person] 54.build\nThe CRC part of build.rs, the person's step of 3.1, is not there.\n");
    assert!(text_of(&run.stderr).contains(&format!("will post on issue 54, by `issue_comment` of watchbill.yml:\n{}", posted)), "{}", text_of(&run.stderr));

    // `none`, or nothing, posts nothing.
    for written in ["none\n", "  \n"] {
        let bench = crate::bench();
        writing_session(&bench, "needs_person", written, 0);
        ok(&bench.start(&["start", "build", "54"]));
        assert_eq!(posts_on_54(&bench), 1, "{:?}: {:?}", written, bench.calls());
    }
}

#[test]
fn test_WB_PUSH_27_the_planner_s_needs_person_is_named_by_the_issue_alone() {
    if !sandbox("test_WB_PUSH_27") {
        return;
    }
    let bench = bench();
    writing_session(&bench, "needs_person", "The files were not there.\n", 0);
    ok(&bench.start(&["start", "planner", "54"]));
    assert_eq!(bench.posted(), "[needs_person] 54\nThe files were not there.\n");
}

#[test]
fn test_WB_PUSH_28_a_written_file_with_an_unsafe_character_is_refused_and_nothing_of_it_is_posted() {
    if !sandbox("test_WB_PUSH_28") {
        return;
    }
    for name in ["questions", "needs_person", "change_comment"] {
        let bench = bench();
        writing_session(&bench, name, "q1: red \u{1b}[31m here\n", 0);
        let role = if name == "change_comment" { "review" } else { "tester" };
        let run = bench.start(&["start", role, "54"]);
        let err = text_of(&run.stderr);
        assert_eq!(code(&run), Some(1), "{}: {}", name, err);
        assert!(labelled_line(&err, &format!("wb start {}", role), "limit").is_some(), "{}: {}", name, err);
        assert!(!err.contains('\u{1b}'), "{}: {:?}", name, err);
        assert_eq!(posts_on_54(&bench), 1, "{}: {:?}", name, bench.calls());
        assert_eq!(bench.left("posted-change-174"), "", "{}", name);
    }
}

#[test]
fn test_WB_PUSH_28_a_written_file_over_64_kib_or_a_link_is_refused() {
    if !sandbox("test_WB_PUSH_28") {
        return;
    }
    let bench = bench();
    bench.session("head -c 65537 /dev/zero | tr '\\0' a > /run/watchbill/needs_person\n");
    let run = bench.start(&["start", "tester", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert_eq!(posts_on_54(&bench), 1, "{:?}", bench.calls());

    let bench = self::bench();
    bench.session("ln -s /etc/hostname /run/watchbill/questions\n");
    let run = bench.start(&["start", "tester", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert_eq!(posts_on_54(&bench), 1, "{:?}", bench.calls());
}

#[test]
fn test_WB_PUSH_28_questions_that_do_not_start_with_their_name_are_refused() {
    if !sandbox("test_WB_PUSH_28") {
        return;
    }
    let bench = bench();
    writing_session(&bench, "questions", "Which brake?\nq1: Which unit?\n", 0);
    let run = bench.start(&["start", "tester", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text_of(&run.stderr));
    assert_eq!(posts_on_54(&bench), 1, "{:?}", bench.calls());
}

#[test]
fn test_WB_PUSH_29_the_folder_wb_brings_in_and_the_files_a_session_writes_are_named_in_the_prompt() {
    if !sandbox("test_WB_PUSH_29") {
        return;
    }
    let bench = bench();
    review_stamp(&bench);
    bench.session("printf '%s' \"$3\" > {log}/prompt\n");
    ok(&bench.start(&["start", "review", "54"]));
    let prompt = bench.left("prompt");
    for path in ["/run/watchbill/in/issue.text", "/run/watchbill/in/change.diff", "/run/watchbill/questions", "/run/watchbill/change_comment", "/run/watchbill/needs_person"] {
        assert!(prompt.contains(path), "{}: {}", path, prompt);
    }
    assert!(!prompt.contains("address.sh") && !prompt.contains("server 174"), "{}", prompt);
}

#[test]
fn test_WB_PUSH_30_the_new_placeholders_given_to_start_are_used_wrongly_but_where_it_prints() {
    for option in ["--brought", "--questions", "--change_comment", "--needs_person"] {
        let bench = bench();
        let run = start_unsandboxed(&bench, &["start", "build", "54", option, "/tmp/elsewhere"]);
        assert_eq!(code(&run), Some(2), "{}: {}", option, text_of(&run.stderr));
        assert!(text_of(&run.stderr).contains(option), "{}", text_of(&run.stderr));
        assert!(!bench.any_posted() && !bench.started());
        let run = start_unsandboxed(&bench, &["start", "build", "54", "--prompt", option, "/tmp/elsewhere"]);
        assert_eq!(code(&run), Some(0), "{}: {}", option, text_of(&run.stderr));
    }
}

// ---- #153, the review of PR #310 ----

#[test]
fn test_WB_PUSH_23_an_empty_list_of_paths_brings_in_no_diff() {
    if !sandbox("test_WB_PUSH_23") {
        return;
    }
    // No `changes`, and no `reads` of the review: the view is empty, and so is the diff it is handed.
    let file = ISSUE_54
        .replace("changes: [src/brake.c, tests/test_brake.c, src/new.c]", "changes: []")
        .replace("  review: [src/brake.h]\n", "  review: []\n");
    let bench = bench_for(&file);
    commit_file(&bench, "src/brake.c", "int brake(void) { return 2; }\n");
    commit_file(&bench, "src/secret.c", "the third secret\n");
    reading_session(&bench);

    ok(&bench.start(&["start", "review", "54"]));

    let diff = bench.left("in-change.diff");
    for never in ["diff --git", "brake", "secret", ".watchbill/issues/54"] {
        assert!(!diff.contains(never), "{}: {}", never, diff);
    }
    // Its one line says which commits it is of.
    assert_eq!(diff.lines().count(), 1, "{}", diff);
}

#[test]
fn test_WB_PUSH_23_the_first_line_of_the_diff_names_the_commit_it_ends_at_and_its_base() {
    if !sandbox("test_WB_PUSH_23") {
        return;
    }
    let bench = bench();
    commit_file(&bench, "src/brake.c", "int brake(void) { return 2; }\n");
    reading_session(&bench);

    ok(&bench.start(&["start", "review", "54"]));

    let diff = bench.left("in-change.diff");
    let first = diff.lines().next().unwrap_or_default();
    let tip = bench.git_text(&["rev-parse", "refs/heads/issue-54"]);
    let base = bench.git_text(&["merge-base", "main", "issue-54"]);
    assert!(first.starts_with("wb: ") && first.contains("refs/heads/issue-54"), "{}", first);
    assert!(first.contains(tip.trim()) && first.contains(base.trim()), "{} {} {}", first, tip, base);
    assert!(diff.lines().nth(1).is_some_and(|line| line.starts_with("diff --git")), "{}", diff);
}

#[test]
fn test_WB_PUSH_23_the_person_s_diff_submodule_does_not_change_the_diff() {
    if !sandbox("test_WB_PUSH_23") {
        return;
    }
    let bench = bench();
    // A submodule at a path of `changes`, whose repository the checkout does not hold.
    let commit = bench.git_text(&["rev-parse", "HEAD"]);
    git(&bench.repo.path, &["update-index", "--add", "--cacheinfo", &format!("160000,{},src/new.c", commit.trim())]);
    git(&bench.repo.path, &["commit", "-q", "-m", "A submodule"]);
    // Not checked out: an empty folder, as git leaves a submodule it was not asked to fetch.
    fs::create_dir_all(bench.repo.path.join("src/new.c")).unwrap();
    git(&bench.repo.path, &["config", "diff.submodule", "diff"]);
    reading_session(&bench);

    ok(&bench.start(&["start", "review", "54"]));

    let diff = bench.left("in-change.diff");
    assert!(diff.contains(&format!("+Subproject commit {}", commit.trim())), "{}", diff);
    assert!(!diff.contains("Submodule src/new.c"), "{}", diff);
}

#[test]
fn test_WB_PUSH_26_a_fix_that_commits_nothing_posts_its_answers() {
    if !sandbox("test_WB_PUSH_26") {
        return;
    }
    let (bench, origin) = pushing();
    review_stamp(&bench);
    fs::write(bench.at("open"), "174\n").unwrap();
    writing_session(&bench, "change_comment", "1. not fixed: the finding names a bound the issue sets.\n", 0);

    ok(&bench.start(&["start", "fix", "54"]));

    assert_eq!(refs_of(&origin), "");
    assert_eq!(bench.left("posted-change-174"), "[fix] 54 round 1\n1. not fixed: the finding names a bound the issue sets.\n");
}

#[test]
fn test_WB_PUSH_26_a_fix_of_a_change_with_no_issue_posts_its_answers_where_it_commits_nothing() {
    if !sandbox("test_WB_PUSH_26") {
        return;
    }
    let bench = bench();
    fs::write(bench.at("comments-change-174"), "[stamp] review round 1\n").unwrap();
    git(&bench.repo.path, &["checkout", "-q", "issue-99"]);
    writing_session(&bench, "change_comment", "1. not fixed: the finding names a bound the issue sets.\n", 0);

    let run = bench.start(&["start", "fix", "--change", "174", "--branch", "issue-99"]);

    ok(&run);
    assert_eq!(bench.left("posted-change-174"), "[fix] 174 round 1\n1. not fixed: the finding names a bound the issue sets.\n", "{}", text_of(&run.stderr));
}

#[test]
fn test_WB_PUSH_26_a_fix_of_a_change_with_no_issue_that_commits_says_why_its_answers_are_held_back() {
    if !sandbox("test_WB_PUSH_26") {
        return;
    }
    let bench = bench();
    fs::write(bench.at("comments-change-174"), "[stamp] review round 1\n").unwrap();
    git(&bench.repo.path, &["checkout", "-q", "issue-99"]);
    fs::write(bench.at("write-change_comment"), "1. fixed: Fix finding 1 of round 1\n").unwrap();
    bench.session("echo 'int brake(void) { return 2; }' > src/brake.c && git commit -q -a -m 'Fix finding 1 of round 1'\ncat {log}/write-change_comment > /run/watchbill/change_comment\n");

    let run = bench.start(&["start", "fix", "--change", "174", "--branch", "issue-99"]);

    // The stamp alone is posted on the change.
    assert_eq!(bench.calls().iter().filter(|call| *call == "post change 174").count(), 1, "{:?}", bench.calls());
    assert!(bench.left("posted-change-174").starts_with("[stamp] fix round 1\n"), "{}", bench.left("posted-change-174"));
    let err = text_of(&run.stderr);
    let held = err.lines().find(|line| line.contains("is not posted")).unwrap_or_else(|| panic!("{}", err));
    assert!(held.contains("there is no issue") && !held.contains("was not pushed"), "{}", held);
    assert!(err.contains("1. fixed: Fix finding 1 of round 1"), "{}", err);
}

#[test]
fn test_WB_PUSH_27_none_in_any_case_with_backticks_or_a_full_stop_posts_nothing() {
    if !sandbox("test_WB_PUSH_27") {
        return;
    }
    for written in ["None.\n", "`none`\n", " NONE \n", "`None`.\n", "none.\n"] {
        let bench = bench();
        writing_session(&bench, "needs_person", written, 0);
        ok(&bench.start(&["start", "build", "54"]));
        assert_eq!(posts_on_54(&bench), 1, "{:?}: {:?}", written, bench.calls());
    }
    // A text that only starts with it is posted.
    let bench = bench();
    writing_session(&bench, "needs_person", "None of the parts is there.\n", 0);
    ok(&bench.start(&["start", "build", "54"]));
    assert_eq!(posts_on_54(&bench), 2, "{:?}", bench.calls());
}

#[test]
fn test_WB_ROLE_119_the_tester_s_prompt_as_handed_out_names_the_files_wb_brings_in_and_no_program_of_the_tracker() {
    let bench = bench();
    let run = prompt_with(&bench.repo.path, &bench.organisation(), &["tester", "54", "--round", "1"]);
    ok(&run);
    let prompt = text_of(&run.stdout);
    assert!(prompt.contains("/run/watchbill/in/issue.text") && prompt.contains("/run/watchbill/questions"), "{}", prompt);
    let words: Vec<&str> = prompt.split(|c: char| !c.is_ascii_alphanumeric()).collect();
    assert!(!prompt.contains("address.sh") && !words.contains(&"gh") && !words.contains(&"glab"), "{}", prompt);
}

// ---- WB-VIEW-149 and 150: what a start says of the network of its view (#195) ----

/// The runner of the bench with `reach` as written, and the forwarder of this build: the test process is not `wb`.
fn reaching(bench: &Bench, reach: &str) {
    let text = read(&bench.organisation()).replace("    binds:", &format!("    reach: {}\n    binds:", reach));
    fs::write(bench.organisation(), text).unwrap();
    watchbill::reach::use_forwarder(Path::new(env!("CARGO_BIN_EXE_wb")));
}

/// A port of this machine's loopback that nothing listens on.
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port()
}

#[test]
fn test_WB_VIEW_149_the_stamp_the_lines_and_the_start_say_the_endpoints_the_session_reaches() {
    if !sandbox("test_WB_VIEW_149") {
        return;
    }
    let bench = bench();
    let port = free_port();
    reaching(&bench, &format!("['127.0.0.1:{}', api.example.org:443]", port));

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let posted = bench.posted();
    let line = format!("\nreach: 127.0.0.1:{} api.example.org:443\n", port);
    let at = posted.find(&line).unwrap_or_else(|| panic!("{}", posted));
    assert!(posted.find("\npart: ").is_some_and(|part| at < part), "{}", posted);
    assert_eq!(read(&bench.lines().join("reach")), format!("127.0.0.1:{}\napi.example.org:443\n", port));
    let err = text(&run.stderr);
    let said = format!("wb start build: the session reaches only 127.0.0.1:{}, api.example.org:443, through wb's proxy", port);
    let said_at = err.find(&said).unwrap_or_else(|| panic!("{}", err));
    assert!(err.find("wb start build: starts build").is_some_and(|starts| said_at < starts), "{}", err);
    // `--stamp` starts no session, and has no line `reach:`.
    let printed = bench.start(&["start", "build", "54", "--stamp"]);
    assert!(!text(&printed.stdout).contains("reach:"), "{}", text(&printed.stdout));
}

#[test]
fn test_WB_VIEW_149_a_runner_with_no_reach_is_said_to_reach_no_network() {
    if !sandbox("test_WB_VIEW_149") {
        return;
    }
    let bench = bench();

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    assert!(bench.posted().contains("\nreach: none\n"), "{}", bench.posted());
    assert_eq!(read(&bench.lines().join("reach")), "none\n");
    let err = text(&run.stderr);
    assert!(err.contains("wb start build: the session reaches no network: the runner 'stand-in' has no reach"), "{}", err);
}

#[test]
fn test_WB_VIEW_150_after_the_session_wb_says_how_many_connections_were_refused_and_where() {
    if !sandbox("test_WB_VIEW_150") {
        return;
    }
    let bench = bench();
    reaching(&bench, &format!("['127.0.0.1:{}']", free_port()));
    let l = bench.log.path.to_str().unwrap();
    let ask = "exec 3<>/dev/tcp/127.0.0.1/3128 && printf 'CONNECT example.org:443 HTTP/1.1\\r\\n\\r\\n' >&3 && read -r -t 15 status <&3; exec 3<&-\n";
    fs::write(bench.at("session.sh"), format!("#!/bin/bash\necho start >> {l}/calls\n{ask}{ask}exit 0\n")).unwrap();

    let run = bench.start(&["start", "build", "54"]);

    // A refusal does not change how the session ended.
    ok(&run);
    let err = text(&run.stderr);
    assert!(err.contains("wb start build: the proxy refused 2 connections"), "{}", err);
    let file = bench.lines().join("refused");
    assert!(err.contains(file.to_str().unwrap()), "{}", err);
    let refused = read(&file);
    assert_eq!(refused.lines().count(), 2, "{}", refused);
    assert!(refused.lines().all(|line| line.starts_with("CONNECT example.org:443 HTTP/1.1: ")), "{}", refused);
}

// ---- #237: each commit taken back carries a note that names its session's stamp (docs/revision.md, point 15) ----

/// The notes of the sessions, as a person reads them.
const NOTES: &str = "refs/notes/watchbill";

/// The bare repository the pushes of the bench go to.
fn origin_of(bench: &Bench) -> PathBuf {
    bench.repo.path.parent().unwrap().join("origin.git")
}

/// What git printed in `folder`, and whether it ended with 0.
fn git_in(folder: &Path, args: &[&str]) -> (bool, String) {
    let run = Command::new("git").arg("-C").arg(folder).args(args).env_clear().envs(env()).stderr(std::process::Stdio::null()).output().unwrap();
    (run.status.success(), text_of(&run.stdout))
}

/// The note of `commit` in `folder`, or none.
fn note_of(folder: &Path, commit: &str) -> Option<String> {
    let (found, said) = git_in(folder, &["notes", &format!("--ref={}", NOTES), "show", commit]);
    found.then_some(said)
}

/// The commits on the checkout's `HEAD` after `head`, oldest first.
fn commits_after(bench: &Bench, head: &str) -> Vec<String> {
    bench.git_text(&["rev-list", "--reverse", &format!("{}..HEAD", head)]).lines().map(str::to_string).collect()
}

/// A build's session of two commits.
fn two_commits(bench: &Bench) {
    bench.session(
        "echo 'int brake(void) { return 2; }' > src/brake.c\n\
         git add src/brake.c && git commit -q -m 'First: brake returns 2'\n\
         echo 'int fresh;' > src/new.c\n\
         git add src/new.c && git commit -q -m 'Second: a new file'\n",
    );
}

#[test]
fn test_WB_VIEW_36_each_commit_taken_back_carries_a_note_naming_the_stamp_commit_and_the_role() {
    if !sandbox("test_WB_VIEW_36") {
        return;
    }
    let bench = bench();
    let head = bench.head();
    two_commits(&bench);

    ok(&bench.start(&["start", "build", "54"]));

    let stamp = bench.git_text(&["rev-parse", "refs/watchbill/54"]).trim().to_string();
    assert!(bench.git_text(&["log", "-1", "--format=%s", &stamp]).starts_with("[stamp] build round 1"));
    let commits = commits_after(&bench, &head);
    assert_eq!(commits.len(), 2);
    for commit in &commits {
        assert_eq!(note_of(&bench.repo.path, commit), Some(format!("stamp: {}\nrole: build\n", stamp)), "the note of {}", commit);
    }
    // Only those: the commits before the session carry none.
    assert_eq!(bench.git_text(&["notes", &format!("--ref={}", NOTES), "list"]).lines().count(), 2);

    // A note is not signed and `wb` never reads one: notes that claim another stamp, role and session change nothing
    // that `wb` says of the job.
    let said = |bench: &Bench| {
        let run = wb_state_bounded(bench, &["status", "54"], 20).expect("status did not end");
        (code(&run), text_of(&run.stdout))
    };
    let before = said(&bench);
    for commit in &commits {
        let forged = format!("stamp: {}\nrole: tester\n", "1".repeat(stamp.len()));
        git(&bench.repo.path, &["notes", &format!("--ref={}", NOTES), "add", "-f", "-m", forged.trim_end(), commit]);
    }
    assert_eq!(said(&bench), before);
}

#[test]
fn test_WB_VIEW_36_a_commit_taken_back_later_carries_the_note_of_the_stamp_the_kept_session_wrote() {
    let bench = bench();
    let (folder, session) = kept_tester(&bench);
    let view = line_of(&read(&session.join("stamp")), "view").unwrap();
    stamps_on_ref(&bench.repo.path, "54", &[&format!("[stamp] tester round 1\nview: {}\ntime: 2026-11-22T10:00:00Z", view)]);
    let stamp = bench.git_text(&["rev-parse", "refs/watchbill/54"]).trim().to_string();
    // The kept session's `stamp` is the text of that commit, as a start leaves it.
    fs::write(session.join("stamp"), bench.git_text(&["log", "-1", "--format=%B", &stamp])).unwrap();
    commit_in(&folder, "src/brake.c", "int brake(void) { return 2; }\n", "The tests of 54");

    let run = take_back(&bench, &[]);

    assert_eq!(code(&run), Some(0), "{}", text_of(&run.stderr));
    let commit = bench.git_text(&["rev-parse", "HEAD"]).trim().to_string();
    assert_eq!(note_of(&bench.repo.path, &commit), Some(format!("stamp: {}\nrole: tester\n", stamp)));
    // A take-back pushes nothing, so no note: the command is said.
    assert!(!git_in(&origin_of(&bench), &["rev-parse", "--verify", "-q", NOTES]).0);
    assert!(text_of(&run.stderr).contains(&format!("git push origin {}", NOTES)), "{}", text_of(&run.stderr));
    assert!(text_of(&run.stderr).contains("a `git fetch` of origin before they are pushed replaces them"), "{}", text_of(&run.stderr));
}

#[test]
fn test_WB_VIEW_37_a_session_that_commits_nothing_writes_no_note_and_pushes_none() {
    if !sandbox("test_WB_VIEW_37") {
        return;
    }
    let (bench, origin) = pushing();
    ok(&bench.start(&["start", "build", "54"]));
    assert!(!git_in(&bench.repo.path, &["rev-parse", "--verify", "-q", NOTES]).0, "a notes ref was made");
    assert!(!git_in(&origin.path.clone(), &["rev-parse", "--verify", "-q", NOTES]).0, "a notes ref was pushed");

    // After a session that did commit, a session that commits nothing leaves the notes as they are: here and at origin.
    two_commits(&bench);
    ok(&bench.start(&["start", "build", "54", "--round", "2"]));
    let notes = bench.git_text(&["rev-parse", NOTES]).trim().to_string();
    assert_eq!(git_in(&origin.path.clone(), &["rev-parse", NOTES]).1.trim(), notes);
    bench.session("exit 0\n");
    ok(&bench.start(&["start", "build", "54", "--round", "3"]));
    assert_eq!(bench.git_text(&["rev-parse", NOTES]).trim(), notes, "a note commit was made for a session with no commit");
    assert_eq!(git_in(&origin.path.clone(), &["rev-parse", NOTES]).1.trim(), notes);
}

#[test]
fn test_WB_VIEW_37_a_fix_of_a_change_with_no_issue_has_no_stamp_commit_so_its_commits_carry_no_note_and_it_is_said() {
    if !sandbox("test_WB_VIEW_37") {
        return;
    }
    let bench = bench();
    fs::write(bench.at("comments-change-174"), "[stamp] review round 1\n").unwrap();
    git(&bench.repo.path, &["checkout", "-q", "issue-99"]);
    let head = bench.head();
    bench.session("echo 'int brake(void) { return 2; }' > src/brake.c && git commit -q -a -m 'Fix finding 1 of round 1'\n");

    let run = bench.start(&["start", "fix", "--change", "174", "--branch", "issue-99"]);

    assert_eq!(commits_after(&bench, &head).len(), 1, "{}", text_of(&run.stderr));
    assert!(!git_in(&bench.repo.path, &["rev-parse", "--verify", "-q", NOTES]).0, "a notes ref was made");
    let err = text_of(&run.stderr);
    assert!(err.contains("no note is written") && err.contains("a comment"), "{}", err);
}

#[test]
fn test_WB_VIEW_37_a_take_back_later_finds_no_stamp_commit_writes_no_note_says_so_and_still_takes_the_commit() {
    let bench = bench();
    let (folder, _) = kept_tester(&bench);
    commit_in(&folder, "src/brake.c", "int brake(void) { return 2; }\n", "The tests of 54");

    let run = take_back(&bench, &[]);

    assert_eq!(code(&run), Some(0), "{}", text_of(&run.stderr));
    assert_eq!(subjects(&bench, 1), vec!["The tests of 54"]);
    assert!(!git_in(&bench.repo.path, &["rev-parse", "--verify", "-q", NOTES]).0, "a notes ref was made");
    assert!(text_of(&run.stderr).contains("no note is written"), "{}", text_of(&run.stderr));
}

#[test]
fn test_WB_VIEW_38_and_39_a_clone_that_fetched_with_the_lines_wb_init_wrote_finds_the_commits_of_a_session_by_their_notes() {
    if !sandbox("test_WB_VIEW_39") {
        return;
    }
    let (bench, origin) = pushing();
    let head = bench.head();
    two_commits(&bench);
    ok(&bench.start(&["start", "build", "54"]));
    let stamp = bench.git_text(&["rev-parse", "refs/watchbill/54"]).trim().to_string();
    let commits = commits_after(&bench, &head);
    // The branch was pushed by the start, and the notes after it.
    assert!(git_in(&origin.path, &["rev-parse", "refs/heads/issue-54"]).0);

    // A clone with no tracker and no `wb`: git alone, with the fetch lines `wb init` wrote in the checkout.
    let clone = folder_named("clone");
    let bare = origin.path.clone();
    git(&clone.path, &["init", "-q"]);
    git(&clone.path, &["remote", "add", "origin", bare.to_str().unwrap()]);
    let lines = bench.git_text(&["config", "--get-all", "remote.origin.fetch"]);
    assert!(lines.lines().any(|line| line == format!("+{0}*:{0}*", NOTES)), "{}", lines);
    for line in lines.lines().filter(|line| line.contains("watchbill")) {
        git(&clone.path, &["config", "--add", "remote.origin.fetch", line]);
    }
    git(&clone.path, &["fetch", "-q", "origin"]);

    let (_, listed) = git_in(&clone.path, &["notes", &format!("--ref={}", NOTES), "list"]);
    let noted: Vec<&str> = listed.lines().filter_map(|line| line.split(' ').nth(1)).collect();
    assert_eq!(noted.len(), 2, "{}", listed);
    for commit in &commits {
        assert!(noted.contains(&commit.as_str()), "{} is not noted in {}", commit, listed);
        assert_eq!(note_of(&clone.path, commit), Some(format!("stamp: {}\nrole: build\n", stamp)));
    }
    // And the stamp the notes name is there, signed, on the job's ref.
    assert_eq!(git_in(&clone.path, &["rev-parse", "refs/watchbill/54"]).1.trim(), stamp);
}

#[test]
fn test_WB_VIEW_41_after_the_commits_are_taken_back_the_notes_ref_is_pushed_to_origin_with_no_force() {
    if !sandbox("test_WB_VIEW_41") {
        return;
    }
    let (bench, origin) = pushing();
    two_commits(&bench);

    let run = bench.start(&["start", "build", "54"]);

    ok(&run);
    let here = bench.git_text(&["rev-parse", NOTES]).trim().to_string();
    assert_eq!(git_in(&origin.path.clone(), &["rev-parse", NOTES]).1.trim(), here);
    let err = text_of(&run.stderr);
    assert!(err.contains(&format!("git push origin {0}:{0}", NOTES)), "{}", err);
    assert!(!err.contains("--force") && !err.contains(&format!("+{}", NOTES)), "{}", err);
}

#[test]
fn test_WB_VIEW_41_with_no_push_the_notes_are_written_not_pushed_and_the_command_is_said() {
    if !sandbox("test_WB_VIEW_41") {
        return;
    }
    let bench = bench();
    two_commits(&bench);

    let run = bench.start(&["start", "build", "54", "--no-push"]);

    ok(&run);
    assert!(git_in(&bench.repo.path, &["rev-parse", "--verify", "-q", NOTES]).0, "no note was written");
    assert!(!git_in(&origin_of(&bench), &["rev-parse", "--verify", "-q", NOTES]).0, "the notes were pushed");
    assert!(text_of(&run.stderr).contains(&format!("git push origin {}", NOTES)), "{}", text_of(&run.stderr));
}

#[test]
fn test_WB_VIEW_42_a_refused_push_of_the_notes_is_said_ends_with_1_and_leaves_the_commits_and_the_notes() {
    if !sandbox("test_WB_VIEW_42") {
        return;
    }
    let (bench, origin) = pushing();
    // Another start's notes are at origin already, and this clone has not fetched them.
    let bare = origin.path.clone();
    let (_, empty) = git_in(&bare, &["mktree"]);
    let other = git_in(
        &bare,
        &["-c", "user.name=Grace Hopper", "-c", "user.email=grace@example.org", "commit-tree", "-m", "another start's notes", empty.trim()],
    )
    .1
    .trim()
    .to_string();
    git(&bare, &["update-ref", NOTES, &other]);
    let head = bench.head();
    two_commits(&bench);

    let run = bench.start(&["start", "build", "54"]);

    let err = text_of(&run.stderr);
    assert_eq!(code(&run), Some(1), "{}", err);
    assert!(err.contains(NOTES) && err.contains("refused"), "{}", err);
    // The commits stay on the branch, with their notes; origin's notes are as they were.
    let commits = commits_after(&bench, &head);
    assert_eq!(commits.len(), 2, "{}", err);
    assert!(commits.iter().all(|commit| note_of(&bench.repo.path, commit).is_some()), "{}", err);
    assert_eq!(git_in(&bare, &["rev-parse", NOTES]).1.trim(), other);
    // And it says that a plain fetch would replace the notes that are not at origin.
    // The advice (review of #331, finding 3): the fetch has its refspec, so it does not depend on `.git/config`; the kept
    // ref is out of reach of the pattern of the fetch line; and it is deleted at the end.
    for step in [
        "git update-ref refs/notes/wb-kept refs/notes/watchbill",
        "git fetch origin +refs/notes/watchbill:refs/notes/watchbill",
        "git notes --ref=refs/notes/watchbill merge -s union refs/notes/wb-kept",
        "git push origin refs/notes/watchbill:refs/notes/watchbill",
        "git update-ref -d refs/notes/wb-kept",
    ] {
        assert!(err.contains(step), "{}: {}", step, err);
    }
    let at = |step: &str| err.find(step).unwrap();
    assert!(at("git update-ref refs/notes/wb-kept") < at("git fetch origin") && at("git push origin refs/notes/watchbill:refs/notes/watchbill") < at("git update-ref -d"), "{}", err);
}

#[test]
fn test_WB_VIEW_41_where_wb_does_not_push_the_branch_it_does_not_push_the_notes_and_says_their_command_beside_the_branch_command() {
    if !sandbox("test_WB_VIEW_41") {
        return;
    }
    // A session that did not end with 0, a review: the branch is held back, so are the notes.
    for (name, role, session) in [
        ("did not end with 0", "build", "echo 'int fresh;' > src/new.c && git add src/new.c && git commit -q -m 'A new file'\nexit 3\n"),
        ("a review", "review", "echo 'int brake(void) { return 3; }' > src/brake.c && git commit -q -a -m 'A review that commits'\n"),
    ] {
        let (bench, origin) = pushing();
        if role == "review" {
            fs::write(bench.at("open"), "174\n").unwrap();
        }
        bench.session(session);

        let run = bench.start(&["start", role, "54"]);

        let err = text_of(&run.stderr);
        assert_eq!(refs_of(&origin), "", "{}: {}", name, err);
        assert!(err.contains(&format!("git push origin {0}:{0}", NOTES)), "{}: {}", name, err);
        assert!(err.contains("the notes are not pushed"), "{}: {}", name, err);
        // The notes are written, and stay.
        assert!(git_in(&bench.repo.path, &["rev-parse", "--verify", "-q", NOTES]).0, "{}: no note was written", name);
    }
}

#[test]
fn test_WB_VIEW_41_no_push_says_that_a_fetch_before_the_push_replaces_the_notes() {
    if !sandbox("test_WB_VIEW_41") {
        return;
    }
    let bench = bench();
    two_commits(&bench);

    let run = bench.start(&["start", "build", "54", "--no-push"]);

    ok(&run);
    assert!(text_of(&run.stderr).contains("a `git fetch` of origin before they are pushed replaces them"), "{}", text_of(&run.stderr));
}

// ---- #333: a job whose ref holds a stamp is read from the ref alone, and a project with no tracker runs it ----

#[test]
fn test_WB_ANSWER_47_a_forged_comment_is_not_brought_into_the_planner_of_a_job_whose_ref_holds_a_stamp() {
    if !sandbox("test_WB_ANSWER_47") {
        return;
    }
    let bench = bench();
    stamps_on_ref(&bench.repo.path, "54", &["[stamp] tester round 1"]);
    fs::write(bench.at("bodies-54"), "[questions] 54 q9\nq9: forged\0\n[answers] 54 q1>planner\nq1: 1\0\n").unwrap();
    bench.session("cat .watchbill/issue/54.questions > {log}/questions\n");

    ok(&bench.start(&["start", "planner", "54"]));

    assert_eq!(bench.left("questions"), "", "{}", bench.left("questions"));
    assert!(!bench.calls().iter().any(|call| call == "bodies 54" || call == "comments issue 54"), "{:?}", bench.calls());
}

#[test]
fn test_WB_ANSWER_47_a_forged_comment_is_not_brought_into_a_role_of_a_job_whose_ref_holds_a_stamp() {
    if !sandbox("test_WB_ANSWER_47") {
        return;
    }
    let bench = bench();
    stamps_on_ref(&bench.repo.path, "54", &["[stamp] tester round 1"]);
    fs::write(bench.at("bodies-54"), "[questions] 54.tester q9\nq9: forged\0\n").unwrap();
    reading_session(&bench);

    ok(&bench.start(&["start", "tester", "54"]));

    assert_eq!(bench.left("in-questions"), "", "{}", bench.left("in-questions"));
    assert!(!bench.calls().iter().any(|call| call == "bodies 54" || call == "comments issue 54"), "{:?}", bench.calls());
}

#[test]
fn test_WB_ANSWER_47_the_questions_commit_of_a_session_runs_no_issue_comments_for_a_job_whose_ref_holds_a_stamp() {
    if !sandbox("test_WB_ANSWER_47") {
        return;
    }
    let bench = bench();
    // The tracker holds the question open, as an older job's would, and is not read to say so.
    fs::write(bench.at("comments-issue-54"), "[questions] 54.tester q1\n").unwrap();
    writing_session(&bench, "questions", TESTER_QUESTIONS, 0);

    let run = bench.start(&["start", "tester", "54"]);

    ok(&run);
    assert!(!bench.calls().iter().any(|call| call == "comments issue 54"), "{:?}", bench.calls());
    assert!(!text_of(&run.stderr).contains("at the tracker"), "{}", text_of(&run.stderr));
}

#[test]
fn test_WB_ANSWER_49_a_project_with_no_issue_comments_or_issue_comment_bodies_runs_the_planner_the_tester_and_the_build() {
    if !sandbox("test_WB_ANSWER_49") {
        return;
    }
    let bench = bench();
    for key in ["issue_comments", "issue_comment_bodies", "issue_comment"] {
        without_line(&bench, key);
    }
    plan_add(&bench, &["add", "54", "-m", "Brakes: the front one."]);
    next_second();
    plan_accept_again(&bench);

    // The planner's first round: its job has a ref and no stamp, and brings in no question.
    bench.session("cat .watchbill/issue/54.questions > {log}/questions\n");
    ok(&bench.start(&["start", "planner", "54"]));
    assert_eq!(bench.left("questions"), "");
    next_second();
    plan_accept_again(&bench);
    // The two roles after it read the ref alone.
    ok(&bench.start(&["start", "tester", "54"]));
    ok(&bench.start(&["start", "build", "54"]));

    let messages = ref_messages(&bench);
    for (at, role) in ["planner", "tester", "build"].iter().enumerate() {
        assert!(messages[at].starts_with(&format!("[stamp] {} round 1\n", role)), "{:?}", messages);
    }
    let run = wb(&bench.repo.path, &["status", "54"], "");
    assert_eq!(code(&run), Some(0), "{}", text_of(&run.stderr));
    assert!(!text_of(&run.stderr).contains("issue_comments"), "{}", text_of(&run.stderr));
    assert!(!bench.calls().iter().any(|call| call.starts_with("bodies") || call.starts_with("comments") || call.starts_with("post issue")), "{:?}", bench.calls());
}

#[test]
fn test_WB_ANSWER_52_a_job_begun_before_is_brought_the_questions_of_the_tracker_in_its_first_session() {
    if !sandbox("test_WB_ANSWER_52") {
        return;
    }
    let bench = bench();
    // Comment stamps only, and no stamp on the ref: the tracker is read, as #238 left it.
    fs::write(bench.at("bodies-54"), "[stamp] planner round 1\0\n[questions] 54.tester q1\nq1: Which brake?\0\n[answers] 54.tester q1>tester\nq1: 1\0\n").unwrap();
    reading_session(&bench);

    ok(&bench.start(&["start", "tester", "54"]));

    let seen = bench.left("in-questions");
    assert!(seen.contains("[questions] 54.tester q1\nq1: Which brake?") && seen.contains("[answers] 54.tester q1>tester\nq1: 1"), "{}", seen);
    assert!(!seen.contains("[stamp]"), "{}", seen);
    assert!(bench.calls().iter().any(|call| call == "bodies 54"), "{:?}", bench.calls());
}
