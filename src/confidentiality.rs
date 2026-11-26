//! Confidentiality (`docs/revision.md`, point 11): the organisation's file, the level of a repository,
//! and whether a runner is cleared for the planner of a repository at its level (#93).
//!
//! The organisation's file is read from one path, fixed in `wb`. No option and no environment variable
//! changes it: a team cannot clear a runner for itself. A test gives its own path to `crate::prompt`.

use std::path::Path;

use crate::rules::{is_exact, EFFORTS};
use crate::yamlfile::{self, Document, Part, YamlError};

/// The organisation's file. It is laid out by the organisation's own means, outside every repository.
pub const ORGANISATION_FILE: &str = "/etc/watchbill/organisation.yml";

const RUNNER_FIELDS: [&str; 8] = ["tools", "model", "agreement", "up_to", "planner", "start", "binds", "reach"];
const RUNNER_NEEDS: [&str; 4] = ["tools", "model", "agreement", "up_to"];
const PLANNER_FIELDS: [&str; 3] = ["up_to", "model", "effort"];

/// The placeholders of a runner's `start` (#84): what `wb start <role>` fills in each word.
pub const START_PLACEHOLDERS: [&str; 3] = ["model", "effort", "prompt"];

/// What a runner is cleared for in the planner: the highest level and the model and effort it runs with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Planner {
    pub up_to: String,
    pub model: String,
    pub effort: String,
}

/// A place to run: where its tools run, which model answers, under which agreement, and the highest level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Runner {
    pub name: String,
    pub tools: String,
    pub model: String,
    pub agreement: String,
    pub up_to: String,
    pub planner: Option<Planner>,
    /// The command that starts a session on this runner, as a list of words with `{model}`, `{effort}` and
    /// `{prompt}` to fill (#84). `None` when the runner has none: `wb start <role>` then starts nothing on it.
    pub start: Option<Vec<String>>,
    /// The paths the runner needs for itself in a view (#126): its program, its settings, its login. None
    /// when the runner has no `binds`.
    pub binds: Vec<Bind>,
    /// The endpoints a session on this runner reaches from a view, through `wb`'s proxy (#195). None when the runner
    /// has no `reach`: the session reaches no network.
    pub reach: Vec<crate::reach::Endpoint>,
}

/// A path a runner needs for itself, put in the view at the same path (#126): read-only unless it is marked
/// writable. It starts with `/`, or with `~/` for the home folder of the person who runs `wb`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bind {
    pub path: String,
    pub writable: bool,
}

/// The organisation's file: the levels from the least to the most restrictive, the runners, and the runner
/// `wb start <role>` takes when none is given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Organisation {
    pub levels: Vec<String>,
    pub runners: Vec<Runner>,
    pub default: Option<String>,
    /// Whether `wb plan add --hash-only` may keep a job's hash and source without its text (#234): `jobs: hash_only:
    /// allowed`. With no `jobs`, it may not.
    pub hash_only: bool,
}

impl Organisation {
    /// Where a level stands in the list: the higher, the more restrictive.
    pub fn rank(&self, level: &str) -> Option<usize> {
        self.levels.iter().position(|name| name == level)
    }

    /// The most restrictive level.
    pub fn strictest(&self) -> &str {
        self.levels.last().map(String::as_str).unwrap_or("")
    }
}

fn at(parts: &[&str]) -> Vec<Part> {
    parts.iter().map(|part| Part::from(*part)).collect()
}

fn with(place: &[Part], part: impl Into<Part>) -> Vec<Part> {
    let mut longer = place.to_vec();
    longer.push(part.into());
    longer
}

/// A text at a place that must be one of the levels.
fn level(doc: &Document, place: &[Part], levels: &[String]) -> Result<String, YamlError> {
    let level = doc.text(place)?;
    if !levels.contains(&level) {
        return Err(doc.fail(place, &format!("`{}` is not one of the levels: {}", level, levels.join(", ")), false));
    }
    Ok(level)
}

fn read_runner(doc: &Document, name: &str, levels: &[String]) -> Result<Runner, YamlError> {
    let place = at(&["runners", name]);
    doc.only(&place, &RUNNER_FIELDS)?;
    doc.need(&place, &RUNNER_NEEDS)?;
    let tools = doc.text(&with(&place, "tools"))?;
    let model = doc.text(&with(&place, "model"))?;
    let agreement = doc.text(&with(&place, "agreement"))?;
    let up_to = level(doc, &with(&place, "up_to"), levels)?;
    let mut planner = None;
    let inner = with(&place, "planner");
    if doc.has(&inner) {
        doc.only(&inner, &PLANNER_FIELDS)?;
        doc.need(&inner, &PLANNER_FIELDS)?;
        let cleared = level(doc, &with(&inner, "up_to"), levels)?;
        let rank = |name: &str| levels.iter().position(|level| level == name);
        if rank(&cleared) > rank(&up_to) {
            return Err(doc.fail(
                &with(&inner, "up_to"),
                &format!(
                    "the planner is cleared up to `{}`, above `{}`, the highest level the runner may be given at all",
                    cleared, up_to
                ),
                false,
            ));
        }
        let model = doc.text(&with(&inner, "model"))?;
        if !is_exact(&model) {
            return Err(doc.fail(
                &with(&inner, "model"),
                &format!(
                    "`{}` is not an exact model ID: it needs a name and a version, such as claude-opus-5-5, and never `latest`",
                    model
                ),
                false,
            ));
        }
        let effort = doc.text(&with(&inner, "effort"))?;
        if !EFFORTS.contains(&effort.as_str()) {
            return Err(doc.fail(
                &with(&inner, "effort"),
                &format!("`{}` is not an effort; it is one of: {}", effort, EFFORTS.join(", ")),
                false,
            ));
        }
        planner = Some(Planner { up_to: cleared, model, effort });
    }
    let mut start = None;
    let words = with(&place, "start");
    if doc.has(&words) {
        start = Some(read_start(doc, &words)?);
    }
    let mut binds = Vec::new();
    let list = with(&place, "binds");
    if doc.has(&list) {
        binds = read_binds(doc, &list)?;
    }
    let mut reach = Vec::new();
    let list = with(&place, "reach");
    if doc.has(&list) {
        reach = read_reach(doc, &list)?;
    }
    Ok(Runner { name: name.to_string(), tools, model, agreement, up_to, planner, start, binds, reach })
}

/// A runner's `reach` (#195): a list of texts, each `<host>:<port>` or `http://<host>:<port>` (`reach::entry`, #336),
/// each host and port once. An empty list is no `reach`.
fn read_reach(doc: &Document, place: &[Part]) -> Result<Vec<crate::reach::Endpoint>, YamlError> {
    let texts = doc.texts(place, true)?;
    let mut reach: Vec<crate::reach::Endpoint> = Vec::new();
    for (index, text) in texts.iter().enumerate() {
        let here = with(place, index);
        let endpoint = crate::reach::entry(text).map_err(|why| doc.fail(&here, &format!("the endpoint '{}' of `reach` {}", text.escape_debug(), why), false))?;
        if reach.iter().any(|one| one.same_place(&endpoint)) {
            return Err(doc.fail(&here, &format!("the endpoint {} is written twice in `reach`; each is written once", endpoint), false));
        }
        reach.push(endpoint);
    }
    Ok(reach)
}

/// A runner's `binds` (#126): a list, each a path, read-only, or `{writable: <path>}`. A path starts with `/`,
/// or with `~/` for the home folder, has no `..` and no control character, and is not the root or the home
/// folder itself.
fn read_binds(doc: &Document, place: &[Part]) -> Result<Vec<Bind>, YamlError> {
    if !doc.has(&with(place, 0)) {
        // An empty list, or a value that is not a list: the loader refuses it as it refuses any list of texts.
        doc.texts(place, true)?;
        return Ok(Vec::new());
    }
    let mut binds = Vec::new();
    let mut index = 0;
    while doc.has(&with(place, index)) {
        let here = with(place, index);
        let (path, at, writable) = if doc.table(&here).is_ok() {
            doc.only(&here, &["writable"])?;
            doc.need(&here, &["writable"])?;
            let at = with(&here, "writable");
            (doc.text(&at)?, at, true)
        } else {
            (doc.text(&here)?, here.clone(), false)
        };
        if let Some(why) = bind_fault(&path) {
            return Err(doc.fail(&at, &format!("the bind '{}' {}", path, why), false));
        }
        binds.push(Bind { path, writable });
        index += 1;
    }
    Ok(binds)
}

/// What is wrong with the path of a bind, if anything.
fn bind_fault(path: &str) -> Option<&'static str> {
    if path.chars().any(|c| c.is_control() || c == '\u{2028}' || c == '\u{2029}') {
        return Some("holds a control character or a line break");
    }
    let rest = match (path.strip_prefix("~/"), path.strip_prefix('/')) {
        (Some(rest), _) | (None, Some(rest)) => rest,
        _ => return Some("is not a path from the root, `/…`, or from the home folder, `~/…`"),
    };
    let parts: Vec<&str> = rest.split('/').filter(|part| !part.is_empty() && *part != ".").collect();
    if parts.contains(&"..") {
        return Some("holds `..`");
    }
    if parts.is_empty() {
        return Some("is the root or the home folder itself, and a view holds neither");
    }
    None
}

/// A runner's `start` (#84): a list of one word or more, each a text of one plain line, with no character
/// that is not printed. A placeholder in a word is one of `START_PLACEHOLDERS`, and none stands in the
/// first word: the program is the organisation's to name, never filled in.
fn read_start(doc: &Document, place: &[Part]) -> Result<Vec<String>, YamlError> {
    let words = doc.texts(place, false)?;
    for (index, word) in words.iter().enumerate() {
        let here = with(place, index);
        if word.is_empty() {
            return Err(doc.fail(&here, "a word of `start` is empty", false));
        }
        if word.chars().any(|c| c.is_control() || c == '\u{2028}' || c == '\u{2029}') {
            return Err(doc.fail(&here, "a word of `start` holds a control character or a line break", false));
        }
        for name in crate::prompt::placeholder_names(word) {
            if !START_PLACEHOLDERS.contains(&name) {
                return Err(doc.fail(
                    &here,
                    &format!("the placeholder {{{}}} is not known in `start`; the known ones are {{model}}, {{effort}} and {{prompt}}", name),
                    false,
                ));
            }
            if index == 0 {
                return Err(doc.fail(
                    &here,
                    &format!("the first word of `start` is the program, and holds no placeholder, such as {{{}}}", name),
                    false,
                ));
            }
        }
    }
    Ok(words)
}

/// Reads and checks the organisation's file at `path`, through the one loader. A file that is not there
/// is refused with its path: without it no runner is cleared. A fault is refused with the place named.
pub fn read(path: &Path) -> Result<Organisation, String> {
    if !path.exists() {
        return Err(format!(
            "there is no organisation's file {}. Without it no runner is cleared, and no prompt is handed out (docs/revision.md, point 11).",
            path.display()
        ));
    }
    let doc = yamlfile::load_following(path).map_err(|why| why.to_string())?;
    read_document(&doc).map_err(|why| why.to_string())
}

fn read_document(doc: &Document) -> Result<Organisation, YamlError> {
    doc.only(&[], &["levels", "runners", "default", "jobs"])?;
    doc.need(&[], &["levels", "runners"])?;
    let levels = doc.texts(&at(&["levels"]), false)?;
    for (index, name) in levels.iter().enumerate() {
        if levels[..index].contains(name) {
            return Err(doc.fail(
                &with(&at(&["levels"]), index),
                &format!("the level `{}` is written twice; each level is written once, from the least to the most restrictive", name),
                false,
            ));
        }
    }
    let mut runners = Vec::new();
    for name in doc.table(&at(&["runners"]))? {
        runners.push(read_runner(doc, &name, &levels)?);
    }
    // The runner `wb start <role>` takes when none is given (#84): one of the runners of the file.
    let mut default = None;
    let place = at(&["default"]);
    if doc.has(&place) {
        let name = doc.text(&place)?;
        if !runners.iter().any(|runner| runner.name == name) {
            let names: Vec<&str> = runners.iter().map(|runner| runner.name.as_str()).collect();
            let have = if names.is_empty() { "it names none".to_string() } else { format!("its runners are: {}", names.join(", ")) };
            return Err(doc.fail(&place, &format!("the default `{}` is not a runner of this file; {}", name, have), false));
        }
        default = Some(name);
    }
    // What `wb plan add` may do with a job (#234): only `hash_only`, which is `allowed` or `refused`.
    let mut hash_only = false;
    let jobs = at(&["jobs"]);
    if doc.has(&jobs) {
        doc.table(&jobs)?;
        doc.only(&jobs, &["hash_only"])?;
        doc.need(&jobs, &["hash_only"])?;
        let place = at(&["jobs", "hash_only"]);
        hash_only = match doc.text(&place)?.as_str() {
            "allowed" => true,
            "refused" => false,
            other => return Err(doc.fail(&place, &format!("`{}` is not `allowed` or `refused`", other), false)),
        };
    }
    Ok(Organisation { levels, runners, default, hash_only })
}

/// The level of the repository at `root`: the line `confidentiality:` of `.watchbill/watchbill.yml`, and
/// the most restrictive level where it is missing. A level that is not on the organisation's list is
/// refused by name, and so are `levels` and `runners`: they are the organisation's.
pub fn repository_level(root: &Path, organisation: &Organisation) -> Result<String, String> {
    let config = root.join(crate::init::CONFIG_FOLDER).join(crate::init::CONFIG_FILE);
    let doc = yamlfile::load(&config).map_err(|why| why.to_string())?;
    for key in ["levels", "runners"] {
        let place = at(&[key]);
        if doc.has(&place) {
            return Err(doc
                .fail(
                    &place,
                    "the levels and the runners are the organisation's, in its own file, and a repository cannot clear a runner for itself. Take the key out",
                    true,
                )
                .to_string());
        }
    }
    let place = at(&["confidentiality"]);
    if !doc.has(&place) {
        return Ok(organisation.strictest().to_string());
    }
    let level = doc.text(&place).map_err(|why| why.to_string())?;
    if organisation.rank(&level).is_none() {
        return Err(doc
            .fail(
                &place,
                &format!("the level `{}` is not one of the organisation's levels: {}", level, organisation.levels.join(", ")),
                false,
            )
            .to_string());
    }
    Ok(level)
}

/// What the planner is handed out with: the level in force, and the model and effort of the runner's planner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cleared {
    pub level: String,
    pub model: String,
    pub effort: String,
}

/// Whether `runner` is cleared for the planner of the repository at `root`, by the organisation's file at
/// `organisation`: the file is there, names the runner, the runner has `planner`, and the repository's
/// level is no higher than the planner's `up_to`. The refusal names the file, the runner, or both levels.
pub fn clear_planner(organisation: &Path, root: &Path, runner: &str) -> Result<Cleared, String> {
    let found = read(organisation)?;
    let level = repository_level(root, &found)?;
    let shown = organisation.display();
    let Some(named) = found.runners.iter().find(|one| one.name == runner) else {
        let names: Vec<&str> = found.runners.iter().map(|one| one.name.as_str()).collect();
        let have = if names.is_empty() { "It names none.".to_string() } else { format!("Its runners are: {}.", names.join(", ")) };
        return Err(format!("the organisation's file {} names no runner '{}'. {}", shown, runner.escape_debug(), have));
    };
    let Some(planner) = &named.planner else {
        return Err(format!(
            "the runner '{}' is not cleared for the planner: it has no `planner` in the organisation's file {}. No prompt is handed out.",
            runner, shown
        ));
    };
    if found.rank(&level) > found.rank(&planner.up_to) {
        return Err(format!(
            "the repository's level is `{}`, and the organisation's file {} clears the runner '{}' for the planner up to `{}` only. No prompt is handed out.",
            level, shown, runner, planner.up_to
        ));
    }
    Ok(Cleared { level, model: planner.model.clone(), effort: planner.effort.clone() })
}
