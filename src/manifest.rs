//! The manifest of the revision: its files, the SHA-256 of each, and the hash of the revision.
//!
//! A file is hashed as the bytes that are shipped, and nothing is rewritten first. A file that is not
//! in the fixed form (UTF-8, LF line endings, one line break at the end) is refused by name
//! (docs/revision.md, point 8). The manifest has one line per file in the form `sha256sum` prints,
//! sorted by path, with the paths relative to the folder that holds `revision/`.
//!
//! The folder that holds `revision/` is fixed when `wb` is built: `PACKAGE_FOLDER`. Nothing here
//! reads the environment, and the functions take the folder as an argument only so that a test can
//! give them a folder of its own.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use sha2::{Digest, Sha256};

/// The folder that holds `revision/`, the manifest and `revisions.txt`. Until the files move
/// (#25) it is the folder of the Python package in the checkout.
pub const PACKAGE_FOLDER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/watchbill");
pub const MANIFEST_FILE: &str = "revision.sha256";
pub const RELEASED_FILE: &str = "revisions.txt";

/// `O_NONBLOCK`, the same on every architecture Enterprise Linux 9 is built for (x86_64, aarch64,
/// ppc64le and s390x). Opened with it, a named pipe does not make the open wait for a writer.
const O_NONBLOCK: i32 = 0o4000;

/// A file of the revision is not in the fixed form, or `revisions.txt` is wrong. The message names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestError(String);

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ManifestError {}

fn refuse<T>(message: String) -> Result<T, ManifestError> {
    Err(ManifestError(message))
}

/// The reason of an error as the C library words it, without the number Rust adds.
fn reason(error: &io::Error) -> String {
    let text = error.to_string();
    match text.rfind(" (os error ") {
        Some(at) => text[..at].to_string(),
        None => text,
    }
}

/// A path as text that can be printed, also when its name is not UTF-8.
fn shown(path: &OsStr) -> String {
    crate::word(path.as_bytes())
}

/// Refuses a path the manifest cannot carry safely, by name. `path` is relative to `base`.
fn refuse_name(base: &Path, path: &OsStr, name: &OsStr, is_folder: bool) -> Result<(), ManifestError> {
    let text = match path.to_str() {
        Some(text) => text,
        None => return refuse(format!("{}: the name is not UTF-8", shown(path))),
    };
    let full = base.join(path);
    let looked = fs::symlink_metadata(&full);
    if looked.as_ref().map(|meta| meta.file_type().is_symlink()).unwrap_or(false) {
        return refuse(format!("{}: is a symbolic link; the revision holds only plain files", text));
    }
    if !is_folder {
        match looked {
            Err(error) => return refuse(format!("{}: cannot be read ({})", text, reason(&error))),
            Ok(meta) if !meta.file_type().is_file() => {
                return refuse(format!("{}: is not a plain file", text));
            }
            Ok(_) => {}
        }
    }
    let name = name.to_str().unwrap_or("");
    if name.starts_with('.') {
        return refuse(format!(
            "{}: is hidden; the revision holds no hidden {}",
            text,
            if is_folder { "folder" } else { "file" }
        ));
    }
    if !is_folder && name.ends_with('~') {
        return refuse(format!("{}: is a backup file", text));
    }
    if name.contains('\n') || name.contains('\r') || name.contains('\\') {
        // `sha256sum` writes such a name escaped, and the manifest is in its form.
        return refuse(format!(
            "{}: the name has a line break or a backslash in it",
            text.replace('\n', "\\n").replace('\r', "\\r")
        ));
    }
    Ok(())
}

/// Lists one folder under `base` and checks what is in it, then goes into its folders in order.
fn walk(base: &Path, folder: &OsStr, found: &mut Vec<String>) -> Result<(), ManifestError> {
    let cannot_list = |error: io::Error| {
        ManifestError(format!("{}/: cannot be listed ({})", shown(folder), reason(&error)))
    };
    let mut folders: Vec<OsString> = Vec::new();
    let mut names: Vec<OsString> = Vec::new();
    for entry in fs::read_dir(base.join(folder)).map_err(cannot_list)? {
        let entry = entry.map_err(cannot_list)?;
        // A link to a folder counts as a folder here, as it does for the Python; it is refused below.
        let kind = entry.file_type();
        let is_folder = match kind {
            Ok(kind) if kind.is_dir() => true,
            Ok(kind) if kind.is_symlink() => fs::metadata(entry.path()).map(|meta| meta.is_dir()).unwrap_or(false),
            _ => false,
        };
        if is_folder {
            folders.push(entry.file_name());
        } else {
            names.push(entry.file_name());
        }
    }
    // The order is that of the bytes of the name: no locale, and not the order of the file system.
    folders.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    names.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));

    let below = |name: &OsStr| {
        let mut path = folder.as_bytes().to_vec();
        path.push(b'/');
        path.extend_from_slice(name.as_bytes());
        OsString::from_vec(path)
    };
    for name in folders.iter().chain(names.iter()) {
        refuse_name(base, &below(name), name, folders.contains(name))?;
    }
    for name in &names {
        found.push(below(name).into_string().unwrap_or_default());
    }
    for name in &folders {
        walk(base, &below(name), found)?;
    }
    Ok(())
}

/// The paths of the files under `revision/`, relative to `base`, with `/`, sorted by path.
///
/// The order is that of the code points of the path. A path the manifest cannot carry safely is
/// refused by name, and so is a revision that holds no file.
pub fn files(base: impl AsRef<Path>) -> Result<Vec<String>, ManifestError> {
    let base = base.as_ref();
    let none = || refuse("revision/: holds no file, or is not there".to_string());
    match fs::symlink_metadata(base.join("revision")) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return refuse("revision/: is a symbolic link; the revision holds only plain files".to_string());
        }
        Ok(meta) if meta.is_dir() => {}
        _ => return none(),
    }
    let mut found = Vec::new();
    walk(base, OsStr::new("revision"), &mut found)?;
    if found.is_empty() {
        return none();
    }
    found.sort();
    Ok(found)
}

fn hex(data: &[u8]) -> String {
    Sha256::digest(data).iter().map(|byte| format!("{:02x}", byte)).collect()
}

/// The SHA-256 of one file, which is refused when it is not in the fixed form.
///
/// The file is opened without waiting, and its kind is looked at again once it is open: a file that
/// was a plain file when the folder was listed may be a named pipe by now (WB-REV-24).
fn hash_file(base: &Path, path: &str) -> Result<String, ManifestError> {
    let cannot = |error: io::Error| ManifestError(format!("{}: cannot be read ({})", path, reason(&error)));
    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NONBLOCK)
        .open(base.join(path))
        .map_err(cannot)?;
    if !file.metadata().map_err(cannot)?.file_type().is_file() {
        return refuse(format!("{}: is not a plain file", path));
    }
    let mut data = Vec::new();
    file.read_to_end(&mut data).map_err(cannot)?;
    if std::str::from_utf8(&data).is_err() {
        return refuse(format!("{}: is not UTF-8", path));
    }
    if data.contains(&b'\r') {
        return refuse(format!("{}: has CR in it; the line endings are LF", path));
    }
    if !data.ends_with(b"\n") {
        return refuse(format!("{}: is empty or does not end with a line break", path));
    }
    Ok(hex(&data))
}

/// The manifest of the files under `revision/` in `base`: `<hash>  <path>`, one line each.
pub fn manifest(base: impl AsRef<Path>) -> Result<String, ManifestError> {
    manifest_with(base, &mut |_| {})
}

/// The same, and `after_look` is called with the path of each file after its kind was looked at and
/// before it is opened. A test turns a file into a named pipe there.
pub fn manifest_with(base: impl AsRef<Path>, after_look: &mut dyn FnMut(&str)) -> Result<String, ManifestError> {
    let base = base.as_ref();
    let mut text = String::new();
    for path in files(base)? {
        after_look(&path);
        text.push_str(&format!("{}  {}\n", hash_file(base, &path)?, path));
    }
    Ok(text)
}

/// The hash of the revision, given its manifest: the SHA-256 of the manifest.
pub fn hash_of(text: &str) -> String {
    hex(text.as_bytes())
}

/// The hash of the revision: the SHA-256 of the manifest.
pub fn revision_hash(base: impl AsRef<Path>) -> Result<String, ManifestError> {
    manifest(base).map(|text| hash_of(&text))
}

fn is_hash(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// The versions of `revisions.txt` with the hash of each: lines of `<version>  <hash>`.
pub fn released(path: impl AsRef<Path>) -> Result<Vec<(String, String)>, ManifestError> {
    let path = path.as_ref();
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    let data = fs::read(path).map_err(|error| ManifestError(format!("{}: cannot be read ({})", name, reason(&error))))?;
    let text = match String::from_utf8(data) {
        Ok(text) => text,
        Err(_) => return refuse(format!("{}: is not UTF-8", name)),
    };
    if text.contains('\r') {
        return refuse(format!("{}: has CR in it; the line endings are LF", name));
    }
    if !text.is_empty() && !text.ends_with('\n') {
        return refuse(format!("{}: does not end with a line break", name));
    }
    let mut lines: Vec<&str> = text.split('\n').collect();
    lines.pop();
    let mut found: Vec<(String, String)> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let number = index + 1;
        let parts: Vec<&str> = line.split("  ").collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[0].contains(' ') || !is_hash(parts[1]) {
            return refuse(format!("{}: line {} is not `<version>  <hash>`", name, number));
        }
        if found.iter().any(|(version, _)| version == parts[0]) {
            return refuse(format!("{}: line {}: version {} stands twice", name, number, parts[0]));
        }
        found.push((parts[0].to_string(), parts[1].to_string()));
    }
    Ok(found)
}
