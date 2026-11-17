//! The command `wb`.

use std::io::{BufRead, Write};
use std::path::Path;

pub mod detect;
pub mod init;
pub mod interrupt;
pub mod manifest;
pub mod rules;
mod word;
pub mod yamlfile;

/// The version of the package.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What `wb --help` prints. The words are those of the Python `wb`.
pub const USAGE: &str = "usage:
  wb init [--yes]
  wb revision
  wb --version

  init      set Watchbill up in the git repository of this folder: propose what
            the repository says about itself (its name, the git server of
            origin, a folder of requirements, the CI driver), ask one line per
            answer, show the file and write .watchbill/watchbill.yml at the
            root of the repository. --yes (-y) takes what it found and asks
            nothing. A file that is already there is never changed
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

/// Runs `wb` with the words after its name and returns the exit status:
/// 0 it holds or it is done, 1 it does not hold or it was refused, 2 it was used wrongly.
pub fn run(args: &[String], input: &mut dyn BufRead, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    // A failed write, such as a closed pipe, does not change what the command ended with.
    match args.first().map(String::as_str) {
        None => {
            let _ = out.write_all(USAGE.as_bytes());
            2
        }
        Some("--help") | Some("-h") | Some("help") => {
            let _ = out.write_all(USAGE.as_bytes());
            0
        }
        Some("--version") | Some("-v") => {
            let _ = writeln!(out, "{}", VERSION);
            0
        }
        Some("init") => run_init(&args[1..], input, out, err),
        Some("revision") => revision(&args[1..], Path::new(manifest::PACKAGE_FOLDER), out, err),
        Some(command) => {
            let _ = write!(err, "wb: no command '{}'\n\n{}", command, USAGE);
            2
        }
    }
}

/// `wb init [--yes]`
fn run_init(args: &[String], input: &mut dyn BufRead, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut yes = false;
    for arg in args {
        if arg == "--yes" || arg == "-y" {
            yes = true;
        } else {
            let _ = write!(err, "wb init: '{}' is not known\n\n{}", arg, USAGE);
            return 2;
        }
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
        Ok(Some(root)) => init::init(&root, VERSION, yes, input, out, err, None),
    }
}

/// `wb revision`: the version of the package, the hash of the revision and the manifest. It writes
/// nothing. The folder that holds `revision/` is an argument so that a test can give its own; `run`
/// gives the one fixed when `wb` was built.
pub fn revision(args: &[String], base: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    if let Some(arg) = args.first() {
        let _ = write!(err, "wb revision: '{}' is not known\n\n{}", arg, USAGE);
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
