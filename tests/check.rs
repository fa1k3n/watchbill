//! `wb check`: whether a repository still is at the revision it is pinned to, and whether the
//! installed `wb` is that revision. Run as a person runs it, in a repository made for the test.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::path::Path;
use watchbill::detect::Setup;
use watchbill::manifest;
use watchbill::yamlfile::{load, Part};

/// A repository that `wb init --yes` has set up.
fn set_up() -> Folder {
    let folder = repository(None);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    // Issue 1 has its file accepted and committed, as no role is handed out without (#97).
    issues_ready(&folder.path, &["1"]);
    folder
}

fn watchbill_dir(folder: &Folder) -> std::path::PathBuf {
    folder.path.join(".watchbill")
}

/// Every plain file under a folder, with its bytes, sorted by path.
fn snapshot(folder: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &Path, at: &Path, found: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().map(|name| name == ".git").unwrap_or(false) {
                continue;
            }
            if path.is_dir() {
                walk(base, &path, found);
            } else {
                found.push((path.strip_prefix(base).unwrap().to_str().unwrap().to_string(), fs::read(&path).unwrap()));
            }
        }
    }
    let mut found = Vec::new();
    walk(folder, folder, &mut found);
    found.sort();
    found
}

/// The line `key:` of `.watchbill/watchbill.yml`, written again with another value.
fn set_line(folder: &Folder, key: &str, value: &str) {
    let file = config(&folder.path);
    let changed: String = read(&file)
        .lines()
        .map(|line| {
            if line.starts_with(&format!("{}: ", key)) {
                format!("{}: {}\n", key, value)
            } else {
                format!("{}\n", line)
            }
        })
        .collect();
    fs::write(file, changed).unwrap();
}

/// The manifest written again for the files as they are now, and the line `revision:` made to match.
fn pin_again(folder: &Folder) {
    let manifest_text = manifest::manifest(watchbill_dir(folder)).expect("the revision could not be listed");
    fs::write(watchbill_dir(folder).join("revision.sha256"), &manifest_text).unwrap();
    set_line(folder, "revision", &manifest::hash_of(&manifest_text));
}

fn installed_hash() -> String {
    manifest::revision_hash(env!("CARGO_MANIFEST_DIR").to_string() + "/share/watchbill").unwrap()
}

#[test]
fn test_WB_CHECK_1_a_repository_fresh_from_init_passes() {
    let folder = set_up();

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_CHECK_2_one_changed_byte_is_named_as_changed() {
    let folder = set_up();
    let file = watchbill_dir(&folder).join("revision/roles/build.md");
    let mut bytes = fs::read(&file).unwrap();
    bytes.insert(0, b'x');
    fs::write(&file, bytes).unwrap();

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains("revision/roles/build.md"), "{}", out);
    assert!(out.contains("changed"), "{}", out);
    assert!(!out.contains("revision/rules.yml"), "only the file that differs is named: {}", out);
}

#[test]
fn test_WB_CHECK_2_every_file_that_differs_is_named() {
    let folder = set_up();
    for path in ["revision/roles/build.md", "revision/rules.yml"] {
        fs::write(watchbill_dir(&folder).join(path), "mine\n").unwrap();
    }

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1));
    let out = text(&run.stdout);
    assert!(out.contains("revision/roles/build.md") && out.contains("revision/rules.yml"), "{}", out);
}

#[test]
fn test_WB_CHECK_3_a_file_that_is_removed_is_named_as_missing() {
    let folder = set_up();
    fs::remove_file(watchbill_dir(&folder).join("revision/rules.yml")).unwrap();

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains("revision/rules.yml"), "{}", out);
    assert!(out.contains("missing"), "{}", out);
}

#[test]
fn test_WB_CHECK_4_a_file_that_is_added_is_named_as_not_of_the_revision() {
    let folder = set_up();
    put(&watchbill_dir(&folder), "revision/roles/extra.md", "mine\n");

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains("revision/roles/extra.md"), "{}", out);
    assert!(out.contains("not of the revision"), "{}", out);
}

#[test]
fn test_WB_CHECK_5_a_manifest_edited_to_match_a_changed_file_is_not_the_revision_line() {
    let folder = set_up();
    fs::write(watchbill_dir(&folder).join("revision/roles/build.md"), "mine\n").unwrap();
    let manifest_text = manifest::manifest(watchbill_dir(&folder)).unwrap();
    fs::write(watchbill_dir(&folder).join("revision.sha256"), &manifest_text).unwrap();

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains("revision.sha256"), "{}", out);
    assert!(out.contains(&manifest::hash_of(&manifest_text)), "the hash of the manifest is named: {}", out);
    assert!(out.contains("watchbill.yml"), "{}", out);
    assert!(!out.contains("changed"), "the files match the manifest: {}", out);
}

#[test]
fn test_WB_CHECK_6_a_repository_pinned_to_another_version_names_both() {
    let folder = set_up();
    set_line(&folder, "watchbill", "0.0.0");

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains("0.0.0"), "the version of the repository: {}", out);
    assert!(out.contains(REVISION), "the version of the installed wb: {}", out);
}

#[test]
fn test_WB_CHECK_6_a_repository_pinned_to_another_hash_names_both() {
    let folder = set_up();
    // The files, the manifest and the line agree, but they are not the revision of the installed wb.
    fs::write(watchbill_dir(&folder).join("revision/roles/build.md"), "mine\n").unwrap();
    pin_again(&folder);
    let pinned = manifest::revision_hash(watchbill_dir(&folder)).unwrap();

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains(&pinned), "the hash of the repository: {}", out);
    assert!(out.contains(&installed_hash()), "the hash of the installed wb: {}", out);
    assert!(!out.contains("changed"), "the files match the manifest: {}", out);
}

#[test]
fn test_WB_CHECK_7_a_folder_with_no_watchbill_is_not_set_up() {
    let folder = repository(None);

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(2), "{}", text(&run.stdout));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("not set up"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_CHECK_7_a_folder_with_a_watchbill_and_no_config_is_not_set_up() {
    let folder = repository(None);
    put(&folder.path, ".watchbill/other", "x\n");

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(2), "{}", text(&run.stdout));
    assert!(text(&run.stderr).contains("not set up"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_CHECK_7_outside_a_git_repository_ends_with_2() {
    let folder = folder();

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(2), "{}", text(&run.stdout));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("not in a git repository"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_CHECK_7_an_option_that_is_not_known_is_refused_with_2() {
    let folder = set_up();

    let run = wb(&folder.path, &["check", "--x"], "");

    assert_eq!(code(&run), Some(2));
    assert!(text(&run.stderr).starts_with("wb check: '--x' is not known\n"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_CHECK_8_the_result_is_the_same_from_a_folder_inside() {
    let folder = set_up();
    put(&folder.path, "src/deep/file.c", "int x;\n");
    let inside = folder.path.join("src/deep");
    fs::write(watchbill_dir(&folder).join("revision/roles/build.md"), "mine\n").unwrap();

    let from_root = wb(&folder.path, &["check"], "");
    let from_inside = wb(&inside, &["check"], "");
    let from_config = wb(&watchbill_dir(&folder), &["check"], "");

    assert_eq!(code(&from_root), Some(1));
    for run in [&from_inside, &from_config] {
        assert_eq!(code(run), code(&from_root));
        assert_eq!(text(&run.stdout), text(&from_root.stdout));
        assert_eq!(text(&run.stderr), text(&from_root.stderr));
    }
}

#[test]
fn test_WB_CHECK_8_a_repository_that_passes_passes_from_a_folder_inside() {
    let folder = set_up();
    put(&folder.path, "src/deep/file.c", "int x;\n");

    let run = wb(&folder.path.join("src/deep"), &["check"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
}

#[test]
fn test_WB_CHECK_9_a_run_writes_nothing_and_reads_no_input() {
    let folder = set_up();
    let before = snapshot(&folder.path);

    // Standard input is closed at once: nothing is asked.
    let passes = wb(&folder.path, &["check"], "");
    assert_eq!(code(&passes), Some(0), "{}", text(&passes.stderr));
    assert_eq!(snapshot(&folder.path), before);

    fs::write(watchbill_dir(&folder).join("revision/rules.yml"), "mine\n").unwrap();
    put(&watchbill_dir(&folder), "revision/extra.md", "mine\n");
    set_line(&folder, "watchbill", "0.0.0");
    let before = snapshot(&folder.path);
    let fails = wb(&folder.path, &["check"], "");
    assert_eq!(code(&fails), Some(1));
    assert_eq!(snapshot(&folder.path), before);
}

#[test]
fn test_WB_CHECK_9_the_status_of_git_is_the_same_after_a_run() {
    let folder = set_up();
    let status = |folder: &Folder| {
        std::process::Command::new("git")
            .arg("-C")
            .arg(&folder.path)
            .args(["status", "--porcelain"])
            .env_clear()
            .envs(env())
            .output()
            .unwrap()
            .stdout
    };
    let before = status(&folder);

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(status(&folder), before);
}

/// What the folder `.watchbill` or one file or folder of it is made, put outside the repository: a
/// copy of what is under `from`, so that a link to it reads as a revision that holds.
fn copy_outside(folder: &Folder, from: &str) -> std::path::PathBuf {
    let outside = folder.path.parent().unwrap().join("outside");
    fs::create_dir_all(&outside).unwrap();
    let to = outside.join(from.replace('/', "_"));
    let status = std::process::Command::new("cp")
        .arg("-r")
        .arg(watchbill_dir(folder).join(from))
        .arg(&to)
        .status()
        .unwrap();
    assert!(status.success());
    to
}

/// What is at `.watchbill/<path>` is taken away, and a link to a copy of it outside the repository is put there.
fn link_to_copy(folder: &Folder, path: &str) {
    let copy = copy_outside(folder, path);
    let at = watchbill_dir(folder).join(path);
    if at.is_dir() {
        fs::remove_dir_all(&at).unwrap();
    } else {
        fs::remove_file(&at).unwrap();
    }
    std::os::unix::fs::symlink(copy, at).unwrap();
}

#[test]
fn test_WB_CHECK_11_a_link_for_a_part_of_the_pin_is_named_and_never_followed() {
    for path in ["revision", "revision/roles", "revision.sha256", "watchbill.yml"] {
        let folder = set_up();
        link_to_copy(&folder, path);

        let run = wb(&folder.path, &["check"], "");

        assert_eq!(code(&run), Some(1), "{}: {}{}", path, text(&run.stdout), text(&run.stderr));
        let out = text(&run.stdout);
        let named = out.lines().filter(|line| line.contains(&format!(".watchbill/{}", path)) && line.contains("link"));
        assert_eq!(named.count(), 1, "{}: the link is named on one line: {}", path, out);
        assert!(!out.contains("missing:"), "{}: what is below a link is not looked at: {}", path, out);
    }
}

#[test]
fn test_WB_CHECK_11_a_link_for_the_watchbill_folder_is_named_and_never_followed() {
    let folder = set_up();
    let moved = folder.path.parent().unwrap().join("outside-watchbill");
    fs::rename(watchbill_dir(&folder), &moved).unwrap();
    std::os::unix::fs::symlink(&moved, watchbill_dir(&folder)).unwrap();

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.lines().any(|line| line.contains(".watchbill") && line.contains("link")), "{}", out);
}

#[test]
fn test_WB_CHECK_11_a_link_for_a_file_of_the_revision_is_named_as_changed() {
    let folder = set_up();
    link_to_copy(&folder, "revision/rules.yml");

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
    let out = text(&run.stdout);
    assert!(
        out.lines().any(|line| line.contains("changed") && line.contains("revision/rules.yml") && line.contains("link")),
        "{}",
        out
    );
}

#[test]
fn test_WB_CHECK_2_a_folder_where_a_file_of_the_revision_goes_is_named_as_changed() {
    let folder = set_up();
    let file = watchbill_dir(&folder).join("revision/rules.yml");
    fs::remove_file(&file).unwrap();
    fs::create_dir(&file).unwrap();

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.lines().any(|line| line.contains("changed") && line.contains("revision/rules.yml")), "{}", out);
    assert!(!out.contains("missing:"), "{}", out);
}

#[test]
fn test_WB_CHECK_4_a_folder_the_manifest_does_not_account_for_is_not_of_the_revision() {
    let folder = set_up();
    fs::create_dir(watchbill_dir(&folder).join("revision/empty")).unwrap();

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(
        out.lines().any(|line| line.contains("not of the revision") && line.contains("revision/empty")),
        "{}",
        out
    );
}

#[test]
fn test_WB_CHECK_3_a_manifest_that_is_not_there_is_named_as_missing() {
    let folder = set_up();
    fs::remove_file(watchbill_dir(&folder).join("revision.sha256")).unwrap();

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.lines().any(|line| line.contains("missing") && line.contains("revision.sha256")), "{}", out);
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_CHECK_12_a_watchbill_yml_the_loader_refuses_ends_with_1_and_is_named() {
    let folder = set_up();
    fs::write(config(&folder.path), "watchbill: 0.0.1\nwatchbill: 0.0.2\n").unwrap();

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains("watchbill.yml"), "{}", out);
    assert_eq!(text(&run.stderr), "");
}

/// Values that `wb init` can be given: what a repository or a person may type. Each is written
/// into every field of a file that `wb check` reads, and the file must be read, and read back as it was.
const VALUES: [&str; 40] = [
    "none", "github", "yes", "no", "Yes", "NO", "y", "n", "on", "off", "true", "false", "null", "Null", "~",
    "1", "1.10", "1.2.3", "0.1.0-rc.1", "1e3", "0x1F", "0o17", "2026-11-17", "12:30", "1_000", ".inf", "-",
    "a'b", "it''s", "\"quoted\"", "a: b", "a #b", "# c", "- x", "{a}", "[a]", "*", "&a", "!tag", "Ünï côdé",
];

#[test]
fn test_WB_CHECK_10_every_value_init_can_write_is_read_back_unchanged() {
    let folder = set_up();
    let hash = installed_hash();
    for value in VALUES {
        let setup = Setup {
            project: value.to_string(),
            git_server: value.to_string(),
            repository: value.to_string(),
            tracker: value.to_string(),
            requirements: value.to_string(),
            ci: value.to_string(),
        };
        fs::write(config(&folder.path), watchbill::init::render(REVISION, &hash, &setup)).unwrap();

        // `wb check` reads the file with the loader, and it holds when what it reads is the pin.
        let run = wb(&folder.path, &["check"], "");
        assert_eq!(code(&run), Some(0), "{:?}: {}{}", value, text(&run.stdout), text(&run.stderr));
        let document = load(config(&folder.path)).unwrap();
        for key in ["project", "git_server", "repository", "tracker", "requirements", "ci"] {
            assert_eq!(document.text(&[Part::from(key)]).as_deref(), Ok(value), "{:?} in {}", value, key);
        }
    }
}

#[test]
fn test_WB_CHECK_10_a_hash_that_starts_with_a_digit_is_written_in_quotes_and_read_back_by_check() {
    let folder = set_up();
    // A revision other than the installed one, whose hash starts with a digit, as the manifest has it.
    let mut pinned = String::new();
    for number in 0..200 {
        fs::write(watchbill_dir(&folder).join("revision/roles/build.md"), format!("mine {}\n", number)).unwrap();
        pinned = manifest::revision_hash(watchbill_dir(&folder)).unwrap();
        if pinned.starts_with(|c: char| c.is_ascii_digit()) {
            break;
        }
    }
    assert!(pinned.starts_with(|c: char| c.is_ascii_digit()), "no hash that starts with a digit was found");
    let manifest_text = manifest::manifest(watchbill_dir(&folder)).unwrap();
    fs::write(watchbill_dir(&folder).join("revision.sha256"), &manifest_text).unwrap();
    let setup = Setup {
        project: "brake-controller".to_string(),
        git_server: "none".to_string(),
        repository: "none".to_string(),
        tracker: "none".to_string(),
        requirements: "none".to_string(),
        ci: "none".to_string(),
    };
    let written = watchbill::init::render(REVISION, &pinned, &setup);
    assert!(written.contains(&format!("revision: '{}'\n", pinned)), "{}", written);
    fs::write(config(&folder.path), written).unwrap();

    let run = wb(&folder.path, &["check"], "");

    // The files, the manifest and the line agree; only the installed `wb` is another revision, and
    // that is the one difference, named with the hash as it was written.
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains(&pinned) && out.contains(&installed_hash()), "{}", out);
    assert!(!out.contains("changed") && !out.contains("watchbill.yml:"), "{}", out);
}

#[test]
fn test_WB_CHECK_13_project_text_with_a_known_placeholder_passes() {
    let folder = set_up();
    put(&folder.path, ".watchbill/project/all.md", "Work on {issue} in {branch}.\n");
    put(&folder.path, ".watchbill/project/build.md", "Here {checkout} is the root.\n");

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
}

#[test]
fn test_WB_CHECK_13_a_placeholder_the_revision_does_not_know_names_the_file_the_line_and_the_word() {
    let folder = set_up();
    put(&folder.path, ".watchbill/project/all.md", "First line.\nWork on {issue}.\nThen {task} too.\n");

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains(".watchbill/project/all.md"), "{}", out);
    assert!(out.contains("line 3"), "{}", out);
    assert!(out.contains("{task}"), "{}", out);
    assert!(out.contains("The known ones are {issue}"), "{}", out);
}

#[test]
fn test_WB_CHECK_13_the_file_of_a_role_is_read_and_the_readme_of_the_project_is_not() {
    let folder = set_up();
    put(&folder.path, ".watchbill/project/tester.md", "Use {task}.\n");
    put(&folder.path, ".watchbill/project/README.md", "Say {task} here.\n");

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    let out = text(&run.stdout);
    assert!(out.contains(".watchbill/project/tester.md"), "{}", out);
    assert!(!out.contains("project/README.md"), "{}", out);
}

#[test]
fn test_WB_CHECK_13_no_project_text_is_unchanged() {
    let folder = set_up();
    fs::remove_dir_all(watchbill_dir(&folder).join("project")).ok();

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
}

#[test]
fn test_WB_CHECK_13_a_run_with_such_text_still_writes_nothing() {
    let folder = set_up();
    put(&folder.path, ".watchbill/project/all.md", "Use {task}.\n");
    let before = snapshot(&folder.path);

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(snapshot(&folder.path), before);
}

#[test]
fn test_WB_CHECK_14_wb_prompt_refuses_the_text_that_wb_check_refuses() {
    let folder = set_up();
    put(&folder.path, ".watchbill/project/all.md", "Use {task}.\n");

    let check = wb(&folder.path, &["check"], "");
    let prompt = wb(&folder.path, &["prompt", "build", "--issue", "1", "--requirements", "R", "--checkout", "/x", "--branch", "b", "--issue_address", "at 1"], "");

    assert_eq!(code(&check), Some(1));
    assert_eq!(code(&prompt), Some(1), "{}", text(&prompt.stderr));
    assert_eq!(text(&prompt.stdout), "");
    let said = text(&check.stdout);
    let line = said.lines().find(|line| line.contains("{task}")).expect("check names the placeholder");
    assert!(text(&prompt.stderr).contains(line), "{}\n---\n{}", said, text(&prompt.stderr));
}

#[test]
fn test_WB_CHECK_14_the_same_text_with_a_known_placeholder_passes_both() {
    let folder = set_up();
    put_committed(&folder.path, ".watchbill/project/all.md", "Use {issue}.\n");

    let check = wb(&folder.path, &["check"], "");
    let prompt = wb(&folder.path, &["prompt", "build", "--issue", "1", "--requirements", "R", "--checkout", "/x", "--branch", "b", "--issue_address", "at 1"], "");

    assert_eq!(code(&check), Some(0));
    assert_eq!(code(&prompt), Some(0), "{}", text(&prompt.stderr));
}

#[test]
fn test_WB_CHECK_13_only_the_project_text_differing_does_not_say_the_revision_is_not_held() {
    let folder = set_up();
    put(&folder.path, ".watchbill/project/all.md", "Use {task}.\n");

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1));
    let out = text(&run.stdout);
    let last = out.lines().last().unwrap();
    assert!(last.contains("The project's text cannot be used with the revision it is pinned to."), "{}", out);
    assert!(!last.contains("not at the revision"), "{}", out);
}

#[test]
fn test_WB_CHECK_16_breakdown_none_names_watchbill_yml_once_and_the_last_line_says_it_does_not_fit() {
    let folder = set_up();
    let mut yml = read(&config(&folder.path));
    yml.push_str("breakdown: none\n");
    put(&folder.path, ".watchbill/watchbill.yml", &yml);

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    let out = text(&run.stdout);
    let line = out.lines().find(|line| line.contains("breakdown: none")).expect("the fault is named");
    assert_eq!(line.matches("watchbill.yml").count(), 1, "{}", out);
    let last = out.lines().last().unwrap();
    assert!(last.contains("watchbill.yml does not fit the revision"), "{}", out);
    assert!(!last.contains("not at the revision"), "{}", out);
}

#[test]
fn test_WB_CHECK_16_only_a_project_file_that_is_a_link_ends_with_the_line_for_the_project_s_text() {
    let folder = set_up();
    put(&folder.path, "elsewhere.md", "Plain text.\n");
    fs::create_dir_all(watchbill_dir(&folder).join("project")).unwrap();
    std::os::unix::fs::symlink(folder.path.join("elsewhere.md"), watchbill_dir(&folder).join("project/all.md")).unwrap();

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    let out = text(&run.stdout);
    let last = out.lines().last().unwrap();
    assert!(last.ends_with("The project's text cannot be used with the revision it is pinned to."), "{}", out);
    assert!(!last.contains("does not fit"), "{}", out);
}

#[test]
fn test_WB_CHECK_15_a_project_file_that_is_a_link_is_named_by_check_and_refused_by_prompt() {
    let folder = set_up();
    put(&folder.path, "elsewhere.md", "Plain text.\n");
    fs::create_dir_all(watchbill_dir(&folder).join("project")).unwrap();
    std::os::unix::fs::symlink(folder.path.join("elsewhere.md"), watchbill_dir(&folder).join("project/all.md")).unwrap();

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    assert!(text(&run.stdout).contains(".watchbill/project/all.md is a link"), "{}", text(&run.stdout));
    assert!(watchbill::prompt::assemble(&folder.path, "build", &[]).is_err());
}

#[test]
fn test_WB_CHECK_15_the_folder_project_as_a_link_is_named_by_check() {
    let folder = set_up();
    put(&folder.path, "elsewhere/all.md", "Plain text.\n");
    let project = watchbill_dir(&folder).join("project");
    fs::remove_dir_all(&project).ok();
    std::os::unix::fs::symlink(folder.path.join("elsewhere"), &project).unwrap();

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    assert!(text(&run.stdout).contains(".watchbill/project is a link"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_CHECK_15_a_folder_where_a_project_file_goes_is_named_by_check() {
    let folder = set_up();
    fs::create_dir_all(watchbill_dir(&folder).join("project/all.md")).unwrap();

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    assert!(text(&run.stdout).contains(".watchbill/project/all.md is a link or not a plain file"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_CHECK_15_a_project_file_that_is_not_utf8_is_named_by_check() {
    let folder = set_up();
    fs::write(watchbill_dir(&folder).join("project/all.md"), [0xff, 0xfe, b'x']).unwrap();

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    assert!(text(&run.stdout).contains(".watchbill/project/all.md is not UTF-8 text"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_CHECK_14_the_refusal_of_assemble_is_the_line_that_check_prints() {
    let folder = set_up();
    put_committed(&folder.path, ".watchbill/project/all.md", "One.\nUse {task}.\n");

    let refusal = watchbill::prompt::assemble(&folder.path, "build", &[]).unwrap_err().to_string();
    let run = wb(&folder.path, &["check"], "");

    assert_eq!(refusal, watchbill::prompt::unknown_in(".watchbill/project/all.md", "One.\nUse {task}.\n")[0]);
    assert!(text(&run.stdout).lines().any(|line| line == refusal), "{}\n---\n{}", refusal, text(&run.stdout));
}

#[test]
fn test_WB_CHECK_16_a_fault_the_loader_gives_names_watchbill_yml_once() {
    let folder = set_up();
    let mut yml = read(&config(&folder.path));
    yml.push_str("breakdown: other\n");
    put(&folder.path, ".watchbill/watchbill.yml", &yml);

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    let out = text(&run.stdout);
    let line = out.lines().find(|line| line.contains("breakdown")).expect("the fault is named");
    assert_eq!(line.matches("watchbill.yml").count(), 1, "{}", out);
    assert!(!out.lines().last().unwrap().contains("not at the revision"), "{}", out);
}

#[test]
fn test_WB_CHECK_16_settings_and_the_project_text_differing_say_so_and_not_that_the_revision_is_not_held() {
    let folder = set_up();
    let mut yml = read(&config(&folder.path));
    yml.push_str("breakdown: none\n");
    put(&folder.path, ".watchbill/watchbill.yml", &yml);
    put(&folder.path, ".watchbill/project/all.md", "Use {task}.\n");

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    let out = text(&run.stdout);
    let last = out.lines().last().unwrap();
    assert!(last.contains("watchbill.yml and the project's text do not fit the revision"), "{}", out);
    assert!(!last.contains("not at the revision"), "{}", out);
}

#[test]
fn test_WB_CHECK_16_no_line_revision_says_that_watchbill_yml_pins_no_revision() {
    let folder = set_up();
    let yml: String = read(&config(&folder.path)).lines().filter(|line| !line.starts_with("revision:")).map(|line| format!("{}\n", line)).collect();
    put(&folder.path, ".watchbill/watchbill.yml", &yml);

    let run = wb(&folder.path, &["check"], "");

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    let out = text(&run.stdout);
    assert!(out.lines().any(|line| line.contains("pins no revision")), "{}", out);
    let last = out.lines().last().unwrap();
    assert!(!last.contains("not at the revision"), "{}", out);
    assert!(!last.contains("does not fit"), "{}", out);
}
