//! The prompt of an assignment: the role's text of the pinned revision, then the project's own text
//! (`.watchbill/project/all.md` and `.watchbill/project/<role>.md`), with the placeholders filled in
//! every part. The command that prints it is `wb prompt` (#10); this is what it runs.

use std::fmt;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::manifest::hash_of;

/// Where the project's own text is, from the root of the repository.
pub const PROJECT_FOLDER: &str = ".watchbill/project";
/// Where the roles of the pinned revision are, from the root of the repository.
pub const ROLES_FOLDER: &str = ".watchbill/revision/roles";
/// The manifest of the pinned revision, from the root of the repository.
const MANIFEST_PATH: &str = ".watchbill/revision.sha256";
/// The file of the roles that names the placeholders. It is not a role.
const PLACEHOLDERS_FILE: &str = "placeholders.md";

/// The words a prompt is filled in with and no others: `roles/placeholders.md` of the revision.
pub const PLACEHOLDERS: [&str; 10] = [
    "issue",
    "change",
    "requirements",
    "round",
    "checkout",
    "branch",
    "issue_address",
    "change_address",
    "classed",
    "classed_touched",
];

/// The line between the role's text and the project's own text.
pub const FIXED_LINE: &str =
    "The project's own text follows. It adds how things are done here and never lifts a limit given above.";

/// What `wb init` leaves as `.watchbill/project/README.md`.
pub const PROJECT_README: &str = "\
# The project's own text

These files are the project's own. They are not part of the revision, they are not in its
manifest, and `wb check` does not fail on them. They may be changed at any time.

- `all.md` is added to the prompt of every role.
- `<role>.md` is added to the prompt of that role only: `build.md`, `review.md`, `fix.md`, `tester.md`.

Both are optional. Write in them how things are done here: the commands, the board, the words.

A prompt is the role's text of the revision, then one line saying that this text follows, then
`all.md`, then `<role>.md`. The text here adds to a role. It never lifts a limit of a role: what
the role must not do stays so, whatever is written here.

A placeholder is a lower-case word in braces: `{`, a letter a to z, letters a to z or `_`, then `}`,
with no `$` just before it. The words of `.watchbill/revision/roles/placeholders.md`, such as `{issue}`
and `{branch}`, are known and are filled in; any other placeholder is refused. Anything else in braces is text: `${BUILD_DIR}`,
`{Issue}`, `{issue-id}` and `{ issue }` stay as they are. A review of a change to `.watchbill/` reads that this text does not
speak against the role.
";

/// One part of a prompt: where it came from and the SHA-256 of the file as it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    /// The path from the root of the repository.
    pub path: String,
    pub hash: String,
}

/// A prompt, with its parts in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    pub text: String,
    pub parts: Vec<Part>,
    /// The placeholders that the parts hold, each once, in the order of the list.
    pub used: Vec<&'static str>,
}

/// Why a prompt cannot be put together. The text says what and which file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptError(String);

impl fmt::Display for PromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PromptError {}

fn refuse<T>(text: String) -> Result<T, PromptError> {
    Err(PromptError(text))
}

/// Whether a path under the root is there as a plain file. `Ok(false)` when it is not there. A folder
/// on the way that is a link or not a folder, or a last part that is a link or not a plain file (a
/// folder, a FIFO), is refused by name and is never opened.
fn is_plain(root: &Path, path: &str) -> Result<bool, PromptError> {
    let names: Vec<&str> = path.split('/').collect();
    let mut at = root.to_path_buf();
    let mut shown = String::new();
    for (n, name) in names.iter().enumerate() {
        at.push(name);
        if !shown.is_empty() {
            shown.push('/');
        }
        shown.push_str(name);
        let last = n + 1 == names.len();
        match fs::symlink_metadata(&at) {
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
            Err(error) => return refuse(format!("{} cannot be read: {}", shown, error)),
            Ok(meta) if last && meta.is_file() => {}
            Ok(meta) if !last && meta.is_dir() => {}
            Ok(_) if last => return refuse(format!("{} is a link or not a plain file, and is not read", shown)),
            Ok(_) => return refuse(format!("{} is a link or not a folder, and is not read", shown)),
        }
    }
    Ok(true)
}

/// The roles of the pinned revision, from its manifest, sorted, each with the hash the manifest gives.
fn roles(root: &Path) -> Result<Vec<(String, String)>, PromptError> {
    let manifest = read_part(root, MANIFEST_PATH.to_string())?
        .ok_or_else(|| PromptError(format!("{} is not there", MANIFEST_PATH)))?;
    let mut found = Vec::new();
    for line in manifest.text.lines() {
        if let Some((hash, path)) = line.split_once("  ") {
            let role = path.strip_prefix("revision/roles/").and_then(|name| name.strip_suffix(".md"));
            if let Some(role) = role.filter(|role| !role.contains('/') && !role.is_empty()) {
                if format!("{}.md", role) != PLACEHOLDERS_FILE {
                    found.push((role.to_string(), hash.to_string()));
                }
            }
        }
    }
    found.sort();
    Ok(found)
}

/// The placeholders in a text: for each, where it starts, where it ends and its name. A placeholder
/// is a lower-case word in braces, as `{issue}`: `{`, a letter a to z, letters a to z or `_`, `}`,
/// with no `$` just before it. Any other brace is text.
fn placeholders(text: &str) -> Vec<(usize, usize, &str)> {
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(open) = text[at..].find('{').map(|n| n + at) {
        let rest = &text[open + 1..];
        let name_len = rest.find(|c: char| !(c.is_ascii_lowercase() || c == '_')).unwrap_or(rest.len());
        let name = &rest[..name_len];
        let named = name.starts_with(|c: char| c.is_ascii_lowercase());
        let dollar = text[..open].ends_with('$');
        if named && !dollar && rest[name_len..].starts_with('}') {
            found.push((open, open + 1 + name_len + 1, name));
            at = open + 1 + name_len + 1;
        } else {
            at = open + 1;
        }
    }
    found
}

/// The names of the placeholders in a text, in the order they stand, as `{issue}` is found in a prompt.
pub(crate) fn placeholder_names(text: &str) -> Vec<&str> {
    placeholders(text).into_iter().map(|(_, _, name)| name).collect()
}

/// The placeholders of the list that the prompt of `role` holds: in the role's text of the revision and in
/// the project's own text. A part that cannot be read gives none here; putting the prompt together names it.
pub fn used_by(root: &Path, role: &str) -> Vec<&'static str> {
    let mut texts = Vec::new();
    for path in [format!("{}/{}.md", ROLES_FOLDER, role), format!("{}/all.md", PROJECT_FOLDER), format!("{}/{}.md", PROJECT_FOLDER, role)] {
        if let Ok(Some(part)) = read_part(root, path) {
            texts.push(part.text);
        }
    }
    PLACEHOLDERS.iter().copied().filter(|name| texts.iter().any(|text| placeholder_names(text).contains(name))).collect()
}

/// A part as read: its path, its text as it is and its hash.
struct Read {
    path: String,
    text: String,
    hash: String,
}

fn read_part(root: &Path, path: String) -> Result<Option<Read>, PromptError> {
    if !is_plain(root, &path)? {
        return Ok(None);
    }
    let bytes = match fs::read(root.join(&path)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return refuse(format!("{} cannot be read: {}", path, error)),
    };
    match String::from_utf8(bytes) {
        Ok(text) => Ok(Some(Read { hash: hash_of(&text), path, text })),
        Err(_) => refuse(format!("{} is not UTF-8 text", path)),
    }
}

/// Puts the prompt of `role` together in the repository at `root`, with the given values for the
/// placeholders. A value is put in as it is given and is never itself read for placeholders.
/// It is refused when the role is not one of the revision, when a file of the project is not UTF-8,
/// when a part holds a placeholder that is not on the list, or one that got no value.
pub fn assemble(root: &Path, role: &str, values: &[(&str, &str)]) -> Result<Prompt, PromptError> {
    assemble_with(root, role, values, None)
}

/// What an issue's file adds to a prompt (#76): a part that stands after the role's text, with the path
/// and hash of the file it was made from. Its text is put in as it is and is never read for placeholders.
pub struct Extra<'a> {
    pub path: &'a str,
    pub hash: &'a str,
    pub text: &'a str,
}

/// As `assemble`, with the part of an issue's file after the role's text and before the project's.
pub fn assemble_with(
    root: &Path,
    role: &str,
    values: &[(&str, &str)],
    extra: Option<&Extra>,
) -> Result<Prompt, PromptError> {
    let have = roles(root)?;
    let expected = match have.iter().find(|(name, _)| name == role) {
        Some((_, hash)) => hash.clone(),
        None => {
            let names: Vec<&str> = have.iter().map(|(name, _)| name.as_str()).collect();
            return refuse(format!("the revision has no role '{}'. Its roles are: {}", role, names.join(", ")));
        }
    };
    let role_path = format!("{}/{}.md", ROLES_FOLDER, role);
    let first = match read_part(root, role_path.clone())? {
        Some(part) => part,
        None => return refuse(format!("{} cannot be read", role_path)),
    };
    if first.hash != expected {
        return refuse(format!(
            "{} is not the file of the manifest ({}): its hash is not the one on its line",
            role_path, MANIFEST_PATH
        ));
    }
    let mut parts = vec![first];
    for name in ["all", role] {
        if let Some(part) = read_part(root, format!("{}/{}.md", PROJECT_FOLDER, name))? {
            parts.push(part);
        }
    }
    // What is not on the list is refused in every part before anything is said of a missing value.
    for part in &parts {
        if let Some(why) = unknown_in(&part.path, &part.text).into_iter().next() {
            return refuse(why);
        }
    }
    for part in &parts {
        for (_, _, name) in placeholders(&part.text) {
            if !values.iter().any(|(given, _)| *given == name) {
                return refuse(format!("{}: the placeholder {{{}}} got no value", part.path, name));
            }
        }
    }

    let mut text = String::new();
    for (n, part) in parts.iter().enumerate() {
        if n == 1 {
            if !text.ends_with('\n') {
                text.push('\n');
            }
            text.push('\n');
            text.push_str(FIXED_LINE);
            text.push('\n');
        }
        if n > 0 {
            // A blank line before each part of the project's text.
            if !text.ends_with('\n') {
                text.push('\n');
            }
            text.push('\n');
        }
        text.push_str(&fill(&part.text, values));
        if let (0, Some(extra)) = (n, extra) {
            // The issue's part follows the role's text as it is, and is not read for placeholders.
            if !text.ends_with('\n') {
                text.push('\n');
            }
            text.push('\n');
            text.push_str(extra.text);
        }
    }
    let used = PLACEHOLDERS
        .iter()
        .copied()
        .filter(|name| parts.iter().any(|part| placeholders(&part.text).iter().any(|(_, _, found)| found == name)))
        .collect();
    let mut parts: Vec<Part> = parts.into_iter().map(|part| Part { path: part.path, hash: part.hash }).collect();
    if let Some(extra) = extra {
        parts.insert(1, Part { path: extra.path.to_string(), hash: extra.hash.to_string() });
    }
    Ok(Prompt { text, parts, used })
}

/// The placeholders of a text that the revision does not know, each as the words that refuse it, with
/// the file and the line. `wb prompt` and `wb check` both judge a text with this and no other.
pub fn unknown_in(path: &str, text: &str) -> Vec<String> {
    placeholders(text)
        .into_iter()
        .filter(|(_, _, name)| !PLACEHOLDERS.contains(name))
        .map(|(start, _, name)| {
            let line = text[..start].matches('\n').count() + 1;
            format!("{}: line {}: the placeholder {{{}}} is not known. The known ones are {}", path, line, name, known())
        })
        .collect()
}

/// What the project's own text holds that the revision does not know: `project/all.md` and
/// `project/<role>.md` for each role of the manifest, as `wb prompt` takes them, read the way it
/// reads them. A file that is not there is no difference; one that `wb prompt` would refuse (a link,
/// not a plain file, not UTF-8) is named as it names it. A link is never followed.
pub fn unknown_in_project(root: &Path) -> Vec<String> {
    let mut names = vec!["all".to_string()];
    if let Ok(found) = roles(root) {
        names.extend(found.into_iter().map(|(role, _)| role));
    }
    let mut found = Vec::new();
    for name in names {
        match read_part(root, format!("{}/{}.md", PROJECT_FOLDER, name)) {
            Ok(Some(part)) => found.extend(unknown_in(&part.path, &part.text)),
            Ok(None) => {}
            Err(why) => {
                // A link for the folder is the same line for each file: said once.
                let why = why.to_string();
                if !found.contains(&why) {
                    found.push(why);
                }
            }
        }
    }
    found
}

fn known() -> String {
    PLACEHOLDERS.iter().map(|name| format!("{{{}}}", name)).collect::<Vec<_>>().join(", ")
}

/// A text with each placeholder replaced by its value, in one pass over the text: what is put in is
/// not read again. A value is put in as it is given, but for `{classed}` and `{classed_touched}` (#227): where
/// one stands after spaces only, the lines of its value after the first start with the same spaces, so that a
/// list item stays one item.
fn fill(text: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::new();
    let mut at = 0;
    for (start, end, name) in placeholders(text) {
        out.push_str(&text[at..start]);
        let value = values.iter().find(|(given, _)| *given == name).map(|(_, value)| *value).unwrap_or("");
        let line_start = text[..start].rfind('\n').map(|n| n + 1).unwrap_or(0);
        let indent = &text[line_start..start];
        if matches!(name, "classed" | "classed_touched") && !indent.is_empty() && indent.chars().all(|c| c == ' ') {
            out.push_str(&value.replace('\n', &format!("\n{}", indent)));
        } else {
            out.push_str(value);
        }
        at = end;
    }
    out.push_str(&text[at..]);
    out
}

/// Refuses a role the revision does not have, naming the roles it has.
pub fn require_role(root: &Path, role: &str) -> Result<(), PromptError> {
    let have = roles(root)?;
    if have.iter().any(|(name, _)| name == role) {
        return Ok(());
    }
    let names: Vec<&str> = have.iter().map(|(name, _)| name.as_str()).collect();
    refuse(format!("the revision has no role '{}'. Its roles are: {}", role, names.join(", ")))
}

/// The text `requirements` has until the breakdown of an issue gives the IDs (#64).
fn default_requirements(issue: &str) -> String {
    format!("the numbered requirements of issue {}", issue)
}

/// The roles that may be handed out for a change with no issue (#116): its description is the spec.
pub const ALONE: [&str; 2] = ["review", "fix"];

/// What `{issue}` and `{issue_address}` are filled with in the text of a review or a fix of a change with
/// no issue (#116). The role's text says what to do where the issue is this. The stamp's `issue:` is empty.
pub const NO_ISSUE: &str = "none";

/// The text `requirements` has for a change with no issue (#116).
fn change_requirements(change: &str) -> String {
    format!("what the description of change {} says the change does", change)
}

/// The branch of a change with no issue (#116): the branch checked out, when it can be the change's.
/// `wb` asks nothing of the network, so it cannot see which branch the change is from. It refuses the
/// branch that `origin/HEAD` names, where changes go, and a HEAD on no branch; and where `origin/HEAD` is
/// not there it cannot tell the two apart, so it refuses too.
fn change_branch(root: &str, change: &str) -> Result<String, String> {
    let ask = |what: &str| -> Option<String> {
        let run = crate::detect::git_output(&["-C", root, "symbolic-ref", "-q", "--short", what], None)?;
        if !run.status.success() {
            return None;
        }
        String::from_utf8(run.stdout).ok().map(|text| text.trim_end_matches('\n').to_string())
    };
    let lead = format!("with no issue, the branch of change {} is the branch checked out", change);
    let Some(head) = ask("HEAD") else {
        return Err(format!("{}, and no branch is checked out. Give --branch", lead));
    };
    let Some(main) = ask("refs/remotes/origin/HEAD") else {
        return Err(format!(
            "{}, and origin/HEAD is not there, so {} cannot be told from the branch that changes go into. Give --branch",
            lead, head
        ));
    };
    if main.strip_prefix("origin/") == Some(head.as_str()) {
        return Err(format!("{}, and {} is the branch that origin/HEAD names, where changes go. Give --branch", lead, head));
    }
    Ok(head)
}

/// The names of the local branches, from the refs of the repository: nothing is asked of the network.
fn local_branches(root: &str) -> Result<Vec<String>, PromptError> {
    let run = crate::detect::git_output(&["-C", root, "for-each-ref", "--format=%(refname)", "refs/heads"], None);
    let Some(run) = run.filter(|run| run.status.success()) else {
        return refuse("the local branches cannot be read with git".to_string());
    };
    let Ok(text) = String::from_utf8(run.stdout) else {
        return refuse("a name of a local branch is not UTF-8 text".to_string());
    };
    Ok(text.lines().map(|line| line.strip_prefix("refs/heads/").unwrap_or(line).to_string()).collect())
}

/// Whether a value may fill a pattern: letters, digits, `.`, `_` and `-`, and as one name for git and
/// for `gh`: no `..`, not starting with `.` or `-`, not ending with `.` or `.lock`. The address is text
/// an agent runs, so a value never carries shell into it.
pub fn is_plain_name(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
        && !value.contains("..")
        && !value.starts_with(['.', '-'])
        && !value.ends_with('.')
        && !value.ends_with(".lock")
}

/// The patterns of `watchbill.yml` that `wb prompt` fills.
const PATTERNS: [&str; 3] = ["issue_address", "change_address", "branch"];

/// Refuses a `watchbill.yml` that still holds the words from before revision 0.0.6, when the issue was
/// called the task (#78): the key `task_address`, and `{task}` in a pattern. Each is named, with what to
/// write in its place. A pattern that is not a text is left to the loader, where it is used. The key
/// `breakdown`, from before revision 0.0.8, is refused by name too (#116).
fn old_words(document: &crate::yamlfile::Document) -> Result<(), PromptError> {
    let old = [crate::yamlfile::Part::from("task_address")];
    if document.has(&old) {
        let why = "the key `task_address` is from before revision 0.0.6, when the issue was called the task; write `issue_address` in its place";
        return refuse(document.fail(&old, why, true).to_string());
    }
    for key in PATTERNS {
        let place = [crate::yamlfile::Part::from(key)];
        if document.text(&place).map(|text| text.contains("{task}")).unwrap_or(false) {
            let why = "the pattern holds `{task}`, the word for the issue before revision 0.0.6; write `{issue}` in its place";
            return refuse(document.fail(&place, why, false).to_string());
        }
    }
    // `breakdown:` is from before revision 0.0.8, when a role needed the issue's file (#116).
    crate::issue::no_breakdown(document).map_err(PromptError)
}

/// Why a derivation is refused. `Used` is a value the caller gave that is not plain, named by its
/// option: the command was used wrongly (2). `Refused` is the rest (1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Derive {
    Used(String),
    Refused(PromptError),
}

impl From<PromptError> for Derive {
    fn from(why: PromptError) -> Derive {
        Derive::Refused(why)
    }
}

/// The values a prompt gets without being given them (#72), added to those given after them: a value
/// given is never replaced. From the repository root come `checkout` and, with the patterns of
/// `watchbill.yml` (read through the loader), the addresses and the branch. `requirements` has its
/// default text. The breakdown takes the branch from the pattern, as it makes the branch; any other
/// role takes the one local branch that starts with it and a `-`, or is the pattern. None, or more
/// than one, is refused with the names found. A review or a fix of a change with no issue (#116) takes
/// the branch checked out, when it can be the change's, and is used wrongly otherwise.
pub fn derive(
    root: &str,
    role: &str,
    given: &[(&'static str, String)],
) -> Result<Vec<(&'static str, String)>, Derive> {
    derive_with(root, role, given, Need::default()).map(|(values, _)| values)
}

/// What a derivation is to work out from the patterns that ask the tracker and the git server (#84).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Need {
    /// The round, from the stamps that `issue_comments` (or `change_comments`) prints.
    pub round: bool,
    /// The change, from what `change_of_branch` prints for the branch.
    pub change: bool,
    /// The classed paths and those the change touches (#227), from `watchbill.yml` and the diff of the branch.
    pub classed: bool,
    /// A pattern that is needed and not there is refused, where without it the value would only be missing.
    pub strict: bool,
}

/// What was worked out from the patterns: the lines that say it, and the stamps that were read.
#[derive(Clone, Debug, Default)]
pub struct Worked {
    pub said: Vec<String>,
    pub stamps: Option<crate::starter::Stamps>,
}

/// Why a value that fills a pattern is refused, named by its option.
pub fn not_plain(option: &str) -> String {
    format!(
        "the value of '--{}' fills a pattern, and is only letters, digits, '.', '_' and '-', with no '..', not starting with '.' or '-', not ending with '.' or '.lock'",
        option
    )
}

/// A pattern of `watchbill.yml` at `root`, read through the loader, or `None` when the file has none.
pub fn pattern_of(root: &str, key: &str) -> Result<Option<String>, PromptError> {
    let document = crate::starter::settings(root).map_err(PromptError)?;
    let place = [crate::yamlfile::Part::from(key)];
    if !document.has(&place) {
        return Ok(None);
    }
    document.text(&place).map(Some).map_err(|why| PromptError(why.to_string()))
}

/// As `derive`, and with what `need` asks worked out from the patterns of `watchbill.yml` that ask the
/// tracker and the git server (#84): the round, from the `[stamp]` lines on the issue, or on the change
/// when there is no issue; and the change, as the one open change from the branch. A value given is
/// never worked out. What was worked out is said in the lines of `Worked`.
pub fn derive_with(
    root: &str,
    role: &str,
    given: &[(&'static str, String)],
    need: Need,
) -> Result<(Vec<(&'static str, String)>, Worked), Derive> {
    let mut values = given.to_vec();
    // A value that fills a pattern must be plain, and is named by its option when it is not.
    let plain = |option: &str, value: &str| -> Result<(), Derive> {
        if is_plain_name(value) {
            return Ok(());
        }
        Err(Derive::Used(not_plain(option)))
    };
    let has = |values: &[(&'static str, String)], name: &str| values.iter().any(|(given, _)| *given == name);
    let get = |values: &[(&'static str, String)], name: &str| {
        values.iter().find(|(given, _)| *given == name).map(|(_, value)| value.clone())
    };
    // The file is read first, so that the words from before the issue are refused whatever is given (#78).
    let document = crate::starter::settings(root).map_err(PromptError)?;
    old_words(&document)?;
    // A pattern of `watchbill.yml`, `None` when the file has none.
    let pattern = |key: &str| -> Result<Option<String>, PromptError> {
        let place = [crate::yamlfile::Part::from(key)];
        if !document.has(&place) {
            return Ok(None);
        }
        document.text(&place).map(Some).map_err(|why| PromptError(why.to_string()))
    };
    let issue = get(&values, "issue");
    let change = get(&values, "change");
    if !has(&values, "checkout") {
        values.push(("checkout", root.to_string()));
    }
    // A review or a fix of a change with no issue takes the change's description as the spec (#116).
    let alone = match (&issue, &change) {
        (None, Some(change)) if ALONE.contains(&role) => Some(change.clone()),
        _ => None,
    };
    if !has(&values, "requirements") {
        if let Some(issue) = &issue {
            values.push(("requirements", default_requirements(issue)));
        } else if let Some(change) = &alone {
            values.push(("requirements", change_requirements(change)));
        }
    }
    if let (false, Some(change)) = (has(&values, "branch"), &alone) {
        values.push(("branch", change_branch(root, change).map_err(Derive::Used)?));
    }
    if let (false, Some(issue)) = (has(&values, "issue_address"), &issue) {
        if let Some(found) = pattern("issue_address")? {
            plain("issue", issue)?;
            values.push(("issue_address", found.replace("{issue}", issue)));
        }
    }
    if let (false, Some(change)) = (has(&values, "change_address"), &change) {
        if let Some(found) = pattern("change_address")? {
            plain("change", change)?;
            values.push(("change_address", found.replace("{change}", change)));
        }
    }
    if let (false, Some(issue)) = (has(&values, "branch"), &issue) {
        if let Some(found) = pattern("branch")? {
            plain("issue", issue)?;
            let start = found.replace("{issue}", issue);
            if role == "breakdown" {
                values.push(("branch", start));
            } else {
                let names: Vec<String> = local_branches(root)?
                    .into_iter()
                    .filter(|name| *name == start || name.starts_with(&format!("{}-", start)))
                    .collect();
                match names.as_slice() {
                    [one] => values.push(("branch", one.clone())),
                    [] => {
                        return Err(PromptError(format!(
                            "no local branch is named {} or starts with {}-. Give --branch",
                            start, start
                        ))
                        .into())
                    }
                    _ => {
                        return Err(PromptError(format!(
                            "more than one local branch is named {} or starts with {}-: {}. Give --branch",
                            start,
                            start,
                            names.join(", ")
                        ))
                        .into())
                    }
                }
            }
        }
    }
    let mut worked = Worked::default();
    // The round: from the stamps on the issue, or on the change when there is no issue (#84, #116).
    let on = match (&issue, &alone) {
        (Some(issue), _) => Some(("issue_comments", "issue", issue.clone())),
        (None, Some(change)) => Some(("change_comments", "change", change.clone())),
        _ => None,
    };
    if let (true, false, Some((key, word, value))) = (need.round, has(&values, "round"), &on) {
        match pattern(key)? {
            None if need.strict => {
                return Err(PromptError(format!(
                    "the round is worked out from the stamps that `{}` of watchbill.yml prints, and it has no `{}`. Add the line (see the README), or give --round",
                    key, key
                ))
                .into())
            }
            None => {}
            Some(found) => {
                plain(word, value)?;
                let stamps = crate::starter::read_stamps(root, key, &found.replace(&format!("{{{}}}", word), value), &format!("{} {}", word, value))
                    .map_err(PromptError)?;
                let (round, said) = stamps.round_of(role).map_err(PromptError)?;
                values.push(("round", round.to_string()));
                worked.said.push(said);
                worked.stamps = Some(stamps);
            }
        }
    }
    // The change: the one open change from the branch of the issue (#84). A change with no issue is given.
    if let (true, false, Some(_)) = (need.change, has(&values, "change"), &issue) {
        let branch = get(&values, "branch");
        match (pattern("change_of_branch")?, branch) {
            (None, _) if need.strict => {
                return Err(PromptError(
                    "the change is worked out by `change_of_branch` of watchbill.yml, and it has none. Add the line (see the README), or give --change".to_string(),
                )
                .into())
            }
            (Some(found), Some(branch)) => {
                if !is_plain_name(&branch) {
                    if given.iter().any(|(name, _)| *name == "branch") {
                        return Err(Derive::Used(not_plain("branch")));
                    }
                    return Err(PromptError(format!(
                        "the branch {} fills `change_of_branch`, and is not only letters, digits, '.', '_' and '-'. Give --change",
                        branch.escape_debug()
                    ))
                    .into());
                }
                let open = crate::starter::open_changes(root, &found.replace("{branch}", &branch)).map_err(PromptError)?;
                match open.as_slice() {
                    [one] if is_plain_name(one) => {
                        values.push(("change", one.clone()));
                        if let (false, Some(found)) = (has(&values, "change_address"), pattern("change_address")?) {
                            values.push(("change_address", found.replace("{change}", one)));
                        }
                    }
                    [one] => {
                        return Err(PromptError(format!(
                            "`change_of_branch` of watchbill.yml printed {} for the branch {}, which is not the name of a change. Give --change",
                            one.escape_debug(),
                            branch
                        ))
                        .into())
                    }
                    [] => {
                        return Err(PromptError(format!(
                            "no open change from the branch {}: `change_of_branch` of watchbill.yml printed none. Give --change",
                            branch
                        ))
                        .into())
                    }
                    more => {
                        return Err(PromptError(format!(
                            "more than one open change from the branch {}: {}. Give --change",
                            branch,
                            more.join(", ")
                        ))
                        .into())
                    }
                }
            }
            _ => {}
        }
    }
    if need.classed {
        let list = crate::starter::classed(root).map_err(PromptError)?;
        if !has(&values, "classed") {
            values.push(("classed", classed_text(&list)));
        }
        if !has(&values, "classed_touched") {
            let branch = get(&values, "branch");
            values.push(("classed_touched", touched_text(root, &list, branch.as_deref(), get(&values, "issue").as_deref())));
        }
    }
    Ok((values, worked))
}

/// What `{classed}` is filled with (#227): the paths one to a line, `none` for `classed: []`, and for a
/// `watchbill.yml` with no `classed` that every path is classed.
fn classed_text(list: &Option<Vec<String>>) -> String {
    match list {
        None => "every path (watchbill.yml has no `classed`)".to_string(),
        Some(paths) if paths.is_empty() => "none".to_string(),
        Some(paths) => paths.join("\n"),
    }
}

/// What `{classed_touched}` is filled with (#227): the paths of the change, from the diff of its branch against
/// the merge base with the default branch, that meet a classed path, one to a line, or `none`. With
/// `classed: []` nothing is asked of the repository. The issue's file and its acceptance are on every branch of
/// the issue by design, and are left out; those of another issue are not, and with no issue nothing is left out.
/// Where the diff cannot be had, the words say that and why.
pub fn touched_text(root: &str, list: &Option<Vec<String>>, branch: Option<&str>, issue: Option<&str>) -> String {
    if matches!(list, Some(paths) if paths.is_empty()) {
        return "none".to_string();
    }
    let told = |why: String| format!("cannot be told: {}", why);
    let Some(branch) = branch else {
        return told("the branch of the change is not known".to_string());
    };
    let Some(default) = crate::status::default_branch(root) else {
        return told("no default branch to compare with".to_string());
    };
    let git = |args: &[&str]| crate::status::git(root, args);
    let tip = [format!("refs/heads/{}", branch), format!("refs/remotes/origin/{}", branch)]
        .into_iter()
        .find(|full| git(&["rev-parse", "--verify", "--quiet", &format!("{}^{{commit}}", full)]).is_some());
    let Some(tip) = tip else {
        return told(format!("the branch {} is not there", branch));
    };
    let Some(base) = git(&["merge-base", &default, &tip]) else {
        return told(format!("the branch {} has no merge base with {}", branch, default));
    };
    let base = String::from_utf8_lossy(&base).trim().to_string();
    let Some(names) = git(&["diff", "--no-ext-diff", "--no-renames", "--name-only", "-z", &base, &tip]) else {
        return told("git gave no diff".to_string());
    };
    let changed: Vec<String> = names.split(|byte| *byte == 0).filter(|name| !name.is_empty()).map(|name| String::from_utf8_lossy(name).to_string()).collect();
    let own: Vec<String> = issue.map(|issue| vec![format!(".watchbill/issues/{}.yml", issue), format!(".watchbill/issues/{}.accepted", issue)]).unwrap_or_default();
    let hit: Vec<&str> = changed
        .iter()
        .filter(|path| !own.contains(path))
        .filter(|path| match list {
            None => true,
            Some(paths) => paths.iter().any(|classed| crate::starter::meet(path, classed)),
        })
        .map(String::as_str)
        .collect();
    if hit.is_empty() {
        "none".to_string()
    } else {
        hit.join("\n")
    }
}

/// The placeholders whose values a stamp prints, in the order the stamp has them.
pub const STAMPED: [&str; 5] = ["issue", "change", "round", "branch", "checkout"];

/// A time as `2026-11-17T18:04:05Z`, in UTC, from seconds since 1970-01-01.
pub fn utc(seconds: u64) -> String {
    let days = (seconds / 86400) as i64;
    let rest = seconds % 86400;
    // The civil date of a day count, as Howard Hinnant gives it.
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year,
        month,
        day,
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// What a stamp says besides the values of the placeholders.
pub struct Stamp<'a> {
    pub role: &'a str,
    pub model: &'a str,
    pub effort: &'a str,
    /// The runner named with `--runner`, or empty.
    pub runner: &'a str,
    pub version: &'a str,
    pub revision: &'a str,
    pub time: &'a str,
    /// The SHA-256 of the list of paths of the view the session runs in, or `none` (#126).
    pub view: &'a str,
    /// The level of the repository, for a session that runs in a view (#209): the line follows `view:`.
    pub level: Option<&'a str>,
    /// How many local changes `--ignore-local` let the start go on with (#172), or none: no line is written.
    pub local: Option<usize>,
    /// An issue's file is used and the option `--requirements` was given: it won over the file (#76).
    pub requirements_given: bool,
}

/// The stamp of an assignment (`docs/revision.md`, point 12): lines of `name: value` in a fixed order,
/// then one line `part: <sha256>  <path>` for each part, as `--parts` gives it. A value that is empty, or (but for the
/// round, #136) that the prompt does not hold, is written as the name and the colon alone. `runner` is empty when no runner is given (#93).
pub fn stamp(said: &Stamp, values: &[(&str, &str)], prompt: &Prompt) -> String {
    // A value the prompt does not hold was not handed out, and is written empty.
    // The round is a fact of the start, not only of the prompt: it is written where it is known (#136).
    let given = |name: &str| {
        if name != "round" && !prompt.used.contains(&name) {
            return "";
        }
        values.iter().find(|(given, _)| *given == name).map(|(_, value)| *value).unwrap_or("")
    };
    let mut lines: Vec<(&str, &str)> = vec![
        ("role", said.role),
        ("model", said.model),
        ("effort", said.effort),
        ("runner", said.runner),
        ("version", said.version),
        ("revision", said.revision),
    ];
    for name in &STAMPED[..3] {
        lines.push((name, given(name)));
    }
    lines.push(("branch", given("branch")));
    lines.push(("checkout", given("checkout")));
    lines.push(("time", said.time));
    lines.push(("view", said.view));
    let mut text = String::new();
    for (name, value) in lines {
        if value.is_empty() {
            text.push_str(&format!("{}:\n", name));
        } else {
            text.push_str(&format!("{}: {}\n", name, value));
        }
    }
    if let Some(level) = said.level {
        text.push_str(&format!("level: {}\n", level));
    }
    if let Some(count) = said.local {
        text.push_str(&format!("local: ignored {}\n", count));
    }
    if said.requirements_given {
        text.push_str("requirements: given\n");
    }
    for part in &prompt.parts {
        text.push_str(&format!("part: {}  {}\n", part.hash, part.path));
    }
    text
}
