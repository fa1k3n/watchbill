//! The issue's file and its acceptance (`docs/revision.md`, point 13): `.watchbill/issues/<issue>.yml`
//! read through the one loader and held to its form, and `wb accept <issue>`.

use crate::yamlfile::{load, Document, Part, Value, YamlError};
use crate::bounded::read_plain;
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

/// Text of the file as `wb` shows what it did not make: a control character, U+2028, U+2029 and the bidi
/// controls (U+202A to U+202E, U+2066 to U+2069) are written escaped, and so is a backslash, so that one line
/// stays one line, text keeps its order, and an escape cannot be forged by writing it out.
pub fn shown(text: &str) -> String {
    let odd = |c: char| c.is_control() || c == '\\' || matches!(c, '\u{2028}' | '\u{2029}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}');
    text.chars().map(|c| if odd(c) { c.escape_default().to_string() } else { c.to_string() }).collect()
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

/// As `read`, for the bytes of the file that were read already, with where each path of its lists stands.
fn read_bytes(path: &Path, bytes: Vec<u8>) -> Result<(Issue, Vec<Listed>), String> {
    let document = crate::yamlfile::load_bytes(path, bytes).map_err(fault)?;
    let issue = issue_of(&document)?;
    let mut listed = Vec::new();
    let lists = std::iter::once(("changes".to_string(), vec![Part::from("changes")], &issue.changes))
        .chain(issue.reads.iter().map(|(role, list)| (format!("reads.{}", role), vec![Part::from("reads"), Part::from(role.as_str())], list)));
    for (list, place, paths) in lists {
        for (index, path) in paths.iter().enumerate() {
            let mut at = place.clone();
            at.push(Part::Index(index));
            listed.push(Listed { list: list.clone(), path: path.clone(), line: document.line(&at).unwrap_or(0) });
        }
    }
    Ok((issue, listed))
}

/// Where a path of a list stands in the issue's file (#173): the list, `changes` or `reads.<role>`, the path as
/// the file writes it, and the line it is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listed {
    pub list: String,
    pub path: String,
    pub line: usize,
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
pub(crate) fn way_fault(root: &Path) -> Option<String> {
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
    let (Ok(bytes), Ok(text)) = (read_plain(&file, MAX_BYTES), crate::bounded::read_text(&accepted, MAX_BYTES)) else {
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
    /// Each path of `changes` and of `reads`, with its list and its line, in the order of the file.
    pub listed: Vec<Listed>,
    /// The `time:` of its acceptance, which a planner round after it voids (#290).
    pub accepted_time: String,
}

/// The most an issue's file may be: it is read whole, and a file larger than an issue needs is refused.
pub const MAX_BYTES: u64 = 1024 * 1024;

/// A character that a value in a prompt never holds: a control character (a line break and a tab
/// are let through where `block` is true, as in the interface), a line or paragraph separator, or a
/// bidirectional control (U+202A to U+202E, U+2066 to U+2069, among them the right-to-left override).
pub(crate) fn is_unsafe(c: char, block: bool) -> bool {
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
    let bytes = match read_plain(&file, MAX_BYTES) {
        Ok(bytes) => bytes,
        Err(crate::bounded::Fault::TooBig(_)) => return Err(too_big()),
        Err(error) => return Err(format!("{} cannot be read ({}). No prompt is handed out.", shown, error)),
    };
    let hash = crate::manifest::hash_bytes(&bytes);
    let accepted = accepted_of(root, issue);
    let body = match fs::symlink_metadata(&accepted) {
        Ok(found) if found.is_file() => crate::bounded::read_text(&accepted, MAX_BYTES).ok(),
        _ => None,
    };
    // The command that accepts it: `wb plan accept` where a plan of the name is there (#290).
    let root_text = root.display().to_string();
    let accepting = if crate::plan::is_plan(&root_text, issue) { "wb plan accept" } else { "wb accept" };
    let Some(body) = body else {
        return Err(format!(
            "the issue's file {} is not accepted. To accept it, run `{} {}`. No prompt is handed out.",
            shown, accepting, issue
        ));
    };
    if body.lines().find_map(|line| line.strip_prefix("sha256: ")) != Some(hash.as_str()) {
        return Err(format!(
            "the issue's file {} is not accepted: it changed since its acceptance. Remove {}/{}.accepted and run `{} {}` again. No prompt is handed out.",
            shown, FOLDER, issue, accepting, issue
        ));
    }
    // A source added to the plan after the acceptance voids it, as an edit does (#290).
    if crate::plan::sources_after_acceptance(&root_text, issue, &body) {
        return Err(format!(
            "the issue's file {} is not accepted: the plan {} has a source added after the acceptance. Remove {}/{}.accepted and run `wb plan accept {}` again. No prompt is handed out.",
            shown, issue, FOLDER, issue, issue
        ));
    }
    let prefix = format!("{}/", root.display().to_string().trim_end_matches('/'));
    let (read, listed) = read_bytes(&file, bytes).map_err(|why| format!("{}. No prompt is handed out.", why.replace(&prefix, "")))?;
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
    let accepted_time = crate::plan::accepted_time(&body);
    Ok(Some(Used { issue: read, path: shown, hash, listed, accepted_time }))
}

/// The gate of a planner round after the acceptance (#290), for a command that hands out a prompt of the issue or
/// starts its session, after `for_prompt` found `used` accepted: a planner round started at or after the acceptance
/// voids it, as `wb status` reads it. `planner` is whether the first lines of the comments, where the command read them
/// already, hold a `[stamp] planner` that counts. With `None`, for a plan of `wb plan add` only, they are read by
/// `issue_comments`, as `wb status` reads them; with no `issue_comments` the whole comments are read at once, and with
/// neither no stamp is read. The stamps' times are read whole, by
/// `issue_comment_bodies`, only where such a stamp is there. `Err` with why where it is void or a pattern failed;
/// `Ok(Some(said))` where the times could not be read, as `watchbill.yml` has no `issue_comment_bodies`; else `Ok(None)`.
pub fn planner_gate(root: &Path, issue: &str, used: &Used, planner: Option<bool>) -> Result<Option<String>, String> {
    let root_text = root.display().to_string();
    let planner = match planner {
        Some(planner) => planner,
        // A command that read no comments asks the tracker only for a plan of `wb plan add`: an issue with no plan keeps
        // the gate of `wb accept` as it was, until #282, and `wb prompt` given every value runs no pattern for it.
        None if !crate::plan::is_plan(&root_text, issue) => return Ok(None),
        None => match crate::prompt::pattern_of(&root_text, "issue_comments").map_err(|why| format!("{}. No prompt is handed out.", why))? {
            // With neither pattern no stamp can be read, as `wb status` reads none: nothing more to say.
            None if crate::prompt::pattern_of(&root_text, "issue_comment_bodies").map_err(|why| format!("{}. No prompt is handed out.", why))?.is_none() => {
                return Ok(None)
            }
            None => true,
            Some(line) => {
                if !crate::prompt::is_plain_name(issue) {
                    return Err(format!("{} is not a plain name that fills `issue_comments`. No prompt is handed out.", self::shown(issue)));
                }
                let printed = crate::starter::run_capped(&root_text, "issue_comments", &line.replace("{issue}", issue), crate::bounded::TEXT_CAP)
                    .map_err(|why| format!("{}. No prompt is handed out.", why))?;
                if printed.len() as u64 > crate::bounded::TEXT_CAP {
                    return Err("`issue_comments` printed more than 1 MiB. No prompt is handed out.".to_string());
                }
                crate::starter::stamps_in(&String::from_utf8_lossy(&printed)).iter().any(|(role, _)| role == "planner")
            }
        },
    };
    if !planner {
        return Ok(None);
    }
    match crate::plan::planned_again(&root_text, issue, &used.accepted_time).map_err(|why| format!("{}. No prompt is handed out.", why))? {
        None => Ok(Some("watchbill.yml has no `issue_comment_bodies`, so the times of the planner's stamps were not read, and the acceptance stands".to_string())),
        Some(false) => Ok(None),
        Some(true) => {
            let accepting = if crate::plan::is_plan(&root_text, issue) { "wb plan accept" } else { "wb accept" };
            Err(format!(
                "the issue's file {} is not accepted: a planner round started at or after its acceptance, so it was planned again after it. Remove {}/{}.accepted and run `{} {}` again. No prompt is handed out.",
                used.path, FOLDER, issue, accepting, issue
            ))
        }
    }
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
pub(crate) fn git_value(root: &str, key: &str) -> Result<String, String> {
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

/// `wb plan accept <issue>` in the repository at `root`, with every line said under the command `name`: 0 it is accepted,
/// 1 it is refused, 2 Watchbill is not set up here. It says what it will write before it writes, and never writes over
/// an acceptance. With `commit` it also commits the acceptance alone, on the issue's branch (#157), and refuses before
/// it writes when the checkout is not on that branch or the issue's file is not as `HEAD` has it. Without `commit`
/// (`--no-commit`) it writes the acceptance and says the commit.
pub(crate) fn accept_as(name: &str, root: &str, issue: &str, commit: bool, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let base = Path::new(root);
    if let Some(why) = way_fault(base) {
        return crate::refusal::limit(err, name, format_args!("{}. Nothing read, nothing written.", why));
    }
    if crate::bounded::is_absent(&base.join(".watchbill").join("watchbill.yml")) {
        let _ = writeln!(err, "{}: .watchbill/watchbill.yml is not there. Watchbill is not set up in this repository.", name);
        return 2;
    }
    if let Err(why) = crate::starter::held(root) {
        return crate::refusal::limit(err, name, format_args!("{}. Nothing read, nothing written.", why));
    }
    let file = file_of(base, issue);
    let shown = format!("{}/{}.yml", FOLDER, issue);
    match fs::symlink_metadata(&file) {
        Ok(found) if found.is_file() => {}
        Ok(_) => {
            return crate::refusal::limit(err, name, format_args!("{} is not a plain file. Nothing written.", shown));
        }
        Err(_) => {
            return crate::refusal::limit(err, name, format_args!("{} is not there, or cannot be read. Nothing written.", shown));
        }
    }
    let bytes = match read_plain(&file, MAX_BYTES) {
        Ok(bytes) => bytes,
        Err(error) => {
            return crate::refusal::limit(err, name, format_args!("{} cannot be read ({}). Nothing written.", shown, error));
        }
    };
    let hash = crate::manifest::hash_bytes(&bytes);
    let read = match read(&file) {
        Ok(read) => read,
        Err(why) => {
            // The file is named from the root of the repository, as every other message is.
            let why = why.replace(&format!("{}/", root.trim_end_matches('/')), "");
            return crate::refusal::limit(err, name, format_args!("{}. Nothing written.", why));
        }
    };
    if read.issue != issue {
        return crate::refusal::limit(err, name, format_args!("{} says the issue is {}, not {}. Nothing written.", shown, self::shown(&read.issue), issue));
    }
    if !read.questions.is_empty() {
        let why = format!("{} holds {} question{}. Nothing written. Answer them in the file first:", shown, read.questions.len(), if read.questions.len() == 1 { "" } else { "s" });
        let said: String = read.questions.iter().map(|question| format!("\n  - {}", self::shown(question))).collect();
        return crate::refusal::limit(err, name, format_args!("{}{}", why, said));
    }
    // `watchbill.yml`, through the one loader, for the files of the build and the pattern `needs` (#125).
    let from_root = |why: String| why.replace(&format!("{}/", root.trim_end_matches('/')), "").trim_end_matches('.').to_string();
    let settings = match crate::starter::settings(root) {
        Ok(document) => document,
        Err(why) => {
            return crate::refusal::limit(err, name, format_args!("{}. Nothing written.", from_root(why)));
        }
    };
    let (build_files, needs) = match build_files_of(&settings).and_then(|files| needs_of(&settings).map(|needs| (files, needs))) {
        Ok(both) => both,
        Err(why) => {
            return crate::refusal::limit(err, name, format_args!("{}. Nothing written.", from_root(why)));
        }
    };
    if let Some(build_files) = &build_files {
        let hits = build_hits(&read.changes, build_files);
        if !hits.is_empty() {
            let said: String = hits.iter().map(|(path, build)| format!("\n  - {} (under `build_files` of watchbill.yml: {})", self::shown(path), self::shown(build))).collect();
            return crate::refusal::limit(
                err,
                name,
                format_args!("`changes` of {} holds a file of the build, and a change never changes the build. Nothing written. Take each out of `changes`:{}", shown, said),
            );
        }
    }
    let accepted = accepted_of(base, issue);
    let shown_accepted = format!("{}/{}.accepted", FOLDER, issue);
    if fs::symlink_metadata(&accepted).is_ok() {
        return crate::refusal::limit(err, name, format_args!("{} is there already, and is never written over. To accept the file again, remove it first. Nothing written.", shown_accepted));
    }
    if commit {
        match ready_to_commit(root, issue) {
            Ok(()) => {}
            Err(NotReady::Fault(why)) => {
                return crate::refusal::limit(err, name, format_args!("{} Nothing written. --no-commit skips the commit, and this with it: it writes the acceptance and commits nothing.", why));
            }
            Err(NotReady::Guard(why)) => {
                return crate::refusal::guard(err, name, "--no-commit", format_args!("{} Nothing written. Or give --no-commit to write the acceptance and commit nothing.", why));
            }
        }
    }
    let identity = git_value(root, "user.name").and_then(|name| git_value(root, "user.email").map(|email| (name, email)));
    let (user, email) = match identity {
        Ok(identity) => identity,
        Err(why) => {
            return crate::refusal::limit(err, name, format_args!("{} Nothing written.", why));
        }
    };
    let seconds = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(since) => since.as_secs(),
        Err(_) => {
            return crate::refusal::limit(err, name, format_args!("the clock of this machine reads before 1970 or cannot be read. Nothing written."));
        }
    };
    let body = format!("sha256: {}\nby: {} <{}>\ntime: {}\n", hash, user, email, crate::prompt::utc(seconds));

    // What each file needs to build is in the view of each role that sees it (#125), by the project's own command.
    match &needs {
        None => {
            let _ = writeln!(out, "{}: watchbill.yml has no `needs`, so the lists were not checked for what each file needs to build.", name);
        }
        Some(pattern) => match check_needs(root, pattern, &read) {
            Err(why) => {
                return crate::refusal::limit(err, name, format_args!("{}. Nothing written.", why));
            }
            Ok(Needs { run, missing }) if missing.is_empty() => {
                let files = if run == 1 { "1 file".to_string() } else { format!("{} files", run) };
                let _ = writeln!(
                    out,
                    "{}: the lists were checked by `needs` of watchbill.yml, for {}: each file the tester and the build see has what it needs to build.",
                    name,
                    files
                );
            }
            Ok(Needs { missing, .. }) => {
                let said: String = missing.iter().map(|(role, file, path)| format!("\n  - {}: {} needs {}", role, self::shown(file), self::shown(path))).collect();
                return crate::refusal::limit(
                    err,
                    name,
                    format_args!("the lists of {} leave out files that are needed to build, by `needs` of watchbill.yml. Nothing written. Add each to `changes` or to the role's `reads`:{}", shown, said),
                );
            }
        },
    }
    if build_files.is_some() {
        let _ = writeln!(out, "{}: no path of `changes` is under `build_files` of watchbill.yml.", name);
    }

    // The file is the one that was read and held to its form.
    if read_plain(&file, MAX_BYTES).map(|again| crate::manifest::hash_bytes(&again)).ok().as_deref() != Some(hash.as_str()) {
        return crate::refusal::limit(err, name, format_args!("{} changed while it was read. Nothing written.", shown));
    }
    let _ = writeln!(out, "{}: will write {}:\n{}", name, shown_accepted, body);
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
                return crate::refusal::limit(err, name, format_args!("{} cannot be written ({}). Nothing written.", shown_accepted, error));
            }
        }
    }
    let Some((part, mut opened)) = written else {
        return crate::refusal::limit(err, name, format_args!("{} cannot be written. Nothing written.", shown_accepted));
    };
    let result = opened.write_all(body.as_bytes()).and_then(|_| opened.sync_all());
    drop(opened);
    let result = result.and_then(|_| fs::hard_link(&part, &accepted));
    let _ = fs::remove_file(&part);
    if let Err(error) = result {
        return crate::refusal::limit(err, name, format_args!("{} could not be written ({}). Nothing written.", shown_accepted, error));
    }
    let _ = writeln!(out, "{}: {} is accepted.", name, shown);
    if !commit {
        let _ = writeln!(out, "{}: commit the acceptance alone, so the history holds it before any work it allows:\n  {}", name, commit_command(issue));
        return 0;
    }
    let _ = writeln!(out, "{}: will commit {} alone, with the message \"{}\", as {} <{}>.", name, shown_accepted, commit_message(issue), user, email);
    let _ = out.flush();
    match commit_alone(root, issue, &user, &email) {
        Ok(held) => {
            let short = crate::detect::git_output(&["-C", root, "rev-parse", "--short", "HEAD"], None)
                .filter(|run| run.status.success())
                .map(|run| String::from_utf8_lossy(&run.stdout).trim().to_string())
                .unwrap_or_default();
            let branch = current_branch(root).unwrap_or_default();
            if held.iter().any(|path| path != &shown_accepted) {
                return crate::refusal::limit(
                    err,
                    name,
                    format_args!(
                        "the commit {} is not alone: it holds {}. A hook of git added files to it. Nothing more is done: undo or amend it as you decide.",
                        short,
                        held.iter().map(|path| self::shown(path)).collect::<Vec<_>>().join(", ")
                    ),
                );
            }
            let _ = writeln!(out, "{}: committed {} alone ({}). What was staged before stays staged.", name, shown_accepted, short);
            let _ = writeln!(out, "{}: wb pushes nothing. Push the branch {} so that the history elsewhere holds the acceptance.", name, branch);
            0
        }
        Err(why) => {
            let code = crate::refusal::limit(err, name, format_args!("the commit failed: {}", why));
            let _ = writeln!(
                err,
                "{}: {} is written and stays, and is unstaged again. Commit it alone, so the history holds it before any work it allows:\n  {}",
                name,
                shown_accepted,
                commit_command(issue)
            );
            code
        }
    }
}

/// The message of the commit of an acceptance.
fn commit_message(issue: &str) -> String {
    format!("Accept the issue's file ({})", issue)
}

/// The branch the checkout is on, or none when it is detached or git could not be run.
pub(crate) fn current_branch(root: &str) -> Option<String> {
    let run = crate::detect::git_output(&["-C", root, "symbolic-ref", "--short", "-q", "HEAD"], None)?;
    if !run.status.success() {
        return None;
    }
    let name = String::from_utf8(run.stdout).ok()?;
    let name = name.trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// What must hold before the acceptance is written, when it is to be committed (#157): the checkout is on
/// the branch of the issue, by the pattern `branch`, and the issue's file is committed in `HEAD` and the
/// same in the working tree.
pub(crate) fn ready_to_commit(root: &str, issue: &str) -> Result<(), NotReady> {
    let pattern = crate::prompt::pattern_of(root, "branch")
        .map_err(|why| NotReady::Fault(why.to_string().replace(&format!("{}/", root.trim_end_matches('/')), "")))?
        .filter(|pattern| pattern.contains("{issue}"))
        .ok_or_else(|| NotReady::Fault("watchbill.yml has no pattern `branch` with {issue}, so the branch of the issue is not known.".to_string()))?;
    let expected = pattern.replace("{issue}", issue);
    match current_branch(root) {
        Some(branch) if crate::status::is_of(&pattern, issue, &branch) => {}
        Some(branch) => {
            return Err(NotReady::Guard(format!(
                "the checkout is on the branch {}, not on the branch of the issue, {} (the pattern `branch`: {}). Switch to it first.",
                shown(&branch),
                expected,
                pattern
            )))
        }
        None => {
            return Err(NotReady::Guard(format!(
                "the checkout is on no branch, not on the branch of the issue, {} (the pattern `branch`: {}). Switch to it first.",
                expected, pattern
            )))
        }
    }
    let path = format!("{}/{}.yml", FOLDER, issue);
    let ask = |args: &[&str]| -> Result<bool, NotReady> {
        let mut all = vec!["-C", root];
        all.extend(args);
        crate::detect::git_output(&all, None).map(|run| run.status.success()).ok_or_else(|| NotReady::Fault("git could not be run.".to_string()))
    };
    let todo = format!("Commit it alone on the branch of the issue first: git add {} && git commit -m \"The issue's file ({})\" -- {}.", path, issue, path);
    if !ask(&["cat-file", "-e", &format!("HEAD:{}", path)])? {
        return Err(NotReady::Guard(format!("the issue's file {} is not committed in HEAD. {} The acceptance holds for the file as the history has it.", path, todo)));
    }
    if !ask(&["diff", "--quiet", "HEAD", "--", &path])? {
        return Err(NotReady::Guard(format!("the issue's file {} differs from HEAD. {} The acceptance holds for the file as the history has it.", path, todo)));
    }
    Ok(())
}

/// Why the acceptance cannot be committed by `wb accept`: a fault, where the commit cannot be made here at all, or a
/// guard, where the checkout is not as the commit needs it. `--no-commit` skips the commit, and both with it (#180).
pub(crate) enum NotReady {
    Fault(String),
    Guard(String),
}

/// Commits the acceptance of the issue alone: `git add` of the one file, then `git commit --only` of it,
/// so what else is staged stays staged and out of the commit. The author is the name and email of the line
/// `by:`, whatever the environment says. Ok holds the paths the commit holds. When the commit fails, the
/// acceptance is unstaged again, so the index is as it was; Err holds what git said.
fn commit_alone(root: &str, issue: &str, name: &str, email: &str) -> Result<Vec<String>, String> {
    commit_path(root, &format!("{}/{}.accepted", FOLDER, issue), &commit_message(issue), name, email)
}

/// Commits the one new file `path` alone with `message`, as `commit_alone` does for an acceptance (#282 uses it for
/// the record of the final acceptance).
pub(crate) fn commit_path(root: &str, path: &str, message: &str, name: &str, email: &str) -> Result<Vec<String>, String> {
    let path = path.to_string();
    let run = |args: &[&str]| -> Result<String, String> {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .stdin(std::process::Stdio::null())
            .output()
            .map_err(|error| format!("git could not be run ({})", error))?;
        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).to_string());
        }
        let said = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        let said: String = said.trim().chars().filter(|c| !c.is_control() || *c == '\n').collect();
        Err(if said.is_empty() { format!("git ended with {}", output.status) } else { said })
    };
    let unstage = || {
        // The acceptance is a new file: with a HEAD the index is reset to it, with none the entry is removed.
        let has_head = run(&["rev-parse", "--verify", "-q", "HEAD"]).is_ok();
        let _ = if has_head { run(&["reset", "-q", "--", &path]) } else { run(&["rm", "--cached", "--quiet", "--", &path]) };
    };
    if let Err(why) = run(&["add", "--", &path]) {
        unstage();
        return Err(why);
    }
    let author = format!("{} <{}>", name, email);
    if let Err(why) = run(&["commit", "-q", "-m", message, "--author", &author, "--only", "--", &path]) {
        unstage();
        return Err(why);
    }
    let held = run(&["diff-tree", "--no-commit-id", "--name-only", "-r", "--root", "HEAD"])?;
    Ok(held.lines().map(|line| line.to_string()).collect())
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
