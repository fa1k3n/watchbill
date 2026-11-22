//! The role prompts of the revision: their files, their parts and what they may name.
//! Every test of tests/test_roles.py is here under the name it has. The pattern matching of the
//! Python tests is written out by hand, since `wb` is built from the standard library and two crates.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

use std::fs;
use std::path::PathBuf;

// The roles with a prompt. The breakdown has one from revision 0.0.5 (#75).
const ROLES: [&str; 5] = ["breakdown", "tester", "build", "review", "fix"];

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

// The line of a prompt that has not yet run on a project.
const DRAFT: &str = "Draft: this prompt has not yet run on a project.";

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
fn test_WB_ROLE_47_each_of_the_five_roles_has_a_prompt_file() {
    for role in ROLES {
        assert!(roles_folder().join(format!("{}.md", role)).is_file(), "{}", role);
    }
}

#[test]
fn test_WB_ROLE_47_no_other_role_has_a_prompt_file() {
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
        // From revision 0.0.8 `asked` follows `needs_person` in the four roles after the breakdown (#113, WB-ROLE-72).
        assert!(found.iter().any(|name| name == "needs_person"), "{}: {:?}", role, found);
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
    let bound = "You do not open a file outside those listed here. The files of an issue are a bound and not a hint: \
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

// Where the work is (#43): the words a session needs to know where it works and where it reads.

#[test]
fn test_WB_ROLE_17_the_list_holds_the_checkout_the_branch_and_the_two_addresses() {
    let names = listed();
    for name in ["{checkout}", "{branch}", "{issue_address}", "{change_address}"] {
        assert!(names.contains(&name.to_string()), "{} is not on the list: {:?}", name, names);
    }
}

// The thing at the tracker is an issue (#78).

#[test]
fn test_WB_ROLE_58_the_list_holds_the_issue_and_its_address_and_not_the_task() {
    let names = listed();
    for name in ["{issue}", "{issue_address}"] {
        assert!(names.contains(&name.to_string()), "{} is not on the list: {:?}", name, names);
    }
    for name in ["{task}", "{task_address}"] {
        assert!(!names.contains(&name.to_string()), "{} is on the list: {:?}", name, names);
    }
}

#[test]
fn test_WB_ROLE_59_no_role_file_holds_the_placeholders_task_or_task_address() {
    let mut seen = 0;
    for entry in fs::read_dir(roles_folder()).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        for old in ["{task}", "{task_address}"] {
            assert!(!text.contains(old), "{} holds {}", path.display(), old);
        }
        seen += 1;
    }
    assert_eq!(seen, ROLES.len() + 1, "every role and the list");
}

#[test]
fn test_WB_ROLE_18_every_prompt_names_its_checkout_its_branch_and_where_its_issue_is_read() {
    for role in ROLES {
        // Said before the first part, where the role is named.
        let opening = split_parts(&prompt(role)).remove(0);
        assert!(opening.contains("You work in the checkout {checkout}, on the branch {branch}."), "{}", role);
        assert!(part(role, "## What you read").contains("The issue {issue}, at {issue_address},"), "{}", role);
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

// A search is an opening (#44).

#[test]
fn test_WB_ROLE_21_every_prompt_says_a_search_opens_every_file_it_reads() {
    let search = "A search (a grep, a find with content, a tool that reads files to answer) opens every file it reads. \
                  Keep it to the files you may read.";
    for role in ROLES {
        assert!(part(role, "## What you read").contains(search), "{}", role);
    }
}

// The last message is checked (#45).

#[test]
fn test_WB_ROLE_22_every_prompt_says_a_last_message_without_its_fields_is_refused_and_asked_for_again() {
    for role in ROLES {
        let body = part(role, "## How you end");
        assert!(body.contains("A last message without these fields is refused, and you are asked for it again."), "{}", role);
    }
}

// A guard: a test of a clause that holds already (#46).

#[test]
fn test_WB_ROLE_23_the_tester_writes_a_guard_runs_it_and_sees_it_pass() {
    let body = part("tester", "## What you produce");
    assert!(body.contains("- Every new test but a guard fails when you run it now"), "{}", body);
    assert!(body.contains("A test of a clause that holds already before the change"), "{}", body);
    assert!(body.contains("It is a guard: write it, run it and see it pass."), "{}", body);
}

#[test]
fn test_WB_ROLE_24_the_last_message_of_the_tester_lists_the_guards_after_red() {
    let found = fields(&part("tester", "## How you end"));
    let red = found.iter().position(|name| name == "red").expect("red");
    assert_eq!(found.get(red + 1).map(String::as_str), Some("guards"), "{:?}", found);
}

#[test]
fn test_WB_ROLE_25_the_builder_keeps_the_guards_passing_before_and_after() {
    let opening = split_parts(&prompt("build")).remove(0);
    assert!(opening.contains("guards, tests of what holds already, which pass before your change and after it"), "{}", opening);
}

// What a role reads is cut where the work needs it (#47).

#[test]
fn test_WB_ROLE_26_the_tester_reads_the_interface_the_issue_names_and_not_the_file() {
    let body = part("tester", "## What you read");
    assert!(body.contains("the interface of the thing under test that the issue names"), "{}", body);
    assert!(body.contains("Where it shares a file with the body, read only the interface and not the file."), "{}", body);
}

#[test]
fn test_WB_ROLE_27_every_role_that_reads_tests_reads_their_helpers() {
    for role in ROLES {
        assert!(part(role, "## What you read").contains("the helpers those tests use"), "{}", role);
    }
}

#[test]
fn test_WB_ROLE_28_a_helper_is_part_of_the_test_it_serves() {
    for role in ["tester", "build", "fix"] {
        assert!(part(role, "## What you never do").contains("A helper a test uses is part of that test."), "{}", role);
    }
}

#[test]
fn test_WB_ROLE_29_the_reviewer_reads_what_the_changed_code_calls() {
    let body = part("review", "## What you read");
    let point = body.split('\n').find(|line| line.starts_with("- What the changed code calls"));
    let point = point.unwrap_or_else(|| panic!("no point for what the changed code calls: {}", body));
    assert!(point.contains("nothing of how it was made"), "{}", point);
}

// A requirement names its cases, and one about a message gives its words (#48).

#[test]
fn test_WB_ROLE_30_the_tester_asks_for_the_cases_a_requirement_does_not_name() {
    let body = part("tester", "## What you never do");
    let point = body.split('\n').find(|line| line.starts_with("- You never guess the cases of a requirement."));
    let point = point.unwrap_or_else(|| panic!("no point for the cases: {}", body));
    assert!(point.contains("\"the same as\" or \"any\""), "{}", point);
    assert!(point.contains("ask which cases there are under `needs_person`"), "{}", point);
}

#[test]
fn test_WB_ROLE_31_the_tester_holds_a_message_to_no_words_of_its_own() {
    let body = part("tester", "## What you never do");
    let point = body.split('\n').find(|line| line.starts_with("- You never hold a message to words of your own."));
    let point = point.unwrap_or_else(|| panic!("no point for the words of a message: {}", body));
    assert!(point.contains("does not give its words, and does not say that any words do"), "{}", point);
    assert!(point.contains("`needs_person`"), "{}", point);
}

// A requirement added during a change, and a fix after the last round (#49).

#[test]
fn test_WB_ROLE_32_round_2_reviews_a_requirement_added_since_round_1_as_round_1_does() {
    let body = part("review", "## What you produce");
    let round_2 = body.split_once("In round 2,").expect("round 2").1;
    assert!(
        round_2.contains("- A requirement added to the change since round 1 is reviewed as in round 1, clause by clause."),
        "{}",
        round_2
    );
}

#[test]
fn test_WB_ROLE_33_a_fix_after_the_last_round_is_allowed_and_checked_by_a_person() {
    let body = part("fix", "## What you produce");
    let point = body.split('\n').find(|line| line.starts_with("- After the last round,"));
    let point = point.unwrap_or_else(|| panic!("no point for after the last round: {}", body));
    assert!(point.contains("only where a person has allowed it"), "{}", point);
    assert!(point.contains("that person checks it with the test that shows it"), "{}", point);
    assert!(point.contains("It is not a round of review."), "{}", point);
}

// The tester runs the project's checks on its commit (#50).

#[test]
fn test_WB_ROLE_34_the_tester_runs_the_checks_of_the_project_after_its_commit() {
    let body = part("tester", "## What you produce");
    let point = body.split('\n').find(|line| line.starts_with("- After the commit, run what the project runs before a change is handed over"));
    let point = point.unwrap_or_else(|| panic!("no point for the checks of the project: {}", body));
    assert!(point.contains("report under `checks` what fails for a reason other than the missing code"), "{}", point);
}

#[test]
fn test_WB_ROLE_35_the_last_message_of_the_tester_gives_the_checks_after_the_guards() {
    let found = fields(&part("tester", "## How you end"));
    let guards = found.iter().position(|name| name == "guards").expect("guards");
    assert_eq!(found.get(guards + 1).map(String::as_str), Some("checks"), "{:?}", found);
}

/// The line of a field of the last message, as the prompt writes it under how it ends.
fn field_line(role: &str, name: &str) -> String {
    let body = part(role, "## How you end");
    let start = format!("- `{}`: ", name);
    let line = body.split('\n').find(|line| line.starts_with(&start));
    line.unwrap_or_else(|| panic!("{}: no field {}", role, name)).to_string()
}

// The red of a file of tests that does not load yet (#60).

#[test]
fn test_WB_ROLE_39_a_file_that_does_not_load_for_the_name_the_issue_asks_for_is_red_for_every_test_in_it() {
    let body = part("tester", "## What you produce");
    assert!(body.contains("A file of tests that does not load, because a name it calls is missing,"), "{}", body);
    assert!(body.contains("is red for every test in it"), "{}", body);
    assert!(body.contains("when the missing name is the one the issue asks for and nothing else is missing"), "{}", body);
}

#[test]
fn test_WB_ROLE_39_the_tester_stops_when_each_test_was_seen_to_fail_on_its_own_or_with_its_file() {
    let body = part("tester", "## How you end");
    assert!(body.contains("you have seen each of them fail, on its own or with the file it is in (or pass, for a guard)"), "{}", body);
}

#[test]
fn test_WB_ROLE_40_the_tester_names_the_missing_names_under_red() {
    let body = part("tester", "## What you produce");
    assert!(body.contains("Name the missing names under `red`."), "{}", body);
    let red = field_line("tester", "red");
    assert!(red.contains("the names it misses"), "{}", red);
}

#[test]
fn test_WB_ROLE_41_the_tester_also_runs_each_test_against_a_stand_in_that_gives_a_wrong_answer() {
    let body = part("tester", "## What you produce");
    let point = body.split('\n').find(|line| line.starts_with("- Such a red rests on one reason for the whole file."));
    let point = point.unwrap_or_else(|| panic!("no point for the stand-in: {}", body));
    assert!(point.contains("Where the language allows it,"), "{}", point);
    assert!(point.contains("against a stand-in that gives a wrong answer"), "{}", point);
    assert!(point.contains("in a scratch copy of the checkout that you never commit"), "{}", point);
    assert!(point.contains("say so under `red`"), "{}", point);
    assert!(point.contains("Leave this out where the project's own text says not to"), "{}", point);
}

#[test]
fn test_WB_ROLE_41_the_stand_in_is_never_committed() {
    // The tester still writes no stand-in for the code that it commits.
    let body = part("tester", "## What you never do");
    let point = body.split('\n').find(|line| line.starts_with("- You never write the code that makes your tests pass"));
    let point = point.unwrap_or_else(|| panic!("no point for the code: {}", body));
    assert!(point.contains("and no stand-in for it that you commit"), "{}", point);
}

#[test]
fn test_WB_ROLE_42_the_reviewer_sees_each_test_fail_on_its_own_where_the_red_was_for_a_whole_file() {
    let body = part("review", "## What you produce");
    let round_1 = body.split_once("In round 2,").expect("round 2").0;
    let point = round_1.split('\n').find(|line| line.contains("red before the code was for the whole file"));
    let point = point.unwrap_or_else(|| panic!("no point for a red of a whole file: {}", round_1));
    assert!(point.contains("see each of those tests fail on its own"), "{}", point);
    assert!(point.contains("Break the code"), "{}", point);
    assert!(point.contains("in a scratch copy that you never commit"), "{}", point);
    assert!(point.contains("A test that does not fail is a finding."), "{}", point);
}

#[test]
fn test_WB_ROLE_42_the_reviewer_breaks_code_only_in_a_scratch_copy() {
    let body = part("review", "## What you never do");
    let point = body.split('\n').find(|line| line.starts_with("- You never fix anything:"));
    let point = point.unwrap_or_else(|| panic!("no point for fixing: {}", body));
    assert!(point.contains("no edit to the change and no commit"), "{}", point);
    assert!(point.contains("scratch copy"), "{}", point);
}

// A guard the issue names is the existing test, run as it is (#62).

/// The point of the tester's part on what it produces that is about a guard the issue names.
fn named_guard_point() -> String {
    let body = part("tester", "## What you produce");
    let point = body.split('\n').find(|line| line.starts_with("- Where the issue names an existing test as the guard of a clause,"));
    point.unwrap_or_else(|| panic!("no point for a guard the issue names: {}", body)).to_string()
}

#[test]
fn test_WB_ROLE_43_a_guard_the_issue_names_is_run_as_it_is_and_listed_by_its_name() {
    let point = named_guard_point();
    assert!(point.contains("that test is the guard"), "{}", point);
    assert!(point.contains("Run it as it is and list it under `guards` by its name."), "{}", point);
    let guards = field_line("tester", "guards");
    assert!(guards.contains("An existing test the issue names is listed by its name."), "{}", guards);
}

#[test]
fn test_WB_ROLE_44_the_tester_writes_no_copy_of_a_guard_the_issue_names() {
    let point = named_guard_point();
    assert!(point.contains("Write no copy of it."), "{}", point);
}

#[test]
fn test_WB_ROLE_45_where_a_named_guard_does_not_cover_the_clause_the_tester_says_so_under_untested() {
    let point = named_guard_point();
    assert!(point.contains("Where you think it does not cover the clause, say so under `untested`."), "{}", point);
}

#[test]
fn test_WB_ROLE_46_a_new_test_that_repeats_an_existing_one_is_a_note() {
    let body = part("review", "## What you produce");
    let round_1 = body.split_once("In round 2,").expect("round 2").0;
    let point = round_1.split('\n').find(|line| line.contains("new test that repeats an existing one"));
    let point = point.unwrap_or_else(|| panic!("no point for a repeated test: {}", round_1));
    assert!(point.contains("is a `note`"), "{}", point);
}

// The session's own word on its model and effort, and the stamp (#69).

/// The text of how a role ends after the list of its fields.
fn after_fields(role: &str) -> String {
    let body = part(role, "## How you end");
    let start = body.find("- `needs_person`: ").unwrap_or_else(|| panic!("{}: no field needs_person", role));
    body[start..].split_once('\n').map(|(_, rest)| rest.to_string()).unwrap_or_default()
}

#[test]
fn test_WB_ROLE_36_model_and_effort_are_what_the_session_says_of_itself() {
    for role in ROLES {
        for name in ["model", "effort"] {
            let line = field_line(role, name);
            assert!(line.contains(": what your session says of itself, or `unknown`."), "{}: {}", role, line);
        }
    }
}

#[test]
fn test_WB_ROLE_37_the_stamp_on_the_issue_says_what_the_session_was_handed_out_as() {
    for role in ROLES {
        let text = after_fields(role);
        assert!(text.contains("The stamp on the issue says what you were handed out as, and it is what counts."), "{}: {}", role, text);
    }
}

#[test]
fn test_WB_ROLE_38_the_session_reads_its_stamp_and_says_where_its_own_word_differs() {
    for role in ROLES {
        let text = after_fields(role);
        assert!(text.contains("Read the stamp of your own session on the issue, where it is given"), "{}: {}", role, text);
        assert!(text.contains("Where your own word differs from it, say so under `model` and `effort`."), "{}: {}", role, text);
    }
}

// The breakdown: it reads the issue and the code, and writes the issue's file (#75).

/// The point of a part of the breakdown's prompt that starts with `start`.
fn breakdown_point(heading: &str, start: &str) -> String {
    let body = part("breakdown", heading);
    let point = body.split('\n').find(|line| line.starts_with(start));
    point.unwrap_or_else(|| panic!("no point that starts with {:?}: {}", start, body)).to_string()
}

#[test]
fn test_WB_ROLE_48_the_breakdown_reads_the_issue_at_its_address_and_the_code_in_the_checkout() {
    let issue = breakdown_point("## What you read", "- The issue {issue}, at {issue_address},");
    assert!(issue.contains("as a person wrote it"), "{}", issue);
    let code = breakdown_point("## What you read", "- The code in {checkout}:");
    for what in ["the source", "the tests and the helpers those tests use", "the settings of the project's checks"] {
        assert!(code.contains(what), "{}: {}", what, code);
    }
    assert!(code.contains("You are the one role that reads the code without a list"), "{}", code);
}

#[test]
fn test_WB_ROLE_49_the_breakdown_writes_the_issue_s_file_in_its_form() {
    let body = part("breakdown", "## What you produce");
    let point = breakdown_point("## What you produce", "- The issue's file, `.watchbill/issues/{issue}.yml`,");
    assert!(point.contains("every key there and in this order"), "{}", point);
    // The form, as a block of YAML, with each key at the start of a line in the order the program holds it to.
    let form = body.split_once("```yaml\n").expect("the form").1.split_once("```").expect("the end of the form").0;
    let keys: Vec<&str> = form
        .split('\n')
        .filter(|line| !line.starts_with(' ') && line.contains(':'))
        .map(|line| line.split(':').next().unwrap())
        .collect();
    assert_eq!(
        keys,
        ["issue", "from", "requirements", "interface", "changes", "reads", "existing_tests_changed", "questions"],
        "{}",
        form
    );
    assert!(form.starts_with("issue: '{issue}'\n"), "{}", form);
    for line in ["  - id:", "    text:", "    source:", "    guard:", "  tester:", "  build:", "  review:", "  fix:"] {
        assert!(form.split('\n').any(|each| each.starts_with(line)), "{}: {}", line, form);
    }
}

#[test]
fn test_WB_ROLE_50_the_breakdown_commits_the_file_alone_on_the_branch() {
    let point = breakdown_point("## What you produce", "- Commit the file alone, on the branch {branch},");
    assert!(point.contains("The commit holds that file and nothing else."), "{}", point);
    let end = part("breakdown", "## How you end");
    assert!(end.contains("Stop when the issue's file is committed alone on {branch}"), "{}", end);
}

#[test]
fn test_WB_ROLE_51_the_breakdown_never_changes_the_issue_the_code_or_a_test() {
    let point = breakdown_point("## What you never do", "- You never change the issue, the code or a test.");
    assert!(point.contains("not its text, its title or what is written under it"), "{}", point);
    assert!(point.contains("A helper a test uses is part of that test."), "{}", point);
}

#[test]
fn test_WB_ROLE_52_the_breakdown_puts_what_the_issue_leaves_open_under_questions_and_does_not_answer_it() {
    let point = breakdown_point("## What you produce", "- What the issue leaves open goes under `questions`");
    assert!(point.contains("You do not answer it"), "{}", point);
    assert!(point.contains("A person answers"), "{}", point);
    let never = breakdown_point("## What you never do", "- You never answer what the issue leaves open.");
    assert!(never.contains("`questions`"), "{}", never);
}

#[test]
fn test_WB_ROLE_53_the_breakdown_finds_the_guards_among_the_existing_tests() {
    let point = breakdown_point("## What you produce", "- Find the guards among the existing tests.");
    assert!(point.contains("name that test under `guard` of the requirement"), "{}", point);
    assert!(point.contains("Name no test you have not found in the code."), "{}", point);
}

#[test]
fn test_WB_ROLE_54_the_breakdown_lists_for_each_role_what_it_may_read_as_narrow_as_the_work_allows() {
    let point = breakdown_point("## What you produce", "- `reads`: for each role,");
    assert!(point.contains("what it may read beside the paths of `changes`"), "{}", point);
    assert!(point.contains("as narrow as the work allows"), "{}", point);
}

#[test]
fn test_WB_ROLE_55_the_breakdown_reads_the_settings_of_the_checks_so_that_what_a_check_needs_is_in_the_issue() {
    let point = breakdown_point("## What you produce", "- Read the settings of the project's checks,");
    assert!(point.contains("such as a built page or a generated file"), "{}", point);
    assert!(point.contains("is in the issue"), "{}", point);
}

#[test]
fn test_WB_ROLE_56_the_first_line_of_the_breakdown_says_the_program_cleared_the_runner_and_the_session_does_not_judge_it() {
    let text = prompt("breakdown");
    let first: Vec<&str> = text.split('\n').take(2).collect();
    assert_eq!(first.get(1).copied(), Some(""), "{:?}", first);
    let line = first.first().copied().unwrap_or_default();
    assert!(line.starts_with("You read more of the code than any other role, and all that you read goes to the model that answers you."), "{:?}", first);
    assert!(line.contains("The program handed you this prompt only after the organisation's file cleared your runner for the breakdown at this repository's level; your stamp names the runner, the model and the effort."), "{:?}", first);
    assert!(line.contains("Do not judge the clearance yourself, and do not stop for it."), "{:?}", first);
}

#[test]
fn test_WB_ROLE_93_no_role_text_asks_the_session_to_check_a_clearance_of_its_runner() {
    for role in ROLES {
        let text = prompt(role);
        for phrase in ["local model", "only where a person has cleared"] {
            assert!(!text.contains(phrase), "{} holds {:?}", role, phrase);
        }
    }
}

#[test]
fn test_WB_ROLE_57_the_last_message_of_the_breakdown_has_its_fields_in_order() {
    let found = fields(&part("breakdown", "## How you end"));
    assert_eq!(found, ["model", "effort", "file", "requirements", "questions", "asked", "needs_person"], "{:?}", found);
    let requirements = field_line("breakdown", "requirements");
    assert!(requirements.contains("how many requirements the file holds"), "{}", requirements);
    let questions = field_line("breakdown", "questions");
    assert!(questions.contains("how many questions the file holds"), "{}", questions);
    let file = field_line("breakdown", "file");
    assert!(file.contains("the commit that holds it"), "{}", file);
}

// The level of a finding stands where the program reads it (#82, for `wb handover` of #88).

#[test]
fn test_WB_ROLE_68_a_finding_of_the_review_starts_with_its_number_and_its_level() {
    let body = part("review", "## What you produce");
    let round_1 = body.split_once("In round 2,").expect("round 2").0;
    assert!(round_1.contains("starts with its number and its level: `<n>. must` or `<n>. note`"), "{}", round_1);
    assert!(!prompt("review").contains("F1"), "the review still numbers its findings F1");
    let findings = field_line("review", "findings");
    assert!(findings.contains("`<n>. must` or `<n>. note`"), "{}", findings);
}

#[test]
fn test_WB_ROLE_68_the_fix_reads_a_finding_by_its_number_and_its_level() {
    let body = part("fix", "## What you produce");
    assert!(body.contains("A finding starts with its number and its level: `<n>. must` or `<n>. note`."), "{}", body);
}

// The breakdown asks its questions on the issue, and a second session folds the answers in (#95). From
// revision 0.0.9 it asks in one comment for the session, and a person answers with `wb answer` (#145).

#[test]
fn test_WB_ROLE_62_a_second_fresh_session_folds_the_answers_into_the_file_and_commits_it_alone() {
    let point = breakdown_point("## What you produce", "- A second session, a fresh one, folds the answers in.");
    for what in [
        "you read the issue with its comments",
        "take each answer of a person as part of the issue",
        "in the requirements, `changes`, `reads` or `existing_tests_changed`",
        "remove the question answered",
        "A question with no answer you keep",
        "You commit the file alone again.",
    ] {
        assert!(point.contains(what), "{}: {}", what, point);
    }
}

#[test]
fn test_WB_ROLE_63_the_last_message_of_the_breakdown_names_the_comments_it_posted() {
    let found = fields(&part("breakdown", "## How you end"));
    let at = found.iter().position(|name| name == "asked").expect("no field asked");
    assert_eq!(found[at - 1], "questions", "{:?}", found);
    let asked = field_line("breakdown", "asked");
    assert!(asked.contains("the comments you posted"), "{}", asked);
    let questions = field_line("breakdown", "questions");
    assert!(questions.contains("open"), "{}", questions);
}

// The prompts after the second run (#98). From revision 0.0.9 the breakdown alone pushes: the tester, the
// build and the fix run in a view with no remote (#126), and a person pushes their commits (#145).

/// The line of a part of a role's prompt that starts with "Stop when".
fn stop_line(role: &str) -> String {
    let body = part(role, "## How you end");
    let stop = body.split('\n').find(|line| line.starts_with("Stop when")).map(String::from);
    stop.unwrap_or_else(|| panic!("{}: no line that starts with Stop when", role))
}

#[test]
fn test_WB_ROLE_91_the_breakdown_says_to_commit_and_then_push_to_the_branch() {
    let body = part("breakdown", "## What you produce");
    let found = body.split('\n').any(|line| {
        let lower = line.to_lowercase();
        match (lower.find("commit"), lower.find("push")) {
            (Some(commit), Some(push)) => commit < push && lower[push..].contains("to {branch}"),
            _ => false,
        }
    });
    assert!(found, "no point says to commit and then push to {{branch}}: {}", body);
    assert!(stop_line("breakdown").contains("pushed to {branch}"), "{}", stop_line("breakdown"));
}

#[test]
fn test_WB_ROLE_91_the_last_message_of_the_breakdown_names_the_commit_that_was_pushed() {
    let file = field_line("breakdown", "file");
    assert!(file.contains("the commit that holds it, as pushed to {branch}"), "{}", file);
}

// The roles that commit in a view, and push nothing (#145).
const COMMITTERS: [&str; 3] = ["tester", "build", "fix"];

#[test]
fn test_WB_ROLE_88_the_tester_the_build_and_the_fix_commit_on_the_branch_and_push_nothing() {
    for role in COMMITTERS {
        let body = part(role, "## What you produce");
        let point = body.split('\n').find(|line| line.contains("You push nothing and open no change: a person pushes the branch."));
        let point = point.unwrap_or_else(|| panic!("{}: no point says that a person pushes: {}", role, body));
        assert!(point.to_lowercase().contains("commit") && point.contains("{branch}"), "{}: {}", role, point);
        let never = part(role, "## What you never do");
        assert!(never.contains("- You never push, and never open a change."), "{}: {}", role, never);
        let stop = stop_line(role);
        assert!(stop.contains("committed on {branch}"), "{}: {}", role, stop);
        assert!(!stop.contains("push"), "{}: {}", role, stop);
    }
}

#[test]
fn test_WB_ROLE_88_the_tester_the_build_and_the_fix_are_not_told_to_push_anywhere() {
    for role in COMMITTERS {
        for line in prompt(role).split('\n') {
            let lower = line.to_lowercase();
            // What the project's own text says about pushing is read (WB-ROLE-20), and a person pushes.
            let allowed = ["how work is pushed and posted here", "you push nothing", "you never push", "a person pushes"];
            let rest = allowed.iter().fold(lower.clone(), |text, phrase| text.replace(phrase, ""));
            assert!(!rest.contains("push"), "{}: {}", role, line);
        }
    }
}

#[test]
fn test_WB_ROLE_89_the_last_message_names_the_last_commit_made() {
    for role in COMMITTERS {
        let found = fields(&part(role, "## How you end"));
        assert!(found.iter().any(|name| name == "committed"), "{}: {:?}", role, found);
        assert!(!found.iter().any(|name| name == "pushed"), "{}: {:?}", role, found);
        let committed = field_line(role, "committed");
        assert!(committed.contains("your last commit") || committed.contains("the commit of your tests"), "{}: {}", role, committed);
        assert!(committed.contains("on {branch}"), "{}: {}", role, committed);
    }
    let found = fields(&part("build", "## How you end"));
    assert!(!found.iter().any(|name| name == "change"), "{:?}", found);
}

#[test]
fn test_WB_ROLE_90_the_message_of_the_build_s_last_commit_is_the_description_of_the_change() {
    let body = part("build", "## What you produce");
    assert!(
        body.contains("The message of your last commit names the issue {issue}, the requirements {requirements}, and anything in the change that a reviewer would not expect: the change is opened from your commits."),
        "{}",
        body
    );
}

#[test]
fn test_WB_ROLE_65_a_requirement_is_what_the_change_must_do_and_a_test_or_a_reading_of_the_code_can_show() {
    let point = breakdown_point("## What you produce", "- A requirement is what the change must do,");
    assert!(point.contains("and that a test or a reading of the code can show"), "{}", point);
    assert!(point.contains("A clause that no test can show stays a requirement and says so in its text"), "{}", point);
    assert!(point.contains("so that the review knows to read for it"), "{}", point);
}

#[test]
fn test_WB_ROLE_65_how_an_existing_test_changes_goes_under_existing_tests_changed_never_as_a_requirement() {
    let point = breakdown_point("## What you produce", "- A requirement is what the change must do,");
    assert!(point.contains("How an existing test changes goes under `existing_tests_changed`, with the reason, and never as a requirement."), "{}", point);
    let list = breakdown_point("## What you produce", "- `existing_tests_changed`:");
    assert!(list.contains("by name, and the reason"), "{}", list);
}

#[test]
fn test_WB_ROLE_78_the_breakdown_s_lists_must_pass_the_check_of_needs() {
    let point = breakdown_point("## What you produce", "- Your lists must pass the check of `needs`.");
    assert!(point.contains("`wb accept` runs it for each path of `changes` and of the tester's and the build's `reads`"), "{}", point);
    assert!(point.contains("in neither that role's `reads` nor `changes`"), "{}", point);
}

#[test]
fn test_WB_ROLE_78_the_breakdown_runs_the_command_itself_before_it_commits_the_issue_s_file() {
    let point = breakdown_point("## What you produce", "- Your lists must pass the check of `needs`.");
    assert!(point.contains("Run the command yourself, for each of those paths, before you commit the issue's file"), "{}", point);
    let read = breakdown_point("## What you read", "- The settings of Watchbill in `.watchbill/`");
    assert!(read.contains("`needs`") && read.contains("`build_files`"), "{}", read);
}

#[test]
fn test_WB_ROLE_77_no_role_file_holds_the_draft_line() {
    // The fix ran on a project for the first time at 0.0.7, so the line left its file (#113).
    let mut names: Vec<String> = fs::read_dir(roles_folder())
        .expect("the roles folder could not be read")
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert!(names.contains(&"fix.md".to_string()), "{:?}", names);
    for name in names {
        let text = String::from_utf8(raw(&name)).unwrap();
        assert!(!text.contains(DRAFT), "{}", name);
        assert!(!text.to_lowercase().contains("has not yet run on a project"), "{}", name);
    }
}

#[test]
fn test_WB_ROLE_67_where_the_address_shows_no_description_the_review_says_so_as_a_finding() {
    let body = part("review", "## What you read");
    let point = body.split('\n').find(|line| line.starts_with("- Then its description and its commit messages,"));
    let point = point.unwrap_or_else(|| panic!("no point for the description: {}", body));
    assert!(point.contains("Where {change_address} shows no description of the change, say so as a finding"), "{}", point);
    assert!(point.contains("do not judge the change as if it had none"), "{}", point);
}

// A role started alone (#116).

/// The point of "## What you read" that starts with `lead`, or a failure that names the role.
fn read_point(role: &str, lead: &str) -> String {
    let body = part(role, "## What you read");
    let point = body.split('\n').find(|line| line.starts_with(lead)).map(String::from);
    point.unwrap_or_else(|| panic!("{}: no point that starts with {}", role, lead))
}

#[test]
fn test_WB_ROLE_75_the_review_and_the_fix_say_that_with_no_issue_the_change_s_description_is_the_spec() {
    for role in ["review", "fix"] {
        let point = read_point(role, "- Where the issue is none,");
        assert!(point.contains("the change was made with no issue, and its description, at {change_address}, is the spec"), "{}: {}", role, point);
        assert!(point.contains("read it wherever this prompt says the issue"), "{}: {}", role, point);
        assert!(point.contains("Your stamp, where it is given, is on the change."), "{}: {}", role, point);
    }
}

#[test]
fn test_WB_ROLE_75_the_review_says_an_empty_description_in_a_finding_and_the_fix_under_seen() {
    let review = read_point("review", "- Where the issue is none,");
    assert!(review.contains("Where the description is empty, there is no spec: say so as a finding"), "{}", review);
    let fix = read_point("fix", "- Where the issue is none,");
    assert!(fix.contains("Where the description is empty, there is no spec: say so under `seen`"), "{}", fix);
    let seen = part("fix", "## How you end");
    assert!(seen.contains("- `seen`:"), "{}", seen);
}

#[test]
fn test_WB_ROLE_79_the_tester_and_the_build_say_that_with_no_file_they_work_from_the_issue_s_numbered_items() {
    for role in ["tester", "build"] {
        let point = read_point(role, "- Where no part of the issue's file follows this text,");
        assert!(point.contains("the issue has no file"), "{}: {}", role, point);
        assert!(point.contains("you work from the issue's own numbered items: they are the requirements"), "{}: {}", role, point);
    }
}

// Every role asks on the issue, plainly and with a suggested answer (#113).

// The roles after the breakdown, which ask what only a person can settle under `needs_person`.
const ASKERS: [&str; 4] = ["tester", "build", "review", "fix"];

/// The name of a question of the role, up to its number: `{issue}.q` for the breakdown (#95),
/// `{issue}.<role>.q` for the others.
fn question_name(role: &str) -> String {
    if role == "breakdown" {
        "{issue}.q".to_string()
    } else {
        format!("{{issue}}.{}.q", role)
    }
}

/// The line of a role's prompt that starts with `lead`, or a failure that names the role.
fn line_of(role: &str, lead: &str) -> String {
    let text = prompt(role);
    let line = text.split('\n').find(|line| line.starts_with(lead)).map(String::from);
    line.unwrap_or_else(|| panic!("{}: no line that starts with {:?}", role, lead))
}

/// The point that says the points of `needs_person` are posted on the issue, in one comment (#145).
fn asking_point(role: &str) -> String {
    line_of(role, "- The points of `needs_person` are also posted on the issue, in one comment for the session,")
}

/// The first words of a role's comments of questions and answers: `{issue}` for the breakdown, which keeps the
/// names #95 gave its questions, and `{issue}.<role>` for the others (#145).
fn questions_of(role: &str) -> String {
    if role == "breakdown" {
        "{issue}".to_string()
    } else {
        format!("{{issue}}.{}", role)
    }
}

/// The block of a role's prompt that shows the form of its comment of questions.
fn questions_form(role: &str) -> String {
    let text = prompt(role);
    let start = format!("```text\n[questions] {} q1 q2\n", questions_of(role));
    let (_, rest) = text.split_once(&start).unwrap_or_else(|| panic!("{}: no block that starts with {:?}", role, start));
    format!("[questions] {} q1 q2\n{}", questions_of(role), rest.split_once("```").unwrap_or_else(|| panic!("{}: the block does not end", role)).0)
}

#[test]
fn test_WB_ANSWER_9_the_four_post_their_questions_in_one_comment_whose_first_line_names_them() {
    for role in ASKERS {
        let point = asking_point(role);
        let first = format!("Its first line is `[questions] {}` and the name of each question it holds, as `[questions] {} q1 q2`.", questions_of(role), questions_of(role));
        assert!(point.contains(&first), "{}: {}", role, point);
        assert!(point.contains("before your last message"), "{}: {}", role, point);
    }
}

#[test]
fn test_WB_ANSWER_9_the_breakdown_posts_its_questions_in_one_comment_whose_first_line_names_them() {
    let point = breakdown_point("## What you produce", "- The new questions are also posted on the issue, in one comment for the session,");
    assert!(point.contains("Its first line is `[questions] {issue}` and the name of each question it holds, as `[questions] {issue} q1 q2`."), "{}", point);
    assert!(point.contains("a question named `{issue}.q<n>` under `questions` is `q<n>` in the comment"), "{}", point);
}

#[test]
fn test_WB_ANSWER_9_each_question_is_one_line_and_the_rest_is_folded_under_details() {
    for role in ROLES {
        let text = prompt(role);
        assert!(text.contains("- Each question is one line, its name and what is unclear. The rest is folded under `<details>`, which the tracker shows closed. The comment is in this form:"), "{}", role);
        let form = questions_form(role);
        let lines: Vec<&str> = form.split('\n').collect();
        assert_eq!(lines[1], "q1: <what is unclear>", "{}: {}", role, form);
        assert_eq!(lines[2], "<details>", "{}: {}", role, form);
        for line in ["Why it matters: <one or two sentences>", "1. <the first choice>", "2. <the second choice>", "Suggested: 1, because <the reason>", "</details>", "q2: <what is unclear>"] {
            assert!(lines.contains(&line), "{}: {}: {}", role, line, form);
        }
        // The details end before the next question starts.
        let end = lines.iter().position(|line| *line == "</details>").unwrap();
        let next = lines.iter().position(|line| *line == "q2: <what is unclear>").unwrap();
        assert!(end < next, "{}: {}", role, form);
    }
}

#[test]
fn test_WB_ANSWER_9_each_question_says_whether_it_changes_the_issue_s_file() {
    for role in ROLES {
        let form = questions_form(role);
        let said: Vec<&str> = form.split('\n').filter(|line| line.starts_with("changes the issue's file: ")).collect();
        assert!(!said.is_empty(), "{}: {}", role, form);
        for line in said {
            assert!(["changes the issue's file: yes", "changes the issue's file: no"].contains(&line), "{}: {}", role, line);
        }
        let point = line_of(role, "  - Whether the answer changes the issue's file:");
        assert!(point.contains("`changes the issue's file: yes`"), "{}: {}", role, point);
    }
    // A second breakdown folds every answer of the breakdown into the file.
    let point = line_of("breakdown", "  - Whether the answer changes the issue's file:");
    assert!(point.contains("always `changes the issue's file: yes`"), "{}", point);
    for role in ASKERS {
        let point = line_of(role, "  - Whether the answer changes the issue's file:");
        assert!(point.contains("and `no` where it may not"), "{}: {}", role, point);
    }
}

#[test]
fn test_WB_ANSWER_10_the_number_of_a_question_counts_on_within_the_role_from_the_highest_on_the_issue() {
    for role in ROLES {
        let text = prompt(role);
        let counts = format!(
            "`<n>` counts on from the highest number of the {}'s questions already on the issue, in `[questions] {}` and `[question] {}<n>` comments,",
            role,
            questions_of(role),
            question_name(role)
        );
        assert!(text.contains(&counts), "{}: {}", role, counts);
        assert!(text.contains("so that no name is used twice."), "{}", role);
    }
}

#[test]
fn test_WB_ROLE_83_a_question_is_never_posted_twice() {
    for role in ROLES {
        let text = prompt(role);
        assert!(text.contains("You post no question twice: a question that stands on the issue already is not posted again."), "{}", role);
    }
}

#[test]
fn test_WB_ROLE_82_with_no_issue_the_review_and_the_fix_post_their_questions_on_the_change() {
    for role in ["review", "fix"] {
        let point = asking_point(role);
        assert!(point.contains("Where the issue is none, they are posted on the change, and `{change}` stands in place of `{issue}` in each name."), "{}: {}", role, point);
    }
}

/// The point that says how a person answers.
fn answering_point(role: &str) -> String {
    let lead = format!("- A person answers with `wb answer`, in one comment whose first line is `[answers] {}` and the names of the questions it answers,", questions_of(role));
    line_of(role, &lead)
}

#[test]
fn test_WB_ROLE_84_a_person_answers_with_wb_answer_in_one_comment_a_line_for_each_question() {
    for role in ROLES {
        let point = answering_point(role);
        assert!(point.contains("then one line for each: `q1: 2`, or `q1: <words>`."), "{}: {}", role, point);
    }
}

#[test]
fn test_WB_ROLE_84_an_answer_counts_only_from_a_person() {
    for role in ROLES {
        let point = answering_point(role);
        assert!(point.contains("An answer counts only from a person."), "{}: {}", role, point);
        assert!(point.contains("a comment that starts with `[question]` or `[questions]` is never an answer"), "{}: {}", role, point);
    }
}

/// The point that says what an answer does.
fn answer_point(role: &str) -> String {
    line_of(role, "- An answer that changes what the issue's file says is not yours to take")
}

#[test]
fn test_WB_ROLE_71_an_answer_that_changes_the_issue_s_file_is_not_taken_by_the_role() {
    for role in ASKERS {
        let point = answer_point(role);
        assert!(point.contains("A person changes the file and accepts it again, and your role is started again."), "{}: {}", role, point);
    }
}

#[test]
fn test_WB_ROLE_80_any_other_answer_is_taken_by_the_next_session_of_the_role() {
    for role in ASKERS {
        let point = answer_point(role);
        assert!(point.contains("Any other answer is taken by the next session of your role, which reads the issue with its comments."), "{}: {}", role, point);
    }
}

#[test]
fn test_WB_ROLE_80_the_four_read_the_issue_with_its_comments() {
    for role in ASKERS {
        let point = read_point(role, "- The issue {issue}, at {issue_address},");
        assert!(point.contains("with its comments"), "{}: {}", role, point);
        assert!(point.contains("the questions of your role, the answers of a person to them, and your stamp"), "{}: {}", role, point);
    }
}

#[test]
fn test_WB_ROLE_72_the_last_messages_of_the_four_end_with_needs_person_and_then_asked() {
    for role in ASKERS {
        let found = fields(&part(role, "## How you end"));
        assert!(found.len() >= 2, "{}: {:?}", role, found);
        assert_eq!(found[found.len() - 2..], ["needs_person", "asked"], "{}: {:?}", role, found);
        let asked = field_line(role, "asked");
        assert!(asked.contains("the comments you posted, each by its first line, or `none`."), "{}: {}", role, asked);
    }
}

#[test]
fn test_WB_ROLE_87_a_question_holds_what_is_unclear_why_it_matters_the_choices_a_suggestion_and_whether_it_changes_the_file() {
    for role in ROLES {
        let text = prompt(role);
        for what in [
            "A question is written so that a person can answer it without opening the code. In plain words, it holds what is unclear, why it matters, the choices, a suggested answer and whether the answer changes the issue's file, and nothing else:",
            "  - What is unclear: one sentence, in the issue's own words where it has them.",
            "  - Why it matters: what the change does differently by the answer, in one or two sentences, without the code's names unless the person needs them to answer.",
            "  - The choices, numbered from 1, where there are any.",
            "  - A suggested answer, marked as a suggestion, with its reason in one sentence: `Suggested: 1, because …`. It names its choice by number. You never act on it before a person answers.",
            "  - Whether the answer changes the issue's file: ",
            "  - Nothing else: no recap of the issue, and no list of what you read.",
        ] {
            assert!(text.contains(what), "{}: {}", role, what);
        }
    }
}

#[test]
fn test_WB_ROLE_87_a_question_is_never_edited_once_posted() {
    for role in ROLES {
        let point = line_of(role, "- A question is never edited once posted,");
        assert!(point.contains("so that a number always means what it meant when the person answered."), "{}: {}", role, point);
        assert!(point.contains("A question that has to change is posted again as a new question, with the next number."), "{}: {}", role, point);
    }
}

#[test]
fn test_WB_ROLE_85_an_answer_is_read_by_number_or_in_words() {
    for role in ROLES {
        let text = prompt(role);
        for what in [
            "- An answer is read so:",
            "  - By number: `q1: 2` takes choice 2 of that question, in its words.",
            "  - In words: any other text after the name is the answer, such as \"2, but only for the first command\", or plain words.",
        ] {
            assert!(text.contains(what), "{}: {}", role, what);
        }
    }
}

#[test]
fn test_WB_ROLE_85_a_number_that_is_not_a_choice_leaves_the_question_open() {
    for role in ROLES {
        let point = line_of(role, "  - In words:");
        assert!(point.contains("A number that is not one of the question's choices is no answer, and the question stays open."), "{}: {}", role, point);
    }
}

#[test]
fn test_WB_ROLE_86_the_forms_of_revision_0_0_8_are_still_read_and_not_posted() {
    for role in ROLES {
        let point = line_of(role, "- An issue begun under revision 0.0.8 holds the forms of that revision.");
        let question = format!("`[question] {}<n>`", question_name(role));
        let answer = format!("`[answer] {}<n>`", question_name(role));
        for what in [
            "Read them, and do not post them:",
            question.as_str(),
            answer.as_str(),
            "several in one comment, each on a line of its own",
            "`[answer] {issue} suggested`, which takes the suggested answer of every question posted before it that has no answer yet",
        ] {
            assert!(point.contains(what), "{}: {}: {}", role, what, point);
        }
    }
}

#[test]
fn test_WB_ROLE_76_every_role_gives_its_model_and_effort_from_its_stamp() {
    for role in ROLES {
        let text = after_fields(role);
        let stamp = format!("Your model and effort are in your stamp on the issue: the latest comment whose first line starts with `[stamp] {}`.", role);
        assert!(text.contains(&stamp), "{}: {}", role, text);
        assert!(text.contains("give `model` and `effort` in your last message from there"), "{}: {}", role, text);
    }
}

// A new folder in `changes` ends with `/` (#197): since #192 a path that is not in `HEAD` and has no `/` covers
// that path only.

#[test]
fn test_WB_ROLE_92_the_breakdown_says_a_new_folder_in_changes_ends_with_a_slash_and_a_file_never_does() {
    let body = part("breakdown", "## What you produce");
    let found = body.split('\n').any(|line| {
        let lower = line.to_lowercase();
        lower.contains("`changes`")
            && lower.contains("folder")
            && lower.contains("not yet in the repository")
            && lower.contains("ends with `/`")
            && lower.contains("a file never does")
    });
    assert!(found, "no point says that a new folder in `changes` ends with `/` and a file never does: {}", body);
}

// ---- the review and classed code (#227) ----

#[test]
fn test_WB_ROLE_94_the_review_is_told_the_classed_paths_and_those_the_change_touches_by_the_program() {
    let text = prompt("review");
    assert!(text.contains("{classed}"), "the review does not use {{classed}}");
    assert!(text.contains("{classed_touched}"), "the review does not use {{classed_touched}}");
    assert!(text.contains("Say first whether the change touches classed code"), "{}", text);
}

#[test]
fn test_WB_ROLE_95_the_review_names_the_issues_file_and_its_acceptance_as_the_process_own_files_and_no_finding() {
    let text = prompt("review");
    assert!(text.contains(".watchbill/issues/{issue}.yml"), "{}", text);
    assert!(text.contains(".watchbill/issues/{issue}.accepted"), "{}", text);
    assert!(text.contains("They are not part of what the issue asks and are not a finding."), "{}", text);
    assert!(text.contains("A change to the issue's file after the acceptance is a finding."), "{}", text);
}

#[test]
fn test_WB_ROLE_96_the_review_does_not_ask_the_session_to_find_which_code_is_classed() {
    let text = prompt("review");
    assert!(!text.contains("where the project says which code is classed"), "{}", text);
    assert!(!text.contains("If you cannot tell, say that you cannot."), "{}", text);
    assert!(text.contains("Never say that the classed code cannot be told where the program gave the paths."), "{}", text);
}

#[test]
fn test_WB_ROLE_97_no_other_role_speaks_of_classed_code() {
    for role in ["breakdown", "tester", "build", "fix"] {
        assert!(!prompt(role).to_lowercase().contains("classed"), "{} speaks of classed code", role);
    }
}
