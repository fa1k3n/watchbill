//! `wb upgrade` (#174): pins a repository to the installed revision, and says what it will write first. The
//! repositories are made for the test, set up by `wb init --yes` and then put back to an older stand-in revision.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const GITHUB: &str = "https://github.com/acme/brake-controller.git";
const GITLAB: &str = "https://gitlab.example.com/brakes/controller.git";

fn package() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("share/watchbill")
}

/// What git prints, as text.
fn git_text(folder: &Path, args: &[&str]) -> String {
    let run = Command::new("git")
        .arg("-C")
        .arg(folder)
        .args(args)
        .env_clear()
        .envs(env())
        .stderr(Stdio::null())
        .output()
        .expect("git could not be started");
    String::from_utf8_lossy(&run.stdout).to_string()
}

fn commit_all(folder: &Path, message: &str) {
    git(folder, &["add", "--all", "--", ".watchbill"]);
    git(folder, &["commit", "-q", "-m", message]);
}

/// A repository with `origin` at `address`, set up by `wb init --yes` and committed, with a git identity.
fn set_up(address: &str) -> Folder {
    let folder = repository(Some(address));
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    commit_all(&folder.path, "Watchbill");
    folder
}

/// The value of the line `key:` of a text of `watchbill.yml`, as it is written.
fn line_of<'a>(yml: &'a str, key: &str) -> Option<&'a str> {
    yml.lines().find_map(|line| line.strip_prefix(&format!("{}: ", key)))
}

/// Writes the file with each line `key: …` given replaced, or removed where the value is none.
fn with_lines(yml: &str, lines: &[(&str, Option<&str>)]) -> String {
    let mut out = String::new();
    for line in yml.lines() {
        let found = lines.iter().find(|(key, _)| line.starts_with(&format!("{}: ", key)));
        match found {
            Some((key, Some(value))) => out.push_str(&format!("{}: {}\n", key, value)),
            Some((_, None)) => {}
            None => out.push_str(&format!("{}\n", line)),
        }
    }
    out
}

/// The older stand-in revision: one file changed, one missing, one the installed revision has not, in a
/// folder of its own; the manifest made for those files; the version 0.0.1 and the hash of that manifest.
/// The lines given are then set or removed, and all of it is committed. The file as `wb init` wrote it is
/// given back.
fn make_older(folder: &Path, lines: &[(&str, Option<&str>)]) -> String {
    let fresh = read(&config(folder));
    let base = folder.join(".watchbill");
    let mut models = read(&base.join("revision/models.yml"));
    models.push_str("# an older text\n");
    fs::write(base.join("revision/models.yml"), models).unwrap();
    fs::remove_file(base.join("revision/roles/fix.md")).unwrap();
    put(&base, "revision/old/notes.md", "An old note.\n");
    let manifest = watchbill::manifest::manifest(&base).unwrap();
    fs::write(base.join("revision.sha256"), &manifest).unwrap();
    let hash = watchbill::manifest::hash_of(&manifest);
    let pin = format!("'{}'", hash);
    let mut all: Vec<(&str, Option<&str>)> = vec![("watchbill", Some("0.0.1")), ("revision", Some(&pin))];
    all.extend_from_slice(lines);
    fs::write(config(folder), with_lines(&fresh, &all)).unwrap();
    commit_all(folder, "Watchbill 0.0.1");
    fresh
}

/// The usual older repository: GitHub, `draft_change` missing and `branch` of another value.
fn older() -> (Folder, String) {
    let folder = set_up(GITHUB);
    let fresh = make_older(&folder.path, &[("draft_change", None), ("branch", Some("'work-{issue}'"))]);
    (folder, fresh)
}

/// Every file of the repository but `.git`, with its bytes.
fn snapshot(folder: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, folder: &Path, found: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(folder).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(base).unwrap().to_str().unwrap().to_string();
            if relative == ".git" {
                continue;
            }
            let meta = fs::symlink_metadata(&path).unwrap();
            if meta.is_dir() {
                found.insert(format!("{}/", relative), Vec::new());
                walk(base, &path, found);
            } else {
                found.insert(relative, fs::read(&path).unwrap_or_default());
            }
        }
    }
    let mut found = BTreeMap::new();
    walk(folder, folder, &mut found);
    found
}

/// Every plain file under a folder, as a path relative to it, sorted.
fn files_under(folder: &Path) -> Vec<String> {
    snapshot(folder).into_keys().filter(|path| !path.ends_with('/')).collect()
}

fn installed_hash(folder: &Path) -> String {
    let printed = text(&wb(folder, &["revision"], "").stdout);
    printed.lines().find_map(|line| line.strip_prefix("revision ")).unwrap().to_string()
}

fn upgrade(folder: &Path, args: &[&str]) -> Output {
    let mut all = vec!["upgrade"];
    all.extend_from_slice(args);
    wb(folder, &all, "")
}

/// The lines of the plan: those that say a write, indented by two spaces.
fn plan(run: &Output) -> Vec<String> {
    text(&run.stdout).lines().filter(|line| line.starts_with("  ") && !line.starts_with("  git ")).map(str::to_string).collect()
}

/// Whether the plan has a line of this verb and this path.
fn says(plan: &[String], verb: &str, path: &str) -> bool {
    plan.iter().any(|line| {
        let mut words = line.split_whitespace();
        words.next() == Some(verb) && words.next() == Some(path)
    })
}

#[test]
fn test_WB_UPGRADE_1_the_files_and_the_manifest_are_those_of_the_installed_revision() {
    let (folder, _) = older();

    let run = upgrade(&folder.path, &[]);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    let theirs = files_under(&package().join("revision"));
    assert!(!theirs.is_empty());
    assert_eq!(files_under(&folder.path.join(".watchbill/revision")), theirs);
    for path in &theirs {
        let mine = fs::read(folder.path.join(".watchbill/revision").join(path)).unwrap();
        assert_eq!(mine, fs::read(package().join("revision").join(path)).unwrap(), "{}", path);
    }
    assert_eq!(fs::read(folder.path.join(".watchbill/revision.sha256")).unwrap(), fs::read(package().join("revision.sha256")).unwrap());
}

#[test]
fn test_WB_UPGRADE_1_a_file_the_older_revision_had_and_the_installed_has_not_is_removed() {
    let (folder, _) = older();

    let run = upgrade(&folder.path, &[]);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    assert!(!folder.path.join(".watchbill/revision/old/notes.md").exists());
    // Its folder holds no file of the installed revision, and goes with it.
    assert!(!folder.path.join(".watchbill/revision/old").exists());
    // The project's own text is not the revision's, and stays.
    assert!(folder.path.join(".watchbill/project/README.md").is_file());
}

#[test]
fn test_WB_UPGRADE_1_a_link_under_the_revision_is_refused_and_never_followed() {
    let folder = set_up(GITHUB);
    let elsewhere = folder_named("elsewhere");
    fs::write(elsewhere.path.join("fix.md"), "Not the revision's.\n").unwrap();
    let roles = folder.path.join(".watchbill/revision/roles");
    fs::remove_file(roles.join("fix.md")).unwrap();
    std::os::unix::fs::symlink(elsewhere.path.join("fix.md"), roles.join("fix.md")).unwrap();
    fs::write(config(&folder.path), with_lines(&read(&config(&folder.path)), &[("watchbill", Some("0.0.1"))])).unwrap();
    commit_all(&folder.path, "A link");
    let before = snapshot(&folder.path);

    let run = upgrade(&folder.path, &[]);

    assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
    assert!(text(&run.stderr).contains(".watchbill/revision/roles/fix.md"), "{}", text(&run.stderr));
    assert_eq!(snapshot(&folder.path), before);
    assert_eq!(read(&elsewhere.path.join("fix.md")), "Not the revision's.\n");
}

#[test]
fn test_WB_UPGRADE_2_the_lines_watchbill_and_revision_are_set_to_the_installed_version_and_hash() {
    let (folder, _) = older();

    let run = upgrade(&folder.path, &[]);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    let yml = read(&config(&folder.path));
    assert_eq!(line_of(&yml, "watchbill"), Some(REVISION), "{}", yml);
    let hash = installed_hash(&folder.path);
    assert_eq!(line_of(&yml, "revision").map(|value| value.trim_matches('\'')), Some(hash.as_str()), "{}", yml);
    assert_eq!(yml.lines().filter(|line| line.starts_with("watchbill:") || line.starts_with("revision:")).count(), 2, "{}", yml);
}

#[test]
fn test_WB_UPGRADE_3_a_pattern_the_file_lacks_is_added_as_wb_init_writes_it() {
    let (folder, fresh) = older();
    assert!(line_of(&read(&config(&folder.path)), "draft_change").is_none());

    let run = upgrade(&folder.path, &[]);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    let yml = read(&config(&folder.path));
    assert_eq!(line_of(&yml, "draft_change"), line_of(&fresh, "draft_change"), "{}", yml);
    assert!(line_of(&yml, "draft_change").is_some());
}

#[test]
fn test_WB_UPGRADE_3_a_gitlab_pattern_the_file_lacks_is_added_with_the_host_of_origin() {
    let folder = set_up(GITLAB);
    let fresh = make_older(&folder.path, &[("change_checks", None), ("issue_comment_bodies", None)]);

    let run = upgrade(&folder.path, &[]);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    let yml = read(&config(&folder.path));
    for key in ["change_checks", "issue_comment_bodies"] {
        assert!(line_of(&fresh, key).unwrap().contains("gitlab.example.com"), "{}", key);
        assert_eq!(line_of(&yml, key), line_of(&fresh, key), "{}", key);
    }
}

#[test]
fn test_WB_UPGRADE_4_a_pattern_the_file_has_with_another_value_is_kept_as_it_is() {
    let (folder, _) = older();

    let run = upgrade(&folder.path, &[]);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    let yml = read(&config(&folder.path));
    assert_eq!(line_of(&yml, "branch"), Some("'work-{issue}'"), "{}", yml);
    assert_eq!(yml.lines().filter(|line| line.starts_with("branch:")).count(), 1, "{}", yml);
}

#[test]
fn test_WB_UPGRADE_4_every_other_line_of_the_file_is_left_as_it_is() {
    let (folder, _) = older();
    let mut before = read(&config(&folder.path));
    before.push_str("# A line of the team's own.\nneeds: 'true'\n");
    fs::write(config(&folder.path), &before).unwrap();
    commit_all(&folder.path, "The team's lines");

    let run = upgrade(&folder.path, &[]);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    let after = read(&config(&folder.path));
    let old: Vec<&str> = before.lines().collect();
    let new: Vec<&str> = after.lines().collect();
    assert!(new.len() > old.len(), "{}", after);
    for (index, line) in old.iter().enumerate() {
        if line.starts_with("watchbill:") || line.starts_with("revision:") {
            continue;
        }
        assert_eq!(new[index], *line, "line {} of\n{}", index + 1, after);
    }
}

#[test]
fn test_WB_UPGRADE_4_no_line_classed_or_build_files_is_added() {
    let folder = repository(Some(GITHUB));
    put(&folder.path, ".github/workflows/ci.yml", "on: push\n");
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(line_of(&read(&config(&folder.path)), "build_files").is_some());
    commit_all(&folder.path, "Watchbill");
    make_older(&folder.path, &[("classed", None), ("build_files", None)]);

    let run = upgrade(&folder.path, &[]);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    let yml = read(&config(&folder.path));
    assert!(line_of(&yml, "classed").is_none(), "{}", yml);
    assert!(line_of(&yml, "build_files").is_none(), "{}", yml);
}

#[test]
fn test_WB_UPGRADE_5_each_write_is_said_before_it_is_written() {
    let (folder, _) = older();

    let run = upgrade(&folder.path, &[]);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    let said = plan(&run);
    assert!(says(&said, "add", ".watchbill/revision/roles/fix.md"), "{:?}", said);
    assert!(says(&said, "change", ".watchbill/revision/models.yml"), "{:?}", said);
    assert!(says(&said, "remove", ".watchbill/revision/old/notes.md"), "{:?}", said);
    assert!(says(&said, "change", ".watchbill/revision.sha256"), "{:?}", said);
    assert!(says(&said, "set", ".watchbill/watchbill.yml:"), "{:?}", said);
    assert!(said.iter().any(|line| line.contains("set") && line.contains(&format!("watchbill: {}", REVISION))), "{:?}", said);
    assert!(said.iter().any(|line| line.contains("set") && line.contains("revision: ")), "{:?}", said);
    assert!(said.iter().any(|line| line.contains("add") && line.contains("draft_change: ")), "{:?}", said);
    // A file that is the same is not said.
    assert!(!said.iter().any(|line| line.contains("revision/rules.yml")), "{:?}", said);
    // All of it before the line that says it is written.
    let out = text(&run.stdout);
    let written = out.find("\nWritten.").unwrap_or_else(|| panic!("{}", out));
    for line in &said {
        assert!(out.find(line.as_str()).unwrap() < written, "{}", line);
    }
}

#[test]
fn test_WB_UPGRADE_6_dry_run_says_the_same_and_writes_nothing() {
    let (folder, _) = older();
    let (other, _) = older();
    let before = snapshot(&folder.path);

    let dry = upgrade(&folder.path, &["--dry-run"]);

    assert_eq!(code(&dry), Some(0), "{}{}", text(&dry.stdout), text(&dry.stderr));
    assert_eq!(snapshot(&folder.path), before);
    assert_eq!(git_text(&folder.path, &["status", "--porcelain", "--ignored"]), "");
    let run = upgrade(&other.path, &[]);
    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    let (dry_plan, run_plan) = (plan(&dry), plan(&run));
    assert!(!dry_plan.is_empty());
    assert_eq!(dry_plan, run_plan);
    assert!(text(&dry.stdout).contains("Nothing written"), "{}", text(&dry.stdout));
}

#[test]
fn test_WB_UPGRADE_7_at_the_installed_revision_already_it_ends_with_0_and_writes_nothing() {
    let folder = set_up(GITHUB);
    let before = snapshot(&folder.path);

    for args in [&[][..], &["--dry-run"][..]] {
        let run = upgrade(&folder.path, args);

        assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
        assert!(text(&run.stdout).contains("Nothing to do"), "{}", text(&run.stdout));
        assert!(plan(&run).is_empty(), "{:?}", plan(&run));
        assert_eq!(snapshot(&folder.path), before);
    }
}

#[test]
fn test_WB_UPGRADE_7_a_second_upgrade_before_the_commit_has_nothing_to_do() {
    let (folder, _) = older();
    assert_eq!(code(&upgrade(&folder.path, &[])), Some(0));
    let before = snapshot(&folder.path);

    let run = upgrade(&folder.path, &[]);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    assert!(text(&run.stdout).contains("Nothing to do"), "{}", text(&run.stdout));
    assert_eq!(snapshot(&folder.path), before);
}

#[test]
fn test_WB_UPGRADE_8_a_change_under_watchbill_that_is_not_committed_is_refused_and_named() {
    type Change = fn(&Path) -> &'static str;
    let changes: [Change; 5] = [
        |folder| {
            fs::write(folder.join(".watchbill/revision/rules.yml"), "changed: true\n").unwrap();
            ".watchbill/revision/rules.yml"
        },
        |folder| {
            let mut yml = read(&config(folder));
            yml.push_str("needs: 'true'\n");
            fs::write(config(folder), yml).unwrap();
            git(folder, &["add", "--", ".watchbill/watchbill.yml"]);
            ".watchbill/watchbill.yml"
        },
        |folder| {
            put(folder, ".watchbill/project/all.md", "Our words.\n");
            ".watchbill/project/all.md"
        },
        |folder| {
            fs::write(folder.join(".git/info/exclude"), "*.orig\n").unwrap();
            put(folder, ".watchbill/revision/rules.yml.orig", "kept aside\n");
            ".watchbill/revision/rules.yml.orig"
        },
        |folder| {
            fs::remove_file(folder.join(".watchbill/revision.sha256")).unwrap();
            ".watchbill/revision.sha256"
        },
    ];
    for change in changes {
        let (folder, _) = older();
        let named = change(&folder.path);
        let before = snapshot(&folder.path);
        let status = git_text(&folder.path, &["status", "--porcelain"]);

        for args in [&[][..], &["--dry-run"][..]] {
            let run = upgrade(&folder.path, args);

            assert_eq!(code(&run), Some(1), "{}: {}{}", named, text(&run.stdout), text(&run.stderr));
            assert!(text(&run.stderr).contains(named), "{}: {}", named, text(&run.stderr));
            assert!(plan(&run).is_empty(), "{}: {:?}", named, plan(&run));
            assert_eq!(snapshot(&folder.path), before, "{}", named);
            assert_eq!(git_text(&folder.path, &["status", "--porcelain"]), status, "{}", named);
        }
    }
}

#[test]
fn test_WB_UPGRADE_9_an_installed_version_older_than_the_pinned_one_is_refused() {
    let folder = set_up(GITHUB);
    fs::write(config(&folder.path), with_lines(&read(&config(&folder.path)), &[("watchbill", Some("99.0.0"))])).unwrap();
    commit_all(&folder.path, "A newer pin");
    let before = snapshot(&folder.path);

    for args in [&[][..], &["--dry-run"][..]] {
        let run = upgrade(&folder.path, args);

        assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
        let err = text(&run.stderr);
        assert!(err.contains("99.0.0") && err.contains(REVISION), "{}", err);
        assert!(plan(&run).is_empty(), "{:?}", plan(&run));
        assert_eq!(snapshot(&folder.path), before);
    }
}

#[test]
fn test_WB_UPGRADE_9_versions_compare_as_numbers_and_not_as_text() {
    use watchbill::upgrade::compare;
    assert_eq!(compare("0.10.0", "0.0.9"), Some(Ordering::Greater));
    assert_eq!(compare("0.0.10", "0.0.9"), Some(Ordering::Greater));
    assert_eq!(compare("0.9.0", "0.10.0"), Some(Ordering::Less));
    assert_eq!(compare("0.12.0", "0.11.0"), Some(Ordering::Greater));
    assert_eq!(compare("1.0.0", "0.12.0"), Some(Ordering::Greater));
    assert_eq!(compare("0.0.9", "0.0.9"), Some(Ordering::Equal));
    assert_eq!(compare("0.0.9", "0.0.10"), Some(Ordering::Less));
    for odd in ["next", "", "0.0", "0..9", "0.0.9-rc1", "v0.0.9", "0.0.9 "] {
        assert_eq!(compare("0.0.9", odd), None, "{:?}", odd);
    }
}

#[test]
fn test_WB_UPGRADE_9_a_pinned_version_that_cannot_be_compared_is_refused_by_name() {
    let folder = set_up(GITHUB);
    fs::write(config(&folder.path), with_lines(&read(&config(&folder.path)), &[("watchbill", Some("next"))])).unwrap();
    commit_all(&folder.path, "A pin that is no version");
    let before = snapshot(&folder.path);

    let run = upgrade(&folder.path, &[]);

    assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
    assert!(text(&run.stderr).contains("next"), "{}", text(&run.stderr));
    assert_eq!(snapshot(&folder.path), before);
}

#[test]
fn test_WB_UPGRADE_10_the_check_of_wb_check_runs_after_the_writes_and_is_said() {
    let (folder, _) = older();

    let run = upgrade(&folder.path, &[]);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    let out = text(&run.stdout);
    let sentence = "The repository is at the revision it is pinned to, and the installed wb is that revision.";
    let at = out.find(sentence).unwrap_or_else(|| panic!("{}", out));
    assert!(out.find("\nWritten.").unwrap() < at, "{}", out);
    assert_eq!(code(&wb(&folder.path, &["check"], "")), Some(0));
}

#[test]
fn test_WB_UPGRADE_10_a_check_that_does_not_hold_after_the_writes_is_said_with_1() {
    let (folder, _) = older();
    put(&folder.path, ".watchbill/project/all.md", "Use {task}.\n");
    commit_all(&folder.path, "The project's text");

    let run = upgrade(&folder.path, &[]);

    assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains("{task}"), "{}", out);
    // The writes stay.
    assert_eq!(line_of(&read(&config(&folder.path)), "watchbill"), Some(REVISION));
}

#[test]
fn test_WB_UPGRADE_11_it_commits_nothing_and_says_the_command_that_commits_the_change() {
    let (folder, _) = older();
    let head = git_text(&folder.path, &["rev-parse", "HEAD"]);

    let run = upgrade(&folder.path, &[]);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    assert_eq!(git_text(&folder.path, &["rev-parse", "HEAD"]), head);
    assert_eq!(git_text(&folder.path, &["diff", "--cached", "--name-only"]), "");
    assert!(git_text(&folder.path, &["status", "--porcelain"]).contains(".watchbill/"));
    // The command said is one line, and commits the change and nothing else.
    let out = text(&run.stdout);
    let command = out
        .lines()
        .filter_map(|line| line.strip_prefix("  "))
        .find(|line| line.starts_with("git add"))
        .map(str::to_string)
        .unwrap_or_else(|| panic!("{}", out));
    put(&folder.path, "src/brake.c", "int brake;\n");
    let status = Command::new("/bin/sh")
        .arg("-c")
        .arg(&command)
        .current_dir(&folder.path)
        .env_clear()
        .envs(env())
        .stdout(Stdio::null())
        .status()
        .unwrap();
    assert!(status.success(), "{}", command);
    assert_ne!(git_text(&folder.path, &["rev-parse", "HEAD"]), head);
    assert_eq!(git_text(&folder.path, &["status", "--porcelain"]), "?? src/\n");
    assert_eq!(code(&wb(&folder.path, &["check"], "")), Some(0));
}

#[test]
fn test_WB_UPGRADE_12_with_no_watchbill_yml_it_ends_with_2_naming_wb_init() {
    let folder = repository(Some(GITHUB));
    for args in [&[][..], &["--dry-run"][..]] {
        let run = upgrade(&folder.path, args);

        assert_eq!(code(&run), Some(2), "{}{}", text(&run.stdout), text(&run.stderr));
        assert!(text(&run.stderr).contains("wb init"), "{}", text(&run.stderr));
        assert!(!folder.path.join(".watchbill").exists());
    }
    // Outside a repository, too.
    let plain = common::folder();
    let run = upgrade(&plain.path, &[]);
    assert_eq!(code(&run), Some(2), "{}", text(&run.stderr));
    assert_eq!(fs::read_dir(&plain.path).unwrap().count(), 0);
}

#[test]
fn test_WB_UPGRADE_12_it_runs_anywhere_inside_the_repository() {
    let (folder, _) = older();
    put(&folder.path, "src/brake.c", "int brake;\n");

    let run = upgrade(&folder.path.join("src"), &[]);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    assert_eq!(line_of(&read(&config(&folder.path)), "watchbill"), Some(REVISION));
    assert!(!folder.path.join("src/.watchbill").exists());
}

#[test]
fn test_WB_BUILD_42_wb_upgrade_is_not_held_and_says_that_the_file_is_to_be_committed() {
    let (folder, _) = older();

    let run = upgrade(&folder.path, &[]);

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    let all = format!("{}{}", text(&run.stdout), text(&run.stderr));
    assert!(!all.contains("a person reads the change"), "{}", all);
    let tail = &text(&run.stdout)[text(&run.stdout).find("\nWritten.").unwrap()..];
    assert!(tail.contains(".watchbill/watchbill.yml"), "{}", tail);
    assert!(tail.contains("commit"), "{}", tail);
    // The file now differs from HEAD, and wb upgrade is still not held by it.
    let again = upgrade(&folder.path, &[]);
    assert_eq!(code(&again), Some(0), "{}", text(&again.stderr));
    assert!(!text(&again.stderr).contains("a person reads the change"));
}

#[test]
fn test_WB_BUILD_105_a_revision_folder_over_64_mib_in_all_is_refused_by_upgrade_with_1() {
    let folder = set_up(GITHUB);
    let extra = folder.path.join(".watchbill/revision/extra");
    fs::create_dir(&extra).unwrap();
    // Each file is within the cap of one file (16 MiB); the five together are over 64 MiB.
    for number in 0..5 {
        fs::write(extra.join(format!("{}.md", number)), "x".repeat(15 * 1024 * 1024) + "\n").unwrap();
    }
    fs::write(config(&folder.path), with_lines(&read(&config(&folder.path)), &[("watchbill", Some("0.0.1"))])).unwrap();
    commit_all(&folder.path, "A big revision folder");
    let before = snapshot(&folder.path);

    let run = upgrade(&folder.path, &[]);

    assert_eq!(code(&run), Some(1), "{}{}", text(&run.stdout), text(&run.stderr));
    assert!(text(&run.stderr).contains("64 MiB"), "{}", text(&run.stderr));
    assert_eq!(snapshot(&folder.path), before);
}

#[test]
fn test_WB_UPGRADE_13_issue_text_is_added_where_it_is_missing_and_said_first() {
    for origin in [GITHUB, GITLAB] {
        let folder = set_up(origin);
        let fresh = make_older(&folder.path, &[("issue_text", None)]);
        assert!(line_of(&read(&config(&folder.path)), "issue_text").is_none());

        let run = upgrade(&folder.path, &[]);

        assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
        assert!(format!("{}{}", text(&run.stdout), text(&run.stderr)).contains("issue_text"), "{}", text(&run.stdout));
        let yml = read(&config(&folder.path));
        assert!(line_of(&fresh, "issue_text").is_some(), "{}", fresh);
        assert_eq!(line_of(&yml, "issue_text"), line_of(&fresh, "issue_text"), "{}", yml);
    }
}
