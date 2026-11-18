//! The task's file and its acceptance (`docs/revision.md`, point 13): `.watchbill/tasks/<task>.yml`
//! read through the one loader and held to its form, and `wb accept <task>`.

use crate::yamlfile::{load, Document, Part, Value, YamlError};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Where the task files are, below the root of the repository.
pub const FOLDER: &str = ".watchbill/tasks";

/// The keys of the file, in the order they stand.
const KEYS: [&str; 8] = [
    "task",
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

/// One requirement of a task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Requirement {
    pub id: String,
    pub text: String,
    pub source: String,
    pub guard: Option<String>,
}

/// A task's file, held to its form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub task: String,
    pub from: String,
    pub requirements: Vec<Requirement>,
    pub interface: String,
    pub changes: Vec<String>,
    /// The lists of `reads`, by the name of the role, in the order of the file.
    pub reads: Vec<(String, Vec<String>)>,
    pub existing_tests_changed: Vec<String>,
    pub questions: Vec<String>,
}

/// A task is named by letters, digits, `.`, `_` and `-`, at most 64 of them, and not by a name that starts with `.`.
pub fn is_name(task: &str) -> bool {
    !task.is_empty()
        && task.len() <= 64
        && !task.starts_with('.')
        && task.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

/// Text of the file as `wb` shows what it did not make: a control character is written escaped, so
/// that one line stays one line.
fn shown(text: &str) -> String {
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
        Some(".watchbill") => Some("lies under .watchbill/, which a task does not name"),
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

/// Reads a task's file through the one loader and holds it to its form. The error names the first
/// fault, with the file and the line.
pub fn read(path: impl AsRef<Path>) -> Result<Task, String> {
    let document = load(path).map_err(fault)?;
    keys_of(&document, &[], &KEYS, KEYS.len())?;

    let task = text(&document, &["task"])?;
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
    Ok(Task { task, from, requirements, interface, changes, reads, existing_tests_changed, questions })
}

fn file_of(root: &Path, task: &str) -> std::path::PathBuf {
    root.join(FOLDER).join(format!("{}.yml", task))
}

fn accepted_of(root: &Path, task: &str) -> std::path::PathBuf {
    root.join(FOLDER).join(format!("{}.accepted", task))
}

/// A folder on the way to the task files that is a link, or is there and is not a folder: it is named,
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

/// Whether a task's file is accepted: its `.accepted` is there, and the SHA-256 in it is that of
/// the file as it is now. Nothing else is looked at.
pub fn is_accepted(root: &Path, task: &str) -> bool {
    if !is_name(task) || way_fault(root).is_some() {
        return false;
    }
    let plain = |path: &Path| fs::symlink_metadata(path).map(|found| found.is_file()).unwrap_or(false);
    let (file, accepted) = (file_of(root, task), accepted_of(root, task));
    if !plain(&file) || !plain(&accepted) {
        return false;
    }
    let (Ok(bytes), Ok(text)) = (fs::read(&file), fs::read_to_string(&accepted)) else {
        return false;
    };
    let hash = crate::manifest::hash_bytes(&bytes);
    text.lines().find_map(|line| line.strip_prefix("sha256: ")) == Some(hash.as_str())
}

/// The roles that are given the task's file in their prompts.
pub const ROLES: [&str; 4] = ["tester", "build", "review", "fix"];

/// A task's file that a prompt is made from: read, held to its form, and accepted as it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Used {
    pub task: Task,
    /// The path from the root of the repository.
    pub path: String,
    /// The SHA-256 of the file as it was read.
    pub hash: String,
}

/// A character that a path in a prompt never holds: a control character, a line or paragraph
/// separator, or the mark that turns the direction of the text.
fn is_unsafe(c: char) -> bool {
    c.is_control() || matches!(c, '\u{2028}' | '\u{2029}' | '\u{202E}')
}

/// The task's file for a prompt of a role (`wb prompt`, #76). `Ok(None)` when the task has no file, so
/// a project goes on without one. When there is one it is used only if it is accepted as it is and
/// holds to its form; else the reason is the error, and no prompt is handed out.
pub fn for_prompt(root: &Path, task: &str) -> Result<Option<Used>, String> {
    if !is_name(task) {
        return Ok(None);
    }
    if let Some(why) = way_fault(root) {
        return Err(format!("{}. No prompt is handed out.", why));
    }
    let shown = format!("{}/{}.yml", FOLDER, task);
    let file = file_of(root, task);
    match fs::symlink_metadata(&file) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{} cannot be read ({}). No prompt is handed out.", shown, error)),
        Ok(found) if !found.is_file() => return Err(format!("{} is not a plain file. No prompt is handed out.", shown)),
        Ok(_) => {}
    }
    let bytes = fs::read(&file).map_err(|error| format!("{} cannot be read ({}). No prompt is handed out.", shown, error))?;
    let hash = crate::manifest::hash_bytes(&bytes);
    let accepted = accepted_of(root, task);
    let body = match fs::symlink_metadata(&accepted) {
        Ok(found) if found.is_file() => fs::read_to_string(&accepted).ok(),
        _ => None,
    };
    let Some(body) = body else {
        return Err(format!(
            "the task's file {} is not accepted. To accept it, run `wb accept {}`. No prompt is handed out.",
            shown, task
        ));
    };
    if body.lines().find_map(|line| line.strip_prefix("sha256: ")) != Some(hash.as_str()) {
        return Err(format!(
            "the task's file {} is not accepted: it changed since its acceptance. Remove {}/{}.accepted and run `wb accept {}` again. No prompt is handed out.",
            shown, FOLDER, task, task
        ));
    }
    let prefix = format!("{}/", root.display().to_string().trim_end_matches('/'));
    let read = read(&file).map_err(|why| format!("{}. No prompt is handed out.", why.replace(&prefix, "")))?;
    if fs::read(&file).map(|now| crate::manifest::hash_bytes(&now)).ok().as_deref() != Some(hash.as_str()) {
        return Err(format!("{} changed while it was read. No prompt is handed out.", shown));
    }
    if read.task != task {
        return Err(format!("{} says the task is {}, not {}. No prompt is handed out.", shown, self::shown(&read.task), task));
    }
    if !read.questions.is_empty() {
        return Err(format!("{} holds a question, and a file with one is not used. No prompt is handed out.", shown));
    }
    let lists = std::iter::once(("changes".to_string(), &read.changes))
        .chain(read.reads.iter().map(|(role, list)| (format!("reads.{}", role), list)));
    for (name, list) in lists {
        if list.iter().any(|path| path.chars().any(is_unsafe)) {
            return Err(format!(
                "{}: a path of `{}` holds a control character, a line or paragraph separator or a direction mark. No prompt is handed out.",
                shown, name
            ));
        }
    }
    Ok(Some(Used { task: read, path: shown, hash }))
}

impl Used {
    /// What `{requirements}` is filled with: each ID and its text, one to a line.
    pub fn requirements(&self) -> String {
        if self.task.requirements.is_empty() {
            return "none: the task's file holds no requirement".to_string();
        }
        let lines: Vec<String> = self.task.requirements.iter().map(|r| format!("- {}: {}", r.id, r.text)).collect();
        format!("\n{}", lines.join("\n"))
    }

    /// The two parts of a prompt the task's file adds for a role: the interface, and what the role may
    /// change and read. It is put in as it is, and is never read for placeholders.
    pub fn text(&self, role: &str) -> String {
        let list = |paths: &[String]| -> String {
            if paths.is_empty() {
                return "- none\n".to_string();
            }
            paths.iter().map(|path| format!("- {}\n", path)).collect()
        };
        let reads = self.task.reads.iter().find(|(name, _)| name == role).map(|(_, paths)| paths.as_slice()).unwrap_or(&[]);
        let interface = if self.task.interface.ends_with('\n') { self.task.interface.clone() } else { format!("{}\n", self.task.interface) };
        format!(
            "## The interface\n\n{}\n## What you may change and read\n\nYou may change:\n\n{}\nYou may read, besides those:\n\n{}",
            interface,
            list(&self.task.changes),
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

/// `wb accept <task>` in the repository at `root`: 0 it is accepted, 1 it is refused, 2 Watchbill is
/// not set up here. It says what it will write before it writes, and never writes over an acceptance.
pub fn accept(root: &str, task: &str, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let base = Path::new(root);
    if let Some(why) = way_fault(base) {
        let _ = writeln!(err, "wb accept: {}. Nothing read, nothing written.", why);
        return 1;
    }
    if !base.join(".watchbill").join("watchbill.yml").is_file() {
        let _ = writeln!(err, "wb accept: .watchbill/watchbill.yml is not there. Watchbill is not set up in this repository.");
        return 2;
    }
    let file = file_of(base, task);
    let shown = format!("{}/{}.yml", FOLDER, task);
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
    if read.task != task {
        let _ = writeln!(err, "wb accept: {} says the task is {}, not {}. Nothing written.", shown, self::shown(&read.task), task);
        return 1;
    }
    if !read.questions.is_empty() {
        let _ = writeln!(err, "wb accept: {} holds {} question{}. Nothing written. Answer them in the file first:", shown, read.questions.len(), if read.questions.len() == 1 { "" } else { "s" });
        for question in &read.questions {
            let _ = writeln!(err, "  - {}", self::shown(question));
        }
        return 1;
    }
    let accepted = accepted_of(base, task);
    let shown_accepted = format!("{}/{}.accepted", FOLDER, task);
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

    // The file is the one that was read and held to its form.
    if fs::read(&file).map(|again| crate::manifest::hash_bytes(&again)).ok().as_deref() != Some(hash.as_str()) {
        let _ = writeln!(err, "wb accept: {} changed while it was read. Nothing written.", shown);
        return 1;
    }
    let _ = writeln!(out, "wb accept: will write {}:\n{}", shown_accepted, body);
    // Whole or not at all: the acceptance is written to a file of its own in the same folder, then
    // linked to its name, which fails when the name is there. A file left by a crash is another name.
    let tasks = base.join(FOLDER);
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|since| since.subsec_nanos()).unwrap_or(0);
    let mut written = None;
    for attempt in 0..100u32 {
        let part = tasks.join(format!(".{}.accepted.{}-{}-{}", task, std::process::id(), nanos, attempt));
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
    0
}
