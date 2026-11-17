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

/// Values that `wb init` can be given: what a repository or a person may type. Each is written
/// into every field, and must be read back as it was.
const VALUES: [&str; 40] = [
    "none", "github", "yes", "no", "Yes", "NO", "y", "n", "on", "off", "true", "false", "null", "Null", "~",
    "1", "1.10", "1.2.3", "0.1.0-rc.1", "1e3", "0x1F", "0o17", "2026-11-17", "12:30", "1_000", ".inf", "-",
    "a'b", "it''s", "\"quoted\"", "a: b", "a #b", "# c", "- x", "{a}", "[a]", "*", "&a", "!tag", "Ünï côdé",
];

#[test]
fn test_WB_CHECK_10_every_value_init_can_write_is_read_back_unchanged() {
    let folder = folder();
    let file = folder.path.join("watchbill.yml");
    for value in VALUES {
        let setup = Setup {
            project: value.to_string(),
            git_server: value.to_string(),
            repository: value.to_string(),
            tracker: value.to_string(),
            requirements: value.to_string(),
            ci: value.to_string(),
        };
        let written = watchbill::init::render("1.2.3", &"ab".repeat(32), &setup);
        fs::write(&file, &written).unwrap();

        let document = match load(&file) {
            Ok(document) => document,
            Err(why) => panic!("{:?}: the file is refused: {}\n{}", value, why, written),
        };
        for key in ["project", "git_server", "repository", "tracker", "requirements", "ci"] {
            let read = document.text(&[Part::from(key)]);
            assert_eq!(read.as_deref(), Ok(value), "{:?} in {}\n{}", value, key, written);
        }
        assert_eq!(document.text(&[Part::from("watchbill")]).as_deref(), Ok("1.2.3"));
        assert_eq!(document.text(&[Part::from("revision")]).unwrap(), "ab".repeat(32));
    }
}

#[test]
fn test_WB_CHECK_10_the_file_that_init_wrote_is_read_back() {
    let folder = set_up();

    let document = load(config(&folder.path)).expect("the file of wb init is refused");

    assert_eq!(document.text(&[Part::from("watchbill")]).as_deref(), Ok(REVISION));
    assert_eq!(document.text(&[Part::from("revision")]).unwrap(), installed_hash());
    assert_eq!(document.text(&[Part::from("project")]).as_deref(), Ok("brake-controller"));
}
