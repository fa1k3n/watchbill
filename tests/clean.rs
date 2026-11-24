//! `wb view --kept` and `wb clean` (#176): what a kept view holds is listed, and the views whose work is all taken
//! back are removed, after they are said.
//!
//! A kept view is made as `wb start` makes it (`watchbill::view::make`), with the lines of its session beside it;
//! no session is run, so no bubblewrap is needed. A view that git cannot read is a FIFO in the place of its head
//! object, and `wb` runs as a child under a bound (`bounded`).

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};

const GIT_USER: [&str; 6] = ["-c", "user.name=Ada Lovelace", "-c", "user.email=ada@example.org", "-c", "commit.gpgsign=false"];

struct Bench {
    repo: Folder,
    state: Folder,
}

/// A repository on the branch `main` with one commit, and an empty state folder.
fn bench() -> Bench {
    let repo = repository(None);
    git(&repo.path, &["config", "user.name", "Ada Lovelace"]);
    git(&repo.path, &["config", "user.email", "ada@example.org"]);
    git(&repo.path, &["config", "commit.gpgsign", "false"]);
    put(&repo.path, "src/brake.c", "int brake(void) { return 1; }\n");
    git(&repo.path, &["add", "--", "src/brake.c"]);
    git(&repo.path, &["commit", "-q", "-m", "The code"]);
    git(&repo.path, &["branch", "-M", "main"]);
    Bench { repo, state: folder_named("state") }
}

const RAN: &str = "outcome: ended\nstatus: 0\nseconds: 5\n";

/// What a kept view is made with in a test.
struct Keep<'a> {
    session: &'a str,
    round: u32,
    /// The subjects of the commits made in the view.
    commits: &'a [&'a str],
    /// The paths left uncommitted in the view.
    uncommitted: &'a [&'a str],
    outcome: &'a str,
    /// The text of `back` from the view's commits (`$1`, `$2`, ... are their names, then `$FOLDER` the view), or none.
    back: Option<&'a str>,
}

impl<'a> Keep<'a> {
    fn new(session: &'a str) -> Keep<'a> {
        Keep { session, round: 1, commits: &[], uncommitted: &[], outcome: RAN, back: None }
    }
}

fn role_of(session: &str) -> &str {
    session.split("Z-").nth(1).unwrap().split('-').next().unwrap()
}

fn issue_of(session: &str) -> &str {
    session.rsplit('-').next().unwrap()
}

fn commit_in(view: &Path, path: &str, subject: &str) -> String {
    put(view, path, "int made;\n");
    git(view, &["add", "--", path]);
    let mut args: Vec<&str> = GIT_USER.to_vec();
    args.extend(["-c", "maintenance.auto=false", "commit", "-q", "-m", subject]);
    git(view, &args);
    head_of(view)
}

fn head_of(folder: &Path) -> String {
    let run = Command::new("git").arg("-C").arg(folder).args(["rev-parse", "HEAD"]).env_clear().envs(env()).output().unwrap();
    text(&run.stdout).trim().to_string()
}

/// A view of `repo` made as `wb` makes it, with what `keep` says in it, and the folder of the lines of its session.
/// The commits are named in `back` by the words `{commit0}`, `{commit1}`, `{view}` and `{head}` (the checkout's).
fn keep_in(repo: &Path, state: &Path, keep: &Keep) -> (PathBuf, PathBuf) {
    let root = fs::canonicalize(repo).unwrap();
    let below = state.join("watchbill").join("views");
    let made = watchbill::view::make(root.to_str().unwrap(), &["src/brake.c".to_string()], &below, keep.session).unwrap();
    let mut commits = Vec::new();
    for (number, subject) in keep.commits.iter().enumerate() {
        commits.push(commit_in(&made.folder, &format!("made-{}.c", number), subject));
    }
    for path in keep.uncommitted {
        put(&made.folder, path, "left\n");
    }
    let session = state.join("watchbill").join("sessions").join(keep.session);
    fs::create_dir_all(&session).unwrap();
    fs::write(session.join("stamp"), format!("[stamp] {} round {}\n", role_of(keep.session), keep.round)).unwrap();
    fs::write(session.join("outcome"), keep.outcome).unwrap();
    fs::write(
        session.join("folder"),
        format!("folder: {}\nbase: {}\nhead: {}\nbranch: {}\nissue: {}\n", made.folder.display(), made.base, made.head, made.branch, issue_of(keep.session)),
    )
    .unwrap();
    if let Some(back) = keep.back {
        let mut back = back.replace("{view}", &made.folder.display().to_string()).replace("{head}", &made.head);
        for (number, commit) in commits.iter().enumerate() {
            back = back.replace(&format!("{{commit{}}}", number), commit);
        }
        fs::write(session.join("back"), back).unwrap();
    }
    (made.folder, session)
}

fn keep(bench: &Bench, keep: &Keep) -> (PathBuf, PathBuf) {
    keep_in(&bench.repo.path, &bench.state.path, keep)
}

/// A kept view whose one commit was all taken back (`took:`), with nothing uncommitted: nothing is left in it.
fn done(bench: &Bench, session: &str) -> (PathBuf, PathBuf) {
    keep(
        bench,
        &Keep { commits: &["Taken"], back: Some("taken: {commit0} Taken\ntook: {commit0} {head}\nview: kept {view}\n"), ..Keep::new(session) },
    )
}

/// `wb` as a person runs it in the repository of the bench, with the state folder of the bench.
fn wb_in(bench: &Bench, args: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wb"));
    command.args(args).current_dir(&bench.repo.path).env_clear().envs(env()).env("XDG_STATE_HOME", &bench.state.path).stdin(Stdio::null());
    command.output().unwrap()
}

/// The same, with a bound: the child is killed when the time is up, and the result is none.
fn wb_bounded(bench: &Bench, args: &[&str], seconds: u64) -> Option<Output> {
    use std::io::Read;
    let began = std::time::Instant::now();
    let mut child: Child = Command::new(env!("CARGO_BIN_EXE_wb"))
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
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            let (stdout, stderr) = reading.join().unwrap();
            return Some(Output { status, stdout, stderr });
        }
        if began.elapsed() > std::time::Duration::from_secs(seconds) {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// A FIFO in the place of the loose object of the view's newest commit: git blocks for ever opening it.
fn fifo_for_head(view: &Path) {
    let head = head_of(view);
    let object = view.join(".git/objects").join(&head[..2]).join(&head[2..]);
    fs::remove_file(&object).unwrap();
    assert!(Command::new("mkfifo").arg(&object).status().unwrap().success());
}

fn out_of(run: &Output) -> String {
    text(&run.stdout)
}

fn err_of(run: &Output) -> String {
    String::from_utf8_lossy(&run.stderr).to_string()
}

/// Every plain file below a folder with its bytes, and every other entry by its kind: what a command left.
fn tree(folder: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut open = vec![folder.to_path_buf()];
    while let Some(at) = open.pop() {
        let Ok(meta) = fs::symlink_metadata(&at) else { continue };
        let name = at.strip_prefix(folder).unwrap().display().to_string();
        if meta.is_dir() {
            found.push((name, "dir".to_string()));
            if let Ok(entries) = fs::read_dir(&at) {
                open.extend(entries.filter_map(|entry| entry.ok()).map(|entry| entry.path()));
            }
        } else if meta.is_file() {
            found.push((name, String::from_utf8_lossy(&fs::read(&at).unwrap()).to_string()));
        } else {
            found.push((name, "other".to_string()));
        }
    }
    found.sort();
    found
}

fn git_out(folder: &Path, args: &[&str]) -> String {
    let run = Command::new("git").arg("-C").arg(folder).args(args).env_clear().envs(env()).output().unwrap();
    text(&run.stdout)
}

/// The lines of the listing of one view: the line that opens with its folder, and the indented lines below it.
fn block<'a>(out: &'a str, folder: &Path) -> Vec<&'a str> {
    let start = format!("{}: ", folder.display());
    let mut lines = out.lines().skip_while(|line| !line.starts_with(&start));
    let mut found: Vec<&str> = lines.next().into_iter().collect();
    found.extend(lines.take_while(|line| line.starts_with("  ")));
    found
}

// ---- WB-VIEW-125: `wb view --kept` lists a kept view with its role, round, issue and why, and its commits ----

#[test]
fn test_WB_VIEW_125_view_kept_lists_a_kept_view_with_its_role_round_issue_and_a_commit_not_taken() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { round: 2, commits: &["The tests of 54"], ..Keep::new("2026-11-22T100000Z-tester-54") });
    let run = wb_in(&bench, &["view", "--kept"]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    let out = out_of(&run);
    let lines = block(&out, &folder);
    assert_eq!(lines.first().copied(), Some(format!("{}: tester round 2, issue 54", folder.display()).as_str()), "{}", out);
    assert!(lines.iter().any(|line| line.starts_with("  why: ")), "{}", out);
    let commit = lines.iter().find(|line| line.starts_with("  commit ")).unwrap_or_else(|| panic!("{}", out));
    assert!(commit.ends_with(" The tests of 54"), "{}", out);
    assert_eq!(err_of(&run), "");
}

#[test]
fn test_WB_VIEW_125_the_why_comes_from_back_and_a_commit_that_was_refused_is_named() {
    let bench = bench();
    let (folder, _) = keep(
        &bench,
        &Keep { commits: &["Refused one"], back: Some("refused: {commit0} Refused one: it changes a path that the issue does not name\nview: kept {view}\n"), ..Keep::new("2026-11-22T100000Z-build-54") },
    );
    let out = out_of(&wb_in(&bench, &["view", "--kept"]));
    let lines = block(&out, &folder);
    assert!(lines.iter().any(|line| line.starts_with("  why: ") && line.contains("refused") && line.contains("does not name")), "{}", out);
    assert!(lines.iter().any(|line| line.starts_with("  commit ") && line.contains(" Refused one")), "{}", out);
    // Without a `back` the session left none, and that is the reason.
    let (other, _) = keep(&bench, &Keep { commits: &["Alone"], ..Keep::new("2026-11-22T110000Z-tester-55") });
    let out = out_of(&wb_in(&bench, &["view", "--kept"]));
    assert!(block(&out, &other).iter().any(|line| line.starts_with("  why: ") && line.contains("no `back`")), "{}", out);
}

#[test]
fn test_WB_VIEW_125_with_nothing_kept_nothing_is_printed_and_it_ends_with_0() {
    let bench = bench();
    let run = wb_in(&bench, &["view", "--kept"]);
    assert_eq!((code(&run), out_of(&run), err_of(&run)), (Some(0), String::new(), String::new()));
}

// ---- WB-VIEW-126: the paths left, the views with nothing left, a view of another repository, a view not read ----

#[test]
fn test_WB_VIEW_126_paths_left_uncommitted_are_listed_one_to_a_line() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { uncommitted: &["left.txt", "other.txt"], ..Keep::new("2026-11-22T100000Z-build-54") });
    let out = out_of(&wb_in(&bench, &["view", "--kept"]));
    let lines = block(&out, &folder);
    assert!(lines.contains(&"  uncommitted left.txt") && lines.contains(&"  uncommitted other.txt"), "{}", out);
}

#[test]
fn test_WB_VIEW_126_a_view_whose_commits_were_all_taken_is_listed_as_nothing_left() {
    let bench = bench();
    let (folder, _) = done(&bench, "2026-11-22T100000Z-build-54");
    let out = out_of(&wb_in(&bench, &["view", "--kept"]));
    let lines = block(&out, &folder);
    assert!(lines.first().is_some(), "{}", out);
    assert!(lines.iter().any(|line| line.starts_with("  why: ") && line.contains("nothing is left")), "{}", out);
    assert!(!lines.iter().any(|line| line.starts_with("  commit ") || line.starts_with("  uncommitted ")), "{}", out);
}

#[test]
fn test_WB_VIEW_126_a_view_of_another_repository_is_not_listed() {
    let bench = bench();
    let (mine, _) = keep(&bench, &Keep { commits: &["Mine"], ..Keep::new("2026-11-22T100000Z-tester-54") });
    // Another repository, with a commit of its own, on the same machine.
    let other = repository_named("other", None);
    git(&other.path, &["config", "gc.auto", "0"]);
    put(&other.path, "src/brake.c", "int other(void) { return 7; }\n");
    git(&other.path, &["add", "--", "src/brake.c"]);
    let mut args: Vec<&str> = GIT_USER.to_vec();
    args.extend(["commit", "-q", "-m", "The other code, which is not the code of the first"]);
    git(&other.path, &args);
    git(&other.path, &["branch", "-M", "main"]);
    let (theirs, _) = keep_in(&other.path, &bench.state.path, &Keep { commits: &["Theirs"], ..Keep::new("2026-11-22T120000Z-tester-77") });
    let out = out_of(&wb_in(&bench, &["view", "--kept"]));
    assert!(!block(&out, &mine).is_empty(), "{}", out);
    assert!(block(&out, &theirs).is_empty() && !out.contains("Theirs"), "{}", out);
    // And `wb clean` does not touch it.
    let _ = wb_in(&bench, &["clean", &theirs.file_name().unwrap().to_string_lossy()]);
    assert!(theirs.exists());
}

#[test]
fn test_WB_VIEW_126_a_view_that_cannot_be_read_is_listed_as_not_read_within_a_bound() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { commits: &["Tests"], ..Keep::new("2026-11-22T100000Z-tester-54") });
    fifo_for_head(&folder);
    let run = wb_bounded(&bench, &["view", "--kept"], 15).expect("wb view --kept did not end in 15 seconds");
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    let out = out_of(&run);
    assert!(block(&out, &folder).iter().any(|line| line.starts_with("  not read: ")), "{}", out);
}

// ---- WB-VIEW-127: `wb view --kept` writes nothing and takes no other word ----

#[test]
fn test_WB_VIEW_127_view_kept_writes_nothing() {
    let bench = bench();
    keep(&bench, &Keep { commits: &["Tests"], uncommitted: &["a.txt"], ..Keep::new("2026-11-22T100000Z-tester-54") });
    done(&bench, "2026-11-22T110000Z-build-54");
    let (state, repo) = (tree(&bench.state.path), tree(&bench.repo.path));
    let refs = git_out(&bench.repo.path, &["for-each-ref"]);
    let run = wb_in(&bench, &["view", "--kept"]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert_eq!(tree(&bench.state.path), state);
    assert_eq!(tree(&bench.repo.path), repo);
    assert_eq!(git_out(&bench.repo.path, &["for-each-ref"]), refs);
}

#[test]
fn test_WB_VIEW_127_view_kept_takes_no_other_word_and_ends_with_2() {
    let bench = bench();
    for args in [&["view", "--kept", "54"][..], &["view", "54", "tester", "--kept"][..], &["view", "--kept", "--in"][..], &["view", "--kept", "--kept"][..], &["view", "--kept", "--out", "x"][..]] {
        let run = wb_in(&bench, args);
        assert_eq!(code(&run), Some(2), "{:?}: {}", args, err_of(&run));
        assert_eq!(out_of(&run), "", "{:?}", args);
        assert!(err_of(&run).starts_with("wb view: "), "{:?}: {}", args, err_of(&run));
    }
}

#[test]
fn test_WB_VIEW_127_outside_a_repository_it_ends_with_2() {
    let folder = folder();
    let run = wb(&folder.path, &["view", "--kept"], "");
    assert_eq!(code(&run), Some(2), "{}", err_of(&run));
    assert!(err_of(&run).contains("not in a git repository"), "{}", err_of(&run));
}

// ---- WB-VIEW-128: `wb clean` removes a view whose work was all taken, and says so first ----

#[test]
fn test_WB_VIEW_128_clean_removes_a_view_whose_work_was_all_taken_back_and_says_what_it_removes() {
    let bench = bench();
    let (folder, _) = done(&bench, "2026-11-22T100000Z-build-54");
    let run = wb_in(&bench, &["clean"]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert_eq!(out_of(&run), format!("Removing {}\n", folder.display()));
    assert!(!folder.exists());
    assert_eq!(err_of(&run), "");
}

#[test]
fn test_WB_VIEW_128_every_view_that_is_removed_is_said_on_a_line_of_its_own() {
    let bench = bench();
    let (first, _) = done(&bench, "2026-11-22T100000Z-build-54");
    let (second, _) = done(&bench, "2026-11-22T110000Z-tester-55");
    let run = wb_in(&bench, &["clean"]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    let out = out_of(&run);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2, "{}", out);
    assert!(lines.contains(&format!("Removing {}", first.display()).as_str()) && lines.contains(&format!("Removing {}", second.display()).as_str()), "{}", out);
    assert!(!first.exists() && !second.exists());
}

#[test]
fn test_WB_VIEW_128_a_view_with_no_commit_and_nothing_uncommitted_is_removed() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep::new("2026-11-22T100000Z-tester-54"));
    let run = wb_in(&bench, &["clean"]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(!folder.exists());
}

#[test]
fn test_WB_VIEW_128_with_nothing_to_remove_it_says_nothing_and_ends_with_0() {
    let bench = bench();
    let run = wb_in(&bench, &["clean"]);
    assert_eq!((code(&run), out_of(&run), err_of(&run)), (Some(0), String::new(), String::new()));
}

#[test]
fn test_WB_VIEW_128_a_view_that_a_session_left_without_write_permission_is_removed() {
    use std::os::unix::fs::PermissionsExt;
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep::new("2026-11-22T100000Z-tester-54"));
    put(&folder, "locked/inside.txt", "x\n");
    fs::set_permissions(folder.join("locked"), fs::Permissions::from_mode(0o500)).unwrap();
    let run = wb_in(&bench, &["clean"]);
    // The untracked file is a path left uncommitted: it is kept. Named, it is removed, locked folder and all.
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(folder.exists(), "{}", out_of(&run));
    let name = folder.file_name().unwrap().to_string_lossy().to_string();
    let run = wb_in(&bench, &["clean", &name]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(!folder.exists());
}

// ---- WB-VIEW-129: work left keeps a view, unless it is named ----

#[test]
fn test_WB_VIEW_129_a_view_with_a_commit_not_taken_is_kept_and_said_so() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { commits: &["Not taken"], ..Keep::new("2026-11-22T100000Z-tester-54") });
    let run = wb_in(&bench, &["clean"]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(folder.exists());
    let out = out_of(&run);
    assert!(out.starts_with(&format!("Keeping {}: ", folder.display())) && out.contains("1 commit not taken"), "{}", out);
    assert!(!out.contains("Removing"), "{}", out);
}

#[test]
fn test_WB_VIEW_129_a_view_with_a_path_uncommitted_is_kept_and_said_so() {
    let bench = bench();
    let (folder, _) = done(&bench, "2026-11-22T100000Z-build-54");
    put(&folder, "left.txt", "uncommitted\n");
    let run = wb_in(&bench, &["clean"]);
    assert!(folder.exists());
    let out = out_of(&run);
    assert!(out.starts_with(&format!("Keeping {}: ", folder.display())) && out.contains("1 path uncommitted"), "{}", out);
}

#[test]
fn test_WB_VIEW_129_a_view_kept_beside_one_that_is_done_is_kept_and_the_other_removed() {
    let bench = bench();
    let (kept, _) = keep(&bench, &Keep { commits: &["Not taken"], ..Keep::new("2026-11-22T100000Z-tester-54") });
    let (gone, _) = done(&bench, "2026-11-22T110000Z-build-54");
    let run = wb_in(&bench, &["clean"]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(kept.exists() && !gone.exists());
}

#[test]
fn test_WB_VIEW_129_a_view_named_is_removed_with_its_work_and_the_work_is_said_first() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { commits: &["Not taken"], uncommitted: &["left.txt"], ..Keep::new("2026-11-22T100000Z-tester-54") });
    let name = folder.file_name().unwrap().to_string_lossy().to_string();
    let run = wb_in(&bench, &["clean", &name]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    let out = out_of(&run);
    assert!(out.starts_with(&format!("Removing {}", folder.display())), "{}", out);
    assert!(out.contains("1 commit not taken") && out.contains("1 path uncommitted"), "{}", out);
    assert!(!folder.exists());
}

#[test]
fn test_WB_VIEW_129_a_view_named_by_its_whole_path_is_removed_and_the_others_are_not() {
    let bench = bench();
    let (named, _) = keep(&bench, &Keep { commits: &["One"], ..Keep::new("2026-11-22T100000Z-tester-54") });
    let (other, _) = keep(&bench, &Keep { commits: &["Two"], ..Keep::new("2026-11-22T110000Z-tester-55") });
    let run = wb_in(&bench, &["clean", named.to_str().unwrap()]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(!named.exists() && other.exists());
}

// ---- WB-VIEW-130: --dry-run and -n remove nothing ----

#[test]
fn test_WB_VIEW_130_dry_run_and_n_remove_nothing_and_say_would_remove() {
    for flag in ["--dry-run", "-n"] {
        let bench = bench();
        let (gone, _) = done(&bench, "2026-11-22T100000Z-build-54");
        let (kept, _) = keep(&bench, &Keep { commits: &["Not taken"], ..Keep::new("2026-11-22T110000Z-tester-54") });
        let before = tree(&bench.state.path);
        let run = wb_in(&bench, &["clean", flag]);
        assert_eq!(code(&run), Some(0), "{}: {}", flag, err_of(&run));
        let out = out_of(&run);
        assert!(out.lines().any(|line| line == format!("Would remove {}", gone.display())), "{}: {}", flag, out);
        assert!(out.lines().any(|line| line.starts_with(&format!("Keeping {}: ", kept.display()))), "{}: {}", flag, out);
        assert!(!out.contains("Removing"), "{}: {}", flag, out);
        assert_eq!(tree(&bench.state.path), before, "{}", flag);
    }
}

#[test]
fn test_WB_VIEW_130_dry_run_with_a_name_says_what_the_view_holds_and_removes_nothing() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { commits: &["Not taken"], ..Keep::new("2026-11-22T100000Z-tester-54") });
    let name = folder.file_name().unwrap().to_string_lossy().to_string();
    let run = wb_in(&bench, &["clean", "-n", &name]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(out_of(&run).starts_with(&format!("Would remove {}", folder.display())) && out_of(&run).contains("1 commit not taken"), "{}", out_of(&run));
    assert!(folder.exists());
}

// ---- WB-VIEW-131: it never removes the checkout, a branch or a session's lines ----

#[test]
fn test_WB_VIEW_131_it_removes_no_checkout_no_branch_and_no_lines_of_a_session() {
    let bench = bench();
    git(&bench.repo.path, &["branch", "issue-54"]);
    let (gone, session) = done(&bench, "2026-11-22T100000Z-build-54");
    let (kept, kept_session) = keep(&bench, &Keep { commits: &["Not taken"], ..Keep::new("2026-11-22T110000Z-tester-54") });
    let refs = git_out(&bench.repo.path, &["for-each-ref"]);
    let (lines, kept_lines) = (tree(&session), tree(&kept_session));
    let checkout: Vec<(String, String)> = tree(&bench.repo.path).into_iter().filter(|(name, _)| !name.starts_with(".git")).collect();
    for args in [&["clean"][..], &["clean", kept.file_name().unwrap().to_str().unwrap()][..]] {
        let run = wb_in(&bench, args);
        assert_eq!(code(&run), Some(0), "{:?}: {}", args, err_of(&run));
    }
    assert!(!gone.exists() && !kept.exists());
    assert_eq!(git_out(&bench.repo.path, &["for-each-ref"]), refs);
    assert_eq!(tree(&session), lines);
    assert_eq!(tree(&kept_session), kept_lines);
    let after: Vec<(String, String)> = tree(&bench.repo.path).into_iter().filter(|(name, _)| !name.starts_with(".git")).collect();
    assert_eq!(after, checkout);
    // The state folder and the folder of the views stay, and so does a folder of the state that is no view.
    assert!(bench.state.path.join("watchbill/views").is_dir());
}

#[test]
fn test_WB_VIEW_131_a_name_that_is_not_a_view_of_the_listing_removes_nothing_and_ends_with_1() {
    let bench = bench();
    let (_, session) = done(&bench, "2026-11-22T100000Z-build-54");
    let sessions = bench.state.path.join("watchbill/sessions");
    symlink(&bench.repo.path, bench.state.path.join("watchbill/views/linked")).unwrap();
    let before = (tree(&bench.state.path.join("watchbill/sessions")), tree(&bench.repo.path));
    let names = [
        "..".to_string(),
        ".".to_string(),
        "linked".to_string(),
        "nothing".to_string(),
        bench.repo.path.display().to_string(),
        sessions.display().to_string(),
        session.display().to_string(),
        format!("{}/watchbill/views/../sessions", bench.state.path.display()),
        format!("{}/watchbill/views/linked", bench.state.path.display()),
        String::new(),
    ];
    for name in &names {
        let run = wb_in(&bench, &["clean", name]);
        assert_eq!(code(&run), Some(1), "{:?}: {}", name, err_of(&run));
        assert_eq!(out_of(&run), "", "{:?}", name);
        assert!(err_of(&run).starts_with("wb clean: limit: "), "{:?}: {}", name, err_of(&run));
    }
    assert_eq!((tree(&bench.state.path.join("watchbill/sessions")), tree(&bench.repo.path)), before);
    assert!(bench.repo.path.join("src/brake.c").exists());
}

// ---- WB-VIEW-132: a view not read, and a view of a session that runs, are never removed ----

#[test]
fn test_WB_VIEW_132_a_view_that_cannot_be_read_is_not_removed_named_or_not_and_wb_does_not_wait() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { commits: &["Tests"], ..Keep::new("2026-11-22T100000Z-tester-54") });
    fifo_for_head(&folder);
    let run = wb_bounded(&bench, &["clean"], 15).expect("wb clean did not end in 15 seconds");
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(folder.exists());
    assert!(out_of(&run).starts_with(&format!("Keeping {}: not read: ", folder.display())), "{}", out_of(&run));
    let name = folder.file_name().unwrap().to_string_lossy().to_string();
    let run = wb_bounded(&bench, &["clean", &name], 15).expect("wb clean <view> did not end in 15 seconds");
    assert_eq!(code(&run), Some(1), "{}", err_of(&run));
    assert!(folder.exists());
    assert!(err_of(&run).contains("not read"), "{}", err_of(&run));
}

/// A process that is a `wb` to `wb` (`wb`, by the name of its program) and waits: a copy of `sleep`.
fn running_wb() -> (Folder, Child) {
    let place = folder_named("running");
    let program = place.path.join("wb");
    fs::copy("/usr/bin/sleep", &program).or_else(|_| fs::copy("/bin/sleep", &program)).unwrap();
    let child = Command::new(&program).arg("60").spawn().unwrap();
    (place, child)
}

#[test]
fn test_WB_VIEW_132_a_view_of_a_session_that_runs_is_not_removed_named_or_not() {
    let bench = bench();
    let (_place, mut child) = running_wb();
    let outcome = format!("outcome: running\npid: {}\nstarted: {}\n", child.id(), watchbill::kept::start_time(child.id()).unwrap());
    let (folder, _) = keep(&bench, &Keep { outcome: &outcome, ..Keep::new("2026-11-22T100000Z-tester-54") });
    let name = folder.file_name().unwrap().to_string_lossy().to_string();
    let plain = wb_in(&bench, &["clean"]);
    let named = wb_in(&bench, &["clean", &name]);
    let kept = wb_in(&bench, &["view", "--kept"]);
    let _ = child.kill();
    let _ = child.wait();
    assert_eq!(code(&plain), Some(0), "{}", err_of(&plain));
    assert_eq!(out_of(&plain), "");
    assert_eq!(code(&named), Some(1), "{}", err_of(&named));
    assert!(err_of(&named).contains("runs"), "{}", err_of(&named));
    assert!(folder.exists());
    // Its view is the session's own, and is not listed.
    assert_eq!(out_of(&kept), "");
}

#[test]
fn test_WB_VIEW_132_a_view_whose_session_was_cut_off_is_kept_unless_it_is_named() {
    let bench = bench();
    let (folder, _) = keep(&bench, &Keep { outcome: "outcome: running\npid: 1\n", ..Keep::new("2026-11-22T100000Z-tester-54") });
    let run = wb_in(&bench, &["clean"]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(folder.exists());
    assert!(out_of(&run).starts_with(&format!("Keeping {}: ", folder.display())) && out_of(&run).contains("cut off"), "{}", out_of(&run));
    let name = folder.file_name().unwrap().to_string_lossy().to_string();
    let run = wb_in(&bench, &["clean", &name]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(!folder.exists());
}

// ---- WB-VIEW-133: usage ----

#[test]
fn test_WB_VIEW_133_a_word_that_is_not_known_is_used_wrongly_and_ends_with_2() {
    let bench = bench();
    let (folder, _) = done(&bench, "2026-11-22T100000Z-build-54");
    for args in [
        &["clean", "--force"][..],
        &["clean", "-f"][..],
        &["clean", "--kept"][..],
        &["clean", "--in"][..],
        &["clean", "-n", "-n"][..],
        &["clean", "--dry-run", "--dry-run"][..],
        &["clean", "one", "two"][..],
        &["clean", "-n", "one", "two"][..],
    ] {
        let run = wb_in(&bench, args);
        assert_eq!(code(&run), Some(2), "{:?}: {}", args, err_of(&run));
        assert_eq!(out_of(&run), "", "{:?}", args);
        assert!(err_of(&run).starts_with("wb clean: ") && err_of(&run).contains("usage:"), "{:?}: {}", args, err_of(&run));
        assert!(folder.exists(), "{:?}", args);
    }
}

#[test]
fn test_WB_VIEW_133_a_folder_that_is_not_a_repository_ends_with_2_and_removes_nothing() {
    let bench = bench();
    let (folder, _) = done(&bench, "2026-11-22T100000Z-build-54");
    let elsewhere = self::folder();
    let run = Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(["clean"])
        .current_dir(&elsewhere.path)
        .env_clear()
        .envs(env())
        .env("XDG_STATE_HOME", &bench.state.path)
        .output()
        .unwrap();
    assert_eq!(code(&run), Some(2), "{}", err_of(&run));
    assert!(err_of(&run).contains("not in a git repository"), "{}", err_of(&run));
    assert!(folder.exists());
}

#[test]
fn test_WB_VIEW_133_with_no_state_folder_nothing_is_kept_and_it_ends_with_0() {
    let bench = bench();
    let run = Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(["clean"])
        .current_dir(&bench.repo.path)
        .env_clear()
        .envs(env().into_iter().filter(|(name, _)| name != "XDG_STATE_HOME" && name != "HOME"))
        .output()
        .unwrap();
    assert_eq!((code(&run), out_of(&run)), (Some(0), String::new()), "{}", err_of(&run));
}

// ---- WB-VIEW-134: the usage, the help, the man pages, the completion and the README name them ----

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn test_WB_VIEW_134_clean_is_a_built_command_with_its_usage_and_help() {
    assert!(watchbill::layout::planned("clean").is_none());
    assert!(watchbill::ARMS.contains(&"clean"));
    assert!(watchbill::USAGE.contains("\n  wb clean [--dry-run | -n] [<view>]\n"), "{}", watchbill::USAGE);
    assert!(watchbill::USAGE.contains("\n            or --kept\n"), "{}", watchbill::USAGE);
    let folder = folder();
    let run = wb(&folder.path, &["help", "clean"], "");
    assert_eq!(code(&run), Some(0));
    assert!(out_of(&run).contains("wb clean [--dry-run | -n] [<view>]") && out_of(&run).contains("  clean "), "{}", out_of(&run));
    let help = wb(&folder.path, &["help"], "");
    assert!(out_of(&help).contains("clean") && !out_of(&help).contains("planned for 0.12.0, #176"), "{}", out_of(&help));
}

#[test]
fn test_WB_VIEW_134_the_man_pages_the_completion_and_the_readme_name_them() {
    let page = fs::read_to_string(root().join("packaging/man/wb-clean.1")).expect("packaging/man/wb-clean.1");
    assert!(page.contains("\\-\\-dry\\-run") && page.contains("wb\\-clean"), "{}", page);
    let view = fs::read_to_string(root().join("packaging/man/wb-view.1")).unwrap();
    assert!(view.contains("\\-\\-kept"), "wb-view.1");
    let main = fs::read_to_string(root().join("packaging/man/wb.1")).unwrap();
    assert!(main.contains("wb\\-clean"), "wb.1");
    let spec = fs::read_to_string(root().join("packaging/watchbill.spec")).unwrap();
    assert!(spec.contains("wb-clean.1"), "the spec");
    let completion = fs::read_to_string(root().join("packaging/wb.bash")).unwrap();
    assert!(completion.lines().any(|line| line.starts_with("_wb_flags_clean=") && line.contains("--dry-run")), "the completion");
    assert!(completion.lines().any(|line| line.starts_with("_wb_flags_view=") && line.contains("--kept")), "the completion");
    let readme = fs::read_to_string(root().join("README.md")).unwrap();
    assert!(readme.contains("wb clean") && readme.contains("wb view --kept"), "README.md");
}
