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
fn test_WB_ROLE_26_the_tester_reads_the_interface_the_task_names_and_not_the_file() {
    let body = part("tester", "## What you read");
    assert!(body.contains("the interface of the thing under test that the task names"), "{}", body);
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
fn test_WB_ROLE_39_a_file_that_does_not_load_for_the_name_the_task_asks_for_is_red_for_every_test_in_it() {
    let body = part("tester", "## What you produce");
    assert!(body.contains("A file of tests that does not load, because a name it calls is missing,"), "{}", body);
    assert!(body.contains("is red for every test in it"), "{}", body);
    assert!(body.contains("when the missing name is the one the task asks for and nothing else is missing"), "{}", body);
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

// A guard the task names is the existing test, run as it is (#62).

/// The point of the tester's part on what it produces that is about a guard the task names.
fn named_guard_point() -> String {
    let body = part("tester", "## What you produce");
    let point = body.split('\n').find(|line| line.starts_with("- Where the task names an existing test as the guard of a clause,"));
    point.unwrap_or_else(|| panic!("no point for a guard the task names: {}", body)).to_string()
}

#[test]
fn test_WB_ROLE_43_a_guard_the_task_names_is_run_as_it_is_and_listed_by_its_name() {
    let point = named_guard_point();
    assert!(point.contains("that test is the guard"), "{}", point);
    assert!(point.contains("Run it as it is and list it under `guards` by its name."), "{}", point);
    let guards = field_line("tester", "guards");
    assert!(guards.contains("An existing test the task names is listed by its name."), "{}", guards);
}

#[test]
fn test_WB_ROLE_44_the_tester_writes_no_copy_of_a_guard_the_task_names() {
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
