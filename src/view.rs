//! The view (#126): a session of the tester, the build, the review or the fix of an issue with an accepted file
//! sees only the paths its issue's file lists for it, and the program keeps that limit, not the prompt.
//!
//! - **The view** is a new git repository, made by `wb` from the trusted side: the paths of `changes` and of the
//!   role's `reads`, as they are in `HEAD` of the checkout, in one commit, with no remote and no history.
//! - **The sandbox** is bubblewrap: the view at the path of the checkout, writable; the system read-only; `/tmp`
//!   and `/run` of its own; the home folder empty but for the runner's `binds`; the folder of the organisation's
//!   file covered. The network stays as the runner needs it.
//! - **What comes back**: each commit of the view, in order, put on the checkout's branch when it keeps to
//!   `changes`, and refused, with every commit after it, when it does not. `wb` pushes nothing: `wb start <role>` says
//!   the command that pushes, and a person runs it (#145).
//! - **Every other session** (#202): the planner, and a role with no issue's file, run in a view of every file
//!   of `HEAD`, made the same way. The planner's is writable but for `.watchbill/` other than `.watchbill/issues/`
//!   (#295), and only its issue's file comes back; that of a role with no issue's file is writable but for
//!   `.watchbill/`, and what comes back keeps out of `.watchbill/` and `build_files`.
//!
//! Once the session has run, what is in the view is the session's. `wb` never runs git with the view's own
//! settings, which would run a command the session wrote there outside the sandbox: it reads the view's objects
//! from a repository of its own, through `alternates`, and reads the view's `HEAD` from its files.

use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use crate::starter::parts;

/// bubblewrap, at the path the distribution's package puts it. It is fixed in `wb`, as the organisation's file
/// is: it is not looked for on the `PATH`, so that nothing put there stands in for it.
pub const BWRAP: &str = "/usr/bin/bwrap";

/// The paths of the view of `role`: those of `changes`, then those of the role's `reads`, each once, as the
/// issue's file writes them.
pub fn paths(issue: &crate::issue::Issue, role: &str) -> Vec<String> {
    let reads = issue.reads.iter().find(|(name, _)| name == role).map(|(_, list)| list.as_slice()).unwrap_or(&[]);
    let mut found: Vec<String> = Vec::new();
    for path in issue.changes.iter().chain(reads) {
        if !found.contains(path) {
            found.push(path.clone());
        }
    }
    found
}

/// A local change that a view would miss (#172): a path of the view whose file in the working tree differs from
/// `HEAD`, or an untracked file under a path of `changes`.
#[derive(Clone, Debug, PartialEq)]
pub struct Local {
    pub path: String,
    /// It is under a path of `changes`, and no flag passes it. Otherwise it is under `reads` only.
    pub changes: bool,
}

/// Whether `path` is `listed` or lies under it.
fn under(path: &str, listed: &str) -> bool {
    let listed = listed.trim_end_matches('/');
    path == listed || path.strip_prefix(listed).is_some_and(|rest| rest.starts_with('/'))
}

/// The local changes the view of `role` would miss, sorted: a view is made from `HEAD`, so a path of its paths that
/// differs from `HEAD` in the index or in the working tree (changed, staged, deleted, a rename as both its
/// paths), a file marked `assume-unchanged` or `skip-worktree` that is changed, and any untracked file under a path
/// of `changes`, would not be in it. An untracked file is named whatever ignores it, by the rules of the checkout
/// or of the person's global file: the fast-forward at the end writes over an ignored file without a word. Reads
/// only: git is told not to take its optional locks. Each path is as the working tree says it, not escaped.
pub fn local_changes(root: &str, issue: &crate::issue::Issue, role: &str) -> Result<Vec<Local>, String> {
    let listed = paths(issue, role);
    // No path, no view to guard: git would take an empty list of paths as the whole checkout.
    if listed.is_empty() {
        return Ok(Vec::new());
    }
    let names = changed(root, Some(&listed), &issue.changes)?;
    Ok(names
        .into_iter()
        .map(|path| {
            let changes = issue.changes.iter().any(|listed| under(&path, listed));
            Local { path, changes }
        })
        .collect())
}

/// The local changes a view of the whole tree would miss (#202), sorted: as `local_changes`, over every path of the
/// checkout, with every untracked file that git does not ignore. `file`, the planner's issue's file, stands as
/// `changes`: it is named also where it is untracked and ignored, and no flag passes it. With none, for a role with
/// no issue's file, `--ignore-local` passes every one.
pub fn local_changes_whole(root: &str, file: Option<&str>) -> Result<Vec<Local>, String> {
    let others: Vec<String> = file.map(str::to_string).into_iter().collect();
    let names = changed(root, None, &others)?;
    Ok(names
        .into_iter()
        .map(|path| {
            let changes = file.is_some_and(|file| under(&path, file));
            Local { path, changes }
        })
        .collect())
}

/// The paths of `listed`, or of the whole checkout where it is none, that differ from `HEAD` in the index or the
/// working tree, or are marked unchanged and are changed; with the whole checkout, every untracked file that git
/// does not ignore; and every untracked file under `others`, ignored or not. Sorted, each once.
fn changed(root: &str, listed: Option<&[String]>, others: &[String]) -> Result<Vec<String>, String> {
    let root_path = Path::new(root);
    let split = |output: Vec<u8>| -> Vec<String> {
        output.split(|byte| *byte == 0).filter(|name| !name.is_empty()).map(|name| String::from_utf8_lossy(name).to_string()).collect()
    };
    let with = |first: &[&'static str], paths: Option<&[String]>| -> Result<Vec<u8>, String> {
        let mut command: Vec<&str> = vec!["--no-optional-locks", "--literal-pathspecs"];
        command.extend(first);
        if let Some(paths) = paths {
            command.push("--");
            command.extend(paths.iter().map(String::as_str));
        }
        git(root_path, &command, &[], None)
    };
    // Index and working tree against `HEAD`, both columns; a rename is two entries. Over the whole checkout, an
    // untracked file that is not ignored is named too, each file of a new folder on its own.
    let untracked = if listed.is_some() { "--untracked-files=no" } else { "--untracked-files=all" };
    let status = with(&["status", "--porcelain=v1", "-z", "--no-renames", untracked], listed)?;
    let mut names: Vec<String> = split(status).into_iter().filter_map(|entry| entry.get(3..).map(str::to_string)).collect();
    // A file marked as unchanged is not looked at by status: compare it with `HEAD` here.
    let marked: Vec<(char, String)> = split(with(&["ls-files", "-v", "-z"], listed)?)
        .into_iter()
        .filter_map(|entry| {
            let tag = entry.chars().next()?;
            (tag.is_ascii_lowercase() || tag == 'S').then(|| (tag, entry.get(2..).unwrap_or_default().to_string()))
        })
        .collect();
    if !marked.is_empty() {
        let names_marked: Vec<String> = marked.iter().map(|(_, path)| path.clone()).collect();
        let tree = split(with(&["ls-tree", "-r", "-z", "HEAD"], Some(&names_marked))?);
        for (tag, path) in &marked {
            let at_head = tree.iter().find_map(|entry| {
                let (meta, name) = entry.split_once('\t')?;
                (name == path).then(|| meta.split(' ').collect::<Vec<_>>())
            });
            let in_tree = root_path.join(path);
            let now = match fs::symlink_metadata(&in_tree) {
                // A path the sparse checkout leaves out is not a change.
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    if matches!(tag, 'S' | 's') && at_head.is_some() {
                        continue;
                    }
                    None
                }
                Err(_) => None,
                Ok(found) if found.file_type().is_symlink() => {
                    let target = fs::read_link(&in_tree).map(|target| target.into_os_string().into_encoded_bytes()).unwrap_or_default();
                    git(root_path, &["hash-object", "--stdin"], &[], Some(&target)).ok().map(|oid| line(&oid))
                }
                Ok(_) => git(root_path, &["hash-object", "--", path], &[], None).ok().map(|oid| line(&oid)),
            };
            let same = match (&at_head, &now) {
                (Some(meta), Some(now)) => meta.get(2) == Some(&now.as_str()),
                _ => false,
            };
            if !same {
                names.push(path.clone());
            }
        }
    }
    if !others.is_empty() {
        names.extend(split(with(&["ls-files", "--others", "-z"], Some(others))?));
    }
    names.sort();
    names.dedup();
    Ok(names)
}

/// What the start says of local changes it refuses (#172), after `wb start <role>:`. `ignore` is whether
/// `--ignore-local` was given.
pub fn local_message(found: &[Local], ignore: bool) -> String {
    let list = |changes: bool| -> String {
        let of: Vec<&Local> = found.iter().filter(|local| local.changes == changes).collect();
        let mut named: Vec<String> = of.iter().take(SHOWN).map(|local| crate::issue::shown(&local.path)).collect();
        if of.len() > SHOWN {
            named.push(format!("and {} more", of.len() - SHOWN));
        }
        named.join(", ")
    };
    let (changes, reads) = (list(true), list(false));
    let mut text = String::from(
        "a local change to a file of the view stops the start: the view is made from HEAD, and a session in it would not see the change",
    );
    if !changes.is_empty() {
        text.push_str(&format!("\n  changes: {}", changes));
    }
    if !reads.is_empty() {
        text.push_str(&format!("\n  reads: {}", reads));
    }
    if changes.is_empty() {
        text.push_str("\nCommit or put the change away, or give --ignore-local to start without it: the session sees HEAD");
    } else if ignore {
        text.push_str("\nA local change under changes cannot be overridden, with --ignore-local or otherwise: the session's commits would not come back over it. Commit it or put it away");
    } else {
        text.push_str("\nA local change under changes cannot be overridden: commit it or put it away. --ignore-local passes a change under reads only");
    }
    text
}

/// The list of paths as the session folder holds it and the stamp hashes it: one to a line.
pub fn list(paths: &[String]) -> String {
    paths.iter().map(|path| format!("{}\n", path)).collect()
}

/// The stamp's `view:`: the SHA-256 of the list.
pub fn hash(paths: &[String]) -> String {
    crate::manifest::hash_bytes(list(paths).as_bytes())
}

/// Runs git in `at` with `args`, with no variable `GIT_*` of this process and with `env`, and `input` on standard
/// input. The result is standard output; a git that cannot be run, or ends with anything but 0, is the error, with
/// the first line it said.
fn git(at: &Path, args: &[&str], env: &[(&str, &Path)], input: Option<&[u8]>) -> Result<Vec<u8>, String> {
    run_git(Some(at), &[], args, env, input)
}

thread_local! {
    /// The time a git run may take, while a `Bounded` is held: a view is hostile, and git may wait on what it holds.
    static BOUND: std::cell::Cell<Option<std::time::Duration>> = const { std::cell::Cell::new(None) };
}

thread_local! {
    /// The time by which a whole piece of work must be over, while a `Deadline` is held (#223, #265): the instant,
    /// the time it was given, and what the work is called.
    static DEADLINE: std::cell::Cell<Option<(std::time::Instant, std::time::Duration, &'static str)>> = const { std::cell::Cell::new(None) };
}

/// Holds the deadline of a whole piece of work (a take-back, the read of one view) for this thread, until it is
/// dropped.
struct Deadline(Option<(std::time::Instant, std::time::Duration, &'static str)>);

impl Drop for Deadline {
    fn drop(&mut self) {
        DEADLINE.with(|deadline| deadline.set(self.0));
    }
}

/// A deadline of `total` from now. Where one is held already that ends sooner, it stays; the earlier one is held
/// again when the guard is dropped (#265).
fn deadline(total: std::time::Duration, what: &'static str) -> Deadline {
    let before = DEADLINE.with(|deadline| deadline.get());
    let new = (std::time::Instant::now() + total, total, what);
    DEADLINE.with(|deadline| deadline.set(Some(before.filter(|held| held.0 <= new.0).unwrap_or(new))));
    Deadline(before)
}

/// Holds the time bound of `git` for this thread, until it is dropped (#222).
struct Bounded(());

impl Drop for Bounded {
    fn drop(&mut self) {
        BOUND.with(|bound| bound.set(None));
    }
}

fn bounded(limit: std::time::Duration) -> Bounded {
    BOUND.with(|bound| bound.set(Some(limit)));
    Bounded(())
}

/// Waits for `child` at most `limit`: past it the child is killed, by its own process ID, and waited for, and the
/// result is a `TimedOut` error. The output is read through a cap.
fn wait_bounded(mut child: std::process::Child, limit: std::time::Duration) -> std::io::Result<std::process::Output> {
    use std::io::Read;
    fn reading(pipe: Option<impl Read + Send + 'static>) -> std::thread::JoinHandle<Vec<u8>> {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(pipe) = pipe {
                let _ = pipe.take(64 * 1024 * 1024).read_to_end(&mut bytes);
            }
            bytes
        })
    }
    let (out, err) = (reading(child.stdout.take()), reading(child.stderr.take()));
    let began = std::time::Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            // As on a time-out: the child is not left to run.
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        }
        if began.elapsed() > limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::ErrorKind::TimedOut.into());
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    };
    Ok(std::process::Output { status, stdout: out.join().unwrap_or_default(), stderr: err.join().unwrap_or_default() })
}

/// The git command of a run: the first word that is not an option (`-c` takes a word of its own).
fn command_of<'a>(args: &[&'a str]) -> &'a str {
    let mut words = args.iter().copied();
    while let Some(word) = words.next() {
        if word == "-c" || word == "-C" {
            words.next();
        } else if !word.starts_with('-') {
            return word;
        }
    }
    ""
}

/// `git` as `git` runs it, with `before` words after `git`, in `at` where there is a folder.
fn run_git(at: Option<&Path>, before: &[OsString], args: &[&str], env: &[(&str, &Path)], input: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let ran = run_git_status(at, before, args, env, input)?;
    if ran.code == 0 {
        Ok(ran.stdout)
    } else {
        Err(ran.said)
    }
}

/// A git run that ended: its exit status, its standard output, and the words for an end that is not 0.
struct Ran {
    code: i32,
    stdout: Vec<u8>,
    said: String,
}

/// As `run_git`, with the exit status kept, so that a decision is made on the status and never on the words. The error
/// is a git that could not be run, or was ended after its bound.
fn run_git_status(at: Option<&Path>, before: &[OsString], args: &[&str], env: &[(&str, &Path)], input: Option<&[u8]>) -> Result<Ran, String> {
    let mut limit = BOUND.with(|bound| bound.get());
    // What is said where the deadline, and not the bound of the run, ended it.
    let mut by_deadline = None;
    // One deadline for a whole piece of work, besides the bound of each run.
    if let Some((end, total, what)) = DEADLINE.with(|deadline| deadline.get()) {
        let left = end.saturating_duration_since(std::time::Instant::now());
        if left.is_zero() {
            return Err(format!("the {} took more than {} seconds in all, and was ended", what, total.as_secs()));
        }
        if limit.map_or(true, |limit| left < limit) {
            by_deadline = Some(format!("the {} took more than {} seconds in all, and was ended", what, total.as_secs()));
        }
        limit = Some(limit.map_or(left, |limit| limit.min(left)));
    }
    let mut command = Command::new("git");
    if let Some(at) = at {
        command.arg("-C").arg(at);
    }
    command.args(before).args(args);
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    // A git run in the temporary store of `wb` is fully `wb`'s own: no setting of the system or of the person is read.
    if at.is_some_and(|at| at.parent() == Some(std::env::temp_dir().as_path()) && at.file_name().is_some_and(|name| name.to_string_lossy().starts_with("wb-view-"))) {
        command.env("GIT_CONFIG_NOSYSTEM", "1").env("GIT_CONFIG_GLOBAL", "/dev/null");
    }
    for (name, value) in env {
        command.env(name, value);
    }
    command.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() }).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| format!("git could not be run: {}", error))?;
    let stdin = child.stdin.take();
    let output = std::thread::scope(|scope| {
        if let (Some(input), Some(mut stdin)) = (input, stdin) {
            scope.spawn(move || {
                let _ = stdin.write_all(input);
            });
        }
        match limit {
            None => child.wait_with_output(),
            Some(limit) => wait_bounded(child, limit),
        }
    })
    .map_err(|error| {
        if error.kind() == std::io::ErrorKind::TimedOut {
            by_deadline.unwrap_or_else(|| format!("`git {}` took more than {} seconds and was ended", command_of(args), limit.map_or(0, |limit| limit.as_secs())))
        } else {
            format!("git could not be waited for: {}", error)
        }
    })?;
    if output.status.success() {
        return Ok(Ran { code: 0, stdout: output.stdout, said: String::new() });
    }
    let said = String::from_utf8_lossy(&output.stderr);
    let first = said.lines().find(|line| !line.trim().is_empty()).unwrap_or("").trim().to_string();
    let code = output.status.code().unwrap_or(-1);
    let said = format!("`git {}` ended with {}{}", command_of(args), code, if first.is_empty() { String::new() } else { format!(": {}", first) });
    Ok(Ran { code, stdout: output.stdout, said })
}

fn line(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim().to_string()
}

/// A folder of its own, new, readable by its owner alone: `<base>`, or `<base>-2`, `-3` and on when it is there.
fn new_folder(below: &Path, base: &str) -> Result<PathBuf, String> {
    fs::create_dir_all(below).map_err(|error| format!("{} could not be made: {}", below.display(), error))?;
    for number in 1..10_000 {
        let folder = if number == 1 { below.join(base) } else { below.join(format!("{}-{}", base, number)) };
        match fs::DirBuilder::new().mode(0o700).create(&folder) {
            Ok(()) => return Ok(folder),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("{} could not be made: {}", folder.display(), error)),
        }
    }
    Err(format!("no new folder could be made in {}", below.display()))
}

/// Removes a folder and all below it, also folders the session left without write permission.
pub(crate) fn remove(folder: &Path) -> std::io::Result<()> {
    let mut open = vec![folder.to_path_buf()];
    while let Some(at) = open.pop() {
        if let Ok(found) = fs::symlink_metadata(&at) {
            if found.is_dir() {
                let _ = fs::set_permissions(&at, fs::Permissions::from_mode(0o700));
                if let Ok(entries) = fs::read_dir(&at) {
                    open.extend(entries.filter_map(|entry| entry.ok()).map(|entry| entry.path()));
                }
            }
        }
    }
    fs::remove_dir_all(folder)
}

/// A view made: its folder, the one commit it was made with, the commit of the checkout it was made from, and
/// the branch its commits go back to: the stamp's branch, or the one checked out where the stamp names none (#296).
#[derive(Clone, Debug)]
pub struct View {
    pub folder: PathBuf,
    pub base: String,
    pub head: String,
    /// The branch, as `refs/heads/<name>`.
    pub branch: String,
}

impl View {
    /// The branch's name, as a person writes it.
    pub fn branch_name(&self) -> &str {
        self.branch.strip_prefix("refs/heads/").unwrap_or(&self.branch)
    }
}

/// The checkout's branch, as `refs/heads/<name>`, and the commit `HEAD` names. A `HEAD` that names no branch is
/// refused: the commits of the view go back onto the branch.
pub fn branch_of(root: &str) -> Result<(String, String), String> {
    let root = Path::new(root);
    let branch = git(root, &["symbolic-ref", "-q", "HEAD"], &[], None).map(|out| line(&out)).map_err(|_| {
        "the checkout's HEAD names no branch, and the commits of the view go back onto the branch HEAD names. Check out the issue's branch".to_string()
    })?;
    let head = git(root, &["rev-parse", "--verify", "-q", "HEAD^{commit}"], &[], None)
        .map(|out| line(&out))
        .map_err(|_| "the checkout's HEAD names no commit, and the view is made from it".to_string())?;
    Ok((branch, head))
}

/// The ref of the branch named `name`, as `refs/heads/<name>`, where git takes it as the name of a branch.
fn branch_ref(root: &str, name: &str) -> Result<String, String> {
    let full = format!("refs/heads/{}", name);
    let fine = !name.starts_with('-') && run_git_status(Some(Path::new(root)), &[], &["check-ref-format", &full], &[], None)?.code == 0;
    if !fine {
        return Err(format!("the branch '{}' that the commits of the view go back onto is not a name git takes for a branch", name.escape_debug()));
    }
    Ok(full)
}

/// The commit the branch `branch` (`refs/heads/<name>`) of the checkout names, or none where it is not there.
fn tip_of(root: &str, branch: &str) -> Result<Option<String>, String> {
    let ran = run_git_status(Some(Path::new(root)), &[], &["rev-parse", "--verify", "-q", &format!("{}^{{commit}}", branch)], &[], None)?;
    match ran.code {
        0 => Ok(Some(line(&ran.stdout))),
        1 => Ok(None),
        _ => Err(ran.said),
    }
}

/// The branch checked out in the checkout at `root`, as `refs/heads/<name>`, or none where `HEAD` names none.
fn checked_out(root: &str) -> Result<Option<String>, String> {
    let ran = run_git_status(Some(Path::new(root)), &[], &["symbolic-ref", "-q", "HEAD"], &[], None)?;
    match ran.code {
        0 => Ok(Some(line(&ran.stdout))),
        1 => Ok(None),
        _ => Err(ran.said),
    }
}

/// The worktree of the repository, other than the checkout at `root`, that has `branch` checked out, if any.
fn checked_out_elsewhere(root: &str, branch: &str) -> Result<Option<String>, String> {
    let listed = git(Path::new(root), &["worktree", "list", "--porcelain", "-z"], &[], None)?;
    let here = fs::canonicalize(root).ok();
    let mut tree: Option<String> = None;
    for entry in listed.split(|byte| *byte == 0).map(|entry| String::from_utf8_lossy(entry).to_string()) {
        if let Some(path) = entry.strip_prefix("worktree ") {
            tree = Some(path.to_string());
        } else if entry.strip_prefix("branch ") == Some(branch) {
            if let Some(path) = &tree {
                if fs::canonicalize(path).ok() != here {
                    return Ok(Some(path.clone()));
                }
            }
        }
    }
    Ok(None)
}

/// How the branch checked out stands to `stamped`, the stamp's branch that the commits of the session come back onto,
/// before a start (#296, #298).
#[derive(Debug, PartialEq)]
pub enum OnBranch {
    /// The start goes on.
    Fine,
    /// A limit: the stamp's branch is there, and the checkout's `HEAD` is not its commit, so the view, made from `HEAD`,
    /// would not be the branch's tree.
    Limit(String),
    /// A guard, passed by `--ignore-branch`: the checkout is on another branch at the same tree, or on a branch that is
    /// neither the default branch nor the stamp's, where that is not there yet. The person may mean it.
    Guard(String),
}

/// How the checkout at `root` stands to `stamped` (`OnBranch`). A checkout on no branch, or a name git does not take,
/// is `Fine` here: the view refuses it, as before. Reads only.
pub fn on_branch(root: &str, stamped: &str) -> OnBranch {
    let (Ok(Some(checked)), Ok(full)) = (checked_out(root), branch_ref(root, stamped)) else { return OnBranch::Fine };
    let Ok(Some(head)) = tip_of(root, "HEAD") else { return OnBranch::Fine };
    let Ok(tip) = tip_of(root, &full) else { return OnBranch::Fine };
    let name = checked.strip_prefix("refs/heads/").unwrap_or(&checked).to_string();
    let (shown, branch) = (crate::issue::shown(stamped), crate::issue::shown(&name));
    if let Some(tip) = &tip {
        if *tip != head {
            return OnBranch::Limit(format!(
                "the checkout is on {} at {}, and {}, the branch of the stamp that the commits of the session come back onto, is at {}: the session would run in the tree of {}, not of {}. Check out {}",
                branch,
                short(&head),
                shown,
                short(tip),
                branch,
                shown,
                shown
            ));
        }
    }
    if name == stamped {
        return OnBranch::Fine;
    }
    // The default branch is where a planner starts, before the stamp's branch is there.
    let default = crate::status::default_branch(root).map(|full| {
        let short = full.strip_prefix("refs/remotes/origin/").or_else(|| full.strip_prefix("refs/heads/")).unwrap_or(&full);
        short.to_string()
    });
    if tip.is_none() && default.as_deref() == Some(name.as_str()) {
        return OnBranch::Fine;
    }
    let why = match (&tip, &default) {
        (Some(_), _) => format!("the checkout is on {}, not on {}, the branch of the stamp that the commits of the session come back onto, though both are at {}", branch, shown, short(&head)),
        (None, Some(default)) => format!(
            "the checkout is on {}, which is neither {}, the branch of the stamp that the commits of the session come back onto, nor {}, the default branch",
            branch,
            shown,
            crate::issue::shown(default)
        ),
        (None, None) => format!(
            "the checkout is on {}, which is neither {}, the branch of the stamp that the commits of the session come back onto, nor the default branch, which is not found",
            branch, shown
        ),
    };
    OnBranch::Guard(format!("{}. The commits would go onto {}, and {} would not move. Check out {}, or give --ignore-branch to go on", why, shown, branch, shown))
}

/// Makes the view of `paths` from `HEAD` of the checkout at `root`, in a new folder below `below` named `base`.
/// A path that is not in `HEAD` is not made: the session may make it. A folder that cannot be made, or a git that
/// fails, is the error, and nothing is left.
pub fn make(root: &str, paths: &[String], below: &Path, base: &str) -> Result<View, String> {
    make_onto(root, paths, below, base, None).map(|(view, _)| view)
}

/// The same, with its commits to go back onto the branch `onto`, by its name, where one is given (#296): the
/// stamp's branch, which need not be the one checked out, nor be there yet. The view's own branch has its name.
/// Besides the view, the commit of that branch as the view is made, or none where it is not there: the take-back
/// at the end of the session finds it so, or takes nothing.
pub fn make_onto(root: &str, paths: &[String], below: &Path, base: &str, onto: Option<&str>) -> Result<(View, Option<String>), String> {
    make_of(root, Some(paths), below, base, onto).map(|(view, _, at)| (view, at))
}

/// How much a view of the whole tree holds: its files and their bytes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub files: usize,
    pub bytes: u64,
}

/// Makes the view of every file of `HEAD` of the checkout at `root` (#202), as `make` makes a view of paths, for the
/// planner and for a role with no issue's file. The files are copied by git's own checkout from the view's index,
/// never one `git show` for each. `.watchbill/` and `.watchbill/issues/` are folders of the view, made where they
/// are not there, as the sandbox binds over them; one of them that is anything else, a link above all, is the error.
pub fn make_whole(root: &str, below: &Path, base: &str) -> Result<(View, Size), String> {
    make_whole_onto(root, below, base, None).map(|(view, size, _)| (view, size))
}

/// The same, with its commits to go back onto the branch `onto`, as `make_onto` says, and the commit of that branch.
pub fn make_whole_onto(root: &str, below: &Path, base: &str, onto: Option<&str>) -> Result<(View, Size, Option<String>), String> {
    let (view, size, at) = make_of(root, None, below, base, onto)?;
    match bind_points(&view.folder) {
        Ok(()) => Ok((view, size, at)),
        Err(why) => {
            let _ = remove(&view.folder);
            Err(format!("the view could not be made: {}", why))
        }
    }
}

/// Makes sure that `.watchbill/` and `.watchbill/issues/` of a view of the whole tree are its own folders.
fn bind_points(folder: &Path) -> Result<(), String> {
    for name in [".watchbill", ".watchbill/issues"] {
        let path = folder.join(name);
        match fs::symlink_metadata(&path) {
            Ok(found) if found.is_dir() => {}
            Ok(_) => return Err(format!("{} of HEAD is not a folder, and the sandbox binds a folder there", name)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::DirBuilder::new().mode(0o700).create(&path).map_err(|error| format!("{} could not be made: {}", name, error))?;
            }
            Err(error) => return Err(format!("{} could not be looked at: {}", name, error)),
        }
    }
    Ok(())
}

/// The work of `make` and `make_whole`: the listed paths, or every file of `HEAD` where `paths` is none.
fn make_of(root: &str, paths: Option<&[String]>, below: &Path, base: &str, onto: Option<&str>) -> Result<(View, Size, Option<String>), String> {
    let (checked, head) = branch_of(root)?;
    let branch = match onto {
        Some(name) => branch_ref(root, name)?,
        None => checked,
    };
    let at = tip_of(root, &branch)?;
    let folder = new_folder(below, base)?;
    match fill(Path::new(root), paths, &folder, &branch, &head) {
        Ok((commit, size)) => Ok((View { folder, base: commit, head, branch }, size, at)),
        Err(why) => {
            let _ = remove(&folder);
            Err(format!("the view could not be made: {}", why))
        }
    }
}

/// A git command in `at`, with no variable `GIT_*` of this process, for a run whose input or output is a pipe.
fn git_command(at: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");
    command.arg("-C").arg(at).args(args);
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    command
}

/// Copies the objects named in `blobs`, one to a line, from the checkout at `root` into the repository of the view
/// at `folder`, as one pack streamed from one git to the other: a whole tree is never held in `wb`'s memory, and no
/// delta is looked for.
fn copy_objects(root: &Path, blobs: &[u8], folder: &Path) -> Result<(), String> {
    let first = |bytes: &[u8]| String::from_utf8_lossy(bytes).lines().find(|line| !line.trim().is_empty()).unwrap_or("").trim().to_string();
    let mut packer = git_command(root, &["pack-objects", "--stdout", "-q", "--window=0"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("git could not be run: {}", error))?;
    let packed = packer.stdout.take().ok_or("git gave no pipe")?;
    let indexer = git_command(folder, &["index-pack", "--stdin"]).stdin(Stdio::from(packed)).stdout(Stdio::null()).stderr(Stdio::piped()).spawn();
    let indexer = match indexer {
        Ok(indexer) => indexer,
        Err(error) => {
            let _ = packer.kill();
            let _ = packer.wait();
            return Err(format!("git could not be run: {}", error));
        }
    };
    let mut stdin = packer.stdin.take().ok_or("git gave no pipe")?;
    let (packed, indexed) = std::thread::scope(|scope| {
        scope.spawn(move || {
            let _ = stdin.write_all(blobs);
        });
        let indexed = scope.spawn(move || indexer.wait_with_output());
        let packed = packer.wait_with_output();
        (packed, indexed.join().unwrap_or_else(|_| Err(std::io::Error::other("the wait for git failed"))))
    });
    let packed = packed.map_err(|error| format!("git could not be waited for: {}", error))?;
    let indexed = indexed.map_err(|error| format!("git could not be waited for: {}", error))?;
    if !packed.status.success() {
        return Err(format!("`git pack-objects` ended with {}: {}", packed.status.code().unwrap_or(-1), first(&packed.stderr)));
    }
    if !indexed.status.success() {
        return Err(format!("`git index-pack` ended with {}: {}", indexed.status.code().unwrap_or(-1), first(&indexed.stderr)));
    }
    Ok(())
}

/// The work of `make` once the folder is there: a repository with the listed files of `HEAD`, or all of them, and
/// one commit. The result is the commit, and the files and bytes of the view.
fn fill(root: &Path, paths: Option<&[String]>, folder: &Path, branch: &str, head: &str) -> Result<(String, Size), String> {
    let short = branch.strip_prefix("refs/heads/").unwrap_or(branch);
    let literal = [("GIT_LITERAL_PATHSPECS", Path::new("1"))];
    // The entries of `HEAD` below each path, or all of them, as `ls-tree` gives them with their sizes; a submodule
    // is left out.
    let listed = match paths {
        Some(paths) => {
            let wanted: Vec<String> = paths.iter().map(|path| parts(path).join("/")).filter(|path| !path.is_empty()).collect();
            let mut args = vec!["ls-tree", "-r", "-z", "-l", "--full-tree", head, "--"];
            args.extend(wanted.iter().map(String::as_str));
            if wanted.is_empty() {
                Vec::new()
            } else {
                git(root, &args, &literal, None)?
            }
        }
        None => git(root, &["ls-tree", "-r", "-z", "-l", "--full-tree", head], &[], None)?,
    };
    let mut entries: Vec<u8> = Vec::new();
    let mut blobs: Vec<u8> = Vec::new();
    let mut size = Size { files: 0, bytes: 0 };
    for entry in listed.split(|byte| *byte == 0).filter(|entry| !entry.is_empty()) {
        let Some(tab) = entry.iter().position(|byte| *byte == b'\t') else { continue };
        let meta = String::from_utf8_lossy(&entry[..tab]);
        let words: Vec<&str> = meta.split_whitespace().collect();
        let [mode, kind, object, bytes] = words.as_slice() else { continue };
        if *kind != "blob" {
            continue;
        }
        entries.extend_from_slice(format!("{} {} {}\t", mode, kind, object).as_bytes());
        entries.extend_from_slice(&entry[tab + 1..]);
        entries.push(0);
        blobs.extend_from_slice(object.as_bytes());
        blobs.push(b'\n');
        size.files += 1;
        size.bytes += bytes.parse::<u64>().unwrap_or(0);
    }
    let at = folder.to_str().ok_or("the folder of the view is not UTF-8 text")?;
    git(root, &["init", "-q", "--template=", &format!("--initial-branch={}", short), at], &[], None)?;
    // The session commits as the person: the identity of the checkout, where it has one.
    for key in ["user.name", "user.email"] {
        if let Ok(value) = git(root, &["config", "--get", key], &[], None) {
            git(folder, &["config", key, &line(&value)], &[], None)?;
        }
    }
    if !blobs.is_empty() {
        if paths.is_some() {
            let pack = git(root, &["pack-objects", "--stdout", "-q"], &[], Some(&blobs))?;
            git(folder, &["unpack-objects", "-q"], &[], Some(&pack))?;
        } else {
            copy_objects(root, &blobs, folder)?;
        }
        git(folder, &["update-index", "--add", "-z", "--index-info"], &[], Some(&entries))?;
        git(folder, &["checkout-index", "--all", "--force"], &[], None)?;
    }
    let tree = line(&git(folder, &["write-tree"], &[], None)?);
    let who = [
        ("GIT_AUTHOR_NAME", Path::new("Watchbill")),
        ("GIT_AUTHOR_EMAIL", Path::new("watchbill@localhost")),
        ("GIT_COMMITTER_NAME", Path::new("Watchbill")),
        ("GIT_COMMITTER_EMAIL", Path::new("watchbill@localhost")),
    ];
    let message: &[u8] = match paths {
        Some(_) => b"The view: the paths of the issue's file for this role, as they are in HEAD of the checkout.\n",
        None => b"The view: every file of HEAD of the checkout.\n",
    };
    let commit = line(&git(folder, &["commit-tree", "--no-gpg-sign", &tree], &who, Some(message))?);
    git(folder, &["update-ref", branch, &commit], &[], None)?;
    Ok((commit, size))
}

/// How the tree of the view stands in the sandbox (#202).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Layout {
    /// A view of the paths of an issue's file: all of it writable.
    Listed,
    /// The planner's view of the whole tree: writable, so that a build and the tests can run, but for
    /// `.watchbill/` other than `.watchbill/issues/`, where it writes its issue's file (#295).
    Planner,
    /// The view of the whole tree of a role with no issue's file: writable, but for `.watchbill/`.
    Whole,
}

/// What a session in a view is run with.
pub struct Plan<'a> {
    pub view: &'a Path,
    pub checkout: &'a Path,
    /// The runner's `binds`, each as its path from the root and whether it is writable.
    pub binds: Vec<(PathBuf, bool)>,
    /// The folder of the organisation's file, covered by an empty one, or none when it is not there.
    pub organisation: Option<&'a Path>,
    /// The home folder, an empty one of the session's own, or none.
    pub home: Option<&'a Path>,
    /// Whether the session is given no controlling terminal (`setsid`).
    pub new_session: bool,
    /// The runner's `start`, filled.
    pub words: &'a [String],
}

/// The system, as the sandbox has it, read-only: `/usr`, the links to it or the folders beside it, `/etc` and
/// `/sys`; `/proc` and `/dev` of its own; `/tmp` and `/run` empty, but for the folder of the resolver where
/// `/etc/resolv.conf` points into `/run`, so that the model can be reached.
/// The folders of the system a program of a runner may lie in, as the sandbox holds them (`system`); `/sys` and
/// `/proc` hold no program.
pub const SYSTEM: [&str; 8] = ["/usr", "/bin", "/sbin", "/lib", "/lib64", "/lib32", "/libx32", "/etc"];

fn system() -> Vec<OsString> {
    let mut words: Vec<OsString> = Vec::new();
    let mut add = |more: &[&str]| words.extend(more.iter().map(OsString::from));
    add(&["--ro-bind", "/usr", "/usr"]);
    for top in ["/bin", "/sbin", "/lib", "/lib64", "/lib32", "/libx32"] {
        match fs::symlink_metadata(top) {
            Ok(found) if found.file_type().is_symlink() => {
                if let Ok(target) = fs::read_link(top) {
                    if let Some(target) = target.to_str() {
                        add(&["--symlink", target, top]);
                    }
                }
            }
            Ok(found) if found.is_dir() => add(&["--ro-bind", top, top]),
            _ => {}
        }
    }
    add(&["--ro-bind", "/etc", "/etc"]);
    if Path::new("/sys").is_dir() {
        add(&["--ro-bind", "/sys", "/sys"]);
    }
    add(&["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp", "--tmpfs", "/run"]);
    if let Ok(resolver) = fs::canonicalize("/etc/resolv.conf") {
        if let (true, Some(folder)) = (resolver.starts_with("/run"), resolver.parent().and_then(Path::to_str)) {
            add(&["--ro-bind", folder, folder]);
        }
    }
    words
}

/// The namespaces of the sandbox: all but the network's.
const UNSHARE: [&str; 6] = ["--unshare-user", "--unshare-ipc", "--unshare-pid", "--unshare-uts", "--unshare-cgroup-try", "--die-with-parent"];

/// The words bubblewrap is given for a session in a view of the paths of an issue's file.
pub fn arguments(plan: &Plan) -> Vec<OsString> {
    arguments_in(plan, Layout::Listed)
}

/// Where the folder that `wb` makes for what a session writes for it stands in the sandbox (#151): outside the view's
/// tree, so that nothing in it is ever a commit.
pub const OUT_FOLDER: &str = "/run/watchbill";

/// The file the build writes the change's description to, `{description}` of the prompt (#151).
pub const DESCRIPTION: &str = "/run/watchbill/description";

/// The folder of what `wb` brings in for a session (#153), `{brought}` of the prompt: read-only, below `OUT_FOLDER`.
pub const BROUGHT: &str = "/run/watchbill/in";

/// Where a session writes its questions, `{questions}` of the prompt (#153).
pub const QUESTIONS: &str = "/run/watchbill/questions";

/// Where the review writes its findings and the fix its answers, `{change_comment}` of the prompt (#153).
pub const CHANGE_COMMENT: &str = "/run/watchbill/change_comment";

/// Where a session writes what only a person can settle, `{needs_person}` of the prompt (#153).
pub const NEEDS_PERSON: &str = "/run/watchbill/needs_person";

/// The folder below the folder of `wb`'s own that is bound read-only at `BROUGHT` (#153).
pub const BROUGHT_FOLDER: &str = "in";

/// The variables that hold or lead to a login of the code host (#151): `gh`'s, `glab`'s and ssh's, the secrets GitLab
/// CI hands a job, and the address of the session bus, which leads to the keyring `gh` keeps its login in. None is
/// handed into the sandbox, as no `GIT_*` is. They are named one by one, never by an ending such as `_TOKEN`, which
/// would take the runner's own login of its model too.
pub const LOGINS: [&str; 19] = [
    "GH_TOKEN",
    "GITHUB_TOKEN",
    "GH_ENTERPRISE_TOKEN",
    "GITHUB_ENTERPRISE_TOKEN",
    "GH_CONFIG_DIR",
    "GITLAB_TOKEN",
    "GLAB_TOKEN",
    "GITLAB_ACCESS_TOKEN",
    "GLAB_CONFIG_DIR",
    "OAUTH_TOKEN",
    "GLAB_CLIENT_CERT_PASSWORD",
    "CI_JOB_TOKEN",
    "CI_REGISTRY_PASSWORD",
    "CI_DEPENDENCY_PROXY_PASSWORD",
    "CI_JOB_JWT",
    "CI_JOB_JWT_V2",
    "SSH_AUTH_SOCK",
    "SSH_ASKPASS",
    "DBUS_SESSION_BUS_ADDRESS",
];

/// Takes the variables of a login of the code host, and every `GIT_*`, out of what `command` is handed.
pub fn without_logins(command: &mut Command) {
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    for name in LOGINS {
        command.env_remove(name);
    }
}

/// The words bubblewrap is given for a session in a view laid out as `layout`. The binds inside the view come after
/// the view itself, so that they stand over it; their sources are folders of the view (`make_whole`).
pub fn arguments_in(plan: &Plan, layout: Layout) -> Vec<OsString> {
    arguments_out(plan, layout, None)
}

/// The same, with `out`, a folder of `wb`'s own, bound writable at `OUT_FOLDER` after the view (#151).
pub fn arguments_out(plan: &Plan, layout: Layout, out: Option<&Path>) -> Vec<OsString> {
    let mut words: Vec<OsString> = UNSHARE.iter().map(OsString::from).collect();
    if plan.new_session {
        words.push("--new-session".into());
    }
    words.extend(system());
    let mut add = |more: &[&std::ffi::OsStr]| words.extend(more.iter().map(OsString::from));
    if let Some(home) = plan.home.filter(|home| home.is_absolute() && *home != Path::new("/")) {
        add(&["--tmpfs".as_ref(), home.as_os_str()]);
    }
    for (path, writable) in &plan.binds {
        let how = if *writable { "--bind-try" } else { "--ro-bind-try" };
        add(&[how.as_ref(), path.as_os_str(), path.as_os_str()]);
    }
    // The organisation's own folder, where `wb` reads its file, and the folder of the file given, when another.
    let fixed = Path::new(crate::confidentiality::ORGANISATION_FILE).parent().filter(|folder| folder.is_dir());
    for folder in [fixed, plan.organisation.filter(|folder| Some(*folder) != fixed)].into_iter().flatten() {
        add(&["--tmpfs".as_ref(), folder.as_os_str()]);
    }
    // What the session writes for `wb` (#151), in `/run` of the sandbox's own, before the view and away from it.
    if let Some(out) = out {
        add(&["--bind".as_ref(), out.as_os_str(), OUT_FOLDER.as_ref()]);
        // What `wb` brought in (#153), read-only over the folder it lies in.
        let brought = out.join(BROUGHT_FOLDER);
        if brought.is_dir() {
            add(&["--ro-bind".as_ref(), brought.as_os_str(), BROUGHT.as_ref()]);
        }
    }
    add(&["--bind".as_ref(), plan.view.as_os_str(), plan.checkout.as_os_str()]);
    let inside: &[(&str, &str)] = match layout {
        Layout::Listed => &[],
        Layout::Planner => &[("--ro-bind", ".watchbill"), ("--bind", ".watchbill/issues")],
        Layout::Whole => &[("--ro-bind", ".watchbill")],
    };
    for (how, path) in inside {
        let (from, to) = (plan.view.join(path), plan.checkout.join(path));
        add(&[how.as_ref(), from.as_os_str(), to.as_os_str()]);
    }
    add(&["--chdir".as_ref(), plan.checkout.as_os_str(), "--".as_ref()]);
    words.extend(plan.words.iter().map(OsString::from));
    words
}

/// Whether a session in a sandbox has a controlling terminal: only where the kernel refuses a program the
/// injection of input into its terminal (`TIOCSTI`), as `dev.tty.legacy_tiocsti = 0` says. Elsewhere a session
/// could type into the person's shell after it ends, and it is given no controlling terminal.
pub fn new_session() -> bool {
    fs::read_to_string("/proc/sys/dev/tty/legacy_tiocsti").map(|text| text.trim() != "0").unwrap_or(true)
}

/// Whether bubblewrap at `bwrap` can make a sandbox on this machine: it is there, and runs `/usr/bin/true` in
/// the namespaces and the system of a view. The error says why not.
pub fn works(bwrap: &Path) -> Result<(), String> {
    if !bwrap.is_file() {
        return Err(format!(
            "there is no {}, and every session that wb start starts runs in a view, in bubblewrap. Install the package bubblewrap",
            bwrap.display()
        ));
    }
    let mut command = Command::new(bwrap);
    command.args(UNSHARE).args(system()).args(["--", "/usr/bin/true"]);
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
    let output = command.output().map_err(|error| format!("{} could not be run: {}", bwrap.display(), error))?;
    if output.status.success() {
        return Ok(());
    }
    let said = String::from_utf8_lossy(&output.stderr);
    let first = said.lines().find(|line| !line.trim().is_empty()).unwrap_or("it said nothing").trim().to_string();
    let off = match fs::read_to_string("/proc/sys/user/max_user_namespaces") {
        Ok(text) if text.trim() == "0" => " User namespaces are off on this machine: /proc/sys/user/max_user_namespaces is 0.",
        _ => " Unprivileged user namespaces may be off on this machine.",
    };
    Err(format!("bubblewrap ({}) could not make a sandbox here: {}.{} Every session that wb start starts runs in a view", bwrap.display(), first.trim_end_matches('.'), off))
}

/// Runs the session in its view, in bubblewrap at `bwrap`, in the foreground, with the person's standard
/// input, output and error, and no variable `GIT_*` of this process. While it runs, an interrupt, a quit or a hangup at the
/// terminal does not end `wb`, and a termination is sent on to the session (`interrupt::hold`).
///
/// bubblewrap passes no signal on and dies of an interrupt, a quit, a hangup or a termination, and the session
/// with it (#257, #271). So it is started with all four ignored, the session is started with them as `wb` found
/// them (`env --default-signal`), and `wb` sends them on to the session itself, which bubblewrap names on
/// `--info-fd` (`session_of`): the termination always, the others only where the session has no controlling
/// terminal, as elsewhere it gets them from the terminal.
pub fn run(bwrap: &Path, plan: &Plan) -> Result<ExitStatus, String> {
    run_saying(bwrap, plan, "wb start", &mut std::io::sink())
}

/// The same, saying on `err`, as `name`, where the session was not found in bubblewrap (WB-BUILD-87).
pub fn run_saying(bwrap: &Path, plan: &Plan, name: &str, err: &mut dyn Write) -> Result<ExitStatus, String> {
    run_in(bwrap, plan, Layout::Listed, name, err)
}

/// The same, in a view laid out as `layout` (#202).
pub fn run_in(bwrap: &Path, plan: &Plan, layout: Layout, name: &str, err: &mut dyn Write) -> Result<ExitStatus, String> {
    run_out(bwrap, plan, layout, None, name, err)
}

/// The same, with `out` bound at `OUT_FOLDER` (#151).
pub fn run_out(bwrap: &Path, plan: &Plan, layout: Layout, out: Option<&Path>, name: &str, err: &mut dyn Write) -> Result<ExitStatus, String> {
    run_watched(bwrap, plan, layout, out, None, name, err)
}

/// The same, with the session watched by `watch` while `wb` waits for it (#267): a quiet is said on `err`, and past
/// `max_minutes` the session is sent the termination, and killed after the grace.
pub fn run_watched(bwrap: &Path, plan: &Plan, layout: Layout, out: Option<&Path>, watch: Option<&mut crate::watchdog::Watchdog>, name: &str, err: &mut dyn Write) -> Result<ExitStatus, String> {
    use std::os::fd::AsRawFd;
    use std::os::unix::process::{CommandExt, ExitStatusExt};
    let held = crate::interrupt::hold();
    let (reader, writer) = std::io::pipe().map_err(|error| format!("the session could not be started: no pipe: {}", error))?;
    let info = writer.as_raw_fd();
    // The signals the session gets back: those that were not ignored when `wb` started.
    let back: Vec<&str> = crate::interrupt::NAMED
        .into_iter()
        .filter(|(number, _)| !crate::interrupt::was_ignored(*number))
        .map(|(_, name)| name)
        .collect();
    let mut words = arguments_out(plan, layout, out);
    // Without `env` the signals cannot be given back to the session, and bubblewrap ignores none, as before #257.
    let ignore = back.is_empty() || Path::new(ENV).is_file();
    if ignore && !back.is_empty() {
        let at = words.len() - plan.words.len();
        words.splice(at..at, [OsString::from(ENV), OsString::from(format!("--default-signal={}", back.join(","))), OsString::from("--")]);
    }
    words.splice(0..0, [OsString::from("--info-fd"), OsString::from(info.to_string())]);
    let mut command = Command::new(bwrap);
    command.args(words).current_dir(plan.checkout);
    without_logins(&mut command);
    command.stdin(Stdio::inherit()).stdout(Stdio::inherit()).stderr(Stdio::inherit());
    // Only `signal(2)` and `fcntl(2)` run between fork and exec.
    unsafe {
        command.pre_exec(move || crate::interrupt::before_bubblewrap(info, ignore));
    }
    if plan.new_session {
        held.terminal_sent_on();
    }
    let spawned = command.spawn();
    drop(writer);
    let mut child = spawned.map_err(|error| format!("the session could not be started: {} could not be run: {}", bwrap.display(), error))?;
    let session = session_of(reader, child.id());
    match &session {
        Some(fd) => held.session_held(fd),
        // No session found: a termination and a hangup go to bubblewrap, which ignores them where it was started
        // so, and, with no controlling terminal, an interrupt or a quit kills bubblewrap, and the session with it
        // (#271). With one, the session gets those from the terminal.
        None => {
            held.session_lost(child.id());
            // A session that has ended already is not said to be lost, and bubblewrap's number, waited for now and
            // free for another, is let go at once.
            match ended_within(&mut child, ENDING) {
                Ok(None) => {
                    let more = if plan.new_session {
                        ", and a hangup does not reach it either; an interrupt or a quit (Ctrl-C, Ctrl-\\) ends bubblewrap and the session with it"
                    } else {
                        ""
                    };
                    let _ = writeln!(err, "{}: the session was not found in bubblewrap, so a termination of wb goes to bubblewrap, not to the session{}", name, more);
                }
                Ok(Some(_)) => held.ended(),
                Err(_) => {}
            }
        }
    }
    let status = match watch {
        Some(watch) => wait_watched(&mut child, watch, name, err),
        None => child.wait(),
    };
    held.ended();
    drop(session);
    let status = status.map_err(|error| format!("the session could not be waited for: {}", error))?;
    // bubblewrap gives a session ended by signal n as 128 + n.
    Ok(match status.code() {
        Some(code) if (129..=192).contains(&code) => ExitStatus::from_raw(code - 128),
        _ => status,
    })
}

/// Waits for bubblewrap as the watchdog (#267): it looks at the view now and then, says a quiet on `err`, and past
/// `max_minutes` says the limit, sends the termination on to the session as a termination of `wb` is (#221), and after
/// the grace kills the session and bubblewrap.
fn wait_watched(child: &mut std::process::Child, watch: &mut crate::watchdog::Watchdog, name: &str, err: &mut dyn Write) -> std::io::Result<ExitStatus> {
    use crate::watchdog::Event;
    let mut kill_at: Option<std::time::Instant> = None;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        match watch.look() {
            Some(Event::Quiet(minutes)) => {
                let _ = writeln!(
                    err,
                    "{}: quiet: nothing has changed in the view for {} {}. The session goes on; it is ended at {} minutes (max_minutes).",
                    name,
                    minutes,
                    if minutes == 1 { "minute" } else { "minutes" },
                    watch.limits().max_minutes
                );
            }
            Some(Event::Max) => {
                crate::refusal::limit(
                    err,
                    name,
                    format_args!(
                        "the session has run for {} minutes, the `max_minutes` of its role, and is ended as a termination of wb ends it: it is sent the termination, and killed if it has not ended {} seconds after. Its commits are taken back.",
                        watch.limits().max_minutes,
                        crate::watchdog::grace().as_secs()
                    ),
                );
                crate::interrupt::terminate_session();
                kill_at = Some(std::time::Instant::now() + crate::watchdog::grace());
            }
            None => {}
        }
        if kill_at.is_some_and(|at| std::time::Instant::now() >= at) {
            kill_at = None;
            crate::interrupt::kill_session();
            // bubblewrap itself, and with it what it runs (`--die-with-parent`).
            let _ = child.kill();
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// The time of the newest change in a view that a session runs in (#267), as `wb status` reads it: in seconds since
/// 1970, the newer of the time of the commit its `HEAD` names, where that is not the commit it was made with, and the
/// newest time of the entries under `paths`. The commit is read as `work_left` reads a view: through the checkout's
/// own repository with the view's objects as alternates, each git run within its bound; the entries by their metadata
/// alone. None where nothing was found.
pub fn last_change(root: &str, view: &View, paths: &[String]) -> Result<Option<u64>, String> {
    let _bound = bounded(test_seconds("WB_TEST_RUN_BOUND").unwrap_or(std::time::Duration::from_secs(3)));
    let _deadline = deadline(test_seconds("WB_TEST_VIEW_DEADLINE").unwrap_or(VIEW_DEADLINE), "read of the view");
    let head = head_of(&view.folder.join(".git"))?;
    let mut newest: Option<u64> = None;
    if head != view.base {
        let size: u64 = line(&on_view(root, view, &["cat-file", "-s", &head], None)?).parse().map_err(|_| format!("the size of the commit {} is not a number", short(&head)))?;
        if size > 64 * 1024 {
            return Err(format!("the commit {} is of {} bytes, and is not read", short(&head), size));
        }
        let bytes = on_view(root, view, &["cat-file", "commit", &head], None)?;
        let header = &bytes[..bytes.windows(2).position(|two| two == b"\n\n").unwrap_or(bytes.len())];
        let committed = String::from_utf8_lossy(header)
            .lines()
            .find_map(|line| line.strip_prefix("committer "))
            .and_then(|rest| rest.rsplit(' ').nth(1).and_then(|seconds| seconds.parse::<u64>().ok()))
            .ok_or_else(|| format!("the commit {} names no time it was committed", short(&head)))?;
        newest = Some(committed);
    }
    let (files, _) = crate::watchdog::files_of(&view.folder, paths);
    if let Some(seconds) = files.and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok()).map(|since| since.as_secs()) {
        newest = Some(newest.map_or(seconds, |before| before.max(seconds)));
    }
    Ok(newest)
}

/// `env` of the system, which the sandbox has read-only under `/usr`.
const ENV: &str = "/usr/bin/env";

/// How long `wb` waits for bubblewrap to name its child, and then for that child's own child, the session.
const FIND_SESSION: std::time::Duration = std::time::Duration::from_secs(2);

/// How long a child is given to be waited for after the info of bubblewrap ended with no session in it: a process
/// that ends closes its files, the info fd among them, before it can be waited for (#273).
const ENDING: std::time::Duration = std::time::Duration::from_millis(500);

/// `child.try_wait()`, repeated for `grace` while the child still runs.
fn ended_within(child: &mut std::process::Child, grace: std::time::Duration) -> std::io::Result<Option<ExitStatus>> {
    let end = std::time::Instant::now() + grace;
    loop {
        match child.try_wait() {
            Ok(None) if std::time::Instant::now() < end => std::thread::sleep(std::time::Duration::from_millis(5)),
            other => return other,
        }
    }
}

/// The session that bubblewrap at `bwrap` (its process number) started, held by a pidfd, or none (#257).
///
/// With `--unshare-pid` the `child-pid` of `--info-fd` is bubblewrap's own reaper, process 1 of the sandbox, and
/// the session is its one child. Each step is checked before the next: the reaper's parent is `bwrap`, it has one
/// child, that child's parent is the reaper, and it still is after the pidfd is open, so that the pidfd is of
/// the session and of no other process. The info is read for 2 seconds and 4 KiB at most.
fn session_of(reader: std::io::PipeReader, bwrap: u32) -> Option<std::os::fd::OwnedFd> {
    use std::io::Read;
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = reader;
        let mut info = Vec::new();
        let mut buffer = [0u8; 512];
        while info.len() < 4096 && !info.contains(&b'}') {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => info.extend_from_slice(&buffer[..count]),
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        let _ = send.send(info);
    });
    let info = receive.recv_timeout(FIND_SESSION).ok()?;
    let reaper = child_pid(&info)?;
    if parent_of(reaper)? != bwrap as i32 {
        return None;
    }
    let end = std::time::Instant::now() + FIND_SESSION;
    let session = loop {
        let children = children_of(reaper)?;
        match children.as_slice() {
            [one] => break *one,
            [] if std::time::Instant::now() < end => std::thread::sleep(std::time::Duration::from_millis(5)),
            _ => return None,
        }
    };
    if parent_of(session)? != reaper {
        return None;
    }
    let fd = crate::interrupt::pidfd(session)?;
    // The parent is checked again, and the pidfd's process found alive: a live process keeps its number, so the
    // process whose parent was checked is the pidfd's own.
    if parent_of(session)? != reaper || !crate::interrupt::alive(&fd) {
        return None;
    }
    Some(fd)
}

/// The number after `"child-pid":` in bubblewrap's info, when it is a process other than the first.
fn child_pid(info: &[u8]) -> Option<i32> {
    let text = std::str::from_utf8(info).ok()?;
    let rest = text[text.find("\"child-pid\"")? + "\"child-pid\"".len()..].trim_start().strip_prefix(':')?.trim_start();
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || digits.len() > 10 {
        return None;
    }
    digits.parse::<i32>().ok().filter(|pid| *pid > 1)
}

/// A file of `/proc`, 4 KiB at most.
fn proc_file(path: &str) -> Option<String> {
    use std::io::Read;
    let mut text = String::new();
    fs::File::open(path).ok()?.take(4096).read_to_string(&mut text).ok()?;
    Some(text)
}

/// The parent of process `pid`, from `/proc/<pid>/stat`, unless it has ended (a zombie).
fn parent_of(pid: i32) -> Option<i32> {
    let stat = proc_file(&format!("/proc/{}/stat", pid))?;
    // The name of the program, in brackets, may hold anything: the fields after it are read.
    let mut fields = stat[stat.rfind(')')? + 1..].split_whitespace();
    if fields.next()? == "Z" {
        return None;
    }
    fields.next()?.parse().ok()
}

/// The children of process `pid` (of its first thread), from `/proc`.
fn children_of(pid: i32) -> Option<Vec<i32>> {
    let text = proc_file(&format!("/proc/{}/task/{}/children", pid, pid))?;
    text.split_whitespace().map(|word| word.parse().ok()).collect()
}

/// What was done with the commits of a view.
#[derive(Debug, Default)]
pub struct Back {
    /// Each commit put on the branch: its name there and its subject.
    pub taken: Vec<(String, String)>,
    /// The commit of the view that each of `taken` was made from, in the same order (#277).
    pub from: Vec<String>,
    /// The commit refused, its subject and why.
    pub refused: Option<(String, String, String)>,
    /// The commits after the one refused, not taken: each its name in the view and its subject.
    pub after: Vec<(String, String)>,
    /// What `wb` passed over in reading the rules, said on standard error.
    pub notes: Vec<String>,
    /// The paths left uncommitted in the view.
    pub uncommitted: Vec<String>,
    /// Why nothing of the view could be taken back, when it could not.
    pub failed: Option<String>,
}

impl Back {
    /// Whether all of the view's work is on the branch: nothing refused, failed or left uncommitted.
    pub fn all_taken(&self) -> bool {
        self.refused.is_none() && self.after.is_empty() && self.uncommitted.is_empty() && self.failed.is_none()
    }

    /// Whether anything was refused, or could not be taken back: `wb` then ends with 1.
    pub fn refused_any(&self) -> bool {
        self.refused.is_some() || self.failed.is_some()
    }

    /// The file `back` of the session's folder, with the last line `view: removed` or `view: kept <folder>`.
    pub fn text(&self, kept: Option<&Path>) -> String {
        let mut text = String::new();
        for (at, (commit, subject)) in self.taken.iter().enumerate() {
            text.push_str(&format!("taken: {} {}\n", commit, subject));
            if let Some(from) = self.from.get(at) {
                text.push_str(&format!("took: {} {}\n", from, commit));
            }
        }
        if let Some((commit, subject, why)) = &self.refused {
            text.push_str(&format!("refused: {} {}: {}\n", commit, subject, why));
        }
        for (commit, subject) in &self.after {
            text.push_str(&format!("not taken: {} {}\n", commit, subject));
        }
        for path in &self.uncommitted {
            text.push_str(&format!("uncommitted: {}\n", crate::issue::shown(path)));
        }
        if let Some(why) = &self.failed {
            text.push_str(&format!("failed: {}\n", crate::issue::shown(why)));
        }
        match kept {
            Some(folder) => text.push_str(&format!("view: kept {}\n", folder.display())),
            None => text.push_str("view: removed\n"),
        }
        text
    }
}

/// A name of an object: 40 or 64 small hexadecimal digits.
pub(crate) fn is_object(name: &str) -> bool {
    (name.len() == 40 || name.len() == 64) && name.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// The most a small file of a view or of the lines of a start is read.
const MOST_BYTES: u64 = crate::bounded::TEXT_CAP;

/// A small file, read as it is by the one reader (`bounded::read_plain`): a plain file of 1 MiB or less, never
/// through a link, never waiting. What is not, or is not text, is none.
pub(crate) fn small_file(path: &Path) -> Option<String> {
    String::from_utf8(crate::bounded::read_plain(path, MOST_BYTES).ok()?).ok()
}

/// The commit the view's `HEAD` names, read from the files of its `.git`, without git: a branch, loose or packed,
/// or a commit named in `HEAD` itself.
pub(crate) fn head_of(git_dir: &Path) -> Result<String, String> {
    let none = || "the view's HEAD names no commit".to_string();
    let head = small_file(&git_dir.join("HEAD")).ok_or_else(none)?;
    let head = head.trim_end();
    let Some(name) = head.strip_prefix("ref: ") else {
        return if is_object(head) { Ok(head.to_string()) } else { Err(none()) };
    };
    let plain = name.starts_with("refs/heads/") && name.split('/').all(|part| !part.is_empty() && !part.starts_with('.') && !part.contains(".."));
    if !plain {
        return Err(none());
    }
    if let Some(loose) = small_file(&git_dir.join(name)) {
        let loose = loose.trim();
        return if is_object(loose) { Ok(loose.to_string()) } else { Err(none()) };
    }
    let packed = small_file(&git_dir.join("packed-refs")).unwrap_or_default();
    packed
        .lines()
        .find_map(|line| line.split_once(' ').filter(|(_, found)| *found == name).map(|(object, _)| object.to_string()))
        .filter(|object| is_object(object))
        .ok_or_else(none)
}

/// One change of a commit, as `diff-tree --raw` gives it.
struct Change {
    mode: String,
    object: String,
    path: String,
}

/// What a commit of the view changes, against its parent.
fn changes_of(store: &Path, parent: &str, commit: &str) -> Result<Vec<Change>, String> {
    let raw = git(store, &["diff-tree", "-r", "-z", "--no-renames", "--raw", parent, commit], &[], None)?;
    let mut found = Vec::new();
    let mut fields = raw.split(|byte| *byte == 0).filter(|field| !field.is_empty());
    while let (Some(head), Some(path)) = (fields.next(), fields.next()) {
        let head = String::from_utf8_lossy(head);
        let words: Vec<&str> = head.trim_start_matches(':').split(' ').collect();
        let [_, mode, _, object, _] = words.as_slice() else {
            return Err(format!("git said a change of {} in a form not known: {}", commit, head));
        };
        let path = String::from_utf8(path.to_vec()).map_err(|_| format!("a path of the commit {} is not UTF-8 text", commit))?;
        found.push(Change { mode: mode.to_string(), object: object.to_string(), path });
    }
    Ok(found)
}

/// Characters HFS+ ignores in a name, so that `.g<one of them>it` is `.git` there (git's `is_hfs_dotgit`).
fn hfs_ignorable(c: char) -> bool {
    matches!(c, '\u{200c}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{206a}'..='\u{206f}' | '\u{feff}')
}

/// What follows a name NTFS reads as the name: nothing, dots and spaces, and then nothing or a stream (`:`).
fn ntfs_tail(rest: &str) -> bool {
    let rest = rest.trim_start_matches([' ', '.']);
    rest.is_empty() || rest.starts_with(':')
}

/// Whether `lower` (in lower case) is the NTFS short name of a file called `.<dot>`, as git's
/// `is_ntfs_dot_generic` has it: the first 6 letters of `dot` and `~1` to `~4`, or the fall-back name, 8
/// characters with the first of `prefix` (6 letters), a `~` and digits, which NTFS gives when the short names
/// of the first kind are taken.
fn ntfs_short(lower: &str, dot: &str, prefix: &str) -> bool {
    let bytes = lower.as_bytes();
    if lower.len() >= 8 && dot.len() >= 6 && lower.is_char_boundary(6) && lower[..6] == dot[..6] && bytes[6] == b'~' && (b'1'..=b'4').contains(&bytes[7]) {
        return lower.is_char_boundary(8) && ntfs_tail(&lower[8..]);
    }
    let prefix = prefix.as_bytes();
    let mut tilde = false;
    let mut at = 0;
    while at < 8 {
        let Some(&c) = bytes.get(at) else { return false };
        if tilde {
            if !c.is_ascii_digit() {
                return false;
            }
        } else if c == b'~' {
            at += 1;
            if !matches!(bytes.get(at), Some(b'1'..=b'9')) {
                return false;
            }
            tilde = true;
        } else if at >= 6 || c != prefix[at] {
            return false;
        }
        at += 1;
    }
    ntfs_tail(&lower[8..])
}

/// Whether `name` is `.<dot>` in a form git refuses: in any case, with the characters HFS+ ignores, as NTFS has
/// it (trailing dots and spaces, a stream after a colon), or as its short name. `short` is the prefix of the
/// fall-back short names, or none where only the name itself counts.
fn is_dot(name: &str, dot: &str, short: Option<&str>) -> bool {
    let plain: String = name.chars().filter(|c| !hfs_ignorable(*c)).collect::<String>().to_ascii_lowercase();
    if plain.strip_prefix('.') == Some(dot) {
        return true;
    }
    let lower = name.to_ascii_lowercase();
    if lower.strip_prefix('.').and_then(|rest| rest.strip_prefix(dot)).is_some_and(ntfs_tail) {
        return true;
    }
    short.is_some_and(|prefix| ntfs_short(&lower, dot, prefix))
}

/// Why git does not take `path` as it is in a commit, if it does not: the rule of git's own `verify_path`, as it
/// is when `core.protectNTFS` and `core.protectHFS` are on, which `wb` holds for every checkout. An empty part,
/// `.` and `..`; a part `.git` in any case, or in a form HFS+ or NTFS reads as it; and `.gitmodules` as a link
/// (`mode` is that of the entry). The backslash separates parts, as it does for NTFS.
pub fn refused_by_git(path: &str, mode: &str) -> Option<String> {
    for part in path.split(['/', '\\']) {
        if part.is_empty() || part == "." || part == ".." {
            return Some("it has an empty part, `.` or `..`".to_string());
        }
        if is_dot(part, "git", None) || part.to_ascii_lowercase().strip_prefix("git~1").is_some_and(ntfs_tail) {
            return Some("it has a part .git".to_string());
        }
        if mode == "120000" && is_dot(part, "gitmodules", Some("gi7eba")) {
            return Some("it is a link named .gitmodules".to_string());
        }
    }
    None
}

/// What an entry of `changes` covers: its parts, and whether it is a folder (every path below it) or a file
/// (that path only).
pub struct Covered {
    parts: Vec<String>,
    folder: bool,
}

/// Reads the entries of `changes` as the checkout's `head` has them (the trusted side, never the view): `head` is
/// asked first. An entry that is a folder there is a folder; a file (or a link, or a submodule) there is a file,
/// also when it is written with a `/`. For an entry that is not there, the `/` at its end decides: with it a
/// folder, without it a file.
pub fn covered(root: &str, head: &str, changes: &[String]) -> Result<Vec<Covered>, String> {
    let mut found = Vec::new();
    for entry in changes {
        let names: Vec<String> = parts(entry).into_iter().map(str::to_string).collect();
        if names.is_empty() {
            continue;
        }
        let wanted = names.join("/");
        let args = ["ls-tree", "-z", "--full-tree", head, "--", wanted.as_str()];
        let listed = git(Path::new(root), &args, &[("GIT_LITERAL_PATHSPECS", Path::new("1"))], None)?;
        let text = String::from_utf8_lossy(&listed);
        let there = text.split('\0').find_map(|line| line.split_once('\t').filter(|(_, path)| *path == wanted).map(|(meta, _)| meta.split(' ').nth(1) == Some("tree")));
        found.push(Covered { parts: names, folder: there.unwrap_or_else(|| entry.ends_with('/')) });
    }
    Ok(found)
}

/// What is wrong with the raw bytes of a tree, if anything: an entry in a mode git does not write (`100664` is
/// rewritten as `100644`), an empty name or one with a `/`, a name twice, or entries not in the order git sorts
/// them (by name, a folder as if its name ended with `/`). Such a tree is not the one git would write from its
/// entries. `width` is the length of an object's name in bytes.
fn tree_fault(bytes: &[u8], width: usize) -> Option<String> {
    let mut names: std::collections::HashSet<Vec<u8>> = std::collections::HashSet::new();
    let mut last: Option<Vec<u8>> = None;
    let mut rest = bytes;
    while !rest.is_empty() {
        let Some(space) = rest.iter().position(|byte| *byte == b' ') else { return Some("an entry is cut short".to_string()) };
        let (mode, after) = (&rest[..space], &rest[space + 1..]);
        let Some(nul) = after.iter().position(|byte| *byte == 0) else { return Some("an entry is cut short".to_string()) };
        let (name, after) = (&after[..nul], &after[nul + 1..]);
        if after.len() < width {
            return Some("an entry is cut short".to_string());
        }
        rest = &after[width..];
        let folder = mode == b"40000";
        if ![&b"100644"[..], b"100755", b"120000", b"40000", b"160000"].contains(&mode) {
            return Some(format!("the mode {} of {} is one git rewrites", String::from_utf8_lossy(mode), crate::issue::shown(&String::from_utf8_lossy(name))));
        }
        if name.is_empty() || name.contains(&b'/') {
            return Some(format!("{} is not a name of a tree's entry", crate::issue::shown(&String::from_utf8_lossy(name))));
        }
        if !names.insert(name.to_vec()) {
            return Some(format!("the name {} is there twice", crate::issue::shown(&String::from_utf8_lossy(name))));
        }
        let mut key = name.to_vec();
        key.push(if folder { b'/' } else { 0 });
        if last.as_ref().is_some_and(|before| *before >= key) {
            return Some(format!("{} is not in the order git sorts the entries", crate::issue::shown(&String::from_utf8_lossy(name))));
        }
        last = Some(key);
    }
    None
}

/// Why the trees that `commit` of the view brings, against `parent`, are not in the form git writes, if they are
/// not (see `tree_fault`). The trees are read through `wb`'s own store, never with the view's settings. Without
/// it the commit `wb` puts together from the changes would not be the one the session made, and `diff-tree`
/// does not always show it.
fn not_standard(store: &Path, parent: &str, commit: &str) -> Result<Option<String>, String> {
    let listed = git(store, &["rev-list", "--objects", commit, &format!("^{}", parent)], &[], None)?;
    let names: Vec<u8> = String::from_utf8_lossy(&listed).lines().filter_map(|line| line.split(' ').next()).flat_map(|id| format!("{}\n", id).into_bytes()).collect();
    let kinds = git(store, &["cat-file", "--batch-check=%(objectname) %(objecttype)"], &[], Some(&names))?;
    let trees: Vec<u8> = String::from_utf8_lossy(&kinds)
        .lines()
        .filter_map(|line| line.split_once(' ').filter(|(_, kind)| *kind == "tree").map(|(id, _)| format!("{}\n", id)))
        .flat_map(String::into_bytes)
        .collect();
    if trees.is_empty() {
        return Ok(None);
    }
    let width = commit.len() / 2;
    let all = git(store, &["cat-file", "--batch"], &[], Some(&trees))?;
    let mut rest = all.as_slice();
    while !rest.is_empty() {
        let end = rest.iter().position(|byte| *byte == b'\n').ok_or("git said a tree in a form not known")?;
        let head = String::from_utf8_lossy(&rest[..end]).to_string();
        let words: Vec<&str> = head.split(' ').collect();
        let size: usize = words.get(2).and_then(|size| size.parse().ok()).ok_or("git said a tree in a form not known")?;
        let body = rest.get(end + 1..end + 1 + size).ok_or("git said a tree in a form not known")?;
        if let Some(fault) = tree_fault(body, width) {
            return Ok(Some(format!("a tree of it, {}, is not in the form git writes: {}", short(words[0]), fault)));
        }
        rest = rest.get(end + 2 + size..).unwrap_or_default();
    }
    Ok(None)
}

/// What may come back from a view: the paths of `changes` of an issue's file; for the planner, its issue's file
/// alone; for a role with no issue's file, any path (#202). Never a path under `.watchbill/` but the planner's
/// issue's file, never one under `build_files`, never one git does not take.
#[derive(Clone, Debug, PartialEq)]
pub enum Allowed {
    Changes(Vec<String>),
    /// `.watchbill/issues/<issue>.yml`, as a plain file.
    IssueFile(String),
    Whole,
}

/// `Allowed`, as the checkout's `HEAD` reads it.
enum Rule {
    Changes(Vec<Covered>),
    IssueFile(String),
    Whole,
}

/// Why a path that a commit changes is not taken, if it is not: for the planner, any path but its issue's file,
/// and that file as anything but a plain file; otherwise under `.watchbill/`, a file of the build, a path git does
/// not take, or outside `changes`.
fn not_allowed(path: &str, mode: &str, rule: &Rule, build_files: &[String]) -> Option<String> {
    let found = parts(path);
    if let Rule::IssueFile(file) = rule {
        if found != parts(file) {
            return Some(format!("{} is not {}, the one file the planner's work comes back as", crate::issue::shown(path), file));
        }
        if mode == "000000" {
            return Some(format!("{} is deleted, and the planner's work comes back only as that file, written", crate::issue::shown(path)));
        }
        if mode != "100644" {
            return Some(format!("{} comes back only as a plain file", crate::issue::shown(path)));
        }
    } else if found.first() == Some(&".watchbill") {
        return Some(format!("{} is under .watchbill/", crate::issue::shown(path)));
    }
    if let Some(build) = build_files.iter().find(|build| crate::starter::meet(path, build)) {
        return Some(format!("{} is under `build_files` of watchbill.yml: {}", crate::issue::shown(path), build));
    }
    if let Some(why) = refused_by_git(path, mode) {
        return Some(format!("{} is a path git does not take: {}", crate::issue::shown(path), why));
    }
    let Rule::Changes(changes) = rule else { return None };
    let inside = changes.iter().any(|entry| {
        let n = entry.parts.len();
        found.len() >= n && found[..n].iter().zip(&entry.parts).all(|(a, b)| a == b) && (entry.folder || found.len() == n)
    });
    if !inside {
        return Some(format!("{} is outside `changes`", crate::issue::shown(path)));
    }
    None
}

/// The first of `path` and its leading parts that the working tree of the checkout at `root` holds untracked, ignored
/// or not, if any: the path itself, there as anything at all, or a leading part that is there as anything but a
/// folder (a file, a link), and that is not in its `head`. Each part is looked at, never followed: a link is a link.
/// The fast-forward would write over it, or delete it to make a folder.
fn untracked_in_checkout(root: &str, head: &str, path: &str) -> Result<Option<String>, String> {
    let parts = parts(path);
    let mut at = PathBuf::from(root);
    for (index, part) in parts.iter().enumerate() {
        at.push(part);
        let last = index + 1 == parts.len();
        let in_the_way = match fs::symlink_metadata(&at) {
            Err(_) => return Ok(None),
            Ok(found) => last || !found.file_type().is_dir(),
        };
        if in_the_way {
            let shown = parts[..=index].join("/");
            let listed = git(Path::new(root), &["ls-tree", "-z", "--full-tree", head, "--", &shown], &[("GIT_LITERAL_PATHSPECS", Path::new("1"))], None)?;
            return Ok(listed.is_empty().then_some(shown));
        }
    }
    Ok(None)
}

/// A commit of the view as git keeps it: its author, as `name`, `email` and `date`, and its message.
struct Raw {
    author: (String, String, String),
    message: Vec<u8>,
}

impl Raw {
    fn subject(&self) -> String {
        crate::issue::shown(String::from_utf8_lossy(&self.message).lines().next().unwrap_or(""))
    }
}

fn raw_of(store: &Path, commit: &str) -> Result<Raw, String> {
    let bytes = git(store, &["cat-file", "commit", commit], &[], None)?;
    let split = bytes.windows(2).position(|two| two == b"\n\n").map(|at| at + 2).unwrap_or(bytes.len());
    let head = String::from_utf8_lossy(&bytes[..split]);
    let author = head.lines().find_map(|line| line.strip_prefix("author ")).ok_or_else(|| format!("the commit {} has no author", commit))?;
    let (name, rest) = author.split_once(" <").ok_or_else(|| format!("the author of {} is not in its form", commit))?;
    let (email, date) = rest.split_once("> ").ok_or_else(|| format!("the author of {} is not in its form", commit))?;
    Ok(Raw { author: (name.to_string(), email.to_string(), date.to_string()), message: bytes[split..].to_vec() })
}

/// A short name of a commit, as a person reads it.
pub(crate) fn short(commit: &str) -> &str {
    &commit[..commit.len().min(12)]
}

/// Takes the work of the session back from `view` to the checkout at `root`: each commit, in order, while it
/// keeps to what is `allowed`, then the paths left uncommitted. A commit that does not is refused, and none after it is
/// taken. What is said goes to `err`, after `name`; `list` is where the whole list of paths left is written. `at` is
/// the commit of the view's branch when the view was made, or none where it was not there, as `make_onto` gave it:
/// the branch is found so, or nothing is taken (#296).
#[allow(clippy::too_many_arguments)]
pub fn take_back(name: &str, root: &str, view: &View, at: Option<&str>, allowed: &Allowed, build_files: &[String], list: Option<&Path>, err: &mut dyn Write) -> Back {
    take_back_in(name, root, view, allowed, build_files, list, Taken::AtTheEnd(at.map(str::to_string)), err)
}

/// When a take-back is made, and what it finds the branch at.
enum Taken {
    /// At the end of the session: the branch is as the view was made with it, at this commit or not there.
    AtTheEnd(Option<String>),
    /// After the session (#223), the number of commits that was said.
    Later(usize, Vec<String>),
}

impl Taken {
    fn later(&self) -> Option<usize> {
        match self {
            Taken::Later(count, _) => Some(*count),
            Taken::AtTheEnd(_) => None,
        }
    }

    /// The commits of the view that an earlier take-back put on the branch (#277).
    fn already(&self) -> &[String] {
        match self {
            Taken::Later(_, already) => already,
            Taken::AtTheEnd(_) => &[],
        }
    }
}

/// The time a whole take-back may take, besides the 60 seconds of each git run.
const TAKE_BACK_DEADLINE: std::time::Duration = std::time::Duration::from_secs(300);

/// The same for a view kept after its session (#223), by a person who runs `wb start <role> <issue> --take-back`:
/// the checkout's `HEAD` need not be the commit the view was made from. It may be a descendant of it whose changes
/// touch none of the paths that a commit of the view changes; the commits then go on top of it. The branch must be
/// the view's. Every git run ends within a bound, for a view is hostile.
pub fn take_back_later(name: &str, root: &str, view: &View, changes: &[String], build_files: &[String], expected: usize, list: Option<&Path>, err: &mut dyn Write) -> Back {
    take_back_later_of(name, root, view, &Allowed::Changes(changes.to_vec()), build_files, expected, &[], list, err)
}

/// The same, with what may come back as `allowed` (#202), and the commits of the view that an earlier take-back put on
/// the branch, which are not taken again (#277).
#[allow(clippy::too_many_arguments)]
pub fn take_back_later_of(name: &str, root: &str, view: &View, allowed: &Allowed, build_files: &[String], expected: usize, already: &[String], list: Option<&Path>, err: &mut dyn Write) -> Back {
    let _bound = bounded(std::time::Duration::from_secs(60));
    let _deadline = deadline(TAKE_BACK_DEADLINE, "take-back");
    take_back_in(name, root, view, allowed, build_files, list, Taken::Later(expected, already.to_vec()), err)
}

#[allow(clippy::too_many_arguments)]
fn take_back_in(name: &str, root: &str, view: &View, allowed: &Allowed, build_files: &[String], list: Option<&Path>, taken: Taken, err: &mut dyn Write) -> Back {
    let mut back = Back::default();
    let store = match Store::new(view) {
        Ok(store) => store,
        Err(why) => {
            back.failed = Some(why);
            return said(name, view, back, list, err);
        }
    };
    if let Err(why) = commits(root, view, &store.path, allowed, build_files, &taken, &mut back) {
        back.failed = Some(why);
    }
    match uncommitted(root, &store.path, &view.folder, &view.head, &mut back.notes) {
        // What the planner leaves beside its issue's file can never come back: it stays in the view, and goes with
        // it (#295).
        Ok(found) => match allowed {
            Allowed::IssueFile(file) => back.uncommitted = found.into_iter().filter(|path| parts(path) == parts(file)).collect(),
            _ => back.uncommitted = found,
        },
        Err(why) => {
            if back.failed.is_none() {
                back.failed = Some(format!("what is left uncommitted in the view could not be read: {}", why));
            }
        }
    }
    drop(store);
    said(name, view, back, list, err)
}

/// What a kept view holds that is not on the branch (#222), read as `take_back` reads a view and through a
/// repository of `wb`'s own, never with the settings of the view: the commits of the view after the one it was made
/// with (the first 200, the first parent only), each its name and subject, and the paths left uncommitted. Reads only.
/// A commit named in `taken` was put on the branch by a take-back, and is not listed (#277).
pub fn work_left(root: &str, view: &View, taken: &[String]) -> Result<Left, String> {
    // Every git run here ends after 3 seconds: a view is hostile, and git may wait for ever on what it holds. All of
    // them end within one deadline for the view (#265).
    let _bound = bounded(test_seconds("WB_TEST_RUN_BOUND").unwrap_or(std::time::Duration::from_secs(3)));
    let _deadline = deadline(test_seconds("WB_TEST_VIEW_DEADLINE").unwrap_or(VIEW_DEADLINE), "read of the view");
    let objects = view.folder.join(".git").join("objects");
    match fs::symlink_metadata(&objects) {
        Ok(found) if found.is_dir() => {}
        _ => return Err(format!("{} is not the folder it was made as, and nothing of the view is read through it", objects.display())),
    }
    let head = head_of(&view.folder.join(".git"))?;
    // The commits are read through the checkout's own repository, with the view's objects as alternates: nothing is
    // written, and no setting of the view is read.
    let gitdir = line(&git(Path::new(root), &["rev-parse", "--absolute-git-dir"], &[], None)?);
    let env = [
        ("GIT_ALTERNATE_OBJECT_DIRECTORIES", objects.as_path()),
        ("GIT_CONFIG_NOSYSTEM", Path::new("1")),
        ("GIT_CONFIG_GLOBAL", Path::new("/dev/null")),
        ("GIT_ATTR_NOSYSTEM", Path::new("1")),
    ];
    let before = [OsString::from("--no-replace-objects"), OsString::from(format!("--git-dir={}", gitdir))];
    let on = |args: &[&str]| run_git(None, &before, args, &env, None);
    let mut commits: Vec<(String, String)> = Vec::new();
    let mut more = 0;
    if head != view.base {
        // Git ends with 1 where the commit is not an ancestor; any other end (128 for an object it cannot read, a
        // time-out) is said as what it is.
        let ran = run_git_status(None, &before, &["merge-base", "--is-ancestor", &view.base, &head], &env, None)?;
        match ran.code {
            0 => {}
            1 => return Err(format!("the view's HEAD, {}, does not come from the commit the view was made with", short(&head))),
            _ => return Err(ran.said),
        }
        let range = format!("{}..{}", view.base, head);
        let listed = on(&["rev-list", "--first-parent", "--reverse", &range])?;
        let listed = String::from_utf8_lossy(&listed);
        let all: Vec<&str> = listed.lines().filter(|commit| is_object(commit) && !taken.iter().any(|done| done == commit)).collect();
        more = all.len().saturating_sub(MOST_COMMITS);
        for commit in all.into_iter().take(MOST_COMMITS) {
            commits.push((commit.to_string(), subject_capped(&on, commit)?));
        }
    }
    let store = Store::new(view)?;
    let mut notes = Vec::new();
    let uncommitted = uncommitted(root, &store.path, &view.folder, &view.head, &mut notes)?;
    Ok(Left { commits, more, uncommitted })
}

/// A time in whole seconds that a test sets by the variable `name`, so that a test does not race a bound of `wb`
/// (#279). It is read only by a build with debug assertions, which `cargo test` makes; `cargo build --release`, as the
/// RPM builds `wb`, has no code that reads it, so no value changes `wb` as installed.
#[cfg(debug_assertions)]
fn test_seconds(name: &str) -> Option<std::time::Duration> {
    std::env::var(name).ok()?.parse().ok().filter(|seconds| *seconds > 0).map(std::time::Duration::from_secs)
}

#[cfg(not(debug_assertions))]
fn test_seconds(_name: &str) -> Option<std::time::Duration> {
    None
}

/// The time `wb` may spend on reading one kept view, besides the bound of each git run (#265).
const VIEW_DEADLINE: std::time::Duration = std::time::Duration::from_secs(20);

/// The most commits of a kept view that are named.
const MOST_COMMITS: usize = 200;

/// What `work_left` found.
#[derive(Debug)]
pub struct Left {
    /// The first commits of the view not taken, each its name and subject.
    pub commits: Vec<(String, String)>,
    /// How many more commits there are than are named.
    pub more: usize,
    /// The paths left uncommitted.
    pub uncommitted: Vec<String>,
}

/// A git run on the objects of a view, as `work_left` makes it: through the checkout's own repository with the view's
/// objects as alternates, and no setting read. The caller holds the bound.
fn on_view(root: &str, view: &View, args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let objects = view.folder.join(".git").join("objects");
    match fs::symlink_metadata(&objects) {
        Ok(found) if found.is_dir() => {}
        _ => return Err(format!("{} is not the folder it was made as, and nothing of the view is read through it", objects.display())),
    }
    let gitdir = line(&git(Path::new(root), &["rev-parse", "--absolute-git-dir"], &[], None)?);
    let env = [
        ("GIT_ALTERNATE_OBJECT_DIRECTORIES", objects.as_path()),
        ("GIT_CONFIG_NOSYSTEM", Path::new("1")),
        ("GIT_CONFIG_GLOBAL", Path::new("/dev/null")),
        ("GIT_ATTR_NOSYSTEM", Path::new("1")),
    ];
    let before = [OsString::from("--no-replace-objects"), OsString::from(format!("--git-dir={}", gitdir))];
    run_git(None, &before, args, &env, input)
}

/// Reads the header of every commit, tree and file the view's commits add, before a take-back starts, so that an
/// object git would wait for fails here, within 3 seconds per run, and not later in the take (#223).
pub fn objects_readable(root: &str, view: &View) -> Result<(), String> {
    let _bound = bounded(test_seconds("WB_TEST_RUN_BOUND").unwrap_or(std::time::Duration::from_secs(3)));
    let _deadline = deadline(test_seconds("WB_TEST_VIEW_DEADLINE").unwrap_or(VIEW_DEADLINE), "read of the view");
    let head = head_of(&view.folder.join(".git"))?;
    if head == view.base {
        return Ok(());
    }
    let listed = on_view(root, view, &["rev-list", "--objects", "--first-parent", &format!("{}..{}", view.base, head)], None)?;
    let names: Vec<u8> = String::from_utf8_lossy(&listed)
        .lines()
        .filter_map(|entry| entry.split(' ').next().filter(|name| is_object(name)))
        .flat_map(|name| format!("{}\n", name).into_bytes())
        .collect();
    // The header is read where the take reads it, in `wb`'s own store with the view as its only alternate: an object
    // the checkout holds too is checked from the view's copy, which is the one the take reads (#265).
    let store = Store::new(view)?;
    let checked = git(&store.path, &["cat-file", "--batch-check"], &[], Some(&names))?;
    if String::from_utf8_lossy(&checked).lines().any(|entry| entry.ends_with(" missing")) {
        return Err("an object of the view's commits is missing".to_string());
    }
    Ok(())
}

/// The subject of a commit of a view, read with a cap: a message over 64 KiB is not read, and a subject is cut at
/// 200 characters.
fn subject_capped(on: &dyn Fn(&[&str]) -> Result<Vec<u8>, String>, commit: &str) -> Result<String, String> {
    let size: u64 = line(&on(&["cat-file", "-s", commit])?).parse().map_err(|_| format!("the size of the commit {} is not a number", short(commit)))?;
    if size > 64 * 1024 {
        return Ok(format!("(a commit of {} bytes, not read)", size));
    }
    let bytes = on(&["cat-file", "commit", commit])?;
    let split = bytes.windows(2).position(|two| two == b"\n\n").map(|at| at + 2).unwrap_or(bytes.len());
    let first = String::from_utf8_lossy(&bytes[split..]).lines().next().unwrap_or("").chars().take(200).collect::<String>();
    Ok(crate::issue::shown(&first))
}

/// A repository of `wb`'s own, through which the objects of a view are read, and removed when it is dropped.
struct Store {
    path: PathBuf,
}

impl Store {
    fn new(view: &View) -> Result<Store, String> {
        let git_dir = view.folder.join(".git");
        let objects = git_dir.join("objects");
        for folder in [&git_dir, &objects] {
            match fs::symlink_metadata(folder) {
                Ok(found) if found.is_dir() => {}
                _ => return Err(format!("{} is not the folder it was made as, and nothing of the view is read through it", folder.display())),
            }
        }
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|since| since.subsec_nanos()).unwrap_or(0);
        let path = new_folder(&std::env::temp_dir(), &format!("wb-view-{}-{}", std::process::id(), nanos))?;
        let store = Store { path };
        git(&store.path, &["init", "-q", "--bare", "--template=", "."], &[], None)?;
        let alternates = store.path.join("objects/info");
        fs::create_dir_all(&alternates).map_err(|error| error.to_string())?;
        let line = format!("{}\n", objects.display());
        fs::write(alternates.join("alternates"), line).map_err(|error| error.to_string())?;
        Ok(store)
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        let _ = remove(&self.path);
    }
}

/// Takes the commits of the view back onto the branch, as far as they keep to the lists. The error is why none
/// could be taken.
fn commits(root: &str, view: &View, store: &Path, allowed: &Allowed, build_files: &[String], taken: &Taken, back: &mut Back) -> Result<(), String> {
    let later = taken.later();
    let head = head_of(&view.folder.join(".git"))?;
    if head == view.base {
        return Ok(());
    }
    // The commits go onto the stamp's branch, whichever branch is checked out (#296). At the end of a session it is
    // as it was when the view was made; after the session (#223) it is as it is now. Where it is not there, it is
    // made at the commit the view was made from. Where it is there, its commit is that one or a descendant of it, and
    // the commits go on top of it once their paths are seen to be untouched.
    let expected = match taken {
        Taken::AtTheEnd(at) => at.clone(),
        Taken::Later(..) => tip_of(root, &view.branch)?,
    };
    let onto = expected.clone().unwrap_or_else(|| view.head.clone());
    if onto != view.head && git(Path::new(root), &["merge-base", "--is-ancestor", &view.head, &onto], &[], None).is_err() {
        // Behind the commit the view was made from, or gone apart from it: said as what it is (#298).
        let behind = git(Path::new(root), &["merge-base", "--is-ancestor", &onto, &view.head], &[], None).is_ok();
        let how = if behind { "is behind" } else { "has gone apart from" };
        return Err(format!(
            "the branch {} is at {}, which {} {}, the commit the view was made from, so nothing is taken back",
            view.branch_name(),
            short(&onto),
            how,
            short(&view.head)
        ));
    }
    let moved;
    let (view, origin) = if onto != view.head {
        moved = View { head: onto, ..view.clone() };
        (&moved, Some(view.head.clone()))
    } else {
        (view, None)
    };
    let rule = match allowed {
        Allowed::Changes(changes) => Rule::Changes(covered(root, &view.head, changes)?),
        Allowed::IssueFile(file) => Rule::IssueFile(file.clone()),
        Allowed::Whole => Rule::Whole,
    };
    if git(store, &["merge-base", "--is-ancestor", &view.base, &head], &[], None).is_err() {
        return Err(format!("the view's HEAD, {}, does not come from the commit the view was made with", short(&head)));
    }
    let listed = git(store, &["rev-list", "--first-parent", "--reverse", "--parents", &format!("{}..{}", view.base, head)], &[], None)?;
    let listed = String::from_utf8_lossy(&listed).to_string();
    // The commits that an earlier take-back put on the branch come first, in a row, and are not taken again (#277).
    let mut parent = view.base.clone();
    let mut skipped = 0;
    for line in listed.lines() {
        let words: Vec<&str> = line.split(' ').collect();
        if words.len() == 2 && words[1] == parent && taken.already().iter().any(|done| done == words[0]) {
            parent = words[0].to_string();
            skipped += 1;
        } else {
            break;
        }
    }
    // What was said is what is taken: the count found now is the one said, or nothing is taken.
    if let Some(said) = later {
        let found = listed.lines().count() - skipped;
        if found != said {
            return Err(format!("the view holds {} commits now, and {} were said, so nothing is taken back", found, said));
        }
    }
    // Each commit of the view, its parent and what it changes, until one is refused.
    let mut wanted: Vec<(String, Raw, Vec<Change>)> = Vec::new();
    for line in listed.lines().skip(skipped) {
        let words: Vec<&str> = line.split(' ').collect();
        let commit = words[0].to_string();
        let raw = raw_of(store, &commit)?;
        if back.refused.is_some() {
            back.after.push((commit, raw.subject()));
            continue;
        }
        let why = if words.len() != 2 {
            Some("it has not one parent, and a merge is not taken".to_string())
        } else if words[1] != parent {
            Some(format!("its parent is not {}", short(&parent)))
        } else {
            let found = changes_of(store, &parent, &commit)?;
            let mut faults: Vec<String> = found.iter().filter_map(|change| not_allowed(&change.path, &change.mode, &rule, build_files)).collect();
            faults.extend(found.iter().filter(|change| change.mode == "160000").map(|change| format!("{} is a submodule", crate::issue::shown(&change.path))));
            // A path that the checkout holds untracked, ignored or not, would be written over by the fast-forward without
            // a word (#202): it is refused, whatever may come back.
            if faults.is_empty() {
                for change in found.iter().filter(|change| change.mode != "000000") {
                    if let Some(found) = untracked_in_checkout(root, &view.head, &change.path)? {
                        faults.push(format!("{} is in the checkout and not tracked there, and would be written over", crate::issue::shown(&found)));
                    }
                }
            }
            // The paths are judged first: a tree that holds a path git does not take is said so, not as odd.
            if faults.is_empty() {
                if let Some(why) = not_standard(store, &parent, &commit)? {
                    faults.push(why);
                }
            }
            if faults.is_empty() {
                wanted.push((commit.clone(), raw, found));
                parent = commit;
                continue;
            }
            Some(faults.join(", "))
        };
        back.refused = Some((commit, raw.subject(), why.unwrap_or_default()));
    }
    if wanted.is_empty() {
        return Ok(());
    }
    // The branch moved on since the view was made: what it changed must not meet what the commits change.
    if let Some(origin) = origin.filter(|origin| *origin != view.head) {
        let theirs = changes_of(Path::new(root), &origin, &view.head)?;
        let meets = |one: &str, other: &str| {
            one == other || one.strip_prefix(other).is_some_and(|rest| rest.starts_with('/')) || other.strip_prefix(one).is_some_and(|rest| rest.starts_with('/'))
        };
        let touched = wanted
            .iter()
            .flat_map(|(_, _, found)| found.iter())
            .find_map(|change| theirs.iter().find(|their| meets(&their.path, &change.path)).map(|their| their.path.clone()));
        if let Some(path) = touched {
            back.after.splice(0..0, wanted.iter().map(|(commit, raw, _)| (commit.clone(), raw.subject())));
            return Err(format!(
                "the branch {} has moved on from {} to {}, and a commit there changes {}, which a commit of the view changes too, so nothing is taken back",
                view.branch_name(),
                short(&origin),
                short(&view.head),
                crate::issue::shown(&path)
            ));
        }
    }
    // The branch is where it was found, or nothing is taken: made, moved or deleted since, it is not written over.
    // One that another worktree has checked out is not moved under its files.
    let not_taken = |back: &mut Back| {
        back.after.splice(0..0, wanted.iter().map(|(commit, raw, _)| (commit.clone(), raw.subject())));
    };
    let since = if later.is_some() { "since the take-back began" } else { "since the view was made" };
    let now = match tip_of(root, &view.branch) {
        Ok(now) => now,
        Err(why) => {
            not_taken(back);
            return Err(format!("the branch {} could not be read, so nothing is taken back: {}", view.branch_name(), why));
        }
    };
    if now != expected {
        not_taken(back);
        let how = match (&expected, &now) {
            (None, Some(now)) => format!("was made, at {}, {}", short(now), since),
            (Some(was), None) => format!("was at {} and is deleted {}", short(was), since),
            (Some(was), Some(now)) => format!("has moved from {} to {} {}", short(was), short(now), since),
            (None, None) => String::new(),
        };
        return Err(format!("the branch {} {}, so nothing is taken back", view.branch_name(), how));
    }
    let here = match checked_out(root) {
        Ok(found) => found.as_deref() == Some(view.branch.as_str()),
        Err(why) => {
            not_taken(back);
            return Err(format!("the branch checked out could not be read, so nothing is taken back: {}", why));
        }
    };
    if !here {
        match checked_out_elsewhere(root, &view.branch) {
            Ok(None) => {}
            Ok(Some(tree)) => {
                not_taken(back);
                return Err(format!(
                    "the branch {} is checked out in the worktree {}, and moving it would leave its files behind, so nothing is taken back",
                    view.branch_name(),
                    crate::issue::shown(&tree)
                ));
            }
            Err(why) => {
                not_taken(back);
                return Err(format!("the worktrees of the repository could not be read, so nothing is taken back: {}", why));
            }
        }
    }
    // The files of the commits, into the checkout's own store.
    let root = Path::new(root);
    let mut objects: Vec<u8> = Vec::new();
    for (_, _, found) in &wanted {
        for change in found.iter().filter(|change| change.mode != "000000") {
            objects.extend_from_slice(format!("{}\n", change.object).as_bytes());
        }
    }
    if !objects.is_empty() {
        let pack = git(store, &["pack-objects", "--stdout", "-q"], &[], Some(&objects))?;
        git(root, &["unpack-objects", "-q"], &[], Some(&pack))?;
    }
    // Each commit again, on the branch, by an index of `wb`'s own: the tree of the one before, with its changes.
    let index = store.join("index-of-the-checkout");
    let env = [("GIT_INDEX_FILE", index.as_path())];
    git(root, &["read-tree", &view.head], &env, None)?;
    let zero = "0".repeat(view.head.len());
    let mut tip = view.head.clone();
    let mut before = line(&git(root, &["rev-parse", &format!("{}^{{tree}}", view.head)], &[], None)?);
    let mut made: Vec<(String, String)> = Vec::new();
    let mut from: Vec<String> = Vec::new();
    for (index, (commit, raw, found)) in wanted.iter().enumerate() {
        let mut info: Vec<u8> = Vec::new();
        for change in found {
            let line = if change.mode == "000000" { format!("0 {}\t{}", zero, change.path) } else { format!("{} {}\t{}", change.mode, change.object, change.path) };
            info.extend_from_slice(line.as_bytes());
            info.push(0);
        }
        git(root, &["update-index", "-z", "--index-info"], &env, Some(&info))?;
        let tree = line(&git(root, &["write-tree"], &env, None)?);
        // The commit is taken as it was made: what it changes against the one before is what the view's commit
        // changed, path for path, mode and file; or it is refused, and none after it is taken. The tree of the
        // view holds fewer paths than the checkout's, so the changes are compared, not the trees.
        let put = changes_of(root, &before, &tree)?;
        let key = |change: &Change| (change.path.clone(), change.mode.clone(), change.object.clone());
        let (mut put, mut made_in_view): (Vec<_>, Vec<_>) = (put.iter().map(key).collect(), found.iter().map(key).collect());
        put.sort();
        made_in_view.sort();
        if put != made_in_view {
            let mut after: Vec<(String, String)> = wanted[index + 1..].iter().map(|(commit, raw, _)| (commit.clone(), raw.subject())).collect();
            if let Some((earlier, subject, _)) = back.refused.take() {
                after.push((earlier, subject));
            }
            after.append(&mut back.after);
            back.after = after;
            let first = put.iter().chain(&made_in_view).find(|one| !(put.contains(one) && made_in_view.contains(one)));
            let at = first.map(|(path, _, _)| format!(": at {}", crate::issue::shown(path))).unwrap_or_default();
            let why = format!("the commit that would be put on the branch differs from the one made in the view{}", at);
            back.refused = Some((commit.clone(), raw.subject(), why));
            break;
        }
        let (author, email, date) = &raw.author;
        let who = [("GIT_AUTHOR_NAME", Path::new(author)), ("GIT_AUTHOR_EMAIL", Path::new(email)), ("GIT_AUTHOR_DATE", Path::new(date))];
        tip = line(&git(root, &["commit-tree", &tree, "-p", &tip], &who, Some(&raw.message))?);
        before = tree;
        made.push((tip.clone(), raw.subject()));
        from.push(commit.clone());
    }
    if made.is_empty() {
        return Ok(());
    }
    // The branch checked out is read again just before the move (#298): one changed since is not moved under.
    let still = checked_out(&root.to_string_lossy()).map(|found| found.as_deref() == Some(view.branch.as_str()));
    if still != Ok(here) {
        back.after.splice(0..0, wanted.iter().take(made.len()).map(|(commit, raw, _)| (commit.clone(), raw.subject())));
        return Err(format!(
            "the branch checked out changed while the commits were taken, so they were made, up to {}, and not put on {}",
            tip,
            view.branch_name()
        ));
    }
    if here {
        // On the branch checked out, by a fast-forward of the checkout: git refuses it where a change of the working
        // tree would be lost.
        if let Err(why) = git(root, &["merge", "--ff-only", "-q", &tip], &[], None) {
            back.after.splice(0..0, wanted.iter().take(made.len()).map(|(commit, raw, _)| (commit.clone(), raw.subject())));
            return Err(format!(
                "the commits were made, up to {}, and could not be put on {}: {}. Put them there with `git merge --ff-only {}`",
                tip,
                view.branch_name(),
                why,
                tip
            ));
        }
    } else {
        // On a branch that is not checked out (#296), by its ref alone, with the value it was found with: git refuses
        // it where the branch was changed since. The branch checked out, the index and the working tree are not touched.
        let old = expected.clone().unwrap_or_else(|| zero.clone());
        if let Err(why) = git(root, &["update-ref", "-m", "wb: take-back", &view.branch, &tip, &old], &[], None) {
            back.after.splice(0..0, wanted.iter().take(made.len()).map(|(commit, raw, _)| (commit.clone(), raw.subject())));
            return Err(format!(
                "the commits were made, up to {}, and could not be put on {}: {}. Put them there with `git update-ref {} {} {}`",
                tip,
                view.branch_name(),
                why,
                view.branch,
                tip,
                old
            ));
        }
        let how = match &expected {
            Some(was) => format!("only its ref was moved, from {} to {}", short(was), short(&tip)),
            None => format!("it was not there, and only its ref was made, at {}, on top of {}, the commit the view was made from", short(&tip), short(&view.head)),
        };
        back.notes.push(format!("{} is not checked out: {}; the branch checked out, the index and the working tree are as they were", view.branch_name(), how));
    }
    back.taken = made;
    back.from = from;
    Ok(())
}

/// The ignore rules of the checkout, written in a folder of `wb`'s own below `store`: every `.gitignore` of `head`
/// at its place, and the checkout's `.git/info/exclude`. They are read from the checkout, never from the view, so
/// that a session cannot widen them. The result is the folder to take as a work tree of the store.
fn rules(root: &str, store: &Path, head: &str, notes: &mut Vec<String>) -> Result<PathBuf, String> {
    let root = Path::new(root);
    let tree = store.join("rules");
    fs::create_dir_all(&tree).map_err(|error| error.to_string())?;
    let listed = git(root, &["ls-tree", "-r", "-z", "--full-tree", head], &[], None)?;
    for entry in listed.split(|byte| *byte == 0).filter(|entry| !entry.is_empty()) {
        let text = String::from_utf8_lossy(entry);
        let Some((meta, path)) = text.split_once('\t') else { continue };
        let words: Vec<&str> = meta.split(' ').collect();
        let [mode, kind, object] = words.as_slice() else { continue };
        let found = parts(path);
        // A link is not read as a `.gitignore`, as git does not.
        if *kind != "blob" || *mode == "120000" || found.last() != Some(&".gitignore") || found.iter().any(|part| *part == ".git" || *part == "..") {
            continue;
        }
        let blob = git(root, &["cat-file", "blob", object], &[], None)?;
        let at = found.iter().fold(tree.clone(), |at, part| at.join(part));
        if let Some(folder) = at.parent() {
            fs::create_dir_all(folder).map_err(|error| error.to_string())?;
        }
        fs::write(&at, blob).map_err(|error| error.to_string())?;
    }
    let exclude = line(&git(root, &["rev-parse", "--git-path", "info/exclude"], &[], None)?);
    let exclude = if Path::new(&exclude).is_absolute() { PathBuf::from(exclude) } else { root.join(exclude) };
    let info = store.join("info");
    fs::create_dir_all(&info).map_err(|error| error.to_string())?;
    let said = exclude.display();
    let text = match fs::symlink_metadata(&exclude) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            notes.push(format!("the checkout's {} was not read, and its rules are not applied: {}", said, error));
            String::new()
        }
        Ok(found) if found.file_type().is_symlink() => {
            notes.push(format!("the checkout's {} was not read, and its rules are not applied: it is a link", said));
            String::new()
        }
        Ok(found) if !found.is_file() || found.len() > 64 * 1024 * 1024 => {
            notes.push(format!("the checkout's {} was not read, and its rules are not applied: it is not a plain file of 64 MiB or less", said));
            String::new()
        }
        Ok(_) => crate::bounded::read_plain(&exclude, 64 * 1024 * 1024)
            .map_err(|fault| fault.to_string())
            .and_then(|bytes| String::from_utf8(bytes).map_err(|_| "it is not UTF-8 text".to_string()))
            .unwrap_or_else(|error| {
                notes.push(format!("the checkout's {} was not read, and its rules are not applied: {}", said, error));
                String::new()
            }),
    };
    fs::write(info.join("exclude"), text).map_err(|error| error.to_string())?;
    Ok(tree)
}

/// Of `paths`, those the ignore rules of the checkout ignore, by `check-ignore` in the store with `tree` as its
/// work tree. The global rules of the person are not read.
fn ignored(store: &Path, tree: &Path, paths: &[String]) -> Result<Vec<String>, String> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let nowhere = store.join("nowhere");
    let env = [
        ("GIT_DIR", store),
        ("GIT_WORK_TREE", tree),
        ("GIT_CONFIG_GLOBAL", Path::new("/dev/null")),
        ("GIT_CONFIG_NOSYSTEM", Path::new("1")),
        ("XDG_CONFIG_HOME", nowhere.as_path()),
        ("HOME", nowhere.as_path()),
    ];
    let input: Vec<u8> = paths.iter().flat_map(|path| path.bytes().chain(std::iter::once(0))).collect();
    let args = ["check-ignore", "--no-index", "-z", "--stdin"];
    let ran = run_git_status(Some(tree), &[], &args, &env, Some(&input))?;
    let found = match ran.code {
        0 => ran.stdout,
        // git ends with 1 where nothing is ignored.
        1 => Vec::new(),
        _ => return Err(ran.said),
    };
    Ok(found.split(|byte| *byte == 0).filter(|path| !path.is_empty()).map(|path| String::from_utf8_lossy(path).to_string()).collect())
}

/// The paths of the view that differ from its `HEAD`, or are not in it and not ignored by the rules of the
/// checkout (`head` is the commit of the checkout the view was made from): work not committed.
fn uncommitted(root: &str, store: &Path, view: &Path, head_of_checkout: &str, notes: &mut Vec<String>) -> Result<Vec<String>, String> {
    let head = head_of(&view.join(".git"))?;
    let index = store.join("index-of-the-view");
    // No config of the person is read, so no filter driver or textconv can be defined for the view's files, and the
    // view's own config is never read: the repository is `wb`'s own.
    let env = [
        ("GIT_INDEX_FILE", index.as_path()),
        ("GIT_WORK_TREE", view),
        ("GIT_CONFIG_NOSYSTEM", Path::new("1")),
        ("GIT_CONFIG_GLOBAL", Path::new("/dev/null")),
        ("GIT_ATTR_NOSYSTEM", Path::new("1")),
    ];
    let quiet = ["-c", "core.fsmonitor=false", "-c", "core.untrackedCache=false", "-c", "core.attributesFile=/dev/null"];
    let with = |more: &[&str]| -> Vec<String> { quiet.iter().chain(more).map(|word| word.to_string()).collect() };
    let run = |more: &[&str]| -> Result<Vec<u8>, String> {
        let words = with(more);
        let words: Vec<&str> = words.iter().map(String::as_str).collect();
        git(store, &words, &env, None)
    };
    run(&["read-tree", &head])?;
    // `--refresh` ends with 1 where a file needs it; what differs is read next.
    let _ = run(&["update-index", "-q", "--refresh"]);
    let names = |listed: Vec<u8>| -> Vec<String> {
        listed.split(|byte| *byte == 0).filter(|path| !path.is_empty()).map(|path| String::from_utf8_lossy(path).to_string()).collect()
    };
    let changed = names(run(&["diff-files", "--name-only", "-z"])?);
    // No `--exclude-standard`: the view's own `.gitignore` is the session's, and is not read.
    let others = names(run(&["ls-files", "--others", "-z"])?);
    let tree = rules(root, store, head_of_checkout, notes)?;
    let ignored: std::collections::HashSet<String> = ignored(store, &tree, &others)?.into_iter().collect();
    let mut found: Vec<String> = changed;
    found.extend(others.into_iter().filter(|path| !ignored.contains(path)));
    found.sort();
    found.dedup();
    Ok(found)
}

/// Paths as `wb` says them, each with its control characters escaped (#167), joined by commas.
fn shown_all(paths: &[String]) -> String {
    paths.iter().map(|path| crate::issue::shown(path)).collect::<Vec<_>>().join(", ")
}

/// How many paths left uncommitted are named on the terminal; the whole list is in the file `back`.
const SHOWN: usize = 20;

/// Says what was taken back and what was not, and removes the view when all of it was taken; else it is kept,
/// and named.
fn said(name: &str, view: &View, back: Back, list: Option<&Path>, err: &mut dyn Write) -> Back {
    for note in &back.notes {
        let _ = writeln!(err, "{}: {}", name, note);
    }
    for (commit, subject) in &back.taken {
        let _ = writeln!(err, "{}: taken onto {}: {} {}", name, view.branch_name(), short(commit), subject);
    }
    if let Some((commit, subject, why)) = &back.refused {
        crate::refusal::limit(err, name, format_args!("refused, and not taken: the commit {} \"{}\" of the view: {}", short(commit), subject, why));
    }
    for (commit, subject) in &back.after {
        let _ = writeln!(err, "{}: not taken: the commit {} \"{}\" of the view", name, short(commit), subject);
    }
    if back.uncommitted.len() > SHOWN {
        let place = list.map(|file| file.display().to_string()).unwrap_or_else(|| "the file `back` of the session's folder".to_string());
        crate::refusal::limit(
            err,
            name,
            format_args!(
                "{} paths left uncommitted in the view, and not taken; the first {}: {}. The whole list is in {}",
                back.uncommitted.len(),
                SHOWN,
                shown_all(&back.uncommitted[..SHOWN]),
                place
            ),
        );
    } else if !back.uncommitted.is_empty() {
        crate::refusal::limit(err, name, format_args!("left uncommitted in the view, and not taken: {}", shown_all(&back.uncommitted)));
    }
    if let Some(why) = &back.failed {
        crate::refusal::limit(err, name, crate::issue::shown(why));
    }
    back
}

/// Removes the view when all of its work was taken back, and keeps it otherwise. What it did is said; the result
/// is the folder kept, or none.
pub fn tidy(name: &str, view: &View, back: &Back, err: &mut dyn Write) -> Option<PathBuf> {
    if back.all_taken() {
        match remove(&view.folder) {
            Ok(()) => return None,
            Err(error) => {
                let _ = writeln!(err, "{}: the view {} could not be removed: {}", name, view.folder.display(), error);
                return Some(view.folder.clone());
            }
        }
    }
    let _ = writeln!(err, "{}: the view is kept, with what was not taken back: {}", name, view.folder.display());
    Some(view.folder.clone())
}

/// Whether `commit` is a commit of the repository at `root`: a view made from another repository is not this one's (#176).
pub fn in_repository(root: &str, commit: &str) -> bool {
    is_object(commit) && git(Path::new(root), &["cat-file", "-e", &format!("{}^{{commit}}", commit)], &[], None).is_ok()
}

/// Removes a view that no session ran in.
pub fn discard(view: &View) {
    let _ = remove(&view.folder);
}

// ---- `wb view` (#173): what a view holds, read without making it ----

/// What `HEAD` of the checkout holds at or below each path: the path, its mode and its kind (`blob`, `tree`,
/// `commit`), as `ls-tree` gives them. `recursive` lists the files below the paths, or the whole tree where
/// `paths` is empty; otherwise each path itself. Reads only.
fn head_entries(root: &str, paths: &[String], recursive: bool) -> Result<Vec<(String, String, String)>, String> {
    let mut args = vec!["--no-optional-locks", "ls-tree", "-z", "--full-tree"];
    if recursive {
        args.push("-r");
    }
    args.extend(["HEAD", "--"]);
    args.extend(paths.iter().map(String::as_str));
    let listed = git(Path::new(root), &args, &[("GIT_LITERAL_PATHSPECS", Path::new("1"))], None)?;
    Ok(listed
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| {
            let text = String::from_utf8_lossy(entry);
            let (meta, path) = text.split_once('\t')?;
            let words: Vec<&str> = meta.split(' ').collect();
            Some((path.to_string(), words.first()?.to_string(), words.get(1)?.to_string()))
        })
        .collect())
}

/// What `wb view <issue> <role> --in [<folder>]` and `--out [<folder>]` find for a folder of `HEAD` (#209).
pub enum Folder {
    /// The folder is not in `HEAD`, or is a submodule, which holds no files of `HEAD`'s.
    Missing,
    /// The path is a file of `HEAD`.
    File,
    /// The lines to print.
    Listed(String),
}

/// What of the files of `HEAD` the view of `paths` holds (`inside`) or leaves out, in `folder` (the root when
/// it has no parts), one level deep: each file directly in it, as its path from the root, then each folder
/// below it with the count of those files under it, as `<count><tab><folder>/`. Files first, each kind in
/// the order of the bytes. A file of the view that is not in `HEAD` is not counted. Reads only.
pub fn by_folder(root: &str, paths: &[String], folder: &str, inside: bool) -> Result<Folder, String> {
    let all = head_entries(root, &[], true)?;
    let at = parts(folder);
    let wanted: Vec<Vec<&str>> = paths.iter().map(|path| parts(path)).filter(|found| !found.is_empty()).collect();
    let in_view = |found: &[&str]| wanted.iter().any(|entry| found.len() >= entry.len() && found[..entry.len()] == entry[..]);
    let mut seen = at.is_empty();
    let mut files: Vec<String> = Vec::new();
    let mut folders: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for (path, _, kind) in &all {
        let found = parts(path);
        if found.len() > at.len() && found[..at.len()] == at[..] {
            seen = true;
        } else if found == at && !at.is_empty() {
            // A submodule is in `HEAD` as a `commit`: not a file, and not a folder to look into either.
            return Ok(if kind == "blob" { Folder::File } else { Folder::Missing });
        } else {
            continue;
        }
        if kind != "blob" || in_view(&found) != inside {
            continue;
        }
        if found.len() == at.len() + 1 {
            files.push(path.clone());
        } else {
            *folders.entry(found[..at.len() + 1].join("/")).or_default() += 1;
        }
    }
    if !seen {
        return Ok(Folder::Missing);
    }
    files.sort();
    let mut text: String = files.iter().map(|file| format!("{}\n", crate::issue::shown(file))).collect();
    for (below, count) in folders {
        text.push_str(&format!("{}\t{}/\n", count, crate::issue::shown(&below)));
    }
    Ok(Folder::Listed(text))
}

/// What the checkout says of one path, for `wb view <issue> <path>`: the path as its parts from the root, what
/// `HEAD` has there (its mode and kind), and, for a path not in `HEAD`, the rule of the checkout that ignores it.
pub struct Facts {
    pub path: String,
    pub at_head: Option<(String, String)>,
    pub ignored: Option<String>,
}

/// Reads the facts of `path` from the checkout at `root`. Ignored is asked of git in the checkout, by the
/// `.gitignore` files of the working tree and `.git/info/exclude`, and never the person's global file: it
/// writes nothing, as a copy of `HEAD`'s rules would (#159).
pub fn facts(root: &str, path: &str) -> Result<Facts, String> {
    let path = parts(path).join("/");
    let at_head = head_entries(root, std::slice::from_ref(&path), false)?
        .into_iter()
        .find(|(found, _, _)| *found == path)
        .map(|(_, mode, kind)| (mode, kind));
    let mut ignored = None;
    if at_head.is_none() && refused_by_git(&path, "100644").is_none() {
        let args = ["--no-optional-locks", "-c", "core.excludesFile=/dev/null", "check-ignore", "--no-index", "-v", "-z", "--stdin"];
        let input: Vec<u8> = path.bytes().chain(std::iter::once(0)).collect();
        let ran = run_git_status(Some(Path::new(root)), &[], &args, &[], Some(&input))?;
        match ran.code {
            0 => {
                let fields: Vec<String> = ran.stdout.split(|byte| *byte == 0).map(|field| String::from_utf8_lossy(field).to_string()).collect();
                if let [source, number, pattern, ..] = fields.as_slice() {
                    if !pattern.starts_with('!') {
                        ignored = Some(format!("{}, line {}: {}", crate::issue::shown(source), number, crate::issue::shown(pattern)));
                    }
                }
            }
            // git ends with 1 where nothing is ignored.
            1 => {}
            _ => return Err(ran.said),
        }
    }
    Ok(Facts { path, at_head, ignored })
}

/// Whether the view of a role holds a path, and where the issue's file lists it.
pub enum Verdict {
    /// The list (`changes` or `reads.<role>`), the line of the file, and the entry as the file writes it.
    In { list: String, line: usize, entry: String },
    /// Why not.
    Out(String),
}

/// Whether the path of `facts` is in the view of `role` of the issue's file `used`, and why. An entry holds a
/// path as the view is made: a folder of `HEAD` holds every path below it, a file only itself; an entry that is
/// not in `HEAD` is a folder where it ends with `/` (#192). The reasons are taken in a fixed order: under
/// `.watchbill/`, a path git does not take, listed and not in `HEAD`, a submodule, ignored, not in `HEAD`,
/// under `build_files`, not listed.
pub fn judge(root: &str, used: &crate::issue::Used, role: &str, facts: &Facts, build_files: &[String]) -> Result<Verdict, String> {
    let path = facts.path.as_str();
    let found = parts(path);
    let reads = format!("reads.{}", role);
    if found.first() == Some(&".watchbill") {
        return Ok(Verdict::Out("under .watchbill/, which no view holds".to_string()));
    }
    let mode = facts.at_head.as_ref().map(|(mode, _)| mode.as_str()).unwrap_or("100644");
    if let Some(why) = refused_by_git(path, mode) {
        return Ok(Verdict::Out(format!("a path git does not take: {}", why)));
    }
    let lists = std::iter::once(("changes".to_string(), &used.issue.changes))
        .chain(used.issue.reads.iter().filter(|(name, _)| name == role).map(|(_, list)| (reads.clone(), list)));
    let mut below: Vec<String> = Vec::new();
    for (list, entries) in lists {
        for entry in entries {
            let Some(covered) = covered(root, "HEAD", std::slice::from_ref(entry))?.into_iter().next() else { continue };
            let n = covered.parts.len();
            let holds = found.len() >= n && found[..n].iter().zip(&covered.parts).all(|(a, b)| a == b) && (covered.folder || found.len() == n);
            if !holds {
                if n > found.len() && covered.parts[..found.len()].iter().zip(&found).all(|(a, b)| a == b) {
                    below.push(crate::issue::shown(entry));
                }
                continue;
            }
            let line = used.listed.iter().find(|listed| listed.list == list && listed.path == *entry).map(|listed| listed.line).unwrap_or(0);
            let at = format!("{}, line {} of {}: {}", list, line, used.path, crate::issue::shown(entry));
            return Ok(match &facts.at_head {
                None if list == "changes" => Verdict::Out(format!("not in HEAD, and the view is made from HEAD; it is listed in {}. A session may make it", at)),
                None => Verdict::Out(format!("not in HEAD, and the view is made from HEAD; it is listed in {}", at)),
                Some((_, kind)) if kind == "commit" => Verdict::Out(format!("a submodule, which no view holds; it is listed in {}", at)),
                Some(_) => Verdict::In { list, line, entry: crate::issue::shown(entry) },
            });
        }
    }
    let unlisted = format!("not listed in changes or {}", reads);
    if facts.at_head.is_none() {
        return Ok(Verdict::Out(match &facts.ignored {
            Some(rule) => format!("ignored by the checkout ({}), and not in HEAD", rule),
            None => format!("not in HEAD, and {}", unlisted),
        }));
    }
    if let Some(build) = build_files.iter().find(|build| crate::starter::meet(path, build)) {
        return Ok(Verdict::Out(format!(
            "under `build_files` of watchbill.yml ({}), and {}: a role never changes it, and sees it only where its reads list it",
            crate::issue::shown(build),
            unlisted
        )));
    }
    if below.is_empty() {
        return Ok(Verdict::Out(unlisted));
    }
    let more = below.len().saturating_sub(SHOWN);
    below.truncate(SHOWN);
    if more > 0 {
        below.push(format!("and {} more", more));
    }
    Ok(Verdict::Out(format!("{}; the view holds paths below it: {}", unlisted, below.join(", "))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn test_WB_VIEW_72_a_deadline_inside_another_keeps_the_earlier_and_restores_it_when_dropped() {
        let outer = deadline(std::time::Duration::from_secs(300), "take-back");
        let held = || DEADLINE.with(|deadline| deadline.get()).map(|(_, total, what)| (total.as_secs(), what));
        {
            let _inner = deadline(std::time::Duration::from_secs(20), "read of the view");
            assert_eq!(held(), Some((20, "read of the view")));
            let _longer = deadline(std::time::Duration::from_secs(100), "later");
            assert_eq!(held(), Some((20, "read of the view")), "a later deadline took the place of an earlier one");
        }
        assert_eq!(held(), Some((300, "take-back")), "the deadline of the take-back was not held again");
        drop(outer);
        assert_eq!(held(), None);
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_WB_PUSH_12_no_variable_of_a_login_of_the_code_host_is_handed_into_the_sandbox() {
        let mut command = Command::new("/usr/bin/bwrap");
        without_logins(&mut command);
        let removed: Vec<String> =
            command.get_envs().filter(|(_, value)| value.is_none()).map(|(name, _)| name.to_string_lossy().to_string()).collect();
        for name in [
            "GH_TOKEN",
            "GITHUB_TOKEN",
            "GH_ENTERPRISE_TOKEN",
            "GITHUB_ENTERPRISE_TOKEN",
            "GH_CONFIG_DIR",
            "GITLAB_TOKEN",
            "GLAB_TOKEN",
            "GITLAB_ACCESS_TOKEN",
            "GLAB_CONFIG_DIR",
            "CI_JOB_TOKEN",
            "SSH_AUTH_SOCK",
            "SSH_ASKPASS",
        ] {
            assert!(removed.iter().any(|one| one == name), "{} is handed in: {:?}", name, removed);
        }
    }

    /// The variables `without_logins` takes out of what a command is handed.
    fn removed() -> Vec<String> {
        let mut command = Command::new("/usr/bin/bwrap");
        without_logins(&mut command);
        command.get_envs().filter(|(_, value)| value.is_none()).map(|(name, _)| name.to_string_lossy().to_string()).collect()
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_WB_PUSH_12_glab_s_other_name_of_its_token_and_its_certificate_s_password_are_not_handed_in() {
        let removed = removed();
        for name in ["OAUTH_TOKEN", "GLAB_CLIENT_CERT_PASSWORD"] {
            assert!(removed.iter().any(|one| one == name), "{} is handed in: {:?}", name, removed);
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_WB_PUSH_12_the_secrets_gitlab_ci_hands_a_job_are_not_handed_in() {
        let removed = removed();
        for name in ["CI_JOB_TOKEN", "CI_REGISTRY_PASSWORD", "CI_DEPENDENCY_PROXY_PASSWORD", "CI_JOB_JWT", "CI_JOB_JWT_V2"] {
            assert!(removed.iter().any(|one| one == name), "{} is handed in: {:?}", name, removed);
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_WB_PUSH_12_the_address_of_the_session_bus_is_not_handed_in() {
        // The bus leads to the keyring that `gh` keeps its login in by default.
        let removed = removed();
        assert!(removed.iter().any(|one| one == "DBUS_SESSION_BUS_ADDRESS"), "{:?}", removed);
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_WB_PUSH_8_the_folder_of_the_description_is_bound_at_run_watchbill_in_the_sandbox_s_own_run() {
        let words = ["runner".to_string()];
        let plan = Plan {
            view: Path::new("/var/views/1"),
            checkout: Path::new("/work/brakes"),
            binds: vec![],
            organisation: None,
            home: Some(Path::new("/home/ada")),
            new_session: false,
            words: &words,
        };
        let words: Vec<String> =
            arguments_out(&plan, Layout::Listed, Some(Path::new("/state/watchbill/out/1"))).iter().map(|word| word.to_string_lossy().to_string()).collect();
        let find = |pair: [&str; 3]| words.windows(3).position(|three| three == pair);
        find(["--bind", "/var/views/1", "/work/brakes"]).expect("the view is not bound");
        let out = find(["--bind", "/state/watchbill/out/1", OUT_FOLDER]).expect("the folder of the description is not bound");
        // In the sandbox's own `/run`, made empty before it.
        let run = words.windows(2).position(|two| two == ["--tmpfs", "/run"]).expect("/run is not the sandbox's own");
        assert!(run < out, "{:?}", words);
        assert_eq!(DESCRIPTION, "/run/watchbill/description");
        assert!(DESCRIPTION.starts_with(&format!("{}/", OUT_FOLDER)));
        // Without it, the words are those of `arguments_in`.
        let none: Vec<String> = arguments_out(&plan, Layout::Listed, None).iter().map(|word| word.to_string_lossy().to_string()).collect();
        let before: Vec<String> = arguments_in(&plan, Layout::Listed).iter().map(|word| word.to_string_lossy().to_string()).collect();
        assert_eq!(none, before);
    }
}
