//! The lines a start leaves (#124): for each start of `wb start <role>`, a folder
//! `<state>/watchbill/sessions/<time>-<role>-<issue>/` with the stamp and the outcome, on the machine where
//! it ran. They are diagnostics, not a record: only `wb start` and `wb status` read them back, to name a view
//! kept with work not taken back and a session that was cut off (`kept`, #222). A folder that cannot be written
//! is said, and changes neither the status nor whether the session starts.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

/// The state folder of this machine's user: `$XDG_STATE_HOME` when it is an absolute path, else
/// `$HOME/.local/state`, or none. The XDG specification ignores a relative path.
pub fn state_folder() -> Option<PathBuf> {
    let absolute = |name: &str| std::env::var_os(name).map(PathBuf::from).filter(|path| path.is_absolute());
    absolute("XDG_STATE_HOME").or_else(|| absolute("HOME").map(|home| home.join(".local/state")))
}

/// How a start ended.
pub enum Outcome {
    /// Refused at a check: `prompt`, `rule`, `class`, `start`, `ci` (#127), `terminal` or `view` (#126).
    Refused { check: &'static str, status: i32, message: String },
    /// The stamp was not posted, and no session was started.
    NotPosted { status: i32, message: String },
    /// The session ran, and ended.
    Ended { status: ExitStatus, seconds: u64 },
}

impl Outcome {
    /// The lines of `outcome`, as `name: value`, in a fixed order.
    fn text(&self) -> String {
        match self {
            Outcome::Refused { check, status, message } => {
                format!("outcome: refused\ncheck: {}\nstatus: {}\nmessage: {}\n", check, status, first_line(crate::refusal::unlabelled(message)))
            }
            Outcome::NotPosted { status, message } => {
                format!("outcome: not posted\nstatus: {}\nmessage: {}\n", status, first_line(crate::refusal::unlabelled(message)))
            }
            Outcome::Ended { status, seconds } => match (status.code(), status.signal()) {
                (Some(code), _) => format!("outcome: ended\nstatus: {}\nseconds: {}\n", code, seconds),
                (None, Some(signal)) => format!("outcome: ended\nsignal: {}\nseconds: {}\n", signal_name(signal), seconds),
                (None, None) => format!("outcome: ended\nseconds: {}\n", seconds),
            },
        }
    }
}

/// A signal as `outcome` writes it (#221): the name of the usual ones, else the number.
fn signal_name(signal: i32) -> String {
    match signal {
        1 => "SIGHUP".to_string(),
        2 => "SIGINT".to_string(),
        3 => "SIGQUIT".to_string(),
        9 => "SIGKILL".to_string(),
        15 => "SIGTERM".to_string(),
        other => other.to_string(),
    }
}

/// The first line of a message.
fn first_line(message: &str) -> &str {
    message.lines().next().unwrap_or_default()
}

/// The folder of one start, made when it is first written.
pub struct Lines {
    /// What starts each line said: `wb start <role>`.
    name: String,
    /// The folder the start's folder goes below, `<state>/watchbill/sessions`, or why there is none.
    below: Result<PathBuf, String>,
    /// `<time>-<role>-<issue>`.
    base: String,
    /// The folder made, once it is.
    made: Option<PathBuf>,
    /// Whether it was tried: a folder is made once, and a failure said once.
    tried: bool,
}

impl Lines {
    /// The lines of the start of `role` for `issue` (`none` when there is none), at `seconds` since 1970,
    /// below `state`. With no state folder, or no time, nothing is written.
    pub fn new(name: &str, state: Option<&Path>, seconds: Result<u64, String>, role: &str, issue: Option<&str>) -> Lines {
        let time = seconds.as_ref().map(|seconds| crate::prompt::utc(*seconds).replace(':', "")).unwrap_or_default();
        let below = match (state, seconds) {
            (None, _) => Err("neither XDG_STATE_HOME nor HOME names an absolute folder".to_string()),
            (_, Err(why)) => Err(why),
            (Some(state), Ok(_)) => Ok(state.join("watchbill").join("sessions")),
        };
        Lines { name: name.to_string(), below, base: format!("{}-{}-{}", time, role, issue.unwrap_or("none")), made: None, tried: false }
    }

    /// Makes the folder, a new one, and writes `stamp` in it when there is one. What it will write is said
    /// first; a folder that cannot be written is said, and nothing else follows from it.
    pub fn open(&mut self, stamp: Option<&str>, err: &mut dyn Write) {
        if self.tried {
            return;
        }
        self.tried = true;
        let below = match &self.below {
            Ok(below) => below.clone(),
            Err(why) => {
                let _ = writeln!(err, "{}: no lines of this start are written: {}.", self.name, why);
                return;
            }
        };
        // Never over a folder that is there: `-2`, `-3` and on, and a folder made between the look and the
        // making is passed over too.
        let mut number = 1;
        let folder = loop {
            let folder = match number {
                1 => below.join(&self.base),
                n => below.join(format!("{}-{}", self.base, n)),
            };
            number += 1;
            if fs::symlink_metadata(&folder).is_ok() {
                continue;
            }
            let _ = writeln!(err, "{}: will write the lines of this start in {}", self.name, folder.display());
            if let Err(error) = fs::create_dir_all(&below) {
                let _ = writeln!(err, "{}: the lines of this start could not be written in {}: {}", self.name, folder.display(), error);
                return;
            }
            match fs::create_dir(&folder) {
                Ok(()) => break folder,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && number < 10_000 => continue,
                Err(error) => {
                    let _ = writeln!(err, "{}: the lines of this start could not be written in {}: {}", self.name, folder.display(), error);
                    return;
                }
            }
        };
        if let Some(stamp) = stamp {
            if let Err(error) = new_file(&folder.join("stamp"), stamp) {
                let _ = writeln!(err, "{}: the lines of this start could not be written in {}: {}", self.name, folder.display(), error);
                return;
            }
        }
        self.made = Some(folder);
    }

    /// The path of a file of the folder made, or none when no folder was made.
    pub fn file(&self, file: &str) -> Option<PathBuf> {
        self.made.as_ref().map(|folder| folder.join(file))
    }

    /// Writes a file of its own beside the stamp (#126: `view` and `back`), in the folder made, or in a new
    /// one when none was made yet. A file that cannot be written is said, and nothing else follows from it.
    pub fn write(&mut self, file: &str, text: &str, err: &mut dyn Write) {
        self.open(None, err);
        let Some(folder) = &self.made else { return };
        if let Err(error) = new_file(&folder.join(file), text) {
            let _ = writeln!(err, "{}: the lines of this start could not be written in {}: {}", self.name, folder.display(), error);
        }
    }

    /// Writes `outcome` as `running`, with the process ID of this `wb` and its start time, in the folder made,
    /// before the session starts (#222): `close` replaces it. A `wb` that is ended before its session leaves it.
    pub fn running(&mut self, err: &mut dyn Write) {
        self.open(None, err);
        let Some(folder) = &self.made else { return };
        let started = crate::kept::start_time(std::process::id()).map(|ticks| format!("started: {}\n", ticks)).unwrap_or_default();
        let text = format!("outcome: running\npid: {}\n{}", std::process::id(), started);
        if let Err(error) = new_file(&folder.join("outcome"), &text) {
            let _ = writeln!(err, "{}: the lines of this start could not be written in {}: {}", self.name, folder.display(), error);
        }
    }

    /// Writes `outcome`, in the folder made, or in a new one with `stamp` when none was made yet. It replaces the
    /// `running` of `running`, by a file of its own that is moved onto it.
    pub fn close(&mut self, stamp: Option<&str>, outcome: Outcome, err: &mut dyn Write) {
        self.open(stamp, err);
        let Some(folder) = &self.made else { return };
        if let Err(error) = replace_file(&folder.join("outcome"), &outcome.text()) {
            let _ = writeln!(err, "{}: the lines of this start could not be written in {}: {}", self.name, folder.display(), error);
        }
    }
}

/// Replaces a file wb wrote itself: the text goes to `<path>.new`, which is not there, and is moved onto the file.
fn replace_file(path: &Path, text: &str) -> std::io::Result<()> {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".new");
    let temporary = PathBuf::from(temporary);
    new_file(&temporary, text)?;
    fs::rename(&temporary, path).inspect_err(|_| {
        let _ = fs::remove_file(&temporary);
    })
}

/// Writes a file that is not there.
fn new_file(path: &Path, text: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(text.as_bytes())
}

/// Appends to the file `back` of the folder of a start what a take-back did (#223): the old lines, then `text`. The
/// file is written anew beside it and moved onto it, as `outcome` is; where there is none, `text` is the file.
pub fn append_back(folder: &Path, text: &str) -> std::io::Result<()> {
    let path = folder.join("back");
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => new_file(&path, text),
        Err(error) => Err(error),
        Ok(_) => match crate::view::small_file(&path) {
            Some(old) => replace_file(&path, &format!("{}{}", old, text)),
            None => Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "the file `back` is not a plain text file of 1 MiB or less")),
        },
    }
}
