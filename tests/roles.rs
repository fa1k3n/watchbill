//! The role prompts of the revision: their files, their parts and what they may name.
//! Every test of tests/test_roles.py is here under the name it has. The pattern matching of the
//! Python tests is written out by hand, since `wb` is built from the standard library and two crates.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

use std::fs;
use std::path::PathBuf;

const ROLES: [&str; 4] = ["tester", "build", "review", "fix"];

// The four parts of a prompt, as the headings that start them, in order.
const PARTS: [&str; 4] = ["## What you produce", "## What you read", "## What you never do", "## How you end"];

// Names of products and of words that belong to one product. The adapters carry those.
// A list cannot be complete: it holds the common ones, and a reader looks for the rest.
const PRODUCTS: [&str; 24] = [
    "github",
    "gitlab",
    "gerrit",
    "bitbucket",
    "azure",
    "jira",
    "redmine",
    "bugzilla",
    "jenkins",
    "circleci",
    "zuul",
    "teamcity",
    "bamboo",
    "gitea",
    "forgejo",
    "codeberg",
    "sourcehut",
    "phabricator",
    "youtrack",
    "trello",
    "travis",
    "buildkite",
    "pull request",
    "merge request",
];

fn roles_folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("share/watchbill/revision/roles")
}

fn raw(name: &str) -> Vec<u8> {
    fs::read(roles_folder().join(name)).expect("the file could not be read")
}

fn prompt(role: &str) -> String {
    String::from_utf8(raw(&format!("{}.md", role))).expect("the prompt is not UTF-8")
}

/// What `re.split(r"^## .*$", text, flags=re.MULTILINE)` gives: the text before the first
/// heading, and then the text after each heading up to the next one.
fn split_parts(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut current = String::new();
    for (number, line) in text.split('\n').enumerate() {
        if number > 0 {
            current.push('\n');
        }
        if line.starts_with("## ") {
            found.push(std::mem::take(&mut current));
        } else {
            current.push_str(line);
        }
    }
    found.push(current);
    found
}

/// The text of one part of a prompt: from its heading to the next.
fn part(role: &str, heading: &str) -> String {
    let index = PARTS.iter().position(|name| *name == heading).unwrap();
    split_parts(&prompt(role)).remove(1 + index)
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

/// Whether the phrase stands in the text with no letter before it and none after it (`\bphrase\b`).
/// A phrase that starts with a line break has no word boundary before it, as in the pattern.
fn has_phrase(text: &str, phrase: &str) -> bool {
    text.match_indices(phrase).any(|(at, _)| {
        let before = phrase.starts_with('\n') || text[..at].chars().next_back().map_or(true, |c| !is_word(c));
        let after = text[at + phrase.len()..].chars().next().map_or(true, |c| !is_word(c));
        before && after
    })
}

/// The names of the lines `- `name`: ` (the fields of a last message), in order.
fn fields(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in text.split('\n') {
        if let Some(rest) = line.strip_prefix("- `") {
            let name: String = rest.chars().take_while(|c| c.is_ascii_lowercase() || *c == '_').collect();
            if !name.is_empty() && rest[name.len()..].starts_with("`: ") {
                found.push(name);
            }
        }
    }
    found
}

/// What `\{[^{}]*\}` finds.
fn placeholders(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '{' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '{' && chars[j] != '}' {
                j += 1;
            }
            if j < chars.len() && chars[j] == '}' {
                found.push(chars[i..=j].iter().collect());
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    found
}

/// The placeholders of the list: each stands first on a line of its own point, in backticks.
fn listed() -> Vec<String> {
    let text = String::from_utf8(raw("placeholders.md")).unwrap();
    let mut found = Vec::new();
    for line in text.split('\n') {
        if let Some(rest) = line.strip_prefix("- `{") {
            let name: String = rest.chars().take_while(|c| c.is_ascii_lowercase() || *c == '_').collect();
            if !name.is_empty() && rest[name.len()..].starts_with("}`") {
                found.push(format!("{{{}}}", name));
            }
        }
    }
    found
}

fn sorted_set(items: Vec<String>) -> Vec<String> {
    let mut items = items;
    items.sort();
    items.dedup();
    items
}

#[test]
fn test_WB_ROLE_1_each_of_the_four_roles_has_a_prompt_file() {
    for role in ROLES {
        assert!(roles_folder().join(format!("{}.md", role)).is_file(), "{}", role);
    }
}

#[test]
fn test_WB_ROLE_1_no_other_role_has_a_prompt_file() {
    let mut found: Vec<String> = fs::read_dir(roles_folder())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    found.sort();
    let mut expected: Vec<String> = ROLES.iter().chain(["placeholders"].iter()).map(|role| format!("{}.md", role)).collect();
    expected.sort();
    assert_eq!(found, expected);
}

#[test]
fn test_WB_ROLE_2_each_prompt_has_the_four_parts_in_order() {
    for role in ROLES {
        let headings: Vec<String> =
            prompt(role).split('\n').filter(|line| line.starts_with("## ")).map(String::from).collect();
        assert_eq!(headings, PARTS, "{}", role);
    }
}

#[test]
fn test_WB_ROLE_2_no_part_of_a_prompt_is_empty() {
    for role in ROLES {
        let parts = split_parts(&prompt(role))[1..].to_vec();
        assert_eq!(parts.len(), PARTS.len(), "{}", role);
        for (heading, body) in PARTS.iter().zip(parts) {
            assert!(body.trim().chars().count() > 80, "{} {}", role, heading);
        }
    }
}

#[test]
fn test_WB_ROLE_2_what_you_read_also_says_what_is_not_read() {
    for role in ROLES {
        let body = part(role, "## What you read");
        assert!(
            has_phrase(&body, "\nYou do not read") || has_phrase(&body, "\nYou read nothing"),
            "{}",
            role
        );
    }
}

#[test]
fn test_WB_ROLE_2_every_point_of_what_you_never_do_is_a_never() {
    for role in ROLES {
        let body = part(role, "## What you never do");
        let points: Vec<&str> = body.split('\n').filter(|line| line.starts_with("- ")).collect();
        assert!(points.len() >= 3, "{}", role);
        for point in points {
            assert!(point.starts_with("- You never "), "{}: {}", role, point);
        }
    }
}

#[test]
fn test_WB_ROLE_2_how_you_end_gives_the_fields_of_the_last_message_in_one_form() {
    let sentence = "Your last message has these fields, every time, in this order, \
                    each on a line of its own that starts with its name:";
    for role in ROLES {
        let body = part(role, "## How you end");
        assert!(body.contains(sentence), "{}", role);
        let found = fields(body.split_once(sentence).unwrap().1);
        assert!(found.len() >= 3, "{}", role);
        assert_eq!(found.last().unwrap(), "needs_person", "{}", role);
    }
}

#[test]
fn test_WB_ROLE_14_the_last_message_of_every_role_starts_with_model_and_effort() {
    for role in ROLES {
        let body = part(role, "## How you end");
        let found = fields(&body);
        assert_eq!(found[..2], ["model", "effort"], "{}", role);
        assert!(body.contains("or `unknown`"), "{}", role);
    }
}

#[test]
fn test_WB_ROLE_15_every_role_opens_no_file_outside_its_list_and_asks_first() {
    let bound = "You do not open a file outside those listed here. The files of a task are a bound and not a hint: \
                 a file may be closed to you for a reason you cannot see, and to read it first and say so afterwards \
                 is already a leak. If you need another, stop before you open it and say which and why under `needs_person`.";
    for role in ROLES {
        assert!(part(role, "## What you read").contains(bound), "{}", role);
        assert!(has_phrase(&part(role, "## What you never do"), "never open"), "{}", role);
    }
}

#[test]
fn test_WB_ROLE_15_no_role_reads_whatever_it_finds_it_needs() {
    // A list of what is read that ends in "what you need" is no list.
    for role in ROLES {
        let body = part(role, "## What you read");
        for point in body.split('\n').filter(|line| line.starts_with("- ")) {
            for lead in ["the code", "whatever", "anything"] {
                assert!(!has_phrase(point, &format!("{} you need", lead)), "{}: {}", role, point);
            }
        }
    }
}

#[test]
fn test_WB_ROLE_3_the_placeholders_are_one_list_in_a_file() {
    let names = listed();
    assert!(!names.is_empty());
    assert_eq!(names.len(), sorted_set(names.clone()).len(), "a placeholder stands twice");
    // Every placeholder the file shows is a point of the list: none is only named in passing.
    let text = String::from_utf8(raw("placeholders.md")).unwrap();
    assert_eq!(sorted_set(placeholders(&text)), sorted_set(names));
}

#[test]
fn test_WB_ROLE_4_a_prompt_uses_no_placeholder_that_is_not_on_the_list() {
    let names = listed();
    for role in ROLES {
        let used = sorted_set(placeholders(&prompt(role)));
        let outside: Vec<&String> = used.iter().filter(|name| !names.contains(name)).collect();
        assert!(outside.is_empty(), "{}: {:?}", role, outside);
        assert!(!used.is_empty(), "a prompt that is filled in with nothing: {}", role);
    }
}

#[test]
fn test_WB_ROLE_5_every_placeholder_on_the_list_is_used_by_a_prompt() {
    let mut used = Vec::new();
    for role in ROLES {
        used.extend(placeholders(&prompt(role)));
    }
    let unused: Vec<String> = listed().into_iter().filter(|name| !used.contains(name)).collect();
    assert!(unused.is_empty(), "{:?}", unused);
}

#[test]
fn test_WB_ROLE_6_no_prompt_names_a_product_of_a_git_server_a_tracker_or_a_ci_driver() {
    for role in ROLES {
        let text = prompt(role).to_lowercase();
        for product in PRODUCTS {
            assert!(!text.contains(product), "{} names {}", role, product);
        }
    }
}

#[test]
fn test_WB_ROLE_7_every_prompt_is_utf8_with_lf_and_ends_with_one_line_break() {
    for name in ROLES.iter().chain(["placeholders"].iter()) {
        let data = raw(&format!("{}.md", name));
        assert!(std::str::from_utf8(&data).is_ok(), "{}", name);
        assert!(!data.contains(&b'\r'), "{}", name);
        assert!(!data.starts_with(b"\xef\xbb\xbf"), "a byte order mark: {}", name);
        assert!(data.ends_with(b"\n"), "{}", name);
        assert!(!data.ends_with(b"\n\n"), "{}", name);
    }
}

#[test]
fn test_WB_ROLE_8_each_prompt_starts_with_a_line_saying_it_is_a_draft() {
    for role in ROLES {
        let text = prompt(role);
        let first = text.split('\n').next().unwrap();
        assert_eq!(first, "Draft: this prompt has not yet run on a project.", "{}", role);
    }
}

// Where the work is (#43): the words a session needs to know where it works and where it reads.

#[test]
fn test_WB_ROLE_17_the_list_holds_the_checkout_the_branch_and_the_two_addresses() {
    let names = listed();
    for name in ["{checkout}", "{branch}", "{task_address}", "{change_address}"] {
        assert!(names.contains(&name.to_string()), "{} is not on the list: {:?}", name, names);
    }
}

#[test]
fn test_WB_ROLE_18_every_prompt_names_its_checkout_its_branch_and_where_its_task_is_read() {
    for role in ROLES {
        // Said before the first part, where the role is named.
        let opening = split_parts(&prompt(role)).remove(0);
        assert!(opening.contains("You work in the checkout {checkout}, on the branch {branch}."), "{}", role);
        assert!(part(role, "## What you read").contains("The task {task}, at {task_address},"), "{}", role);
    }
}

#[test]
fn test_WB_ROLE_19_review_and_fix_name_where_the_change_is_read_and_posted() {
    for role in ["review", "fix"] {
        assert!(part(role, "## What you read").contains("{change}, at {change_address}"), "{}", role);
    }
}

#[test]
fn test_WB_ROLE_20_every_prompt_takes_pushing_and_posting_from_the_project_text() {
    for role in ROLES {
        let body = part(role, "## What you read");
        let point = body.split('\n').find(|line| line.starts_with("- The project's own text, where it follows this prompt:"));
        let point = point.unwrap_or_else(|| panic!("{}: no point for the project's own text", role));
        assert!(point.contains("how work is pushed and posted here"), "{}: {}", role, point);
    }
}
