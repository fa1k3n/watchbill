//! `wb plan add` (#234; `docs/revision.md`, point 15): a job is taken in. Its text is stored in git as a blob, under the
//! hash of its content, and its name is a ref, `refs/watchbill/<name>`, that points at a commit whose tree holds
//! `job.yml` and the text of each source under `sources/<hash>`. A plan may have several sources, each under its own
//! hash (#290, the owner's decision 1): other text under a name that is there is a new commit after the one before.
//!
//! Git is the store: the objects are written with `hash-object`, `mktree` and `commit-tree`, and the ref is moved with
//! `update-ref` from the value it had. It says what it will write first, asks nothing of the network unless `--from`
//! runs a pattern, and only queues: it runs no prompt and starts no session. Here also the fetch line of the jobs that
//! `wb init` and `wb upgrade` add to the remote `origin`.

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
                Ok((commit, job)) if job.sources.iter().any(|source| source.hash == hash) => {
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
