//! `wb init`: sets Watchbill up in a repository, the way `git init` starts one.
//!
//! It proposes what the repository says about itself, asks one line per answer,
//! shows the files and writes them: `.watchbill/watchbill.yml`, and the revision the package carries,
//! its files under `.watchbill/revision/` and its manifest as `.watchbill/revision.sha256`
//! (docs/revision.md, point 7). A file that is already there is never changed, and when one is
//! there nothing at all is written.

use std::fs;
use std::io::{self, BufRead, ErrorKind, Write};
use std::path::{Path, PathBuf};

use crate::detect::{detect, strip, universal_newlines, Setup, NONE};
use crate::interrupt;
use crate::manifest;

pub const CONFIG_FOLDER: &str = ".watchbill";
pub const CONFIG_FILE: &str = "watchbill.yml";
const REVISION_FOLDER: &str = "revision";

const QUESTIONS: [(&str, &str); 6] = [
    ("project", "project"),
    ("git_server", "git server (github, gitlab, gerrit, bitbucket or none)"),
    ("repository", "repository at the git server"),
    ("tracker", "tracker (github, gitlab, jira or none)"),
    ("requirements", "requirements (a path to files, the name of a tool, or none)"),
    ("ci", "ci driver (github-actions, gitlab-ci, jenkins or none)"),
];

/// A project name is lower-case letters, digits, `.`, `_` and `-`, and starts with a letter or a digit.
fn is_project_name(answer: &str) -> bool {
    let mut chars = answer.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "._-".contains(c))
}

/// Why an answer cannot be taken, or `None` when it can.
fn fault(key: &str, answer: &str) -> Option<&'static str> {
    if answer.chars().count() > 200 {
        return Some("that is over 200 characters");
    }
    // A control character would break the line of the file.
    if answer.chars().any(|c| c <= '\x1f' || c == '\x7f') {
        return Some("that holds a character that cannot be written");
    }
    if key == "project" && !is_project_name(answer) {
        return Some(
            "a project name is lower-case letters, digits, '.', '_' and '-', and starts with a letter or a digit",
        );
    }
    None
}

/// A word that starts with a letter, and is not one of the words YAML reads as a truth value or as
/// nothing: plain is sure to be read back as the same text.
fn is_plain_word(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || "_./*-".contains(c))
        && !["y", "n", "yes", "no", "true", "false", "on", "off", "null"].contains(&value.to_ascii_lowercase().as_str())
}

/// A version of three numbers or more, which no YAML number or date has the form of.
fn is_version(value: &str) -> bool {
    let digits = |text: &str| text.find(|c: char| !c.is_ascii_digit()).unwrap_or(text.len());
    let mut rest = value;
    for _ in 0..2 {
        let n = digits(rest);
        if n == 0 || !rest[n..].starts_with('.') {
            return false;
        }
        rest = &rest[n + 1..];
    }
    digits(rest) > 0 && rest.chars().all(|c| c.is_ascii_alphanumeric() || "._+-".contains(c))
}

/// A value as YAML reads it back unchanged: plain when that is certain, else in single quotes.
pub fn yaml_value(value: &str) -> String {
    if is_version(value) || is_plain_word(value) {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}

/// The file `wb init` writes.
pub fn render(revision: &str, hash: &str, setup: &Setup) -> String {
    let rows = [
        ("watchbill", revision),
        ("revision", hash),
        ("project", &setup.project),
        ("git_server", &setup.git_server),
        ("repository", &setup.repository),
        ("tracker", &setup.tracker),
        ("requirements", &setup.requirements),
        ("ci", &setup.ci),
    ];
    let mut text = String::from(
        "# How Watchbill is set up in this repository. Written by `wb init`.\n\
         # `watchbill` is the version of the package the repository is set up for.\n",
    );
    for (key, value) in rows {
        text.push_str(&format!("{}: {}\n", key, yaml_value(value)));
    }
    text
}

/// What is said, with 1, when reading the answers failed: an answer that is not UTF-8 text, or an
/// error of reading, which is told as it is.
fn read_failure(error: &io::Error) -> String {
    if error.kind() == ErrorKind::InvalidData {
        "wb init: what was typed is not UTF-8 text. Nothing written.\n".to_string()
    } else {
        format!("wb init: cannot read standard input: {}. Nothing written.\n", error)
    }
}

/// The revision of the package, as it is to be written: each file with its bytes, the manifest and
/// its hash. Paths are relative to `.watchbill/`.
struct Pin {
    files: Vec<(String, Vec<u8>)>,
    hash: String,
}

/// Reads the revision from the package. Each file is read once, and what was read must be what the
/// manifest says of it, so that what is written is what is hashed.
fn read_pin(package: &Path) -> Result<Pin, String> {
    let manifest = manifest::manifest(package).map_err(|why| why.to_string())?;
    let mut files = Vec::new();
    for line in manifest.lines() {
        let (hash, path) = line.split_once("  ").ok_or("the manifest is not in the form of sha256sum")?;
        let bytes = fs::read(package.join(path)).map_err(|error| format!("{}: cannot be read ({})", path, error))?;
        let same = std::str::from_utf8(&bytes).map(|text| manifest::hash_of(text) == hash).unwrap_or(false);
        if !same {
            return Err(format!("{}: changed while it was read", path));
        }
        files.push((path.to_string(), bytes));
    }
    let hash = manifest::hash_of(&manifest);
    files.push((manifest::MANIFEST_FILE.to_string(), manifest.into_bytes()));
    Ok(Pin { files, hash })
}

/// The first path that is in the way of the files, or `None`. It is a file that is there already; a
/// folder on the way to a file that is a link or not a folder (a link would take the files to where it
/// points); or `.watchbill/revision` of any kind, so that no other file stays beside the revision
/// (WB-PIN-4, 7 and 8).
fn in_the_way(folder: &str, targets: &[String]) -> Option<String> {
    if let Some(file) = targets.iter().find(|path| fs::symlink_metadata(path).is_ok()) {
        return Some(file.clone());
    }
    for target in targets {
        let mut path = PathBuf::from(folder);
        let below = Path::new(target).strip_prefix(folder).ok()?.parent()?;
        let mut names = vec![path.clone()];
        for part in below {
            path.push(part);
            names.push(path.clone());
        }
        for each in names {
            match fs::symlink_metadata(&each) {
                Ok(meta) if meta.is_dir() => {}
                Err(error) if error.kind() == ErrorKind::NotFound => break,
                _ => return Some(each.to_string_lossy().into_owned()),
            }
        }
    }
    let revision = format!("{}/{}", folder, REVISION_FOLDER);
    fs::symlink_metadata(&revision).ok().map(|_| revision)
}

/// The place of the answers: where the questions go and where the lines come from.
struct Talk<'a> {
    input: &'a mut dyn BufRead,
    out: &'a mut dyn Write,
}

impl Talk<'_> {
    fn say(&mut self, text: &str) {
        // A failed write, such as a closed pipe, does not change what the command ended with.
        let _ = writeln!(self.out, "{}", text);
    }

    /// A question and its answer, one line from standard input: `Ok(None)` when there are no more.
    /// A line that is not UTF-8 is an error of the kind `InvalidData`.
    fn ask(&mut self, prompt: &str) -> io::Result<Option<String>> {
        let _ = write!(self.out, "{}", prompt);
        let _ = self.out.flush();
        let mut line = Vec::new();
        self.input.read_until(b'\n', &mut line)?;
        if line.is_empty() {
            // No more input: end the line of the question, so what follows starts on its own.
            let _ = writeln!(self.out);
            return Ok(None);
        }
        if line.last() == Some(&b'\n') {
            line.pop();
        }
        match String::from_utf8(line) {
            Ok(text) => Ok(Some(text)),
            Err(_) => {
                let _ = writeln!(self.out);
                Err(io::Error::new(ErrorKind::InvalidData, "not UTF-8"))
            }
        }
    }

    fn answers(&mut self, found: &Setup) -> io::Result<Setup> {
        let mut setup = found.clone();
        for (key, text) in QUESTIONS {
            if key == "repository" && setup.git_server == NONE {
                setup.repository = NONE.to_string();
                continue;
            }
            loop {
                let current = match key {
                    "project" => &setup.project,
                    "git_server" => &setup.git_server,
                    "repository" => &setup.repository,
                    "tracker" => &setup.tracker,
                    "requirements" => &setup.requirements,
                    _ => &setup.ci,
                };
                let line = self.ask(&format!("{}: ({}) ", text, current))?;
                let answer = strip(line.as_deref().unwrap_or("")).to_string();
                if answer.is_empty() {
                    break;
                }
                match fault(key, &answer) {
                    None => {
                        match key {
                            "project" => setup.project = answer,
                            "git_server" => setup.git_server = answer,
                            "repository" => setup.repository = answer,
                            "tracker" => setup.tracker = answer,
                            "requirements" => setup.requirements = answer,
                            _ => setup.ci = answer,
                        }
                        break;
                    }
                    Some(why) => self.say(&format!("Not taken: {}.", why)),
                }
            }
        }
        Ok(setup)
    }
}

/// Runs `wb init` in the repository at `root`. Returns the exit status.
pub fn init(
    root: &str,
    revision: &str,
    package: &Path,
    yes: bool,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
    env: Option<&[(String, String)]>,
) -> i32 {
    let mut talk = Talk { input, out };
    let folder = if root.ends_with('/') { format!("{}{}", root, CONFIG_FOLDER) } else { format!("{}/{}", root, CONFIG_FOLDER) };
    let file = format!("{}/{}", folder, CONFIG_FILE);
    let pin = match read_pin(package) {
        Ok(pin) => pin,
        Err(why) => {
            let _ = writeln!(err, "wb init: the revision of the package cannot be read: {}. Nothing written.", why);
            return 1;
        }
    };
    // The files, as paths to write: the manifest and the files of the revision, then the config last,
    // so that a repository that holds it is one that was set up whole.
    let mut targets: Vec<String> = pin.files.iter().map(|(path, _)| format!("{}/{}", folder, path)).collect();
    targets.push(file.clone());
    if let Some(found) = in_the_way(&folder, &targets) {
        talk.say(&format!("{} is already there. wb init does not change it. Nothing written.", found));
        return 1;
    }

    let mut setup = match detect(root, env) {
        Ok(setup) => setup,
        Err(why) => {
            let _ = writeln!(err, "wb init: {}. Nothing written.", why);
            return 1;
        }
    };
    if !yes {
        talk.say(&format!("This sets Watchbill {} up in {}.", revision, root));
        talk.say(&format!("It asks a few things and writes {}/{}. What it found in the", CONFIG_FOLDER, CONFIG_FILE));
        talk.say("repository stands in brackets: Enter keeps it. Ctrl-C leaves at any time.");
        talk.say("");
        setup = match talk.answers(&setup) {
            Ok(setup) => setup,
            Err(error) => {
                let _ = err.write_all(read_failure(&error).as_bytes());
                return 1;
            }
        };
    }

    let text = render(revision, &pin.hash, &setup);
    talk.say(&format!("\nAbout to write {}:\n\n{}", file, text));
    talk.say("and these files of the revision, byte for byte as the package has them:\n");
    for path in &targets[..targets.len() - 1] {
        talk.say(&format!("  {}", path));
    }
    talk.say("");
    if !yes {
        match talk.ask("Is this OK? (yes) ") {
            Ok(None) => {
                // No answer at all, because the input ended, is no yes.
                talk.say("Nothing written.");
                return 1;
            }
            Ok(Some(answer)) => {
                if !["", "y", "yes"].contains(&strip(&answer).to_lowercase().as_str()) {
                    talk.say("Nothing written.");
                    return 1;
                }
            }
            Err(error) => {
                let _ = err.write_all(read_failure(&error).as_bytes());
                return 1;
            }
        }
    }

    // From here an interrupt no longer leaves nothing written.
    interrupt::writing();
    let mut contents: Vec<&[u8]> = pin.files.iter().map(|(_, bytes)| bytes.as_slice()).collect();
    contents.push(text.as_bytes());
    if let Err((path, error)) = write_all(&targets, &contents) {
        if error.kind() == ErrorKind::AlreadyExists {
            talk.say(&format!("{} is already there. wb init does not change it. Nothing written.", path.display()));
        } else {
            let _ = writeln!(err, "wb init: cannot write {}: {}. Nothing written.", path.display(), error);
        }
        return 1;
    }
    for path in &targets {
        talk.say(&format!("Wrote {}.", path));
    }
    talk.say("Keep them under version control with the code.");
    0
}

/// Writes every file, the last one last. Nothing is written over a file that is there. When one
/// fails, what this call made is removed again, so a failure leaves no half set-up repository: the
/// error is the path that failed and why.
fn write_all(files: &[String], contents: &[&[u8]]) -> Result<(), (PathBuf, io::Error)> {
    let mut made: Vec<(PathBuf, bool)> = Vec::new();
    let result = (|| {
        for (file, bytes) in files.iter().zip(contents) {
            let file = Path::new(file);
            // The folders above it, from the outside in, each noted when this call made it.
            let mut missing = Vec::new();
            let mut above = file.parent();
            while let Some(path) = above {
                if path.exists() || path.as_os_str().is_empty() {
                    break;
                }
                missing.push(path);
                above = path.parent();
            }
            for path in missing.into_iter().rev() {
                fs::create_dir(path).map_err(|error| (path.to_path_buf(), error))?;
                made.push((path.to_path_buf(), true));
            }
            // `create_new`: never over a file that came to be there meanwhile.
            let mut out = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(file)
                .map_err(|error| (file.to_path_buf(), error))?;
            made.push((file.to_path_buf(), false));
            out.write_all(bytes).map_err(|error| (file.to_path_buf(), error))?;
        }
        Ok(())
    })();
    if result.is_err() {
        for (path, is_folder) in made.into_iter().rev() {
            let _ = if is_folder { fs::remove_dir(&path) } else { fs::remove_file(&path) };
        }
    }
    result
}

/// The root of the git repository the command runs in: `Ok(None)` outside one. An error is a path
/// that is not UTF-8.
pub fn repository_root(env: Option<&[(String, String)]>) -> Result<Option<String>, String> {
    match crate::detect::git_output(&["rev-parse", "--show-toplevel"], env) {
        Some(run) if run.status.success() => match String::from_utf8(run.stdout) {
            Ok(text) => {
                let root = strip(&universal_newlines(&text)).to_string();
                Ok(if root.is_empty() { None } else { Some(root) })
            }
            Err(_) => Err("the path of the repository is not UTF-8 text".to_string()),
        },
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn test_WB_PIN_10_a_target_outside_the_folder_is_in_the_way_and_named() {
        let found = in_the_way("/nowhere/.watchbill", &["/elsewhere/file.md".to_string()]);

        let message = found.expect("a target outside the folder was taken as nothing in the way");
        assert!(message.contains("/elsewhere/file.md"), "{}", message);
    }
}
