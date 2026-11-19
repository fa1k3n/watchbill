//! The starter (#84): what `wb <role>` does after the checks of `wb prompt`. It reads the stamps on the issue,
//! finds the change of a branch, checks the class of the paths a session may change, posts the stamp and
//! starts the session with the runner's `start`.
//!
//! The patterns of `watchbill.yml` that ask the tracker and the git server (`issue_comment`, `issue_comments`,
//! `change_comment`, `change_comments`, `change_of_branch`) are the team's own commands. Each is one line for
//! `/bin/sh -c`, as an agent runs the addresses, run in the root of the checkout, with the values filled in
//! before: each is a plain name (`prompt::is_plain_name`), so no value carries shell into it. The runner's
//! `start` is a list of words, and is run with no shell.

use std::io::Write;
use std::process::{Command, Stdio};

use crate::yamlfile::{self, Part};

/// The roles whose stamps count the rounds of their own role. The fix counts the review's.
const ROUNDS_OF_THEIR_OWN: [&str; 4] = ["breakdown", "tester", "build", "review"];

/// The longest word, with its closing zero byte, that Linux hands a program (`MAX_ARG_STRLEN`).
pub const LONGEST_WORD: usize = 128 * 1024;

/// Runs one pattern, filled, as one line of `/bin/sh -c` in `root`, with `input` on standard input or
/// nothing. Standard error is the person's. The result is what it printed on standard output; a command
/// that cannot be run, or ends with anything but 0, is refused with the key of the pattern named.
pub fn run(root: &str, key: &str, line: &str, input: Option<&[u8]>) -> Result<Vec<u8>, String> {
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
    let text = String::from_utf8_lossy(&printed);
    let found = text
        .lines()
        .filter_map(|line| {
            let rest = line.trim_end_matches('\r').strip_prefix("[stamp] ")?;
            match rest.split(' ').collect::<Vec<_>>().as_slice() {
                [role, "round", n] if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) => Some((role.to_string(), n.parse().ok()?)),
                _ => None,
            }
        })
        .collect();
    Ok(Stamps { on: on.to_string(), key: key.to_string(), found })
}

/// The open changes that `change_of_branch`, filled as `line`, prints: one to a line, empty lines left out.
pub fn open_changes(root: &str, line: &str) -> Result<Vec<String>, String> {
    let printed = run(root, "change_of_branch", line, None)?;
    Ok(String::from_utf8_lossy(&printed).lines().map(str::trim).filter(|line| !line.is_empty()).map(str::to_string).collect())
}

/// A path of `classed` or of `changes`, as its parts: `.` and empty parts left out.
fn parts(path: &str) -> Vec<&str> {
    path.split('/').filter(|part| !part.is_empty() && *part != ".").collect()
}

/// The classed paths of `watchbill.yml` at `root` (#84): the list `classed`, or `None` when the file has none,
/// and then every path is classed. A list that is not of relative paths without `..` is refused with its place.
pub fn classed(root: &str) -> Result<Option<Vec<String>>, String> {
    let config = std::path::Path::new(root).join(crate::init::CONFIG_FOLDER).join(crate::init::CONFIG_FILE);
    let document = yamlfile::load(&config).map_err(|why| why.to_string())?;
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
fn meet(one: &str, other: &str) -> bool {
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
/// the session and does not end `wb` (`interrupt::hold`). The result is the session's status: 0 when it
/// ended with 0.
pub fn session(root: &str, words: &[String]) -> Result<bool, String> {
    let (program, rest) = words.split_first().ok_or_else(|| "`start` holds no word".to_string())?;
    let _held = crate::interrupt::hold();
    let status = Command::new(program)
        .args(rest)
        .current_dir(root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| format!("the session could not be started: {} could not be run: {}", program, error))?;
    Ok(status.success())
}
