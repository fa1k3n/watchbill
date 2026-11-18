//! The rule table and the model table of the revision: read, and checked.
//!
//! This module only reads and checks. Nothing here acts on a rule. It is the only part of `wb`
//! that names the two tables, and it reads them through `yamlfile`.

use std::collections::BTreeMap;
use std::path::Path;

use crate::word::is_word;
use crate::yamlfile::{self, Document, Part, YamlError};

/// The roles of the revision. The officer has rows and no prompt yet. The breakdown has a prompt
/// from 0.0.5 (#75).
pub const ROLES: [&str; 6] = ["breakdown", "tester", "build", "review", "fix", "officer"];
/// The roles with a prompt and no row for a vendor's runner in the model table: they run on a local
/// runner, or where a person has cleared another for them (docs/revision.md, point 5).
pub const LOCAL_ONLY: [&str; 1] = ["breakdown"];
pub const ROLE_FIELDS: [&str; 6] = ["step", "produces", "reads", "may", "may_not_also_hold", "prompt"];
pub const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];

/// The folder of the revision, fixed when `wb` is built (`build.rs`): `revision/` in the folder
/// of `manifest::PACKAGE_FOLDER`.
pub const REVISION_FOLDER: &str = concat!(env!("WB_SHARE_FOLDER"), "/revision");
pub const RULES_FILE: &str = concat!(env!("WB_SHARE_FOLDER"), "/revision/rules.yml");
pub const MODELS_FILE: &str = concat!(env!("WB_SHARE_FOLDER"), "/revision/models.yml");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Role {
    pub name: String,
    pub step: String,
    pub produces: String,
    pub reads: Vec<String>,
    pub may: Vec<String>,
    pub may_not_also_hold: Vec<String>,
    /// A file below the revision folder, or `none`.
    pub prompt: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
    pub model: String,
    pub effort: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rules {
    pub review_rounds: i64,
    /// How many times a session is asked for a last message that has its fields.
    pub last_message_asks: i64,
    pub always_to_a_person: Vec<String>,
    pub classed_code_verifier: String,
    pub officer_queues: Vec<String>,
    /// The roles, in order, that a requirement a person adds to a change under review goes to.
    pub added_requirement_goes_to: Vec<String>,
    pub roles: BTreeMap<String, Role>,
    pub models: BTreeMap<String, Model>,
    /// The models of local runners, by name. Empty until the first local runner exists (point 11).
    pub local_models: Vec<String>,
}

fn at(parts: &[&str]) -> Vec<Part> {
    parts.iter().map(|part| Part::from(*part)).collect()
}

fn with(place: &[Part], part: impl Into<Part>) -> Vec<Part> {
    let mut longer = place.to_vec();
    longer.push(part.into());
    longer
}

/// `merg` at the start of a word: merge, merges, merged, merging.
fn says_merge(entry: &str) -> bool {
    // Each character of the entry in lower case, beside the character it comes from. A character
    // before `merg` is a letter only when both are: a capital that an older Unicode does not know
    // stays itself in the Python of Enterprise Linux 9, and is no letter there.
    let lower: Vec<(char, char)> = entry.chars().flat_map(|c| c.to_lowercase().map(move |l| (l, c))).collect();
    let merg = ['m', 'e', 'r', 'g'];
    (0..lower.len()).any(|at| {
        lower[at..].len() >= merg.len()
            && lower[at..at + merg.len()].iter().map(|(l, _)| *l).eq(merg)
            && !(at > 0 && is_word(lower[at - 1].0) && is_word(lower[at - 1].1))
    })
}

fn role(doc: &Document, name: &str) -> Result<Role, YamlError> {
    let place = at(&["roles", name]);
    doc.only(&place, &ROLE_FIELDS)?;
    doc.need(&place, &ROLE_FIELDS)?;
    let prompt = doc.text(&with(&place, "prompt"))?;
    if prompt != "none" && (!prompt.ends_with(".md") || prompt.starts_with('/') || prompt.split('/').any(|part| part == "..")) {
        return Err(doc.fail(
            &with(&place, "prompt"),
            &format!("`none` or a file below the revision folder, such as roles/{}.md, is expected", name),
            false,
        ));
    }
    let row = Role {
        name: name.to_string(),
        step: doc.text(&with(&place, "step"))?,
        produces: doc.text(&with(&place, "produces"))?,
        reads: doc.texts(&with(&place, "reads"), false)?,
        may: doc.texts(&with(&place, "may"), false)?,
        may_not_also_hold: doc.texts(&with(&place, "may_not_also_hold"), false)?,
        prompt,
    };
    for (index, entry) in row.may.iter().enumerate() {
        if says_merge(entry) {
            return Err(doc.fail(
                &with(&with(&place, "may"), index),
                &format!("{} may not merge: no agent does, and a person is the one who merges", name),
                false,
            ));
        }
    }
    for (index, other) in row.may_not_also_hold.iter().enumerate() {
        let here = with(&with(&place, "may_not_also_hold"), index);
        if other == name {
            return Err(doc.fail(&here, &format!("{} cannot exclude itself", name), false));
        }
        if row.may_not_also_hold[..index].contains(other) {
            return Err(doc.fail(&here, &format!("`{}` is written twice", other), false));
        }
        if !ROLES.contains(&other.as_str()) {
            return Err(doc.fail(
                &here,
                &format!("{} may not also hold `{}`, which is not one of the roles: {}", name, other, ROLES.join(", ")),
                false,
            ));
        }
    }
    Ok(row)
}

fn read_rules(path: &Path) -> Result<Rules, YamlError> {
    let doc = yamlfile::load(path)?;
    let top = [
        "review_rounds",
        "last_message_asks",
        "always_to_a_person",
        "classed_code",
        "officer_queues",
        "added_requirement_goes_to",
        "roles",
    ];
    doc.only(&[], &top)?;
    doc.need(&[], &top)?;
    let rounds = doc.whole(&at(&["review_rounds"]), Some(1))?;
    let asks = doc.whole(&at(&["last_message_asks"]), Some(1))?;
    let person = doc.texts(&at(&["always_to_a_person"]), false)?;
    if person.iter().enumerate().any(|(index, name)| person[..index].contains(name)) {
        return Err(doc.fail(&at(&["always_to_a_person"]), "a name is written twice", false));
    }
    if !person.iter().any(|name| name == "merge") {
        return Err(doc.fail(
            &at(&["always_to_a_person"]),
            "a merge always goes to a person, so `merge` is expected in this list",
            false,
        ));
    }
    doc.only(&at(&["classed_code"]), &["verifier"])?;
    doc.need(&at(&["classed_code"]), &["verifier"])?;
    let verifier = doc.text(&at(&["classed_code", "verifier"]))?;
    if verifier != "person" {
        return Err(doc.fail(
            &at(&["classed_code", "verifier"]),
            "a change that touches classed code needs a person as its verifier, so `person` is expected",
            false,
        ));
    }
    let queues = doc.texts(&at(&["officer_queues"]), true)?;
    let added = doc.texts(&at(&["added_requirement_goes_to"]), false)?;
    doc.only(&at(&["roles"]), &ROLES)?;
    doc.need(&at(&["roles"]), &ROLES)?;
    let mut roles = BTreeMap::new();
    for name in ROLES {
        roles.insert(name.to_string(), role(&doc, name)?);
    }
    // "May not also hold" holds both ways.
    for name in ROLES {
        for other in &roles[name].may_not_also_hold {
            if !roles[other].may_not_also_hold.iter().any(|back| back == name) {
                return Err(doc.fail(
                    &at(&["roles", name, "may_not_also_hold"]),
                    &format!("{} may not also hold {}, but {} does not say the same of {}", name, other, other, name),
                    false,
                ));
            }
        }
    }
    // The officer holds no other role on an issue it has handed out.
    for other in ROLES {
        if other != "officer" && !roles["officer"].may_not_also_hold.iter().any(|name| name == other) {
            return Err(doc.fail(
                &at(&["roles", "officer", "may_not_also_hold"]),
                &format!("the officer holds no other role, so `{}` is expected here", other),
                false,
            ));
        }
    }
    // An added requirement goes to roles that a session can hold, each once.
    let with_prompt: Vec<&str> = ROLES.iter().copied().filter(|name| roles[*name].prompt != "none").collect();
    for (index, name) in added.iter().enumerate() {
        let here = with(&at(&["added_requirement_goes_to"]), index);
        if added[..index].contains(name) {
            return Err(doc.fail(&here, &format!("`{}` is written twice", name), false));
        }
        if !with_prompt.contains(&name.as_str()) {
            return Err(doc.fail(
                &here,
                &format!("`{}` is not a role with a prompt; those are: {}", name, with_prompt.join(", ")),
                false,
            ));
        }
    }
    Ok(Rules {
        review_rounds: rounds,
        last_message_asks: asks,
        always_to_a_person: person,
        classed_code_verifier: verifier,
        officer_queues: queues,
        added_requirement_goes_to: added,
        roles,
        models: BTreeMap::new(),
        local_models: Vec::new(),
    })
}

/// A name and a version: lower-case letters and digits in parts joined by `-`, the first part
/// starting with a letter, the last a number, and no part `latest`.
fn is_exact(model: &str) -> bool {
    let parts: Vec<&str> = model.split('-').collect();
    let alnum = |part: &&str| !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    parts.len() >= 2
        && parts[0].starts_with(|c: char| c.is_ascii_lowercase())
        && parts.iter().all(alnum)
        && parts[parts.len() - 1].chars().all(|c| c.is_ascii_digit())
        && !parts.contains(&"latest")
}

/// The model rows of the roles, and the names under `local_models`.
type Models = (BTreeMap<String, Model>, Vec<String>);

fn read_models(path: &Path, roles: &BTreeMap<String, Role>) -> Result<Models, YamlError> {
    let doc = yamlfile::load(path)?;
    doc.only(&[], &["models", "local_models"])?;
    doc.need(&[], &["models", "local_models"])?;
    let local = doc.table(&at(&["local_models"]))?;
    if !local.is_empty() {
        return Err(doc.fail(
            &at(&["local_models"]),
            "the place for the models of local runners stays empty until the first local runner exists (docs/revision.md, points 5 and 11)",
            false,
        ));
    }
    let given = doc.table(&at(&["models"]))?;
    let with_prompt: Vec<&str> = ROLES
        .iter()
        .copied()
        .filter(|name| roles[*name].prompt != "none" && !LOCAL_ONLY.contains(name))
        .collect();
    for name in &given {
        if LOCAL_ONLY.contains(&name.as_str()) {
            return Err(doc.fail(
                &at(&["models", name]),
                &format!(
                    "{} has no row for a vendor's runner: it runs on a local runner, or on one a person has cleared for it (docs/revision.md, point 5)",
                    name
                ),
                false,
            ));
        }
        if !ROLES.contains(&name.as_str()) {
            return Err(doc.fail(
                &at(&["models", name]),
                &format!("`{}` is not one of the roles: {}", name, ROLES.join(", ")),
                false,
            ));
        }
        if !with_prompt.contains(&name.as_str()) {
            return Err(doc.fail(&at(&["models", name]), &format!("{} has no prompt, so it has no row here", name), false));
        }
    }
    for name in &with_prompt {
        if !given.iter().any(|row| row == name) {
            return Err(doc.fail(&at(&["models"]), &format!("{} has a prompt but no row here", name), false));
        }
    }
    let mut models = BTreeMap::new();
    for name in with_prompt {
        let place = at(&["models", name]);
        doc.only(&place, &["model", "effort"])?;
        doc.need(&place, &["model", "effort"])?;
        let model = doc.text(&with(&place, "model"))?;
        if !is_exact(&model) {
            return Err(doc.fail(
                &with(&place, "model"),
                &format!(
                    "`{}` is not an exact model ID: it needs a name and a version, such as claude-sonnet-5-5, and never `latest`",
                    model
                ),
                false,
            ));
        }
        let effort = doc.text(&with(&place, "effort"))?;
        if !EFFORTS.contains(&effort.as_str()) {
            return Err(doc.fail(
                &with(&place, "effort"),
                &format!("`{}` is not an effort; it is one of: {}", effort, EFFORTS.join(", ")),
                false,
            ));
        }
        models.insert(name.to_string(), Model { model, effort });
    }
    Ok((models, local))
}

/// Reads and checks both tables. A table that is wrong is refused, naming the file and the place.
pub fn load(rules_file: impl AsRef<Path>, models_file: impl AsRef<Path>) -> Result<Rules, YamlError> {
    let mut table = read_rules(rules_file.as_ref())?;
    (table.models, table.local_models) = read_models(models_file.as_ref(), &table.roles)?;
    Ok(table)
}

/// Reads and checks the two tables of the revision `wb` was built with.
pub fn load_revision() -> Result<Rules, YamlError> {
    load(RULES_FILE, MODELS_FILE)
}
