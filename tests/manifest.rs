//! The manifest of the revision, the hash of it, `revisions.txt` and `wb revision`.
//! Every test of tests/test_manifest.py is here under the name it has, and then the tests of #24.
//! The Python tests that replace a function with a mock are written with the file system itself:
//! a folder with a mode, a named pipe, a file that is turned into a pipe by a hook.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

use sha2::{Digest, Sha256};
use watchbill::manifest::{self, ManifestError};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn package() -> PathBuf {
    root().join("share/watchbill")
}

fn fixtures() -> PathBuf {
    root().join("tests/fixtures")
}

fn small() -> PathBuf {
    fixtures().join("small")
}

fn read_bytes(path: impl AsRef<Path>) -> Vec<u8> {
    fs::read(path).expect("the file could not be read")
}

fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data).iter().map(|byte| format!("{:02x}", byte)).collect()
}

/// A folder made for one test, with the files the test puts in it.
struct Scratch {
    folder: common::Folder,
}

impl Scratch {
    fn new() -> Scratch {
        Scratch { folder: common::folder() }
    }

    fn path(&self) -> &Path {
        &self.folder.path
    }

    fn put(&self, path: &str, data: &[u8]) {
        let file = self.path().join(path);
        fs::create_dir_all(file.parent().unwrap()).expect("the folder could not be made");
        fs::write(file, data).expect("the file could not be written");
    }

    fn manifest(&self) -> Result<String, ManifestError> {
        manifest::manifest(self.path())
    }

    /// The message of the refusal. The manifest is expected to be refused.
    fn refused(&self) -> String {
        match self.manifest() {
            Ok(text) => panic!("the manifest was given: {}", text),
            Err(error) => error.to_string(),
        }
    }
}

/// Gives a folder its mode back when the test is over, so that it can be removed.
struct Restore(PathBuf);

impl Drop for Restore {
    fn drop(&mut self) {
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
    }
}

fn chmod(path: &Path, mode: u32) -> Restore {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("the mode could not be set");
    Restore(path.to_path_buf())
}

/// Root enters and reads whatever the mode is, so a test of a mode cannot be run as root.
fn is_root(scratch: &Scratch) -> bool {
    fs::metadata(scratch.path()).map(|meta| meta.uid() == 0).unwrap_or(false)
}

fn mkfifo(path: &Path) {
    let status = Command::new("mkfifo").arg(path).status().expect("mkfifo could not be started");
    assert!(status.success());
}

/// Runs the work in a thread and fails when it does not end: a pipe that is opened hangs.
fn within<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static) -> T {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = sender.send(work());
    });
    receiver.recv_timeout(Duration::from_secs(20)).expect("it did not end: a file that is no plain file was waited on")
}

/// Every path below the folder, with the bytes of each file.
fn snapshot(folder: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut seen = BTreeMap::new();
    let mut open = vec![folder.to_path_buf()];
    while let Some(base) = open.pop() {
        for entry in fs::read_dir(&base).expect("the folder could not be listed") {
            let path = entry.unwrap().path();
            if path.is_dir() {
                seen.insert(path.clone(), None);
                open.push(path);
            } else {
                seen.insert(path.clone(), Some(read_bytes(&path)));
            }
        }
    }
    seen
}

fn copy_folder(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_folder(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn lines_of(text: &str) -> Vec<String> {
    text.lines().map(String::from).collect()
}

// ---- TestManifest

#[test]
fn test_WB_REV_2_the_manifest_of_a_fixed_set_of_files_is_the_fixture_byte_for_byte() {
    assert_eq!(
        manifest::manifest(small()).unwrap().into_bytes(),
        read_bytes(fixtures().join("small.sha256"))
    );
}

#[test]
fn test_WB_REV_2_a_line_is_the_hash_two_spaces_and_the_path() {
    for line in lines_of(&manifest::manifest(small()).unwrap()) {
        let (digest, path) = line.split_once("  ").unwrap();
        assert_eq!(digest.len(), 64);
        assert_eq!(digest, sha256_hex(&read_bytes(small().join(path))));
    }
}

#[test]
fn test_WB_REV_2_the_paths_are_sorted_by_path_with_capitals_first() {
    let paths: Vec<String> = lines_of(&manifest::manifest(small()).unwrap())
        .iter()
        .map(|line| line.split_once("  ").unwrap().1.to_string())
        .collect();
    assert_eq!(paths, ["revision/B.md", "revision/a.md", "revision/sub/c.yml", "revision/sub/d.md"]);
}

#[test]
fn test_WB_REV_2_the_order_does_not_follow_the_order_the_file_system_gives() {
    // The files are made backwards, and the order they were made in is the one a file system may give.
    let scratch = Scratch::new();
    for path in ["revision/z.md", "revision/sub/y.md", "revision/sub/B.md", "revision/a.md", "revision/B.md"] {
        scratch.put(path, b"one\n");
    }
    let paths: Vec<String> = lines_of(&scratch.manifest().unwrap())
        .iter()
        .map(|line| line.split_once("  ").unwrap().1.to_string())
        .collect();
    assert_eq!(
        paths,
        ["revision/B.md", "revision/a.md", "revision/sub/B.md", "revision/sub/y.md", "revision/z.md"]
    );
    let again = Scratch::new();
    for path in ["revision/B.md", "revision/a.md", "revision/sub/B.md", "revision/sub/y.md", "revision/z.md"] {
        again.put(path, b"one\n");
    }
    assert_eq!(again.manifest().unwrap(), scratch.manifest().unwrap());
}

#[test]
fn test_WB_REV_2_the_paths_use_a_slash_and_start_with_revision() {
    for line in lines_of(&manifest::manifest(small()).unwrap()) {
        let path = line.split_once("  ").unwrap().1;
        assert!(path.starts_with("revision/"), "{}", path);
        assert!(!path.contains('\\'), "{}", path);
    }
}

#[test]
fn test_WB_REV_2_the_manifest_ends_with_one_line_break_and_has_no_other_empty_line() {
    let text = manifest::manifest(small()).unwrap();
    assert!(text.ends_with('\n'));
    assert!(!text.contains("\n\n"));
}

#[test]
fn test_WB_REV_3_the_hash_of_the_revision_is_the_sha256_of_the_manifest() {
    let expected = sha256_hex(&read_bytes(fixtures().join("small.sha256")));
    assert_eq!(manifest::revision_hash(small()).unwrap(), expected);
    assert_eq!(expected, "15eb321a06cfdecea8ed004c5e6697de33f28fb8e27097671c75262e7caa8b6f");
}

#[test]
fn test_WB_REV_3_a_changed_byte_in_a_file_changes_the_hash() {
    let scratch = Scratch::new();
    copy_folder(&small(), &scratch.path().join("copy"));
    let before = manifest::revision_hash(scratch.path().join("copy")).unwrap();
    scratch.put("copy/revision/a.md", b"two\n");
    assert_ne!(manifest::revision_hash(scratch.path().join("copy")).unwrap(), before);
}

// ---- TestForm

fn refused_form(path: &str, data: &[u8]) -> String {
    let scratch = Scratch::new();
    scratch.put(&format!("revision/{}", path), data);
    scratch.refused()
}

#[test]
fn test_WB_REV_4_a_file_with_crlf_is_refused_and_named() {
    let scratch = Scratch::new();
    scratch.put("revision/good.md", b"fine\n");
    scratch.put("revision/roles/bad.md", b"one\r\ntwo\r\n");
    assert!(scratch.refused().contains("revision/roles/bad.md"));
}

#[test]
fn test_WB_REV_4_a_file_that_is_not_utf8_is_refused_and_named() {
    let scratch = Scratch::new();
    scratch.put("revision/good.md", b"fine\n");
    scratch.put("revision/latin.md", b"Gr\xfc\xdfe\n");
    assert!(scratch.refused().contains("revision/latin.md"));
}

#[test]
fn test_WB_REV_4_a_file_with_a_lone_cr_is_refused_and_named() {
    assert!(refused_form("old.md", b"one\rtwo\n").contains("revision/old.md"));
}

#[test]
fn test_WB_REV_4_a_file_with_utf8_and_lf_is_taken() {
    let scratch = Scratch::new();
    scratch.put("revision/ok.md", "Grüße\n".as_bytes());
    assert!(scratch.manifest().unwrap().contains("revision/ok.md"));
}

#[test]
fn test_WB_REV_11_a_file_without_a_line_break_at_the_end_is_refused_and_named() {
    assert!(refused_form("open.md", b"no end").contains("revision/open.md"));
}

#[test]
fn test_WB_REV_11_an_empty_file_is_refused_and_named() {
    assert!(refused_form("empty.md", b"").contains("revision/empty.md"));
}

// ---- TestWhatIsRefused

#[test]
fn test_WB_REV_14_a_revision_folder_that_holds_no_file_is_refused_by_name() {
    let scratch = Scratch::new();
    fs::create_dir_all(scratch.path().join("revision/roles")).unwrap();
    assert!(scratch.refused().contains("revision/"));
}

#[test]
fn test_WB_REV_14_no_revision_folder_is_refused_by_name() {
    assert!(Scratch::new().refused().contains("revision/"));
}

#[test]
fn test_WB_REV_14_the_hash_of_nothing_is_not_given() {
    assert!(manifest::revision_hash(Scratch::new().path()).is_err());
}

#[test]
fn test_WB_REV_14_wb_revision_with_no_file_ends_with_1_and_prints_no_revision() {
    let scratch = Scratch::new();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = watchbill::revision(&[], scratch.path(), &mut out, &mut err);
    assert_eq!(status, 1);
    assert!(String::from_utf8(err).unwrap().contains("revision/"));
    assert!(out.is_empty());
}

#[test]
fn test_WB_REV_16_a_file_that_cannot_be_read_is_refused_by_name() {
    let scratch = Scratch::new();
    scratch.put("revision/a.md", b"one\n");
    if is_root(&scratch) {
        return;
    }
    let _restore = chmod(&scratch.path().join("revision/a.md"), 0);
    assert!(scratch.refused().contains("revision/a.md"));
}

#[test]
fn test_WB_REV_16_a_file_whose_mode_cannot_be_looked_at_is_refused_by_name() {
    let scratch = Scratch::new();
    scratch.put("revision/a.md", b"one\n");
    scratch.put("revision/roles/b.md", b"one\n");
    if is_root(&scratch) {
        return;
    }
    // The folder can be listed and cannot be searched: the kind of what is in it cannot be looked at.
    let _restore = chmod(&scratch.path().join("revision/roles"), 0o444);
    assert!(scratch.refused().contains("revision/roles/b.md"));
}

#[test]
fn test_WB_REV_16_a_folder_that_can_be_listed_and_not_entered_is_refused_by_name() {
    let scratch = Scratch::new();
    scratch.put("revision/ok.md", b"one\n");
    scratch.put("revision/roles/b.md", b"one\n");
    if is_root(&scratch) {
        return;
    }
    let _restore = chmod(&scratch.path().join("revision/roles"), 0o444);
    assert!(scratch.refused().contains("revision/roles/b.md"));
}

#[test]
fn test_WB_REV_16_a_file_name_that_is_not_utf8_is_refused_and_shown() {
    let scratch = Scratch::new();
    scratch.put("revision/ok.md", b"one\n");
    let name = scratch.path().join("revision").join(OsStr::from_bytes(b"bad\xff.md"));
    if fs::write(name, b"x\n").is_err() {
        return; // this file system takes no such name
    }
    let message = scratch.refused();
    assert!(message.contains("revision/bad"), "{}", message);
    assert!(message.contains("UTF-8"), "{}", message);
}

#[test]
fn test_WB_REV_16_a_folder_name_that_is_not_utf8_is_refused_and_shown_with_no_panic() {
    let scratch = Scratch::new();
    scratch.put("revision/ok.md", b"one\n");
    let folder = scratch.path().join("revision").join(OsStr::from_bytes(b"bad\xfe"));
    if fs::create_dir(&folder).is_err() {
        return; // this file system takes no such name
    }
    fs::write(folder.join("x.md"), b"x\n").unwrap();
    let message = scratch.refused();
    assert!(message.contains("revision/bad"), "{}", message);
    assert!(message.contains("UTF-8"), "{}", message);
}

#[test]
fn test_WB_REV_20_a_folder_that_cannot_be_listed_is_refused_by_name_when_the_listing_fails() {
    let scratch = Scratch::new();
    scratch.put("revision/ok.md", b"one\n");
    scratch.put("revision/roles/a.md", b"one\n");
    if is_root(&scratch) {
        return;
    }
    // It can be entered and cannot be read.
    let _restore = chmod(&scratch.path().join("revision/roles"), 0o300);
    assert!(scratch.refused().contains("revision/roles"));
}

#[test]
fn test_WB_REV_20_a_folder_that_cannot_be_read_by_its_mode_is_refused_by_name() {
    let scratch = Scratch::new();
    scratch.put("revision/ok.md", b"one\n");
    scratch.put("revision/roles/a.md", b"one\n");
    if is_root(&scratch) {
        return;
    }
    let _restore = chmod(&scratch.path().join("revision/roles"), 0);
    assert!(scratch.refused().contains("revision/roles"));
}

#[test]
fn test_WB_REV_21_a_named_pipe_is_refused_by_name_and_not_opened() {
    let scratch = Scratch::new();
    scratch.put("revision/ok.md", b"one\n");
    mkfifo(&scratch.path().join("revision/pipe"));
    let base = scratch.path().to_path_buf();
    // Opened for reading, a pipe with no writer is waited on for ever.
    let message = within(move || manifest::manifest(&base).unwrap_err().to_string());
    assert!(message.contains("revision/pipe"), "{}", message);
}

#[test]
fn test_WB_REV_17_a_file_name_with_a_line_break_is_refused_and_shown() {
    let scratch = Scratch::new();
    scratch.put("revision/a\nb.md", b"one\n");
    assert!(scratch.refused().contains("revision/a"));
}

#[test]
fn test_WB_REV_17_a_file_name_with_a_backslash_is_refused_and_shown() {
    let scratch = Scratch::new();
    scratch.put("revision/a\\b.md", b"one\n");
    assert!(scratch.refused().contains("revision/a"));
}

#[test]
fn test_WB_REV_17_a_file_name_with_a_carriage_return_is_refused_and_shown() {
    let scratch = Scratch::new();
    scratch.put("revision/a\rb.md", b"one\n");
    assert!(scratch.refused().contains("revision/a"));
}

#[test]
fn test_WB_REV_17_a_name_with_a_space_or_non_ascii_letters_is_taken() {
    let scratch = Scratch::new();
    scratch.put("revision/a b.md", b"one\n");
    scratch.put("revision/gr\u{fc}.md", b"one\n");
    assert!(scratch.manifest().unwrap().contains("revision/a b.md"));
}

#[test]
fn test_WB_REV_18_a_symbolic_link_to_a_file_is_refused_by_name() {
    let scratch = Scratch::new();
    scratch.put("outside.md", b"one\n");
    scratch.put("revision/ok.md", b"one\n");
    std::os::unix::fs::symlink(scratch.path().join("outside.md"), scratch.path().join("revision/link.md")).unwrap();
    assert!(scratch.refused().contains("revision/link.md"));
}

#[test]
fn test_WB_REV_18_a_symbolic_link_to_a_folder_is_refused_by_name() {
    let scratch = Scratch::new();
    scratch.put("revision/ok.md", b"one\n");
    fs::create_dir_all(scratch.path().join("elsewhere")).unwrap();
    std::os::unix::fs::symlink(scratch.path().join("elsewhere"), scratch.path().join("revision/linked")).unwrap();
    assert!(scratch.refused().contains("revision/linked"));
}

#[test]
fn test_WB_REV_18_a_broken_symbolic_link_is_refused_by_name() {
    let scratch = Scratch::new();
    scratch.put("revision/ok.md", b"one\n");
    std::os::unix::fs::symlink(scratch.path().join("nothing"), scratch.path().join("revision/gone.md")).unwrap();
    assert!(scratch.refused().contains("revision/gone.md"));
}

#[test]
fn test_WB_REV_18_a_hidden_file_is_refused_by_name() {
    let scratch = Scratch::new();
    scratch.put("revision/ok.md", b"one\n");
    scratch.put("revision/.a.md.swp", b"one\n");
    assert!(scratch.refused().contains("revision/.a.md.swp"));
}

#[test]
fn test_WB_REV_18_a_file_in_a_hidden_folder_is_refused_by_name() {
    let scratch = Scratch::new();
    scratch.put("revision/ok.md", b"one\n");
    scratch.put("revision/.git/x.md", b"one\n");
    assert!(scratch.refused().contains("revision/.git"));
}

#[test]
fn test_WB_REV_18_a_backup_file_that_ends_with_a_tilde_is_refused_by_name() {
    let scratch = Scratch::new();
    scratch.put("revision/ok.md", b"one\n");
    scratch.put("revision/a.md~", b"one\n");
    assert!(scratch.refused().contains("revision/a.md~"));
}

// ---- TestPackageData: the files the spec installs

/// The patterns of the files the spec installs under `/usr/share/watchbill/`: the lines of
/// `%files` that begin with `%{_datadir}/watchbill/`, written relative to that folder.
fn patterns() -> Vec<String> {
    patterns_of(&String::from_utf8(read_bytes(root().join("packaging/watchbill.spec"))).unwrap())
}

/// The same for the text of a spec. A line that leaves a file of the revision out of the package,
/// `%exclude` of a path under `/usr/share/watchbill`, is refused: rpmbuild does not stop at it.
fn patterns_of(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let files = text.split("\n%files").nth(1).expect("the spec has no %files");
    for line in files.lines() {
        let line = line.trim();
        assert!(
            !(line.starts_with("%exclude") && line.contains("watchbill")),
            "the spec excludes a file of the revision: {}",
            line
        );
        if let Some(rest) = line.strip_prefix("%{_datadir}/watchbill/") {
            found.push(rest.to_string());
        }
    }
    found
}

/// `fnmatch.fnmatchcase` for a pattern with `*` and `?`.
fn fnmatch(pattern: &[char], name: &[char]) -> bool {
    match pattern.first() {
        None => name.is_empty(),
        Some('*') => (0..=name.len()).any(|skip| fnmatch(&pattern[1..], &name[skip..])),
        Some('?') => !name.is_empty() && fnmatch(&pattern[1..], &name[1..]),
        Some(c) => name.first() == Some(c) && fnmatch(&pattern[1..], &name[1..]),
    }
}

fn uncovered(paths: &[&str], patterns: &[String]) -> Vec<String> {
    // A pattern of the spec takes one folder level for each `*`-free part: no `**`.
    paths
        .iter()
        .filter(|path| {
            !patterns.iter().any(|pattern| {
                let (p, n): (Vec<char>, Vec<char>) = (pattern.chars().collect(), path.chars().collect());
                fnmatch(&p, &n) && path.matches('/').count() == pattern.matches('/').count()
            })
        })
        .map(|path| path.to_string())
        .collect()
}

fn committed_paths() -> Vec<String> {
    let text = String::from_utf8(read_bytes(package().join("revision.sha256"))).unwrap();
    text.lines().map(|line| line.split_once("  ").unwrap().1.to_string()).collect()
}

#[test]
fn test_WB_REV_22_every_path_of_the_committed_manifest_is_among_the_files_the_spec_installs() {
    let paths = committed_paths();
    let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
    assert!(!patterns().is_empty(), "the spec installs no file of the revision");
    assert_eq!(uncovered(&paths, &patterns()), Vec::<String>::new());
}

#[test]
fn test_WB_REV_22_the_manifest_and_revisions_txt_are_among_the_files_the_spec_installs() {
    assert_eq!(uncovered(&["revision.sha256", "revisions.txt"], &patterns()), Vec::<String>::new());
}

#[test]
fn test_WB_REV_22_a_file_in_a_new_folder_or_of_a_new_type_is_found() {
    let paths = ["revision/lenses/one.md", "revision/roles/x.txt", "revision/roles/a.md"];
    assert_eq!(uncovered(&paths, &patterns()), ["revision/lenses/one.md", "revision/roles/x.txt"]);
}

// ---- TestCommitted

#[test]
fn test_WB_REV_5_the_committed_manifest_is_the_one_computed_from_the_files() {
    assert_eq!(
        String::from_utf8(read_bytes(package().join("revision.sha256"))).unwrap(),
        manifest::manifest(package()).unwrap()
    );
}

#[test]
fn test_WB_REV_5_a_changed_prompt_without_a_new_manifest_fails() {
    let scratch = Scratch::new();
    let copy = scratch.path().join("watchbill");
    copy_folder(&package(), &copy);
    let build = copy.join("revision/roles/build.md");
    let mut data = read_bytes(&build);
    data.extend_from_slice(b"One more line.\n");
    fs::write(&build, data).unwrap();
    assert_ne!(
        String::from_utf8(read_bytes(copy.join("revision.sha256"))).unwrap(),
        manifest::manifest(&copy).unwrap()
    );
}

/// `sha256sum -c revision.sha256` in the folder, or None when `sha256sum` is not here.
fn sha256sum_check(folder: &Path) -> Option<bool> {
    Command::new("sha256sum")
        .args(["-c", "revision.sha256"])
        .current_dir(folder)
        .output()
        .ok()
        .map(|run| run.status.success())
}

#[test]
fn test_WB_REV_6_sha256sum_passes_on_the_manifest_in_the_folder_that_holds_revision() {
    if let Some(passed) = sha256sum_check(&package()) {
        assert!(passed);
    }
}

#[test]
fn test_WB_REV_6_sha256sum_fails_when_a_file_is_changed() {
    let scratch = Scratch::new();
    let copy = scratch.path().join("watchbill");
    copy_folder(&package(), &copy);
    let fix = copy.join("revision/roles/fix.md");
    let mut data = read_bytes(&fix);
    data.extend_from_slice(b"x\n");
    fs::write(&fix, data).unwrap();
    if let Some(passed) = sha256sum_check(&copy) {
        assert!(!passed);
    }
}

#[test]
fn test_WB_REV_6_every_file_under_revision_is_in_the_committed_manifest() {
    let mut listed = committed_paths();
    listed.sort();
    let mut found: Vec<String> = snapshot(&package().join("revision"))
        .into_iter()
        .filter(|(_, data)| data.is_some())
        .map(|(path, _)| path.strip_prefix(package()).unwrap().to_str().unwrap().to_string())
        .collect();
    found.sort();
    assert_eq!(listed, found);
    assert!(listed.len() >= 7);
}

// ---- TestPrompts

fn prompt_problems(folder: &Path, named: &[String]) -> Vec<(&'static str, String)> {
    // What is wrong with the prompt files in a folder of roles: a file no role names, a name without a file.
    let mut found: Vec<String> = fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name != "placeholders.md")
        .collect();
    found.sort();
    let mut problems: Vec<(&'static str, String)> =
        found.iter().filter(|name| !named.contains(name)).map(|name| ("no role names it", name.clone())).collect();
    problems.extend(named.iter().filter(|name| !found.contains(name)).map(|name| ("no file", name.clone())));
    problems
}

fn named() -> Vec<String> {
    let rules = watchbill::rules::load_revision().expect("the tables could not be read");
    let mut names: Vec<String> = rules
        .roles
        .values()
        .filter(|role| role.prompt != "none")
        .map(|role| role.prompt.rsplit('/').next().unwrap().to_string())
        .collect();
    names.sort();
    names
}

#[test]
fn test_WB_REV_7_every_prompt_file_a_role_names_exists() {
    let rules = watchbill::rules::load_revision().expect("the tables could not be read");
    for role in rules.roles.values() {
        if role.prompt != "none" {
            let path = package().join("revision").join(&role.prompt);
            assert!(path.is_file(), "{}", role.prompt);
        }
    }
}

#[test]
fn test_WB_REV_7_every_prompt_file_is_named_by_a_role() {
    // placeholders.md is the list of words a prompt is filled in with (WB-ROLE-3). It is no prompt.
    assert_eq!(prompt_problems(&package().join("revision/roles"), &named()), vec![]);
}

#[test]
fn test_WB_REV_7_a_prompt_file_that_no_role_names_is_found_by_the_same_check() {
    let scratch = Scratch::new();
    let mut names = named();
    names.extend(["placeholders.md".to_string(), "stray.md".to_string()]);
    for name in names {
        scratch.put(&name, b"x\n");
    }
    assert_eq!(
        prompt_problems(scratch.path(), &named()),
        vec![("no role names it", "stray.md".to_string())]
    );
}

#[test]
fn test_WB_REV_7_a_role_whose_file_is_missing_is_found_by_the_same_check() {
    let scratch = Scratch::new();
    assert_eq!(
        prompt_problems(scratch.path(), &["tester.md".to_string()]),
        vec![("no file", "tester.md".to_string())]
    );
}

#[test]
fn test_WB_REV_7_the_file_of_the_placeholders_is_there_and_no_role_names_it() {
    assert!(package().join("revision/roles/placeholders.md").is_file());
    assert!(!named().contains(&"placeholders.md".to_string()));
}

// ---- TestReleased

fn wrong_hashes(entries: &[(String, String)], version: &str, digest: &str) -> Vec<(String, String)> {
    // The entries that name the version with another hash than the computed one.
    entries.iter().filter(|(v, d)| v == version && d != digest).cloned().collect()
}

fn pair(version: &str, digest: &str) -> (String, String) {
    (version.to_string(), digest.to_string())
}

fn released_of(scratch: &Scratch) -> Result<Vec<(String, String)>, ManifestError> {
    manifest::released(scratch.path().join("revisions.txt"))
}

fn released_refusal(data: &[u8]) -> String {
    let scratch = Scratch::new();
    scratch.put("revisions.txt", data);
    released_of(&scratch).unwrap_err().to_string()
}

#[test]
fn test_WB_REV_8_the_current_version_does_not_stand_with_another_hash() {
    let entries = manifest::released(package().join(manifest::RELEASED_FILE)).unwrap();
    assert_eq!(wrong_hashes(&entries, VERSION, &manifest::revision_hash(package()).unwrap()), vec![]);
}

#[test]
fn test_WB_REV_8_the_same_check_finds_the_version_with_another_hash() {
    let scratch = Scratch::new();
    scratch.put("revisions.txt", format!("{}  {}\n", VERSION, "0".repeat(64)).as_bytes());
    let entries = released_of(&scratch).unwrap();
    assert_eq!(
        wrong_hashes(&entries, VERSION, &manifest::revision_hash(package()).unwrap()),
        vec![pair(VERSION, &"0".repeat(64))]
    );
}

#[test]
fn test_WB_REV_8_the_same_check_lets_the_right_hash_and_other_versions_pass() {
    let digest = manifest::revision_hash(package()).unwrap();
    let entries = [pair(VERSION, &digest), pair("0.0.0", &"0".repeat(64))];
    assert_eq!(wrong_hashes(&entries, VERSION, &digest), vec![]);
}

#[test]
fn test_WB_REV_8_a_line_that_is_not_a_version_and_a_hash_is_refused_and_names_the_line() {
    let message = released_refusal(b"0.0.1  abc\n");
    assert!(message.contains("revisions.txt"), "{}", message);
    assert!(message.contains("line 1"), "{}", message);
}

#[test]
fn test_WB_REV_8_a_version_twice_is_refused() {
    let line = format!("0.0.1  {}\n", "a".repeat(64));
    let message = released_refusal(format!("{}{}", line, line).as_bytes());
    assert!(message.contains("0.0.1"), "{}", message);
}

#[test]
fn test_WB_REV_19_revisions_txt_with_crlf_is_refused_and_named() {
    let message = released_refusal(format!("0.0.1  {}\r\n", "a".repeat(64)).as_bytes());
    assert!(message.contains("revisions.txt"), "{}", message);
    assert!(message.contains("CR"), "{}", message);
}

#[test]
fn test_WB_REV_19_revisions_txt_without_a_line_break_at_the_end_is_refused_and_named() {
    let message = released_refusal(format!("0.0.1  {}", "a".repeat(64)).as_bytes());
    assert!(message.contains("revisions.txt"), "{}", message);
}

#[test]
fn test_WB_REV_19_revisions_txt_that_is_not_utf8_is_refused_and_named() {
    let message = released_refusal(b"\xff\xfe\n");
    assert!(message.contains("revisions.txt"), "{}", message);
}

#[test]
fn test_WB_REV_19_a_good_revisions_txt_with_two_versions_is_read() {
    let scratch = Scratch::new();
    scratch.put("revisions.txt", format!("0.0.1  {}\n0.0.2  {}\n", "a".repeat(64), "b".repeat(64)).as_bytes());
    assert_eq!(
        released_of(&scratch).unwrap(),
        vec![pair("0.0.1", &"a".repeat(64)), pair("0.0.2", &"b".repeat(64))]
    );
}

#[test]
fn test_WB_REV_8_the_file_is_in_the_package_and_holds_the_form() {
    let text = String::from_utf8(read_bytes(package().join("revisions.txt"))).unwrap();
    assert!(text.is_empty() || text.ends_with('\n'));
    manifest::released(package().join("revisions.txt")).unwrap();
}

// ---- TestCommand

fn run_wb(folder: &Path, args: &[&str]) -> std::process::Output {
    common::wb(folder, args, "")
}

#[test]
fn test_WB_REV_9_wb_revision_prints_the_version_the_hash_and_the_manifest_and_ends_with_0() {
    let scratch = Scratch::new();
    let done = run_wb(scratch.path(), &["revision"]);
    let out = common::text(&done.stdout);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(done.status.code(), Some(0), "{}", common::text(&done.stderr));
    assert!(lines[0].contains(VERSION));
    assert!(lines[1].contains(&manifest::revision_hash(package()).unwrap()));
    assert!(out.contains(&manifest::manifest(package()).unwrap()));
    assert_eq!(common::text(&done.stderr), "");
}

#[test]
fn test_WB_REV_9_the_hash_it_prints_is_the_sha256_of_the_manifest_it_prints() {
    let scratch = Scratch::new();
    let out = common::text(&run_wb(scratch.path(), &["revision"]).stdout);
    let lines: Vec<&str> = out.lines().collect();
    let digest = lines[1].split_whitespace().last().unwrap();
    let printed = format!("{}\n", lines[3..].join("\n"));
    assert_eq!(lines[2], "");
    assert_eq!(sha256_hex(printed.as_bytes()), digest);
}

#[test]
fn test_WB_REV_9_the_hash_it_prints_is_the_same_in_another_folder() {
    let (one, other) = (Scratch::new(), Scratch::new());
    assert_eq!(run_wb(one.path(), &["revision"]).stdout, run_wb(other.path(), &["revision"]).stdout);
}

#[test]
fn test_WB_REV_10_wb_revision_writes_nothing() {
    let scratch = Scratch::new();
    let watched = [scratch.path().to_path_buf(), root().join("src"), root().join("requirements")];
    let before: Vec<_> = watched.iter().map(|folder| snapshot(folder)).collect();
    let done = run_wb(scratch.path(), &["revision"]);
    assert_eq!(done.status.code(), Some(0), "{}", common::text(&done.stderr));
    let after: Vec<_> = watched.iter().map(|folder| snapshot(folder)).collect();
    assert!(before == after);
}

#[test]
fn test_WB_REV_12_wb_revision_with_an_argument_ends_with_2_and_prints_no_revision() {
    let scratch = Scratch::new();
    let done = run_wb(scratch.path(), &["revision", "--now"]);
    assert_eq!(done.status.code(), Some(2));
    assert_eq!(common::text(&done.stdout), "");
    assert!(common::text(&done.stderr).contains("--now"));
}

#[test]
fn test_WB_REV_13_wb_revision_with_a_file_that_is_refused_ends_with_1_and_names_it() {
    let scratch = Scratch::new();
    scratch.put("revision/bad.md", b"one\r\n");
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = watchbill::revision(&[], scratch.path(), &mut out, &mut err);
    assert_eq!(status, 1);
    assert!(String::from_utf8(err).unwrap().contains("revision/bad.md"));
    assert!(out.is_empty());
}

// ---- The tests of #24

#[test]
fn test_WB_REV_23_a_revision_folder_that_is_a_symbolic_link_is_refused_by_name() {
    let scratch = Scratch::new();
    scratch.put("elsewhere/a.md", b"one\n");
    std::os::unix::fs::symlink(scratch.path().join("elsewhere"), scratch.path().join("revision")).unwrap();
    let message = scratch.refused();
    assert!(message.contains("revision/"), "{}", message);
    assert!(message.contains("symbolic link"), "{}", message);
    let (mut out, mut err) = (Vec::new(), Vec::new());
    assert_eq!(watchbill::revision(&[], scratch.path(), &mut out, &mut err), 1);
    assert!(out.is_empty());
}

#[test]
fn test_WB_REV_24_a_file_that_becomes_a_named_pipe_after_it_was_looked_at_is_refused_without_waiting() {
    let scratch = Scratch::new();
    scratch.put("revision/a.md", b"one\n");
    scratch.put("revision/b.md", b"one\n");
    let base = scratch.path().to_path_buf();
    let message = within(move || {
        // The file is swapped for a pipe between the look at its kind and the open.
        let mut swap = |path: &str| {
            if path == "revision/b.md" {
                fs::remove_file(base.join(path)).unwrap();
                mkfifo(&base.join(path));
            }
        };
        manifest::manifest_with(&base, &mut swap).unwrap_err().to_string()
    });
    assert!(message.contains("revision/b.md"), "{}", message);
    assert!(message.contains("not a plain file"), "{}", message);
}

#[test]
fn test_WB_REV_25_no_option_and_no_environment_variable_changes_the_folder_read() {
    let scratch = Scratch::new();
    scratch.put("other/revision/x.md", b"one\n");
    let other = scratch.path().join("other");
    let plain = run_wb(scratch.path(), &["revision"]);
    let mut command = Command::new(env!("CARGO_BIN_EXE_wb"));
    command.arg("revision").current_dir(scratch.path());
    for name in [
        "WB_REVISION_FOLDER",
        "WB_FOLDER",
        "WATCHBILL_FOLDER",
        "WATCHBILL_REVISION",
        "WATCHBILL_HOME",
        "PACKAGE_FOLDER",
        "XDG_DATA_DIRS",
        "XDG_DATA_HOME",
        "DESTDIR",
    ] {
        command.env(name, &other);
    }
    let with_env = command.output().unwrap();
    assert_eq!(with_env.stdout, plain.stdout);
    assert_eq!(with_env.status.code(), Some(0));
    for option in ["--folder", "--revision", "--base"] {
        let run = run_wb(scratch.path(), &["revision", option, other.to_str().unwrap()]);
        assert_eq!(run.status.code(), Some(2), "{}", option);
        assert_eq!(common::text(&run.stdout), "", "{}", option);
    }
}

#[test]
fn test_WB_REV_25_the_code_that_reads_the_revision_does_not_read_the_environment() {
    let text = String::from_utf8(read_bytes(root().join("src/manifest.rs"))).unwrap();
    assert!(!text.contains("env::var"));
    assert!(!text.contains("env::args"));
    assert!(text.contains("PACKAGE_FOLDER"));
}

#[test]
fn test_WB_REV_24_a_file_that_becomes_a_link_after_it_was_looked_at_is_refused_by_name() {
    let scratch = Scratch::new();
    scratch.put("revision/a.md", b"one\n");
    scratch.put("revision/b.md", b"one\n");
    scratch.put("outside.md", b"two\n");
    let base = scratch.path().to_path_buf();
    let message = manifest::manifest_with(&base.clone(), &mut |path: &str| {
        if path == "revision/b.md" {
            fs::remove_file(base.join(path)).unwrap();
            std::os::unix::fs::symlink(base.join("outside.md"), base.join(path)).unwrap();
        }
    })
    .unwrap_err()
    .to_string();
    assert!(message.contains("revision/b.md"), "{}", message);
}

#[test]
fn test_WB_REV_24_a_file_that_is_replaced_by_another_after_it_was_looked_at_is_refused_by_name() {
    let scratch = Scratch::new();
    scratch.put("revision/a.md", b"one\n");
    let base = scratch.path().to_path_buf();
    let message = manifest::manifest_with(&base.clone(), &mut |path: &str| {
        fs::write(base.join("new.md"), b"two\n").unwrap();
        fs::rename(base.join("new.md"), base.join(path)).unwrap();
    })
    .unwrap_err()
    .to_string();
    assert!(message.contains("revision/a.md"), "{}", message);
}

#[test]
fn test_WB_REV_24_a_file_that_is_not_changed_is_hashed() {
    let scratch = Scratch::new();
    scratch.put("revision/a.md", b"one\n");
    let base = scratch.path().to_path_buf();
    let mut seen = Vec::new();
    let text = manifest::manifest_with(&base, &mut |path: &str| seen.push(path.to_string())).unwrap();
    assert_eq!(seen, ["revision/a.md"]);
    assert_eq!(text, format!("{}  revision/a.md\n", sha256_hex(b"one\n")));
}

#[test]
fn test_WB_REV_25_built_in_a_checkout_the_folder_is_share_watchbill_of_the_checkout() {
    // The build fixes it: the checkout's own `share/watchbill`, unless the spec gives `WB_SHARE`.
    assert_eq!(Path::new(manifest::PACKAGE_FOLDER), package());
}

#[test]
#[should_panic(expected = "the spec excludes a file of the revision")]
fn test_WB_REV_22_a_spec_that_excludes_a_file_of_the_revision_is_refused() {
    patterns_of("Name: x\n%files\n%{_datadir}/watchbill/revision/roles/*.md\n%exclude %{_datadir}/watchbill/revision/roles/fix.md\n");
}

#[test]
fn test_WB_REV_22_the_spec_names_the_files_of_the_revision_and_excludes_none() {
    let text = String::from_utf8(read_bytes(root().join("packaging/watchbill.spec"))).unwrap();
    assert_eq!(patterns_of(&text), patterns());
    assert!(!text.lines().any(|line| line.trim().starts_with("%exclude")));
}

// ---- The comments of the revision (#38)

fn revision_files(folder: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            revision_files(&path, found);
        } else {
            found.push(path);
        }
    }
}

/// The words of a text that name a file: they end in one of the extensions the repository uses.
/// What stands round a word (quotes, a comma, a full stop, a star) is not part of it.
fn named_files(text: &str) -> Vec<String> {
    const EXTENSIONS: [&str; 9] = ["py", "rs", "md", "yml", "yaml", "toml", "sh", "txt", "json"];
    text.split(|c: char| !(c.is_ascii_alphanumeric() || "_./-".contains(c)))
        .map(|word| word.trim_matches(|c| c == '.' || c == '/'))
        .filter(|word| match word.rsplit_once('.') {
            Some((stem, extension)) => !stem.is_empty() && EXTENSIONS.contains(&extension),
            None => false,
        })
        .map(|word| word.to_string())
        .collect()
}

/// What a text names that is neither at the root of the repository nor under `revision/`.
fn named_but_missing(text: &str, root: &Path, revision: &Path) -> Vec<String> {
    named_files(text).into_iter().filter(|name| !root.join(name).is_file() && !revision.join(name).is_file()).collect()
}

#[test]
fn test_WB_REV_26_a_name_that_is_not_in_the_repository_is_caught() {
    let scratch = Scratch::new();
    scratch.put("src/rules.rs", b"");
    scratch.put("revision/roles/fix.md", b"");
    let text = "# Read by watchbill/rules.py, or src/rules.rs, \"roles/fix.md\", roles/gone.md.\nversion 0.0.1, e.g. this.\n";
    assert_eq!(
        named_but_missing(text, scratch.path(), &scratch.path().join("revision")),
        vec!["watchbill/rules.py".to_string(), "roles/gone.md".to_string()]
    );
}

#[test]
fn test_WB_REV_26_a_text_that_names_only_files_that_are_there_is_not_caught() {
    let scratch = Scratch::new();
    scratch.put("src/rules.rs", b"");
    let text = "Read by src/rules.rs.\n";
    assert_eq!(named_but_missing(text, scratch.path(), &scratch.path().join("revision")), Vec::<String>::new());
    assert_eq!(named_files(text), vec!["src/rules.rs".to_string()]);
}

#[test]
fn test_WB_REV_26_no_file_of_the_revision_names_a_file_that_is_not_in_the_repository() {
    let mut found = Vec::new();
    revision_files(&package().join("revision"), &mut found);
    assert!(!found.is_empty());
    let mut naming = Vec::new();
    for path in &found {
        let text = String::from_utf8_lossy(&read_bytes(path)).into_owned();
        for name in named_but_missing(&text, &root(), &package().join("revision")) {
            naming.push(format!("{}: {}", path.display(), name));
        }
    }
    assert_eq!(naming, Vec::<String>::new());
    // The files that are named are found at all, so the check looks at something.
    let rules = String::from_utf8_lossy(&read_bytes(package().join("revision/rules.yml"))).into_owned();
    assert!(named_files(&rules).contains(&"src/rules.rs".to_string()));
}
