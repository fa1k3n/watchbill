//! `wb upgrade [--dry-run]` (#174): pins a repository to the installed revision, and says what it will write
//! first.
//!
//! It makes `.watchbill/revision/` hold exactly the files of the installed revision and `.watchbill/revision.sha256`
//! its manifest, sets the lines `watchbill:` and `revision:` of `watchbill.yml`, and adds each pattern the
//! installed `wb init` would write that the file lacks. It writes over only what is the revision's own, and only
//! where git holds all of `.watchbill/` as committed, so every byte it writes over is in `HEAD`. It says each
//! write before it writes, runs the check of `wb check` afterwards, and commits nothing.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::detect::{origin_host, Setup, NONE};
use crate::init::{self, CONFIG_FILE, CONFIG_FOLDER};
use crate::manifest::MANIFEST_FILE;
use crate::yamlfile::{self, Document, Node, Part, Value};

/// The lines `wb init` writes that are settings a person decides, not patterns: `wb upgrade` never adds them. A
/// file with no `classed` takes every path as classed, and `classed: []` would lift that.
const SETTINGS: [&str; 10] =
    ["watchbill", "revision", "project", "git_server", "repository", "tracker", "requirements", "ci", "classed", "build_files"];

/// Two versions compared as versions: numbers separated by dots, at least three, compared number by number, so
/// that 0.10.0 is newer than 0.0.9. `None` where either is not of that form.
pub fn compare(a: &str, b: &str) -> Option<Ordering> {
    fn numbers(version: &str) -> Option<Vec<u64>> {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 3 {
            return None;
        }
        parts
            .iter()
            .map(|part| if !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()) { part.parse().ok() } else { None })
            .collect()
    }
    let (a, b) = (numbers(a)?, numbers(b)?);
    for at in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(at).copied().unwrap_or(0), b.get(at).copied().unwrap_or(0));
        if x != y {
            return Some(x.cmp(&y));
        }
    }
    Some(Ordering::Equal)
}

/// One write to the files of the revision, with the path relative to `.watchbill/`.
enum Step {
    RemoveFile(String),
    RemoveFolder(String),
    /// A file and its bytes; `added` where nothing is there now.
    Write { path: String, bytes: Vec<u8>, added: bool },
}

impl Step {
    fn said(&self) -> String {
        let name = |path: &str| format!("{}/{}", CONFIG_FOLDER, path);
        match self {
            Step::RemoveFile(path) => format!("  remove  {}", name(path)),
            Step::RemoveFolder(path) => format!("  remove  {}/", name(path)),
            Step::Write { path, added: true, .. } => format!("  add     {}", name(path)),
            Step::Write { path, added: false, .. } => format!("  change  {}", name(path)),
        }
    }
}

/// What is under `.watchbill/revision/`: each file with its bytes, and each folder. A link, or a thing that is
/// neither a file nor a folder, is refused by name and never followed.
/// The most the files under `.watchbill/revision/` are held in memory together.
const MOST_TOTAL: u64 = 64 * 1024 * 1024;

fn look_under(base: &Path, folder: &Path, files: &mut BTreeMap<String, Vec<u8>>, folders: &mut Vec<String>, total: &mut u64) -> Result<(), String> {
    let shown = |path: &Path| {
        let relative = path.strip_prefix(base).unwrap_or(path);
        format!("{}/{}", CONFIG_FOLDER, crate::word(relative.as_os_str().as_bytes()))
    };
    let entries = fs::read_dir(folder).map_err(|error| format!("{} cannot be listed ({})", shown(folder), error))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("{} cannot be listed ({})", shown(folder), error))?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path).map_err(|error| format!("{} cannot be looked at ({})", shown(&path), error))?;
        let Some(relative) = path.strip_prefix(base).ok().and_then(|relative| relative.to_str()) else {
            return Err(format!("{}: the name is not UTF-8", shown(&path)));
        };
        if meta.file_type().is_symlink() {
            return Err(format!("{} is a link, and wb upgrade never follows one", shown(&path)));
        } else if meta.is_dir() {
            folders.push(relative.to_string());
            look_under(base, &path, files, folders, total)?;
        } else if meta.is_file() {
            let bytes = crate::bounded::read_plain(&path, crate::bounded::REVISION_CAP).map_err(|error| format!("{} cannot be read ({})", shown(&path), error))?;
            *total += bytes.len() as u64;
            if *total > MOST_TOTAL {
                return Err(format!("the files under {}/revision are over 64 MiB in all, which is too much to read; wb upgrade reads none of them further", CONFIG_FOLDER));
            }
            files.insert(relative.to_string(), bytes);
        } else {
            return Err(format!("{} is not a plain file", shown(&path)));
        }
    }
    Ok(())
}

/// The writes that make the files of the revision those of the installed one: the removals first, files then
/// folders below their files, then the files of the revision, then its manifest.
fn file_steps(base: &Path, pin: &[(String, Vec<u8>)]) -> Result<Vec<Step>, String> {
    let manifest = base.join(MANIFEST_FILE);
    let mut manifest_now = None;
    match fs::symlink_metadata(&manifest) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("{}/{} cannot be looked at ({})", CONFIG_FOLDER, MANIFEST_FILE, error)),
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(format!("{}/{} is a link, and wb upgrade never follows one", CONFIG_FOLDER, MANIFEST_FILE))
        }
        Ok(meta) if !meta.is_file() => return Err(format!("{}/{} is not a plain file", CONFIG_FOLDER, MANIFEST_FILE)),
        Ok(_) => {
            manifest_now =
                Some(crate::bounded::read_plain(&manifest, crate::bounded::REVISION_CAP).map_err(|error| format!("{}/{} cannot be read ({})", CONFIG_FOLDER, MANIFEST_FILE, error))?)
        }
    }
    let revision = base.join("revision");
    let mut files = BTreeMap::new();
    let mut folders = Vec::new();
    match fs::symlink_metadata(&revision) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("{}/revision cannot be looked at ({})", CONFIG_FOLDER, error)),
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(format!("{}/revision is a link, and wb upgrade never follows one", CONFIG_FOLDER))
        }
        Ok(meta) if !meta.is_dir() => return Err(format!("{}/revision is not a folder", CONFIG_FOLDER)),
        Ok(_) => look_under(base, &revision, &mut files, &mut folders, &mut 0)?,
    }
    let installed: BTreeMap<&str, &[u8]> =
        pin.iter().filter(|(path, _)| path != MANIFEST_FILE).map(|(path, bytes)| (path.as_str(), bytes.as_slice())).collect();
    let mut steps = Vec::new();
    for path in files.keys().filter(|path| !installed.contains_key(path.as_str())) {
        steps.push(Step::RemoveFile(path.clone()));
    }
    // A folder that holds no file of the installed revision goes; a folder below another comes first.
    folders.sort();
    for folder in folders.iter().rev() {
        let below = format!("{}/", folder);
        if !installed.keys().any(|path| path.starts_with(&below)) {
            steps.push(Step::RemoveFolder(folder.clone()));
        }
    }
    for (path, bytes) in &installed {
        match files.get(*path) {
            Some(now) if now.as_slice() == *bytes => {}
            now => steps.push(Step::Write { path: path.to_string(), bytes: bytes.to_vec(), added: now.is_none() }),
        }
    }
    if let Some((_, bytes)) = pin.iter().find(|(path, _)| path == MANIFEST_FILE) {
        if manifest_now.as_deref() != Some(bytes.as_slice()) {
            steps.push(Step::Write { path: MANIFEST_FILE.to_string(), bytes: bytes.clone(), added: manifest_now.is_none() });
        }
    }
    Ok(steps)
}

/// The same value, as the loader reads it, wherever it stands in the file.
fn same(a: &Node, b: &Node) -> bool {
    match (&a.value, &b.value) {
        (Value::Plain(x, r), Value::Plain(y, s)) => x == y && r == s,
        (Value::Quoted(x), Value::Quoted(y)) => x == y,
        (Value::List(x), Value::List(y)) => x.len() == y.len() && x.iter().zip(y).all(|(x, y)| same(x, y)),
        (Value::Table(x), Value::Table(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|((k, x), (l, y))| k.text == l.text && same(x, y))
        }
        _ => false,
    }
}

fn entries(document: &Document) -> &[(yamlfile::Key, Node)] {
    match &document.root().value {
        Value::Table(entries) => entries,
        _ => &[],
    }
}

/// The new text of `watchbill.yml` and what is said of each line set or added, or why the lines cannot be set.
/// `wanted` are the two lines, `added` the patterns the file lacks, each with the value it is to read as.
fn line_steps(text: &str, document: &Document, wanted: &[(&str, String)], added: &[(String, String, String)]) -> Result<(String, Vec<String>), String> {
    let place = format!("{}/{}", CONFIG_FOLDER, CONFIG_FILE);
    let by_hand = |why: &str| format!("{} {}; set the lines watchbill: and revision: by hand", place, why);
    if text.contains('\r') {
        return Err(by_hand("has CR in it, and wb upgrade writes lines that end with LF only"));
    }
    let mut lines: Vec<String> = text.split_inclusive('\n').map(str::to_string).collect();
    let mut appended: Vec<(String, String)> = Vec::new();
    let mut said = Vec::new();
    for (key, value) in wanted {
        let line = format!("{}: {}", key, init::yaml_value(value));
        let found = entries(document).iter().find(|(name, _)| name.text == *key);
        match found {
            Some((name, node)) => {
                if document.text(&[Part::from(*key)]).ok().as_deref() == Some(value.as_str()) {
                    continue;
                }
                let at = name.line - 1;
                let prefix = format!("{}:", key);
                let follows = lines.get(at + 1).map(|next| next.starts_with([' ', '\t']) && !next.trim().is_empty()).unwrap_or(false);
                if node.line != name.line || follows || !lines.get(at).is_some_and(|now| now.starts_with(&prefix)) {
                    return Err(by_hand(&format!("has the line {}: not as one line of its own", key)));
                }
                let was = lines[at][prefix.len()..].trim().to_string();
                said.push(format!("  set     {}: {} (was {})", place, line, was));
                lines[at] = format!("{}\n", line);
            }
            None => {
                said.push(format!("  add     {}: {}", place, line));
                appended.push((key.to_string(), line));
            }
        }
    }
    for (key, line, _) in added {
        said.push(format!("  add     {}: {}", place, line));
        appended.push((key.clone(), line.clone()));
    }
    if !appended.is_empty() {
        if let Some(last) = lines.last_mut() {
            if !last.ends_with('\n') {
                last.push('\n');
            }
        }
        lines.extend(appended.iter().map(|(_, line)| format!("{}\n", line)));
    }
    let new = lines.concat();
    // What the loader reads back is what was meant: the same keys, each other value the same, and the new values.
    let read = yamlfile::load_bytes(document.path(), new.clone().into_bytes()).map_err(|why| by_hand(&format!("would not be read once the lines are set ({})", why)))?;
    let old_keys: Vec<&str> = entries(document).iter().map(|(name, _)| name.text.as_str()).collect();
    let new_keys: Vec<&str> = entries(&read).iter().map(|(name, _)| name.text.as_str()).collect();
    let mut meant = old_keys.clone();
    meant.extend(appended.iter().map(|(key, _)| key.as_str()).filter(|key| !old_keys.contains(key)));
    let mut holds = new_keys == meant;
    for (name, node) in entries(document) {
        if !wanted.iter().any(|(key, _)| *key == name.text) {
            holds &= entries(&read).iter().any(|(other, value)| other.text == name.text && same(node, value));
        }
    }
    for (key, value) in wanted {
        holds &= read.text(&[Part::from(*key)]).ok().as_deref() == Some(value.as_str());
    }
    for (key, _, value) in added {
        holds &= read.text(&[Part::from(key.as_str())]).ok().as_deref() == Some(value.as_str());
    }
    if !holds {
        return Err(by_hand("would not be read back with the same keys and values once the lines are set"));
    }
    Ok(if said.is_empty() { (text.to_string(), said) } else { (new, said) })
}

/// The patterns the installed `wb init` would write for this repository that the file does not have: each key,
/// its line as `wb init` writes it, and the value it reads as.
fn missing_patterns(root: &str, document: &Document, version: &str, hash: &str) -> Result<Vec<(String, String, String)>, String> {
    let text_of = |key: &str| document.text(&[Part::from(key)]).unwrap_or_else(|_| NONE.to_string());
    let setup = Setup {
        project: text_of("project"),
        git_server: text_of("git_server"),
        repository: text_of("repository"),
        tracker: text_of("tracker"),
        requirements: text_of("requirements"),
        ci: text_of("ci"),
    };
    let host = if setup.git_server == "gitlab" { origin_host(root, None) } else { String::new() };
    let rendered = init::render_with(version, hash, &setup, &[], &[], &host);
    let written = yamlfile::load_bytes("wb init", rendered.clone().into_bytes()).map_err(|why| format!("what wb init writes cannot be read: {}", why))?;
    let mut missing = Vec::new();
    for (name, _) in entries(&written) {
        let key = name.text.as_str();
        if SETTINGS.contains(&key) || document.has(&[Part::from(key)]) {
            continue;
        }
        let line = rendered.lines().find(|line| line.starts_with(&format!("{}: ", key))).unwrap_or_default().to_string();
        let value = written.text(&[Part::from(key)]).map_err(|why| why.to_string())?;
        missing.push((key.to_string(), line, value));
    }
    Ok(missing)
}

/// The paths under `.watchbill/` that git does not hold as committed: changed, staged, not tracked or ignored. The
/// index is not written to (`--no-optional-locks`), and no replace ref is read.
fn not_committed(root: &str) -> Result<Vec<String>, String> {
    let run = Command::new("git")
        .args(["--no-optional-locks", "--no-replace-objects", "-C", root])
        .args(["status", "--porcelain=v1", "-z", "--untracked-files=all", "--ignored=matching", "--no-renames", "--", CONFIG_FOLDER])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    match run {
        Ok(run) if run.status.success() => Ok(run
            .stdout
            .split(|byte| *byte == 0)
            .filter(|record| record.len() > 3)
            .map(|record| crate::word(&record[3..]))
            .collect()),
        _ => Err("git cannot say which files under .watchbill/ are committed".to_string()),
    }
}

/// What was done, so that a write that fails part way can be put back.
enum Done {
    Wrote(PathBuf, Option<Vec<u8>>),
    RemovedFile(PathBuf, Vec<u8>),
    RemovedFolder(PathBuf),
    MadeFolder(PathBuf),
}

/// Writes a file whole: to a new file beside it, then in its place, so that it is never half written.
fn replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let aside = path.with_file_name(format!(".{}.wb-upgrade", name));
    let written = (|| {
        let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&aside)?;
        file.write_all(bytes)?;
        file.sync_all()
    })();
    let done = written.and_then(|_| fs::rename(&aside, path));
    if done.is_err() {
        let _ = fs::remove_file(&aside);
    }
    done
}

fn apply(base: &Path, steps: &[Step], config: &Path, yml: &[u8], old_yml: &[u8], done: &mut Vec<Done>) -> Result<(), (PathBuf, io::Error)> {
    for step in steps {
        match step {
            Step::RemoveFile(path) => {
                let full = base.join(path);
                let old = crate::bounded::read_plain(&full, crate::bounded::REVISION_CAP).map_err(|fault| (full.clone(), fault.into_io()))?;
                fs::remove_file(&full).map_err(|error| (full.clone(), error))?;
                done.push(Done::RemovedFile(full, old));
            }
            Step::RemoveFolder(path) => {
                let full = base.join(path);
                fs::remove_dir(&full).map_err(|error| (full.clone(), error))?;
                done.push(Done::RemovedFolder(full));
            }
            Step::Write { path, bytes, .. } => {
                let full = base.join(path);
                let mut missing = Vec::new();
                let mut above = full.parent();
                while let Some(folder) = above {
                    if fs::symlink_metadata(folder).is_ok() {
                        break;
                    }
                    missing.push(folder.to_path_buf());
                    above = folder.parent();
                }
                for folder in missing.into_iter().rev() {
                    fs::create_dir(&folder).map_err(|error| (folder.clone(), error))?;
                    done.push(Done::MadeFolder(folder));
                }
                let old = crate::bounded::read_plain(&full, crate::bounded::REVISION_CAP).ok();
                replace(&full, bytes).map_err(|error| (full.clone(), error))?;
                done.push(Done::Wrote(full, old));
            }
        }
    }
    if yml != old_yml {
        replace(config, yml).map_err(|error| (config.to_path_buf(), error))?;
        done.push(Done::Wrote(config.to_path_buf(), Some(old_yml.to_vec())));
    }
    Ok(())
}

/// Puts back what `apply` did, the last first.
fn put_back(done: Vec<Done>) {
    for step in done.into_iter().rev() {
        let _ = match step {
            Done::Wrote(path, Some(old)) => replace(&path, &old),
            Done::Wrote(path, None) => fs::remove_file(&path),
            Done::RemovedFile(path, old) => fs::OpenOptions::new().write(true).create_new(true).open(&path).and_then(|mut file| file.write_all(&old)),
            Done::RemovedFolder(path) => fs::create_dir(&path),
            Done::MadeFolder(path) => fs::remove_dir(&path),
        };
    }
}

fn refused(err: &mut dyn Write, why: &str) -> i32 {
    crate::refusal::limit(err, "wb upgrade", format_args!("{}. Nothing written.", why))
}

/// Runs `wb upgrade` in the repository at `root`, against the installed `wb`: its `version` and the revision its
/// package holds in `package`. 0 done and the check holds, or nothing to do; 1 refused, or the check does not
/// hold; 2 the repository is not set up.
pub fn upgrade(root: &str, version: &str, package: &Path, dry_run: bool, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let base = Path::new(root).join(CONFIG_FOLDER);
    let config = base.join(CONFIG_FILE);
    let place = format!("{}/{}", CONFIG_FOLDER, CONFIG_FILE);
    let not_set_up = |err: &mut dyn Write| {
        let _ = writeln!(err, "wb upgrade: {} is not there. Watchbill is not set up in this repository: wb init sets it up.", place);
        2
    };
    match fs::symlink_metadata(&base) {
        Ok(meta) if meta.file_type().is_symlink() => return refused(err, &format!("{} is a link, and wb upgrade never follows one", CONFIG_FOLDER)),
        Ok(meta) if meta.is_dir() => {}
        _ => return not_set_up(err),
    }
    match fs::symlink_metadata(&config) {
        Ok(meta) if meta.file_type().is_symlink() => return refused(err, &format!("{} is a link, and wb upgrade never follows one", place)),
        Ok(meta) if !meta.is_file() => return refused(err, &format!("{} is not a plain file", place)),
        Ok(_) => {}
        Err(_) => return not_set_up(err),
    }
    let pin = match init::read_pin(package) {
        Ok(pin) => pin,
        Err(why) => return refused(err, &format!("the revision of the installed wb cannot be read: {}", why)),
    };
    let old_yml = match crate::bounded::read_plain(&config, crate::bounded::TEXT_CAP) {
        Ok(bytes) => bytes,
        Err(error) => return refused(err, &format!("{} cannot be read ({})", place, error)),
    };
    let document = match yamlfile::load_bytes(&config, old_yml.clone()) {
        Ok(document) => document,
        Err(why) => return refused(err, &why.to_string()),
    };
    if !matches!(document.root().value, Value::Table(_)) {
        return refused(err, &format!("{} is not a mapping of keys and values", place));
    }
    let pinned = if document.has(&[Part::from("watchbill")]) {
        match document.text(&[Part::from("watchbill")]) {
            Ok(pinned) => Some(pinned),
            Err(why) => return refused(err, &why.to_string()),
        }
    } else {
        None
    };
    if let Some(pinned) = &pinned {
        match compare(version, pinned) {
            None => {
                return refused(
                    err,
                    &format!(
                        "the version the repository is pinned to, {}, cannot be compared with the installed version {}: a version is numbers separated by dots",
                        pinned, version
                    ),
                )
            }
            Some(Ordering::Less) => {
                return refused(
                    err,
                    &format!(
                        "the installed wb is version {}, older than version {} that the repository is pinned to. Install wb {} or newer",
                        version, pinned, pinned
                    ),
                )
            }
            _ => {}
        }
    }
    let pinned_hash = document.text(&[Part::from("revision")]).ok();

    // What there is to write.
    let steps = match file_steps(&base, &pin.files) {
        Ok(steps) => steps,
        Err(why) => return refused(err, &why),
    };
    let added = match missing_patterns(root, &document, version, &pin.hash) {
        Ok(added) => added,
        Err(why) => return refused(err, &why),
    };
    let wanted = [("watchbill", version.to_string()), ("revision", pin.hash.clone())];
    let old_text = String::from_utf8_lossy(&old_yml).into_owned();
    let (yml, lines) = match line_steps(&old_text, &document, &wanted, &added) {
        Ok(found) => found,
        Err(why) => return refused(err, &why),
    };
    // The fetch line of the jobs (#234, point 15), where the remote `origin` is there and lacks it.
    let fetch = match crate::plan::fetch_missing(root) {
        Ok(fetch) => fetch,
        Err(why) => return refused(err, &why),
    };
    if steps.is_empty() && lines.is_empty() {
        if !fetch {
            crate::refusal::inform(
                out,
                format_args!("wb upgrade: the repository is at the installed revision, version {}, revision {}. Nothing to do: nothing written.", version, pin.hash),
            );
            return 0;
        }
        // Only the fetch line: nothing under .watchbill/ changes, so there is no check to run again and nothing to commit.
        let _ = writeln!(out, "wb upgrade: the repository is at the installed revision, version {}, revision {}.", version, pin.hash);
        let _ = writeln!(out, "{}", if dry_run { "It would write this:" } else { "It writes this:" });
        let _ = writeln!(out, "  add {}", crate::plan::fetch_said());
        if dry_run {
            let _ = writeln!(out, "--dry-run: Nothing written.");
            return 0;
        }
        crate::interrupt::writing();
        if let Err(why) = crate::plan::add_fetch(root) {
            return refused(err, &format!("the fetch line {} could not be added: {}", crate::plan::FETCH, why));
        }
        crate::refusal::inform(out, format_args!("\nWritten."));
        return 0;
    }
    match not_committed(root) {
        Err(why) => return refused(err, &why),
        Ok(paths) if !paths.is_empty() => {
            let code = crate::refusal::limit(err, "wb upgrade", format_args!("these paths under {}/ are not as git holds them committed:", CONFIG_FOLDER));
            for path in &paths {
                let _ = writeln!(err, "  {}", path);
            }
            let _ = writeln!(
                err,
                "wb upgrade writes over the files of the revision only where git holds them, so that git gives them back. Commit each change, or take it back with git, first. Nothing written."
            );
            return code;
        }
        Ok(_) => {}
    }

    let _ = writeln!(
        out,
        "wb upgrade: the repository is pinned to version {}, revision {}; the installed wb is version {}, revision {}.",
        pinned.as_deref().unwrap_or("(none)"),
        pinned_hash.as_deref().unwrap_or("(none)"),
        version,
        pin.hash
    );
    let _ = writeln!(out, "{}", if dry_run { "It would write these, in this order:" } else { "It writes these, in this order:" });
    for step in &steps {
        let _ = writeln!(out, "{}", step.said());
    }
    for line in &lines {
        let _ = writeln!(out, "{}", line);
    }
    if fetch {
        let _ = writeln!(out, "  add {}", crate::plan::fetch_said());
    }
    if dry_run {
        let _ = writeln!(out, "--dry-run: Nothing written.");
        return 0;
    }

    crate::interrupt::writing();
    let mut done = Vec::new();
    if let Err((path, error)) = apply(&base, &steps, &config, yml.as_bytes(), &old_yml, &mut done) {
        put_back(done);
        let shown = path.strip_prefix(root).map(|path| path.to_path_buf()).unwrap_or(path);
        return crate::refusal::limit(err, "wb upgrade", format_args!("cannot write {}: {}. What it had written is put back. Nothing written.", shown.display(), error));
    }
    if fetch {
        if let Err(why) = crate::plan::add_fetch(root) {
            return crate::refusal::limit(err, "wb upgrade", format_args!("the fetch line {} could not be added: {}. The files are written.", crate::plan::FETCH, why));
        }
    }
    crate::refusal::inform(out, format_args!("\nWritten.\n\nThe check of wb check:"));
    // not a refusal: the check after the upgrade is an answer, which `check` marks; it ends with 1 where it does not hold.
    let checked = crate::check::check(root, version, package, out, err);
    crate::refusal::inform(
        out,
        format_args!(
            "\nNothing is committed. Commit the change alone, with nothing else in the commit; until {} is committed, wb runs none of its patterns (#186):\n  git add --all -- ':/{1}' && git commit -m 'Watchbill {2}' -- ':/{1}'",
            place, CONFIG_FOLDER, version
        ),
    );
    checked
}
