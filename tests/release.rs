//! The release workflow: what `.github/workflows/release.yml` runs on, builds and uploads, and the
//! script that checks the tag. No test runs GitHub and none asks anything of the network.

#![allow(non_snake_case)]
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn workflow() -> String {
    fs::read_to_string(root().join(".github/workflows/release.yml")).expect("the workflow could not be read")
}

/// The lines of the workflow that are not comments.
fn lines() -> Vec<String> {
    workflow().lines().filter(|l| !l.trim_start().starts_with('#')).map(|l| l.to_string()).collect()
}

/// The text of one job: from its key at two spaces to the next job.
fn job(name: &str) -> String {
    let all = lines();
    let start = all.iter().position(|l| l == &format!("  {}:", name)).unwrap_or_else(|| panic!("no job {}", name));
    let mut text = Vec::new();
    for l in &all[start + 1..] {
        if l.starts_with("  ") && !l.starts_with("   ") {
            break;
        }
        text.push(l.clone());
    }
    text.join("\n")
}

fn check_tag(tag: &str) -> (i32, String) {
    let out = Command::new("sh")
        .arg(root().join("packaging/check-tag.sh"))
        .arg(tag)
        .output()
        .expect("sh could not be run");
    (out.status.code().unwrap_or(-1), String::from_utf8_lossy(&out.stderr).to_string())
}

#[test]
fn test_WB_PKG_17_the_workflow_runs_on_pushed_tags_of_three_numbers_and_on_nothing_else() {
    let text = lines().join("\n");
    let on = text.split("\njobs:").next().unwrap();
    assert!(on.contains("on:\n  push:\n    tags:\n"), "{}", on);
    assert!(on.contains("'[0-9]+.[0-9]+.[0-9]+'"), "{}", on);
    assert!(!on.contains("branches"), "{}", on);
    assert!(!on.contains("pull_request"), "{}", on);
    assert!(!on.contains("workflow_dispatch"), "{}", on);
    assert!(!on.contains("schedule"), "{}", on);
    let patterns = on.lines().filter(|l| l.trim_start().starts_with("- ")).count();
    assert_eq!(patterns, 1, "{}", on);
}

#[test]
fn test_WB_PKG_17_every_action_is_from_github_and_pinned_to_a_commit() {
    let uses: Vec<String> = lines().iter().filter(|l| l.contains("uses:")).cloned().collect();
    assert!(!uses.is_empty());
    for u in uses {
        let value = u.split("uses:").nth(1).unwrap().split('#').next().unwrap().trim().to_string();
        let (name, sha) = value.split_once('@').unwrap_or_else(|| panic!("not pinned: {}", u));
        assert!(name.starts_with("actions/"), "{}", u);
        assert!(sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()), "not a commit: {}", u);
        assert!(u.contains("# v"), "no version in a comment: {}", u);
    }
}

#[test]
fn test_WB_PKG_18_the_tag_is_checked_by_the_script_before_any_build() {
    let check = job("check");
    assert!(check.contains("packaging/check-tag.sh"), "{}", check);
    let build = job("build");
    assert!(build.contains("needs: check") || build.contains("needs: [check]"), "{}", build);
}

#[test]
fn test_WB_PKG_18_the_script_passes_for_the_current_version() {
    let (code, err) = check_tag(env!("CARGO_PKG_VERSION"));
    assert_eq!(code, 0, "{}", err);
}

#[test]
fn test_WB_PKG_18_the_script_fails_for_a_version_that_is_not_in_revisions_txt() {
    let (code, err) = check_tag("9.9.9");
    assert_eq!(code, 1, "{}", err);
    assert!(err.contains("9.9.9"), "{}", err);
}

#[test]
fn test_WB_PKG_18_the_script_fails_for_a_version_that_is_listed_but_is_not_the_version_of_cargo_toml() {
    let listed = fs::read_to_string(root().join("share/watchbill/revisions.txt")).unwrap();
    let other = listed.lines().next().unwrap().split_whitespace().next().unwrap().to_string();
    assert_ne!(other, env!("CARGO_PKG_VERSION"));
    let (code, err) = check_tag(&other);
    assert_eq!(code, 1, "{}", err);
    assert!(err.contains("Cargo.toml"), "{}", err);
}

#[test]
fn test_WB_PKG_18_the_script_fails_for_a_hash_that_is_not_the_hash_of_the_manifest() {
    let scratch = std::env::temp_dir().join(format!("wb-check-tag-{}", std::process::id()));
    let share = scratch.join("share/watchbill");
    fs::create_dir_all(&share).unwrap();
    fs::copy(root().join("Cargo.toml"), scratch.join("Cargo.toml")).unwrap();
    fs::copy(root().join("share/watchbill/revision.sha256"), share.join("revision.sha256")).unwrap();
    fs::write(share.join("revisions.txt"), format!("{}  {}\n", env!("CARGO_PKG_VERSION"), "0".repeat(64))).unwrap();
    let out = Command::new("sh")
        .arg(root().join("packaging/check-tag.sh"))
        .arg(env!("CARGO_PKG_VERSION"))
        .arg(&scratch)
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(&scratch);
    assert_eq!(out.status.code(), Some(1), "{}", String::from_utf8_lossy(&out.stderr));
    assert!(String::from_utf8_lossy(&out.stderr).contains("hash"));
}

#[test]
fn test_WB_PKG_19_the_build_runs_in_fedora_and_in_enterprise_linux_9_with_rpm_sh() {
    let build = job("build");
    assert!(build.contains("registry.fedoraproject.org/fedora:"), "{}", build);
    assert!(build.contains("docker.io/library/almalinux:9"), "{}", build);
    assert!(build.contains("packaging/rpm.sh"), "{}", build);
    assert!(build.contains("crb"), "{}", build);
    assert!(build.contains("epel-release"), "{}", build);
    assert!(build.contains("fetch-depth: 0"), "{}", build);
    assert!(build.contains("safe.directory"), "{}", build);
    let fedora = workflow().matches("registry.fedoraproject.org/fedora:").count();
    assert_eq!(fedora, 1, "the Fedora release is written in one place");
}

#[test]
fn test_WB_PKG_19_the_build_installs_the_rpm_and_stops_unless_wb_revision_gives_the_tag() {
    let build = job("build");
    assert!(build.contains("dnf install -y"), "{}", build);
    assert!(build.contains("wb revision"), "{}", build);
    assert!(build.contains("GITHUB_REF_NAME"), "{}", build);
    let install = build.find("wb revision").unwrap();
    let make = build.find("packaging/rpm.sh").unwrap();
    assert!(make < install, "wb revision is run before the build");
}

#[test]
fn test_WB_PKG_20_the_release_needs_both_builds_and_uploads_three_kinds_of_file_and_no_debug_package() {
    let release = job("release");
    assert!(release.contains("needs:") && release.contains("build"), "{}", release);
    assert!(release.contains("gh release create"), "{}", release);
    assert!(release.contains("SHA256SUMS"), "{}", release);
    assert!(release.contains("sha256sum"), "{}", release);
    assert!(release.contains(".src.rpm"), "{}", release);
    assert!(release.contains("watchbill-[0-9]"), "{}", release);
    let text = lines().join("\n");
    assert!(!text.contains("gh release upload"), "an upload would write into a release that is there");
    assert!(!text.contains("--clobber"), "something is written over");
    assert!(!text.contains("gh release edit"));
    assert!(!text.contains("gh release delete"));
    for l in release.lines().filter(|l| l.contains("gh release create") || l.contains(".rpm")) {
        assert!(!l.contains("debuginfo") && !l.contains("debugsource"), "{}", l);
    }
}

#[test]
fn test_WB_PKG_20_the_release_is_never_written_over_and_its_text_is_the_message_of_the_tag() {
    let release = job("release");
    assert!(release.contains("gh release view"), "{}", release);
    assert!(release.contains("already"), "{}", release);
    assert!(release.contains("--notes-file"), "{}", release);
    assert!(release.contains("%(contents:subject)") && release.contains("%(contents:body)"), "{}", release);
    assert!(!release.contains("%(contents)"), "a signed tag would put its signature into the notes");
    assert!(release.contains("GH_TOKEN: ${{ github.token }}"), "{}", release);
}

#[test]
fn test_WB_PKG_20_the_token_can_write_only_in_the_release_job() {
    let text = lines().join("\n");
    let top = text.split("\njobs:").next().unwrap();
    assert!(top.contains("permissions:\n  contents: read\n"), "{}", top);
    assert!(!top.contains("write"), "{}", top);
    assert_eq!(text.matches("contents: write").count(), 1, "contents: write once");
    assert!(job("release").contains("permissions:\n      contents: write"), "{}", job("release"));
    assert!(!job("check").contains("write") && !job("build").contains("write"));
    for name in ["check", "build"] {
        let j = job(name);
        let checkouts = j.matches("actions/checkout@").count();
        assert_eq!(checkouts, 1, "{}", name);
        assert!(j.contains("persist-credentials: false"), "the token stays in .git/config of {}", name);
    }
    assert!(!text.contains("permissions:\n      contents: write\n      "), "more than contents: write");
}

#[test]
fn test_WB_PKG_19_the_fedora_release_is_45_and_the_documents_are_kept_for_the_crates() {
    // Changed from the test that pinned 44 (#120): with the documents kept, 45 builds.
    let text = workflow();
    assert!(text.contains("registry.fedoraproject.org/fedora:45"), "{}", text);
    assert!(text.contains("tsflags="), "the reason for the flag is not written");
}

#[test]
fn test_WB_PKG_21_the_build_installs_its_packages_with_the_documents() {
    // The container images of Fedora set tsflags=nodocs, so dnf leaves out README.md of
    // rust-cpufeatures-devel, which the crate includes. The install keeps the documents.
    let build = job("build");
    let line = build
        .lines()
        .find(|l| l.contains("dnf install") && l.contains("rpm-build"))
        .expect("the install of the build's packages");
    assert!(line.contains("--setopt=tsflags= "), "{}", line);
}

/// A scratch checkout with the three files the script reads, the version of this package, and the
/// given line of revisions.txt (none when empty).
fn scratch(name: &str, line: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("wb-check-tag-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let share = dir.join("share/watchbill");
    fs::create_dir_all(&share).unwrap();
    fs::copy(root().join("Cargo.toml"), dir.join("Cargo.toml")).unwrap();
    fs::copy(root().join("share/watchbill/revision.sha256"), share.join("revision.sha256")).unwrap();
    fs::write(share.join("revisions.txt"), line).unwrap();
    dir
}

fn hash_of_manifest() -> String {
    let listed = fs::read_to_string(root().join("share/watchbill/revisions.txt")).unwrap();
    listed
        .lines()
        .find(|l| l.starts_with(&format!("{} ", env!("CARGO_PKG_VERSION"))))
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .to_string()
}

fn run_in(dir: &PathBuf) -> (Option<i32>, String) {
    let out = Command::new("sh")
        .arg(root().join("packaging/check-tag.sh"))
        .arg(env!("CARGO_PKG_VERSION"))
        .arg(dir)
        .output()
        .unwrap();
    (out.status.code(), String::from_utf8_lossy(&out.stderr).to_string())
}

fn git_in(dir: &PathBuf, args: &[&str]) {
    let out = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.org", "-c", "tag.gpgSign=false", "-c", "commit.gpgSign=false"])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git could not be run");
    assert!(out.status.success(), "{:?}: {}", args, String::from_utf8_lossy(&out.stderr));
}

#[test]
fn test_WB_PKG_18_the_script_fails_for_the_version_of_cargo_toml_that_has_no_line_in_revisions_txt() {
    let dir = scratch("noline", &format!("0.0.1  {}\n", "0".repeat(64)));
    let (code, err) = run_in(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(code, Some(1), "{}", err);
    assert!(err.contains("revisions.txt has no line"), "{}", err);
}

#[test]
fn test_WB_PKG_18_the_script_refuses_a_tag_that_is_not_annotated() {
    let dir = scratch("light", &format!("{}  {}\n", env!("CARGO_PKG_VERSION"), hash_of_manifest()));
    git_in(&dir, &["init", "-q"]);
    git_in(&dir, &["add", "."]);
    git_in(&dir, &["commit", "-q", "-m", "x"]);
    git_in(&dir, &["tag", env!("CARGO_PKG_VERSION")]);
    let (code, err) = run_in(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(code, Some(1), "{}", err);
    assert!(err.contains("not annotated"), "{}", err);
}

#[test]
fn test_WB_PKG_18_the_script_refuses_an_annotated_tag_whose_message_lacks_the_hash() {
    let dir = scratch("nomsg", &format!("{}  {}\n", env!("CARGO_PKG_VERSION"), hash_of_manifest()));
    git_in(&dir, &["init", "-q"]);
    git_in(&dir, &["add", "."]);
    git_in(&dir, &["commit", "-q", "-m", "x"]);
    git_in(&dir, &["tag", "-a", env!("CARGO_PKG_VERSION"), "-m", "a release"]);
    let (code, err) = run_in(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(code, Some(1), "{}", err);
    assert!(err.contains("message of the tag"), "{}", err);
}

#[test]
fn test_WB_PKG_18_the_script_passes_for_an_annotated_tag_whose_message_holds_the_hash() {
    let dir = scratch("good", &format!("{}  {}\n", env!("CARGO_PKG_VERSION"), hash_of_manifest()));
    git_in(&dir, &["init", "-q"]);
    git_in(&dir, &["add", "."]);
    git_in(&dir, &["commit", "-q", "-m", "x"]);
    let message = format!("{}\n\nrevision {}", env!("CARGO_PKG_VERSION"), hash_of_manifest());
    git_in(&dir, &["tag", "-a", env!("CARGO_PKG_VERSION"), "-m", &message]);
    let (code, err) = run_in(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(code, Some(0), "{}", err);
}
