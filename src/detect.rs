//! What `wb init` proposes for a repository, read from the repository itself.
//!
//! The name of its folder, the address of `origin`, and the files that a CI driver
//! or a folder of requirements leaves behind. Nothing here asks the network, and
//! nothing but the host and the path is kept of an address.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::{Command, Stdio};

pub const NONE: &str = "none";

/// The answers of `wb init`, one per line of the file it writes.
#[derive(Clone, Debug, PartialEq)]
pub struct Setup {
    pub project: String,
    pub git_server: String,
    pub repository: String,
    pub tracker: String,
    pub requirements: String,
    pub ci: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Remote {
    pub host: String,
    pub port: String,
    pub path: String,
}

/// What Python takes for white space in `str.strip()`: the white space of Unicode and the four
/// separators `\x1c` to `\x1f`.
fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
}

/// The text without the white space at its ends, as `str.strip()` of Python takes it.
pub fn strip(text: &str) -> &str {
    text.trim_matches(is_space)
}

/// The text a program gave, as Python reads it from a pipe in text mode: every line break is `\n`.
pub fn universal_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// What follows `scheme://`, and the rest after an optional `user[:password]@`, as one pattern
/// `[^/:@]+(:[0-9]+)?/(.+)` and not more: the host, the port and the path.
fn host_port_path(text: &str) -> Option<Remote> {
    let host_end = text.find(['/', ':', '@']).unwrap_or(text.len());
    let (host, rest) = text.split_at(host_end);
    if host.is_empty() {
        return None;
    }
    let (port, rest) = match rest.strip_prefix(':') {
        Some(after) => {
            let digits = after.find(|c: char| !c.is_ascii_digit()).unwrap_or(after.len());
            if digits == 0 {
                return None;
            }
            after.split_at(digits)
        }
        None => ("", rest),
    };
    let path = rest.strip_prefix('/')?;
    if path.is_empty() || path.contains('\n') {
        return None;
    }
    Some(Remote { host: host.to_lowercase(), port: port.to_string(), path: clean(path) })
}

/// A letter of the pattern `[a-z]` of Python with its flag for case. Beyond ASCII it takes four, found
/// by testing every code point on Python 3.9 and 3.14: U+0130, U+0131, U+017F and U+212A.
fn is_scheme_letter(c: char) -> bool {
    c.is_ascii_alphabetic() || matches!(c, '\u{130}' | '\u{131}' | '\u{17f}' | '\u{212a}')
}

/// `scheme://[user[:password]@]host[:port]/path`
fn full_form(text: &str) -> Option<Remote> {
    let colon = text.find(':')?;
    let scheme = &text[..colon];
    let after = text[colon..].strip_prefix("://")?;
    let mut chars = scheme.chars();
    if !is_scheme_letter(chars.next()?) || !chars.all(|c| is_scheme_letter(c) || c.is_ascii_digit() || "+.-".contains(c)) {
        return None;
    }
    // A user name ends at the first `@`, when no `/` comes before it. Without it is the other way to read it.
    if let Some(end) = after.find(['@', '/']) {
        if after[end..].starts_with('@') {
            if let Some(remote) = host_port_path(&after[end + 1..]) {
                return Some(remote);
            }
        }
    }
    host_port_path(after)
}

/// `[user@]host:path`
fn short_form(text: &str) -> Option<Remote> {
    let run = text.find(['@', '/', ':']).unwrap_or(text.len());
    let after = if run > 0 && text[run..].starts_with('@') { &text[run + 1..] } else { text };
    let host_end = after.find(['/', ':', '@']).unwrap_or(after.len());
    let (host, rest) = after.split_at(host_end);
    let path = rest.strip_prefix(':')?;
    if host.is_empty() || path.is_empty() || path.starts_with('/') || path.contains('\n') {
        return None;
    }
    Some(Remote { host: host.to_lowercase(), port: String::new(), path: clean(path) })
}

fn clean(path: &str) -> String {
    let path = path.trim_matches('/');
    path.strip_suffix(".git").unwrap_or(path).to_string()
}

/// The host, port and path of a git address; a user name, password or token in it is dropped.
pub fn parse_remote(url: &str) -> Option<Remote> {
    let text = strip(url);
    full_form(text).or_else(|| short_form(text))
}

/// The kind of git server an address points at, as far as the address can tell.
pub fn git_server_of(remote: Option<&Remote>) -> &'static str {
    let remote = match remote {
        Some(remote) => remote,
        None => return NONE,
    };
    let host = remote.host.as_str();
    if host == "github.com" {
        "github"
    } else if host.contains("gitlab") {
        "gitlab"
    } else if host.contains("gerrit") || remote.port == "29418" {
        "gerrit"
    } else if host.contains("bitbucket") {
        "bitbucket"
    } else if host == "dev.azure.com" || host.ends_with(".visualstudio.com") {
        "azure-devops"
    } else {
        "other"
    }
}

/// `My Project` -> `my-project`: a name that is safe as a folder and in an address.
pub fn project_name(folder: &str) -> String {
    let base = folder.trim_end_matches('/').rsplit('/').next().unwrap_or("").to_lowercase();
    let mut name = String::new();
    let mut in_run = false;
    for c in base.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() || "._-".contains(c) {
            name.push(c);
            in_run = false;
        } else if !in_run {
            name.push('-');
            in_run = true;
        }
    }
    let name = name.trim_matches(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit()));
    if name.is_empty() { "project".to_string() } else { name.to_string() }
}

/// The names of the files in a folder. A name that is not UTF-8 is not counted.
fn files_in(folder: &Path) -> Vec<String> {
    let entries = match std::fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };
    entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect()
}

/// The ending of a file name as `os.path.splitext` of Python gives it: from the last dot, and none
/// for a name that is only dots before it.
fn ending(name: &str) -> &str {
    match name.rfind('.') {
        Some(dot) if name[..dot].chars().any(|c| c != '.') => &name[dot..],
        _ => "",
    }
}

const REQUIREMENT_FOLDERS: [&str; 3] = ["requirements", "docs/requirements", "doc/requirements"];

/// The first folder of requirements that holds a file, with the file ending most of its files have.
fn requirements_of(root: &Path) -> String {
    for folder in REQUIREMENT_FOLDERS {
        let mut count: BTreeMap<&str, usize> = BTreeMap::new();
        let names = files_in(&root.join(folder));
        for name in &names {
            let end = ending(name);
            if !end.is_empty() {
                *count.entry(end).or_insert(0) += 1;
            }
        }
        // The most, and the first of them in the order of the text.
        let mut best: Option<(&str, usize)> = None;
        for (end, n) in &count {
            if best.map_or(true, |(_, most)| *n > most) {
                best = Some((end, *n));
            }
        }
        if let Some((end, _)) = best {
            return format!("{}/*{}", folder, end);
        }
    }
    NONE.to_string()
}

fn has(root: &Path, path: &str) -> bool {
    root.join(path).exists()
}

/// The CI drivers, each with the files or folders it is known by that were found, in the order of the drivers.
/// A driver none of whose files was found is left out.
fn drivers(root: &Path) -> Vec<(&'static str, Vec<&'static str>)> {
    let workflows = files_in(&root.join(".github").join("workflows")).iter().any(|name| name.ends_with(".yml") || name.ends_with(".yaml"));
    let drivers: [(&str, Vec<(&str, bool)>); 7] = [
        ("github-actions", vec![(".github/workflows", workflows)]),
        ("gitlab-ci", vec![(".gitlab-ci.yml", has(root, ".gitlab-ci.yml"))]),
        ("jenkins", vec![("Jenkinsfile", has(root, "Jenkinsfile"))]),
        ("azure-pipelines", vec![("azure-pipelines.yml", has(root, "azure-pipelines.yml"))]),
        ("bitbucket-pipelines", vec![("bitbucket-pipelines.yml", has(root, "bitbucket-pipelines.yml"))]),
        // The driver is known by its file, and the folder that holds it is its own.
        ("circleci", vec![(".circleci", has(root, ".circleci/config.yml"))]),
        ("zuul", vec![(".zuul.yaml", has(root, ".zuul.yaml")), ("zuul.d", has(root, "zuul.d"))]),
    ];
    drivers
        .into_iter()
        .map(|(name, files)| (name, files.into_iter().filter(|(_, found)| *found).map(|(path, _)| path).collect::<Vec<_>>()))
        .filter(|(_, found)| !found.is_empty())
        .collect()
}

/// The CI driver: the first that is found is proposed.
fn ci_of(root: &Path) -> String {
    drivers(root).first().map_or(NONE, |(name, _)| name).to_string()
}

/// The files and folders of every CI driver found in the repository at `root` (#125), in the order of the
/// drivers, as paths from its root. `wb init` proposes them as `build_files`.
pub fn ci_files(root: &Path) -> Vec<String> {
    drivers(root).into_iter().flat_map(|(_, found)| found).map(str::to_string).collect()
}

/// Runs `git` with the given words, with standard error thrown away. `None` when it could not be started.
pub fn git_output(args: &[&str], env: Option<&[(String, String)]>) -> Option<std::process::Output> {
    let mut command = Command::new("git");
    command.args(args).stdin(Stdio::inherit()).stdout(Stdio::piped()).stderr(Stdio::null());
    if let Some(env) = env {
        command.env_clear().envs(env.iter().map(|(name, value)| (name, value)));
    }
    command.output().ok()
}

/// The address of `origin`, or an empty text when there is none. An address that is not UTF-8 is refused.
fn origin(root: &str, env: Option<&[(String, String)]>) -> Result<String, String> {
    match git_output(&["-C", root, "remote", "get-url", "origin"], env) {
        Some(run) if run.status.success() => match String::from_utf8(run.stdout) {
            Ok(text) => Ok(strip(&universal_newlines(&text)).to_string()),
            Err(_) => Err("the address of origin is not UTF-8 text".to_string()),
        },
        _ => Ok(String::new()),
    }
}

/// The host of `origin` when it is plain text and has no port, else an empty text. The user name and the
/// token of an address are never kept. A port is not kept: for `ssh` it is not the port of the web.
pub fn origin_host(root: &str, env: Option<&[(String, String)]>) -> String {
    let remote = origin(root, env).ok().and_then(|address| parse_remote(&address));
    remote
        .filter(|r| r.port.is_empty() && !r.host.is_empty() && r.host.chars().all(|c| c.is_ascii_alphanumeric() || ".-".contains(c)))
        .map_or(String::new(), |r| r.host)
}

/// What the repository in `root` says about itself. The error is what cannot be read as text.
pub fn detect(root: &str, env: Option<&[(String, String)]>) -> Result<Setup, String> {
    let remote = parse_remote(&origin(root, env)?);
    let git_server = git_server_of(remote.as_ref());
    let path = Path::new(root);
    Ok(Setup {
        project: project_name(root),
        git_server: git_server.to_string(),
        repository: remote.as_ref().filter(|r| !r.path.is_empty()).map_or(NONE.to_string(), |r| r.path.clone()),
        // The tracker of a code host is the likely one; any other is the team's to name.
        tracker: if git_server == "github" || git_server == "gitlab" { git_server } else { NONE }.to_string(),
        requirements: requirements_of(path),
        ci: ci_of(path),
    })
}
