//! What `wb init` writes of the revision: its files, its manifest and its hash, in a repository made for the test.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn package() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("share/watchbill")
}

fn real(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap()
}

/// Every plain file under a folder, as a path relative to it, sorted.
fn files_under(base: &Path, folder: &Path, found: &mut Vec<String>) {
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files_under(base, &path, found);
        } else {
            found.push(path.strip_prefix(base).unwrap().to_str().unwrap().to_string());
        }
    }
    found.sort();
}

fn all_files(folder: &Path) -> Vec<String> {
    let mut found = Vec::new();
    if folder.exists() {
        files_under(folder, folder, &mut found);
    }
    found
}

fn set_up() -> (Folder, std::process::Output) {
    let folder = repository(None);
    let run = wb(&folder.path, &["init", "--yes"], "");
    (folder, run)
}

#[test]
fn test_WB_PIN_1_the_files_of_the_revision_are_written_byte_for_byte() {
    let (folder, run) = set_up();

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let theirs = all_files(&package().join("revision"));
    assert!(!theirs.is_empty());
    assert_eq!(all_files(&folder.path.join(".watchbill/revision")), theirs);
    for path in theirs {
        let mine = fs::read(folder.path.join(".watchbill/revision").join(&path)).unwrap();
        assert_eq!(mine, fs::read(package().join("revision").join(&path)).unwrap(), "{}", path);
    }
}

#[test]
fn test_WB_PIN_2_the_manifest_is_the_one_of_the_package() {
    let (folder, run) = set_up();

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(
        fs::read(folder.path.join(".watchbill/revision.sha256")).unwrap(),
        fs::read(package().join("revision.sha256")).unwrap()
    );
}

#[test]
fn test_WB_PIN_3_the_hash_of_the_revision_is_the_line_revision_of_the_file() {
    let (folder, run) = set_up();

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let printed = text(&wb(&folder.path, &["revision"], "").stdout);
    let hash = printed.lines().find_map(|line| line.strip_prefix("revision ")).unwrap().to_string();
    assert_eq!(hash.len(), 64);
    assert!(read(&config(&folder.path)).contains(&format!("\nrevision: {}\n", watchbill::init::yaml_value(&hash))));
}

#[test]
fn test_WB_PIN_4_a_file_that_is_there_stops_everything_and_is_named() {
    for target in ["revision/rules.yml", "revision/roles/build.md", "revision.sha256", "watchbill.yml"] {
        let folder = repository(None);
        put(&folder.path, &format!(".watchbill/{}", target), "mine\n");

        let run = wb(&folder.path, &["init", "--yes"], "");

        assert_eq!(code(&run), Some(1), "{}", target);
        let file = real(&folder.path).join(".watchbill").join(target);
        assert!(text(&run.stdout).contains(file.to_str().unwrap()), "{}: {}", target, text(&run.stdout));
        assert_eq!(all_files(&folder.path.join(".watchbill")), vec![target.to_string()], "{}", target);
        assert_eq!(read(&folder.path.join(".watchbill").join(target)), "mine\n");
    }
}

#[test]
fn test_WB_PIN_4_a_file_that_is_there_stops_it_before_it_asks() {
    let folder = repository(None);
    put(&folder.path, ".watchbill/revision.sha256", "mine\n");

    let run = wb(&folder.path, &["init"], "");

    assert_eq!(code(&run), Some(1));
    assert!(!text(&run.stdout).contains("project:"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_PIN_4_a_no_at_the_last_question_writes_none_of_the_files() {
    let folder = repository(None);

    let run = wb(&folder.path, &["init"], &lines(&["", "", "", "", "", "no"]));

    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stdout).contains("revision.sha256"), "the files were shown");
    assert!(text(&run.stdout).contains("Nothing written."));
    assert!(!folder.path.join(".watchbill").exists());
}

#[test]
fn test_WB_PIN_5_every_file_is_named_before_it_is_asked_whether_it_is_ok() {
    let folder = repository(None);

    let run = wb(&folder.path, &["init"], &lines(&["", "", "", "", "", "no"]));

    let out = text(&run.stdout);
    let ok = out.find("Is this OK?").unwrap();
    let root = real(&folder.path).join(".watchbill");
    let mut named = vec![root.join("watchbill.yml"), root.join("revision.sha256")];
    for path in all_files(&package().join("revision")) {
        named.push(root.join("revision").join(path));
    }
    for file in named {
        let at = out.find(file.to_str().unwrap()).unwrap_or_else(|| panic!("{} is not named", file.display()));
        assert!(at < ok, "{} is named after the question", file.display());
    }
}

#[test]
fn test_WB_PIN_5_with_yes_every_file_is_named_before_it_is_written() {
    let folder = repository(None);

    let run = wb(&folder.path, &["init", "--yes"], "");

    let out = text(&run.stdout);
    let wrote = out.find("Wrote ").unwrap();
    let file = real(&folder.path).join(".watchbill/revision/roles/build.md");
    assert!(out[..wrote].contains(file.to_str().unwrap()), "{}", out);
}

#[test]
fn test_WB_PIN_6_sha256sum_passes_on_the_written_manifest() {
    let (folder, run) = set_up();

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let check = Command::new("sha256sum")
        .args(["-c", "revision.sha256"])
        .current_dir(folder.path.join(".watchbill"))
        .output()
        .expect("sha256sum could not be started");
    assert!(check.status.success(), "{}{}", text(&check.stdout), text(&check.stderr));
}

/// Runs `wb init --yes` with a link, a file or a folder put where the revision goes, and checks that
/// it ends with 1 before it asks, names the path, and writes nothing but what was put there.
fn refused_at(prepare: impl Fn(&Path, &Path), way: &str) -> Folder {
    let folder = repository(None);
    let outside = folder_named("outside");
    prepare(&folder.path, &outside.path);

    let run = wb(&folder.path, &["init", "--yes"], "");

    assert_eq!(code(&run), Some(1), "{}: {}", way, text(&run.stdout));
    let named = real(&folder.path).join(".watchbill").join(way);
    assert!(text(&run.stdout).contains(named.to_str().unwrap()), "{}: {}", way, text(&run.stdout));
    assert!(text(&run.stdout).contains("Nothing written."), "{}", text(&run.stdout));
    assert!(all_files(&outside.path).is_empty(), "{}: something was written outside", way);
    assert!(!folder.path.join(".watchbill/watchbill.yml").exists(), "{}", way);
    // Keep the folder of `outside` alive as long as the repository: it is a part of it.
    std::mem::forget(outside);
    folder
}

fn link(to: &Path, at: &Path) {
    fs::create_dir_all(at.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(to, at).unwrap();
}

#[test]
fn test_WB_PIN_7_a_link_to_a_folder_under_revision_is_refused_before_it_asks() {
    refused_at(|repo, outside| link(outside, &repo.join(".watchbill/revision/roles")), "revision/roles");
}

#[test]
fn test_WB_PIN_7_a_link_for_the_folder_watchbill_is_refused_before_it_asks() {
    let folder = repository(None);
    let outside = folder_named("outside");
    link(&outside.path, &folder.path.join(".watchbill"));

    let run = wb(&folder.path, &["init"], "");

    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stdout).contains(real(&folder.path).join(".watchbill").to_str().unwrap()));
    assert!(!text(&run.stdout).contains("project:"), "it asked");
    assert!(all_files(&outside.path).is_empty());
}

#[test]
fn test_WB_PIN_7_a_file_where_a_folder_goes_is_refused_before_it_asks() {
    for way in [".watchbill", ".watchbill/revision/roles"] {
        let folder = repository(None);
        put(&folder.path, way, "mine\n");

        let run = wb(&folder.path, &["init"], "");

        assert_eq!(code(&run), Some(1), "{}", way);
        assert!(text(&run.stdout).contains(real(&folder.path).join(way).to_str().unwrap()), "{}", way);
        assert!(!text(&run.stdout).contains("project:"), "{}: it asked", way);
        assert_eq!(read(&folder.path.join(way)), "mine\n");
    }
}

#[test]
fn test_WB_PIN_8_a_revision_that_is_there_is_refused_as_a_file_a_link_a_folder_and_an_empty_folder() {
    refused_at(|repo, _| put(repo, ".watchbill/revision", "mine\n"), "revision");
    refused_at(|repo, outside| link(outside, &repo.join(".watchbill/revision")), "revision");
    let stale = refused_at(|repo, _| put(repo, ".watchbill/revision/stale.md", "old\n"), "revision");
    assert_eq!(all_files(&stale.path.join(".watchbill")), vec!["revision/stale.md".to_string()]);
    refused_at(|repo, _| fs::create_dir_all(repo.join(".watchbill/revision")).unwrap(), "revision");
}

#[test]
fn test_WB_PIN_9_a_write_that_fails_part_way_removes_what_it_made_and_keeps_what_was_there() {
    let folder = repository(None);
    put(&folder.path, ".watchbill/project/notes.md", "mine\n");
    // A limit on the size of a file, which holds as root too: no file of the revision of 512 bytes
    // or more can be written, and an ignored signal makes the write fail and not end the program.
    let run = Command::new("sh")
        .args(["-c", "ulimit -f 1; trap '' XFSZ; exec \"$0\" init --yes", env!("CARGO_BIN_EXE_wb")])
        .current_dir(&folder.path)
        .env_clear()
        .envs(env())
        .output()
        .unwrap();

    assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
    assert!(text(&run.stderr).contains("cannot write"), "{}", text(&run.stderr));
    assert_eq!(all_files(&folder.path.join(".watchbill")), vec!["project/notes.md".to_string()]);
    assert!(!folder.path.join(".watchbill/revision").exists());
}
