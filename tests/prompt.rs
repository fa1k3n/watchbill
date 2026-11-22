//! How a prompt is put together from the revision and the project's own text, in a repository
//! made for the test. `wb prompt`, the command, is #10; here the library is run.
//! The test "`wb check` passes with project files there" (WB-PROMPT-8) needs `wb check` of #8
//! and moves to #10.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use watchbill::prompt::{assemble, FIXED_LINE};

const VALUES: [(&str, &str); 10] = [
    ("issue", "T-1 brake light"),
    ("change", "42"),
    ("requirements", "WB-X-1, WB-X-2"),
    ("round", "1"),
    ("checkout", "/work/brakes"),
    ("branch", "issue-1"),
    ("issue_address", "tracker T-1"),
    ("change_address", "server 42"),
    ("classed", "src/safety"),
    ("classed_touched", "none"),
];

fn package() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("share/watchbill")
}

/// A repository set up by `wb init --yes`.
fn set_up() -> Folder {
    let folder = repository(None);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    folder
}

/// The text of a role of the package, filled in with `VALUES`.
fn filled(role: &str) -> String {
    let mut text = fs::read_to_string(package().join("revision/roles").join(format!("{}.md", role))).unwrap();
    for (name, value) in VALUES {
        text = text.replace(&format!("{{{}}}", name), value);
    }
    text
}

fn sha256(path: &Path) -> String {
    let run = Command::new("sha256sum").arg(path).output().expect("sha256sum could not be started");
    text(&run.stdout).split_whitespace().next().unwrap().to_string()
}

fn refused(root: &Path, role: &str) -> String {
    match assemble(root, role, &VALUES) {
        Ok(prompt) => panic!("it was not refused: {}", prompt.text),
        Err(why) => why.to_string(),
    }
}

#[test]
fn test_WB_PROMPT_1_the_parts_come_in_order_with_the_fixed_line_after_the_role() {
    let folder = set_up();
    put(&folder.path, ".watchbill/project/all.md", "Everyone: use make.\n");
    put(&folder.path, ".watchbill/project/build.md", "Builders: run make test.\n");

    let prompt = assemble(&folder.path, "build", &VALUES).unwrap();

    let expected =
        format!("{}\n{}\n\nEveryone: use make.\n\nBuilders: run make test.\n", filled("build"), FIXED_LINE);
    assert_eq!(prompt.text, expected);
    assert!(FIXED_LINE.contains("never") && !FIXED_LINE.contains('\n'), "{}", FIXED_LINE);
}

#[test]
fn test_WB_PROMPT_1_each_project_file_is_used_alone_where_the_other_is_not_there() {
    let folder = set_up();
    put(&folder.path, ".watchbill/project/all.md", "Everyone: use make.\n");
    let only_all = assemble(&folder.path, "build", &VALUES).unwrap().text;
    assert_eq!(only_all, format!("{}\n{}\n\nEveryone: use make.\n", filled("build"), FIXED_LINE));

    fs::remove_file(folder.path.join(".watchbill/project/all.md")).unwrap();
    put(&folder.path, ".watchbill/project/review.md", "Reviewers: be brief.\n");
    // The file of another role is not part of this prompt.
    assert_eq!(assemble(&folder.path, "build", &VALUES).unwrap().text, filled("build"));
    let only_role = assemble(&folder.path, "review", &VALUES).unwrap().text;
    assert_eq!(only_role, format!("{}\n{}\n\nReviewers: be brief.\n", filled("review"), FIXED_LINE));
}

#[test]
fn test_WB_PROMPT_2_with_no_project_text_the_prompt_is_the_role_filled_and_nothing_more() {
    let folder = set_up();

    for role in ["tester", "build", "review", "fix"] {
        let prompt = assemble(&folder.path, role, &VALUES).unwrap();
        assert_eq!(prompt.text, filled(role), "{}", role);
        assert!(!prompt.text.contains(FIXED_LINE), "{}", role);
        assert_eq!(prompt.parts.len(), 1, "{}", role);
    }
}

#[test]
fn test_WB_PROMPT_14_a_placeholder_that_got_no_value_is_refused_by_name_in_the_library() {
    let folder = set_up();

    let why = match assemble(&folder.path, "build", &VALUES[1..]) {
        Ok(_) => panic!("it was not refused"),
        Err(why) => why.to_string(),
    };

    assert!(why.contains("{issue}"), "{}", why);
}

#[test]
fn test_WB_PROMPT_3_a_placeholder_not_on_the_list_is_refused_in_any_part_and_the_file_is_named() {
    for (file, in_role) in [
        (".watchbill/project/all.md", false),
        (".watchbill/project/build.md", false),
        (".watchbill/revision/roles/build.md", true),
    ] {
        let folder = set_up();
        if in_role {
            let path = folder.path.join(file);
            let mut text = read(&path);
            text.push_str("\nSee {colour}.\n");
            fs::write(&path, text).unwrap();
            // The manifest says the same of the file, so that only the placeholder is at fault.
            let manifest = folder.path.join(".watchbill/revision.sha256");
            let old = sha256(&folder.path.join(".watchbill/revision/roles/build.md"));
            let lines: Vec<String> = read(&manifest)
                .lines()
                .map(|line| match line.strip_suffix("  revision/roles/build.md") {
                    Some(_) => format!("{}  revision/roles/build.md", old),
                    None => line.to_string(),
                })
                .collect();
            fs::write(&manifest, lines.join("\n") + "\n").unwrap();
        } else {
            put(&folder.path, file, "Work on {colour}.\n");
        }

        let why = refused(&folder.path, "build");

        assert!(why.contains(file), "{}: {}", file, why);
        assert!(why.contains("{colour}"), "{}: {}", file, why);
    }
}

#[test]
fn test_WB_PROMPT_3_text_that_is_no_placeholder_is_left_as_it_is() {
    let folder = set_up();
    put(&folder.path, ".watchbill/project/all.md", "Settings: {\"a\": 1}, {}, { issue }, {1}.\n");

    let prompt = assemble(&folder.path, "build", &VALUES).unwrap();

    assert!(prompt.text.ends_with("Settings: {\"a\": 1}, {}, { issue }, {1}.\n"), "{}", prompt.text);
}

#[test]
fn test_WB_PROMPT_4_a_value_that_is_filled_in_is_not_read_for_placeholders() {
    let folder = set_up();
    let mut values =
        vec![("issue", "fix {change} and {nothing}"), ("change", "42"), ("requirements", "R-{issue}"), ("round", "2")];
    values.extend(VALUES[4..].iter().copied());

    let prompt = assemble(&folder.path, "build", &values).unwrap();

    assert!(prompt.text.contains("builder of issue fix {change} and {nothing}."), "{}", prompt.text);
    assert!(prompt.text.contains("R-{issue}"));
    assert!(!prompt.text.contains("fix 42"));
}

#[test]
fn test_WB_PROMPT_5_a_project_file_that_is_not_utf8_is_refused_by_name() {
    for file in ["all.md", "build.md"] {
        let folder = set_up();
        fs::create_dir_all(folder.path.join(".watchbill/project")).unwrap();
        fs::write(folder.path.join(".watchbill/project").join(file), b"caf\xe9\n").unwrap();

        let why = refused(&folder.path, "build");

        assert!(why.contains(&format!(".watchbill/project/{}", file)), "{}: {}", file, why);
        assert!(why.contains("UTF-8"), "{}", why);
    }
}

#[test]
fn test_WB_PROMPT_6_a_role_the_revision_does_not_have_is_refused_with_the_roles_it_has() {
    let folder = set_up();

    for role in ["pilot", "", "../rules", "placeholders", "build.md"] {
        let why = refused(&folder.path, role);

        for name in ["build", "fix", "review", "tester"] {
            assert!(why.contains(name), "{:?}: {}", role, why);
        }
        assert!(!why.contains("placeholders") || role == "placeholders", "{:?}: {}", role, why);
    }
}

#[test]
fn test_WB_PROMPT_7_the_parts_are_given_in_order_with_path_and_sha256() {
    let folder = set_up();
    put(&folder.path, ".watchbill/project/all.md", "Everyone: use make.\n");
    put(&folder.path, ".watchbill/project/build.md", "Builders: run make test.\n");

    let prompt = assemble(&folder.path, "build", &VALUES).unwrap();

    let paths: Vec<&str> = prompt.parts.iter().map(|part| part.path.as_str()).collect();
    assert_eq!(
        paths,
        vec![".watchbill/revision/roles/build.md", ".watchbill/project/all.md", ".watchbill/project/build.md"]
    );
    for part in &prompt.parts {
        assert_eq!(part.hash, sha256(&folder.path.join(&part.path)), "{}", part.path);
    }
}

#[test]
fn test_WB_PROMPT_7_the_hash_of_the_role_part_is_the_one_on_its_line_of_the_manifest() {
    let folder = set_up();
    let manifest = read(&folder.path.join(".watchbill/revision.sha256"));

    for role in ["tester", "build", "review", "fix"] {
        let prompt = assemble(&folder.path, role, &VALUES).unwrap();

        let line = format!("{}  revision/roles/{}.md", prompt.parts[0].hash, role);
        assert!(manifest.lines().any(|each| each == line), "{}: {}", role, line);
    }
}

#[test]
fn test_WB_PROMPT_9_wb_init_leaves_the_readme_of_the_project_files() {
    let folder = set_up();

    let readme = read(&folder.path.join(".watchbill/project/README.md"));

    assert!(readme.contains("all.md") && readme.contains("<role>.md"), "{}", readme);
    assert!(readme.contains("never") && readme.contains("limit"), "{}", readme);
}

#[test]
fn test_WB_PROMPT_9_the_readme_is_named_before_the_question_and_before_it_is_written() {
    let folder = repository(None);

    let run = wb(&folder.path, &["init"], &lines(&["", "", "", "", "", "", "no"]));

    let out = text(&run.stdout);
    let file = fs::canonicalize(&folder.path).unwrap().join(".watchbill/project/README.md");
    let at = out.find(file.to_str().unwrap()).unwrap_or_else(|| panic!("not named: {}", out));
    assert!(at < out.find("Is this OK?").unwrap());
    assert!(!folder.path.join(".watchbill").exists());
}

#[test]
fn test_WB_PROMPT_9_a_readme_that_is_there_stops_everything_before_the_questions() {
    let folder = repository(None);
    put(&folder.path, ".watchbill/project/README.md", "mine\n");

    let run = wb(&folder.path, &["init"], "");

    assert_eq!(code(&run), Some(1));
    let file = fs::canonicalize(&folder.path).unwrap().join(".watchbill/project/README.md");
    assert!(text(&run.stdout).contains(file.to_str().unwrap()), "{}", text(&run.stdout));
    assert!(!text(&run.stdout).contains("project:"), "it asked");
    assert!(!folder.path.join(".watchbill/revision").exists());
    assert_eq!(read(&folder.path.join(".watchbill/project/README.md")), "mine\n");
}

#[test]
fn test_WB_PROMPT_9_a_link_for_the_folder_project_is_refused_before_the_questions() {
    let folder = repository(None);
    let outside = folder_named("outside");
    fs::create_dir_all(folder.path.join(".watchbill")).unwrap();
    std::os::unix::fs::symlink(&outside.path, folder.path.join(".watchbill/project")).unwrap();

    let run = wb(&folder.path, &["init"], "");

    assert_eq!(code(&run), Some(1));
    assert!(!text(&run.stdout).contains("project:"), "it asked");
    assert_eq!(fs::read_dir(&outside.path).unwrap().count(), 0);
}

fn outside_file() -> Folder {
    let outside = folder_named("outside");
    put(&outside.path, "secret.md", "SECRET TEXT\n");
    outside
}

fn link(to: &Path, at: &Path) {
    fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(to, at).unwrap();
}

#[test]
fn test_WB_PROMPT_1_a_project_file_that_is_a_link_is_refused_by_name_and_not_read() {
    for file in ["all.md", "build.md"] {
        let folder = set_up();
        let outside = outside_file();
        link(&outside.path.join("secret.md"), &folder.path.join(".watchbill/project").join(file));

        let why = refused(&folder.path, "build");

        assert!(why.contains(&format!(".watchbill/project/{}", file)), "{}: {}", file, why);
        assert!(!why.contains("SECRET"), "{}", why);
    }
}

#[test]
fn test_WB_PROMPT_1_a_project_folder_that_is_a_link_is_refused_by_name_and_not_read() {
    let folder = set_up();
    let outside = outside_file();
    put(&outside.path, "all.md", "SECRET TEXT\n");
    fs::remove_dir_all(folder.path.join(".watchbill/project")).unwrap();
    link(&outside.path, &folder.path.join(".watchbill/project"));

    let why = refused(&folder.path, "build");

    assert!(why.contains(".watchbill/project"), "{}", why);
    assert!(!why.contains("SECRET"), "{}", why);
}

#[test]
fn test_WB_PROMPT_1_a_project_file_that_is_not_a_plain_file_is_refused_and_does_not_hang() {
    let folder = set_up();
    let fifo = folder.path.join(".watchbill/project/all.md");
    let made = Command::new("mkfifo").arg(&fifo).status().expect("mkfifo could not be started");
    assert!(made.success());

    let why = refused(&folder.path, "build");
    assert!(why.contains(".watchbill/project/all.md"), "{}", why);

    fs::remove_file(&fifo).unwrap();
    fs::create_dir(&fifo).unwrap();
    let why = refused(&folder.path, "build");
    assert!(why.contains(".watchbill/project/all.md"), "{}", why);
}

#[test]
fn test_WB_PROMPT_1_the_file_of_the_role_that_is_a_link_is_refused_by_name() {
    let folder = set_up();
    let outside = outside_file();
    let file = folder.path.join(".watchbill/revision/roles/build.md");
    fs::remove_file(&file).unwrap();
    link(&outside.path.join("secret.md"), &file);

    let why = refused(&folder.path, "build");

    assert!(why.contains(".watchbill/revision/roles/build.md"), "{}", why);
    assert!(!why.contains("SECRET"), "{}", why);
}

#[test]
fn test_WB_PROMPT_6_the_roles_are_those_of_the_manifest_and_not_every_file_of_the_folder() {
    let folder = set_up();
    put(&folder.path, ".watchbill/revision/roles/pilot.md", "You are a pilot.\n");

    let why = refused(&folder.path, "pilot");

    for name in ["build", "fix", "review", "tester"] {
        assert!(why.contains(name), "{}", why);
    }
    assert!(!why.contains("pilot.md"), "{}", why);
    assert!(why.contains("no role 'pilot'"), "{}", why);
}

#[test]
fn test_WB_PROMPT_7_a_role_file_that_is_not_the_one_of_the_manifest_is_refused_and_named() {
    let folder = set_up();
    let file = folder.path.join(".watchbill/revision/roles/build.md");
    let mut changed = read(&file);
    changed.push_str("One more rule.\n");
    fs::write(&file, changed).unwrap();

    let why = refused(&folder.path, "build");

    assert!(why.contains(".watchbill/revision/roles/build.md"), "{}", why);
    assert!(why.contains("manifest"), "{}", why);
    // The others are as the manifest says, and are used.
    assert!(assemble(&folder.path, "fix", &VALUES).is_ok());
}

#[test]
fn test_WB_PROMPT_3_a_placeholder_is_a_lower_case_word_in_braces_with_no_dollar_before_it() {
    let folder = set_up();
    let text = "Shell: ${BUILD_DIR} ${issue} {Issue} {issue-id} { issue } {round2} {} {a b} $ {x}\n";
    // Of these only `{x}` is a placeholder, and it is not on the list.
    put(&folder.path, ".watchbill/project/all.md", &text.replace(" {x}", ""));

    let prompt = assemble(&folder.path, "build", &VALUES).unwrap();

    assert!(prompt.text.ends_with(&text.replace(" {x}", "")), "{}", prompt.text);

    put(&folder.path, ".watchbill/project/all.md", text);
    let why = refused(&folder.path, "build");
    assert!(why.contains("{x}") && why.contains(".watchbill/project/all.md"), "{}", why);
    for word in ["a_b", "issue_id", "b"] {
        put(&folder.path, ".watchbill/project/all.md", &format!("{{{}}}\n", word));
        assert!(refused(&folder.path, "build").contains(&format!("{{{}}}", word)), "{}", word);
    }
}

#[test]
fn test_WB_PROMPT_3_both_readmes_say_the_rule() {
    let rule = "a lower-case word in braces";
    let folder = set_up();
    assert!(read(&folder.path.join(".watchbill/project/README.md")).contains(rule));
    assert!(read(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("README.md")).contains(rule));
}

#[test]
fn test_WB_PROMPT_9_a_link_where_project_goes_is_named_as_a_link() {
    let folder = repository(None);
    let outside = folder_named("outside");
    fs::create_dir_all(folder.path.join(".watchbill")).unwrap();
    std::os::unix::fs::symlink(&outside.path, folder.path.join(".watchbill/project")).unwrap();

    let run = wb(&folder.path, &["init", "--yes"], "");

    assert_eq!(code(&run), Some(1));
    let named = fs::canonicalize(&folder.path).unwrap().join(".watchbill/project");
    let out = text(&run.stdout);
    assert!(out.contains(&format!("{} is a link", named.to_str().unwrap())), "{}", out);
    assert_eq!(fs::read_dir(&outside.path).unwrap().count(), 0);
}

#[test]
fn test_WB_PROMPT_19_utc_writes_fixed_times_in_the_form_of_the_stamp() {
    for (seconds, expected) in [
        (0, "1970-01-01T00:00:00Z"),
        (951_782_400, "2000-02-29T00:00:00Z"),
        (951_868_800, "2000-03-01T00:00:00Z"),
        (1_709_164_800, "2024-02-29T00:00:00Z"),
        (1_709_251_199, "2024-02-29T23:59:59Z"),
        (1_735_689_599, "2024-12-31T23:59:59Z"),
        (1_735_689_600, "2025-01-01T00:00:00Z"),
        (1_763_390_645, "2025-11-17T14:44:05Z"),
        (4_107_542_399, "2100-02-28T23:59:59Z"),
        (4_107_542_400, "2100-03-01T00:00:00Z"),
        (253_402_300_799, "9999-12-31T23:59:59Z"),
    ] {
        assert_eq!(watchbill::prompt::utc(seconds), expected, "{}", seconds);
    }
}
