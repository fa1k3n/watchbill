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
pub const PLACEHOLDERS: [&str; 8] =
    ["issue", "change", "requirements", "round", "checkout", "branch", "issue_address", "change_address"];

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
        for (_, _, name) in placeholders(&part.text) {
            if !PLACEHOLDERS.contains(&name) {
                return refuse(format!("{}: the placeholder {{{}}} is not known. The known ones are {}", part.path, name, known()));
            }
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

fn known() -> String {
    PLACEHOLDERS.iter().map(|name| format!("{{{}}}", name)).collect::<Vec<_>>().join(", ")
}

/// A text with each placeholder replaced by its value, in one pass over the text: what is put in is
/// not read again.
fn fill(text: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::new();
    let mut at = 0;
    for (start, end, name) in placeholders(text) {
        out.push_str(&text[at..start]);
        let value = values.iter().find(|(given, _)| *given == name).map(|(_, value)| *value).unwrap_or("");
        out.push_str(value);
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
/// write in its place. A pattern that is not a text is left to the loader, where it is used.
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
    Ok(())
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
/// than one, is refused with the names found.
pub fn derive(
    root: &str,
    role: &str,
    given: &[(&'static str, String)],
) -> Result<Vec<(&'static str, String)>, Derive> {
    let mut values = given.to_vec();
    // A value that fills a pattern must be plain, and is named by its option when it is not.
    let plain = |option: &str, value: &str| -> Result<(), Derive> {
        if is_plain_name(value) {
            return Ok(());
        }
        Err(Derive::Used(format!(
            "the value of '--{}' fills a pattern, and is only letters, digits, '.', '_' and '-', with no '..', not starting with '.' or '-', not ending with '.' or '.lock'",
            option
        )))
    };
    let has = |values: &[(&'static str, String)], name: &str| values.iter().any(|(given, _)| *given == name);
    let get = |values: &[(&'static str, String)], name: &str| {
        values.iter().find(|(given, _)| *given == name).map(|(_, value)| value.clone())
    };
    let config = Path::new(root).join(crate::init::CONFIG_FOLDER).join(crate::init::CONFIG_FILE);
    // The file is read first, so that the words from before the issue are refused whatever is given (#78).
    let document = crate::yamlfile::load(&config).map_err(|why| PromptError(why.to_string()))?;
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
    if let (false, Some(issue)) = (has(&values, "requirements"), &issue) {
        values.push(("requirements", default_requirements(issue)));
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
    Ok(values)
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
    pub version: &'a str,
    pub revision: &'a str,
    pub time: &'a str,
    /// An issue's file is used and the option `--requirements` was given: it won over the file (#76).
    pub requirements_given: bool,
}

/// The stamp of an assignment (`docs/revision.md`, point 12): lines of `name: value` in a fixed order,
/// then one line `part: <sha256>  <path>` for each part, as `--parts` gives it. A value that is empty, or that the prompt does not hold, is
/// written as the name and the colon alone. `runner` is empty until a runner is given (#11).
pub fn stamp(said: &Stamp, values: &[(&str, &str)], prompt: &Prompt) -> String {
    // A value the prompt does not hold was not handed out, and is written empty.
    let given = |name: &str| {
        if !prompt.used.contains(&name) {
            return "";
        }
        values.iter().find(|(given, _)| *given == name).map(|(_, value)| *value).unwrap_or("")
    };
    let mut lines: Vec<(&str, &str)> = vec![
        ("role", said.role),
        ("model", said.model),
        ("effort", said.effort),
        ("runner", ""),
        ("version", said.version),
        ("revision", said.revision),
    ];
    for name in &STAMPED[..3] {
        lines.push((name, given(name)));
    }
    lines.push(("branch", given("branch")));
    lines.push(("checkout", given("checkout")));
    lines.push(("time", said.time));
    let mut text = String::new();
    for (name, value) in lines {
        if value.is_empty() {
            text.push_str(&format!("{}:\n", name));
        } else {
            text.push_str(&format!("{}: {}\n", name, value));
        }
    }
    if said.requirements_given {
        text.push_str("requirements: given\n");
    }
    for part in &prompt.parts {
        text.push_str(&format!("part: {}  {}\n", part.hash, part.path));
    }
    text
}
