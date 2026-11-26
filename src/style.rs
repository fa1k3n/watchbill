//! What `wb init` finds of the project's code style by itself, with no model (#289): the project's own style files
//! and the plain facts of a bounded sample of its code, both as `HEAD` has them, written into
//! `.watchbill/project/all.md` with their sources, so that every role is told them.
//!
//! Only `HEAD` is read, with no replace refs and no graft (`signers::git`), and each file by its size first, so that
//! nothing over a bound is read whole (#261). `all.md` goes into every prompt, so a line that a prompt would read as a
//! placeholder, or that holds a control character, is left out and counted.

use std::collections::BTreeMap;

/// The path of the file, from the root of the repository.
pub const ALL_MD: &str = ".watchbill/project/all.md";

/// The names of the style files, as a file of any folder is called. A name that holds `misra` is one too.
const STYLE_FILES: [&str; 13] = [
    ".editorconfig",
    ".clang-format",
    "_clang-format",
    ".clang-tidy",
    "rustfmt.toml",
    ".rustfmt.toml",
    ".pylintrc",
    "pylintrc",
    ".flake8",
    "uncrustify.cfg",
    ".astylerc",
    "CODING_STYLE.md",
    "CONTRIBUTING.md",
];

/// The most style files taken.
const MOST_STYLE_FILES: usize = 20;
/// The largest style file copied.
const STYLE_CAP: u64 = 64 * 1024;
/// The most lines of one style file copied.
const MOST_LINES: usize = 200;
/// The most code files in the sample, and the largest of them.
const MOST_SAMPLED: usize = 200;
const SAMPLE_CAP: u64 = 256 * 1024;

/// The kinds of code file the sample takes, by their extension.
const CODE: [&str; 24] =
    ["c", "h", "cc", "cpp", "cxx", "hpp", "hh", "hxx", "inc", "s", "S", "asm", "rs", "py", "go", "java", "kt", "js", "ts", "cs", "swift", "m", "mm", "sh"];

/// A file of `HEAD`: its path, its object and its size.
struct Entry {
    path: String,
    object: String,
    size: u64,
}

/// The plain files of `HEAD`, outside `.watchbill/`: none where there is no `HEAD`.
fn head_files(root: &str) -> Result<Vec<Entry>, String> {
    if crate::signers::git(root, &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"], None).is_err() {
        return Ok(Vec::new());
    }
    let listed = crate::signers::git(root, &["ls-tree", "-r", "-l", "-z", "--full-tree", "HEAD"], None)?;
    let mut entries = Vec::new();
    for record in listed.split(|byte| *byte == 0).filter(|record| !record.is_empty()) {
        let Ok(record) = std::str::from_utf8(record) else { continue };
        let Some((head, path)) = record.split_once('\t') else { continue };
        let words: Vec<&str> = head.split_whitespace().collect();
        let [mode, kind, object, size] = words[..] else { continue };
        if kind != "blob" || !(mode == "100644" || mode == "100755") || path.starts_with(".watchbill/") {
            continue;
        }
        let Ok(size) = size.parse() else { continue };
        entries.push(Entry { path: path.to_string(), object: object.to_string(), size });
    }
    Ok(entries)
}

/// The bytes of a blob of `HEAD`, by its object.
fn blob(root: &str, object: &str) -> Result<Vec<u8>, String> {
    crate::signers::git(root, &["cat-file", "blob", object], None)
}

fn name_of(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn is_style_file(path: &str) -> bool {
    let name = name_of(path);
    STYLE_FILES.contains(&name) || name.to_ascii_lowercase().contains("misra")
}

/// Whether a line may go into a prompt as it is: no control character but a tab, no line or paragraph separator, no
/// bidirectional control, and nothing a prompt reads as a placeholder.
pub(crate) fn fit(line: &str) -> bool {
    let control = |c: char| (c.is_control() && c != '\t') || matches!(c, '\u{2028}' | '\u{2029}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}');
    !line.chars().any(control) && crate::prompt::placeholder_names(line).is_empty()
}

/// A path as it is written in `all.md`: `None` where it does not fit a line of a prompt.
fn shown(path: &str) -> Option<&str> {
    fit(path).then_some(path)
}

/// Of `.clang-tidy`, the options of `readability-identifier-naming`, each as `<option>: <value>`, in both forms
/// clang-tidy reads: `- key: <option>` with `value: <value>` on the next line, and `<option>: <value>`.
fn naming_rules(text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut found = Vec::new();
    let mut at = 0;
    while at < lines.len() {
        let line = lines[at].trim();
        at += 1;
        if !line.to_ascii_lowercase().contains("readability-identifier-naming.") {
            continue;
        }
        if let Some(key) = line.strip_prefix("- key:").or_else(|| line.strip_prefix("-  key:")) {
            let value = lines.get(at).map(|next| next.trim()).and_then(|next| next.strip_prefix("value:")).map(str::trim);
            if value.is_some() {
                at += 1;
            }
            found.push(format!("{}: {}", key.trim(), value.unwrap_or("")).trim_end().to_string());
        } else {
            found.push(line.trim_start_matches("- ").to_string());
        }
    }
    found
}

/// The lines of a style file as they are copied: at most `MOST_LINES`, with those that do not fit left out. Gives the
/// lines, how many were left out, and how many were past the bound.
fn copied(path: &str, text: &str) -> (Vec<String>, usize, usize) {
    let lines: Vec<String> = if name_of(path) == ".clang-tidy" { naming_rules(text) } else { text.lines().map(|line| line.trim_end_matches('\r').to_string()).collect() };
    let (mut kept, mut left_out, mut past) = (Vec::new(), 0, 0);
    for line in lines {
        if !fit(&line) {
            left_out += 1;
        } else if kept.len() < MOST_LINES {
            kept.push(line);
        } else {
            past += 1;
        }
    }
    (kept, left_out, past)
}

/// The facts of the files of one kind.
#[derive(Default)]
struct Kind {
    files: usize,
    spaces: usize,
    tabs: usize,
    /// The indent in spaces, and in how many files.
    widths: BTreeMap<usize, usize>,
    lf: usize,
    crlf: usize,
    /// The longest line: its characters and its path.
    longest: Option<(usize, String)>,
}

/// The indent of a file indented with spaces: the widest of 8, 4, 3 and 2 that nine in ten indented lines are a
/// whole number of. A line that goes on a comment (` *`) is not counted.
fn indent_of(widths: &[usize]) -> Option<usize> {
    if widths.is_empty() {
        return None;
    }
    [8, 4, 3, 2].into_iter().find(|step| widths.iter().filter(|width| *width % step == 0).count() * 10 >= widths.len() * 9)
}

impl Kind {
    fn add(&mut self, path: &str, text: &str) {
        self.files += 1;
        if text.contains("\r\n") {
            self.crlf += 1;
        } else {
            self.lf += 1;
        }
        let (mut with_spaces, mut with_tabs) = (0, 0);
        let mut widths = Vec::new();
        for line in text.split('\n') {
            let line = line.strip_suffix('\r').unwrap_or(line);
            let length = line.chars().count();
            if self.longest.as_ref().is_none_or(|(most, _)| length > *most) {
                if let Some(path) = shown(path) {
                    self.longest = Some((length, path.to_string()));
                }
            }
            let rest = line.trim_start_matches([' ', '\t']);
            if rest.is_empty() || rest.len() == line.len() {
                continue;
            }
            if line.starts_with('\t') {
                with_tabs += 1;
            } else if !rest.starts_with('*') {
                with_spaces += 1;
                widths.push(line.len() - line.trim_start_matches(' ').len());
            }
        }
        if with_tabs > with_spaces {
            self.tabs += 1;
        } else if with_spaces > 0 {
            self.spaces += 1;
            if let Some(width) = indent_of(&widths) {
                *self.widths.entry(width).or_default() += 1;
            }
        }
    }

    fn line(&self, extension: &str) -> String {
        let files = |count: usize| if count == 1 { "1 file".to_string() } else { format!("{} files", count) };
        let mut said = format!("- `*.{}`: seen in {}; indented with spaces in {}, with tabs in {}", extension, files(self.files), self.spaces, self.tabs);
        if let Some((width, count)) = self.widths.iter().max_by_key(|(width, count)| (**count, **width)) {
            said.push_str(&format!("; the indent is {} spaces in {}", width, count));
        }
        said.push_str(&format!("; line endings LF in {}, CRLF in {}", self.lf, self.crlf));
        if let Some((length, path)) = &self.longest {
            said.push_str(&format!("; the longest line has {} characters, in {}", length, path));
        }
        said.push('.');
        said
    }
}

/// A fence of backticks longer than any run of them in `lines`, so that no line ends the block.
fn fence(lines: &[String]) -> String {
    let longest = lines.iter().map(|line| line.split(|c| c != '`').map(str::len).max().unwrap_or(0)).max().unwrap_or(0);
    "`".repeat((longest + 1).max(3))
}

/// The text of `all.md` for the repository at `root`, from `HEAD`: `Ok(None)` where it gives no style file and no
/// code file. The error is a fault of git.
pub fn facts(root: &str) -> Result<Option<String>, String> {
    let entries = head_files(root)?;
    let mut text = String::from(
        "# The code style\n\n\
         Found by `wb init` in the files of HEAD, with no model (#289). Each fact names where it was found. \
         A person reads it, corrects it and commits it: until then no prompt takes it.\n",
    );
    let styles: Vec<&Entry> = entries.iter().filter(|entry| is_style_file(&entry.path)).take(MOST_STYLE_FILES).collect();
    if !styles.is_empty() {
        text.push_str("\n## The project's own style files\n");
    }
    for entry in &styles {
        let Some(path) = shown(&entry.path) else {
            text.push_str("\nA style file whose path does not fit a prompt is not copied.\n");
            continue;
        };
        text.push_str(&format!("\n### `{}`\n\n", path));
        if entry.size > STYLE_CAP {
            text.push_str(&format!("It is over {} KiB, and is not copied: read it in the repository.\n", STYLE_CAP / 1024));
            continue;
        }
        let Ok(body) = String::from_utf8(blob(root, &entry.object)?) else {
            text.push_str("It is not UTF-8 text, and is not copied.\n");
            continue;
        };
        let (lines, left_out, past) = copied(&entry.path, &body);
        if lines.is_empty() && name_of(&entry.path) == ".clang-tidy" {
            // Where its naming rules were all left out, the line below says how many: it is not said to hold none.
            if left_out == 0 {
                text.push_str("It holds no naming rule (`readability-identifier-naming`).\n");
            }
        } else {
            let fence = fence(&lines);
            text.push_str(&format!("{}\n{}{}\n", fence, lines.iter().map(|line| format!("{}\n", line)).collect::<String>(), fence));
        }
        if left_out > 0 {
            let plural = if left_out == 1 { "line" } else { "lines" };
            text.push_str(&format!("\n{} {} left out: a prompt would read it as a placeholder, or it holds a control character.\n", left_out, plural));
        }
        if past > 0 {
            text.push_str(&format!("\n{} more lines are not copied: at most {} are.\n", past, MOST_LINES));
        }
    }
    // The sample: the code files of `HEAD` within the bound, in the order of the tree.
    let mut kinds: BTreeMap<String, Kind> = BTreeMap::new();
    let sampled = entries
        .iter()
        .filter(|entry| entry.size <= SAMPLE_CAP && !is_style_file(&entry.path))
        .filter_map(|entry| name_of(&entry.path).rsplit_once('.').filter(|(stem, _)| !stem.is_empty()).map(|(_, extension)| (entry, extension)))
        .filter(|(_, extension)| CODE.contains(extension))
        .take(MOST_SAMPLED);
    for (entry, extension) in sampled {
        if let Ok(body) = String::from_utf8(blob(root, &entry.object)?) {
            kinds.entry(extension.to_string()).or_default().add(&entry.path, &body);
        }
    }
    if !kinds.is_empty() {
        text.push_str(&format!(
            "\n## What the code shows\n\nFrom a sample of at most {} code files of HEAD of {} KiB or less, for each kind of file:\n\n",
            MOST_SAMPLED,
            SAMPLE_CAP / 1024
        ));
        for (extension, kind) in &kinds {
            text.push_str(&kind.line(extension));
            text.push('\n');
        }
    }
    Ok((!styles.is_empty() || !kinds.is_empty()).then_some(text))
}
