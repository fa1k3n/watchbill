//! The command `wb`.

use lexopt::{Arg, Parser};
use std::ffi::OsString;
use std::io::{BufRead, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub mod check;
pub mod detect;
pub mod init;
pub mod interrupt;
pub mod manifest;
pub mod prompt;
pub mod rules;
mod word;
pub mod yamlfile;

/// The version of the package.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What `wb --help` prints. The words are those of the Python `wb`.
pub const USAGE: &str = "usage:
  wb init [--yes]
  wb check
  wb prompt <role> [--task <text>] [--change <text>] [--requirements <text>]
            [--round <text>] [--parts]
  wb revision
  wb --version

  init      set Watchbill up in the git repository of this folder: propose what
            the repository says about itself (its name, the git server of
            origin, a folder of requirements, the CI driver), ask one line per
            answer, show the file and write .watchbill/watchbill.yml at the
            root of the repository. --yes (-y) takes what it found and asks
            nothing. A file that is already there is never changed
  check     say whether the repository still is at the revision it is pinned to:
            each file under .watchbill/revision/ against the manifest, the
            manifest against the revision line of watchbill.yml, and that
            revision against the installed wb. Ends with 0 when all of it
            holds, 1 when something differs. Writes nothing
  prompt    print the prompt of a role on standard output and nothing else: the
            role's text of the pinned revision with its placeholders filled
            from --task, --change, --requirements and --round, then the
            project's own text. It runs the check of wb check first and prints
            nothing when that fails. --parts prints the path and the SHA-256 of
            each part, in the form sha256sum prints, in place of the text.
            Ends with 0 when the prompt is printed, 1 when it is refused, 2
            when it was used wrongly or the repository is not set up. Writes
            nothing
  revision  print the version of the package, the hash of its revision and the
            manifest: the hash of every file the revision holds. Writes nothing
";

/// Runs a command so that a fault in `wb` itself is not shown as a panic: the hook prints nothing,
/// and a panic that is caught ends the command with 1 (it did not hold) and one line on `err`.
/// `main` runs every command through it. The body is a parameter so that a test can make it panic.
pub fn guarded(body: impl FnOnce() -> i32, err: &mut dyn Write) -> i32 {
    std::panic::set_hook(Box::new(|_| {}));
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(code) => code,
        Err(_) => {
            let _ = writeln!(err, "wb: an internal error. This is a fault in wb, not in what it was given.");
            1
        }
    }
}

/// A word of the command line as text. What is not UTF-8 is written as the Python `wb` writes
/// it: each byte that is not part of a character as `\udcXX`.
pub fn word(bytes: &[u8]) -> String {
    let mut text = String::new();
    let mut rest = bytes;
    while !rest.is_empty() {
        match std::str::from_utf8(rest) {
            Ok(valid) => {
                text.push_str(valid);
                break;
            }
            Err(error) => {
                let (valid, after) = rest.split_at(error.valid_up_to());
                text.push_str(std::str::from_utf8(valid).unwrap_or(""));
                text.push_str(&format!("\\udc{:02x}", after[0]));
                rest = &after[1..];
            }
        }
    }
    text
}

/// A word of the command line as `lexopt` read it, held apart from the parser.
enum Word {
    Short(char),
    Long(String),
    Value(OsString),
}

/// The next option or word. An option given a value it does not take, as `--yes=x` or `-y=x`, is
/// refused here, and the error is the word to name: the option with its value.
fn next(parser: &mut Parser) -> Result<Option<Word>, String> {
    match parser.next() {
        Ok(None) => Ok(None),
        Ok(Some(Arg::Short(option))) => Ok(Some(Word::Short(option))),
        Ok(Some(Arg::Long(option))) => Ok(Some(Word::Long(option.to_string()))),
        Ok(Some(Arg::Value(value))) => Ok(Some(Word::Value(value))),
        Err(lexopt::Error::UnexpectedValue { option, value }) => Err(format!("{}={}", option, word(value.as_bytes()))),
        // `next` of lexopt 0.3 gives no other error. Were a later one to, the text of `lexopt` is
        // not shown: the word is named empty and the command is still refused with 2.
        Err(_) => Err(String::new()),
    }
}

/// A word a command does not take, as `wb` names it: an option alone, with the value written
/// after it, and a value as it was given.
fn named(found: Word, parser: &mut Parser) -> String {
    match found {
        // `-x=1`: the value is named with the option. Otherwise what follows `-x` in the word is
        // other options, and the first that is not known is named alone.
        Word::Short(option) => match parser.next() {
            Err(lexopt::Error::UnexpectedValue { value, .. }) => format!("-{}={}", option, word(value.as_bytes())),
            _ => format!("-{}", option),
        },
        Word::Long(option) => match parser.optional_value() {
            Some(value) => format!("--{}={}", option, word(value.as_bytes())),
            None => format!("--{}", option),
        },
        Word::Value(value) => word(value.as_bytes()),
    }
}

/// Runs `wb` with the words after its name and returns the exit status:
/// 0 it holds or it is done, 1 it does not hold or it was refused, 2 it was used wrongly.
pub fn run(args: &[OsString], input: &mut dyn BufRead, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    // A failed write, such as a closed pipe, does not change what the command ended with.
    let mut parser = Parser::from_args(args.iter().cloned());
    // The first word is the command. `-h` and `-v` are taken only alone, so an option there that is
    // not taken was the whole first word, and it is named as it was given.
    let first = || word(args.first().map(|arg| arg.as_bytes()).unwrap_or_default());
    let command = match next(&mut parser) {
        Ok(None) => {
            let _ = out.write_all(USAGE.as_bytes());
            return 2;
        }
        Ok(Some(Word::Short('h'))) if parser.optional_value().is_none() => "help".to_string(),
        Ok(Some(Word::Long(option))) if option == "help" && parser.optional_value().is_none() => "help".to_string(),
        Ok(Some(Word::Short('v'))) if parser.optional_value().is_none() => "version".to_string(),
        Ok(Some(Word::Long(option))) if option == "version" && parser.optional_value().is_none() => {
            "version".to_string()
        }
        Ok(Some(Word::Value(value))) => match value.to_str() {
            Some(name @ ("help" | "init" | "check" | "prompt" | "revision")) => name.to_string(),
            _ => {
                let _ = write!(err, "wb: no command '{}'\n\n{}", word(value.as_bytes()), USAGE);
                return 2;
            }
        },
        _ => {
            let _ = write!(err, "wb: no command '{}'\n\n{}", first(), USAGE);
            return 2;
        }
    };
    // Words after `--help` and `--version` change nothing, as in the Python `wb`.
    match command.as_str() {
        "help" => {
            let _ = out.write_all(USAGE.as_bytes());
            0
        }
        "version" => {
            let _ = writeln!(out, "{}", VERSION);
            0
        }
        "init" => run_init(&mut parser, input, out, err),
        "check" => run_check(&mut parser, out, err),
        "prompt" => run_prompt(&mut parser, out, err),
        _ => run_revision(&mut parser, Path::new(manifest::PACKAGE_FOLDER), out, err),
    }
}

/// `wb init [--yes]`
fn run_init(parser: &mut Parser, input: &mut dyn BufRead, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut yes = false;
    loop {
        let refused = match next(parser) {
            Ok(None) => break,
            Ok(Some(Word::Short('y'))) => {
                yes = true;
                continue;
            }
            Ok(Some(Word::Long(option))) if option == "yes" => {
                yes = true;
                continue;
            }
            Ok(Some(found)) => named(found, parser),
            Err(refused) => refused,
        };
        let _ = write!(err, "wb init: '{}' is not known\n\n{}", refused, USAGE);
        return 2;
    }

    interrupt::install();
    match init::repository_root(None) {
        Err(why) => {
            let _ = writeln!(err, "wb init: {}. Nothing written.", why);
            1
        }
        Ok(None) => {
            let _ = writeln!(err, "wb init: this folder is not in a git repository. Watchbill is set up in one.");
            1
        }
        Ok(Some(root)) => init::init(&root, VERSION, Path::new(manifest::PACKAGE_FOLDER), yes, input, out, err, None),
    }
}

/// `wb check`: the repository against its pinned revision and the installed `wb`. It writes nothing.
fn run_check(parser: &mut Parser, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let refused = match next(parser) {
        Ok(None) => None,
        Ok(Some(found)) => Some(named(found, parser)),
        Err(refused) => Some(refused),
    };
    if let Some(refused) = refused {
        let _ = write!(err, "wb check: '{}' is not known\n\n{}", refused, USAGE);
        return 2;
    }
    match init::repository_root(None) {
        Err(why) => {
            let _ = writeln!(err, "wb check: {}.", why);
            2
        }
        Ok(None) => {
            let _ = writeln!(err, "wb check: this folder is not in a git repository. Watchbill is set up in one.");
            2
        }
        Ok(Some(root)) => check::check(&root, VERSION, Path::new(manifest::PACKAGE_FOLDER), out, err),
    }
}

/// `wb prompt <role> [--task X] [--change X] [--requirements X] [--round X] [--parts]`: the prompt of
/// a role, or with `--parts` the path and hash of each of its parts. Nothing is printed unless the
/// check of `wb check` holds and the prompt can be put together. It writes nothing.
fn run_prompt(parser: &mut Parser, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let used = |err: &mut dyn Write, why: String| {
        let _ = write!(err, "wb prompt: {}\n\n{}", why, USAGE);
        2
    };
    let mut role: Option<String> = None;
    let mut values: Vec<(&'static str, String)> = Vec::new();
    let mut parts = false;
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(refused) => return used(err, format!("'{}' is not known", refused)),
        };
        match found {
            Word::Long(option) if option == "parts" => parts = true,
            Word::Long(option) if prompt::PLACEHOLDERS.contains(&option.as_str()) => {
                let name = prompt::PLACEHOLDERS.iter().find(|name| **name == option).unwrap();
                // A value is whatever follows, also when it starts with `-`.
                let value = match parser.value() {
                    Ok(value) => value,
                    Err(_) => return used(err, format!("'--{}' needs a value", option)),
                };
                let Ok(value) = value.into_string() else {
                    return used(err, format!("the value of '--{}' is not UTF-8 text", option));
                };
                if value.is_empty() {
                    return used(err, format!("the value of '--{}' is empty", option));
                }
                if values.iter().any(|(given, _)| given == name) {
                    return used(err, format!("'--{}' is given twice", option));
                }
                values.push((name, value));
            }
            Word::Value(value) if role.is_none() => role = Some(word(value.as_bytes())),
            found => {
                let refused = named(found, parser);
                return used(err, format!("'{}' is not known", refused));
            }
        }
    }
    let Some(role) = role else {
        return used(err, "a role is needed".to_string());
    };
    let root = match init::repository_root(None) {
        Err(why) => {
            let _ = writeln!(err, "wb prompt: {}.", why);
            return 2;
        }
        Ok(None) => {
            let _ = writeln!(err, "wb prompt: this folder is not in a git repository. Watchbill is set up in one.");
            return 2;
        }
        Ok(Some(root)) => root,
    };
    // What `wb check` says is for standard error here: standard output holds the prompt or nothing.
    let (mut said, mut why) = (Vec::new(), Vec::new());
    let status = check::check(&root, VERSION, Path::new(manifest::PACKAGE_FOLDER), &mut said, &mut why);
    if status != 0 {
        let _ = writeln!(err, "wb prompt: nothing printed, the check of `wb check` does not hold.");
        let _ = err.write_all(&said);
        let _ = err.write_all(&why);
        return status;
    }
    let given: Vec<(&str, &str)> = values.iter().map(|(name, value)| (*name, value.as_str())).collect();
    match prompt::assemble(Path::new(&root), &role, &given) {
        Err(why) => {
            let _ = writeln!(err, "wb prompt: {}", why);
            1
        }
        Ok(found) => {
            if parts {
                let lines: String = found.parts.iter().map(|part| format!("{}  {}\n", part.hash, part.path)).collect();
                let _ = out.write_all(lines.as_bytes());
            } else {
                let _ = out.write_all(found.text.as_bytes());
            }
            0
        }
    }
}

/// `wb revision`: the version of the package, the hash of the revision and the manifest. It writes
/// nothing. The folder that holds `revision/` is an argument so that a test can give its own; `run`
/// gives the one fixed when `wb` was built.
pub fn revision(args: &[OsString], base: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    run_revision(&mut Parser::from_args(args.iter().cloned()), base, out, err)
}

fn run_revision(parser: &mut Parser, base: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let refused = match next(parser) {
        Ok(None) => None,
        Ok(Some(found)) => Some(named(found, parser)),
        Err(refused) => Some(refused),
    };
    if let Some(refused) = refused {
        let _ = write!(err, "wb revision: '{}' is not known\n\n{}", refused, USAGE);
        return 2;
    }
    match manifest::manifest(base) {
        Err(why) => {
            let _ = writeln!(err, "wb revision: {}", why);
            1
        }
        Ok(text) => {
            let _ = write!(out, "watchbill {}\nrevision {}\n\n{}", VERSION, manifest::hash_of(&text), text);
            0
        }
    }
}
