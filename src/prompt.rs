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
/// The file of the roles that names the placeholders. It is not a role.
const PLACEHOLDERS_FILE: &str = "placeholders.md";

/// The words a prompt is filled in with and no others: `roles/placeholders.md` of the revision.
pub const PLACEHOLDERS: [&str; 4] = ["task", "change", "requirements", "round"];

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

A word in braces is a placeholder. Only `{task}`, `{change}`, `{requirements}` and `{round}` are
known; any other is refused. A review of a change to `.watchbill/` reads that this text does not
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

/// The roles of the pinned revision, sorted.
fn roles(root: &Path) -> Result<Vec<String>, PromptError> {
    let folder = root.join(ROLES_FOLDER);
    let entries = fs::read_dir(&folder)
        .or_else(|error| refuse(format!("{} cannot be read: {}", ROLES_FOLDER, error)))?;
    let mut names = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(role) = name.strip_suffix(".md") {
            if name != PLACEHOLDERS_FILE && entry.path().is_file() {
                names.push(role.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

/// The placeholders in a text: for each, where it starts, where it ends and its name. A placeholder
/// is a name in braces, as `{task}`: letters, digits and `_`, not starting with a digit. Any other
/// brace is text.
fn placeholders(text: &str) -> Vec<(usize, usize, &str)> {
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(open) = text[at..].find('{').map(|n| n + at) {
        let rest = &text[open + 1..];
        let name_len = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).unwrap_or(rest.len());
        let name = &rest[..name_len];
        let named = !name.is_empty() && !name.starts_with(|c: char| c.is_ascii_digit());
        if named && rest[name_len..].starts_with('}') {
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
    if !have.iter().any(|name| name == role) {
        return refuse(format!("the revision has no role '{}'. Its roles are: {}", role, have.join(", ")));
    }
    let role_path = format!("{}/{}.md", ROLES_FOLDER, role);
    let first = match read_part(root, role_path.clone())? {
        Some(part) => part,
        None => return refuse(format!("{} cannot be read", role_path)),
    };
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
    let parts = parts.into_iter().map(|part| Part { path: part.path, hash: part.hash }).collect();
    Ok(Prompt { text, parts })
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
