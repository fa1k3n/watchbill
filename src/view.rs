//! The view (#126): a session of the tester, the build, the review or the fix of an issue with an accepted file
//! sees only the paths its issue's file lists for it, and the program keeps that limit, not the prompt.
//!
//! - **The view** is a new git repository, made by `wb` from the trusted side: the paths of `changes` and of the
//!   role's `reads`, as they are in `HEAD` of the checkout, in one commit, with no remote and no history.
//! - **The sandbox** is bubblewrap: the view at the path of the checkout, writable; the system read-only; `/tmp`
//!   and `/run` of its own; the home folder empty but for the runner's `binds`; the folder of the organisation's
//!   file covered. The network stays as the runner needs it.
//! - **What comes back**: each commit of the view, in order, put on the checkout's branch when it keeps to
//!   `changes`, and refused, with every commit after it, when it does not. `wb` pushes nothing: `wb <role>` says
//!   the command that pushes, and a person runs it (#145).
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
    let mut command = Command::new("git");
    command.arg("-C").arg(at).args(args);
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
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
        child.wait_with_output()
    })
    .map_err(|error| format!("git could not be waited for: {}", error))?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    let said = String::from_utf8_lossy(&output.stderr);
    let first = said.lines().find(|line| !line.trim().is_empty()).unwrap_or("").trim().to_string();
    Err(format!("`git {}` ended with {}{}", args.first().copied().unwrap_or(""), output.status.code().unwrap_or(-1), if first.is_empty() { String::new() } else { format!(": {}", first) }))
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
fn remove(folder: &Path) -> std::io::Result<()> {
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
/// the branch of the checkout its commits go back to.
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

/// Makes the view of `paths` from `HEAD` of the checkout at `root`, in a new folder below `below` named `base`.
/// A path that is not in `HEAD` is not made: the session may make it. A folder that cannot be made, or a git that
/// fails, is the error, and nothing is left.
pub fn make(root: &str, paths: &[String], below: &Path, base: &str) -> Result<View, String> {
    let (branch, head) = branch_of(root)?;
    let folder = new_folder(below, base)?;
    match fill(Path::new(root), paths, &folder, &branch, &head) {
        Ok(commit) => Ok(View { folder, base: commit, head, branch }),
        Err(why) => {
            let _ = remove(&folder);
            Err(format!("the view could not be made: {}", why))
        }
    }
}

/// The work of `make` once the folder is there: a repository with the listed files of `HEAD` and one commit.
fn fill(root: &Path, paths: &[String], folder: &Path, branch: &str, head: &str) -> Result<String, String> {
    let short = branch.strip_prefix("refs/heads/").unwrap_or(branch);
    let literal = [("GIT_LITERAL_PATHSPECS", Path::new("1"))];
    // The entries of `HEAD` below each path, as `ls-tree` gives them; a submodule is left out.
    let wanted: Vec<String> = paths.iter().map(|path| parts(path).join("/")).filter(|path| !path.is_empty()).collect();
    let mut args = vec!["ls-tree", "-r", "-z", "--full-tree", head, "--"];
    args.extend(wanted.iter().map(String::as_str));
    let listed = if wanted.is_empty() { Vec::new() } else { git(root, &args, &literal, None)? };
    let mut entries: Vec<u8> = Vec::new();
    let mut blobs: Vec<u8> = Vec::new();
    for entry in listed.split(|byte| *byte == 0).filter(|entry| !entry.is_empty()) {
        let text = String::from_utf8_lossy(entry);
        let mut words = text.split_whitespace();
        let (_, kind, object) = (words.next(), words.next(), words.next());
        if kind != Some("blob") {
            continue;
        }
        entries.extend_from_slice(entry);
        entries.push(0);
        blobs.extend_from_slice(object.unwrap_or_default().as_bytes());
        blobs.push(b'\n');
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
        let pack = git(root, &["pack-objects", "--stdout", "-q"], &[], Some(&blobs))?;
        git(folder, &["unpack-objects", "-q"], &[], Some(&pack))?;
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
    let message = b"The view: the paths of the issue's file for this role, as they are in HEAD of the checkout.\n";
    let commit = line(&git(folder, &["commit-tree", "--no-gpg-sign", &tree], &who, Some(message))?);
    git(folder, &["update-ref", branch, &commit], &[], None)?;
    Ok(commit)
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

/// The words bubblewrap is given for a session in a view.
pub fn arguments(plan: &Plan) -> Vec<OsString> {
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
    add(&["--bind".as_ref(), plan.view.as_os_str(), plan.checkout.as_os_str()]);
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
            "there is no {}, and a session of an issue with an accepted file runs only in a view, in bubblewrap. Install the package bubblewrap",
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
    Err(format!("bubblewrap ({}) could not make a sandbox here: {}.{} A session of an issue with an accepted file runs only in a view", bwrap.display(), first.trim_end_matches('.'), off))
}

/// Runs the session in its view, in bubblewrap at `bwrap`, in the foreground, with the person's standard
/// input, output and error, and no variable `GIT_*` of this process. While it runs, an interrupt or a quit at the
/// terminal does not end `wb` (`interrupt::hold`).
pub fn run(bwrap: &Path, plan: &Plan) -> Result<ExitStatus, String> {
    let _held = crate::interrupt::hold();
    let mut command = Command::new(bwrap);
    command.args(arguments(plan)).current_dir(plan.checkout);
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    command.stdin(Stdio::inherit()).stdout(Stdio::inherit()).stderr(Stdio::inherit());
    command.status().map_err(|error| format!("the session could not be started: {} could not be run: {}", bwrap.display(), error))
}

/// What was done with the commits of a view.
#[derive(Debug, Default)]
pub struct Back {
    /// Each commit put on the branch: its name there and its subject.
    pub taken: Vec<(String, String)>,
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
        for (commit, subject) in &self.taken {
            text.push_str(&format!("taken: {} {}\n", commit, subject));
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
fn is_object(name: &str) -> bool {
    (name.len() == 40 || name.len() == 64) && name.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

/// A small file of the view's `.git`, read as it is: a plain file, never through a link.
fn small_file(path: &Path) -> Option<String> {
    let found = fs::symlink_metadata(path).ok()?;
    if !found.is_file() || found.len() > 64 * 1024 * 1024 {
        return None;
    }
    fs::read_to_string(path).ok()
}

/// The commit the view's `HEAD` names, read from the files of its `.git`, without git: a branch, loose or packed,
/// or a commit named in `HEAD` itself.
fn head_of(git_dir: &Path) -> Result<String, String> {
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

/// Why a path that a commit changes is not taken, if it is not: under `.watchbill/`, a file of the build, a path
/// git does not take, or outside `changes`.
fn not_allowed(path: &str, mode: &str, changes: &[Covered], build_files: &[String]) -> Option<String> {
    let found = parts(path);
    if found.first() == Some(&".watchbill") {
        return Some(format!("{} is under .watchbill/", crate::issue::shown(path)));
    }
    if let Some(build) = build_files.iter().find(|build| crate::starter::meet(path, build)) {
        return Some(format!("{} is under `build_files` of watchbill.yml: {}", crate::issue::shown(path), build));
    }
    if let Some(why) = refused_by_git(path, mode) {
        return Some(format!("{} is a path git does not take: {}", crate::issue::shown(path), why));
    }
    let inside = changes.iter().any(|entry| {
        let n = entry.parts.len();
        found.len() >= n && found[..n].iter().zip(&entry.parts).all(|(a, b)| a == b) && (entry.folder || found.len() == n)
    });
    if !inside {
        return Some(format!("{} is outside `changes`", crate::issue::shown(path)));
    }
    None
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
fn short(commit: &str) -> &str {
    &commit[..commit.len().min(12)]
}

/// Takes the work of the session back from `view` to the checkout at `root`: each commit, in order, while it
/// keeps to `changes`, then the paths left uncommitted. A commit that does not is refused, and none after it is
/// taken. What is said goes to `err`, after `name`; `list` is where the whole list of paths left is written.
pub fn take_back(name: &str, root: &str, view: &View, changes: &[String], build_files: &[String], list: Option<&Path>, err: &mut dyn Write) -> Back {
    let mut back = Back::default();
    let store = match Store::new(view) {
        Ok(store) => store,
        Err(why) => {
            back.failed = Some(why);
            return said(name, view, back, list, err);
        }
    };
    if let Err(why) = commits(root, view, &store.path, changes, build_files, &mut back) {
        back.failed = Some(why);
    }
    match uncommitted(root, &store.path, &view.folder, &view.head, &mut back.notes) {
        Ok(found) => back.uncommitted = found,
        Err(why) => {
            if back.failed.is_none() {
                back.failed = Some(format!("what is left uncommitted in the view could not be read: {}", why));
            }
        }
    }
    drop(store);
    said(name, view, back, list, err)
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
fn commits(root: &str, view: &View, store: &Path, changes: &[String], build_files: &[String], back: &mut Back) -> Result<(), String> {
    let head = head_of(&view.folder.join(".git"))?;
    if head == view.base {
        return Ok(());
    }
    let covered = covered(root, &view.head, changes)?;
    if git(store, &["merge-base", "--is-ancestor", &view.base, &head], &[], None).is_err() {
        return Err(format!("the view's HEAD, {}, does not come from the commit the view was made with", short(&head)));
    }
    let listed = git(store, &["rev-list", "--first-parent", "--reverse", "--parents", &format!("{}..{}", view.base, head)], &[], None)?;
    let listed = String::from_utf8_lossy(&listed).to_string();
    // Each commit of the view, its parent and what it changes, until one is refused.
    let mut wanted: Vec<(String, Raw, Vec<Change>)> = Vec::new();
    let mut parent = view.base.clone();
    for line in listed.lines() {
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
            let mut faults: Vec<String> = found.iter().filter_map(|change| not_allowed(&change.path, &change.mode, &covered, build_files)).collect();
            faults.extend(found.iter().filter(|change| change.mode == "160000").map(|change| format!("{} is a submodule", crate::issue::shown(&change.path))));
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
    // The checkout is where the view was made from, or nothing is taken.
    match branch_of(root) {
        Ok((branch, now)) if branch == view.branch && now == view.head => {}
        _ => {
            back.after.splice(0..0, wanted.iter().map(|(commit, raw, _)| (commit.clone(), raw.subject())));
            return Err(format!("the checkout is no longer at {} on {}, where the view was made from, so nothing is taken back", short(&view.head), view.branch_name()));
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
    }
    if made.is_empty() {
        return Ok(());
    }
    // On the branch, by a fast-forward of the checkout: git refuses it where a change of the working tree would be lost.
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
    back.taken = made;
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
        Ok(_) => fs::read_to_string(&exclude).unwrap_or_else(|error| {
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
    let found = match git(tree, &args, &env, Some(&input)) {
        Ok(found) => found,
        // git ends with 1 where nothing is ignored.
        Err(why) if why == "`git check-ignore` ended with 1" => Vec::new(),
        Err(why) => return Err(why),
    };
    Ok(found.split(|byte| *byte == 0).filter(|path| !path.is_empty()).map(|path| String::from_utf8_lossy(path).to_string()).collect())
}

/// The paths of the view that differ from its `HEAD`, or are not in it and not ignored by the rules of the
/// checkout (`head` is the commit of the checkout the view was made from): work not committed.
fn uncommitted(root: &str, store: &Path, view: &Path, head_of_checkout: &str, notes: &mut Vec<String>) -> Result<Vec<String>, String> {
    let head = head_of(&view.join(".git"))?;
    let index = store.join("index-of-the-view");
    let env = [("GIT_INDEX_FILE", index.as_path()), ("GIT_WORK_TREE", view)];
    let quiet = ["-c", "core.fsmonitor=false", "-c", "core.untrackedCache=false"];
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
        let _ = writeln!(err, "{}: refused, and not taken: the commit {} \"{}\" of the view: {}", name, short(commit), subject, why);
    }
    for (commit, subject) in &back.after {
        let _ = writeln!(err, "{}: not taken: the commit {} \"{}\" of the view", name, short(commit), subject);
    }
    if back.uncommitted.len() > SHOWN {
        let place = list.map(|file| file.display().to_string()).unwrap_or_else(|| "the file `back` of the session's folder".to_string());
        let _ = writeln!(
            err,
            "{}: {} paths left uncommitted in the view, and not taken; the first {}: {}. The whole list is in {}",
            name,
            back.uncommitted.len(),
            SHOWN,
            shown_all(&back.uncommitted[..SHOWN]),
            place
        );
    } else if !back.uncommitted.is_empty() {
        let _ = writeln!(err, "{}: left uncommitted in the view, and not taken: {}", name, shown_all(&back.uncommitted));
    }
    if let Some(why) = &back.failed {
        let _ = writeln!(err, "{}: {}", name, crate::issue::shown(why));
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

/// Removes a view that no session ran in.
pub fn discard(view: &View) {
    let _ = remove(&view.folder);
}
