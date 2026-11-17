//! `wb check`: whether a repository still is at the revision it is pinned to, and whether the
//! installed `wb` is that revision.
//!
//! It compares three things and writes nothing: each file under `.watchbill/revision/` with the
//! manifest `.watchbill/revision.sha256`; the manifest with the line `revision:` of
//! `watchbill.yml`; and the version and the hash that line and `watchbill:` give with those of the
//! installed `wb`. Every difference is named. Files are read as bytes and are not refused for their
//! form: a file that is not as it was shipped simply has another hash.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

use crate::init::{CONFIG_FILE, CONFIG_FOLDER};
use crate::manifest::{self, MANIFEST_FILE};
use crate::yamlfile::{load, Part};

/// What a path under `revision/` is.
enum Kind {
    File,
    Other,
}

/// The paths under `folder`, relative to `base` and written with `/`. A folder is gone into; a link
/// or anything that is not a plain file is `Other`, and is never opened.
fn walk(base: &Path, folder: &Path, found: &mut BTreeMap<String, Kind>) {
    let Ok(entries) = fs::read_dir(folder) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let relative = path.strip_prefix(base).unwrap_or(&path);
        let shown = relative
            .components()
            .map(|part| crate::word(part.as_os_str().as_bytes()))
            .collect::<Vec<_>>()
            .join("/");
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.is_dir() => walk(base, &path, found),
            Ok(meta) if meta.is_file() => {
                found.insert(shown, Kind::File);
            }
            _ => {
                found.insert(shown, Kind::Other);
            }
        }
    }
}

fn is_hash(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// The lines of the manifest as `(hash, path)`; a line that is not in the form of `sha256sum` for a
/// file under `revision/` is a difference of its own.
fn parse(manifest: &str, found: &mut Vec<String>) -> Vec<(String, String)> {
    let mut lines = Vec::new();
    for (index, line) in manifest.lines().enumerate() {
        let parts = line.split_once("  ");
        let good = parts.filter(|(hash, path)| {
            is_hash(hash)
                && path.starts_with("revision/")
                && !path.contains('\\')
                && path.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
        });
        match good {
            Some((hash, path)) => lines.push((hash.to_string(), path.to_string())),
            None => found.push(format!(
                "manifest: line {} of {} is not `<hash>  <path of a file under revision/>`",
                index + 1,
                MANIFEST_FILE
            )),
        }
    }
    lines
}

/// Checks the repository at `root` against the installed `wb`: its `version` and the revision its
/// package holds in `package`. 0 all holds, 1 something differs, 2 the repository is not set up.
pub fn check(root: &str, version: &str, package: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let folder = Path::new(root).join(CONFIG_FOLDER);
    let config = folder.join(CONFIG_FILE);
    if !config.is_file() {
        let _ = writeln!(
            err,
            "wb check: {}/{} is not there. Watchbill is not set up in this repository.",
            CONFIG_FOLDER, CONFIG_FILE
        );
        return 2;
    }
    let mut found: Vec<String> = Vec::new();

    // The files of the revision against the manifest.
    let manifest_path = folder.join(MANIFEST_FILE);
    let manifest_bytes = fs::read(&manifest_path).ok();
    let mut pinned_hash = None;
    match &manifest_bytes {
        None => found.push(format!("missing: {}/{}", CONFIG_FOLDER, MANIFEST_FILE)),
        Some(bytes) => {
            pinned_hash = Some(manifest::hash_bytes(bytes));
            let listed = parse(&String::from_utf8_lossy(bytes), &mut found);
            let mut on_disk = BTreeMap::new();
            walk(&folder, &folder.join("revision"), &mut on_disk);
            for (hash, path) in &listed {
                match on_disk.get(path) {
                    None => found.push(format!("missing: {}/{}", CONFIG_FOLDER, path)),
                    Some(Kind::Other) => {
                        found.push(format!("changed: {}/{} (it is not a plain file)", CONFIG_FOLDER, path))
                    }
                    Some(Kind::File) => match fs::read(folder.join(path)) {
                        Ok(bytes) if manifest::hash_bytes(&bytes) == *hash => {}
                        Ok(_) => found.push(format!("changed: {}/{}", CONFIG_FOLDER, path)),
                        Err(error) => {
                            found.push(format!("changed: {}/{} (it cannot be read: {})", CONFIG_FOLDER, path, error))
                        }
                    },
                }
            }
            for path in on_disk.keys() {
                if !listed.iter().any(|(_, listed)| listed == path) {
                    found.push(format!("not of the revision: {}/{}", CONFIG_FOLDER, path));
                }
            }
        }
    }

    // The manifest against the line `revision:`, and the pin against the installed `wb`.
    match load(&config) {
        Err(why) => found.push(format!("watchbill.yml: {}", why)),
        Ok(document) => {
            let line = document.text(&[Part::from("revision")]);
            let pinned_version = document.text(&[Part::from("watchbill")]);
            match (&line, &pinned_hash) {
                (Err(why), _) => found.push(format!("watchbill.yml: {}", why)),
                (Ok(line), Some(hash)) if line != hash => found.push(format!(
                    "the manifest {} has the hash {}, and the line revision of {} says {}",
                    MANIFEST_FILE, hash, CONFIG_FILE, line
                )),
                _ => {}
            }
            if let Err(why) = &pinned_version {
                found.push(format!("watchbill.yml: {}", why));
            }
            let installed = manifest::revision_hash(package);
            match (&pinned_version, &line, &installed) {
                (_, _, Err(why)) => found.push(format!("the installed wb cannot read its revision: {}", why)),
                (Ok(pinned_version), Ok(line), Ok(installed)) if pinned_version != version || line != installed => {
                    found.push(format!(
                        "the repository is pinned to version {}, revision {}; the installed wb is version {}, revision {}",
                        pinned_version, line, version, installed
                    ))
                }
                _ => {}
            }
        }
    }

    if found.is_empty() {
        let _ = writeln!(out, "The repository is at the revision it is pinned to, and the installed wb is that revision.");
        return 0;
    }
    for line in &found {
        let _ = writeln!(out, "{}", line);
    }
    let _ = writeln!(out, "{} difference{}. The repository is not at the revision it is pinned to.", found.len(), if found.len() == 1 { "" } else { "s" });
    1
}
