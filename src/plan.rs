//! `wb plan add` (#234; `docs/revision.md`, point 15): a job is taken in. Its text is stored in git as a blob, under the
//! hash of its content, and its name is a ref, `refs/watchbill/<name>`, that points at a commit whose tree holds
//! `job.yml` and the text of each source under `sources/<hash>`. A plan may have several sources, each under its own
//! hash (#290, the owner's decision 1): other text under a name that is there is a new commit after the one before.
//!
//! Git is the store: the objects are written with `hash-object`, `mktree` and `commit-tree`, and the ref is moved with
//! `update-ref` from the value it had. It says what it will write first, asks nothing of the network unless `--from`
//! runs a pattern, and only queues: it runs no prompt and starts no session. Here also the fetch line of the jobs that
//! `wb init` and `wb upgrade` add to the remote `origin`.
//!
//! `wb plan list`, `show`, `edit`, `accept` and `remove` (#290) manage a plan as git manages a remote or a note. A plan
//! is a job; its issue's file is `.watchbill/issues/<name>.yml`, accepted as point 13 has it, and its acceptance is void
//! after a source added, or a planner round, later than it. A plan is removed by a marker, and nothing is rewritten.

use std::io::{BufRead, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::init::yaml_value;
use crate::yamlfile::{self, Part};

/// The fetch line of the jobs, which `wb init` and `wb upgrade` add to the remote `origin`, so that a clone and the
/// build server see them (point 15).
pub const FETCH: &str = "+refs/watchbill/*:refs/watchbill/*";

/// Where the refs of the jobs are.
pub const PREFIX: &str = "refs/watchbill/";

const NAME: &str = "wb plan add";

/// How the text of a job is given: exactly one of these.
pub enum Text {
    /// `-F <file>`; `-` is standard input.
    File(String),
    /// `-m <text>`, once or more.
    Messages(Vec<String>),
    /// `--from <adapter> <number>`.
    From(String, String),
}

/// What `wb plan add` was given.
pub struct Add {
    pub name: Option<String>,
    pub text: Text,
    pub source: Option<String>,
    pub level: Option<String>,
    pub hash_only: bool,
}

/// One source of a job, as `job.yml` holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Source {
    hash: String,
    kept: bool,
    source: String,
    level: String,
    by: String,
    time: String,
}

/// A job as the newest commit of its ref holds it.
struct Job {
    /// The name it was first taken in under.
    name: String,
    /// Every name of the job, the first first: each is a ref that points at its newest commit.
    names: Vec<String>,
    sources: Vec<Source>,
}

/// Runs git in `root`, with no variable `GIT_*` of this process, and `input` on standard input. What it printed, or why
/// it failed, with the first line of what git said.
fn git(root: &str, args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>, String> {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args(args);
    for (name, _) in std::env::vars_os() {
        if name.to_string_lossy().starts_with("GIT_") {
            command.env_remove(name);
        }
    }
    command.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() }).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| format!("git could not be run: {}", error))?;
    if let (Some(input), Some(mut stdin)) = (input, child.stdin.take()) {
        let _ = stdin.write_all(input);
    }
    let output = child.wait_with_output().map_err(|error| format!("git could not be waited for: {}", error))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        let said = String::from_utf8_lossy(&output.stderr);
        let first = said.lines().next().unwrap_or("").trim();
        Err(format!("git {} failed{}", args.first().copied().unwrap_or(""), if first.is_empty() { String::new() } else { format!(": {}", first) }))
    }
}

/// What git printed, as one line of text with no line break at its end.
fn git_line(root: &str, args: &[&str], input: Option<&[u8]>) -> Result<String, String> {
    let printed = git(root, args, input)?;
    String::from_utf8(printed).map(|text| text.trim_end_matches('\n').to_string()).map_err(|_| format!("git {} printed what is not UTF-8 text", args[0]))
}

/// Whether the remote `origin` is there and lacks the fetch line of the jobs.
pub fn fetch_missing(root: &str) -> Result<bool, String> {
    if git(root, &["config", "--get", "remote.origin.url"], None).is_err() {
        return Ok(false);
    }
    // `--get-all` ends with 1 where there is no line at all.
    let lines = git(root, &["config", "--get-all", "remote.origin.fetch"], None).unwrap_or_default();
    Ok(!String::from_utf8_lossy(&lines).lines().any(|line| line == FETCH))
}

/// Adds the fetch line of the jobs to the remote `origin`, after the lines that are there.
pub fn add_fetch(root: &str) -> Result<(), String> {
    git(root, &["config", "--add", "remote.origin.fetch", FETCH], None).map(|_| ())
}

/// What is said of the fetch line before it is written.
pub fn fetch_said() -> String {
    format!("the fetch line {} of the remote origin, in .git/config, so that a clone fetches the jobs", FETCH)
}

/// Why a name is not the name of a job, if it is not: git refuses `refs/watchbill/<name>` as a ref.
fn name_fault(root: &str, name: &str) -> Option<String> {
    let full = format!("{}{}", PREFIX, name);
    if !name.is_empty() && git(root, &["check-ref-format", &full], None).is_ok() {
        return None;
    }
    Some(format!(
        "'{}' is not the name of a job: the name is a ref, {}<name>, and git check-ref-format refuses it. A name may hold / (github/54), and holds no ':', space, '..', '~', '^', '?', '*', '[' or '\\', no part that starts with '.' or ends with '.lock', no '//' and no '/' at its end",
        name.escape_debug(),
        PREFIX
    ))
}

/// One value of `git config`, or why there is none.
fn identity(root: &str, key: &str) -> Result<String, String> {
    let value = git_line(root, &["config", key], None).unwrap_or_default();
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("git has no {} for this repository. Set it with `git config {} <value>`", key, key));
    }
    if value.chars().any(|c| c.is_control()) || value.contains(['<', '>']) {
        return Err(format!("the {} of git holds a control character, '<' or '>'", key));
    }
    Ok(value.to_string())
}

/// Reads `job.yml` of the commit `commit`, through the one loader. `label` names it in a refusal.
fn read_job(root: &str, commit: &str, label: &str) -> Result<Job, String> {
    let bytes = git(root, &["cat-file", "blob", &format!("{}:job.yml", commit)], None).map_err(|_| format!("{} holds no job.yml", label))?;
    let doc = yamlfile::load_bytes(format!("{}:job.yml", label), bytes).map_err(|why| why.to_string())?;
    let fail = |why: yamlfile::YamlError| why.to_string();
    doc.only(&[], &["name", "names", "sources"]).map_err(fail)?;
    doc.need(&[], &["name", "names", "sources"]).map_err(fail)?;
    let name = doc.text(&[Part::from("name")]).map_err(fail)?;
    let names = doc.texts(&[Part::from("names")], false).map_err(fail)?;
    if !names.contains(&name) {
        return Err(doc.fail(&[Part::from("names")], &format!("the names do not hold the name `{}`", name), false).to_string());
    }
    let mut sources = Vec::new();
    let mut index = 0;
    while doc.has(&[Part::from("sources"), Part::from(index)]) {
        let place = [Part::from("sources"), Part::from(index)];
        const FIELDS: [&str; 6] = ["hash", "text", "source", "level", "by", "time"];
        doc.only(&place, &FIELDS).map_err(fail)?;
        doc.need(&place, &FIELDS).map_err(fail)?;
        let field = |key: &str| doc.text(&[Part::from("sources"), Part::from(index), Part::from(key)]).map_err(fail);
        let text = field("text")?;
        let kept = match text.as_str() {
            "kept" => true,
            "hash only" => false,
            _ => {
                let place = [Part::from("sources"), Part::from(index), Part::from("text")];
                return Err(doc.fail(&place, &format!("`{}` is not `kept` or `hash only`", text), false).to_string());
            }
        };
        sources.push(Source { hash: field("hash")?, kept, source: field("source")?, level: field("level")?, by: field("by")?, time: field("time")? });
        index += 1;
    }
    if sources.is_empty() {
        doc.texts(&[Part::from("sources")], false).map_err(fail)?;
    }
    Ok(Job { name, names, sources })
}

/// `job.yml` of a job: its first name, all its names, and each source, in the order they were added.
fn job_yml(job: &Job) -> String {
    let mut text = String::from("# A job of Watchbill, written by `wb plan add` (docs/revision.md, point 15).\n");
    text.push_str(&format!("name: {}\nnames:\n", yaml_value(&job.name)));
    for name in &job.names {
        text.push_str(&format!("  - {}\n", yaml_value(name)));
    }
    text.push_str("sources:\n");
    for source in &job.sources {
        text.push_str(&format!("  - hash: {}\n", yaml_value(&source.hash)));
        text.push_str(&format!("    text: {}\n", yaml_value(if source.kept { "kept" } else { "hash only" })));
        text.push_str(&format!("    source: {}\n", yaml_value(&source.source)));
        text.push_str(&format!("    level: {}\n", yaml_value(&source.level)));
        text.push_str(&format!("    by: {}\n", yaml_value(&source.by)));
        text.push_str(&format!("    time: {}\n", yaml_value(&source.time)));
    }
    text
}

/// The text of `-m`, as `git commit -m` makes it: each on a line of its own after the one before, so that each is a
/// paragraph, then cleaned up as git's `whitespace` clean-up does: the whitespace at the end of each line is taken
/// off, runs of blank lines are one, and blank lines at the start and the end are taken out.
pub fn joined(messages: &[String]) -> String {
    let mut built = String::new();
    for message in messages {
        if !built.is_empty() {
            built.push('\n');
        }
        built.push_str(message);
        if !built.ends_with('\n') {
            built.push('\n');
        }
    }
    stripspace(&built)
}

/// git's clean-up of whitespace, as `git stripspace` does it.
fn stripspace(text: &str) -> String {
    let mut out = String::new();
    let mut blank = false;
    for line in text.split('\n') {
        let line = line.trim_end_matches([' ', '\t', '\r', '\x0b', '\x0c']);
        if line.is_empty() {
            blank = true;
            continue;
        }
        if blank && !out.is_empty() {
            out.push('\n');
        }
        blank = false;
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// What a source given to `wb plan add` may be: one line of text with no control character.
pub fn source_fault(source: &str) -> Option<&'static str> {
    if source.trim().is_empty() {
        return Some("is empty");
    }
    if source.chars().any(|c| c.is_control() || c == '\u{2028}' || c == '\u{2029}') {
        return Some("holds a control character or a line break");
    }
    None
}

/// Reads at most `cap` bytes and one more.
fn read_capped(reader: &mut dyn Read, cap: u64) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(cap + 1).read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Why a text cannot be a job's, if it cannot: over the cap, not UTF-8, or nothing but whitespace.
fn text_fault(bytes: &[u8], cap: u64) -> Option<&'static str> {
    if bytes.len() as u64 > cap {
        return Some("the text is over 1 MiB");
    }
    match std::str::from_utf8(bytes) {
        Err(_) => Some("the text is not UTF-8 text"),
        Ok(text) if crate::detect::strip(text).is_empty() => Some("the text is empty"),
        Ok(_) => None,
    }
}

/// The entries of a tree, each as `mktree` reads it, but those named in `leave`.
fn entries(root: &str, tree: &str, leave: &[&str]) -> Result<Vec<String>, String> {
    let listed = git(root, &["ls-tree", "-z", tree], None)?;
    let mut kept = Vec::new();
    for entry in listed.split(|byte| *byte == 0).filter(|entry| !entry.is_empty()) {
        let entry = String::from_utf8(entry.to_vec()).map_err(|_| format!("the tree {} holds a name that is not UTF-8 text", tree))?;
        let name = entry.split_once('\t').map(|(_, name)| name).unwrap_or("");
        if !leave.contains(&name) {
            kept.push(entry);
        }
    }
    Ok(kept)
}

/// Makes a tree of entries, as `ls-tree -z` gives them.
fn mktree(root: &str, entries: &[String]) -> Result<String, String> {
    let mut input = Vec::new();
    for entry in entries {
        input.extend_from_slice(entry.as_bytes());
        input.push(0);
    }
    git_line(root, &["mktree", "-z"], Some(&input))
}

/// The commit of a job: `job.yml` of `job`, the texts of the commit before, `old`, and `text`, the hash of a text to add
/// to them; the other entries of the tree before are kept as they are. Its message is `message`.
fn write_commit(root: &str, job: &Job, old: Option<&str>, text: Option<&str>, message: &str) -> Result<String, String> {
    let yml = job_yml(job);
    let yml_hash = git_line(root, &["hash-object", "-t", "blob", "-w", "--stdin"], Some(yml.as_bytes()))?;
    let mut texts = match old {
        Some(commit) if git(root, &["rev-parse", "--verify", "-q", &format!("{}:sources", commit)], None).is_ok() => {
            entries(root, &format!("{}:sources", commit), &[])?
        }
        _ => Vec::new(),
    };
    if let Some(hash) = text {
        texts.push(format!("100644 blob {}\t{}", hash, hash));
    }
    let mut top = match old {
        Some(commit) => entries(root, &format!("{}^{{tree}}", commit), &["job.yml", "sources"])?,
        None => Vec::new(),
    };
    top.push(format!("100644 blob {}\tjob.yml", yml_hash));
    if !texts.is_empty() {
        top.push(format!("040000 tree {}\tsources", mktree(root, &texts)?));
    }
    let tree = mktree(root, &top)?;
    let mut args = vec!["commit-tree", tree.as_str()];
    if let Some(commit) = old {
        args.push("-p");
        args.push(commit);
    }
    git_line(root, &args, Some(message.as_bytes()))
}

/// Moves each ref of `moved` from `old` to `new`, and makes `made` at `new`, in one transaction of `update-ref`: either
/// all of them or none, and none where a ref is no longer what it was read as.
fn move_refs(root: &str, moved: &[String], made: Option<&str>, old: &str, new: &str) -> Result<(), String> {
    let mut input = String::new();
    for name in moved {
        input.push_str(&format!("update {} {} {}\n", name, new, old));
    }
    if let Some(name) = made {
        input.push_str(&format!("create {} {}\n", name, new));
    }
    git(root, &["update-ref", "--stdin"], Some(input.as_bytes())).map(|_| ())
}

/// Says what will be written: the blob of the text, or none, `job.yml`, the tree and the commit, the refs it moves to
/// the commit from the one before, and the ref it makes.
fn say(out: &mut dyn Write, job: &Job, text: Option<&str>, kept: &[&str], old: Option<&str>, moved: &[String], made: Option<&str>) {
    let _ = writeln!(out, "It writes:");
    if let Some(line) = text {
        let _ = writeln!(out, "  {}", line);
    }
    let _ = writeln!(out, "  job.yml:");
    for line in job_yml(job).lines() {
        let _ = writeln!(out, "    {}", line);
    }
    let tree: Vec<String> = std::iter::once("job.yml".to_string()).chain(kept.iter().map(|hash| format!("sources/{}", hash))).collect();
    let _ = writeln!(out, "  a tree of {}", tree.join(", "));
    match old {
        Some(commit) => {
            let _ = writeln!(out, "  a commit after {}", commit);
        }
        None => {
            let _ = writeln!(out, "  a commit, the first of the job");
        }
    }
    for name in moved {
        let _ = writeln!(out, "  the ref {} moved to it from that commit", name);
    }
    if let Some(name) = made {
        let _ = writeln!(out, "  the ref {} made at it", name);
    }
}

/// Runs `wb plan add` in the repository at `root`, with a relative `-F` read from `folder`, and the organisation's
/// file at `organisation`. 0 the job is taken in or nothing changes; 1 refused; 2 not set up, or no adapter for `--from`.
pub fn add(root: &str, folder: &Path, organisation: &Path, add: &Add, input: &mut dyn BufRead, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let limit = |err: &mut dyn Write, why: &str| crate::refusal::limit(err, NAME, format_args!("{}. Nothing written.", why));
    let place = format!("{}/{}", crate::init::CONFIG_FOLDER, crate::init::CONFIG_FILE);
    if !Path::new(root).join(&place).is_file() {
        let _ = writeln!(err, "{}: {} is not there. Watchbill is not set up in this repository: wb init sets it up.", NAME, place);
        return 2;
    }
    // `--from`: until adapters are built, the adapter is the tracker of `watchbill.yml`, read by its pattern `issue_text`.
    let mut pattern = None;
    if let Text::From(adapter, number) = &add.text {
        let settings = match crate::starter::settings(root) {
            Ok(settings) => settings,
            Err(why) => return limit(err, &why),
        };
        let tracker = settings.text(&[Part::from("tracker")]).ok();
        let line = settings.text(&[Part::from("issue_text")]).ok();
        match (tracker.as_deref() == Some(adapter.as_str()), line) {
            (true, Some(line)) => pattern = Some(line.replace("{issue}", number)),
            _ => {
                let _ = writeln!(
                    err,
                    "{}: no adapter '{}' is set up: {} has no tracker: {} with a pattern issue_text. Pipe the text in: <the command that prints it> | wb plan add {}/{} -F -",
                    NAME, adapter, place, adapter, adapter, number
                );
                return 2;
            }
        }
    }
    let given_name = match &add.text {
        Text::From(adapter, number) => Some(format!("{}/{}", adapter, number)),
        _ => add.name.clone(),
    };
    if let Some(name) = &given_name {
        if let Some(why) = name_fault(root, name) {
            return limit(err, &why);
        }
    }

    // The text of -F and -m, read before anything else is asked: a file is read only where it is a plain file, through
    // the cap, so that a FIFO or a device never makes `wb` wait or fill the memory.
    let cap = crate::bounded::TEXT_CAP;
    let read = match &add.text {
        Text::File(path) if path == "-" => match read_capped(input, cap) {
            Ok(bytes) => Some(bytes),
            Err(error) => return limit(err, &format!("standard input cannot be read: {}", error)),
        },
        Text::File(path) => match crate::bounded::read_following(&folder.join(path), cap) {
            Ok(bytes) => Some(bytes),
            Err(crate::bounded::Fault::TooBig(_)) => return limit(err, "the text is over 1 MiB"),
            Err(crate::bounded::Fault::NotPlain) => {
                return limit(err, &format!("{} is not a plain file, and only a plain file is read: a pipe is read with -F -", path))
            }
            Err(fault) => return limit(err, &format!("{} cannot be read: {}", path, fault)),
        },
        Text::Messages(messages) => Some(joined(messages).into_bytes()),
        Text::From(..) => None,
    };
    if let Some(why) = read.as_deref().and_then(|bytes| text_fault(bytes, cap)) {
        return limit(err, why);
    }

    // The level: the organisation's, no higher than the repository's.
    let found = match crate::confidentiality::read(organisation) {
        Ok(found) => found,
        Err(why) => return limit(err, &format!("{} The level of a job is one of the organisation's levels", why.trim_end_matches('.'))),
    };
    let repository = match crate::confidentiality::repository_level(Path::new(root), &found) {
        Ok(level) => level,
        Err(why) => return limit(err, &why),
    };
    let level = match &add.level {
        None => repository.clone(),
        Some(level) => match found.rank(level) {
            None => return limit(err, &format!("the level `{}` is not one of the organisation's levels: {}", level, found.levels.join(", "))),
            Some(rank) if Some(rank) > found.rank(&repository) => {
                return limit(err, &format!("the level `{}` is above `{}`, the level of the repository: a job of it would be in every clone", level, repository));
            }
            Some(_) => level.clone(),
        },
    };
    if add.hash_only && !found.hash_only {
        return limit(
            err,
            &format!("--hash-only is not allowed: the organisation's file {} has no `jobs: hash_only: allowed`", organisation.display()),
        );
    }
    let by = match (identity(root, "user.name"), identity(root, "user.email")) {
        (Ok(name), Ok(email)) => format!("{} <{}>", name, email),
        (Err(why), _) | (_, Err(why)) => return limit(err, &why),
    };

    // The text of --from, read by its pattern after the level is known.
    let (bytes, source) = match read {
        Some(bytes) => (bytes, add.source.clone()),
        None => {
            let line = pattern.clone().unwrap_or_default();
            if let Some(why) = source_fault(&line) {
                return limit(err, &format!("the pattern issue_text, filled, {}", why));
            }
            match crate::starter::run_capped(root, "issue_text", &line, cap) {
                Ok(bytes) => (bytes, Some(line)),
                Err(why) => return limit(err, &why),
            }
        }
    };
    if let Some(why) = text_fault(&bytes, cap) {
        return limit(err, why);
    }
    let source = source.unwrap_or_else(|| "none".to_string());
    let hash = match git_line(root, &["hash-object", "-t", "blob", "--stdin"], Some(&bytes)) {
        Ok(hash) => hash,
        Err(why) => return limit(err, &why),
    };
    let name = given_name.unwrap_or_else(|| hash.chars().take(12).collect());
    let full = format!("{}{}", PREFIX, name);

    // The refs there are, each with what it points at.
    let listed = match git_line(root, &["for-each-ref", "--format=%(objectname) %(refname)", PREFIX], None) {
        Ok(listed) => listed,
        Err(why) => return limit(err, &why),
    };
    let refs: Vec<(String, String)> = listed
        .lines()
        .filter_map(|line| line.split_once(' '))
        .map(|(object, name)| (object.to_string(), name.to_string()))
        .collect();
    if let Some((_, other)) = refs.iter().find(|(_, other)| other.starts_with(&format!("{}/", full)) || full.starts_with(&format!("{}/", other))) {
        return limit(err, &format!("the name '{}' meets the job {} as a folder, and git cannot hold both refs", name, &other[PREFIX.len()..]));
    }
    let commit_of = |object: &str| git_line(root, &["rev-parse", "--verify", "-q", &format!("{}^{{commit}}", object)], None);
    let at = |commit: &str| -> Vec<String> { refs.iter().filter(|(object, _)| object == commit).map(|(_, name)| name.clone()).collect() };
    let _ = writeln!(out, "{}: the text is {} bytes, and its hash is {}.", NAME, bytes.len(), hash);

    // The job the text goes to: the one of the name, or one that has the text already, or a new one.
    let mut existing = None;
    if let Some((object, _)) = refs.iter().find(|(_, other)| *other == full) {
        let commit = match commit_of(object) {
            Ok(commit) => commit,
            Err(_) => return limit(err, &format!("{} is not a commit", full)),
        };
        let job = match read_job(root, &commit, &full) {
            Ok(job) => job,
            Err(why) => return limit(err, &why),
        };
        // A plan removed keeps its name (#290): nothing is taken in under it.
        if is_removed(root, &commit) {
            return limit(err, &format!("the plan {} was removed (commit {}), and its name stays taken: give the text another name", name, commit));
        }
        if job.sources.iter().any(|source| source.hash == hash) {
            let _ = writeln!(out, "{}: the text is already a source of the job {}. Nothing to do. Nothing written.", NAME, name);
            return 0;
        }
        existing = Some((commit, job, false));
    } else {
        // Text that is already a job under another name gives that job this name too. A ref that is not a job is passed
        // over, and said once.
        let mut passed = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        for (object, other) in &refs {
            if seen.contains(object) {
                continue;
            }
            seen.push(object.clone());
            let held = commit_of(object).and_then(|commit| read_job(root, &commit, other).map(|job| (commit, job)));
            match held {
                // The text of a plan removed is not taken as that plan's (#290).
                Ok((commit, job)) if job.sources.iter().any(|source| source.hash == hash) && !is_removed(root, &commit) => {
                    existing = Some((commit, job, true));
                    break;
                }
                Ok(_) => {}
                Err(_) => passed.extend(refs.iter().filter(|(o, _)| o == object).map(|(_, name)| name.clone())),
            }
        }
        if !passed.is_empty() {
            let _ = writeln!(err, "{}: passed over, as not jobs: {}.", NAME, passed.join(", "));
        }
    }

    let seconds = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|time| time.as_secs()).unwrap_or(0);
    let added = Source { hash: hash.clone(), kept: !add.hash_only, source, level, by, time: crate::prompt::utc(seconds) };
    let (job, old, moved, made, text, message) = match existing {
        // A name more for a job that has the text: job.yml lists it, and every name of the job moves to the new commit.
        Some((commit, mut job, true)) => {
            let _ = writeln!(out, "{}: the text is already a source of the job {}. This gives it the name {} too.", NAME, job.name, name);
            job.names.push(name.clone());
            let moved = at(&commit);
            (job, Some(commit), moved, Some(full.clone()), None, format!("plan add {}\n\nname {} for the job {}\n", name, name, hash))
        }
        // A source more under a name that is there: every name of the job moves to the new commit.
        Some((commit, mut job, false)) => {
            let there: Vec<&str> = job.sources.iter().map(|source| source.hash.as_str()).collect();
            let _ = writeln!(out, "{}: the job {} has the source {}. This adds the source {}.", NAME, name, there.join(", "), hash);
            job.sources.push(added.clone());
            let moved = at(&commit);
            (job, Some(commit), moved, None, Some(hash.clone()), format!("plan add {}\n\nsource {}\n", name, hash))
        }
        None => {
            let job = Job { name: name.clone(), names: vec![name.clone()], sources: vec![added.clone()] };
            (job, None, Vec::new(), Some(full.clone()), Some(hash.clone()), format!("plan add {}\n\nsource {}\n", name, hash))
        }
    };
    let adds_source = text.is_some();
    let text = text.filter(|_| !add.hash_only);
    let kept: Vec<&str> = job.sources.iter().filter(|source| source.kept).map(|source| source.hash.as_str()).collect();
    let blob_line = match &text {
        Some(hash) => Some(format!("the blob {} of the text", hash)),
        None if adds_source => Some(format!("no blob of the text: --hash-only keeps its hash {} and its source alone", hash)),
        None => None,
    };
    say(out, &job, blob_line.as_deref(), &kept, old.as_deref(), &moved, made.as_deref());

    crate::interrupt::writing();
    let failed = |err: &mut dyn Write, why: String| crate::refusal::limit(err, NAME, format_args!("{}. No ref is written.", why));
    if let Some(hash) = &text {
        match git_line(root, &["hash-object", "-t", "blob", "-w", "--stdin"], Some(&bytes)) {
            Ok(written) if written == *hash => {}
            Ok(written) => return failed(err, format!("git wrote the text as {}, not {}", written, hash)),
            Err(why) => return failed(err, why),
        }
    }
    let commit = match write_commit(root, &job, old.as_deref(), text.as_deref(), &message) {
        Ok(commit) => commit,
        Err(why) => return failed(err, why),
    };
    if let Err(why) = move_refs(root, &moved, made.as_deref(), old.as_deref().unwrap_or(""), &commit) {
        return failed(err, why);
    }
    for name in &moved {
        let _ = writeln!(out, "Wrote {}: moved to commit {}.", name, commit);
    }
    if let Some(name) = &made {
        let _ = writeln!(out, "Wrote {}: made at commit {}.", name, commit);
    }
    0
}

// ---- `wb plan list`, `show`, `edit`, `accept` and `remove` (#290) ----

/// The marker that `wb plan remove` adds to the tree of a plan: a plan whose newest commit holds it is removed.
const REMOVED: &str = "removed";

/// A plan as its ref has it: the commit, the job, and whether it is removed.
struct Plan {
    commit: String,
    job: Job,
    removed: bool,
}

/// The plan of `name`, or `None` where its ref is not there or is not a job.
fn plan_of(root: &str, name: &str) -> Option<Plan> {
    if name_fault(root, name).is_some() {
        return None;
    }
    let full = format!("{}{}", PREFIX, name);
    let commit = git_line(root, &["rev-parse", "--verify", "-q", &format!("{}^{{commit}}", full)], None).ok()?;
    let job = read_job(root, &commit, &full).ok()?;
    let removed = is_removed(root, &commit);
    Some(Plan { commit, job, removed })
}

/// Whether the tree of a plan's commit holds the marker of `wb plan remove`.
fn is_removed(root: &str, commit: &str) -> bool {
    git(root, &["cat-file", "-e", &format!("{}:{}", commit, REMOVED)], None).is_ok()
}

/// The plan of `name` that is there and not removed, or the refusal that names `wb plan list`.
fn live_plan(root: &str, name: &str, command: &str, err: &mut dyn Write) -> Result<Plan, i32> {
    match plan_of(root, name) {
        Some(plan) if !plan.removed => Ok(plan),
        Some(plan) => Err(crate::refusal::limit(
            err,
            command,
            format_args!("the plan {} was removed (commit {}). wb plan list lists the plans. Nothing written.", crate::issue::shown(name), plan.commit),
        )),
        None => Err(crate::refusal::limit(
            err,
            command,
            format_args!("there is no plan {}. wb plan list lists the plans, and wb plan add takes one in. Nothing written.", crate::issue::shown(name)),
        )),
    }
}

/// Whether a text is a time as `wb` writes it: `2026-11-24T10:00:00Z`, which sorts as the time does.
fn is_time(time: &str) -> bool {
    let bytes = time.as_bytes();
    bytes.len() == 20
        && bytes.iter().enumerate().all(|(at, byte)| match at {
            4 | 7 => *byte == b'-',
            10 => *byte == b'T',
            13 | 16 => *byte == b':',
            19 => *byte == b'Z',
            _ => byte.is_ascii_digit(),
        })
}

/// The value of the line `<key>: ` of a text.
fn line_of<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().find_map(|line| line.trim_end_matches('\r').strip_prefix(&format!("{}: ", key)))
}

/// Whether a source of the job was added after an acceptance at `accepted`, to the second. A time that cannot be read
/// counts as after.
fn source_after(job: &Job, accepted: &str) -> bool {
    !is_time(accepted) || job.sources.iter().any(|source| !is_time(&source.time) || source.time.as_str() > accepted)
}

/// Whether a plan of `name` is there, not removed.
pub(crate) fn is_plan(root: &str, name: &str) -> bool {
    plan_of(root, name).is_some_and(|plan| !plan.removed)
}

/// Whether the plan of `name`, where there is one, has a source added after the acceptance whose text is `accepted`
/// (the text of `.accepted`). `false` where there is no plan of the name.
pub(crate) fn sources_after_acceptance(root: &str, name: &str, accepted: &str) -> bool {
    match plan_of(root, name) {
        Some(plan) if !plan.removed => source_after(&plan.job, line_of(accepted, "time").unwrap_or("")),
        _ => false,
    }
}

/// Whether one of the whole comments `bodies` is a `[stamp] planner` whose time is the same as or later than the
/// acceptance's, `accepted`: the stamp is written as the round starts, so a round of the same second ends after it.
/// A time that cannot be read counts as after.
pub(crate) fn planner_after(bodies: &[String], accepted: &str) -> bool {
    bodies.iter().any(|body| {
        let first = body.split('\n').next().unwrap_or("").trim_end_matches('\r');
        let is_planner = first
            .strip_prefix("[stamp] planner round ")
            .is_some_and(|round| !round.is_empty() && round.chars().all(|c| c.is_ascii_digit()));
        is_planner && line_of(body, "time").map_or(true, |time| !is_time(time) || !is_time(accepted) || time >= accepted)
    })
}

/// The time an acceptance was written, from the text of `.accepted`.
pub(crate) fn accepted_time(accepted: &str) -> String {
    line_of(accepted, "time").unwrap_or("").to_string()
}

/// `wb plan list` in the repository at `root`: each plan once, by its first name, one to a line, and nothing else.
pub fn list(root: &str, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let listed = match git_line(root, &["for-each-ref", "--format=%(objectname) %(refname)", PREFIX], None) {
        Ok(listed) => listed,
        Err(why) => return crate::refusal::limit(err, "wb plan list", format_args!("{}.", why)),
    };
    let refs: Vec<(&str, &str)> = listed.lines().filter_map(|line| line.split_once(' ')).collect();
    let mut seen: Vec<&str> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut passed: Vec<&str> = Vec::new();
    for (object, full) in &refs {
        if seen.contains(object) {
            continue;
        }
        seen.push(object);
        let held = git_line(root, &["rev-parse", "--verify", "-q", &format!("{}^{{commit}}", object)], None)
            .and_then(|commit| read_job(root, &commit, full).map(|job| (commit, job)));
        match held {
            Ok((commit, job)) => {
                if !is_removed(root, &commit) {
                    names.push(job.name);
                }
            }
            Err(_) => passed.extend(refs.iter().filter(|(o, _)| o == object).map(|(_, name)| *name)),
        }
    }
    names.sort();
    names.dedup();
    for name in &names {
        let _ = writeln!(out, "{}", crate::issue::shown(name));
    }
    if !passed.is_empty() {
        let _ = writeln!(err, "wb plan list: passed over, as not jobs: {}.", passed.join(", "));
    }
    0
}

/// The issue's file of a plan and its acceptance, from the root.
fn files_of(name: &str) -> (String, String) {
    (format!("{}/{}.yml", crate::issue::FOLDER, name), format!("{}/{}.accepted", crate::issue::FOLDER, name))
}

/// Reads a plain file of the working tree, through the cap of an issue's file, or `None`.
fn plain(root: &str, path: &str) -> Option<Vec<u8>> {
    let full = Path::new(root).join(path);
    match std::fs::symlink_metadata(&full) {
        Ok(found) if found.is_file() => crate::bounded::read_plain(&full, crate::issue::MAX_BYTES).ok(),
        _ => None,
    }
}

/// The state of a plan whose issue's file is `bytes`, by its acceptance and its sources; the planner's stamps are
/// read after, by the caller, where it is accepted. `Ok((by, time))` where it is accepted, else the state's words.
fn state_of(root: &str, plan: &Plan, name: &str, bytes: &[u8]) -> Result<(String, String), &'static str> {
    let (_, accepted) = files_of(name);
    let Some(body) = plain(root, &accepted).and_then(|body| String::from_utf8(body).ok()) else {
        return Err("not accepted");
    };
    if line_of(&body, "sha256") != Some(crate::manifest::hash_bytes(bytes).as_str()) {
        return Err("not accepted (edited after the acceptance)");
    }
    let time = accepted_time(&body);
    if source_after(&plan.job, &time) {
        return Err("not accepted (a source added after the acceptance)");
    }
    Ok((line_of(&body, "by").unwrap_or("").to_string(), time))
}

/// `wb plan show <name>` in the repository at `root`: the plan's names, its sources, its issue's file and its state.
pub fn show(root: &str, name: &str, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    const COMMAND: &str = "wb plan show";
    // It only reads (#254): every pattern is read from the file of `HEAD`.
    let _read_only = match crate::starter::read_only(root) {
        Err(not_read) if not_read.status == 2 => {
            let _ = writeln!(err, "{}: {}.", COMMAND, not_read.why);
            return 2;
        }
        Err(not_read) => return crate::refusal::limit(err, COMMAND, format_args!("{}.", not_read.why)),
        Ok((guard, differs)) => {
            if differs {
                let _ = writeln!(err, "{}", crate::starter::drift_line(COMMAND));
            }
            guard
        }
    };
    let plan = match live_plan(root, name, COMMAND, err) {
        Ok(plan) => plan,
        Err(code) => return code,
    };
    let shown = crate::issue::shown;
    let mut text = format!("plan {}\nnames: {}\n", shown(name), plan.job.names.iter().map(|name| shown(name)).collect::<Vec<_>>().join(", "));
    for (at, source) in plan.job.sources.iter().enumerate() {
        text.push_str(&format!(
            "source {}: {}  from {}  {}  level {}  by {}  {}\n",
            at + 1,
            shown(&source.hash),
            shown(&source.source),
            if source.kept { "kept" } else { "hash only" },
            shown(&source.level),
            shown(&source.by),
            shown(&source.time)
        ));
    }
    if !crate::issue::is_name(name) {
        text.push_str(&format!(
            "issue's file: none, as {} is not an issue's name (letters, digits, '.', '_' and '-'), until point 15\nstate: not planned\n",
            shown(name)
        ));
        let _ = out.write_all(text.as_bytes());
        return 0;
    }
    let (file, _) = files_of(name);
    text.push_str(&format!("issue's file: {}\n", file));
    let Some(bytes) = plain(root, &file) else {
        text.push_str(&format!("state: not planned ({} is not there)\n", file));
        let _ = out.write_all(text.as_bytes());
        return 0;
    };
    let state = match state_of(root, &plan, name, &bytes) {
        Err(words) => words.to_string(),
        Ok((by, time)) => {
            // A planner round after the acceptance voids it (the owner, 2026-11-24): its stamp is read whole.
            let line = match crate::prompt::pattern_of(root, "issue_comment_bodies") {
                Ok(line) => line,
                Err(why) => return crate::refusal::limit(err, COMMAND, format_args!("{}", why)),
            };
            match line {
                None => {
                    let _ = writeln!(err, "{}: watchbill.yml has no `issue_comment_bodies`, so the planner's stamps were not read.", COMMAND);
                    format!("accepted by {} at {}", shown(&by), shown(&time))
                }
                Some(line) => match crate::starter::run_capped(root, "issue_comment_bodies", &line.replace("{issue}", name), crate::bounded::TEXT_CAP) {
                    Err(why) => return crate::refusal::limit(err, COMMAND, format_args!("{}", why)),
                    Ok(printed) if printed.len() as u64 > crate::bounded::TEXT_CAP => {
                        return crate::refusal::limit(err, COMMAND, format_args!("`issue_comment_bodies` printed more than 1 MiB"))
                    }
                    Ok(printed) if planner_after(&crate::answer::split_bodies(&printed), &time) => {
                        "not accepted (planned again after the acceptance)".to_string()
                    }
                    Ok(_) => format!("accepted by {} at {}", shown(&by), shown(&time)),
                },
            }
        }
    };
    text.push_str(&format!("state: {}\n\n", state));
    let file_text = String::from_utf8_lossy(&bytes);
    for line in file_text.split_inclusive('\n') {
        let (body, end) = match line.strip_suffix('\n') {
            Some(body) => (body, "\n"),
            None => (line, ""),
        };
        if body.chars().any(|c| crate::issue::is_unsafe(c, true) && c != '\t') {
            text.push_str(&shown(body));
        } else {
            text.push_str(body);
        }
        text.push_str(end);
    }
    let _ = out.write_all(text.as_bytes());
    0
}

/// Refuses a plan whose name is not an issue's name: it has no issue's file until point 15.
fn issue_name(name: &str, command: &str, err: &mut dyn Write) -> Result<(), i32> {
    if crate::issue::is_name(name) {
        return Ok(());
    }
    Err(crate::refusal::limit(
        err,
        command,
        format_args!(
            "the plan {} has no issue's file: its name is not an issue's name (letters, digits, '.', '_' and '-'), and until point 15 an issue's file is named by one. Nothing written.",
            crate::issue::shown(name)
        ),
    ))
}

/// `wb plan accept <name> [--no-commit]`: the acceptance of the plan's issue's file, as `wb accept` made it (point 13).
pub fn accept(root: &str, name: &str, commit: bool, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    const COMMAND: &str = "wb plan accept";
    if let Err(code) = live_plan(root, name, COMMAND, err) {
        return code;
    }
    if let Err(code) = issue_name(name, COMMAND, err) {
        return code;
    }
    crate::issue::accept_as(COMMAND, root, name, commit, out, err)
}

/// `wb accept <name>` where a plan of the name is there (#290): `Some(2)` with the words that name `wb plan accept`
/// where its issue's file is not accepted, by its hash and its sources; `None` where there is no plan of the name, or
/// it is accepted, and `wb accept` goes on as before.
pub fn accept_word(root: &str, name: &str, err: &mut dyn Write) -> Option<i32> {
    let plan = plan_of(root, name).filter(|plan| !plan.removed)?;
    let (file, _) = files_of(name);
    let accepted = plain(root, &file).map(|bytes| state_of(root, &plan, name, &bytes).is_ok()).unwrap_or(false);
    if accepted {
        return None;
    }
    let _ = writeln!(
        err,
        "wb accept: {} is a plan, and it is not accepted. A plan is accepted with `wb plan accept {}`; wb accept is the acceptance of the work (#282). Nothing written.",
        crate::issue::shown(name),
        name
    );
    Some(2)
}

/// `wb plan edit <name>`: opens the plan's issue's file in the editor git would use, and commits the edit alone.
pub fn edit(root: &str, name: &str, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    const COMMAND: &str = "wb plan edit";
    let limit = |err: &mut dyn Write, why: String| crate::refusal::limit(err, COMMAND, why);
    if let Err(code) = live_plan(root, name, COMMAND, err) {
        return code;
    }
    if let Err(code) = issue_name(name, COMMAND, err) {
        return code;
    }
    if let Err(why) = crate::starter::held(root) {
        return limit(err, format!("{}. Nothing opened, nothing written.", why));
    }
    let (file, _) = files_of(name);
    let Some(before) = plain(root, &file) else {
        return limit(err, format!("{} is not there, or is not a plain file: the planner writes it, with wb start planner {}. Nothing opened.", file, name));
    };
    // It is committed alone, on the issue's branch, from the file as `HEAD` has it: what `wb accept` checks before a commit.
    if let Err(crate::issue::NotReady::Fault(why) | crate::issue::NotReady::Guard(why)) = crate::issue::ready_to_commit(root, name) {
        return limit(err, format!("{} Nothing opened, nothing written.", why.trim_end()));
    }
    // The editor git would use: GIT_EDITOR, core.editor, VISUAL, EDITOR, then git's own default.
    let editor = match Command::new("git").arg("-C").arg(root).args(["var", "GIT_EDITOR"]).stdin(Stdio::null()).stderr(Stdio::piped()).output() {
        Ok(run) if run.status.success() => String::from_utf8_lossy(&run.stdout).trim_end_matches('\n').to_string(),
        Ok(run) => {
            let said = String::from_utf8_lossy(&run.stderr).lines().next().unwrap_or("").trim().to_string();
            return limit(err, format!("git var GIT_EDITOR gives no editor: {}. Nothing opened.", said));
        }
        Err(error) => return limit(err, format!("git could not be run: {}. Nothing opened.", error)),
    };
    let path = Path::new(root).join(&file);
    let _ = writeln!(out, "{}: opens {} in {}.", COMMAND, file, crate::issue::shown(&editor));
    let _ = out.flush();
    // As git runs it: by the shell, with the file as its argument.
    let ran = Command::new("/bin/sh").arg("-c").arg(format!("{} \"$@\"", editor)).arg(&editor).arg(&path).current_dir(root).status();
    match ran {
        Ok(status) if status.success() => {}
        Ok(status) => {
            let said = status.code().map_or_else(|| "a signal".to_string(), |code| code.to_string());
            return limit(err, format!("the editor ended with {}. Nothing committed: {} is left as the editor left it.", said, file));
        }
        Err(error) => return limit(err, format!("the editor could not be run: {}. Nothing committed.", error)),
    }
    let Some(after) = plain(root, &file) else {
        return limit(err, format!("{} is not a plain file after the editor. Nothing committed.", file));
    };
    if after == before {
        let _ = writeln!(out, "{}: nothing changed in {}. Nothing committed.", COMMAND, file);
        return 0;
    }
    if let Err(why) = crate::issue::read(&path) {
        let why = why.replace(&format!("{}/", root.trim_end_matches('/')), "");
        return limit(err, format!("{} no longer holds its form: {}. Nothing committed: the edit is left in the working tree.", file, why.trim_end_matches('.')));
    }
    let message = format!("Edit the issue's file ({})", name);
    let _ = writeln!(out, "{}: will commit {} alone, with the message \"{}\".", COMMAND, file, message);
    crate::interrupt::writing();
    if let Err(why) = git(root, &["commit", "-q", "--only", "-m", &message, "--", &file], None) {
        return limit(err, format!("the commit failed: {}. The edit is left in the working tree.", why));
    }
    let held = git_line(root, &["diff-tree", "--no-commit-id", "--name-only", "-r", "--root", "HEAD"], None).unwrap_or_default();
    let short = git_line(root, &["rev-parse", "--short", "HEAD"], None).unwrap_or_default();
    if held.lines().any(|path| path != file) {
        return limit(err, format!("the commit {} is not alone: a hook of git added files to it. Nothing more is done: undo or amend it as you decide.", short));
    }
    let _ = writeln!(out, "{}: committed {} alone ({}). An acceptance of the file before holds no longer.", COMMAND, file, short);
    0
}

/// `wb plan remove <name>`: a marker added after the plan's commit, every name moved to it, nothing rewritten.
/// `state` is this machine's state folder, where the views kept are.
pub fn remove(root: &str, name: &str, state: Option<&Path>, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    const COMMAND: &str = "wb plan remove";
    let limit = |err: &mut dyn Write, why: String| crate::refusal::limit(err, COMMAND, why);
    let plan = match live_plan(root, name, COMMAND, err) {
        Ok(plan) => plan,
        Err(code) => return code,
    };
    if let Err(why) = crate::starter::held(root) {
        return limit(err, format!("{}. Nothing written.", why));
    }
    // A role's work on it that is not done: a branch of the name in the flow, or a view kept on this machine.
    let pattern = match crate::prompt::pattern_of(root, "branch") {
        Ok(pattern) => pattern.unwrap_or_else(|| "issue-{issue}".to_string()),
        Err(why) => return limit(err, format!("{}. Nothing written.", why)),
    };
    let Some(default) = crate::status::default_branch(root) else {
        return limit(err, "no default branch was found, so whether work on the plan is not done cannot be told. Nothing written.".to_string());
    };
    let flow = match crate::status::in_flow(root, &default, &pattern) {
        Ok(flow) => flow,
        Err(why) => return limit(err, format!("{}. Nothing written.", why)),
    };
    let mut holding: Vec<String> = flow
        .iter()
        .filter_map(|full| full.strip_prefix("refs/heads/").or_else(|| full.strip_prefix("refs/remotes/origin/")).map(|branch| (full, branch)))
        .filter(|(_, branch)| crate::status::is_of(&pattern, name, branch))
        .map(|(full, _)| format!("the branch {} holds work that {} does not", full, default))
        .collect();
    if let (Some(state), true) = (state, crate::issue::is_name(name)) {
        let kept = crate::kept::count(root, state, name);
        if kept > 0 {
            holding.push(format!("this machine keeps {} view{} of it with work not taken back", kept, if kept == 1 { "" } else { "s" }));
        }
    }
    if !holding.is_empty() {
        return limit(err, format!("a role's work on the plan {} is not done: {}. Nothing written.", crate::issue::shown(name), holding.join("; ")));
    }
    let by = match (identity(root, "user.name"), identity(root, "user.email")) {
        (Ok(user), Ok(email)) => format!("{} <{}>", user, email),
        (Err(why), _) | (_, Err(why)) => return limit(err, format!("{}. Nothing written.", why)),
    };
    let seconds = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|time| time.as_secs()).unwrap_or(0);
    let marker = format!("by: {}\ntime: {}\n", by, crate::prompt::utc(seconds));
    let moved: Vec<String> = match git_line(root, &["for-each-ref", "--points-at", &plan.commit, "--format=%(refname)", PREFIX], None) {
        Ok(listed) => listed.lines().map(str::to_string).collect(),
        Err(why) => return limit(err, format!("{}. Nothing written.", why)),
    };
    let _ = writeln!(out, "It writes:");
    let _ = writeln!(out, "  the marker {} of the plan:", REMOVED);
    for line in marker.lines() {
        let _ = writeln!(out, "    {}", line);
    }
    let _ = writeln!(out, "  a tree of what the plan's commit holds, and {}", REMOVED);
    let _ = writeln!(out, "  a commit after {}", plan.commit);
    for full in &moved {
        let _ = writeln!(out, "  the ref {} moved to it from that commit", full);
    }
    crate::interrupt::writing();
    let failed = |err: &mut dyn Write, why: String| crate::refusal::limit(err, COMMAND, format_args!("{}. No ref is written.", why));
    let blob = match git_line(root, &["hash-object", "-t", "blob", "-w", "--stdin"], Some(marker.as_bytes())) {
        Ok(blob) => blob,
        Err(why) => return failed(err, why),
    };
    let mut top = match entries(root, &format!("{}^{{tree}}", plan.commit), &[REMOVED]) {
        Ok(top) => top,
        Err(why) => return failed(err, why),
    };
    top.push(format!("100644 blob {}\t{}", blob, REMOVED));
    let tree = match mktree(root, &top) {
        Ok(tree) => tree,
        Err(why) => return failed(err, why),
    };
    let message = format!("plan remove {}\n", name);
    let commit = match git_line(root, &["commit-tree", &tree, "-p", &plan.commit], Some(message.as_bytes())) {
        Ok(commit) => commit,
        Err(why) => return failed(err, why),
    };
    if let Err(why) = move_refs(root, &moved, None, &plan.commit, &commit) {
        return failed(err, why);
    }
    for full in &moved {
        let _ = writeln!(out, "Wrote {}: moved to commit {}.", full, commit);
    }
    0
}

/// The text of a plan for its planner (#290, decision 1): every source in the order it was added. One source whose
/// text is kept is that text as it is; otherwise each is under a line that names it, and a source not kept says so.
/// `None` where there is no plan of `name`, or it is removed.
pub(crate) fn planner_text(root: &str, name: &str) -> Result<Option<Vec<u8>>, String> {
    let Some(plan) = plan_of(root, name).filter(|plan| !plan.removed) else {
        return Ok(None);
    };
    let text_of = |hash: &str| git(root, &["cat-file", "blob", &format!("{}:sources/{}", plan.commit, hash)], None);
    if let [source] = plan.job.sources.as_slice() {
        if source.kept {
            return text_of(&source.hash).map(Some).map_err(|why| format!("the text of the plan {} cannot be read: {}", name, why));
        }
    }
    let mut text = Vec::new();
    for (at, source) in plan.job.sources.iter().enumerate() {
        let shown = crate::issue::shown(&source.source);
        if !source.kept {
            text.extend_from_slice(format!("--- source {}: {}, from {}: hash only: its text is not kept ---\n", at + 1, source.hash, shown).as_bytes());
            continue;
        }
        text.extend_from_slice(format!("--- source {}: {}, from {} ---\n", at + 1, source.hash, shown).as_bytes());
        let body = text_of(&source.hash).map_err(|why| format!("the text of the plan {} cannot be read: {}", name, why))?;
        text.extend_from_slice(&body);
        if !body.ends_with(b"\n") {
            text.push(b'\n');
        }
    }
    if text.len() as u64 > crate::bounded::TEXT_CAP {
        return Err(format!("the sources of the plan {} are over 1 MiB together", name));
    }
    Ok(Some(text))
}
