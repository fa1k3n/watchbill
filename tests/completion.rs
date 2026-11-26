//! The bash completion of `wb` (#220): the file `packaging/wb.bash` is held to the commands, the flags and
//! the roles of `wb`, and the spec installs it. The tests read the files; they run no bash.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn completion() -> String {
    fs::read_to_string(root().join("packaging/wb.bash")).expect("packaging/wb.bash could not be read")
}

/// The words of a line of the completion such as `_wb_commands="init check"`; none when there is no such line.
fn words_of(text: &str, name: &str) -> BTreeSet<String> {
    let start = format!("{}=\"", name);
    match text.lines().find(|line| line.starts_with(&start)) {
        Some(line) => line[start.len()..].trim_end_matches('"').split_whitespace().map(String::from).collect(),
        None => BTreeSet::new(),
    }
}

/// The flags that `USAGE` names for a command: those of its line and of the lines that go on from it.
fn usage_flags(command: &str) -> BTreeSet<String> {
    let block = watchbill::USAGE.split("\n\n").next().unwrap();
    let mut flags = BTreeSet::new();
    let mut inside = false;
    let mut needs_prompt = false;
    for line in block.lines().skip(1) {
        if line.starts_with("  wb ") {
            inside = line.split_whitespace().nth(1) == Some(command);
        }
        if inside {
            needs_prompt |= line.contains("the options of prompt");
            for word in line.split(|c: char| c.is_whitespace() || "[]()|<>".contains(c)) {
                if word.starts_with("--") {
                    flags.insert(word.to_string());
                }
            }
        }
    }
    if needs_prompt {
        flags.extend(usage_flags("prompt"));
    }
    flags
}

fn built() -> BTreeSet<String> {
    watchbill::layout::COMMANDS
        .iter()
        .filter(|command| watchbill::layout::planned(command.name).is_none())
        .map(|command| command.name.to_string())
        .collect()
}

#[test]
fn test_WB_CMD_16_each_built_command_is_in_the_completion_and_no_other_command() {
    let found = words_of(&completion(), "_wb_commands");
    assert!(!built().is_empty());
    assert_eq!(found, built());
}

#[test]
fn test_WB_CMD_16_the_completion_is_for_wb_and_watchbill() {
    let text = completion();
    assert!(text.lines().any(|line| line.starts_with("complete ") && line.ends_with(" wb watchbill")), "{}", text);
}

#[test]
fn test_WB_CMD_17_each_flag_the_usage_names_for_a_command_is_offered_for_it_and_no_other() {
    let text = completion();
    for command in built() {
        let expected = usage_flags(&command);
        let found = words_of(&text, &format!("_wb_flags_{}", command));
        assert_eq!(found, expected, "{}", command);
    }
}

#[test]
fn test_WB_CMD_17_the_reading_of_usage_finds_the_flags() {
    // A guard on the reading of USAGE: if it found nothing, the test above would hold for nothing.
    assert!(usage_flags("start").contains("--runner"));
    assert!(usage_flags("start").contains("--parts"));
    assert!(usage_flags("view").contains("--out"));
}

#[test]
fn test_WB_CMD_18_each_role_of_the_revision_is_in_the_completion_and_no_other() {
    let folder = root().join("share/watchbill/revision/roles");
    let mut roles = BTreeSet::new();
    for entry in fs::read_dir(folder).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if let Some(role) = name.strip_suffix(".md") {
            if role != "placeholders" {
                roles.insert(role.to_string());
            }
        }
    }
    assert!(roles.contains("build"));
    assert_eq!(words_of(&completion(), "_wb_roles"), roles);
}

#[test]
fn test_WB_CMD_18_the_issues_are_the_files_of_the_checkout_under_watchbill_issues() {
    let text = completion();
    assert!(text.contains(".watchbill/issues/"), "the completion does not look in .watchbill/issues/");
    assert!(text.contains("git rev-parse --show-toplevel"));
}

#[test]
fn test_WB_PKG_22_the_spec_installs_the_completion_and_a_link_for_watchbill() {
    let spec = fs::read_to_string(root().join("packaging/watchbill.spec")).unwrap();
    let folder = "/usr/share/bash-completion/completions";
    assert!(
        spec.lines().any(|line| line.starts_with("install ") && line.contains("packaging/wb.bash") && line.contains(&format!("{}/wb", folder))),
        "the spec does not install packaging/wb.bash as {}/wb",
        folder
    );
    assert!(spec.lines().any(|line| line.starts_with("ln -s wb ") && line.ends_with(&format!("{}/watchbill", folder))));
    for line in [
        "%dir /usr/share/bash-completion".to_string(),
        format!("%dir {}", folder),
        format!("{}/wb", folder),
        format!("{}/watchbill", folder),
    ] {
        assert!(spec.lines().any(|found| found == line), "%files lacks {}", line);
    }
    assert!(!spec.contains("Requires:       bash-completion"), "the package must not require bash-completion");
}

/// The word list of each `compgen -W` of the completion: the text of its quoted argument.
fn word_lists(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in text.lines().filter(|line| !line.trim_start().starts_with('#')) {
        let mut rest = line;
        while let Some(at) = rest.find("compgen -W ") {
            rest = &rest[at + "compgen -W ".len()..];
            let body = rest.strip_prefix('"').expect("a word list of compgen -W is not in double quotes");
            let end = body.find("\" --").expect("a word list of compgen -W has no end");
            found.push(body[..end].to_string());
            rest = &body[end..];
        }
    }
    found
}

#[test]
fn test_WB_CMD_20_no_word_list_of_compgen_is_made_by_a_command_or_holds_a_name_from_disk() {
    // `compgen -W` expands its list a second time, so a file name in it would run as code. A list is made of
    // the fixed lists of the file, the flags of a command that is one of those, and literal words.
    let text = completion();
    let lists = word_lists(&text);
    assert!(lists.len() >= 4, "{:?}", lists);
    let fixed = ["$_wb_commands", "$_wb_roles", "${!var-}"];
    for list in &lists {
        let mut left = list.clone();
        for variable in fixed {
            left = left.replace(variable, "");
        }
        assert!(!left.contains('$') && !left.contains('`') && !left.contains('\\') && !left.contains('('), "compgen -W \"{}\"", list);
    }
    assert!(!text.lines().any(|line| !line.trim_start().starts_with('#') && line.contains("eval ")), "the file uses eval");
}

#[test]
fn test_WB_CMD_21_the_stage_words_of_the_completion_are_the_headings_of_wb_status() {
    let source = fs::read_to_string(root().join("src/status.rs")).unwrap();
    let quoted = |text: &str| -> BTreeSet<String> { text.split('"').skip(1).step_by(2).map(String::from).collect() };
    let line_of = |start: &str| source.lines().find(|line| line.starts_with(start)).unwrap_or_else(|| panic!("{} not found", start)).to_string();
    let mut headings = quoted(line_of("pub const STEPS").split_once('=').unwrap().1);
    headings.extend(quoted(&line_of("pub const QUESTIONS")));
    headings.extend(quoted(&line_of("pub const PERSON")));
    headings.extend(quoted(&line_of("pub const NOT_READ")));
    assert_eq!(headings.len(), 10);
    // `_wb_stages=("Not planned" Planned Accepted ...)`: words in double quotes, or bare.
    let text = completion();
    let line = text.lines().find(|line| line.starts_with("_wb_stages=(")).expect("no _wb_stages");
    let inside = line.strip_prefix("_wb_stages=(").unwrap().strip_suffix(')').unwrap();
    let mut found = BTreeSet::new();
    for (i, part) in inside.split('"').enumerate() {
        if i % 2 == 1 {
            found.insert(part.to_string());
        } else {
            found.extend(part.split_whitespace().map(String::from));
        }
    }
    assert_eq!(found, headings);
}
