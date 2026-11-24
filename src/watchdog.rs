//! The watchdog (#267): the `wb` that waits for a session is the one that sees it run long or do nothing. There is no
//! command and no daemon of its own.
//!
//! Each role has two limits in the rule table, `quiet_minutes` and `max_minutes`, and `watchbill.yml` may set them
//! lower for a role under `session_limits`, never higher. While `wb start` waits, it looks at the view now and then:
//! past `quiet_minutes` with nothing changed in it, it says so in one line, on standard error and in the file `quiet` of
//! the lines of the start, and again each time the quiet doubles. Past `max_minutes` it ends the session as a
//! termination of `wb` does (#221), and kills it after a grace.
//!
//! What is looked at is the view as `wb` sees it, never what the session says: the commit its `HEAD` names, read from
//! its files through the one bounded reader, and the time and the number of the entries under the paths of its list,
//! by their metadata alone: no file of the view is opened, no link is followed in any part of a path, and at most
//! 20000 entries are read, within 5 seconds, in a thread that the end at `max_minutes` never waits for. The quiet is counted by `wb`'s own
//! clock, from the start of the session or the last look that found any of it changed, so a time a session writes on a
//! file or a commit is never read as when it worked.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use crate::rules::{self, Rules};
use crate::yamlfile::{Document, Part, YamlError};

/// The key of `watchbill.yml` under which a team sets lower limits for a role.
pub const KEY: &str = "session_limits";

/// How many entries under the paths of a view are looked at, at most, in one look.
const MOST_ENTRIES: usize = 20_000;

/// The most time between two looks at the view.
const LOOK_EVERY: Duration = Duration::from_secs(10);

/// The most time a look at the view takes: past it, what it read is not taken.
const LOOK_DEADLINE: Duration = Duration::from_secs(5);

/// The time a session is given to end after the termination, before it is killed.
const GRACE: Duration = Duration::from_secs(10);

/// The limits of a session of one role, in minutes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub quiet_minutes: u64,
    pub max_minutes: u64,
}

/// The limits of `role`: those of the rule table `table`, or a lower one that `session_limits` of `watchbill.yml`
/// (`document`) sets. A role or a key that is not known, a value that is not a whole number of at least 1, or one
/// higher than the table's, is refused, naming its place.
pub fn limits_of(document: &Document, table: &Rules, role: &str) -> Result<Limits, YamlError> {
    let set = read(document, table)?;
    let row = table.roles.get(role);
    let of_table = |field: &str| -> i64 {
        match (row, field) {
            (Some(row), "quiet_minutes") => row.quiet_minutes,
            (Some(row), _) => row.max_minutes,
            (None, "quiet_minutes") => rules::DEFAULT_QUIET_MINUTES,
            (None, _) => rules::DEFAULT_MAX_MINUTES,
        }
    };
    let given = set.get(role);
    let quiet = given.and_then(|limits| limits.0).unwrap_or_else(|| of_table("quiet_minutes"));
    let max = given.and_then(|limits| limits.1).unwrap_or_else(|| of_table("max_minutes"));
    Ok(Limits { quiet_minutes: quiet.max(1) as u64, max_minutes: max.max(1) as u64 })
}

/// The limits that `session_limits` of `watchbill.yml` sets, by role: the quiet and the most, where each is given.
fn read(document: &Document, table: &Rules) -> Result<BTreeMap<String, (Option<i64>, Option<i64>)>, YamlError> {
    let mut set = BTreeMap::new();
    let top = [Part::from(KEY)];
    if !document.has(&top) {
        return Ok(set);
    }
    for role in document.table(&top)? {
        let place = [Part::from(KEY), Part::from(role.as_str())];
        let Some(row) = table.roles.get(&role) else {
            return Err(document.fail(&place, &format!("`{}` is not one of the roles: {}", role, rules::ROLES.join(", ")), true));
        };
        document.only(&place, &rules::LIMIT_FIELDS)?;
        let mut limits = (None, None);
        for (field, of_table) in [("quiet_minutes", row.quiet_minutes), ("max_minutes", row.max_minutes)] {
            let here = [Part::from(KEY), Part::from(role.as_str()), Part::from(field)];
            if !document.has(&here) {
                continue;
            }
            let value = document.whole(&here, Some(1))?;
            if value > of_table {
                return Err(document.fail(
                    &here,
                    &format!("{} minutes is higher than the {} minutes of the rule table, and watchbill.yml may set a limit lower, never higher", value, of_table),
                    false,
                ));
            }
            if field == "quiet_minutes" {
                limits.0 = Some(value);
            } else {
                limits.1 = Some(value);
            }
        }
        set.insert(role, limits);
    }
    Ok(set)
}

/// The fault of `session_limits` in `watchbill.yml`, against the rule table of the installed revision, for `wb check`;
/// none where it holds.
pub fn fault(document: &Document) -> Option<String> {
    let table = match rules::load_revision() {
        Ok(table) => table,
        Err(why) => return document.has(&[Part::from(KEY)]).then(|| format!("{} cannot be checked, as the rule table cannot be read: {}", KEY, why)),
    };
    read(document, &table).err().map(|why| why.to_string())
}

/// The limits of `role` for a start in the repository at `root`: the rule table of the installed revision and
/// `watchbill.yml` as `HEAD` has it. The error is why not, naming the key.
pub fn limits_for(root: &str, role: &str) -> Result<Limits, String> {
    let table = rules::load_revision().map_err(|why| why.to_string())?;
    let document = crate::starter::settings(root)?;
    limits_of(&document, &table, role).map_err(|why| why.to_string())
}

/// A time in whole seconds that a test sets by the variable `name` (#279): only a build with debug assertions, which
/// `cargo test` makes, reads it; `cargo build --release`, as the RPM builds `wb`, has no code that reads it.
#[cfg(debug_assertions)]
fn test_seconds(name: &str) -> Option<Duration> {
    std::env::var(name).ok()?.parse().ok().filter(|seconds| *seconds > 0).map(Duration::from_secs)
}

#[cfg(not(debug_assertions))]
fn test_seconds(_name: &str) -> Option<Duration> {
    None
}

/// How long a minute of the limits lasts: 60 seconds, or, in a test build, what `WB_TEST_MINUTE` says.
pub fn minute() -> Duration {
    test_seconds("WB_TEST_MINUTE").unwrap_or(Duration::from_secs(60))
}

/// The grace after the termination: 10 seconds, or, in a test build, what `WB_TEST_GRACE` says.
pub fn grace() -> Duration {
    test_seconds("WB_TEST_GRACE").unwrap_or(GRACE)
}

/// What a look at a view found: the commit its `HEAD` names, and the newest time and the number of the entries under
/// the paths of its list.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Seen {
    head: Option<String>,
    newest: Option<SystemTime>,
    count: usize,
}

/// A look at the view, or none where its entries were not read within `LOOK_DEADLINE`.
fn seen(view: &Path, paths: &[String]) -> Option<Seen> {
    let head = crate::view::head_of(&view.join(".git")).ok();
    let (newest, count) = files_of(view, paths, Instant::now() + LOOK_DEADLINE)?;
    Some(Seen { head, newest, count })
}

/// `O_DIRECTORY` and `O_NOFOLLOW` of Linux on the architectures that `crate::manifest` allows: the standard library
/// has no name for them. A folder opened with both is a folder, and its last part no link.
#[cfg(any(target_arch = "aarch64", target_arch = "powerpc64"))]
const O_DIRECTORY_NOFOLLOW: i32 = 0o40000 | 0o100000;
#[cfg(not(any(target_arch = "aarch64", target_arch = "powerpc64")))]
const O_DIRECTORY_NOFOLLOW: i32 = 0o200000 | 0o400000;

/// The folder at `path`, opened without following a link in its last part and without waiting; none where it is
/// not a folder.
fn open_folder(path: &Path) -> Option<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new().read(true).custom_flags(O_DIRECTORY_NOFOLLOW | crate::manifest::O_NONBLOCK).open(path).ok()
}

/// The path through which what `folder` holds is reached, whatever its own path has become: each name below it is
/// looked up in that folder itself, and no link on the way to it is followed again.
fn through(folder: &fs::File) -> PathBuf {
    use std::os::unix::io::AsRawFd;
    PathBuf::from(format!("/proc/self/fd/{}", folder.as_raw_fd()))
}

/// A folder found under a path of the list, waiting to be read: the folder it is in (none for a path of the list),
/// its name (the path of the list, for one), and its device and inode as it was found.
struct Found {
    parent: Option<usize>,
    name: std::ffi::OsString,
    place: (u64, u64),
}

/// The most bytes of a path that a folder waiting to be read is opened by, as Linux takes it (`PATH_MAX`).
const PATH_MAX: usize = 4096;

/// The newest time of change of the entries under `paths` in the folder `view`, and how many were read; none where
/// the look was not over by `until`.
///
/// Each entry is looked at by its metadata alone and never opened. No link is followed in any part of a path: each
/// part of a path of the list before the last is opened from the view's own folder down as a plain folder, through
/// the one before it, and the path is passed over where one is a link or no folder. A folder under it is opened
/// without following a link and read only where it is still the folder that was found (its device and inode), so
/// that one swapped for a link is passed over. A path that is absolute or has a part `..` is passed over. Every path
/// of the list is looked at first, then what is under the folders among them: at most `MOST_ENTRIES` entries are read
/// in all, an entry that cannot be read counted with them, so that what a look reads and holds is bounded.
pub(crate) fn files_of(view: &Path, paths: &[String], until: Instant) -> Option<(Option<SystemTime>, usize)> {
    use std::os::unix::fs::MetadataExt;
    let mut newest: Option<SystemTime> = None;
    let mut count = 0;
    let mut note = |meta: &fs::Metadata| {
        if let Ok(time) = meta.modified() {
            newest = Some(newest.map_or(time, |before: SystemTime| before.max(time)));
        }
    };
    let mut found: Vec<Found> = Vec::new();
    let mut waiting: Vec<usize> = Vec::new();
    let Some(root) = open_folder(view) else { return Some((None, 0)) };
    'paths: for path in paths {
        if count >= MOST_ENTRIES {
            break;
        }
        if Instant::now() >= until {
            return None;
        }
        let parts = crate::starter::parts(path);
        if path.starts_with('/') || parts.is_empty() || parts.contains(&"..") {
            continue;
        }
        let Some((last, before)) = parts.split_last() else { continue };
        let Ok(mut at) = root.try_clone() else { continue };
        for part in before {
            match open_folder(&through(&at).join(part)) {
                Some(next) => at = next,
                None => continue 'paths,
            }
        }
        count += 1;
        let Ok(meta) = fs::symlink_metadata(through(&at).join(last)) else { continue };
        note(&meta);
        if meta.file_type().is_dir() {
            found.push(Found { parent: None, name: parts.join("/").into(), place: (meta.dev(), meta.ino()) });
            waiting.push(found.len() - 1);
        }
    }
    while let Some(index) = waiting.pop() {
        if count >= MOST_ENTRIES {
            break;
        }
        if Instant::now() >= until {
            return None;
        }
        let Some(path) = path_of(view, &found, index) else { continue };
        let Some(folder) = open_folder(&path) else { continue };
        match folder.metadata() {
            Ok(meta) if (meta.dev(), meta.ino()) == found[index].place => {}
            _ => continue,
        }
        let at = through(&folder);
        let Ok(entries) = fs::read_dir(&at) else { continue };
        for entry in entries {
            if count >= MOST_ENTRIES {
                break;
            }
            if count % 256 == 0 && Instant::now() >= until {
                return None;
            }
            count += 1;
            let Ok(entry) = entry else { continue };
            let Ok(meta) = fs::symlink_metadata(at.join(entry.file_name())) else { continue };
            note(&meta);
            if meta.file_type().is_dir() {
                found.push(Found { parent: Some(index), name: entry.file_name(), place: (meta.dev(), meta.ino()) });
                waiting.push(found.len() - 1);
            }
        }
    }
    Some((newest, count))
}

/// The path of the folder `index` of `found` under `view`; none where it is longer than `PATH_MAX`.
fn path_of(view: &Path, found: &[Found], index: usize) -> Option<PathBuf> {
    let mut names = Vec::new();
    let mut length = view.as_os_str().len();
    let mut at = Some(index);
    while let Some(here) = at {
        length += found[here].name.len() + 1;
        if length > PATH_MAX {
            return None;
        }
        names.push(&found[here].name);
        at = found[here].parent;
    }
    Some(names.iter().rev().fold(view.to_path_buf(), |path, name| path.join(name)))
}

/// The newest time of change of the entries under `paths` in the folder `view`, as `wb status` reads it (#267): read
/// in a thread of its own, so that a read that waits does not hold `wb`, within `LOOK_DEADLINE`. The error is why not.
pub(crate) fn newest_within(view: &Path, paths: &[String]) -> Result<Option<SystemTime>, String> {
    let (view, paths) = (view.to_path_buf(), paths.to_vec());
    let (send, receive) = std::sync::mpsc::channel();
    let spawned = std::thread::Builder::new().name("look".to_string()).spawn(move || {
        let _ = send.send(files_of(&view, &paths, Instant::now() + LOOK_DEADLINE));
    });
    if spawned.is_err() {
        return Err("the entries of the view could not be read".to_string());
    }
    match receive.recv_timeout(LOOK_DEADLINE + Duration::from_secs(1)) {
        Ok(Some((newest, _))) => Ok(newest),
        _ => Err(format!("the entries of the view were not read within {} seconds", LOOK_DEADLINE.as_secs())),
    }
}

/// What a look found that `wb` says.
#[derive(Debug, PartialEq, Eq)]
pub enum Event {
    /// Nothing changed in the view for this many minutes.
    Quiet(u64),
    /// The session has run for `max_minutes`.
    Max,
}

/// The watchdog of one session.
pub struct Watchdog {
    limits: Limits,
    minute: Duration,
    began: Instant,
    view: PathBuf,
    paths: Vec<String>,
    /// The folder of the lines of the start, where the file `quiet` is written.
    lines: Option<PathBuf>,
    seen: Seen,
    /// The look that runs, in a thread of its own, where one does: what it finds comes on this; and when it began.
    looking: Option<(std::sync::mpsc::Receiver<Option<Seen>>, Instant)>,
    /// When a look last found the view changed, or the start.
    changed: Instant,
    /// The quiet, in minutes, at which a line is said next.
    next_quiet: u64,
    next_look: Instant,
    ended: bool,
}

impl Watchdog {
    /// The watchdog of a session in the view `view` with the list `paths`, before it starts: what the view holds now is
    /// where the quiet starts from.
    pub fn new(limits: Limits, view: &Path, paths: &[String], lines: Option<&Path>) -> Watchdog {
        let minute = minute();
        let now = Instant::now();
        Watchdog {
            limits,
            minute,
            began: now,
            view: view.to_path_buf(),
            paths: paths.to_vec(),
            lines: lines.map(Path::to_path_buf),
            seen: seen(view, paths).unwrap_or(Seen { head: None, newest: None, count: 0 }),
            looking: None,
            changed: now,
            next_quiet: limits.quiet_minutes,
            next_look: now,
            ended: false,
        }
    }

    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// Whether the session was ended at `max_minutes`.
    pub fn ended(&self) -> bool {
        self.ended
    }

    /// How long between two looks: 10 seconds, or a quarter of a minute where a minute is shorter (a test's).
    fn every(&self) -> Duration {
        LOOK_EVERY.min(self.minute / 4).max(Duration::from_millis(10))
    }

    /// Looks at the view, where it is time to: what is to be said, if anything. A quiet is written in the file `quiet`
    /// of the lines here. The look itself runs in a thread of its own, one at a time, so that `max_minutes` never waits
    /// for it: a look that has not ended is passed over, and one not over within `LOOK_DEADLINE` finds nothing. The
    /// quiet is weighed when a look has ended, or while one has not ended past `LOOK_DEADLINE`.
    pub fn look(&mut self) -> Option<Event> {
        use std::sync::mpsc::TryRecvError;
        let now = Instant::now();
        if self.ended {
            return None;
        }
        if now.duration_since(self.began) >= self.minute.saturating_mul(self.limits.max_minutes.min(u32::MAX as u64) as u32) {
            self.ended = true;
            return Some(Event::Max);
        }
        // The quiet is weighed when a look has ended, or where one has not ended past its deadline.
        let mut weigh = false;
        match self.looking.as_ref().map(|(looking, _)| looking.try_recv()) {
            Some(Ok(found)) => {
                self.looking = None;
                weigh = true;
                if let Some(found) = found.filter(|found| *found != self.seen) {
                    self.seen = found;
                    self.changed = now;
                    self.next_quiet = self.limits.quiet_minutes;
                }
            }
            Some(Err(TryRecvError::Disconnected)) => self.looking = None,
            Some(Err(TryRecvError::Empty)) => weigh = self.looking.as_ref().is_some_and(|(_, began)| now.duration_since(*began) > LOOK_DEADLINE),
            None if now >= self.next_look => {
                self.next_look = now + self.every();
                let (view, paths) = (self.view.clone(), self.paths.clone());
                let (send, receive) = std::sync::mpsc::channel();
                let spawned = std::thread::Builder::new().name("look".to_string()).spawn(move || {
                    let _ = send.send(seen(&view, &paths));
                });
                if spawned.is_ok() {
                    self.looking = Some((receive, now));
                }
            }
            None => {}
        }
        let quiet = now.duration_since(self.changed);
        if weigh && quiet >= self.minute.saturating_mul(self.next_quiet.min(u32::MAX as u64) as u32) {
            let said = self.next_quiet;
            self.next_quiet = self.next_quiet.saturating_mul(2);
            if let Some(folder) = &self.lines {
                // A file that cannot be written changes nothing: the line is said on standard error all the same.
                let _ = crate::sessions::append(folder, "quiet", &format!("quiet: {}\n", said));
            }
            return Some(Event::Quiet(said));
        }
        None
    }
}

/// A time of `seconds` as `wb status` says it (#267): `12m` under an hour, else `7h05m`.
pub fn shown(seconds: u64) -> String {
    let minutes = seconds / 60;
    if minutes < 60 {
        format!("{}m", minutes)
    } else {
        format!("{}h{:02}m", minutes / 60, minutes % 60)
    }
}
