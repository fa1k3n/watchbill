//! The starter (#84): what `wb start <role>` does after the checks of `wb prompt`. It reads the stamps on the issue,
//! finds the change of a branch, checks the class of the paths a session may change, posts the stamp and
//! starts the session with the runner's `start`.
//!
//! The patterns of `watchbill.yml` that ask the tracker and the git server (`issue_comment`, `issue_comments`,
//! `change_comment`, `change_comments`, `change_of_branch`) are the team's own commands. Each is one line for
//! `/bin/sh -c`, as an agent runs the addresses, run in the root of the checkout, with the values filled in
//! before: each is a plain name (`prompt::is_plain_name`), so no value carries shell into it. The runner's
//! `start` is a list of words, and is run with no shell.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::yamlfile::{self, Part};

/// The roles whose stamps count the rounds of their own role. The fix counts the review's.
const ROUNDS_OF_THEIR_OWN: [&str; 4] = ["planner", "tester", "build", "review"];

/// The longest word, with its closing zero byte, that Linux hands a program (`MAX_ARG_STRLEN`).
pub const LONGEST_WORD: usize = 128 * 1024;

thread_local! {
    /// Set by `read_only` for the command that only reads (#254): `held_bytes` then gives the file of `HEAD`.
    static READ_ONLY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// `place` as `HEAD` has it, read with no replace refs; `None` where it is not in `HEAD`.
fn head_file(root: impl AsRef<std::ffi::OsStr>, place: &str) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .arg("--no-replace-objects")
        .arg("-C")
        .arg(root)
        .arg("show")
        .arg(format!("HEAD:{}", place))
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

/// The refusal where `place` is over 1 MiB in `HEAD`, from its size and before the blob is read (#261): a session
/// can commit a big file, and `git show` would hold all of it.
fn head_too_big(root: &str, place: &str) -> Result<(), String> {
    match head_size(Path::new(root), place) {
        Some(size) if size > crate::bounded::TEXT_CAP => {
            Err(format!("{} is over 1 MiB in HEAD, which is too big. A person makes it smaller and commits it", place))
        }
        _ => Ok(()),
    }
}

/// The size of `place` in `HEAD`, from the object database with no replace refs, without reading the file.
fn head_size(root: &Path, place: &str) -> Option<u64> {
    let output = Command::new("git")
        .arg("--no-replace-objects")
        .arg("-C")
        .arg(root)
        .arg("cat-file")
        .arg("-s")
        .arg(format!("HEAD:{}", place))
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()?.trim().parse().ok()
}

/// The read-only mode of `read_only`, held for the life of the command: dropping it ends the mode, so that the
/// refusal of #186 holds again for whatever runs after it on the thread.
#[must_use = "the mode ends when the guard is dropped"]
pub struct ReadOnly(());

impl Drop for ReadOnly {
    fn drop(&mut self) {
        READ_ONLY.with(|flag| flag.set(false));
    }
}

/// Why `read_only` did not begin: what the command says, and how it ends. Not set up is 2; a file that is set up
/// and refused (over 1 MiB in `HEAD`) is 1, as a refusal is in every command.
#[derive(Debug)]
pub struct NotRead {
    pub status: i32,
    pub why: String,
}

/// For a command that only reads (#254: `wb status`, `wb view`). `Err` where `watchbill.yml` is not in `HEAD` at
/// all (the repository is not set up). Otherwise, while the guard is held, every pattern and key is read from the
/// file of `HEAD`, never the working tree's, and the result says whether the working tree's file differs from
/// it (a file missing there counts as differing): the command says so on standard error and goes on.
pub fn read_only(root: &str) -> Result<(ReadOnly, bool), NotRead> {
    let place = format!("{}/{}", crate::init::CONFIG_FOLDER, crate::init::CONFIG_FILE);
    head_too_big(root, &place).map_err(|why| NotRead { status: 1, why })?;
    let Some(head) = head_file(root, &place) else {
        if !Path::new(root).join(&place).is_file() {
            return Err(NotRead { status: 2, why: format!("{} is not there. Watchbill is not set up in this repository", place) });
        }
        return Err(NotRead {
            status: 2,
            why: format!("{} is not in HEAD. Watchbill is not set up in this repository: commit the file first", place),
        });
    };
    READ_ONLY.with(|flag| flag.set(true));
    Ok((ReadOnly(()), working_tree_differs(&Path::new(root).join(&place), &head)))
}

/// Whether the working tree's file is not the file of `HEAD`. What is not a plain file (a FIFO, a device, a folder, a
/// symlink) or is over 1 MiB counts as differing and is never opened: reading it could wait or grow without end.
fn working_tree_differs(path: &Path, head: &[u8]) -> bool {
    match crate::bounded::read_plain(path, crate::bounded::TEXT_CAP) {
        Ok(now) => now != head,
        Err(_) => true,
    }
}

/// A file of the project's text (#211) as `HEAD` has it: `Ok(Some(bytes))` where the working tree's file is the
/// file of `HEAD`, `Ok(None)` where it is in neither, and otherwise the refusal, which names the file: it differs,
/// it is in the working tree and not in `HEAD`, or it is in `HEAD` and not in the working tree. The working tree's
/// file is compared by `working_tree_differs`, so a FIFO, a device, a folder, a link or a file over the cap counts
/// as differing and is never opened. The caller has seen that no folder on the way is a link.
pub fn project_text(root: &Path, place: &str) -> Result<Option<Vec<u8>>, String> {
    // A file of `HEAD` over the cap is refused by its size, before it is read whole.
    if head_size(root, place).is_some_and(|size| size > 1024 * 1024) {
        return Err(format!("{} is over 1 MiB in HEAD, which is too big for a part of a prompt. A person makes it smaller and commits it", place));
    }
    let head = head_file(root, place);
    let path = root.join(place);
    let there = !matches!(std::fs::symlink_metadata(&path), Err(error) if error.kind() == std::io::ErrorKind::NotFound);
    let said = match (&head, there) {
        (None, false) => return Ok(None),
        (None, true) => "is not in HEAD",
        (Some(_), false) => "is in HEAD and not in the working tree",
        (Some(bytes), true) if working_tree_differs(&path, bytes) => "differs from HEAD",
        (Some(bytes), true) => return Ok(Some(bytes.clone())),
    };
    Err(format!(
        "{} {}, and a session is told what HEAD holds: a person reads the change and commits it, or takes it back with git, before the next command. Nothing is handed out",
        place, said
    ))
}

/// The line that a command that only reads says where `read_only` found a difference.
pub fn drift_line(name: &str) -> String {
    format!(
        "{}: {}/{} differs from HEAD. It goes on with the file as HEAD has it: a pattern changed only in the working tree is not run.",
        name,
        crate::init::CONFIG_FOLDER,
        crate::init::CONFIG_FILE
    )
}

/// The limit of #186: `watchbill.yml` at `root` is held as `HEAD` has it. A pattern of the file is run as the
/// person, so a file that a session changed and nobody committed is never run. `Ok(Some(bytes))` where the file
/// in the working tree is the file of `HEAD` byte for byte: the bytes are those of `HEAD`, and every pattern is
/// parsed from them, so the file is not read again (`settings`). `Ok(None)` where there is no file in the working
/// tree (the commands say that themselves); otherwise the refusal, which names the file. `HEAD` is read with no
/// replace refs, which a session can write with nothing on the branch to show it. No flag lifts it.
pub fn held_bytes(root: &str) -> Result<Option<Vec<u8>>, String> {
    let place = format!("{}/{}", crate::init::CONFIG_FOLDER, crate::init::CONFIG_FILE);
    head_too_big(root, &place)?;
    // A command that only reads (`read_only`) goes on with the file of `HEAD`, before the working tree is looked at.
    if READ_ONLY.with(|flag| flag.get()) {
        // In this mode the working tree is never the fallback: `HEAD`'s file was there when the mode began.
        return match head_file(root, &place) {
            Some(bytes) => Ok(Some(bytes)),
            None => Err(format!("{} could not be read from HEAD. Nothing is run and nothing is posted", place)),
        };
    }
    // The file is read by the one reader (#261): what is not a plain file, or is over the cap, is never opened.
    let now = match crate::bounded::read_plain(&Path::new(root).join(&place), crate::bounded::TEXT_CAP) {
        Ok(now) => Some(now),
        Err(crate::bounded::Fault::NotPlain | crate::bounded::Fault::TooBig(_)) => None,
        Err(_) => return Ok(None),
    };
    let said = match (head_file(root, &place), now) {
        (Some(bytes), Some(now)) if bytes == now => return Ok(Some(bytes)),
        (Some(_), _) => "differs from HEAD",
        (None, Some(_)) => "is not in HEAD",
        (None, None) => "is not a plain file of 1 MiB or less",
    };
    Err(format!(
        "{} {}, and a pattern of it is run as the person: a person reads the change and commits it, or takes it back with git, before the next command. Nothing is run and nothing is posted",
        place, said
    ))
}

/// As `held_bytes`, for the commands that read no pattern from the file themselves.
pub fn held(root: &str) -> Result<(), String> {
    held_bytes(root).map(|_| ())
}

/// `watchbill.yml` at `root`, read through the one loader from the bytes of `HEAD` that `held_bytes` read: the
/// file is held (refused where it differs) and read once. Where there is no file in the working tree, it is read
/// from there as before, and the loader says why it cannot be.
pub fn settings(root: &str) -> Result<yamlfile::Document, String> {
    let path = Path::new(root).join(crate::init::CONFIG_FOLDER).join(crate::init::CONFIG_FILE);
    match held_bytes(root)? {
        Some(bytes) => yamlfile::load_bytes(&path, bytes).map_err(|why| why.to_string()),
        None => yamlfile::load(&path).map_err(|why| why.to_string()),
    }
}

/// Runs one pattern, filled, as one line of `/bin/sh -c` in `root`, with `input` on standard input or
/// nothing. Standard error is the person's. The result is what it printed on standard output; a command
/// that cannot be run, or ends with anything but 0, is refused with the key of the pattern named.
pub fn run(root: &str, key: &str, line: &str, input: Option<&[u8]>) -> Result<Vec<u8>, String> {
    held(root)?;
    let mut command = Command::new("/bin/sh");
    command.arg("-c").arg(line).current_dir(root).stdout(Stdio::piped()).stderr(Stdio::inherit());
    command.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() });
    let mut child = command.spawn().map_err(|error| format!("`{}` of watchbill.yml could not be run: {}", key, error))?;
    if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
        // A command that does not read all of it still ends as it ends; its status says what happened.
        let _ = stdin.write_all(input);
    }
    let output = child.wait_with_output().map_err(|error| format!("`{}` of watchbill.yml could not be waited for: {}", key, error))?;
    match output.status.code() {
        Some(0) => Ok(output.stdout),
        Some(code) => Err(format!("`{}` of watchbill.yml ended with {}", key, code)),
        None => Err(format!("`{}` of watchbill.yml was ended by a signal", key)),
    }
}

/// Runs one pattern as `run` does, with nothing on standard input, and reads what it prints through `cap` (#295):
/// a pattern that prints more is ended, and refused with its key named.
pub fn run_capped(root: &str, key: &str, line: &str, cap: u64) -> Result<Vec<u8>, String> {
    use std::io::Read;
    held(root)?;
    let mut command = Command::new("/bin/sh");
    command.arg("-c").arg(line).current_dir(root).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::inherit());
    let mut child = command.spawn().map_err(|error| format!("`{}` of watchbill.yml could not be run: {}", key, error))?;
    let mut printed = Vec::new();
    let read = match child.stdout.take() {
        Some(out) => out.take(cap + 1).read_to_end(&mut printed).map(|_| ()),
        None => Ok(()),
    };
    if printed.len() as u64 > cap {
        // The pipe is closed with `take`: what the command starts after this gets no more room to print.
        let _ = child.kill();
        let _ = child.wait();
        let most = if cap % (1024 * 1024) == 0 { format!("{} MiB", cap / (1024 * 1024)) } else { format!("{} bytes", cap) };
        return Err(format!("`{}` of watchbill.yml printed more than {}, and no more of it is read", key, most));
    }
    let status = child.wait().map_err(|error| format!("`{}` of watchbill.yml could not be waited for: {}", key, error))?;
    read.map_err(|error| format!("what `{}` of watchbill.yml printed could not be read: {}", key, error))?;
    match status.code() {
        Some(0) => Ok(printed),
        Some(code) => Err(format!("`{}` of watchbill.yml ended with {}", key, code)),
        None => Err(format!("`{}` of watchbill.yml was ended by a signal", key)),
    }
}

/// The stamps on an issue or a change: each `[stamp] <role> round <n>` line that a pattern printed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stamps {
    /// Where they were read: `issue 54` or `change 174`.
    pub on: String,
    /// The key of the pattern that printed them.
    pub key: String,
    /// The role and the round of each stamp, in the order they were printed.
    pub found: Vec<(String, u64)>,
}

impl Stamps {
    /// How many stamps of a role there are.
    pub fn count(&self, role: &str) -> usize {
        self.found.iter().filter(|(name, _)| name == role).count()
    }

    /// The round of a session of `role` (#84): one more than the stamps of its role, and for the fix the
    /// number of review stamps, as it answers the last review. A fix with no review before it is refused.
    /// The second value says what was worked out, and from what.
    pub fn round_of(&self, role: &str) -> Result<(usize, String), String> {
        let counted = if ROUNDS_OF_THEIR_OWN.contains(&role) { role } else { "review" };
        let found = self.count(counted);
        let stamps = match found {
            0 => format!("no {} stamp", counted),
            1 => format!("1 {} stamp", counted),
            n => format!("{} {} stamps", n, counted),
        };
        if counted == role {
            return Ok((found + 1, format!("round {} ({} on {})", found + 1, stamps, self.on)));
        }
        if found == 0 {
            return Err(no_review(&self.on, &self.key));
        }
        Ok((found, format!("round {} ({} on {})", found, stamps, self.on)))
    }
}

/// Why a fix is refused when there is no review stamp: the rule table's order, a fix follows a review.
pub fn no_review(on: &str, key: &str) -> String {
    format!(
        "a fix follows a review (the rule table: the fix answers the findings of a review), and there is no review stamp on {} (from `{}` of watchbill.yml). The fix has nothing to answer",
        on, key
    )
}

/// Reads the stamps that the pattern `key`, filled as `line`, prints for `on`: one comment's first line to a
/// line. Only the lines `[stamp] <role> round <n>` are read; every stamp is posted by `wb`, so their form is
/// known, and a line of another form is not a stamp.
pub fn read_stamps(root: &str, key: &str, line: &str, on: &str) -> Result<Stamps, String> {
    let printed = run(root, key, line, None)?;
    let found = stamps_in(&String::from_utf8_lossy(&printed));
    Ok(Stamps { on: on.to_string(), key: key.to_string(), found })
}

/// The role and the round of a line `<start><role> round <n>`, where `start` is `[stamp] ` or `[revert] `.
fn role_and_round(start: &str, line: &str) -> Option<(String, u64)> {
    let rest = line.trim_end_matches('\r').strip_prefix(start)?;
    match rest.split(' ').collect::<Vec<_>>().as_slice() {
        [role, "round", n] if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) => Some((role.to_string(), n.parse().ok()?)),
        _ => None,
    }
}

/// Which of the lines of `text` are taken back (#142), one flag to a line: see `counted`. A marker is not
/// taken back itself in this list: it is what took the others back. A `[revert] planner` counts for nothing:
/// the planner is taken back with git, and `wb revert` refuses it, so one posted by hand is not read.
pub fn taken_back(text: &str) -> Vec<bool> {
    let lines: Vec<&str> = text.lines().map(|line| line.trim_end_matches('\r')).collect();
    let mut kept: Vec<usize> = Vec::new();
    let mut gone = vec![false; lines.len()];
    for (at, line) in lines.iter().enumerate() {
        match role_and_round("[revert] ", line) {
            Some((role, n)) if role != "planner" => {
                if let Some(held) = kept.iter().rposition(|held| role_and_round("[stamp] ", lines[*held]) == Some((role.clone(), n))) {
                    for dropped in kept.drain(held..) {
                        gone[dropped] = true;
                    }
                }
            }
            _ => kept.push(at),
        }
    }
    gone
}

/// The first lines of comments that still count (#142), one to a line. A line `[revert] <role> round <n>` takes
/// back the last `[stamp] <role> round <n>` before it and every line after that stamp, up to the marker: stamps,
/// questions and answers, whatever their role. The marker goes too. A marker with no such stamp before it counts
/// for nothing. A marker is read only as the whole of a first line, as a stamp is. A marker of the planner is
/// not read (`taken_back`).
pub fn counted(text: &str) -> Vec<String> {
    let gone = taken_back(text);
    text.lines()
        .map(|line| line.trim_end_matches('\r'))
        .zip(gone)
        .filter(|(line, gone)| !gone && role_and_round("[revert] ", line).map_or(true, |(role, _)| role == "planner"))
        .map(|(line, _)| line.to_string())
        .collect()
}

/// What the CI of a change says (#127): the word that `change_checks` prints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ci {
    Green,
    Red,
    Pending,
}

impl Ci {
    /// The word as `change_checks` prints it.
    pub fn word(self) -> &'static str {
        match self {
            Ci::Green => "green",
            Ci::Red => "red",
            Ci::Pending => "pending",
        }
    }

    /// Why a review is not started on a change with this CI, or none for green.
    pub fn refusal(self, change: &str) -> Option<String> {
        match self {
            Ci::Green => None,
            Ci::Pending => Some(format!("the CI of change {} is pending: wait for it to end, then start the review again.", change)),
            Ci::Red => Some(format!(
                "the CI of change {} is red, so no review is started: the change goes to a person. The CI builds the whole tree and runs every test after a change built in a view, so the lists or the stubs of the planner may be wrong.",
                change
            )),
        }
    }
}

/// Reads the CI of `change` by `change_checks` of `watchbill.yml` at `root`, run for the change: `None` when the
/// file has no `change_checks`. What it prints must be one word, `green`, `red` or `pending`, around which space
/// and a line end are left out; any other text, or a status but 0, is refused with the key named.
pub fn ci(root: &str, change: Option<&str>) -> Result<Option<Ci>, String> {
    let Some(line) = crate::prompt::pattern_of(root, "change_checks").map_err(|why| why.to_string())? else {
        return Ok(None);
    };
    let Some(change) = change.filter(|change| crate::prompt::is_plain_name(change)) else {
        return Err("the CI is read by `change_checks` of watchbill.yml for a change, and there is none. Give --change".to_string());
    };
    let printed = run(root, "change_checks", &line.replace("{change}", change), None)?;
    let printed = String::from_utf8_lossy(&printed);
    match printed.trim() {
        "green" => Ok(Some(Ci::Green)),
        "red" => Ok(Some(Ci::Red)),
        "pending" => Ok(Some(Ci::Pending)),
        other => {
            let shown: String = other.chars().take(80).collect();
            Err(format!("`change_checks` of watchbill.yml printed \"{}\", and it must print one word: green, red or pending", shown.escape_debug()))
        }
    }
}

/// The stamps in the first lines of comments, one to a line, that still count (`counted`): the role and the
/// round of each `[stamp] <role> round <n>`.
pub fn stamps_in(text: &str) -> Vec<(String, u64)> {
    counted(text).iter().filter_map(|line| role_and_round("[stamp] ", line)).collect()
}

/// The open changes that `change_of_branch`, filled as `line`, prints: one to a line, empty lines left out.
pub fn open_changes(root: &str, line: &str) -> Result<Vec<String>, String> {
    let printed = run(root, "change_of_branch", line, None)?;
    Ok(String::from_utf8_lossy(&printed).lines().map(str::trim).filter(|line| !line.is_empty()).map(str::to_string).collect())
}

/// A path of `classed` or of `changes`, as its parts: `.` and empty parts left out.
pub(crate) fn parts(path: &str) -> Vec<&str> {
    path.split('/').filter(|part| !part.is_empty() && *part != ".").collect()
}

/// The classed paths of `watchbill.yml` at `root` (#84): the list `classed`, or `None` when the file has none,
/// and then every path is classed. A list that is not of relative paths without `..` is refused with its place.
pub fn classed(root: &str) -> Result<Option<Vec<String>>, String> {
    let document = settings(root)?;
    let place = [Part::from("classed")];
    if !document.has(&place) {
        return Ok(None);
    }
    let paths = document.texts(&place, true).map_err(|why| why.to_string())?;
    for (index, path) in paths.iter().enumerate() {
        if path.starts_with('/') || parts(path).is_empty() || parts(path).contains(&"..") {
            let at = [Part::from("classed"), Part::from(index)];
            return Err(document
                .fail(&at, "a classed path is a file or a folder from the root of the repository, not `/` at its start, with no `..`, and not the root", false)
                .to_string());
        }
    }
    Ok(Some(paths))
}

/// Whether two paths meet: one is the other, or lies below it.
pub(crate) fn meet(one: &str, other: &str) -> bool {
    let (one, other) = (parts(one), parts(other));
    let shorter = one.len().min(other.len());
    one[..shorter] == other[..shorter]
}

/// Refuses classed code (#84): a path of `changes` that meets a classed path, or, where `changes` is not known
/// (no issue's file), any classed path at all. With no `classed` in `watchbill.yml`, every path is classed:
/// where a class is not given, the most restrictive applies. A session sees the whole checkout until #65.
pub fn class(root: &str, changes: Option<&[String]>) -> Result<(), String> {
    let classed = classed(root)?;
    let until = "Classed code is not worked on by a session started here until a session sees only its files (#65)";
    match (changes, classed) {
        (None, Some(list)) if list.is_empty() => Ok(()),
        (None, None) => Err(format!(
            "watchbill.yml says no `classed`, so every path is classed, and with no issue's file the paths the session may change are not known. {}. Write the classed paths as `classed: [<paths>]`, or `classed: []` when no code is classed.",
            until
        )),
        (None, Some(list)) => Err(format!(
            "with no issue's file the paths the session may change are not known, and `classed` of watchbill.yml names {}. {}. The role is started here only with an issue's file, or with `classed: []`.",
            list.join(", "),
            until
        )),
        (Some(changes), None) => match changes.first() {
            None => Ok(()),
            Some(_) => Err(format!(
                "watchbill.yml says no `classed`, so every path is classed, and the issue's `changes` hold {}. {}. Write the classed paths as `classed: [<paths>]`, or `classed: []` when no code is classed.",
                changes.join(", "),
                until
            )),
        },
        (Some(changes), Some(list)) => {
            let hit: Vec<&str> = changes.iter().filter(|path| list.iter().any(|classed| meet(path, classed))).map(String::as_str).collect();
            if hit.is_empty() {
                return Ok(());
            }
            Err(format!("the issue's `changes` hold classed code: {} (`classed` of watchbill.yml: {}). {}.", hit.join(", "), list.join(", "), until))
        }
    }
}

/// The words of `start` with `{model}`, `{effort}` and `{prompt}` filled, in one pass over each word: what
/// is put in is not read again.
pub fn fill(words: &[String], model: &str, effort: &str, prompt: &str) -> Vec<String> {
    words
        .iter()
        .map(|word| {
            let mut out = String::new();
            let mut rest = word.as_str();
            while let Some(open) = rest.find('{') {
                out.push_str(&rest[..open]);
                let after = &rest[open..];
                let value = [("{model}", model), ("{effort}", effort), ("{prompt}", prompt)]
                    .into_iter()
                    .find(|(name, _)| after.starts_with(name) && !out.ends_with('$'));
                match value {
                    Some((name, value)) => {
                        out.push_str(value);
                        rest = &after[name.len()..];
                    }
                    None => {
                        out.push('{');
                        rest = &after[1..];
                    }
                }
            }
            out.push_str(rest);
            out
        })
        .collect()
}

/// Starts the session: the words of `start`, filled, with no shell, in `root`, in the foreground, with the
/// person's standard input, output and error. While it runs, an interrupt or a quit at the terminal goes to
/// the session and does not end `wb` (`interrupt::hold`). The result is the session's status.
pub fn session(root: &str, words: &[String]) -> Result<std::process::ExitStatus, String> {
    let (program, rest) = words.split_first().ok_or_else(|| "`start` holds no word".to_string())?;
    let held = crate::interrupt::hold();
    let mut child = Command::new(program)
        .args(rest)
        .current_dir(root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("the session could not be started: {} could not be run: {}", program, error))?;
    held.session(child.id());
    let status = child.wait();
    held.ended();
    status.map_err(|error| format!("the session could not be waited for: {}", error))
}
