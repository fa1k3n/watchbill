//! The one reader of a file that a session can write (#261).

use std::fs;
use std::path::PathBuf;
use watchbill::bounded::{read_plain, Fault};

fn place(name: &str) -> PathBuf {
    let base = std::env::var_os("TMPDIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    let folder = base.join(format!("wb-bounded-{}-{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&folder);
    fs::create_dir_all(&folder).unwrap();
    folder
}

#[test]
fn test_WB_BUILD_101_a_plain_file_within_the_cap_is_read_whole() {
    let folder = place("plain");
    fs::write(folder.join("f"), b"abc\n").unwrap();
    assert_eq!(read_plain(&folder.join("f"), 4).unwrap(), b"abc\n");
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn test_WB_BUILD_101_a_file_over_the_cap_is_refused_and_a_file_at_it_is_not() {
    let folder = place("cap");
    fs::write(folder.join("f"), b"abcde").unwrap();
    assert!(matches!(read_plain(&folder.join("f"), 4), Err(Fault::TooBig)));
    assert_eq!(read_plain(&folder.join("f"), 5).unwrap(), b"abcde");
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn test_WB_BUILD_101_a_file_that_is_not_there_is_not_found_and_other_things_are_not_plain() {
    let folder = place("kinds");
    assert!(matches!(read_plain(&folder.join("none"), 4), Err(Fault::NotFound)));
    assert!(matches!(read_plain(&folder, 4), Err(Fault::NotPlain)));
    std::os::unix::fs::symlink("/dev/zero", folder.join("zero")).unwrap();
    assert!(matches!(read_plain(&folder.join("zero"), 4), Err(Fault::NotPlain)));
    let made = std::process::Command::new("mkfifo").arg(folder.join("pipe")).status().unwrap();
    assert!(made.success());
    assert!(matches!(read_plain(&folder.join("pipe"), 4), Err(Fault::NotPlain)));
    fs::remove_dir_all(&folder).unwrap();
}
