//! Confidentiality (`docs/revision.md`, point 11): the organisation's file, the level of a repository,
//! and whether a runner is cleared for the breakdown of a repository at its level (#93).
//!
//! The organisation's file is read from one path, fixed in `wb`. No option and no environment variable
//! changes it: a team cannot clear a runner for itself. A test gives its own path to `crate::prompt`.

use std::path::Path;

use crate::rules::{is_exact, EFFORTS};
use crate::yamlfile::{self, Document, Part, YamlError};

/// The organisation's file. It is laid out by the organisation's own means, outside every repository.
pub const ORGANISATION_FILE: &str = "/etc/watchbill/organisation.yml";

const RUNNER_FIELDS: [&str; 5] = ["tools", "model", "agreement", "up_to", "breakdown"];
const RUNNER_NEEDS: [&str; 4] = ["tools", "model", "agreement", "up_to"];
const BREAKDOWN_FIELDS: [&str; 3] = ["up_to", "model", "effort"];

/// What a runner is cleared for in the breakdown: the highest level and the model and effort it runs with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Breakdown {
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
    pub breakdown: Option<Breakdown>,
}

/// The organisation's file: the levels from the least to the most restrictive, and the runners.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Organisation {
    pub levels: Vec<String>,
    pub runners: Vec<Runner>,
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
    let mut breakdown = None;
    let inner = with(&place, "breakdown");
    if doc.has(&inner) {
        doc.only(&inner, &BREAKDOWN_FIELDS)?;
        doc.need(&inner, &BREAKDOWN_FIELDS)?;
        let cleared = level(doc, &with(&inner, "up_to"), levels)?;
        let rank = |name: &str| levels.iter().position(|level| level == name);
        if rank(&cleared) > rank(&up_to) {
            return Err(doc.fail(
                &with(&inner, "up_to"),
                &format!(
                    "the breakdown is cleared up to `{}`, above `{}`, the highest level the runner may be given at all",
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
        breakdown = Some(Breakdown { up_to: cleared, model, effort });
    }
    Ok(Runner { name: name.to_string(), tools, model, agreement, up_to, breakdown })
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
    let doc = yamlfile::load(path).map_err(|why| why.to_string())?;
    read_document(&doc).map_err(|why| why.to_string())
}

fn read_document(doc: &Document) -> Result<Organisation, YamlError> {
    doc.only(&[], &["levels", "runners"])?;
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
    Ok(Organisation { levels, runners })
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

/// What the breakdown is handed out with: the level in force, and the model and effort of the runner's breakdown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cleared {
    pub level: String,
    pub model: String,
    pub effort: String,
}

/// Whether `runner` is cleared for the breakdown of the repository at `root`, by the organisation's file at
/// `organisation`: the file is there, names the runner, the runner has `breakdown`, and the repository's
/// level is no higher than the breakdown's `up_to`. The refusal names the file, the runner, or both levels.
pub fn clear_breakdown(organisation: &Path, root: &Path, runner: &str) -> Result<Cleared, String> {
    let found = read(organisation)?;
    let level = repository_level(root, &found)?;
    let shown = organisation.display();
    let Some(named) = found.runners.iter().find(|one| one.name == runner) else {
        let names: Vec<&str> = found.runners.iter().map(|one| one.name.as_str()).collect();
        let have = if names.is_empty() { "It names none.".to_string() } else { format!("Its runners are: {}.", names.join(", ")) };
        return Err(format!("the organisation's file {} names no runner '{}'. {}", shown, runner.escape_debug(), have));
    };
    let Some(breakdown) = &named.breakdown else {
        return Err(format!(
            "the runner '{}' is not cleared for the breakdown: it has no `breakdown` in the organisation's file {}. No prompt is handed out.",
            runner, shown
        ));
    };
    if found.rank(&level) > found.rank(&breakdown.up_to) {
        return Err(format!(
            "the repository's level is `{}`, and the organisation's file {} clears the runner '{}' for the breakdown up to `{}` only. No prompt is handed out.",
            level, shown, runner, breakdown.up_to
        ));
    }
    Ok(Cleared { level, model: breakdown.model.clone(), effort: breakdown.effort.clone() })
}
