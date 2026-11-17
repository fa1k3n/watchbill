//! The command `wb`.

use std::io::Write;

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

/// Runs `wb` with the words after its name and returns the exit status:
/// 0 it holds or it is done, 1 it does not hold or it was refused, 2 it was used wrongly.
///
/// `init` and `revision` are not built in this program yet, so they are commands it does not know.
pub fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
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
        Some(command) => {
            let _ = write!(err, "wb: no command '{}'\n\n{}", command, USAGE);
            2
        }
    }
}
