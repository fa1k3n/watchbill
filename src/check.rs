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
#[derive(PartialEq)]
enum Kind {
    File,
    Folder,
    Link,
    Other,
}

/// The paths under `folder`, relative to `base` and written with `/`. A folder is gone into; a link
/// is never followed, and nothing that is not a folder or a plain file is ever opened.
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
            Ok(meta) if meta.file_type().is_symlink() => {
                found.insert(shown, Kind::Link);
            }
            Ok(meta) if meta.is_dir() => {
                found.insert(shown, Kind::Folder);
                walk(base, &path, found);
            }
            Ok(meta) if meta.is_file() => {
                found.insert(shown, Kind::File);
            }
            _ => {
                found.insert(shown, Kind::Other);
            }
        }
    }
}

/// What is at a path, looked at without following a link: `None` when nothing is there.
fn look(path: &Path) -> Option<Kind> {
    let meta = fs::symlink_metadata(path).ok()?;
    Some(if meta.file_type().is_symlink() {
        Kind::Link
    } else if meta.is_dir() {
        Kind::Folder
    } else if meta.is_file() {
        Kind::File
    } else {
        Kind::Other
    })
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

const LINK: &str = "(a link is never followed)";

/// A fault in `watchbill.yml` as a line. The loader's message starts with the path of the file, and
/// the others name it themselves: the file is named once.
fn about_config(why: &dyn std::fmt::Display) -> String {
    let why = why.to_string();
    if why.contains(CONFIG_FILE) {
        why
    } else {
        format!("{}: {}", CONFIG_FILE, why)
    }
}

fn finish(found: &[String], settings: usize, unpinned: bool, project: &[String], out: &mut dyn Write) -> i32 {
    if found.is_empty() && project.is_empty() {
        crate::refusal::inform(out, format_args!("The repository is at the revision it is pinned to, and the installed wb is that revision."));
        return 0;
    }
    for line in found.iter().chain(project) {
        let _ = writeln!(out, "{}", line);
    }
    let count = found.len() + project.len();
    let plural = if count == 1 { "" } else { "s" };
    if found.is_empty() {
        let _ = writeln!(out, "{} difference{}. The project's text cannot be used with the revision it is pinned to.", count, plural);
    } else if found.len() == settings && project.is_empty() && unpinned && found.len() == 1 {
        let _ = writeln!(out, "{} difference{}. {} pins no revision.", count, plural, CONFIG_FILE);
    } else if found.len() == settings && project.is_empty() {
        let _ = writeln!(out, "{} difference{}. {} does not fit the revision it is pinned to.", count, plural, CONFIG_FILE);
    } else if found.len() == settings {
        let _ = writeln!(out, "{} difference{}. {} and the project's text do not fit the revision it is pinned to.", count, plural, CONFIG_FILE);
    } else {
        let _ = writeln!(out, "{} difference{}. The repository is not at the revision it is pinned to.", count, plural);
    }
    // A difference is the answer of the check, on standard output, as the plumbing has it: not a refusal.
    crate::refusal::answer();
    1
}

/// Checks the repository at `root` against the installed `wb`: its `version` and the revision its
/// package holds in `package`. 0 all holds, 1 something differs, 2 the repository is not set up.
/// A link is never followed: it is named, and what is behind it is not looked at.
pub fn check(root: &str, version: &str, package: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let folder = Path::new(root).join(CONFIG_FOLDER);
    let config = folder.join(CONFIG_FILE);
    let name = |path: &str| format!("{}/{}", CONFIG_FOLDER, path);
    let mut found: Vec<String> = Vec::new();
    // How many of `found` are faults in the settings of `watchbill.yml`.
    let mut settings = 0;
    let mut unpinned = false;
    match look(&folder) {
        Some(Kind::Link) => {
            found.push(format!("link: {} {}", CONFIG_FOLDER, LINK));
            return finish(&found, 0, false, &[], out);
        }
        Some(Kind::Folder) => {}
        _ => {
            let _ = writeln!(err, "wb check: {} is not there. Watchbill is not set up in this repository.", name(CONFIG_FILE));
            return 2;
        }
    }
    let config_kind = look(&config);
    if config_kind.is_none() {
        let _ = writeln!(err, "wb check: {} is not there. Watchbill is not set up in this repository.", name(CONFIG_FILE));
        return 2;
    }

    // The files of the revision against the manifest.
    let manifest_path = folder.join(MANIFEST_FILE);
    let mut pinned_hash = None;
    let mut listed = Vec::new();
    match look(&manifest_path) {
        None => found.push(format!("missing: {}", name(MANIFEST_FILE))),
        Some(Kind::Link) => found.push(format!("link: {} {}", name(MANIFEST_FILE), LINK)),
        Some(Kind::File) => match crate::bounded::read_plain(&manifest_path, crate::bounded::REVISION_CAP) {
            Ok(bytes) => {
                pinned_hash = Some(manifest::hash_bytes(&bytes));
                listed = parse(&String::from_utf8_lossy(&bytes), &mut found);
            }
            Err(error) => found.push(format!("changed: {} (it cannot be read: {})", name(MANIFEST_FILE), error)),
        },
        Some(_) => found.push(format!("changed: {} (it is not a plain file)", name(MANIFEST_FILE))),
    }
    if pinned_hash.is_some() {
        let mut on_disk = BTreeMap::new();
        let revision = folder.join("revision");
        // Below a link, or a thing that is not a folder, nothing is looked at.
        let looked_into = match look(&revision) {
            Some(Kind::Link) => {
                found.push(format!("link: {} {}", name("revision"), LINK));
                false
            }
            Some(Kind::Folder) | None => {
                walk(&folder, &revision, &mut on_disk);
                true
            }
            Some(_) => {
                found.push(format!("changed: {} (it is not a folder)", name("revision")));
                false
            }
        };
        if looked_into {
            for (hash, path) in &listed {
                // What is below a link is not looked at: the link is named once.
                let mut above = String::new();
                let below_link = path.split('/').rev().skip(1).collect::<Vec<_>>().into_iter().rev().any(|part| {
                    if !above.is_empty() {
                        above.push('/');
                    }
                    above.push_str(part);
                    on_disk.get(&above) == Some(&Kind::Link)
                });
                if below_link {
                    continue;
                }
                match on_disk.get(path) {
                    None => found.push(format!("missing: {}", name(path))),
                    Some(Kind::Folder) => found.push(format!("changed: {} (it is a folder, not a plain file)", name(path))),
                    Some(Kind::Link) => found.push(format!("changed: {} (it is a link, {})", name(path), "never followed")),
                    Some(Kind::Other) => found.push(format!("changed: {} (it is not a plain file)", name(path))),
                    Some(Kind::File) => match crate::bounded::read_plain(&folder.join(path), crate::bounded::REVISION_CAP) {
                        Ok(bytes) if manifest::hash_bytes(&bytes) == *hash => {}
                        Ok(_) => found.push(format!("changed: {}", name(path))),
                        Err(error) => found.push(format!("changed: {} (it cannot be read: {})", name(path), error)),
                    },
                }
            }
            for (path, kind) in &on_disk {
                let is_listed = listed.iter().any(|(_, listed)| listed == path);
                let below = format!("{}/", path);
                match kind {
                    Kind::File if !is_listed => found.push(format!("not of the revision: {}", name(path))),
                    Kind::Other if !is_listed => {
                        found.push(format!("not of the revision: {} (it is not a plain file)", name(path)))
                    }
                    Kind::Link if !is_listed => found.push(format!("link: {} {}", name(path), LINK)),
                    Kind::Folder if !is_listed && !listed.iter().any(|(_, listed)| listed.starts_with(&below)) => {
                        found.push(format!("not of the revision: {}/ (a folder the manifest does not account for)", name(path)))
                    }
                    _ => {}
                }
            }
        }
    }

    // The manifest against the line `revision:`, and the pin against the installed `wb`.
    match config_kind {
        Some(Kind::Link) => found.push(format!("link: {} {}", name(CONFIG_FILE), LINK)),
        Some(Kind::File) => match load(&config) {
            Err(why) => {
                found.push(about_config(&why));
                settings += 1;
            }
            Ok(document) => {
                let line = document.text(&[Part::from("revision")]);
                let pinned_version = document.text(&[Part::from("watchbill")]);
                match (&line, &pinned_hash) {
                    (Err(_), _) if !document.has(&[Part::from("revision")]) => {
                        found.push(format!("{} pins no revision: the line `revision:` is not there", CONFIG_FILE));
                        settings += 1;
                        unpinned = true;
                    }
                    (Err(why), _) => {
                        found.push(about_config(&why));
                        settings += 1;
                    }
                    (Ok(line), Some(hash)) if line != hash => found.push(format!(
                        "the manifest {} has the hash {}, and the line revision of {} says {}",
                        MANIFEST_FILE, hash, CONFIG_FILE, line
                    )),
                    _ => {}
                }
                if let Err(why) = &pinned_version {
                    found.push(about_config(&why));
                    settings += 1;
                }
                // The key `breakdown` is refused by name, as `wb prompt` refuses it (#116).
                if let Err(why) = crate::issue::no_breakdown(&document) {
                    found.push(about_config(&why));
                    settings += 1;
                }
                // The limits of a session (#267): lower than the rule table's, never higher.
                if let Some(why) = crate::watchdog::fault(&document) {
                    found.push(about_config(&why));
                    settings += 1;
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
        },
        _ => found.push(format!("changed: {} (it is not a plain file)", name(CONFIG_FILE))),
    }
    // The project's own text, with the placeholders judged as `wb prompt` judges them, and the project's keys (#235),
    // as the working tree has them, so that a fault is named before the file is merged.
    let project = crate::prompt::unknown_in_project(Path::new(root));
    // A list that is not there is a difference too (#323): without it on the default branch nothing counts.
    let keys = crate::signers::check(Path::new(root));
    if !keys.is_empty() && found.is_empty() && project.is_empty() {
        for line in &keys {
            let _ = writeln!(out, "{}", line);
        }
        let plural = if keys.len() == 1 { "" } else { "s" };
        if keys == [crate::signers::ABSENT] {
            let _ = writeln!(out, "{} difference. {} is not there. {}.", keys.len(), crate::signers::PATH, crate::signers::add_a_key("accept,start"));
        } else {
            let _ = writeln!(out, "{} difference{}. {} does not hold its form: on the default branch, no acceptance would count.", keys.len(), plural, crate::signers::PATH);
        }
        crate::refusal::answer();
        return 1;
    }
    let mut project = project;
    project.extend(keys);
    finish(&found, settings, unpinned, &project, out)
}
