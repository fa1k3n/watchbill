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
