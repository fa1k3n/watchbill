//! The command `wb`.

use std::io::{BufRead, Write};

pub mod detect;
pub mod init;
pub mod interrupt;
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
///
/// `revision` is not built in this program yet (#24), so it is a command it does not know.
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
