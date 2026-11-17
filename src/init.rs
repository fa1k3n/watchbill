//! `wb init`: sets Watchbill up in a repository, the way `git init` starts one.
//!
//! It proposes what the repository says about itself, asks one line per answer,
//! shows the file and writes `.watchbill/watchbill.yml`. A file that is already
//! there is never changed.

use std::fs;
use std::io::{self, BufRead, ErrorKind, Write};
use std::path::Path;

use crate::detect::{detect, strip, universal_newlines, Setup, NONE};
use crate::interrupt;

pub const CONFIG_FOLDER: &str = ".watchbill";
pub const CONFIG_FILE: &str = "watchbill.yml";

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
pub fn render(revision: &str, setup: &Setup) -> String {
    let rows = [
        ("watchbill", revision),
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

/// What is said, with 1, when an answer is not UTF-8 text.
const NOT_UTF_8: &str = "wb init: what was typed is not UTF-8 text. Nothing written.\n";

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
    yes: bool,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
    env: Option<&[(String, String)]>,
) -> i32 {
    let mut talk = Talk { input, out };
    let folder = if root.ends_with('/') { format!("{}{}", root, CONFIG_FOLDER) } else { format!("{}/{}", root, CONFIG_FOLDER) };
    let file = format!("{}/{}", folder, CONFIG_FILE);
    if fs::symlink_metadata(&file).is_ok() {
        talk.say(&format!("{} is already there. wb init does not change it.", file));
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
            Err(_) => {
                let _ = err.write_all(NOT_UTF_8.as_bytes());
                return 1;
            }
        };
    }

    let text = render(revision, &setup);
    talk.say(&format!("\nAbout to write {}:\n\n{}", file, text));
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
            Err(_) => {
                let _ = err.write_all(NOT_UTF_8.as_bytes());
                return 1;
            }
        }
    }

    // From here an interrupt no longer leaves nothing written.
    interrupt::writing();
    if let Err(error) = write_file(Path::new(&folder), Path::new(&file), &text) {
        if error.kind() == ErrorKind::AlreadyExists && fs::symlink_metadata(&file).is_ok() {
            talk.say(&format!("{} is already there. wb init does not change it.", file));
        } else {
            let _ = writeln!(err, "wb init: cannot write {}: {}", file, error);
        }
        return 1;
    }
    talk.say(&format!("Wrote {}.", file));
    talk.say("Keep it under version control with the code.");
    0
}

fn write_file(folder: &Path, file: &Path, text: &str) -> io::Result<()> {
    fs::create_dir_all(folder)?;
    // `create_new`: never over a file that came to be there meanwhile.
    let mut out = fs::OpenOptions::new().write(true).create_new(true).open(file)?;
    out.write_all(text.as_bytes())
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
