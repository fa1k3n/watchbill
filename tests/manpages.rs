//! The man pages (#182): one page of section 1 for each built command, and `wb.1` that lists them. The
//! pages are held to the commands and the flags of `wb`, and the spec installs them. The tests read the
//! files; they run no `man`.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn page(name: &str) -> String {
    fs::read_to_string(root().join("packaging/man").join(name)).unwrap_or_else(|_| panic!("packaging/man/{} could not be read", name))
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

/// A flag as roff has it: a hyphen is written `\-`, so the page is searched for that form.
fn roff(flag: &str) -> String {
    flag.replace('-', "\\-")
}

#[test]
fn test_WB_CMD_22_each_built_command_has_a_page_with_its_name_and_flags() {
    assert!(!built().is_empty());
    for command in built() {
        let text = page(&format!("wb-{}.1", command));
        assert!(text.contains(&format!(".TH WB\\-{} 1", command.to_uppercase())), "{}: no .TH line", command);
        assert!(text.contains(&format!("wb\\-{} \\- ", command)), "{}: no NAME line", command);
        for flag in usage_flags(&command) {
            assert!(text.contains(&roff(&flag)), "wb-{}.1 does not name {}", command, flag);
        }
    }
}

#[test]
fn test_WB_CMD_22_the_reading_of_usage_finds_the_flags() {
    // A guard on the reading of USAGE: if it found nothing, the test above would hold for nothing.
    assert!(usage_flags("start").contains("--runner"));
    assert!(usage_flags("prompt").contains("--stamp"));
    assert!(usage_flags("view").contains("--out"));
}

#[test]
fn test_WB_CMD_22_no_page_is_there_for_a_command_that_is_not_built() {
    let found: BTreeSet<String> = fs::read_dir(root().join("packaging/man"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.starts_with("wb-"))
        .collect();
    let expected: BTreeSet<String> = built().iter().map(|command| format!("wb-{}.1", command)).collect();
    assert_eq!(found, expected);
}

#[test]
fn test_WB_CMD_23_wb_1_lists_each_built_command_and_no_other() {
    let text = page("wb.1");
    assert!(text.contains(".TH WB 1"));
    let found: BTreeSet<String> = text
        .lines()
        .filter_map(|line| line.strip_prefix(".BR wb\\-"))
        .filter_map(|rest| rest.split_whitespace().next())
        .map(String::from)
        .collect();
    assert_eq!(found, built());
    for command in built() {
        assert!(text.contains(&format!(".BR wb\\-{} (1)", command)), "{}: no reference to its page", command);
    }
}

#[test]
fn test_WB_PKG_23_the_spec_installs_and_owns_the_pages() {
    let spec = fs::read_to_string(root().join("packaging/watchbill.spec")).unwrap();
    let mut pages: Vec<String> = built().iter().map(|command| format!("wb-{}.1", command)).collect();
    pages.push("wb.1".to_string());
    for name in pages {
        assert!(
            spec.lines().any(|line| line.starts_with("install ") && line.contains(&format!("packaging/man/{}", name)) && line.contains("%{buildroot}%{_mandir}/man1")),
            "the spec does not install {}",
            name
        );
    }
    // rpm compresses a page as it likes, so the files are named with a star after the name.
    assert!(spec.lines().any(|line| line == "%{_mandir}/man1/wb.1*"), "%files lacks wb.1");
    assert!(spec.lines().any(|line| line == "%{_mandir}/man1/wb-*.1*"), "%files lacks the pages of the commands");
}

/// The text of a page with the roff escapes out: `\-` is `-`, `\e` is `\`, `\&` and the font changes are gone.
fn plain(text: &str) -> String {
    text.replace("\\&", "").replace("\\fB", "").replace("\\fR", "").replace("\\fI", "").replace("\\fP", "").replace("\\-", "-").replace("\\e", "\\")
}

/// The lines of the section of a page, up to the next `.SH`.
fn section(text: &str, name: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut inside = false;
    for line in text.lines() {
        if let Some(heading) = line.strip_prefix(".SH ") {
            inside = heading == name;
        } else if inside {
            found.push(line.to_string());
        }
    }
    found
}

#[test]
fn test_WB_CMD_24_the_synopsis_is_the_usage_line_and_the_options_are_the_flags_of_the_usage() {
    for command in built() {
        let text = page(&format!("wb-{}.1", command));
        // The synopsis, between .nf and .fi, against the usage lines of `wb help <command>`, word for word.
        let synopsis: Vec<String> = plain(&section(&text, "SYNOPSIS").join("\n")).lines().filter(|line| !line.starts_with('.')).map(String::from).collect();
        let page_words: Vec<String> = synopsis.join(" ").split_whitespace().map(String::from).collect();
        let help = watchbill::layout::help_of(&command, watchbill::USAGE).unwrap();
        let usage_part = help.split("\n\n").next().unwrap();
        let usage_words: Vec<String> = usage_part.lines().skip(1).collect::<Vec<_>>().join(" ").split_whitespace().map(String::from).collect();
        assert_eq!(page_words, usage_words, "{}: the SYNOPSIS is not the usage line", command);
        // The flags of the `.B --...` lines of OPTIONS, exactly those of the usage.
        let mut flags = BTreeSet::new();
        for line in section(&text, "OPTIONS") {
            if let Some(rest) = line.strip_prefix(".B ") {
                for word in plain(rest).split(|c: char| c.is_whitespace() || c == ',') {
                    if word.starts_with("--") {
                        flags.insert(word.to_string());
                    }
                }
            }
        }
        assert_eq!(flags, usage_flags(&command), "{}: the OPTIONS are not the flags of the usage", command);
    }
}

/// The words of the lines of a text that are not requests, joined by one space.
fn joined(text: &str) -> String {
    plain(text).lines().filter(|line| !line.starts_with('.')).collect::<Vec<_>>().join(" ").split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn test_WB_CMD_25_each_sentence_of_the_help_is_in_the_page() {
    for command in built() {
        let help = watchbill::layout::help_of(&command, watchbill::USAGE).unwrap();
        let paragraph = help.split("\n\n").nth(1).unwrap_or_else(|| panic!("{}: no paragraph", command));
        let paragraph = paragraph.trim_start().strip_prefix(command.as_str()).unwrap().to_string();
        let paragraph = paragraph.split_whitespace().collect::<Vec<_>>().join(" ");
        let text = joined(&page(&format!("wb-{}.1", command)));
        let sentences: Vec<&str> = paragraph.split(". ").map(|sentence| sentence.trim().trim_end_matches('.')).filter(|sentence| !sentence.is_empty()).collect();
        assert!(sentences.len() >= 2, "{}", command);
        for sentence in sentences {
            // The help begins in lower case, after the name of the command; the page may begin in capitals.
            let capital: String = sentence.chars().next().unwrap().to_uppercase().chain(sentence.chars().skip(1)).collect();
            assert!(text.contains(sentence) || text.contains(&capital), "wb-{}.1 lacks: {}", command, sentence);
        }
    }
}

#[test]
fn test_WB_CMD_26_the_exit_status_of_each_page_names_0_1_and_2() {
    for command in built() {
        let text = page(&format!("wb-{}.1", command));
        let lines = section(&text, "EXIT STATUS");
        for status in ["0", "1", "2"] {
            assert!(lines.iter().any(|line| *line == format!(".B {}", status)), "wb-{}.1 does not name the status {}", command, status);
        }
    }
}
