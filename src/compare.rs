//! `wb compare`: what changed between two revisions (#179).
//!
//! `wb` holds the files of two revisions at most: the installed one, and with `--from <path>` the one
//! the repository at `<path>` is pinned to (its `.watchbill/revision/`). A revision is named by its
//! version or by its full hash. The files are read through the manifest, which refuses a link, a hidden
//! file and a file not in the fixed form by name. The output is fixed: nothing when they are the same,
//! else one line for each file added, removed or changed, sorted by path, then an empty line and the
//! unified diff of each changed file. The diff is `git diff --no-index` run as a process with fixed
//! options and no configuration of the person; nothing is written and nothing asks the network.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::init::{CONFIG_FILE, CONFIG_FOLDER};
use crate::manifest;
use crate::yamlfile::{load, Part};

/// A revision `wb` can read: its version, its hash, the folder that holds its `revision/` and the
/// hash of each file.
struct Held {
    version: String,
    hash: String,
    base: PathBuf,
    files: BTreeMap<String, String>,
}

/// The files of a manifest as path and hash.
fn files_of(text: &str) -> BTreeMap<String, String> {
    text.lines().filter_map(|line| line.split_once("  ")).map(|(hash, path)| (path.to_string(), hash.to_string())).collect()
}

fn installed(package: &Path, version: &str) -> Result<Held, String> {
    let text = manifest::manifest(package).map_err(|why| format!("the installed revision cannot be read: {}", why))?;
    Ok(Held { version: version.to_string(), hash: manifest::hash_of(&text), base: package.to_path_buf(), files: files_of(&text) })
}

/// The revision the repository at `root` is pinned to, read from its files. A link is never followed.
fn pinned(root: &Path) -> Result<Held, String> {
    let folder = root.join(CONFIG_FOLDER);
    let shown = format!("{}/{}", crate::word(root.as_os_str().as_encoded_bytes()), CONFIG_FOLDER);
    match fs::symlink_metadata(&folder) {
        Ok(meta) if meta.file_type().is_symlink() => return Err(format!("{} is a symbolic link, which is never followed", shown)),
        Ok(meta) if meta.is_dir() => {}
        _ => return Err(format!("{} is not there: Watchbill is not set up in that repository", shown)),
    }
    let document = load(folder.join(CONFIG_FILE)).map_err(|why| why.to_string())?;
    let version = document.text(&[Part::from("watchbill")]).map_err(|why| why.to_string())?;
    if !document.has(&[Part::from("revision")]) {
        return Err(format!("{}/{} pins no revision: the line `revision:` is not there", shown, CONFIG_FILE));
    }
    let line = document.text(&[Part::from("revision")]).map_err(|why| why.to_string())?;
    let text = manifest::manifest(&folder).map_err(|why| format!("{}/{}", shown, why))?;
    let hash = manifest::hash_of(&text);
    if hash != line {
        return Err(format!(
            "the files under {}/revision/ are not the revision that {} names ({}): they are {}. `wb check` says which differ",
            shown, CONFIG_FILE, line, hash
        ));
    }
    Ok(Held { version, hash, base: folder, files: files_of(&text) })
}

/// The revision a name stands for, among those held.
fn resolve<'a>(name: &str, held: &'a [(&'static str, Held)]) -> Result<&'a Held, String> {
    let found: Vec<&(&str, Held)> = held.iter().filter(|(_, one)| one.version == name || one.hash == name).collect();
    match found.as_slice() {
        [] => {
            let mut said = String::new();
            for (what, one) in held {
                said.push_str(&format!("{} is version {}, revision {}; ", what, one.version, one.hash));
            }
            if held.len() == 1 {
                said.push_str("no repository is given with --from; ");
            }
            Err(format!("'{}' is not a revision that wb holds: {}a version is its own, a hash is the whole 64 digits", name.escape_debug(), said))
        }
        [(_, one)] => Ok(one),
        [(first, one), (second, other)] if one.hash != other.hash => {
            Err(format!("'{}' names two revisions, the {} and the {}: name one by its hash", name.escape_debug(), first, second))
        }
        [(_, one), ..] => Ok(one),
    }
}

/// The unified diff of one file, from the first hunk on, or the reason `git` gave none.
fn diff(first: &Path, second: &Path) -> Result<String, String> {
    let mut command = Command::new("git");
    command
        .args(["diff", "--no-index", "--no-color", "--no-ext-diff", "--no-textconv", "--no-renames"])
        .args(["--diff-algorithm=myers", "--no-indent-heuristic", "-U3", "--inter-hunk-context=0", "--"])
        .arg(first)
        .arg(second)
        .current_dir("/")
        .env_clear()
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    if let Some(path) = std::env::var_os("PATH") {
        command.env("PATH", path);
    }
    let run = command.output().map_err(|error| format!("git cannot be run for the diff ({})", error))?;
    // `--no-index` ends with 1 where the files differ.
    if !matches!(run.status.code(), Some(0) | Some(1)) {
        return Err("git could not make the diff".to_string());
    }
    let text = String::from_utf8(run.stdout).map_err(|_| "git made a diff that is not UTF-8".to_string())?;
    // From the first hunk on. The words git adds after a hunk's range name the function it guesses from the
    // text, which a version of git may change: they are left out, so that the output is fixed.
    let mut hunks = String::new();
    for line in text.split_inclusive('\n').skip_while(|line| !line.starts_with("@@ ")) {
        match line.strip_prefix("@@ ").and_then(|rest| rest.find(" @@").map(|at| at + 6)) {
            Some(end) => hunks.push_str(&format!("{}\n", &line[..end])),
            None => hunks.push_str(line),
        }
    }
    Ok(hunks)
}

/// `wb compare`: `package` is the folder that holds the installed `revision/`, `from` the root of a
/// repository, `first` and `second` the names of the revisions. 0 the same, 1 they differ, 2 a revision
/// is not found.
pub fn compare(package: &Path, version: &str, from: Option<&Path>, first: &str, second: &str, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let found = (|| -> Result<(Vec<u8>, bool), String> {
        let mut held = vec![("installed revision", installed(package, version)?)];
        if let Some(root) = from {
            held.push(("revision at --from", pinned(root)?));
        }
        let a = resolve(first, &held)?;
        let b = resolve(second, &held)?;
        if a.hash == b.hash {
            return Ok((Vec::new(), false));
        }
        let mut lines: Vec<(&str, &str)> = Vec::new();
        for (path, hash) in &a.files {
            match b.files.get(path) {
                None => lines.push((path, "removed")),
                Some(other) if other != hash => lines.push((path, "changed")),
                _ => {}
            }
        }
        lines.extend(b.files.keys().filter(|path| !a.files.contains_key(*path)).map(|path| (path.as_str(), "added")));
        lines.sort();
        let mut text = String::new();
        for (path, kind) in &lines {
            text.push_str(&format!("{}: {}\n", kind, path));
        }
        let mut first_diff = true;
        for (path, _) in lines.iter().filter(|(_, kind)| *kind == "changed") {
            let hunks = diff(&a.base.join(path), &b.base.join(path))?;
            if first_diff {
                text.push('\n');
                first_diff = false;
            }
            text.push_str(&format!("--- a/{}\n+++ b/{}\n{}", path, path, hunks));
        }
        Ok((text.into_bytes(), true))
    })();
    match found {
        Err(why) => {
            let _ = writeln!(err, "wb compare: {}", why);
            2
        }
        Ok((_, false)) => 0,
        Ok((text, true)) => {
            let _ = out.write_all(&text);
            // A difference is the answer of the command, on standard output, as the plumbing has it.
            crate::refusal::answer();
            1
        }
    }
}
