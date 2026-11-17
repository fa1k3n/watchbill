//! The package: what the spec of the RPM asks for and installs, and that no Python is left.

#![allow(non_snake_case)]
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn spec() -> String {
    fs::read_to_string(root().join("packaging/watchbill.spec")).expect("the spec could not be read")
}

/// The files of the tree, without the folders that are build output or git's own.
fn files_below(folder: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(folder).expect("a folder could not be listed") {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        if path.is_dir() {
            if !["target", ".git", "build"].contains(&name.as_str()) {
                files_below(&path, found);
            }
        } else {
            found.push(path);
        }
    }
}

#[test]
fn test_WB_PKG_7_the_package_requires_git_core_and_no_python_and_no_crate() {
    let text = spec();
    let requires: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with("Requires:") || line.starts_with("Requires("))
        .collect();
    assert_eq!(requires.len(), 1, "{:?}", requires);
    assert!(requires[0].split_whitespace().last() == Some("git-core"), "{}", requires[0]);
    assert!(!text.to_lowercase().contains("python"), "the spec names Python");
    assert!(!text.contains("%pyproject"), "the spec builds a Python package");
}

#[test]
fn test_WB_PKG_7_the_package_is_built_for_the_architecture_of_the_machine() {
    assert!(!spec().lines().any(|line| line.starts_with("BuildArch:")), "the spec fixes an architecture");
}

#[test]
fn test_WB_PKG_7_the_package_installs_wb_and_watchbill() {
    let text = spec();
    let install = text.split("%install").nth(1).unwrap().split("%check").next().unwrap();
    assert!(install.contains("%{buildroot}%{_bindir}/wb"));
    assert!(install.contains("%{buildroot}%{_bindir}/watchbill"));
    assert!(text.contains("%{_bindir}/wb\n"));
    assert!(text.contains("%{_bindir}/watchbill\n"));
}

#[test]
fn test_WB_PKG_7_no_file_of_the_repository_is_python() {
    let mut found = Vec::new();
    files_below(&root(), &mut found);
    let python: Vec<_> = found.iter().filter(|path| path.extension().map_or(false, |e| e == "py")).collect();
    assert!(python.is_empty(), "{:?}", python);
    for name in ["setup.cfg", "pyproject.toml"] {
        assert!(!root().join(name).exists(), "{}", name);
    }
}

#[test]
fn test_WB_REV_25_the_spec_fixes_the_folder_of_the_revision_when_wb_is_built() {
    let text = spec();
    assert!(text.contains("WB_SHARE=%{_datadir}/watchbill"));
    // Only the build has it: the tests of %check read the checkout.
    let build = text.split("%build").nth(1).unwrap().split("%install").next().unwrap();
    assert!(build.contains("WB_SHARE="));
    let check = text.split("%check").nth(1).unwrap().split("%files").next().unwrap();
    assert!(!check.contains("WB_SHARE"));
}

#[test]
fn test_WB_PKG_9_the_version_of_the_package_is_taken_from_cargo_toml_and_from_nowhere_else() {
    let script = fs::read_to_string(root().join("packaging/rpm.sh")).expect("rpm.sh could not be read");
    assert!(script.contains("Cargo.toml"));
    assert!(!script.contains("__init__"));
    assert!(!script.contains("__version__"));
}

#[test]
fn test_WB_PKG_9_the_spec_holds_no_version_of_its_own() {
    let text = spec();
    let line = text.lines().find(|line| line.starts_with("Version:")).expect("the spec has no Version");
    assert_eq!(line.split_whitespace().nth(1), Some("%{wb_version}"), "{}", line);
}

/// The value of a line such as `License:        Apache-2.0` in the spec.
fn spec_licence() -> String {
    let text = spec();
    let line = text.lines().find(|line| line.starts_with("License:")).expect("the spec has no License");
    line["License:".len()..].trim().to_string()
}

/// The value of `license = "..."` in the [package] table of Cargo.toml.
fn cargo_licence() -> String {
    let text = fs::read_to_string(root().join("Cargo.toml")).expect("Cargo.toml could not be read");
    let line = text.lines().find(|line| line.starts_with("license =")).expect("Cargo.toml has no license");
    line.split('"').nth(1).expect("the license is not quoted").to_string()
}

#[test]
fn test_WB_PKG_11_the_spec_and_cargo_toml_name_the_same_licence() {
    assert_eq!(spec_licence(), cargo_licence());
    assert_eq!(spec_licence(), "Apache-2.0");
}

#[test]
fn test_WB_PKG_11_the_licence_is_not_the_placeholder() {
    assert!(!spec_licence().contains("LicenseRef-Not-Chosen"), "{}", spec_licence());
    assert!(!cargo_licence().contains("LicenseRef-Not-Chosen"), "{}", cargo_licence());
}

#[test]
fn test_WB_PKG_12_the_spec_installs_license_and_notice_and_both_are_in_the_repository() {
    let text = spec();
    let files = text.split("\n%files").nth(1).expect("the spec has no %files");
    let line = files.lines().find(|line| line.starts_with("%license")).expect("%files has no %license");
    for name in ["LICENSE", "NOTICE"] {
        assert!(line.split_whitespace().any(|word| word == name), "{}", line);
        assert!(root().join(name).is_file(), "{} is not in the repository", name);
    }
}

/// The crates that Cargo.toml names, in every table of dependencies.
fn cargo_crates(cargo: &str) -> Vec<String> {
    let mut crates = Vec::new();
    let mut in_dependencies = false;
    for line in cargo.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            let table = line.trim_matches(|c| c == '[' || c == ']');
            in_dependencies = table.ends_with("dependencies");
            // A crate may have a table of its own: [dependencies.name].
            if let Some((head, name)) = table.rsplit_once('.') {
                if head.ends_with("dependencies") {
                    crates.push(name.trim_matches('"').to_string());
                }
            }
        } else if in_dependencies && !line.is_empty() && !line.starts_with('#') {
            // `name = ...`, or `name.workspace = true`.
            let key = line.split('=').next().unwrap().trim();
            let name = key.split('.').next().unwrap().trim_matches('"');
            crates.push(name.to_string());
        }
    }
    crates.sort();
    crates
}

/// The crates of the list in CLAUDE.md: the items under "The crates of `wb`", as `` - `name`: ``.
fn listed_crates(claude: &str) -> Vec<String> {
    let mut crates = Vec::new();
    let mut lines = claude.lines().skip_while(|line| !line.starts_with("- The crates of `wb`"));
    lines.next().expect("CLAUDE.md has no list of crates");
    for line in lines.take_while(|line| line.starts_with("  - ")) {
        let name = line["  - ".len()..].split('`').nth(1).expect("an item of the list names no crate");
        crates.push(name.to_string());
    }
    crates.sort();
    crates
}

/// The crates Cargo.toml names that the list does not, and those the list names that Cargo.toml does not.
fn crate_differences(cargo: &str, claude: &str) -> (Vec<String>, Vec<String>) {
    let (named, listed) = (cargo_crates(cargo), listed_crates(claude));
    let unlisted = named.iter().filter(|name| !listed.contains(name)).cloned().collect();
    let unused = listed.iter().filter(|name| !named.contains(name)).cloned().collect();
    (unlisted, unused)
}

#[test]
fn test_WB_PKG_13_the_crates_of_cargo_toml_are_the_crates_claude_md_lists() {
    let cargo = fs::read_to_string(root().join("Cargo.toml")).unwrap();
    let claude = fs::read_to_string(root().join("CLAUDE.md")).unwrap();
    assert_eq!(crate_differences(&cargo, &claude), (vec![], vec![]));
    assert_eq!(listed_crates(&claude), ["lexopt", "sha2", "yaml-rust2"]);
}

#[test]
fn test_WB_PKG_13_the_spec_asks_for_each_listed_crate_when_building() {
    let claude = fs::read_to_string(root().join("CLAUDE.md")).unwrap();
    let text = spec();
    for name in listed_crates(&claude) {
        let wanted = format!("crate({})", name);
        assert!(
            text.lines().any(|line| line.starts_with("BuildRequires:") && line.contains(&wanted)),
            "the spec does not ask for {}",
            wanted
        );
    }
}

const CLAUDE_LIST: &str = "Text.\n- The crates of `wb`, each with what it is for:\n  - `yaml-rust2`: reading YAML\n  - `sha2`: SHA-256\n- Something else.\n";

#[test]
fn test_WB_PKG_13_a_crate_in_cargo_toml_that_is_not_listed_is_caught() {
    let cargo = "[package]\nname = \"w\"\n\n[dependencies]\n# a comment\nyaml-rust2 = \"0.10\"\nsha2 = { version = \"0.10\" }\nhashlink.workspace = true\n\n[dev-dependencies]\nlexopt = \"0.3\"\n\n[dependencies.sha3]\nversion = \"0.10\"\n";
    assert_eq!(crate_differences(cargo, CLAUDE_LIST), (vec!["hashlink".to_string(), "lexopt".to_string(), "sha3".to_string()], vec![]));
}

#[test]
fn test_WB_PKG_13_a_listed_crate_that_is_not_in_cargo_toml_is_caught() {
    let cargo = "[package]\nversion = \"1\"\n\n[dependencies]\nyaml-rust2 = \"0.10\"\n\n[lib]\npath = \"src/lib.rs\"\n";
    assert_eq!(crate_differences(cargo, CLAUDE_LIST), (vec![], vec!["sha2".to_string()]));
}

fn rpm_script() -> String {
    fs::read_to_string(root().join("packaging/rpm.sh")).expect("rpm.sh could not be read")
}

#[test]
fn test_WB_PKG_14_rpm_sh_takes_the_date_of_the_package_from_the_commit_it_builds() {
    let text = rpm_script();
    let line = text
        .lines()
        .find(|line| line.contains("SOURCE_DATE_EPOCH=") && !line.trim_start().starts_with('#'))
        .expect("rpm.sh does not set SOURCE_DATE_EPOCH");
    assert!(line.contains("git") && line.contains("log -1 --format=%ct"), "{}", line);
    assert!(text.contains("export SOURCE_DATE_EPOCH"), "SOURCE_DATE_EPOCH is not exported");
}

#[test]
fn test_WB_PKG_14_rpm_sh_makes_the_tarball_with_the_commit_date_sorted_names_and_root_owner() {
    let text = rpm_script();
    let tar = text.split("\n  tar ").nth(1).expect("no tar command").split("\n\n").next().unwrap();
    for flag in ["--sort=name", "--mtime=\"@$SOURCE_DATE_EPOCH\"", "--owner=0", "--group=0", "--numeric-owner"] {
        assert!(tar.contains(flag), "tar is not given {}", flag);
    }
}

#[test]
fn test_WB_PKG_14_rpm_sh_outside_a_git_checkout_says_it_needs_one_and_stops_with_2() {
    let base = std::env::temp_dir().join(format!("wb-pkg-14-{}", std::process::id()));
    let tree = base.join("tree");
    fs::create_dir_all(tree.join("packaging")).unwrap();
    fs::copy(root().join("packaging/rpm.sh"), tree.join("packaging/rpm.sh")).unwrap();
    fs::copy(root().join("Cargo.toml"), tree.join("Cargo.toml")).unwrap();
    let run = std::process::Command::new("sh")
        .arg(tree.join("packaging/rpm.sh"))
        .arg(base.join("out"))
        .env("GIT_CEILING_DIRECTORIES", &base)
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&run.stderr).to_string();
    let _ = fs::remove_dir_all(&base);
    assert_eq!(run.status.code(), Some(2), "{}", err);
    assert!(err.contains("rpm.sh needs a git checkout"), "{}", err);
    assert!(!err.contains("fatal:"), "git's own error came first: {}", err);
}

#[test]
fn test_WB_PKG_14_rpm_sh_tells_rpmbuild_to_use_that_date_and_to_clamp_the_files_to_it() {
    let text = rpm_script();
    // The command, with the lines it is continued on.
    let build = text.split("\nrpmbuild").nth(1).expect("no rpmbuild line").split("\n\n").next().unwrap();
    for define in [
        "use_source_date_epoch_as_buildtime 1",
        "clamp_mtime_to_source_date_epoch 1",
        "source_date_epoch_from_changelog 0",
    ] {
        assert!(build.contains(define), "rpmbuild is not given {}", define);
    }
}
