//! `wb view` (#173): what a session of a role will see, before it starts, and for one file whether it is in
//! the view and why. It only reads: it makes no view, runs no pattern, writes nothing and asks nothing of the
//! network.
//!
//! The level is read from an organisation's file of the test's own, through `watchbill::view_command`, so
//! that no test reads the real `/etc`. A test of what `wb view` writes runs the program itself.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A repository made for a test, the folder of its stand-ins, and the organisation's file.
struct Bench {
    repo: Folder,
    log: Folder,
    org: Folder,
}

fn script(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\n{}", body)).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

const ORG: &str = "levels: [public, internal]
default: stand-in
runners:
  stand-in:
    tools: this machine
    model: a stand-in
    agreement: none
    up_to: internal
    start: ['{log}/session.sh', '{model}', '{effort}', '{prompt}']
";

/// The issue's file of issue 54: it may change `src/brake.c`, `tests/test_brake.c` and `src/new.c`, which is
/// not there yet; the tester and the build read `src/brake.h` and the folder `include`, the tester also
/// `tests/helper.c`.
const ISSUE_54: &str = "issue: '54'
from: 3f0c1d9e
requirements: []
interface: |
  none
changes: [src/brake.c, tests/test_brake.c, src/new.c]
reads:
  tester: [src/brake.h, include, tests/helper.c]
  build: [src/brake.h, include]
  review: [src/brake.h]
  fix: [src/brake.h]
existing_tests_changed: []
questions: []
";

/// The list of the build of issue 54, as the stamp hashes it.
const BUILD_LIST: &str = "src/brake.c\ntests/test_brake.c\nsrc/new.c\nsrc/brake.h\ninclude\n";

/// A repository set up by `wb init --yes`, with stand-ins for the patterns that note each call, `build_files:
/// [build]`, the level `public`, a `.gitignore` that ignores `*.o`, the code committed, and the accepted file of
/// issue 54 committed on the branch `issue-54`, which is checked out.
fn bench() -> Bench {
    let repo = repository(Some("ssh://example.invalid/brakes/controller.git"));
    let log = folder_named("log");
    let org = folder_named("org");
    let l = log.path.to_str().unwrap();
    script(&log.path.join("post.sh"), &format!("echo \"post $1 $2\" >> {l}/calls\nexit 0\n"));
    script(&log.path.join("comments.sh"), &format!("echo \"comments $1 $2\" >> {l}/calls\nexit 0\n"));
    script(&log.path.join("changes.sh"), &format!("echo \"changes $1\" >> {l}/calls\necho 174\nexit 0\n"));
    script(&log.path.join("needs.sh"), &format!("echo \"needs $1\" >> {l}/calls\nexit 0\n"));
    script(&log.path.join("ssh.sh"), &format!("echo ssh >> {l}/network\nexit 1\n"));
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
    put(&repo.path, ".gitignore", "*.o\n");
    put(&repo.path, "src/brake.c", "int brake(void) { return 1; }\n");
    put(&repo.path, "src/brake.h", "int brake(void);\n");
    put(&repo.path, "src/secret.c", "a secret\n");
    put(&repo.path, "include/a.h", "#define A 1\n");
    put(&repo.path, "include/b.h", "#define B 2\n");
    put(&repo.path, "tests/test_brake.c", "int main(void) { return 0; }\n");
    put(&repo.path, "tests/helper.c", "int helper(void) { return 0; }\n");
    put(&repo.path, "build/Makefile", "all:\n");
    put(&repo.path, "docs/a.md", "a\n");
    put(&repo.path, "docs/deep/b.md", "b\n");
    git(&repo.path, &["add", "--", ".gitignore", ".watchbill", "src", "include", "tests", "build", "docs"]);
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
    fs::write(org.path.join("organisation.yml"), ORG.replace("{log}", l)).unwrap();
    // `wb init` and `wb accept` above leave nothing in the log; what is there after is `wb view`'s.
    let _ = fs::remove_file(log.path.join("calls"));
    Bench { repo, log, org }
}

impl Bench {
    fn organisation(&self) -> PathBuf {
        self.org.path.join("organisation.yml")
    }

    /// `wb view` with the words after `view`, in the test process, with the organisation's file of the test.
    fn view(&self, args: &[&str]) -> Output {
        use std::os::unix::process::ExitStatusExt;
        let args: Vec<std::ffi::OsString> = args.iter().map(std::ffi::OsString::from).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let status = watchbill::view_command(&args, &self.repo.path, &self.organisation(), &mut out, &mut err);
        Output { status: std::process::ExitStatus::from_raw(status << 8), stdout: out, stderr: err }
    }

    fn git_text(&self, args: &[&str]) -> String {
        let run = Command::new("git").arg("-C").arg(&self.repo.path).args(args).env_clear().envs(env()).output().unwrap();
        assert!(run.status.success(), "git {:?}: {}", args, text(&run.stderr));
        text(&run.stdout)
    }

    fn left(&self, name: &str) -> String {
        fs::read_to_string(self.log.path.join(name)).unwrap_or_default()
    }

    /// The line of a role in what `wb view <issue> <path>` printed.
    fn line_of(out: &str, role: &str) -> String {
        out.lines().find(|line| line.starts_with(&format!("{}: ", role))).unwrap_or_else(|| panic!("no line for {}: {}", role, out)).to_string()
    }
}

fn ok(run: &Output) {
    assert_eq!(code(run), Some(0), "{}", text(&run.stderr));
}

/// The value of a line `name: value`.
fn value_of(text: &str, name: &str) -> Option<String> {
    text.lines().find_map(|line| line.strip_prefix(&format!("{}: ", name)).map(str::to_string))
}

/// Every file below `folder`, `.git` too, with its bytes and its time of change.
fn snapshot(folder: &Path) -> BTreeMap<PathBuf, (Vec<u8>, std::time::SystemTime)> {
    let mut found = BTreeMap::new();
    let mut open = vec![folder.to_path_buf()];
    while let Some(at) = open.pop() {
        let meta = fs::symlink_metadata(&at).unwrap();
        if meta.is_dir() {
            for entry in fs::read_dir(&at).unwrap() {
                open.push(entry.unwrap().path());
            }
            found.insert(at.clone(), (Vec::new(), meta.modified().unwrap()));
        } else if meta.is_file() {
            found.insert(at.clone(), (fs::read(&at).unwrap(), meta.modified().unwrap()));
        }
    }
    found
}

// ---- WB-VIEW-23: the paths of a role's view, one to a line ----

#[test]
fn test_WB_VIEW_23_view_of_the_build_prints_the_paths_of_changes_and_the_build_s_reads_one_to_a_line() {
    let bench = bench();
    let run = bench.view(&["54", "build"]);

    ok(&run);
    let out = text(&run.stdout);
    assert_eq!(out, BUILD_LIST);
    assert!(!out.contains("tests/helper.c"), "{}", out);
    let tester = text(&bench.view(&["54", "tester"]).stdout);
    assert_eq!(tester, "src/brake.c\ntests/test_brake.c\nsrc/new.c\nsrc/brake.h\ninclude\ntests/helper.c\n");
    let review = text(&bench.view(&["54", "review"]).stdout);
    assert_eq!(review, "src/brake.c\ntests/test_brake.c\nsrc/new.c\nsrc/brake.h\n");
}

#[test]
fn test_WB_VIEW_23_with_no_flag_the_list_is_flat_and_in_is_not_it() {
    let bench = bench();
    let plain = bench.view(&["54", "build"]);
    let before = bench.view(&["--in", "54", "build"]);

    ok(&plain);
    ok(&before);
    assert_eq!(text(&plain.stdout), BUILD_LIST);
    // `--in` is by folder: at the root it holds folders with counts, not the paths of the list.
    assert_ne!(text(&before.stdout), text(&plain.stdout));
    assert_eq!(text(&before.stdout), "2\tinclude/\n2\tsrc/\n1\ttests/\n");
}

// ---- WB-VIEW-24: no level and no line `view:` ----

#[test]
fn test_WB_VIEW_24_view_prints_the_paths_only_no_level_no_view_line() {
    let bench = bench();
    for (role, list) in [("tester", None), ("build", Some(BUILD_LIST)), ("review", None), ("fix", None)] {
        let run = bench.view(&["54", role]);
        ok(&run);
        let out = text(&run.stdout);
        assert!(!out.contains("level"), "{}: {}", role, out);
        assert!(!out.contains("view:"), "{}: {}", role, out);
        assert!(out.lines().all(|line| !line.contains(": ") && !line.is_empty()), "{}: {}", role, out);
        if let Some(list) = list {
            assert_eq!(out, list);
        }
    }
}

// ---- WB-VIEW-25: `--out [<folder>]`, one level deep ----

#[test]
fn test_WB_VIEW_25_out_at_the_root_prints_the_files_and_the_folders_with_a_count_of_what_is_left_out() {
    let bench = bench();
    let watchbill = bench.git_text(&["ls-tree", "-r", "--name-only", "HEAD", "--", ".watchbill"]).lines().count();
    let run = bench.view(&["54", "build", "--out"]);

    ok(&run);
    // The files left out directly at the root, then each folder with the count of the files left out under it.
    let wanted = format!(".gitignore\n{}\t.watchbill/\n1\tbuild/\n2\tdocs/\n1\tsrc/\n1\ttests/\n", watchbill);
    assert_eq!(text(&run.stdout), wanted);

    // The review reads no `include`: it is left out whole.
    let review = text(&bench.view(&["54", "review", "--out"]).stdout);
    assert!(review.lines().any(|line| line == "2\tinclude/"), "{}", review);
}

#[test]
fn test_WB_VIEW_25_out_of_a_folder_prints_what_is_left_out_in_it_and_a_count_for_each_folder_below() {
    let bench = bench();
    assert_eq!(text(&bench.view(&["54", "build", "--out", "src"]).stdout), "src/secret.c\n");
    assert_eq!(text(&bench.view(&["54", "build", "--out", "tests"]).stdout), "tests/helper.c\n");
    // Files directly in `docs/`, then the folder below it.
    assert_eq!(text(&bench.view(&["54", "build", "--out", "docs"]).stdout), "docs/a.md\n1\tdocs/deep/\n");
    assert_eq!(text(&bench.view(&["54", "build", "--out", "docs/deep"]).stdout), "docs/deep/b.md\n");
    // Nothing of `include` is left out for the build.
    let run = bench.view(&["54", "build", "--out", "include"]);
    ok(&run);
    assert_eq!(text(&run.stdout), "");
    // A flag may come first.
    assert_eq!(text(&bench.view(&["--out", "54", "build"]).stdout), text(&bench.view(&["54", "build", "--out"]).stdout));
}

// ---- WB-VIEW-43: `--in [<folder>]`, one level deep ----

#[test]
fn test_WB_VIEW_43_in_at_the_root_prints_the_folders_that_hold_any_of_the_view_with_a_count() {
    let bench = bench();
    let run = bench.view(&["54", "build", "--in"]);

    ok(&run);
    // `src/new.c` is not in `HEAD`: it is not counted.
    assert_eq!(text(&run.stdout), "2\tinclude/\n2\tsrc/\n1\ttests/\n");
}

#[test]
fn test_WB_VIEW_43_in_a_folder_prints_the_files_of_the_view_in_it_and_a_count_for_each_folder_below() {
    let bench = bench();
    assert_eq!(text(&bench.view(&["54", "build", "--in", "src"]).stdout), "src/brake.c\nsrc/brake.h\n");
    assert_eq!(text(&bench.view(&["54", "build", "--in", "include"]).stdout), "include/a.h\ninclude/b.h\n");
    let tester = text(&bench.view(&["54", "tester", "--in", "tests"]).stdout);
    assert_eq!(tester, "tests/helper.c\ntests/test_brake.c\n");
    // A folder of the view that holds a folder: the count is of every file of the view under it.
    let file = ISSUE_54.replace("changes: [src/brake.c, tests/test_brake.c, src/new.c]", "changes: [src/brake.c, docs]");
    put(&bench.repo.path, ".watchbill/issues/54.yml", &file);
    fs::remove_file(bench.repo.path.join(".watchbill/issues/54.accepted")).unwrap();
    let run = wb(&bench.repo.path, &["accept", "54", "--no-commit"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    git(&bench.repo.path, &["add", "--", ".watchbill/issues"]);
    git(&bench.repo.path, &["commit", "-q", "-m", "A folder in changes"]);
    assert_eq!(text(&bench.view(&["54", "build", "--in", "docs"]).stdout), "docs/a.md\n1\tdocs/deep/\n");
    assert_eq!(text(&bench.view(&["54", "build", "--in"]).stdout), "2\tdocs/\n2\tinclude/\n2\tsrc/\n");
}

#[test]
fn test_WB_VIEW_43_a_folder_is_taken_with_or_without_a_trailing_slash() {
    let bench = bench();
    for flag in ["--in", "--out"] {
        let plain = text(&bench.view(&["54", "build", flag, "src"]).stdout);
        let slash = text(&bench.view(&["54", "build", flag, "src/"]).stdout);
        assert!(!plain.is_empty());
        assert_eq!(plain, slash, "{}", flag);
    }
}

// ---- WB-VIEW-44: names only ----

#[test]
fn test_WB_VIEW_44_a_file_line_is_its_path_alone_with_no_tab_and_no_reason() {
    let bench = bench();
    let out = text(&bench.view(&["54", "build", "--in", "src"]).stdout);
    assert_eq!(out, "src/brake.c\nsrc/brake.h\n");
    assert!(!out.contains('\t'), "{}", out);
    let out = text(&bench.view(&["54", "build", "--out"]).stdout);
    for line in out.lines() {
        match line.split_once('\t') {
            // A folder line: a count, a tab, the folder and a `/`.
            Some((count, folder)) => assert!(count.parse::<usize>().is_ok() && folder.ends_with('/'), "{}", line),
            None => assert_eq!(line, ".gitignore", "{}", out),
        }
    }
}

// ---- WB-VIEW-45: a folder that is not in HEAD ----

#[test]
fn test_WB_VIEW_45_a_folder_not_in_head_ends_with_1_saying_so() {
    let bench = bench();
    for flag in ["--in", "--out"] {
        for folder in ["nothing", "src/new.c/", "src/nothing"] {
            let run = bench.view(&["54", "build", flag, folder]);
            assert_eq!(code(&run), Some(1), "{} {}: {}", flag, folder, text(&run.stderr));
            assert_eq!(text(&run.stdout), "");
            assert!(text(&run.stderr).starts_with("wb view: "), "{}", text(&run.stderr));
            assert!(text(&run.stderr).contains("not in HEAD"), "{}", text(&run.stderr));
        }
    }
}

// ---- WB-VIEW-46: a file given as the folder ----

#[test]
fn test_WB_VIEW_46_a_submodule_given_as_the_folder_is_not_called_a_file() {
    let bench = bench();
    let sha = bench.git_text(&["rev-parse", "HEAD"]);
    git(&bench.repo.path, &["update-index", "--add", "--cacheinfo", &format!("160000,{},vendor/lib", sha.trim())]);
    git(&bench.repo.path, &["commit", "-q", "-m", "A submodule"]);
    for flag in ["--in", "--out"] {
        let run = bench.view(&["54", "build", flag, "vendor/lib"]);
        assert_eq!(code(&run), Some(1), "{}: {}", flag, text(&run.stderr));
        assert!(!text(&run.stderr).contains("is a file"), "{}", text(&run.stderr));
    }
}

#[test]
fn test_WB_VIEW_46_a_file_given_as_the_folder_ends_with_2_naming_the_command_for_a_path() {
    let bench = bench();
    for flag in ["--in", "--out"] {
        let run = bench.view(&["54", "build", flag, "src/brake.c"]);
        assert_eq!(code(&run), Some(2), "{}: {}", flag, text(&run.stderr));
        assert_eq!(text(&run.stdout), "");
        assert!(text(&run.stderr).contains("wb view 54 src/brake.c"), "{}", text(&run.stderr));
    }
}

#[test]
fn test_WB_VIEW_46_a_folder_that_is_not_a_path_from_the_root_ends_with_2() {
    let bench = bench();
    for folder in ["/src", "src/../docs", "../x"] {
        let run = bench.view(&["54", "build", "--in", folder]);
        assert_eq!(code(&run), Some(2), "{}: {}", folder, text(&run.stderr));
        assert_eq!(text(&run.stdout), "");
        assert!(text(&run.stderr).contains("is not a folder from the root of the repository"), "{}: {}", folder, text(&run.stderr));
    }
    // A folder with no flag is a third word that is not known.
    assert_eq!(code(&bench.view(&["54", "build", "src"])), Some(2));
}

// ---- WB-VIEW-26: `--in` and `--out` together, and a command used wrongly ----

#[test]
fn test_WB_VIEW_26_in_and_out_together_end_with_2() {
    let bench = bench();
    for args in [&["54", "build", "--in", "--out"][..], &["54", "build", "--out", "--in"][..]] {
        let run = bench.view(args);
        assert_eq!(code(&run), Some(2), "{:?}", args);
        assert_eq!(text(&run.stdout), "", "{:?}", args);
        assert!(text(&run.stderr).starts_with("wb view: "), "{}", text(&run.stderr));
        assert!(text(&run.stderr).contains("usage:"), "{}", text(&run.stderr));
    }
}

#[test]
fn test_WB_VIEW_26_view_used_wrongly_ends_with_2() {
    let bench = bench();
    for args in [
        &[][..],
        &["54"][..],
        &["54", "build", "extra"][..],
        &["54", "src/brake.c", "--out"][..],
        &["54", "src/brake.c", "--in"][..],
        &["54", "build", "--out", "--out"][..],
        &["54", "build", "--nothing"][..],
        &["../54", "build"][..],
        &["54", "/etc/passwd"][..],
        &["54", "src/../secret"][..],
    ] {
        let run = bench.view(args);
        assert_eq!(code(&run), Some(2), "{:?}: {}", args, text(&run.stderr));
        assert_eq!(text(&run.stdout), "", "{:?}", args);
    }
    // Outside a repository.
    let elsewhere = folder();
    let run = wb(&elsewhere.path, &["view", "54", "build"], "");
    assert_eq!(code(&run), Some(2), "{}", text(&run.stderr));
}

// ---- WB-VIEW-27: a path in the view of a role, with the list and the line ----

#[test]
fn test_WB_VIEW_27_a_path_of_changes_is_in_for_every_role_naming_changes_and_its_line() {
    let bench = bench();
    let run = bench.view(&["54", "src/brake.c"]);

    ok(&run);
    let out = text(&run.stdout);
    let roles: Vec<&str> = out.lines().map(|line| line.split(':').next().unwrap()).collect();
    assert_eq!(roles, ["tester", "build", "review", "fix"], "{}", out);
    for role in ["tester", "build", "review", "fix"] {
        assert_eq!(Bench::line_of(&out, role), format!("{}: in: changes, line 6 of .watchbill/issues/54.yml: src/brake.c", role));
    }
}

#[test]
fn test_WB_VIEW_27_a_path_only_in_the_tester_s_reads_is_in_for_the_tester_and_out_for_the_others() {
    let bench = bench();
    let run = bench.view(&["54", "tests/helper.c"]);

    ok(&run);
    let out = text(&run.stdout);
    assert_eq!(Bench::line_of(&out, "tester"), "tester: in: reads.tester, line 8 of .watchbill/issues/54.yml: tests/helper.c");
    for role in ["build", "review", "fix"] {
        let line = Bench::line_of(&out, role);
        assert!(line.starts_with(&format!("{}: out: ", role)), "{}", line);
        assert!(line.contains("not listed"), "{}", line);
        assert!(line.contains(&format!("reads.{}", role)), "{}", line);
    }
}

#[test]
fn test_WB_VIEW_27_a_file_under_a_folder_of_reads_names_the_folder() {
    let bench = bench();
    let run = bench.view(&["54", "include/a.h"]);

    ok(&run);
    let out = text(&run.stdout);
    assert_eq!(Bench::line_of(&out, "build"), "build: in: reads.build, line 9 of .watchbill/issues/54.yml: include");
    assert!(Bench::line_of(&out, "review").starts_with("review: out: "), "{}", out);
    // The folder itself, written with a `/` or with `./`.
    for path in ["include", "include/", "./include"] {
        let run = bench.view(&["54", path]);
        ok(&run);
        assert!(Bench::line_of(&text(&run.stdout), "build").starts_with("build: in: reads.build, line 9"), "{}", text(&run.stdout));
    }
    // A role's name is the role; the folder `build` is `./build`.
    let run = bench.view(&["54", "./build"]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    assert!(Bench::line_of(&text(&run.stdout), "build").starts_with("build: out: "), "{}", text(&run.stdout));
}

#[test]
fn test_WB_VIEW_27_a_folder_that_only_holds_paths_of_the_view_is_out_and_says_so() {
    let bench = bench();
    let run = bench.view(&["54", "src"]);

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    let line = Bench::line_of(&text(&run.stdout), "build");
    assert!(line.starts_with("build: out: not listed"), "{}", line);
    assert!(line.contains("below it"), "{}", line);
}

// ---- WB-VIEW-28: why a path is out ----

#[test]
fn test_WB_VIEW_28_a_path_out_of_every_view_says_why_for_each_role_and_ends_with_1() {
    let bench = bench();
    put(&bench.repo.path, "src/brake.o", "an object\n");
    for (path, why) in [
        (".watchbill/watchbill.yml", "under .watchbill/"),
        ("build/Makefile", "under `build_files` of watchbill.yml (build)"),
        ("src/brake.o", "ignored by the checkout"),
        ("src/other.c", "not in HEAD"),
        ("src/secret.c", "not listed"),
    ] {
        let run = bench.view(&["54", path]);
        assert_eq!(code(&run), Some(1), "{}: {}", path, text(&run.stderr));
        let out = text(&run.stdout);
        for role in ["tester", "build", "review", "fix"] {
            let line = Bench::line_of(&out, role);
            assert!(line.starts_with(&format!("{}: out: {}", role, why)), "{}: {}", path, line);
        }
    }
}

#[test]
fn test_WB_VIEW_28_a_path_of_changes_not_in_head_is_out_naming_its_list_and_line() {
    let bench = bench();
    let run = bench.view(&["54", "src/new.c"]);

    assert_eq!(code(&run), Some(1));
    let line = Bench::line_of(&text(&run.stdout), "build");
    assert!(line.starts_with("build: out: not in HEAD"), "{}", line);
    assert!(line.contains("changes, line 6 of .watchbill/issues/54.yml"), "{}", line);
}

#[test]
fn test_WB_VIEW_28_a_path_git_does_not_take_is_out() {
    let bench = bench();
    let run = bench.view(&["54", "src/.GIT/config"]);

    assert_eq!(code(&run), Some(1));
    let line = Bench::line_of(&text(&run.stdout), "build");
    assert!(line.starts_with("build: out: a path git does not take"), "{}", line);
}

// ---- WB-VIEW-29: 0 in at least one view, 1 in none ----

#[test]
fn test_WB_VIEW_29_in_one_view_ends_with_0_in_none_with_1() {
    let bench = bench();
    assert_eq!(code(&bench.view(&["54", "tests/helper.c"])), Some(0));
    assert_eq!(code(&bench.view(&["54", "src/brake.h"])), Some(0));
    assert_eq!(code(&bench.view(&["54", "src/secret.c"])), Some(1));
    assert_eq!(code(&bench.view(&["54", "docs"])), Some(1));
}

// ---- WB-VIEW-30: the local changes ----

#[test]
fn test_WB_VIEW_30_a_changed_file_of_the_view_is_not_named_by_view() {
    let bench = bench();
    put(&bench.repo.path, "src/brake.c", "int brake(void) { return 2; }\n");
    put(&bench.repo.path, "src/brake.h", "int brake(int);\n");
    put(&bench.repo.path, "src/secret.c", "changed, and not in the view\n");
    for args in [&["54", "build"][..], &["54", "build", "--in"][..], &["54", "build", "--out"][..], &["54", "build", "--in", "src"][..]] {
        let run = bench.view(args);
        ok(&run);
        let out = text(&run.stdout);
        assert!(!out.contains("local change"), "{:?}: {}", args, out);
        assert!(!out.contains("\n\n"), "{:?}: {}", args, out);
        assert_eq!(text(&run.stderr), "");
    }
    assert_eq!(text(&bench.view(&["54", "build"]).stdout), BUILD_LIST);
}

// ---- WB-PROMPT-60 and 61: the level in the stamp ----

#[test]
fn test_WB_PROMPT_60_the_stamp_of_a_role_in_a_view_holds_the_level_after_the_view_line() {
    let bench = bench();
    for (role, more) in [("tester", &[][..]), ("build", &[][..]), ("review", &["--change", "174"][..]), ("fix", &["--change", "174"][..])] {
        let mut words = vec![role, "54", "--round", "1", "--stamp"];
        words.extend_from_slice(more);
        let stamp = prompt_with(&bench.repo.path, &bench.organisation(), &words);
        ok(&stamp);
        let stamp = text(&stamp.stdout);
        assert_eq!(value_of(&stamp, "level").as_deref(), Some("public"), "{}: {}", role, stamp);
        let lines: Vec<&str> = stamp.lines().collect();
        let at = lines.iter().position(|line| line.starts_with("view: ")).unwrap();
        assert_ne!(value_of(&stamp, "view").as_deref(), Some("none"));
        assert_eq!(lines[at + 1], "level: public", "{}", stamp);
    }
}

#[test]
fn test_WB_PROMPT_60_the_level_is_the_most_restrictive_where_watchbill_yml_has_none() {
    let bench = bench();
    let yml = read(&config(&bench.repo.path)).replace("confidentiality: public\n", "");
    fs::write(config(&bench.repo.path), yml).unwrap();
    commit_config(&bench.repo.path);

    let stamp = prompt_with(&bench.repo.path, &bench.organisation(), &["build", "54", "--round", "1", "--stamp"]);
    ok(&stamp);
    assert_eq!(value_of(&text(&stamp.stdout), "level").as_deref(), Some("internal"), "{}", text(&stamp.stdout));
}

#[test]
fn test_WB_PROMPT_60_the_stamp_takes_runner_as_start_does() {
    let bench = bench();
    let stamp = prompt_with(&bench.repo.path, &bench.organisation(), &["build", "54", "--round", "1", "--runner", "stand-in", "--stamp"]);
    ok(&stamp);
    let stamp = text(&stamp.stdout);
    assert_eq!(value_of(&stamp, "runner").as_deref(), Some("stand-in"), "{}", stamp);
    assert_eq!(value_of(&stamp, "level").as_deref(), Some("public"), "{}", stamp);
}

#[test]
fn test_WB_PROMPT_62_a_level_that_is_not_the_organisation_s_is_its_most_restrictive_and_the_reason_is_said() {
    let bench = bench();
    let yml = read(&config(&bench.repo.path)).replace("confidentiality: public\n", "confidentiality: secret\n");
    fs::write(config(&bench.repo.path), yml).unwrap();
    commit_config(&bench.repo.path);

    let stamp = prompt_with(&bench.repo.path, &bench.organisation(), &["build", "54", "--round", "1", "--stamp"]);
    ok(&stamp);
    assert_eq!(value_of(&text(&stamp.stdout), "level").as_deref(), Some("internal"), "{}", text(&stamp.stdout));
    let said = text(&stamp.stderr);
    assert!(said.contains("the level of the stamp is internal") && said.contains("secret"), "{}", said);

    // The key `levels` is the organisation's: the same.
    let yml = read(&config(&bench.repo.path)).replace("confidentiality: secret\n", "confidentiality: public\nlevels: [public]\n");
    fs::write(config(&bench.repo.path), yml).unwrap();
    commit_config(&bench.repo.path);
    let stamp = prompt_with(&bench.repo.path, &bench.organisation(), &["build", "54", "--round", "1", "--stamp"]);
    ok(&stamp);
    assert_eq!(value_of(&text(&stamp.stdout), "level").as_deref(), Some("internal"), "{}", text(&stamp.stdout));
    assert!(text(&stamp.stderr).contains("organisation's"), "{}", text(&stamp.stderr));
}

#[test]
fn test_WB_PROMPT_63_with_no_organisation_s_file_and_no_level_the_stamp_says_not_known_and_nothing_else() {
    let bench = bench();
    let yml = read(&config(&bench.repo.path)).replace("confidentiality: public\n", "");
    fs::write(config(&bench.repo.path), yml).unwrap();
    commit_config(&bench.repo.path);

    let none = bench.org.path.join("not-there.yml");
    let stamp = prompt_with(&bench.repo.path, &none, &["build", "54", "--round", "1", "--stamp"]);
    ok(&stamp);
    assert_eq!(value_of(&text(&stamp.stdout), "level").as_deref(), Some("not known"), "{}", text(&stamp.stdout));
    assert_eq!(text(&stamp.stderr), "");
    // With the line `confidentiality:` and no organisation's file, the line is the level.
    let yml = read(&config(&bench.repo.path)) + "confidentiality: public\n";
    fs::write(config(&bench.repo.path), yml).unwrap();
    commit_config(&bench.repo.path);
    let stamp = prompt_with(&bench.repo.path, &none, &["build", "54", "--round", "1", "--stamp"]);
    ok(&stamp);
    assert_eq!(value_of(&text(&stamp.stdout), "level").as_deref(), Some("public"), "{}", text(&stamp.stdout));
}

#[test]
fn test_WB_PROMPT_72_the_stamp_of_a_session_in_a_view_of_the_whole_tree_holds_the_level_after_the_view_line() {
    let bench = bench();
    // A review of a change with no issue and a role with no issue's file run in a view of every file of
    // HEAD (#202), and their stamps say the level as every other does.
    for words in [
        &["review", "--change", "174", "--stamp"][..],
        &["build", "99", "--branch", "main", "--round", "1", "--stamp"][..],
    ] {
        let stamp = prompt_with(&bench.repo.path, &bench.organisation(), words);
        ok(&stamp);
        let stamp = text(&stamp.stdout);
        assert_eq!(value_of(&stamp, "view").as_deref(), Some("none"), "{:?}: {}", words, stamp);
        assert_eq!(value_of(&stamp, "level").as_deref(), Some("public"), "{:?}: {}", words, stamp);
        let names: Vec<&str> = stamp.lines().map(|line| line.split(':').next().unwrap()).collect();
        let view = names.iter().position(|name| *name == "view").unwrap();
        assert_eq!(names[view + 1], "level", "{:?}: {}", words, stamp);
    }
}

// ---- WB-VIEW-31: it writes nothing, runs no pattern, asks nothing of the network ----

#[test]
fn test_WB_VIEW_31_view_writes_nothing_in_the_repository_or_the_state_folder_and_runs_no_pattern() {
    let bench = bench();
    put(&bench.repo.path, "src/brake.c", "int brake(void) { return 2; }\n");
    put(&bench.repo.path, "src/brake.o", "an object\n");
    let state = folder_named("state");
    let before = snapshot(&bench.repo.path);
    for args in [
        &["view", "54", "build"][..],
        &["view", "54", "review", "--out"][..],
        &["view", "54", "src/brake.c"][..],
        &["view", "54", "src/brake.o"][..],
        &["view", "54", "breakdown"][..],
        &["view", "99", "build"][..],
    ] {
        let run = Command::new(env!("CARGO_BIN_EXE_wb"))
            .args(args)
            .current_dir(&bench.repo.path)
            .env_clear()
            .envs(env())
            .env("XDG_STATE_HOME", &state.path)
            .output()
            .unwrap();
        assert!(matches!(code(&run), Some(0) | Some(1)), "{:?}: {}", args, text(&run.stderr));
    }
    assert_eq!(snapshot(&bench.repo.path), before);
    assert_eq!(fs::read_dir(&state.path).unwrap().count(), 0);
    assert_eq!(bench.left("calls"), "");
    assert_eq!(bench.left("network"), "");
}

// ---- WB-VIEW-85 to 87: no accepted, committed file, and the breakdown (#202) ----

#[test]
fn test_WB_VIEW_86_an_issue_with_no_file_ends_with_1_saying_the_role_would_run_in_a_view_of_the_whole_tree() {
    let bench = bench();
    for args in [&["99", "build"][..], &["99", "src/brake.c"][..]] {
        let run = bench.view(args);
        assert_eq!(code(&run), Some(1), "{:?}", args);
        assert_eq!(text(&run.stdout), "", "{:?}", args);
        let err = text(&run.stderr);
        assert!(err.contains("a view of every file of HEAD") && err.contains(".watchbill/ read-only"), "{}", err);
        assert!(!err.contains("without a view"), "{}", err);
    }
}

#[test]
fn test_WB_VIEW_87_an_issue_s_file_not_accepted_or_not_committed_ends_with_1() {
    let bench = bench();
    // Not committed: a new issue's file, accepted and left in the working tree.
    put(&bench.repo.path, ".watchbill/issues/55.yml", &ISSUE_54.replace("'54'", "'55'"));
    let run = wb(&bench.repo.path, &["accept", "55", "--no-commit"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let run = bench.view(&["55", "build"]);
    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stderr).contains("is not committed in HEAD"), "{}", text(&run.stderr));
    // Not accepted: the file changed after its acceptance.
    put(&bench.repo.path, ".watchbill/issues/54.yml", &format!("{}# changed\n", ISSUE_54));
    let run = bench.view(&["54", "build"]);
    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stderr).contains("is not accepted"), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "");
}

#[test]
fn test_WB_VIEW_85_the_breakdown_runs_in_a_writable_view_of_the_whole_tree_with_watchbill_read_only_but_its_issues_folder() {
    let bench = bench();
    let run = bench.view(&["54", "breakdown"]);

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    let err = text(&run.stderr);
    assert!(err.contains("a view of every file of HEAD, writable,") && err.contains(".watchbill/ read-only but for .watchbill/issues/"), "{}", err);
    assert!(!err.contains("only .watchbill/issues/ is writable"), "{}", err);
    assert!(!err.contains("without a view"), "{}", err);
}
