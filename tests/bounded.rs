//! The one reader of a file that a session can write (#261).

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
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
    assert!(matches!(read_plain(&folder.join("f"), 4), Err(Fault::TooBig(_))));
    assert_eq!(read_plain(&folder.join("f"), 5).unwrap(), b"abcde");
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn test_WB_BUILD_101_the_refusal_for_size_names_the_limit() {
    let folder = place("limit");
    fs::write(folder.join("f"), vec![b'x'; 1024 * 1024 + 1]).unwrap();
    let fault = read_plain(&folder.join("f"), watchbill::bounded::TEXT_CAP).unwrap_err();
    assert!(fault.to_string().contains("over 1 MiB"), "{}", fault);
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn test_WB_BUILD_101_a_file_that_is_not_there_is_not_found_and_a_folder_is_not_plain() {
    let folder = place("kinds");
    assert!(matches!(read_plain(&folder.join("none"), 4), Err(Fault::NotFound)));
    assert!(matches!(read_plain(&folder, 4), Err(Fault::NotPlain)));
    fs::remove_dir_all(&folder).unwrap();
}

/// The cases that read without end or wait for ever against a reader that is wrong. They run only in the child
/// that `test_WB_BUILD_101_a_link_to_dev_zero_and_a_fifo_are_not_plain` starts, under a cap and a timeout.
#[test]
fn test_WB_BUILD_101_child_a_link_to_dev_zero_and_a_fifo_are_not_plain() {
    if std::env::var_os("WB_BOUNDED_CHILD").is_none() {
        return;
    }
    let folder = place("hostile");
    std::os::unix::fs::symlink("/dev/zero", folder.join("zero")).unwrap();
    assert!(matches!(read_plain(&folder.join("zero"), 4), Err(Fault::NotPlain)));
    let made = Command::new("mkfifo").arg(folder.join("pipe")).status().unwrap();
    assert!(made.success());
    assert!(matches!(read_plain(&folder.join("pipe"), 4), Err(Fault::NotPlain)));
    fs::remove_dir_all(&folder).unwrap();
}

#[test]
fn test_WB_BUILD_101_a_link_to_dev_zero_and_a_fifo_are_not_plain() {
    // The test binary runs again with the one case, under a cap of 1 GiB and a time of 10 seconds: a reader that
    // follows the link fails on the cap, one that opens the FIFO and waits fails on the time.
    let mut child = Command::new("prlimit")
        .arg("--as=1073741824")
        .arg("--")
        .arg(std::env::current_exe().unwrap())
        .args(["--exact", "test_WB_BUILD_101_child_a_link_to_dev_zero_and_a_fifo_are_not_plain", "--test-threads=1"])
        .env("WB_BOUNDED_CHILD", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the test binary could not be started again");
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "the child ended with {}", status);
            return;
        }
        if start.elapsed() > Duration::from_secs(10) {
            // Only the child this test started is ended.
            let _ = child.kill();
            let _ = child.wait();
            panic!("the reader did not end within 10 seconds on a link to /dev/zero or a FIFO");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
