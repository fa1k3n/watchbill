//! The issue's file and its acceptance (`docs/revision.md`, point 13): `.watchbill/issues/<issue>.yml`
//! read through the one loader and held to its form, and `wb accept <issue>`.

use crate::yamlfile::{load, Document, Part, Value, YamlError};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Where the issue files are, below the root of the repository.
pub const FOLDER: &str = ".watchbill/issues";

/// The keys of the file, in the order they stand.
const KEYS: [&str; 8] = [
    "issue",
    "from",
    "requirements",
    "interface",
    "changes",
    "reads",
    "existing_tests_changed",
    "questions",
];

/// The keys of `reads`, in the order they stand.
const READS: [&str; 4] = ["tester", "build", "review", "fix"];

/// The keys of a requirement, in the order they stand. The last, `guard`, may be left out.
const REQUIREMENT: [&str; 4] = ["id", "text", "source", "guard"];

/// One requirement of an issue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Requirement {
    pub id: String,
    pub text: String,
    pub source: String,
    pub guard: Option<String>,
}

/// An issue's file, held to its form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub issue: String,
    pub from: String,
    pub requirements: Vec<Requirement>,
    pub interface: String,
    pub changes: Vec<String>,
    /// The lists of `reads`, by the name of the role, in the order of the file.
    pub reads: Vec<(String, Vec<String>)>,
    pub existing_tests_changed: Vec<String>,
    pub questions: Vec<String>,
}

/// An issue is named by letters, digits, `.`, `_` and `-`, at most 64 of them, and not by a name that starts with `.`.
pub fn is_name(issue: &str) -> bool {
    !issue.is_empty()
        && issue.len() <= 64
        && !issue.starts_with('.')
        && issue.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

/// Text of the file as `wb` shows what it did not make: a control character is written escaped, so
/// that one line stays one line.
pub fn shown(text: &str) -> String {
    text.chars().map(|c| if c.is_control() { c.escape_default().to_string() } else { c.to_string() }).collect()
}

fn fault(error: YamlError) -> String {
    error.to_string()
}

/// Holds the keys of a table to the list: none that is not known, each required one there, and the
/// ones that are there in the order of the list. `required` is how many of the list must be there.
fn keys_of(document: &Document, place: &[Part], keys: &[&str], required: usize) -> Result<(), String> {
    let present = document.table(place).map_err(fault)?;
    let at = |name: &str| {
        let mut at = place.to_vec();
        at.push(Part::from(name));
        at
    };
    for key in &present {
        if !keys.contains(&key.as_str()) {
            let known = keys.iter().map(|name| format!("`{}`", name)).collect::<Vec<_>>().join(", ");
            return Err(document.fail(&at(key), &format!("the key `{}` is not known here; the keys are: {}", key, known), true).to_string());
        }
    }
    for key in &keys[..required] {
        if !present.iter().any(|found| found == key) {
            return Err(document.fail(place, &format!("the key `{}` is missing", key), false).to_string());
        }
    }
    let wanted: Vec<&&str> = keys.iter().filter(|key| present.iter().any(|found| found == **key)).collect();
    for (found, want) in present.iter().zip(wanted) {
        if found != *want {
            return Err(document
                .fail(&at(found), &format!("the key `{}` is out of order; `{}` comes before it", found, want), true)
                .to_string());
        }
    }
    Ok(())
}

/// What is wrong with a path of the file, if anything: it is relative to the root of the repository,
/// has no `..`, and names nothing under `.watchbill/`.
fn path_fault(path: &str) -> Option<&'static str> {
    if path.starts_with('/') {
        return Some("is absolute; a path is relative to the root of the repository");
    }
    if path.split('/').any(|part| part == "..") {
        return Some("holds `..`");
    }
    match path.split('/').find(|part| !part.is_empty() && *part != ".") {
        None => Some("names the root of the repository, not a file of the change"),
        Some(".watchbill") => Some("lies under .watchbill/, which an issue does not name"),
        _ => None,
    }
}

fn paths(document: &Document, place: &[Part]) -> Result<Vec<String>, String> {
    let list = document.texts(place, true).map_err(fault)?;
    for (index, path) in list.iter().enumerate() {
        if let Some(why) = path_fault(path) {
            let mut at = place.to_vec();
            at.push(Part::Index(index));
            return Err(document.fail(&at, &format!("the path '{}' {}", path, why), false).to_string());
        }
    }
    Ok(list)
}

fn text(document: &Document, place: &[&str]) -> Result<String, String> {
    let place: Vec<Part> = place.iter().map(|name| Part::from(*name)).collect();
    document.text(&place).map_err(fault)
}

/// Reads an issue's file through the one loader and holds it to its form. The error names the first
/// fault, with the file and the line.
pub fn read(path: impl AsRef<Path>) -> Result<Issue, String> {
    let document = load(path).map_err(fault)?;
    issue_of(&document)
}

/// As `read`, for the bytes of the file that were read already.
fn read_bytes(path: &Path, bytes: Vec<u8>) -> Result<Issue, String> {
    let document = crate::yamlfile::load_bytes(path, bytes).map_err(fault)?;
    issue_of(&document)
}

fn issue_of(document: &Document) -> Result<Issue, String> {
    keys_of(&document, &[], &KEYS, KEYS.len())?;

    let issue = text(&document, &["issue"])?;
    let from = text(&document, &["from"])?;

    let items = match &document.root().value {
        Value::Table(entries) => match entries.iter().find(|(key, _)| key.text == "requirements") {
            Some((_, node)) => match &node.value {
                Value::List(items) => items.len(),
                _ => return Err(document.fail(&[Part::from("requirements")], "a list of requirements is expected", false).to_string()),
            },
            None => 0,
        },
        _ => 0,
    };
    let mut requirements: Vec<Requirement> = Vec::new();
    for index in 0..items {
        let place = [Part::from("requirements"), Part::Index(index)];
        keys_of(&document, &place, &REQUIREMENT, 3)?;
        let field = |name: &str| {
            let mut at = place.to_vec();
            at.push(Part::from(name));
            at
        };
        let id = document.text(&field("id")).map_err(fault)?;
        if requirements.iter().any(|before| before.id == id) {
            return Err(document.fail(&field("id"), &format!("the requirement ID {} stands twice", id), false).to_string());
        }
        let guard = if document.has(&field("guard")) { Some(document.text(&field("guard")).map_err(fault)?) } else { None };
        requirements.push(Requirement {
            id,
            text: document.text(&field("text")).map_err(fault)?,
            source: document.text(&field("source")).map_err(fault)?,
            guard,
        });
    }

    let interface = text(&document, &["interface"])?;
    let changes = paths(&document, &[Part::from("changes")])?;

    keys_of(&document, &[Part::from("reads")], &READS, READS.len())?;
    let mut reads = Vec::new();
    for role in READS {
        reads.push((role.to_string(), paths(&document, &[Part::from("reads"), Part::from(role)])?));
    }

    let existing_tests_changed = document.texts(&[Part::from("existing_tests_changed")], true).map_err(fault)?;
    let questions = document.texts(&[Part::from("questions")], true).map_err(fault)?;
    Ok(Issue { issue, from, requirements, interface, changes, reads, existing_tests_changed, questions })
}

fn file_of(root: &Path, issue: &str) -> std::path::PathBuf {
    root.join(FOLDER).join(format!("{}.yml", issue))
}

fn accepted_of(root: &Path, issue: &str) -> std::path::PathBuf {
    root.join(FOLDER).join(format!("{}.accepted", issue))
}

/// A folder on the way to the issue files that is a link, or is there and is not a folder: it is named,
/// and nothing is read or written through it.
fn way_fault(root: &Path) -> Option<String> {
    for name in [".watchbill", FOLDER] {
        match fs::symlink_metadata(root.join(name)) {
            Ok(found) if found.file_type().is_symlink() => {
                return Some(format!("{} is a link, and a link is never followed", name))
            }
            Ok(found) if !found.is_dir() => return Some(format!("{} is not a folder", name)),
            _ => {}
        }
    }
    None
}

/// Whether an issue's file is accepted: its `.accepted` is there, and the SHA-256 in it is that of
/// the file as it is now. Nothing else is looked at.
pub fn is_accepted(root: &Path, issue: &str) -> bool {
    if !is_name(issue) || way_fault(root).is_some() {
        return false;
    }
    let plain = |path: &Path| fs::symlink_metadata(path).map(|found| found.is_file()).unwrap_or(false);
    let (file, accepted) = (file_of(root, issue), accepted_of(root, issue));
    if !plain(&file) || !plain(&accepted) {
        return false;
    }
    let (Ok(bytes), Ok(text)) = (fs::read(&file), fs::read_to_string(&accepted)) else {
        return false;
    };
    let hash = crate::manifest::hash_bytes(&bytes);
    text.lines().find_map(|line| line.strip_prefix("sha256: ")) == Some(hash.as_str())
}

/// The roles that are given the issue's file in their prompts.
pub const ROLES: [&str; 4] = ["tester", "build", "review", "fix"];

/// An issue's file that a prompt is made from: read, held to its form, and accepted as it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Used {
    pub issue: Issue,
    /// The path from the root of the repository.
    pub path: String,
    /// The SHA-256 of the file as it was read.
    pub hash: String,
}

/// The most an issue's file may be: it is read whole, and a file larger than an issue needs is refused.
pub const MAX_BYTES: u64 = 1024 * 1024;

/// A character that a value in a prompt never holds: a control character (a line break and a tab
/// are let through where `block` is true, as in the interface), a line or paragraph separator, or a
/// bidirectional control (U+202A to U+202E, U+2066 to U+2069, among them the right-to-left override).
fn is_unsafe(c: char, block: bool) -> bool {
    if block && (c == '\n' || c == '\t') {
        return false;
    }
    c.is_control() || matches!(c, '\u{2028}' | '\u{2029}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// The issue's file for a prompt of a role (`wb prompt`, #76). `Ok(None)` when the issue has no file, so
/// a project goes on without one. When there is one it is used only if it is accepted as it is and
/// holds to its form; else the reason is the error, and no prompt is handed out.
pub fn for_prompt(root: &Path, issue: &str) -> Result<Option<Used>, String> {
    // A name that is not an issue's name (`a/b`, `..`) cannot be the name of an accepted file: `wb accept`
    // refuses it, and the files are looked for by name only. So there is no file to look for, and the
    // prompt goes on as for an issue with none. An issue that fills a pattern is refused by #72 on its own.
    if !is_name(issue) {
        return Ok(None);
    }
    if let Some(why) = way_fault(root) {
        return Err(format!("{}. No prompt is handed out.", why));
    }
    let shown = format!("{}/{}.yml", FOLDER, issue);
    let file = file_of(root, issue);
    match fs::symlink_metadata(&file) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{} cannot be read ({}). No prompt is handed out.", shown, error)),
        Ok(found) if !found.is_file() => return Err(format!("{} is not a plain file. No prompt is handed out.", shown)),
        Ok(_) => {}
    }
    // The file is read once, up to the limit and one byte more: these bytes give the hash, the form
    // and the prompt, so what was checked is what is handed out.
    let too_big = || format!("{} is larger than 1 MiB, and an issue's file is not. No prompt is handed out.", shown);
    let mut bytes = Vec::new();
    let opened = fs::File::open(&file).and_then(|opened| std::io::Read::read_to_end(&mut std::io::Read::take(opened, MAX_BYTES + 1), &mut bytes));
    opened.map_err(|error| format!("{} cannot be read ({}). No prompt is handed out.", shown, error))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(too_big());
    }
    let hash = crate::manifest::hash_bytes(&bytes);
    let accepted = accepted_of(root, issue);
    let body = match fs::symlink_metadata(&accepted) {
        Ok(found) if found.is_file() => fs::read_to_string(&accepted).ok(),
        _ => None,
    };
    let Some(body) = body else {
        return Err(format!(
            "the issue's file {} is not accepted. To accept it, run `wb accept {}`. No prompt is handed out.",
            shown, issue
        ));
    };
    if body.lines().find_map(|line| line.strip_prefix("sha256: ")) != Some(hash.as_str()) {
        return Err(format!(
            "the issue's file {} is not accepted: it changed since its acceptance. Remove {}/{}.accepted and run `wb accept {}` again. No prompt is handed out.",
            shown, FOLDER, issue, issue
        ));
    }
    let prefix = format!("{}/", root.display().to_string().trim_end_matches('/'));
    let read = read_bytes(&file, bytes).map_err(|why| format!("{}. No prompt is handed out.", why.replace(&prefix, "")))?;
    if read.issue != issue {
        return Err(format!("{} says the issue is {}, not {}. No prompt is handed out.", shown, self::shown(&read.issue), issue));
    }
    if !read.questions.is_empty() {
        return Err(format!("{} holds a question, and a file with one is not used. No prompt is handed out.", shown));
    }
    // Every value that reaches a prompt is held to be plain text, and the field is named, never the character.
    let unsafe_in = |name: &str| {
        format!(
            "{}: `{}` holds a control character, a line or paragraph separator or a bidirectional control (such as the right-to-left override). No prompt is handed out.",
            shown, name
        )
    };
    if read.interface.chars().any(|c| is_unsafe(c, true)) {
        return Err(unsafe_in("interface"));
    }
    for (index, requirement) in read.requirements.iter().enumerate() {
        for (field, value) in [("id", &requirement.id), ("text", &requirement.text)] {
            if value.chars().any(|c| is_unsafe(c, false)) {
                return Err(unsafe_in(&format!("requirements[{}].{}", index, field)));
            }
        }
    }
    let lists = std::iter::once(("changes".to_string(), &read.changes))
        .chain(read.reads.iter().map(|(role, list)| (format!("reads.{}", role), list)));
    for (name, list) in lists {
        if list.iter().any(|path| path.chars().any(|c| is_unsafe(c, false))) {
            return Err(unsafe_in(&name));
        }
    }
    Ok(Some(Used { issue: read, path: shown, hash }))
}

impl Used {
    /// What `{requirements}` is filled with: each ID and its text, one to a line.
    pub fn requirements(&self) -> String {
        if self.issue.requirements.is_empty() {
            return "none: the issue's file holds no requirement".to_string();
        }
        let lines: Vec<String> = self.issue.requirements.iter().map(|r| format!("- {}: {}", r.id, r.text)).collect();
        format!("\n{}", lines.join("\n"))
    }

    /// The two parts of a prompt the issue's file adds for a role: the interface, and what the role may
    /// change and read. It is put in as it is, and is never read for placeholders.
    pub fn text(&self, role: &str) -> String {
        let list = |paths: &[String]| -> String {
            if paths.is_empty() {
                return "- none\n".to_string();
            }
            paths.iter().map(|path| format!("- {}\n", path)).collect()
        };
        let reads = self.issue.reads.iter().find(|(name, _)| name == role).map(|(_, paths)| paths.as_slice()).unwrap_or(&[]);
        let interface = if self.issue.interface.ends_with('\n') { self.issue.interface.clone() } else { format!("{}\n", self.issue.interface) };
        format!(
            "## The interface\n\n{}\n## What you may change and read\n\nYou may change:\n\n{}\nYou may read, besides those:\n\n{}",
            interface,
            list(&self.issue.changes),
            list(reads)
        )
    }
}

/// One value of `git config`, or why there is none.
fn git_value(root: &str, key: &str) -> Result<String, String> {
    let none = || format!("git has no {} for this repository. Set it with `git config {} <value>`.", key, key);
    let run = crate::detect::git_output(&["-C", root, "config", key], None).ok_or_else(|| "git could not be run.".to_string())?;
    if !run.status.success() {
        return Err(none());
    }
    let value = String::from_utf8(run.stdout).map_err(|_| format!("the {} of git is not UTF-8 text.", key))?;
    let value = value.trim();
    if value.is_empty() {
        return Err(none());
    }
    if value.chars().any(|c| c.is_control()) {
        return Err(format!("the {} of git holds a control character.", key));
    }
    if value.contains(['<', '>']) {
        return Err(format!("the {} of git holds '<' or '>', and the line `by:` could not be read back one way.", key));
    }
    Ok(value.to_string())
}

/// The roles whose lists are checked by `needs` (#125): they build and run the tests from their view. The
/// review and the fix read the diff, which the view holds.
const BUILDING: [&str; 2] = ["tester", "build"];

/// The list `build_files` of `watchbill.yml` (#125), or `None` when it has none. Each is a file or a folder from
/// the root of the repository, not `/` at its start, with no `..`, and not the root.
pub(crate) fn build_files_of(settings: &Document) -> Result<Option<Vec<String>>, String> {
    let place = [Part::from("build_files")];
    if !settings.has(&place) {
        return Ok(None);
    }
    let paths = settings.texts(&place, true).map_err(fault)?;
    for (index, path) in paths.iter().enumerate() {
        let parts = crate::starter::parts(path);
        if path.starts_with('/') || parts.is_empty() || parts.contains(&"..") {
            let at = [Part::from("build_files"), Part::Index(index)];
            let why = "a path of `build_files` is a file or a folder from the root of the repository, not `/` at its start, with no `..`, and not the root";
            return Err(settings.fail(&at, why, false).to_string());
        }
    }
    Ok(Some(paths))
}

/// The pattern `needs` of `watchbill.yml` (#125), or `None` when it has none.
fn needs_of(settings: &Document) -> Result<Option<String>, String> {
    let place = [Part::from("needs")];
    if !settings.has(&place) {
        return Ok(None);
    }
    settings.text(&place).map(Some).map_err(fault)
}

/// Each path of `changes` that is a file of the build, with the path of `build_files` it meets: it is one,
/// lies under one, or is a folder that holds one.
fn build_hits<'a>(changes: &'a [String], build_files: &'a [String]) -> Vec<(&'a str, &'a str)> {
    changes
        .iter()
        .filter_map(|path| build_files.iter().find(|build| crate::starter::meet(path, build)).map(|build| (path.as_str(), build.as_str())))
        .collect()
}

/// A path as `/bin/sh` reads one word: in single quotes, and a quote in it written `'\''`.
fn quoted(path: &str) -> String {
    format!("'{}'", path.replace('\'', "'\\''"))
}

/// A path that `needs` printed, as its parts from the root: `.` and empty parts left out, and `..` taken back
/// with the part before it, as `cc -MM` prints `src/../include/brake.h`. `None` for a path that is not below
/// the root: one that starts with `/`, or goes above the root.
fn worked_out(path: &str) -> Option<Vec<&str>> {
    if path.starts_with('/') {
        return None;
    }
    let mut found: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                found.pop()?;
            }
            _ => found.push(part),
        }
    }
    (!found.is_empty()).then_some(found)
}

/// What the check of `needs` found: how many files it was run for, and each path printed that is not in the
/// view of a role that sees the file, as the role, the file and the path.
struct Needs {
    run: usize,
    missing: Vec<(&'static str, String, String)>,
}

/// Runs `needs`, filled with each path of `changes` and of the tester's and the build's `reads`, in the root of
/// the checkout (#125). A path it prints is in the view of a role when it is a path of `changes` or of that
/// role's `reads`, or lies under one. A command that ends with anything but 0 is refused with the file named.
fn check_needs(root: &str, pattern: &str, issue: &Issue) -> Result<Needs, String> {
    let reads_of = |role: &str| issue.reads.iter().find(|(name, _)| name == role).map(|(_, list)| list.as_slice()).unwrap_or(&[]);
    let mut files: Vec<&String> = Vec::new();
    for path in issue.changes.iter().chain(BUILDING.iter().flat_map(|role| reads_of(role))) {
        if !files.contains(&path) {
            files.push(path);
        }
    }
    let mut missing: Vec<(&'static str, String, String)> = Vec::new();
    for file in &files {
        let line = pattern.replace("{file}", &quoted(file));
        let printed = crate::starter::run(root, "needs", &line, None).map_err(|why| format!("{} for {}", why, shown(file)))?;
        let printed = String::from_utf8_lossy(&printed);
        for path in printed.lines().map(str::trim).filter(|line| !line.is_empty()) {
            for role in BUILDING {
                let list = reads_of(role);
                if !issue.changes.contains(file) && !list.contains(file) {
                    continue;
                }
                let held = worked_out(path).map_or(false, |path| {
                    issue.changes.iter().chain(list).any(|entry| {
                        let entry = crate::starter::parts(entry);
                        path.len() >= entry.len() && path[..entry.len()] == entry[..]
                    })
                });
                // The path is named as it is from the root, the way it goes in a list.
                let named = worked_out(path).map_or(path.to_string(), |parts| parts.join("/"));
                let miss = (role, file.to_string(), named);
                if !held && !missing.contains(&miss) {
                    missing.push(miss);
                }
            }
        }
    }
    Ok(Needs { run: files.len(), missing })
}

/// `wb accept <issue>` in the repository at `root`: 0 it is accepted, 1 it is refused, 2 Watchbill is
/// not set up here. It says what it will write before it writes, and never writes over an acceptance.
pub fn accept(root: &str, issue: &str, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let base = Path::new(root);
    if let Some(why) = way_fault(base) {
        let _ = writeln!(err, "wb accept: {}. Nothing read, nothing written.", why);
        return 1;
    }
    if !base.join(".watchbill").join("watchbill.yml").is_file() {
        let _ = writeln!(err, "wb accept: .watchbill/watchbill.yml is not there. Watchbill is not set up in this repository.");
        return 2;
    }
    let file = file_of(base, issue);
    let shown = format!("{}/{}.yml", FOLDER, issue);
    match fs::symlink_metadata(&file) {
        Ok(found) if found.is_file() => {}
        Ok(_) => {
            let _ = writeln!(err, "wb accept: {} is not a plain file. Nothing written.", shown);
            return 1;
        }
        Err(_) => {
            let _ = writeln!(err, "wb accept: {} is not there, or cannot be read. Nothing written.", shown);
            return 1;
        }
    }
    let bytes = match fs::read(&file) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = writeln!(err, "wb accept: {} cannot be read ({}). Nothing written.", shown, error);
            return 1;
        }
    };
    let hash = crate::manifest::hash_bytes(&bytes);
    let read = match read(&file) {
        Ok(read) => read,
        Err(why) => {
            // The file is named from the root of the repository, as every other message is.
            let why = why.replace(&format!("{}/", root.trim_end_matches('/')), "");
            let _ = writeln!(err, "wb accept: {}. Nothing written.", why);
            return 1;
        }
    };
    if read.issue != issue {
        let _ = writeln!(err, "wb accept: {} says the issue is {}, not {}. Nothing written.", shown, self::shown(&read.issue), issue);
        return 1;
    }
    if !read.questions.is_empty() {
        let _ = writeln!(err, "wb accept: {} holds {} question{}. Nothing written. Answer them in the file first:", shown, read.questions.len(), if read.questions.len() == 1 { "" } else { "s" });
        for question in &read.questions {
            let _ = writeln!(err, "  - {}", self::shown(question));
        }
        return 1;
    }
    // `watchbill.yml`, through the one loader, for the files of the build and the pattern `needs` (#125).
    let from_root = |why: String| why.replace(&format!("{}/", root.trim_end_matches('/')), "").trim_end_matches('.').to_string();
    let settings = match crate::yamlfile::load(base.join(".watchbill").join("watchbill.yml")) {
        Ok(document) => document,
        Err(why) => {
            let _ = writeln!(err, "wb accept: {}. Nothing written.", from_root(why.to_string()));
            return 1;
        }
    };
    let (build_files, needs) = match build_files_of(&settings).and_then(|files| needs_of(&settings).map(|needs| (files, needs))) {
        Ok(both) => both,
        Err(why) => {
            let _ = writeln!(err, "wb accept: {}. Nothing written.", from_root(why));
            return 1;
        }
    };
    if let Some(build_files) = &build_files {
        let hits = build_hits(&read.changes, build_files);
        if !hits.is_empty() {
            let _ = writeln!(
                err,
                "wb accept: `changes` of {} holds a file of the build, and a change never changes the build. Nothing written. Take each out of `changes`:",
                shown
            );
            for (path, build) in hits {
                let _ = writeln!(err, "  - {} (under `build_files` of watchbill.yml: {})", self::shown(path), self::shown(build));
            }
            return 1;
        }
    }
    let accepted = accepted_of(base, issue);
    let shown_accepted = format!("{}/{}.accepted", FOLDER, issue);
    if fs::symlink_metadata(&accepted).is_ok() {
        let _ = writeln!(err, "wb accept: {} is there already, and is never written over. To accept the file again, remove it first. Nothing written.", shown_accepted);
        return 1;
    }
    let identity = git_value(root, "user.name").and_then(|name| git_value(root, "user.email").map(|email| (name, email)));
    let (name, email) = match identity {
        Ok(identity) => identity,
        Err(why) => {
            let _ = writeln!(err, "wb accept: {} Nothing written.", why);
            return 1;
        }
    };
    let seconds = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(since) => since.as_secs(),
        Err(_) => {
            let _ = writeln!(err, "wb accept: the clock of this machine reads before 1970 or cannot be read. Nothing written.");
            return 1;
        }
    };
    let body = format!("sha256: {}\nby: {} <{}>\ntime: {}\n", hash, name, email, crate::prompt::utc(seconds));

    // What each file needs to build is in the view of each role that sees it (#125), by the project's own command.
    match &needs {
        None => {
            let _ = writeln!(out, "wb accept: watchbill.yml has no `needs`, so the lists were not checked for what each file needs to build.");
        }
        Some(pattern) => match check_needs(root, pattern, &read) {
            Err(why) => {
                let _ = writeln!(err, "wb accept: {}. Nothing written.", why);
                return 1;
            }
            Ok(Needs { run, missing }) if missing.is_empty() => {
                let files = if run == 1 { "1 file".to_string() } else { format!("{} files", run) };
                let _ = writeln!(
                    out,
                    "wb accept: the lists were checked by `needs` of watchbill.yml, for {}: each file the tester and the build see has what it needs to build.",
                    files
                );
            }
            Ok(Needs { missing, .. }) => {
                let _ = writeln!(
                    err,
                    "wb accept: the lists of {} leave out files that are needed to build, by `needs` of watchbill.yml. Nothing written. Add each to `changes` or to the role's `reads`:",
                    shown
                );
                for (role, file, path) in missing {
                    let _ = writeln!(err, "  - {}: {} needs {}", role, self::shown(&file), self::shown(&path));
                }
                return 1;
            }
        },
    }
    if build_files.is_some() {
        let _ = writeln!(out, "wb accept: no path of `changes` is under `build_files` of watchbill.yml.");
    }

    // The file is the one that was read and held to its form.
    if fs::read(&file).map(|again| crate::manifest::hash_bytes(&again)).ok().as_deref() != Some(hash.as_str()) {
        let _ = writeln!(err, "wb accept: {} changed while it was read. Nothing written.", shown);
        return 1;
    }
    let _ = writeln!(out, "wb accept: will write {}:\n{}", shown_accepted, body);
    // Whole or not at all: the acceptance is written to a file of its own in the same folder, then
    // linked to its name, which fails when the name is there. A file left by a crash is another name.
    let issues = base.join(FOLDER);
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|since| since.subsec_nanos()).unwrap_or(0);
    let mut written = None;
    for attempt in 0..100u32 {
        let part = issues.join(format!(".{}.accepted.{}-{}-{}", issue, std::process::id(), nanos, attempt));
        match fs::OpenOptions::new().write(true).create_new(true).open(&part) {
            Ok(opened) => {
                written = Some((part, opened));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                let _ = writeln!(err, "wb accept: {} cannot be written ({}). Nothing written.", shown_accepted, error);
                return 1;
            }
        }
    }
    let Some((part, mut opened)) = written else {
        let _ = writeln!(err, "wb accept: {} cannot be written. Nothing written.", shown_accepted);
        return 1;
    };
    let result = opened.write_all(body.as_bytes()).and_then(|_| opened.sync_all());
    drop(opened);
    let result = result.and_then(|_| fs::hard_link(&part, &accepted));
    let _ = fs::remove_file(&part);
    if let Err(error) = result {
        let _ = writeln!(err, "wb accept: {} could not be written ({}). Nothing written.", shown_accepted, error);
        return 1;
    }
    let _ = writeln!(out, "wb accept: {} is accepted.", shown);
    let _ = writeln!(out, "wb accept: commit the acceptance alone, so the history holds it before any work it allows:\n  {}", commit_command(issue));
    0
}

/// Refuses the key `breakdown` of `watchbill.yml` by name (#116). Until revision 0.0.8, `breakdown: none`
/// let a project hand out roles with no issue's file (#97). Since then a role is handed out without one,
/// and nothing reads the key. The line is named, with the words to remove it. `wb check` and `wb prompt`
/// both judge `watchbill.yml` with this and no other.
pub fn no_breakdown(document: &Document) -> Result<(), String> {
    let place = [Part::from("breakdown")];
    if !document.has(&place) {
        return Ok(());
    }
    // The line as it is written, where it is one plain text, so that the person finds it.
    let line = match document.text(&place) {
        Ok(value) if !value.is_empty() && !value.chars().any(|c| c.is_control() || c == '`') => format!(" `breakdown: {}`", value),
        _ => String::new(),
    };
    let why = format!(
        "the key `breakdown` is no longer read: since revision 0.0.8 a role is handed out with no issue's file, and a file that is there must be accepted and committed. Remove the line{}.",
        line
    );
    Err(document.fail(&place, &why, true).to_string())
}

/// What the commit command for the acceptance of an issue is, as `wb accept` says it and as a refusal says it.
pub fn commit_command(issue: &str) -> String {
    let path = format!("{}/{}.accepted", FOLDER, issue);
    format!("git add {} && git commit -m \"Accept the issue's file ({})\" -- {}", path, issue, path)
}

/// Whether the issue's file and its acceptance are committed in `HEAD` and are the same in the working
/// tree (#97). The first that is not is named, with what to do.
pub fn committed(root: &Path, issue: &str) -> Result<(), String> {
    for (path, what) in [(format!("{}/{}.yml", FOLDER, issue), "the issue's file"), (format!("{}/{}.accepted", FOLDER, issue), "its acceptance")] {
        let root_text = root.display().to_string();
        let todo = if path.ends_with(".yml") {
            format!("Commit it alone on the branch of the issue: git add {} && git commit -m \"The issue's file ({})\" -- {}", path, issue, path)
        } else {
            format!("Commit it alone: {}", commit_command(issue))
        };
        // Git itself says what is committed and what is changed, with its own line ends and filters.
        let ask = |args: &[&str]| -> Result<bool, String> {
            let mut all = vec!["-C", root_text.as_str()];
            all.extend(args);
            crate::detect::git_output(&all, None)
                .map(|run| run.status.success())
                .ok_or_else(|| "git could not be run. No prompt is handed out.".to_string())
        };
        let spec = format!("HEAD:{}", path);
        if !ask(&["cat-file", "-e", &spec])? {
            return Err(format!("{} {} is not committed in HEAD. {}. No prompt is handed out.", what, path, todo));
        }
        if !ask(&["diff", "--quiet", "HEAD", "--", &path])? {
            return Err(format!("{} {} differs from HEAD. {}. No prompt is handed out.", what, path, todo));
        }
    }
    Ok(())
}
