//! `wb compare` (#179): what changed between two revisions, in the files, as fixed output. The installed
//! revision is the one of the checkout; the other is read from a repository pinned to it, made in a folder
//! of the test. `wb` is run as a person runs it.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Output;
use watchbill::manifest;

const PINNED: &str = "0.9.0";

fn share() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("share/watchbill")
}

fn installed_hash() -> String {
    manifest::revision_hash(share()).unwrap()
}

/// Copies the files of a folder, as they are.
fn copy_all(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_all(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A repository pinned to a copy of the installed revision, with the lines `watchbill:` and `revision:`
/// that its files give. `edit` changes the files first.
fn pinned_with(edit: impl FnOnce(&Path)) -> Folder {
    let repo = folder_named("pinned");
    let watch = repo.path.join(".watchbill");
    copy_all(&share().join("revision"), &watch.join("revision"));
    edit(&watch);
    pin(&repo.path, PINNED);
    repo
}

/// Writes the manifest and `watchbill.yml` of the files that are there, for the version given.
fn pin(repo: &Path, version: &str) {
    let watch = repo.join(".watchbill");
    let text = manifest::manifest(&watch).unwrap();
    fs::write(watch.join("revision.sha256"), &text).unwrap();
    fs::write(watch.join("watchbill.yml"), format!("watchbill: {}\nrevision: {}\n", version, manifest::hash_of(&text))).unwrap();
}

fn pinned_hash(repo: &Path) -> String {
    manifest::revision_hash(repo.join(".watchbill")).unwrap()
}

fn compare(args: &[&str]) -> Output {
    let here = folder();
    wb(&here.path, &[&["compare"], args].concat(), "")
}

fn from(repo: &Path, a: &str, b: &str) -> Output {
    compare(&["--from", repo.to_str().unwrap(), a, b])
}

fn lines_of(path: &Path) -> Vec<String> {
    read(path).lines().map(String::from).collect()
}

#[test]
fn test_WB_REV_27_the_same_revision_named_by_version_and_by_hash_prints_nothing_and_ends_with_0() {
    let hash = installed_hash();
    for (a, b) in [(REVISION, REVISION), (hash.as_str(), hash.as_str()), (REVISION, hash.as_str()), (hash.as_str(), REVISION)] {
        let run = compare(&[a, b]);
        assert_eq!(code(&run), Some(0), "{} {}: {}", a, b, text(&run.stderr));
        assert_eq!(text(&run.stdout), "", "{} {}", a, b);
        assert_eq!(text(&run.stderr), "", "{} {}", a, b);
    }
}

#[test]
fn test_WB_REV_27_a_repository_pinned_to_the_installed_revision_is_the_same() {
    let repo = pinned_with(|_| {});
    let hash = installed_hash();
    for (a, b) in [(REVISION, PINNED), (PINNED, hash.as_str()), (hash.as_str(), PINNED)] {
        let run = from(&repo.path, a, b);
        assert_eq!(code(&run), Some(0), "{} {}: {}", a, b, text(&run.stderr));
        assert_eq!(text(&run.stdout), "", "{} {}", a, b);
    }
}

/// A pinned repository that changes `rules.yml` (its first line), `roles/build.md` (a line added at the end),
/// adds `roles/extra.md` and removes `models.yml`.
fn changed_repo() -> Folder {
    pinned_with(|watch| {
        let revision = watch.join("revision");
        let rules = read(&revision.join("rules.yml"));
        fs::write(revision.join("rules.yml"), format!("# changed\n{}", rules.split_once('\n').unwrap().1)).unwrap();
        let build = read(&revision.join("roles/build.md"));
        fs::write(revision.join("roles/build.md"), format!("{}A new last line.\n", build)).unwrap();
        fs::write(revision.join("roles/extra.md"), "Extra.\n").unwrap();
        fs::remove_file(revision.join("models.yml")).unwrap();
    })
}

fn expected_diff(repo: &Folder) -> String {
    let rules = lines_of(&share().join("revision/rules.yml"));
    let build = lines_of(&share().join("revision/roles/build.md"));
    let n = build.len();
    let mut out = String::from("removed: revision/models.yml\nchanged: revision/roles/build.md\nadded: revision/roles/extra.md\nchanged: revision/rules.yml\n\n");
    out.push_str("--- a/revision/roles/build.md\n+++ b/revision/roles/build.md\n");
    out.push_str(&format!("@@ -{},3 +{},4 @@\n", n - 2, n - 2));
    for line in &build[n - 3..] {
        out.push_str(&format!(" {}\n", line));
    }
    out.push_str("+A new last line.\n");
    out.push_str("--- a/revision/rules.yml\n+++ b/revision/rules.yml\n@@ -1,4 +1,4 @@\n");
    out.push_str(&format!("-{}\n+# changed\n", rules[0]));
    for line in &rules[1..4] {
        out.push_str(&format!(" {}\n", line));
    }
    let _ = repo;
    out
}

#[test]
fn test_WB_REV_28_a_changed_added_and_removed_file_is_a_line_each_in_the_order_of_the_paths_and_the_status_is_1() {
    let repo = changed_repo();
    let run = from(&repo.path, REVISION, PINNED);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert_eq!(text(&run.stderr), "");
    let out = text(&run.stdout);
    let head: Vec<&str> = out.lines().take_while(|line| !line.is_empty()).collect();
    assert_eq!(
        head,
        ["removed: revision/models.yml", "changed: revision/roles/build.md", "added: revision/roles/extra.md", "changed: revision/rules.yml"]
    );
}

#[test]
fn test_WB_REV_28_the_unified_diff_of_a_changed_file_follows_the_lines_with_the_names_a_and_b_in_the_order_given() {
    let repo = changed_repo();
    let run = from(&repo.path, REVISION, PINNED);
    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), expected_diff(&repo));

    // Swapped, a is the pinned revision: what was added is removed, and the diff goes the other way.
    let run = from(&repo.path, PINNED, REVISION);
    assert_eq!(code(&run), Some(1));
    let out = text(&run.stdout);
    assert!(out.starts_with("added: revision/models.yml\nchanged: revision/roles/build.md\nremoved: revision/roles/extra.md\nchanged: revision/rules.yml\n\n"), "{}", out);
    assert!(out.contains("--- a/revision/roles/build.md\n+++ b/revision/roles/build.md\n"), "{}", out);
    assert!(out.contains("-A new last line.\n"), "{}", out);
    assert!(out.contains("+# changed\n") == false && out.contains("-# changed\n"), "{}", out);
}

#[test]
fn test_WB_REV_28_a_diff_has_only_the_files_that_changed() {
    let repo = pinned_with(|watch| {
        fs::write(watch.join("revision/roles/extra.md"), "Extra.\n").unwrap();
    });
    let run = from(&repo.path, REVISION, PINNED);
    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "added: revision/roles/extra.md\n");
}

#[test]
fn test_WB_REV_29_a_version_or_a_hash_that_is_not_held_ends_with_2_and_prints_nothing_on_standard_output() {
    let repo = pinned_with(|_| {});
    let unknown_hash = "0".repeat(64);
    let released = manifest::released(share().join("revisions.txt")).unwrap();
    let old = released.iter().find(|(version, _)| version != REVISION).unwrap().clone();
    for args in [
        vec![REVISION, "9.9.9"],
        vec!["9.9.9", REVISION],
        vec![REVISION, unknown_hash.as_str()],
        vec![REVISION, old.0.as_str()],
        vec![REVISION, old.1.as_str()],
        // Without `--from`, the version of a repository is not held.
        vec![REVISION, PINNED],
    ] {
        let run = compare(&args);
        assert_eq!(code(&run), Some(2), "{:?}", args);
        assert_eq!(text(&run.stdout), "", "{:?}", args);
        let said = text(&run.stderr);
        assert!(said.starts_with("wb compare: "), "{:?}: {}", args, said);
        assert!(said.contains(args[1]) || said.contains(args[0]), "{:?}: {}", args, said);
        assert!(!said.contains("usage:"), "{:?}: {}", args, said);
    }
    // With `--from`, a version it does not have is not held either.
    let run = from(&repo.path, REVISION, "9.9.9");
    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stdout), "");
}

#[test]
fn test_WB_REV_29_a_repository_whose_files_are_not_its_pinned_revision_ends_with_2() {
    // A file changed after the pin.
    let repo = pinned_with(|_| {});
    let rules = repo.path.join(".watchbill/revision/rules.yml");
    fs::write(&rules, format!("{}# more\n", read(&rules))).unwrap();
    let run = from(&repo.path, REVISION, PINNED);
    assert_eq!(code(&run), Some(2), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("wb compare: "), "{}", text(&run.stderr));

    // No `.watchbill/` at all.
    let empty = folder_named("empty");
    let run = from(&empty.path, REVISION, PINNED);
    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains(".watchbill"), "{}", text(&run.stderr));

    // A `watchbill.yml` with no line `revision:`.
    let repo = pinned_with(|_| {});
    fs::write(repo.path.join(".watchbill/watchbill.yml"), format!("watchbill: {}\n", PINNED)).unwrap();
    let run = from(&repo.path, REVISION, PINNED);
    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("revision"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_REV_29_a_version_that_two_different_revisions_have_ends_with_2_and_the_hash_names_one() {
    let repo = pinned_with(|watch| fs::write(watch.join("revision/roles/extra.md"), "Extra.\n").unwrap());
    pin(&repo.path, REVISION);
    let run = from(&repo.path, REVISION, REVISION);
    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("two revisions"), "{}", text(&run.stderr));

    let there = pinned_hash(&repo.path);
    let here = installed_hash();
    let run = from(&repo.path, &here, &there);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "added: revision/roles/extra.md\n");
}

#[test]
fn test_WB_REV_30_a_revision_named_by_version_or_by_hash_gives_the_same_output() {
    let repo = changed_repo();
    let there = pinned_hash(&repo.path);
    let here = installed_hash();
    let by_version = from(&repo.path, REVISION, PINNED);
    for (a, b) in [(here.as_str(), PINNED), (REVISION, there.as_str()), (here.as_str(), there.as_str())] {
        let run = from(&repo.path, a, b);
        assert_eq!(code(&run), Some(1));
        assert_eq!(run.stdout, by_version.stdout, "{} {}", a, b);
    }
}

#[test]
fn test_WB_REV_31_a_link_or_a_file_not_in_the_fixed_form_in_the_pinned_revision_is_refused_by_name() {
    let repo = pinned_with(|_| {});
    let watch = repo.path.join(".watchbill/revision");
    symlink("rules.yml", watch.join("roles/link.md")).unwrap();
    let run = from(&repo.path, REVISION, PINNED);
    assert_eq!(code(&run), Some(2), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("roles/link.md"), "{}", text(&run.stderr));

    let repo = pinned_with(|_| {});
    fs::write(repo.path.join(".watchbill/revision/roles/crlf.md"), "a\r\nb\r\n").unwrap();
    let run = from(&repo.path, REVISION, PINNED);
    assert_eq!(code(&run), Some(2), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("roles/crlf.md"), "{}", text(&run.stderr));

    // `.watchbill` itself as a link is never followed.
    let real = pinned_with(|_| {});
    let outer = folder_named("outer");
    symlink(real.path.join(".watchbill"), outer.path.join(".watchbill")).unwrap();
    let run = from(&outer.path, REVISION, PINNED);
    assert_eq!(code(&run), Some(2), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "");
}

#[test]
fn test_WB_REV_32_the_output_does_not_change_with_the_git_configuration_of_the_person() {
    let repo = changed_repo();
    let plain = from(&repo.path, REVISION, PINNED);
    assert_eq!(code(&plain), Some(1));

    let home = folder_named("home");
    let config = home.path.join("config");
    fs::write(
        &config,
        "[diff]\n\tnoprefix = true\n\tcontext = 0\n\talgorithm = patience\n\tmnemonicPrefix = true\n\tcolorMoved = zebra\n\texternal = /bin/false\n\tinterHunkContext = 5\n[color]\n\tui = always\n\tdiff = always\n[core]\n\tabbrev = 4\n\tattributesFile = /nonexistent\n[diff \"x\"]\n\ttextconv = /bin/false\n",
    )
    .unwrap();
    // The configuration is given in the environment of the program, where git looks for it.
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(["compare", "--from", repo.path.to_str().unwrap(), REVISION, PINNED])
        .current_dir(&home.path)
        .env_clear()
        .envs(env())
        .env("HOME", &home.path)
        .env("XDG_CONFIG_HOME", &home.path)
        .env("GIT_CONFIG_GLOBAL", &config)
        .env("GIT_CONFIG_SYSTEM", &config)
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "diff.noprefix")
        .env("GIT_CONFIG_VALUE_0", "true")
        .env("GIT_EXTERNAL_DIFF", "/bin/false")
        .env("GIT_DIFF_OPTS", "--unified=0")
        .output()
        .unwrap();
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert_eq!(run.stdout, plain.stdout);
}

/// Every file below a folder with its bytes, to hold that nothing was written.
fn tree(folder: &Path, found: &mut Vec<(String, Vec<u8>)>) {
    let mut entries: Vec<_> = fs::read_dir(folder).unwrap().map(|entry| entry.unwrap()).collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            found.push((format!("{}/", path.display()), Vec::new()));
            tree(&path, found);
        } else {
            found.push((path.display().to_string(), fs::read(&path).unwrap()));
        }
    }
}

#[test]
fn test_WB_REV_33_it_writes_nothing_and_asks_nothing_of_the_network() {
    let repo = changed_repo();
    let here = folder();
    let mut before = Vec::new();
    tree(&repo.path, &mut before);
    tree(&share(), &mut before);
    tree(&here.path, &mut before);
    let run = wb(&here.path, &["compare", "--from", repo.path.to_str().unwrap(), REVISION, PINNED], "");
    assert_eq!(code(&run), Some(1));
    let mut after = Vec::new();
    tree(&repo.path, &mut after);
    tree(&share(), &mut after);
    tree(&here.path, &mut after);
    assert!(before == after, "a file was written");

    // The command has no network in it: no word of the sources that connects or asks a server.
    let source = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/compare.rs")).unwrap();
    for word in ["TcpStream", "UdpSocket", "curl", "ssh", "http", "fetch"] {
        assert!(!source.contains(word), "src/compare.rs names {}", word);
    }
}

#[test]
fn test_WB_REV_34_a_wrong_use_ends_with_2_and_prints_the_usage() {
    let repo = pinned_with(|_| {});
    let path = repo.path.to_str().unwrap();
    let cases: Vec<Vec<&str>> = vec![
        vec![],
        vec![REVISION],
        vec![REVISION, REVISION, REVISION],
        vec!["--from"],
        vec!["--from", path],
        vec!["--from", path, "--from", path, REVISION, PINNED],
        vec!["--nothing", REVISION, REVISION],
        vec!["-x", REVISION, REVISION],
        vec!["--from=", REVISION, REVISION],
    ];
    for args in cases {
        let run = compare(&args);
        assert_eq!(code(&run), Some(2), "{:?}: {}", args, text(&run.stderr));
        assert_eq!(text(&run.stdout), "", "{:?}", args);
        let said = text(&run.stderr);
        assert!(said.starts_with("wb compare: "), "{:?}: {}", args, said);
        assert!(said.contains(&format!("\n\n{}", watchbill::USAGE)), "{:?}: {}", args, said);
    }
}
