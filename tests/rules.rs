//! The rule table and the model table of the first revision, and the check that reads them.
//! Every test of tests/test_rules.py is here under the name it has, and then the tests of #23.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use watchbill::rules;

const ROLES: [&str; 6] = ["breakdown", "tester", "build", "review", "fix", "officer"];
const WITH_PROMPT: [&str; 5] = ["breakdown", "tester", "build", "review", "fix"];
// The roles with a row for a vendor's runner in the model table: all with a prompt but the breakdown (#75).
const WITH_MODEL: [&str; 4] = ["tester", "build", "review", "fix"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// A value as the Python tests build it, written out as `json.dumps(value, indent=2)` writes it.
/// JSON is YAML, so the tables are written from it.
#[derive(Clone, Debug)]
enum J {
    S(String),
    I(i64),
    F(f64),
    B(bool),
    Null,
    L(Vec<J>),
    M(Vec<(String, J)>),
}

fn s(text: &str) -> J {
    J::S(text.to_string())
}

fn texts(items: &[&str]) -> J {
    J::L(items.iter().map(|item| s(item)).collect())
}

fn map(entries: Vec<(&str, J)>) -> J {
    J::M(entries.into_iter().map(|(key, value)| (key.to_string(), value)).collect())
}

impl J {
    fn get(&mut self, key: &str) -> &mut J {
        match self {
            J::M(entries) => &mut entries.iter_mut().find(|(name, _)| name == key).expect(key).1,
            _ => panic!("not a mapping: {}", key),
        }
    }

    /// Sets a key: in its place when it is there, at the end when it is not, as a Python dict does.
    fn set(&mut self, key: &str, value: J) {
        match self {
            J::M(entries) => match entries.iter_mut().find(|(name, _)| name == key) {
                Some(entry) => entry.1 = value,
                None => entries.push((key.to_string(), value)),
            },
            _ => panic!("not a mapping: {}", key),
        }
    }

    fn del(&mut self, key: &str) {
        match self {
            J::M(entries) => {
                let before = entries.len();
                entries.retain(|(name, _)| name != key);
                assert_eq!(entries.len() + 1, before, "{}", key);
            }
            _ => panic!("not a mapping: {}", key),
        }
    }

    fn list(&mut self) -> &mut Vec<J> {
        match self {
            J::L(items) => items,
            _ => panic!("not a list"),
        }
    }

    fn texts(&self) -> Vec<String> {
        match self {
            J::L(items) => items
                .iter()
                .map(|item| match item {
                    J::S(text) => text.clone(),
                    _ => panic!("not a text"),
                })
                .collect(),
            _ => panic!("not a list"),
        }
    }

    fn dumps(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out
    }

    fn write(&self, out: &mut String, level: usize) {
        let pad = |level: usize| " ".repeat(2 * level);
        match self {
            J::S(text) => {
                out.push('"');
                for c in text.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        c if (c as u32) < 0x20 || (c as u32) > 0x7e => out.push_str(&format!("\\u{:04x}", c as u32)),
                        c => out.push(c),
                    }
                }
                out.push('"');
            }
            J::I(number) => out.push_str(&number.to_string()),
            J::F(number) => out.push_str(&format!("{:?}", number)),
            J::B(truth) => out.push_str(if *truth { "true" } else { "false" }),
            J::Null => out.push_str("null"),
            J::L(items) if items.is_empty() => out.push_str("[]"),
            J::M(entries) if entries.is_empty() => out.push_str("{}"),
            J::L(items) => {
                out.push_str("[\n");
                for (index, item) in items.iter().enumerate() {
                    out.push_str(&pad(level + 1));
                    item.write(out, level + 1);
                    out.push_str(if index + 1 < items.len() { ",\n" } else { "\n" });
                }
                out.push_str(&pad(level));
                out.push(']');
            }
            J::M(entries) => {
                out.push_str("{\n");
                for (index, (key, value)) in entries.iter().enumerate() {
                    out.push_str(&pad(level + 1));
                    J::S(key.clone()).write(out, level + 1);
                    out.push_str(": ");
                    value.write(out, level + 1);
                    out.push_str(if index + 1 < entries.len() { ",\n" } else { "\n" });
                }
                out.push_str(&pad(level));
                out.push('}');
            }
        }
    }
}

fn role_row(name: &str) -> J {
    let others: Vec<&str> = ROLES.iter().copied().filter(|other| *other != name).collect();
    map(vec![
        ("step", s(&format!("a step of {}", name))),
        ("produces", s(&format!("what {} produces", name))),
        ("reads", texts(&["a thing to read"])),
        ("may", texts(&["a thing to do"])),
        ("may_not_also_hold", texts(&others)),
        ("prompt", if WITH_PROMPT.contains(&name) { s(&format!("roles/{}.md", name)) } else { s("none") }),
    ])
}

fn good_rules() -> J {
    map(vec![
        ("review_rounds", J::I(2)),
        ("last_message_asks", J::I(2)),
        (
            "always_to_a_person",
            texts(&[
                "change_to_an_existing_test",
                "change_under_watchbill_folder",
                "verification_of_classed_code",
                "finding_open_after_last_round",
                "merge",
            ]),
        ),
        ("classed_code", map(vec![("verifier", s("person"))])),
        ("officer_queues", J::L(vec![])),
        ("added_requirement_goes_to", texts(&["tester", "build"])),
        ("roles", J::M(ROLES.iter().map(|name| (name.to_string(), role_row(name))).collect())),
    ])
}

fn good_models() -> J {
    let mut models = J::M(vec![]);
    for name in WITH_MODEL {
        models.set(name, map(vec![("model", s("claude-sonnet-5-5")), ("effort", s("medium"))]));
    }
    models.set("review", map(vec![("model", s("claude-opus-5-5")), ("effort", s("high"))]));
    map(vec![("models", models), ("local_models", J::M(vec![]))])
}

/// Two tables in a folder of their own, written from `rules` and `models` unless a text is given.
struct Tables {
    folder: common::Folder,
    rules: J,
    models: J,
}

impl Tables {
    fn new() -> Tables {
        Tables { folder: common::folder_named("revision"), rules: good_rules(), models: good_models() }
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.folder.path.join(name);
        fs::write(&path, text).expect("the table could not be written");
        path
    }

    fn paths_with(&self, rules_text: Option<&str>, models_text: Option<&str>) -> (PathBuf, PathBuf) {
        let rules_text = rules_text.map(str::to_string).unwrap_or_else(|| self.rules.dumps() + "\n");
        let models_text = models_text.map(str::to_string).unwrap_or_else(|| self.models.dumps() + "\n");
        (self.write("rules.yml", &rules_text), self.write("models.yml", &models_text))
    }

    fn load_with(&self, rules_text: Option<&str>, models_text: Option<&str>) -> Result<rules::Rules, String> {
        let (rules_path, models_path) = self.paths_with(rules_text, models_text);
        rules::load(&rules_path, &models_path).map_err(|error| error.to_string())
    }

    fn load(&self) -> rules::Rules {
        match self.load_with(None, None) {
            Ok(table) => table,
            Err(message) => panic!("the tables were refused: {}", message),
        }
    }

    fn refused_with(&self, rules_text: Option<&str>, models_text: Option<&str>) -> String {
        match self.load_with(rules_text, models_text) {
            Ok(_) => panic!("the tables were read"),
            Err(message) => {
                assert!(!message.contains("panicked"), "{}", message);
                message
            }
        }
    }

    fn refused(&self) -> String {
        self.refused_with(None, None)
    }

    fn role(&mut self, name: &str) -> &mut J {
        self.rules.get("roles").get(name)
    }
}

fn first_revision() -> rules::Rules {
    match rules::load_revision() {
        Ok(table) => table,
        Err(error) => panic!("the tables of the revision were refused: {}", error),
    }
}

fn sorted(mut items: Vec<String>) -> Vec<String> {
    items.sort();
    items
}

fn sorted_names(names: &[&str]) -> Vec<String> {
    sorted(names.iter().map(|name| name.to_string()).collect())
}

/// Every file below `folder` whose name ends in `ending`.
fn files(folder: &Path, ending: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut todo = vec![folder.to_path_buf()];
    while let Some(next) = todo.pop() {
        for entry in fs::read_dir(&next).expect("the folder could not be read") {
            let path = entry.expect("the folder could not be read").path();
            if path.is_dir() {
                todo.push(path);
            } else if path.to_string_lossy().ends_with(ending) {
                found.push(path);
            }
        }
    }
    found
}

fn names_a_table(text: &str) -> bool {
    text.contains("rules.yml") || text.contains("models.yml")
}

// TestFirstRevision

#[test]
fn test_WB_RULE_39_the_rule_table_of_the_package_has_the_six_roles() {
    assert_eq!(sorted(first_revision().roles.keys().cloned().collect()), sorted_names(&ROLES));
}

#[test]
fn test_WB_RULE_39_the_files_of_the_package_are_in_the_revision_folder() {
    let revision = root().join("share").join("watchbill").join("revision");
    assert_eq!(Path::new(rules::RULES_FILE), revision.join("rules.yml"));
    assert_eq!(Path::new(rules::MODELS_FILE), revision.join("models.yml"));
    assert!(Path::new(rules::RULES_FILE).is_file());
    assert!(Path::new(rules::MODELS_FILE).is_file());
}

#[test]
fn test_WB_RULE_2_every_role_has_every_field_with_something_in_it() {
    for (name, role) in first_revision().roles {
        for (field, value) in [("step", &role.step), ("produces", &role.produces), ("prompt", &role.prompt)] {
            assert!(!value.trim().is_empty(), "{} {}", name, field);
        }
        for (field, value) in [("reads", &role.reads), ("may", &role.may), ("may_not_also_hold", &role.may_not_also_hold)] {
            assert!(!value.is_empty(), "{} {}", name, field);
        }
    }
}

#[test]
fn test_WB_RULE_2_the_roles_with_a_prompt_name_a_file_in_roles() {
    let table = first_revision();
    for name in WITH_PROMPT {
        assert_eq!(table.roles[name].prompt, format!("roles/{}.md", name));
    }
}

#[test]
fn test_WB_RULE_2_the_roles_that_may_not_also_hold_are_each_other() {
    for (name, role) in first_revision().roles {
        let others: Vec<&str> = ROLES.iter().copied().filter(|other| *other != name).collect();
        assert_eq!(sorted(role.may_not_also_hold.clone()), sorted_names(&others), "{}", name);
    }
}

#[test]
fn test_WB_RULE_2_no_role_may_merge() {
    for (name, role) in first_revision().roles {
        assert!(!role.may.join(" ").to_lowercase().contains("merge"), "{}", name);
    }
}

#[test]
fn test_WB_RULE_3_the_first_table_gives_two_rounds() {
    assert_eq!(first_revision().review_rounds, 2);
}

#[test]
fn test_WB_RULE_4_what_always_goes_to_a_person_is_listed() {
    assert_eq!(
        sorted(first_revision().always_to_a_person),
        sorted_names(&[
            "change_to_an_existing_test",
            "change_under_watchbill_folder",
            "verification_of_classed_code",
            "finding_open_after_last_round",
            "last_message_without_its_fields_after_last_ask",
            "requirement_added_to_a_change",
            "check_of_a_fix_after_last_round",
            "merge",
        ])
    );
}

#[test]
fn test_WB_RULE_5_a_change_to_classed_code_needs_a_person_as_verifier() {
    assert_eq!(first_revision().classed_code_verifier, "person");
}

#[test]
fn test_WB_RULE_6_the_officer_holds_no_other_role_and_never_merges() {
    let table = first_revision();
    let officer = &table.roles["officer"];
    assert_eq!(sorted(officer.may_not_also_hold.clone()), sorted_names(&WITH_PROMPT));
    assert!(!officer.may.join(" ").to_lowercase().contains("merge"));
    assert!(table.always_to_a_person.contains(&"merge".to_string()));
}

#[test]
fn test_WB_RULE_6_the_officer_has_no_prompt_yet() {
    assert_eq!(first_revision().roles["officer"].prompt, "none");
}

#[test]
fn test_WB_RULE_7_the_first_table_names_no_queue() {
    assert!(first_revision().officer_queues.is_empty());
}

#[test]
fn test_WB_RULE_40_the_model_table_has_a_row_for_the_roles_with_a_prompt_but_the_breakdown_only() {
    let table = first_revision();
    assert_eq!(sorted(table.models.keys().cloned().collect()), sorted_names(&WITH_MODEL));
    assert!(!table.models.contains_key("officer"));
    assert!(!table.models.contains_key("breakdown"));
    for (name, role) in &table.roles {
        assert_eq!(table.models.contains_key(name), role.prompt != "none" && name != "breakdown", "{}", name);
    }
}

#[test]
fn test_WB_RULE_9_the_models_and_efforts_of_the_first_table() {
    let table = first_revision();
    for name in ["tester", "build", "fix"] {
        assert_eq!((table.models[name].model.as_str(), table.models[name].effort.as_str()), ("claude-sonnet-5-5", "medium"));
    }
    assert_eq!((table.models["review"].model.as_str(), table.models["review"].effort.as_str()), ("claude-opus-5-5", "high"));
}

// TestShape

#[test]
fn test_WB_RULE_39_a_good_table_is_read() {
    let table = Tables::new().load();
    assert_eq!(sorted(table.roles.keys().cloned().collect()), sorted_names(&ROLES));
}

#[test]
fn test_WB_RULE_39_a_table_without_one_of_the_six_roles_is_refused_and_names_it() {
    let mut t = Tables::new();
    t.rules.get("roles").del("fix");
    if let J::M(roles) = t.rules.get("roles") {
        for (_, role) in roles.iter_mut() {
            role.get("may_not_also_hold").list().retain(|other| !matches!(other, J::S(name) if name == "fix"));
        }
    }
    assert!(t.refused().contains("fix"));
}

#[test]
fn test_WB_RULE_39_a_role_that_is_not_one_of_the_six_is_refused_and_names_it() {
    let mut t = Tables::new();
    t.rules.get("roles").set("captain", role_row("captain"));
    assert!(t.refused().contains("captain"));
}

#[test]
fn test_WB_RULE_3_a_table_without_the_rounds_is_refused_and_names_the_field() {
    let mut t = Tables::new();
    t.rules.del("review_rounds");
    assert!(t.refused().contains("review_rounds"));
}

#[test]
fn test_WB_RULE_4_a_table_without_the_list_for_a_person_is_refused_and_names_it() {
    let mut t = Tables::new();
    t.rules.del("always_to_a_person");
    assert!(t.refused().contains("always_to_a_person"));
}

#[test]
fn test_WB_RULE_4_an_empty_list_for_a_person_is_refused() {
    let mut t = Tables::new();
    t.rules.set("always_to_a_person", J::L(vec![]));
    assert!(t.refused().contains("always_to_a_person"));
}

#[test]
fn test_WB_RULE_5_a_rule_for_classed_code_that_names_no_person_is_refused() {
    let mut t = Tables::new();
    t.rules.set("classed_code", map(vec![("verifier", s("agent"))]));
    assert!(t.refused().contains("classed_code"));
}

#[test]
fn test_WB_RULE_5_a_table_without_the_rule_for_classed_code_is_refused() {
    let mut t = Tables::new();
    t.rules.del("classed_code");
    assert!(t.refused().contains("classed_code"));
}

#[test]
fn test_WB_RULE_6_a_role_that_may_merge_is_refused_and_names_the_role() {
    let mut t = Tables::new();
    t.role("officer").get("may").list().push(s("merge"));
    let message = t.refused();
    assert!(message.contains("officer"), "{}", message);
    assert!(message.contains("merge"), "{}", message);
}

#[test]
fn test_WB_RULE_6_an_officer_that_may_hold_another_role_is_refused() {
    let mut t = Tables::new();
    t.role("officer").set("may_not_also_hold", texts(&["tester"]));
    for name in ["breakdown", "build", "review", "fix"] {
        t.role(name).get("may_not_also_hold").list().retain(|other| !matches!(other, J::S(role) if role == "officer"));
    }
    t.role("tester").set("may_not_also_hold", texts(&["breakdown", "build", "review", "fix", "officer"]));
    assert!(t.refused().contains("officer"));
}

#[test]
fn test_WB_RULE_7_a_table_without_the_queues_is_refused_and_names_the_field() {
    let mut t = Tables::new();
    t.rules.del("officer_queues");
    assert!(t.refused().contains("officer_queues"));
}

#[test]
fn test_WB_RULE_7_a_queue_is_read_as_a_name() {
    let mut t = Tables::new();
    t.rules.set("officer_queues", texts(&["triage"]));
    assert_eq!(t.load().officer_queues, vec!["triage"]);
}

#[test]
fn test_WB_RULE_40_a_model_row_for_a_role_without_a_prompt_is_refused() {
    let mut t = Tables::new();
    t.models.get("models").set("officer", map(vec![("model", s("claude-sonnet-5-5")), ("effort", s("medium"))]));
    let message = t.refused();
    assert!(message.contains("officer"), "{}", message);
    assert!(message.contains("models.yml"), "{}", message);
}

#[test]
fn test_WB_RULE_40_a_role_with_a_prompt_and_no_model_row_is_refused() {
    let mut t = Tables::new();
    t.models.get("models").del("fix");
    let message = t.refused();
    assert!(message.contains("fix"), "{}", message);
    assert!(message.contains("models.yml"), "{}", message);
}

#[test]
fn test_WB_RULE_40_a_model_row_without_a_model_or_an_effort_is_refused_and_names_both() {
    let mut t = Tables::new();
    t.models.get("models").get("build").del("model");
    let message = t.refused();
    assert!(message.contains("build"), "{}", message);
    assert!(message.contains("model"), "{}", message);
    t.models = good_models();
    t.models.get("models").get("review").del("effort");
    let message = t.refused();
    assert!(message.contains("review"), "{}", message);
    assert!(message.contains("effort"), "{}", message);
}

#[test]
fn test_WB_RULE_20_an_effort_that_is_not_known_is_refused() {
    let mut t = Tables::new();
    t.models.get("models").get("tester").set("effort", s("enormous"));
    let message = t.refused();
    assert!(message.contains("tester"), "{}", message);
    assert!(message.contains("effort"), "{}", message);
}

#[test]
fn test_WB_RULE_20_the_efforts_low_to_max_are_accepted() {
    let mut t = Tables::new();
    for effort in ["low", "medium", "high", "xhigh", "max"] {
        t.models.get("models").get("tester").set("effort", s(effort));
        assert_eq!(t.load().models["tester"].effort, effort);
    }
}

#[test]
fn test_WB_RULE_41_a_role_in_may_not_also_hold_that_is_not_one_of_the_six_is_refused() {
    let mut t = Tables::new();
    t.role("build").get("may_not_also_hold").list().push(s("captain"));
    let message = t.refused();
    assert!(message.contains("captain"), "{}", message);
    assert!(message.contains("build"), "{}", message);
}

#[test]
fn test_WB_RULE_41_a_role_that_may_not_also_hold_itself_is_refused() {
    let mut t = Tables::new();
    t.role("build").get("may_not_also_hold").list().push(s("build"));
    assert!(t.refused().contains("build"));
}

// TestRefusals

#[test]
fn test_WB_RULE_24_a_prompt_that_is_not_a_markdown_file_below_the_revision_folder_is_refused() {
    let mut t = Tables::new();
    for bad in ["roles/tester.txt", "roles/tester", "../tester.md", "roles/../../tester.md", "/etc/tester.md", ".."] {
        t.role("tester").set("prompt", s(bad));
        let message = t.refused();
        assert!(message.contains("tester"), "{}: {}", bad, message);
        assert!(message.contains("prompt"), "{}: {}", bad, message);
    }
}

#[test]
fn test_WB_RULE_24_a_prompt_below_the_revision_folder_is_accepted() {
    let mut t = Tables::new();
    for good in ["roles/tester.md", "tester.md", "a/b/tester.md", "roles/a..b.md"] {
        t.role("tester").set("prompt", s(good));
        assert_eq!(t.load().roles["tester"].prompt, good);
    }
}

#[test]
fn test_WB_RULE_25_a_name_written_twice_in_the_list_for_a_person_is_refused() {
    let mut t = Tables::new();
    t.rules.get("always_to_a_person").list().push(s("verification_of_classed_code"));
    let message = t.refused();
    assert!(message.contains("always_to_a_person"), "{}", message);
    assert!(message.contains("twice"), "{}", message);
}

#[test]
fn test_WB_RULE_27_a_table_that_does_not_send_a_merge_to_a_person_is_refused() {
    let mut t = Tables::new();
    t.rules.get("always_to_a_person").list().retain(|name| !matches!(name, J::S(text) if text == "merge"));
    let message = t.refused();
    assert!(message.contains("merge"), "{}", message);
    assert!(message.contains("always_to_a_person"), "{}", message);
}

#[test]
fn test_WB_RULE_6_a_role_that_may_merges_in_any_form_is_refused() {
    let mut t = Tables::new();
    for entry in ["Merges the change.", "Merging the change.", "The change is merged.", "MERGE it"] {
        t.role("officer").set("may", texts(&[entry]));
        assert!(t.refused().contains("officer"), "{}", entry);
    }
}

#[test]
fn test_WB_RULE_6_a_word_that_only_starts_like_merge_is_accepted() {
    let mut t = Tables::new();
    t.role("officer").set("may", texts(&["Read the emergency list."]));
    t.load();
}

#[test]
fn test_WB_RULE_7_a_queue_that_is_not_a_text_is_refused() {
    let mut t = Tables::new();
    for bad in [J::L(vec![J::I(1)]), J::L(vec![texts(&["a"])]), map(vec![("a", s("b"))]), s("triage"), J::Null] {
        t.rules.set("officer_queues", bad.clone());
        assert!(t.refused().contains("officer_queues"), "{:?}", bad);
    }
}

#[test]
fn test_WB_RULE_28_a_role_named_twice_in_the_roles_it_may_not_also_hold_is_refused() {
    for name in ROLES {
        let mut t = Tables::new();
        let first = t.role(name).get("may_not_also_hold").list()[0].clone();
        t.role(name).get("may_not_also_hold").list().push(first);
        let message = t.refused();
        assert!(message.contains(name), "{}", message);
        assert!(message.contains("twice"), "{}", message);
    }
}

#[test]
fn test_WB_RULE_10_one_role_that_excludes_another_that_does_not_exclude_it_is_refused() {
    let mut t = Tables::new();
    t.role("fix").get("may_not_also_hold").list().retain(|other| !matches!(other, J::S(role) if role == "build"));
    let message = t.refused();
    assert!(message.contains("build"), "{}", message);
    assert!(message.contains("fix"), "{}", message);
}

#[test]
fn test_WB_RULE_10_the_other_way_is_refused_too() {
    let mut t = Tables::new();
    t.role("tester").get("may_not_also_hold").list().retain(|other| !matches!(other, J::S(role) if role == "officer"));
    let message = t.refused();
    assert!(message.contains("tester"), "{}", message);
    assert!(message.contains("officer"), "{}", message);
}

#[test]
fn test_WB_RULE_11_a_role_without_one_of_its_fields_is_refused_with_the_role_and_the_field() {
    for field in ["step", "produces", "reads", "may", "may_not_also_hold", "prompt"] {
        let mut t = Tables::new();
        t.role("review").del(field);
        let message = t.refused();
        assert!(message.contains("review"), "{}: {}", field, message);
        assert!(message.contains(field), "{}: {}", field, message);
    }
}

#[test]
fn test_WB_RULE_11_a_field_with_nothing_in_it_is_refused() {
    let mut t = Tables::new();
    t.role("fix").set("step", s("  "));
    let message = t.refused();
    assert!(message.contains("fix"), "{}", message);
    assert!(message.contains("step"), "{}", message);
    t.rules = good_rules();
    t.role("fix").set("reads", J::L(vec![]));
    assert!(t.refused().contains("reads"));
}

#[test]
fn test_WB_RULE_12_an_unknown_key_at_the_top_is_refused_with_where_it_stands() {
    let mut t = Tables::new();
    t.rules.set("surprise", J::I(1));
    let message = t.refused();
    assert!(message.contains("surprise"), "{}", message);
    assert!(message.contains("rules.yml"), "{}", message);
}

#[test]
fn test_WB_RULE_12_an_unknown_key_in_a_role_is_refused_with_where_it_stands() {
    let mut t = Tables::new();
    t.role("build").set("colour", s("blue"));
    let message = t.refused();
    assert!(message.contains("roles.build"), "{}", message);
    assert!(message.contains("colour"), "{}", message);
    assert!(message.contains("line"), "{}", message);
}

#[test]
fn test_WB_RULE_12_an_unknown_key_in_the_rule_for_classed_code_is_refused() {
    let mut t = Tables::new();
    t.rules.get("classed_code").set("extra", s("x"));
    let message = t.refused();
    assert!(message.contains("classed_code"), "{}", message);
    assert!(message.contains("extra"), "{}", message);
}

#[test]
fn test_WB_RULE_12_an_unknown_key_in_the_model_table_is_refused_at_each_depth() {
    let mut t = Tables::new();
    t.models.set("extra", J::I(1));
    let message = t.refused();
    assert!(message.contains("extra"), "{}", message);
    assert!(message.contains("models.yml"), "{}", message);
    t.models = good_models();
    t.models.get("models").get("tester").set("temperature", J::I(1));
    let message = t.refused();
    assert!(message.contains("models.tester"), "{}", message);
    assert!(message.contains("temperature"), "{}", message);
}

#[test]
fn test_WB_RULE_13_a_number_of_rounds_that_is_not_a_whole_number_of_at_least_1_is_refused() {
    let mut t = Tables::new();
    for bad in [J::I(0), J::I(-1), J::F(1.5), s("2"), J::B(true), J::Null, J::L(vec![J::I(2)])] {
        t.rules.set("review_rounds", bad.clone());
        assert!(t.refused().contains("review_rounds"), "{:?}", bad);
    }
}

#[test]
fn test_WB_RULE_13_a_whole_number_of_at_least_1_is_accepted() {
    let mut t = Tables::new();
    for good in [1, 2, 5] {
        t.rules.set("review_rounds", J::I(good));
        assert_eq!(t.load().review_rounds, good);
    }
}

#[test]
fn test_WB_RULE_14_a_model_id_that_is_not_exact_is_refused() {
    let mut t = Tables::new();
    for bad in ["latest", "claude-sonnet", "claude-sonnet-latest", "claude", "sonnet", "", "Claude-Sonnet-5-5", "claude sonnet 5"] {
        t.models.get("models").get("tester").set("model", s(bad));
        let message = t.refused();
        assert!(message.contains("tester"), "{}: {}", bad, message);
        assert!(message.contains("model"), "{}: {}", bad, message);
    }
}

#[test]
fn test_WB_RULE_14_the_two_ids_of_the_first_table_are_accepted() {
    let mut t = Tables::new();
    t.models.get("models").get("tester").set("model", s("claude-opus-5-5"));
    assert_eq!(t.load().models["tester"].model, "claude-opus-5-5");
}

#[test]
fn test_WB_RULE_14_latest_in_the_middle_of_an_id_is_refused() {
    let mut t = Tables::new();
    for bad in ["claude-latest-5", "latest-5", "claude-sonnet-latest-5"] {
        t.models.get("models").get("tester").set("model", s(bad));
        assert!(t.refused().contains("tester"), "{}", bad);
    }
}

#[test]
fn test_WB_RULE_15_a_date_that_does_not_exist_is_refused_with_the_file_and_the_line() {
    let t = Tables::new();
    let text = t.rules.dumps().replace("\"review_rounds\": 2", "review_rounds: 2026-02-30");
    let message = t.refused_with(Some(&(text + "\n")), None);
    assert!(message.contains("rules.yml"), "{}", message);
    assert!(message.contains("line"), "{}", message);
}

#[test]
fn test_WB_RULE_15_a_table_that_cannot_be_built_is_refused_in_the_model_table_too() {
    let message = Tables::new().refused_with(None, Some("models: !!int abc\n"));
    assert!(message.contains("models.yml"), "{}", message);
}

#[test]
fn test_WB_RULE_15_a_rule_table_that_is_not_yaml_is_refused_with_the_file_named() {
    let message = Tables::new().refused_with(Some("roles: [unclosed\n"), None);
    assert!(message.contains("rules.yml"), "{}", message);
}

#[test]
fn test_WB_RULE_15_a_model_table_that_is_not_yaml_is_refused_with_the_file_named() {
    let message = Tables::new().refused_with(None, Some("models: {a: [\n"));
    assert!(message.contains("models.yml"), "{}", message);
}

#[test]
fn test_WB_RULE_15_a_table_that_is_not_a_mapping_is_refused_with_the_file_named() {
    let t = Tables::new();
    assert!(t.refused_with(Some("- a\n- b\n"), None).contains("rules.yml"));
    assert!(t.refused_with(None, Some("just text\n")).contains("models.yml"));
}

#[test]
fn test_WB_RULE_21_a_table_that_is_not_there_is_refused_with_the_file_named() {
    let t = Tables::new();
    let (rules_path, models_path) = t.paths_with(None, None);
    fs::remove_file(&models_path).unwrap();
    let error = rules::load(&rules_path, &models_path).err().expect("a table that is not there was read");
    assert!(error.to_string().contains("models.yml"), "{}", error);
}

#[test]
fn test_WB_RULE_16_a_key_written_twice_in_a_role_is_refused_with_the_place() {
    let t = Tables::new();
    let text = t.rules.dumps().replace(
        "\"step\": \"a step of build\",",
        "\"step\": \"a step of build\",\n      \"step\": \"again\",",
    );
    let message = t.refused_with(Some(&(text + "\n")), None);
    assert!(message.contains("step"), "{}", message);
    assert!(message.contains("rules.yml"), "{}", message);
    assert!(message.contains("line"), "{}", message);
}

#[test]
fn test_WB_RULE_16_a_key_written_twice_at_the_top_of_the_model_table_is_refused() {
    let t = Tables::new();
    let text = t.models.dumps().replacen("\"models\": {", "\"models\": {}, \"models\": {", 1);
    assert!(t.refused_with(None, Some(&(text + "\n"))).contains("models.yml"));
}

#[test]
fn test_WB_RULE_17_a_name_that_yaml_reads_as_something_else_is_refused_with_quotes_asked_for() {
    let t = Tables::new();
    let text = t.rules.dumps().replace("\"step\": \"a step of tester\"", "\"step\": no");
    let message = t.refused_with(Some(&(text + "\n")), None);
    assert!(message.contains("quotes"), "{}", message);
    assert!(message.contains("roles.tester.step"), "{}", message);
}

#[test]
fn test_WB_RULE_17_a_prompt_that_yaml_reads_as_null_is_refused_with_quotes_asked_for() {
    let t = Tables::new();
    let text = t.rules.dumps().replace("\"prompt\": \"none\"", "\"prompt\": null");
    let message = t.refused_with(Some(&(text + "\n")), None);
    assert!(message.contains("quotes"), "{}", message);
    assert!(message.contains("officer"), "{}", message);
}

#[test]
fn test_WB_RULE_17_a_model_that_yaml_reads_as_a_number_is_refused_with_quotes_asked_for() {
    let t = Tables::new();
    let text = t.models.dumps().replace("\"model\": \"claude-opus-5-5\"", "\"model\": 1.10");
    let message = t.refused_with(None, Some(&(text + "\n")));
    assert!(message.contains("quotes"), "{}", message);
    assert!(message.contains("models.review.model"), "{}", message);
}

#[test]
fn test_WB_RULE_17_an_effort_that_yaml_reads_as_a_boolean_is_refused_with_quotes_asked_for() {
    let t = Tables::new();
    let text = t.models.dumps().replace("\"effort\": \"high\"", "\"effort\": on");
    assert!(t.refused_with(None, Some(&(text + "\n"))).contains("quotes"));
}

#[test]
fn test_WB_RULE_17_a_list_of_names_with_a_number_in_it_is_refused() {
    let mut t = Tables::new();
    t.role("fix").set("reads", J::L(vec![s("a"), J::I(1)]));
    assert!(t.refused().contains("quotes"));
}

#[test]
fn test_WB_RULE_18_a_tag_that_would_build_an_object_is_refused_in_a_table() {
    let t = Tables::new();
    let text = t
        .rules
        .dumps()
        .replace("\"step\": \"a step of fix\"", "\"step\": !!python/object/apply:os.system [\"echo hi\"]");
    let message = t.refused_with(Some(&(text + "\n")), None);
    assert!(message.contains("rules.yml"), "{}", message);
}

// The tests of #23.

#[test]
fn test_WB_RULE_29_only_rules_rs_names_the_two_tables() {
    let sources = files(&root().join("src"), ".rs");
    assert!(sources.iter().any(|path| path.ends_with("src/rules.rs")));
    for path in sources {
        if !path.ends_with("src/rules.rs") {
            assert!(!names_a_table(&fs::read_to_string(&path).unwrap()), "{}", path.display());
        }
    }
}

#[test]
fn test_WB_RULE_30_an_alias_in_a_table_is_refused_with_the_line() {
    let t = Tables::new();
    let text = t.rules.dumps().replace("\"verifier\": \"person\"", "\"verifier\": &p \"person\"").replace(
        "\"prompt\": \"none\"",
        "\"prompt\": *p",
    );
    let message = t.refused_with(Some(&(text + "\n")), None);
    assert!(message.contains("rules.yml"), "{}", message);
    assert!(message.contains("alias"), "{}", message);
    assert!(message.contains("anchor"), "{}", message);
}

#[test]
fn test_WB_RULE_14_a_model_id_with_a_line_break_at_the_end_is_refused() {
    let mut t = Tables::new();
    t.models.get("models").get("tester").set("model", s("claude-sonnet-5-5\n"));
    let message = t.refused();
    assert!(message.contains("tester"), "{}", message);
}

#[test]
fn test_WB_RULE_12_a_key_is_named_as_it_is_written() {
    let t = Tables::new();
    let text = t.rules.dumps().replacen("\"review_rounds\": 2,", "\"review_rounds\": 2,\n  no: 1,", 1);
    let message = t.refused_with(Some(&(text + "\n")), None);
    assert!(message.contains("`no`"), "{}", message);
    assert!(message.contains("line 3"), "{}", message);
}

#[test]
fn test_WB_RULE_39_the_tables_read_hold_what_the_files_say() {
    let mut t = Tables::new();
    t.rules.set("officer_queues", texts(&["triage", "small"]));
    let table = t.load();
    assert_eq!(table.officer_queues, vec!["triage", "small"]);
    assert_eq!(table.always_to_a_person, t.rules.get("always_to_a_person").texts());
    let build = &table.roles["build"];
    assert_eq!(build.name, "build");
    assert_eq!(build.step, "a step of build");
    assert_eq!(build.produces, "what build produces");
    assert_eq!(build.reads, vec!["a thing to read"]);
    assert_eq!(build.may, vec!["a thing to do"]);
    assert_eq!(build.may_not_also_hold, vec!["breakdown", "tester", "review", "fix", "officer"]);
    assert_eq!(build.prompt, "roles/build.md");
}

#[test]
fn test_WB_RULE_6_a_mark_that_python_takes_as_no_letter_before_merge_is_refused() {
    // Python's `\b` takes no combining mark as a word character: U+0345, U+0363, U+05B0, U+0903.
    let mut t = Tables::new();
    for mark in ['\u{345}', '\u{363}', '\u{36f}', '\u{5b0}', '\u{903}', '\u{20d0}'] {
        t.role("build").set("may", texts(&[&format!("Open the chan{}merge for review.", mark)]));
        let message = t.refused();
        assert!(message.contains("build"), "U+{:04X}: {}", mark as u32, message);
        assert!(message.contains("merge"), "U+{:04X}: {}", mark as u32, message);
    }
}

#[test]
fn test_WB_RULE_6_a_letter_or_digit_python_takes_as_a_word_character_before_merge_is_accepted() {
    let mut t = Tables::new();
    for letter in ['é', 'ſ', 'ß', 'Ω', 'я', '中', '٣', '_', 'ǅ', 'ª', '²'] {
        t.role("build").set("may", texts(&[&format!("Read the {}merge list.", letter)]));
        t.load();
    }
}

#[test]
fn test_WB_RULE_6_a_capital_whose_lower_case_python_does_not_know_before_merge_is_refused() {
    // U+A7CB and U+A7DC are new in Unicode 16. Python 3.9 leaves them as they are, which is no
    // word character, so `\b` holds before `merg`; Rust lower-cases them to word characters.
    let mut t = Tables::new();
    for capital in ['\u{a7cb}', '\u{a7dc}'] {
        t.role("build").set("may", texts(&[&format!("Open the chan{}merge for review.", capital)]));
        let message = t.refused();
        assert!(message.contains("build"), "U+{:04X}: {}", capital as u32, message);
        assert!(message.contains("merge"), "U+{:04X}: {}", capital as u32, message);
    }
    // A capital Python knows still counts as a letter before `merge`.
    for capital in ['É', 'Ω', 'Я'] {
        t.role("build").set("may", texts(&[&format!("Read the {}merge list.", capital)]));
        t.load();
    }
}

// The last message is checked (#45).

#[test]
fn test_WB_RULE_31_the_first_table_asks_twice_for_a_last_message_with_its_fields() {
    assert_eq!(first_revision().last_message_asks, 2);
}

#[test]
fn test_WB_RULE_31_a_table_without_the_asks_is_refused_and_names_the_field() {
    let mut t = Tables::new();
    t.rules.del("last_message_asks");
    assert!(t.refused().contains("last_message_asks"));
}

#[test]
fn test_WB_RULE_32_a_last_message_without_its_fields_after_the_last_ask_goes_to_a_person() {
    let person = first_revision().always_to_a_person;
    assert!(person.contains(&"last_message_without_its_fields_after_last_ask".to_string()), "{:?}", person);
}

#[test]
fn test_WB_RULE_33_a_number_of_asks_that_is_not_a_whole_number_of_at_least_1_is_refused() {
    let mut t = Tables::new();
    for bad in [J::I(0), J::I(-1), J::F(1.5), s("2"), J::B(true), J::Null, J::L(vec![J::I(2)])] {
        t.rules.set("last_message_asks", bad.clone());
        assert!(t.refused().contains("last_message_asks"), "{:?}", bad);
    }
}

#[test]
fn test_WB_RULE_33_a_whole_number_of_at_least_1_is_accepted() {
    let mut t = Tables::new();
    for good in [1, 2, 5] {
        t.rules.set("last_message_asks", J::I(good));
        assert_eq!(t.load().last_message_asks, good);
    }
}

// A requirement added during a change, and a fix after the last round (#49).

#[test]
fn test_WB_RULE_34_the_first_table_sends_an_added_requirement_to_the_tester_then_the_builder() {
    assert_eq!(first_revision().added_requirement_goes_to, ["tester", "build"]);
}

#[test]
fn test_WB_RULE_34_a_table_without_the_list_for_an_added_requirement_is_refused_and_names_it() {
    let mut t = Tables::new();
    t.rules.del("added_requirement_goes_to");
    assert!(t.refused().contains("added_requirement_goes_to"));
}

#[test]
fn test_WB_RULE_34_the_list_is_read_in_its_order() {
    let mut t = Tables::new();
    t.rules.set("added_requirement_goes_to", texts(&["build", "tester", "review"]));
    assert_eq!(t.load().added_requirement_goes_to, ["build", "tester", "review"]);
}

#[test]
fn test_WB_RULE_35_a_role_without_a_prompt_twice_or_none_in_the_list_for_an_added_requirement_is_refused() {
    let mut t = Tables::new();
    for bad in [
        texts(&["tester", "officer"]),
        texts(&["tester", "captain"]),
        texts(&["tester", "build", "tester"]),
        J::L(vec![]),
        s("tester"),
    ] {
        t.rules.set("added_requirement_goes_to", bad.clone());
        assert!(t.refused().contains("added_requirement_goes_to"), "{:?}", bad);
    }
}

#[test]
fn test_WB_RULE_36_a_requirement_added_to_a_change_goes_to_a_person() {
    let person = first_revision().always_to_a_person;
    assert!(person.contains(&"requirement_added_to_a_change".to_string()), "{:?}", person);
}

#[test]
fn test_WB_RULE_37_the_check_of_a_fix_after_the_last_round_goes_to_a_person() {
    let person = first_revision().always_to_a_person;
    assert!(person.contains(&"check_of_a_fix_after_last_round".to_string()), "{:?}", person);
}

// The breakdown and the place for the models of local runners (#75).

#[test]
fn test_WB_RULE_40_a_model_row_for_the_breakdown_is_refused() {
    let mut t = Tables::new();
    t.models.get("models").set("breakdown", map(vec![("model", s("claude-sonnet-5-5")), ("effort", s("medium"))]));
    let message = t.refused();
    assert!(message.contains("breakdown"), "{}", message);
    assert!(message.contains("models.yml"), "{}", message);
}

#[test]
fn test_WB_RULE_42_the_breakdown_comes_before_the_tester() {
    let step = &first_revision().roles["breakdown"].step;
    assert!(step.contains("The breakdown comes before the tester."), "{}", step);
}

#[test]
fn test_WB_RULE_43_no_other_role_of_an_issue_is_started_until_its_file_is_accepted() {
    let step = &first_revision().roles["breakdown"].step;
    assert!(
        step.contains("No other role of the issue is started until a person has accepted the issue's file."),
        "{}",
        step
    );
}

#[test]
fn test_WB_RULE_44_the_breakdown_holds_no_other_role_and_no_other_role_holds_it() {
    let table = first_revision();
    let others: Vec<&str> = ROLES.iter().copied().filter(|name| *name != "breakdown").collect();
    assert_eq!(sorted(table.roles["breakdown"].may_not_also_hold.clone()), sorted_names(&others));
    for name in others {
        assert!(table.roles[name].may_not_also_hold.iter().any(|other| other == "breakdown"), "{}", name);
    }
    assert_eq!(table.roles["breakdown"].prompt, "roles/breakdown.md");
}

#[test]
fn test_WB_RULE_45_the_model_table_has_an_empty_place_for_the_models_of_local_runners() {
    assert!(first_revision().local_models.is_empty());
    let text = fs::read_to_string(rules::MODELS_FILE).unwrap();
    assert!(text.split('\n').any(|line| line == "local_models: {}"), "{}", text);
    let mut t = Tables::new();
    t.models.del("local_models");
    let message = t.refused();
    assert!(message.contains("local_models"), "{}", message);
    assert!(message.contains("models.yml"), "{}", message);
}

#[test]
fn test_WB_RULE_46_a_model_table_with_something_under_local_models_is_refused() {
    let mut t = Tables::new();
    for bad in [
        map(vec![("breakdown", map(vec![("model", s("granite-4")), ("effort", s("medium"))]))]),
        texts(&["granite-4"]),
        s("granite-4"),
        J::Null,
    ] {
        t.models.set("local_models", bad.clone());
        let message = t.refused();
        assert!(message.contains("local_models"), "{:?}: {}", bad, message);
        assert!(message.contains("models.yml"), "{:?}: {}", bad, message);
    }
}

// The breakdown asks its questions on the issue (#95).

#[test]
fn test_WB_RULE_47_the_breakdown_may_post_its_questions_on_the_issue_and_nothing_else() {
    let row = &first_revision().roles["breakdown"];
    assert!(row.may.contains(&"Post its questions on the issue, one comment each.".to_string()), "{:?}", row.may);
    let posts: Vec<&String> = row.may.iter().filter(|each| each.to_lowercase().contains("post")).collect();
    assert_eq!(posts.len(), 1, "{:?}", row.may);
    assert!(row.reads.contains(&"The issue with its comments, the answers of a person among them.".to_string()), "{:?}", row.reads);
}

// Work is pushed (#98).

#[test]
fn test_WB_RULE_48_the_breakdown_the_tester_the_build_and_the_fix_may_push_to_the_branch_of_the_issue() {
    let revision = first_revision();
    for name in ["breakdown", "tester", "build", "fix"] {
        let row = &revision.roles[name];
        let pushes: Vec<&String> = row.may.iter().filter(|each| each.starts_with("Push") && each.contains("to the branch of the issue")).collect();
        assert_eq!(pushes.len(), 1, "{}: {:?}", name, row.may);
    }
    for name in ["review", "officer"] {
        let row = &revision.roles[name];
        assert!(!row.may.join(" ").to_lowercase().contains("push"), "{}: {:?}", name, row.may);
    }
}
