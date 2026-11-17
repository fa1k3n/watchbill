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
    ["task", "change", "requirements", "round", "checkout", "branch", "task_address", "change_address"];

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
with no `$` just before it. The words of `.watchbill/revision/roles/placeholders.md`, such as `{task}`
and `{branch}`, are known and are filled in; any other placeholder is refused. Anything else in braces is text: `${BUILD_DIR}`,
`{Task}`, `{task-id}` and `{ task }` stay as they are. A review of a change to `.watchbill/` reads that this text does not
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
/// is a lower-case word in braces, as `{task}`: `{`, a letter a to z, letters a to z or `_`, `}`,
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
    }
    let used = PLACEHOLDERS
        .iter()
        .copied()
        .filter(|name| parts.iter().any(|part| placeholders(&part.text).iter().any(|(_, _, found)| found == name)))
        .collect();
    let parts = parts.into_iter().map(|part| Part { path: part.path, hash: part.hash }).collect();
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

/// The placeholders whose values a stamp prints, in the order the stamp has them.
pub const STAMPED: [&str; 5] = ["task", "change", "round", "branch", "checkout"];

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
    for part in &prompt.parts {
        text.push_str(&format!("part: {}  {}\n", part.hash, part.path));
    }
    text
}
