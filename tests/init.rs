//! `wb init`, run as the command a person types, in repositories made for the test.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

/// The hash of the revision: the SHA-256 of the manifest the package holds.
fn package_hash() -> String {
    let manifest = fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/share/watchbill/revision.sha256")).unwrap();
    Sha256::digest(&manifest).iter().map(|byte| format!("{:02x}", byte)).collect()
}

fn expected(rows: &[&str]) -> String {
    let mut all: Vec<String> = HEAD.iter().map(|line| line.to_string()).collect();
    all.push(format!("watchbill: {}", REVISION));
    // The hash is written as `wb init` writes it: in quotes when it starts with a digit.
    all.push(format!("revision: {}", watchbill::init::yaml_value(&package_hash())));
    for row in rows {
        all.push(row.to_string());
        // The classed paths follow the CI driver (#84): none, as `wb init` proposes.
        if row.starts_with("ci: ") {
            all.push("classed: []".to_string());
        }
    }
    // The pattern of the branch is proposed for every server (#72), after the rows given.
    all.push("branch: 'issue-{issue}'".to_string());
    all.push(String::new());
    all.join("\n")
}

fn real(path: &std::path::Path) -> String {
    fs::canonicalize(path).unwrap().to_str().unwrap().to_string()
}

#[test]
fn test_WB_INIT_14_yes_writes_what_the_repository_says_and_asks_nothing() {
    let folder = repository(Some("git@github.com:example/brake-controller.git"));
    put(&folder.path, "requirements/brake.md", "");
    put(&folder.path, ".github/workflows/ci.yml", "");

    let run = wb(&folder.path, &["init", "--yes"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(
        read(&config(&folder.path)),
        expected(&[
            "project: brake-controller",
            "git_server: github",
            "repository: example/brake-controller",
            "tracker: github",
            "requirements: requirements/*.md",
            "ci: github-actions",
            // The CI file that was found is a file of the build (#125), after `classed`.
            "build_files: ['.github/workflows']",
            // The patterns of the starter (#84), before the addresses.
            "issue_comment: 'gh issue comment {issue} --repo example/brake-controller --body-file -'",
            "issue_comments: 'gh api repos/example/brake-controller/issues/{issue}/comments --paginate --jq ''.[].body | split(\"\\n\")[0]'''",
            "change_comment: 'gh pr comment {change} --repo example/brake-controller --body-file -'",
            "change_comments: 'gh api repos/example/brake-controller/issues/{change}/comments --paginate --jq ''.[].body | split(\"\\n\")[0]'''",
            "change_of_branch: 'gh pr list --repo example/brake-controller --head {branch} --state open --json number --jq ''.[].number'''",
            r#"open_issues: 'gh issue list --repo example/brake-controller --state open --limit 1000 --json number,title --jq ''.[] | "\(.number)\t\(.title | gsub("[\n\r]"; " "))"'''"#,
            "issue_address: 'gh issue view {issue} --repo example/brake-controller; gh issue view {issue} --repo example/brake-controller --comments'",
            "change_address: 'gh pr view {change} --repo example/brake-controller; gh pr view {change} --repo example/brake-controller --comments'",
        ])
    );
    assert!(!text(&run.stdout).contains("(brake-controller)"), "no question is put");
}

#[test]
fn test_WB_INIT_7_a_repository_with_nothing_to_find_proposes_none() {
    let folder = repository_named("My Project", None);

    let run = wb(&folder.path, &["init", "-y"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(
        read(&config(&folder.path)),
        expected(&[
            "project: my-project",
            "git_server: none",
            "repository: none",
            "tracker: none",
            "requirements: none",
            "ci: none",
        ])
    );
}

#[test]
fn test_WB_INIT_9_an_answer_replaces_what_was_found_and_an_empty_line_keeps_it() {
    let folder = repository(Some("https://gitlab.example.com/brakes/controller.git"));
    put(&folder.path, ".gitlab-ci.yml", "");
    put(&folder.path, "Jenkinsfile", "");

    // project, git server, repository, tracker, requirements, ci, then the question whether it is OK.
    let run = wb(&folder.path, &["init"], &lines(&["brakes", "gerrit", "", "jira", "polarion", "jenkins", "", "yes"]));

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(
        read(&config(&folder.path)),
        expected(&[
            "project: brakes",
            "git_server: gerrit",
            "repository: brakes/controller",
            "tracker: jira",
            "requirements: polarion",
            "ci: jenkins",
            // Each CI file that was found (#125), whatever the answer for the driver.
            "build_files: ['.gitlab-ci.yml', Jenkinsfile]",
        ])
    );
    let out = text(&run.stdout);
    assert!(out.contains("project: (brake-controller) "));
    assert!(out.contains("ci driver (github-actions, gitlab-ci, jenkins or none): (gitlab-ci) "));
    assert!(out.contains("Is this OK? (yes) "));
}

#[test]
fn test_WB_INIT_10_with_no_git_server_the_repository_is_not_asked_for() {
    let folder = repository(Some("git@github.com:example/brake-controller.git"));

    let run = wb(&folder.path, &["init"], &lines(&["", "none", "", "", "", "", ""]));

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(!text(&run.stdout).contains("repository at the git server"));
    assert!(read(&config(&folder.path)).contains("\ngit_server: none\nrepository: none\n"));
}

#[test]
fn test_WB_INIT_11_input_that_ends_early_keeps_what_was_found_for_the_rest() {
    let folder = repository(Some("git@github.com:example/brake-controller.git"));

    let run = wb(&folder.path, &["init"], &lines(&["brakes"]));

    // The input ends at the question whether it is OK too, so nothing is written (WB-INIT-31).
    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stdout).contains("\nproject: brakes\ngit_server: github\n"));
}

#[test]
fn test_WB_INIT_12_a_project_name_that_cannot_be_used_is_asked_for_again() {
    let folder = repository(None);

    let run = wb(&folder.path, &["init"], &lines(&["Brake Controller", "brakes", "", "", "", "", "", "yes"]));

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("Not taken: a project name is"));
    assert!(read(&config(&folder.path)).contains("\nproject: brakes\n"));
}

#[test]
fn test_WB_INIT_22_an_answer_yaml_would_read_as_something_else_is_written_in_quotes() {
    let folder = repository(None);

    // project, git server (none, so no repository), tracker, requirements, ci, OK.
    let run = wb(&folder.path, &["init"], &lines(&["", "", "true", "docs/it's here/*.md #1", "", "", ""]));

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let file = read(&config(&folder.path));
    assert!(file.contains("\ntracker: 'true'\n"));
    assert!(file.contains("\nrequirements: 'docs/it''s here/*.md #1'\n"));
    assert!(file.contains(&format!("\nwatchbill: {}\n", REVISION)));
}

#[test]
fn test_WB_INIT_16_no_at_the_last_question_writes_nothing() {
    let folder = repository(None);

    let run = wb(&folder.path, &["init"], &lines(&["", "", "", "", "", "", "no"]));

    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stdout).contains("Nothing written."));
    assert!(!folder.path.join(".watchbill").exists());
}

#[test]
fn test_WB_INIT_19_a_file_that_is_already_there_is_left_as_it_is() {
    let folder = repository(None);
    put(&folder.path, ".watchbill/watchbill.yml", "project: kept\n");

    let run = wb(&folder.path, &["init", "--yes"], "");

    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stdout).contains("is already there"));
    assert_eq!(read(&config(&folder.path)), "project: kept\n");
}

#[test]
fn test_WB_INIT_17_from_a_folder_inside_the_repository_the_file_is_written_at_its_root() {
    let folder = repository(None);
    let inner = folder.path.join("src").join("control");
    fs::create_dir_all(&inner).unwrap();

    let run = wb(&inner, &["init", "--yes"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(config(&folder.path).exists());
    assert!(!inner.join(".watchbill").exists());
}

#[test]
fn test_WB_INIT_20_outside_a_git_repository_nothing_is_written() {
    let folder = folder();

    let run = wb(&folder.path, &["init", "--yes"], "");

    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stderr).contains("not in a git repository"));
    assert!(!folder.path.join(".watchbill").exists());
}

#[test]
fn test_WB_INIT_21_a_token_in_the_address_of_origin_never_reaches_the_file_or_the_screen() {
    let folder = repository(Some("https://someone:s3cr3t-token@github.com/example/brake-controller.git"));

    let run = wb(&folder.path, &["init", "--yes"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let everything = read(&config(&folder.path)) + &text(&run.stdout) + &text(&run.stderr);
    assert!(everything.contains("\nrepository: example/brake-controller\n"));
    assert!(!everything.contains("s3cr3t"));
    assert!(!everything.contains("someone"));
}

#[test]
fn test_WB_INIT_25_an_unknown_command_and_an_unknown_flag_end_with_2_and_the_usage() {
    let folder = repository(None);

    let command = wb(&folder.path, &["start"], "");
    assert_eq!(code(&command), Some(2));
    assert!(text(&command.stderr).contains("no command 'start'"));
    assert!(text(&command.stderr).contains("usage:"));

    let flag = wb(&folder.path, &["init", "--force"], "");
    assert_eq!(code(&flag), Some(2));
    assert!(text(&flag.stderr).contains("wb init: '--force' is not known\n\nusage:"));
    assert_eq!(text(&flag.stdout), "");
    assert!(!folder.path.join(".watchbill").exists());
}

#[test]
fn test_WB_INIT_13_an_answer_over_200_characters_or_with_a_control_character_is_asked_for_again() {
    let folder = repository(None);
    let long = "a".repeat(201);
    let tracker = "t".repeat(201);

    let run = wb(
        &folder.path,
        &["init"],
        &lines(&[&long, "bra\x07kes", "brakes", "", &tracker, "tra\x07cker", "", "", "", "", "yes"]),
    );

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains("Not taken: that is over 200 characters."));
    assert!(out.contains("Not taken: that holds a character that cannot be written."));
    assert_eq!(out.matches("Not taken:").count(), 4, "the tracker question too");
    assert!(read(&config(&folder.path)).contains("\nproject: brakes\n"));
    assert!(read(&config(&folder.path)).contains("\ntracker: none\n"));
}

#[test]
fn test_WB_INIT_15_the_file_is_shown_before_it_is_asked_whether_it_is_ok() {
    let folder = repository(None);

    let run = wb(&folder.path, &["init"], &lines(&["", "", "", "", "", "", "no"]));

    let out = text(&run.stdout);
    let shown = out.find(&format!("About to write {}", real(&folder.path) + "/.watchbill/watchbill.yml")).unwrap();
    let project = shown + out[shown..].find("project: brake-controller\n").unwrap();
    assert!(project < out.find("Is this OK?").unwrap());
    assert!(!config(&folder.path).exists());
}

#[test]
fn test_WB_INIT_18_the_version_of_wb_is_written_as_the_line_watchbill() {
    let folder = repository(None);

    let run = wb(&folder.path, &["init", "--yes"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(read(&config(&folder.path)).contains(&format!("\nwatchbill: {}\n", REVISION)));
}

#[test]
fn test_WB_INIT_23_the_file_that_was_written_is_named() {
    let folder = repository(None);

    let run = wb(&folder.path, &["init"], &lines(&["", "", "", "", "", "", ""]));

    let wrote = format!("Wrote {}/.watchbill/watchbill.yml.", real(&folder.path));
    assert!(text(&run.stdout).contains(&wrote));

    let other = repository(None);
    let yes = wb(&other.path, &["init", "--yes"], "");
    assert!(text(&yes.stdout).contains(&format!("Wrote {}/.watchbill/watchbill.yml.", real(&other.path))));
}

#[test]
fn test_WB_INIT_24_it_ends_with_0_when_the_file_is_written() {
    let folder = repository(None);

    let run = wb(&folder.path, &["init", "-y"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(config(&folder.path).exists());
}

#[test]
fn test_WB_INIT_16_y_is_a_yes() {
    let folder = repository(None);

    let run = wb(&folder.path, &["init"], &lines(&["", "", "", "", "", "", "y"]));

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(config(&folder.path).exists());
}

#[test]
fn test_WB_INIT_28_an_empty_line_keeps_the_proposal() {
    let folder = repository(Some("git@github.com:example/brake-controller.git"));

    let run = wb(&folder.path, &["init"], &lines(&["", "", "", "", "", "", "", "yes"]));

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(read(&config(&folder.path))
        .contains("\nproject: brake-controller\ngit_server: github\nrepository: example/brake-controller\n"));
}

#[test]
fn test_WB_INIT_29_the_repository_is_none_when_the_git_server_is_none() {
    let folder = repository(Some("git@github.com:example/brake-controller.git"));

    let run = wb(&folder.path, &["init"], &lines(&["", "none", "", "", "", "", "yes"]));

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(read(&config(&folder.path)).contains("\nrepository: none\n"));
}

#[test]
fn test_WB_INIT_30_yes_writes_what_it_proposes() {
    let folder = repository(Some("git@github.com:example/brake-controller.git"));

    let run = wb(&folder.path, &["init", "--yes"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(read(&config(&folder.path)).contains("\nrepository: example/brake-controller\n"));
}

#[test]
fn test_WB_INIT_31_input_that_ends_at_the_question_whether_it_is_ok_writes_nothing() {
    let folder = repository(None);

    let run = wb(&folder.path, &["init"], "");

    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stdout).contains("Nothing written."));
    assert!(!folder.path.join(".watchbill").exists());

    let late = wb(&folder.path, &["init"], &lines(&["", "", "", "", ""]));
    assert_eq!(code(&late), Some(1));
    assert!(!folder.path.join(".watchbill").exists());
}

/// Reads what the program has printed until `marker` is in it.
fn wait_for_output(child: &mut std::process::Child, marker: &str) -> String {
    let mut stdout = child.stdout.take().unwrap();
    let mut seen = Vec::new();
    let mut chunk = [0u8; 4096];
    while !String::from_utf8_lossy(&seen).contains(marker) {
        let n = stdout.read(&mut chunk).expect("standard output could not be read");
        assert!(n > 0, "wb ended before it printed {:?}: {:?}", marker, String::from_utf8_lossy(&seen));
        seen.extend_from_slice(&chunk[..n]);
    }
    // What is printed after the interrupt is read by the caller.
    child.stdout = Some(stdout);
    String::from_utf8_lossy(&seen).to_string()
}

extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
    fn signal(signal: i32, handler: usize) -> usize;
}

const SIGINT: i32 = 2;

fn interrupt(child: &std::process::Child) {
    assert_eq!(unsafe { kill(child.id() as i32, SIGINT) }, 0, "the signal could not be sent");
}

/// Ends the run of an interrupted child and gives its status, what it printed after the marker, and standard error.
fn finish(mut child: std::process::Child, before: String) -> (Option<i32>, String, String) {
    let mut out = before;
    let mut rest = String::new();
    child.stdout.take().unwrap().read_to_string(&mut rest).unwrap();
    out.push_str(&rest);
    let mut err = String::new();
    child.stderr.take().unwrap().read_to_string(&mut err).unwrap();
    let status = child.wait().unwrap();
    (status.code(), out, err)
}

fn start(folder: &std::path::Path, path: Option<&str>) -> std::process::Child {
    let mut command = Command::new(env!("CARGO_BIN_EXE_wb"));
    command
        .arg("init")
        .current_dir(folder)
        .env_clear()
        .envs(env())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = path {
        command.env("PATH", path);
    }
    command.spawn().expect("wb could not be started")
}

fn interrupted_while_it_asks() -> (Option<i32>, String, String, Folder) {
    let folder = repository(None);
    let mut child = start(&folder.path, None);
    let before = wait_for_output(&mut child, "project: (brake-controller) ");
    interrupt(&child);
    let (status, out, err) = finish(child, before);
    (status, out, err, folder)
}

/// A `git` that waits, so that the interrupt comes while `wb` asks it. Gives the folder it is in and the `PATH`.
fn slow_git() -> (Folder, String) {
    let bin = folder_named("bin");
    let script = bin.path.join("git");
    fs::write(&script, "#!/bin/sh\n: > \"$(dirname \"$0\")/started\"\nexec sleep 5\n").unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("{}:{}", bin.path.display(), std::env::var("PATH").unwrap_or_default());
    (bin, path)
}

fn interrupted_while_it_asks_git() -> (Option<i32>, String, String, Folder) {
    let folder = repository(None);
    let (bin, path) = slow_git();
    let child = start(&folder.path, Some(&path));
    let marker = bin.path.join("started");
    let deadline = Instant::now() + Duration::from_secs(20);
    while !marker.exists() {
        assert!(Instant::now() < deadline, "git was never asked");
        std::thread::sleep(Duration::from_millis(20));
    }
    interrupt(&child);
    let (status, out, err) = finish(child, String::new());
    drop(bin);
    (status, out, err, folder)
}

#[test]
fn test_WB_INIT_32_an_interrupt_ends_with_1() {
    let (status, _, _, folder) = interrupted_while_it_asks();
    assert_eq!(status, Some(1), "while it asks a question");
    assert!(!folder.path.join(".watchbill").exists());

    let (status, _, _, folder) = interrupted_while_it_asks_git();
    assert_eq!(status, Some(1), "while it asks git");
    assert!(!folder.path.join(".watchbill").exists());
}

#[test]
fn test_WB_INIT_33_an_interrupt_says_nothing_written_and_shows_no_traceback() {
    let (_, out, err, _) = interrupted_while_it_asks();
    assert!(out.ends_with("\nNothing written.\n"), "while it asks a question: {:?}", out);
    assert_eq!(err, "", "while it asks a question");

    let (_, out, err, _) = interrupted_while_it_asks_git();
    assert_eq!(out, "\nNothing written.\n", "while it asks git");
    assert_eq!(err, "", "while it asks git");
}

#[test]
fn test_WB_INIT_34_yes_shows_the_file_before_it_writes_it() {
    let folder = repository(None);

    let run = wb(&folder.path, &["init", "--yes"], "");

    let out = text(&run.stdout);
    let shown = out.find(&format!("About to write {}/.watchbill/watchbill.yml", real(&folder.path))).unwrap();
    let project = shown + out[shown..].find("project: brake-controller\n").unwrap();
    assert!(project < out.find("Wrote ").unwrap());
}

#[test]
fn test_WB_INIT_37_any_usable_text_is_taken_for_the_git_server_tracker_and_ci_driver() {
    let folder = repository(None);

    let run = wb(
        &folder.path,
        &["init"],
        &lines(&["", "forgejo", "", "linear", "reqs/*.md", "woodpecker", "", "yes"]),
    );

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let file = read(&config(&folder.path));
    for line in ["git_server: forgejo", "tracker: linear", "ci: woodpecker"] {
        assert!(file.contains(&format!("\n{}\n", line)), "{}", line);
    }
}

#[test]
fn test_WB_INIT_38_an_answer_that_is_not_utf_8_is_refused_in_words() {
    let folder = repository(None);

    let run = wb_bytes(&folder.path, &["init"], b"bra\xffkes\n");

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stderr), "wb init: what was typed is not UTF-8 text. Nothing written.\n");
    assert!(!text(&run.stdout).contains("About to write"));
    assert!(!folder.path.join(".watchbill").exists());

    // At the question whether it is OK too, and with an unterminated last line.
    let late = wb_bytes(&folder.path, &["init"], b"\n\n\n\n\n\xc3");
    assert_eq!(code(&late), Some(1));
    assert!(text(&late.stderr).contains("not UTF-8"));
    assert!(!text(&late.stderr).contains("panicked"));
    assert!(!folder.path.join(".watchbill").exists());
}

#[test]
fn test_WB_INIT_38_a_path_that_is_not_utf_8_is_refused_in_words() {
    let name = std::ffi::OsStr::from_bytes(b"br\xffake");
    let folder = repository_named(name, None);

    let run = wb(&folder.path, &["init", "--yes"], "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stderr), "wb init: the path of the repository is not UTF-8 text. Nothing written.\n");
    assert!(!folder.path.join(".watchbill").exists());
}

#[test]
fn test_WB_INIT_32_an_interrupt_that_came_in_ignored_stays_ignored() {
    use std::os::unix::process::CommandExt;
    let folder = repository(None);
    let mut command = Command::new(env!("CARGO_BIN_EXE_wb"));
    command
        .arg("init")
        .current_dir(&folder.path)
        .env_clear()
        .envs(env())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // As under `nohup`: the parent has set SIGINT to be ignored. Python then keeps ignoring it.
    unsafe {
        command.pre_exec(|| {
            signal(SIGINT, 1);
            Ok(())
        });
    }
    let mut child = command.spawn().expect("wb could not be started");
    let before = wait_for_output(&mut child, "project: (brake-controller) ");
    interrupt(&child);
    std::thread::sleep(Duration::from_millis(500));
    assert!(child.try_wait().unwrap().is_none(), "wb ended on an interrupt that is ignored");
    use std::io::Write;
    child.stdin.take().unwrap().write_all(lines(&["", "", "", "", "", "", "yes"]).as_bytes()).unwrap();
    let (status, out, err) = finish(child, before);
    assert_eq!(status, Some(0), "{}", err);
    assert!(!out.contains("Nothing written."));
    assert!(config(&folder.path).exists());
}

#[test]
fn test_WB_INIT_38_an_error_of_reading_is_not_called_not_utf_8() {
    let folder = repository(None);
    // Standard input is a folder: reading it fails with an error that is not about text.
    let run = Command::new(env!("CARGO_BIN_EXE_wb"))
        .arg("init")
        .current_dir(&folder.path)
        .env_clear()
        .envs(env())
        .stdin(fs::File::open("/").unwrap())
        .output()
        .expect("wb could not be started");

    assert_eq!(code(&run), Some(1));
    let err = text(&run.stderr);
    assert!(err.starts_with("wb init: cannot read standard input: "), "{}", err);
    assert!(err.ends_with(". Nothing written.\n"), "{}", err);
    assert!(!err.contains("UTF-8"), "{}", err);
    assert!(!folder.path.join(".watchbill").exists());
}

// ---- the patterns of `wb prompt` (#72) ----

#[test]
fn test_WB_INIT_43_a_github_origin_gets_the_patterns_of_the_two_addresses_and_the_branch() {
    let folder = repository(Some("https://github.com/example/brake-controller.git"));

    let run = wb(&folder.path, &["init", "--yes"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let file = read(&config(&folder.path));
    assert!(
        file.ends_with(
            "issue_address: 'gh issue view {issue} --repo example/brake-controller; gh issue view {issue} --repo example/brake-controller --comments'\n\
             change_address: 'gh pr view {change} --repo example/brake-controller; gh pr view {change} --repo example/brake-controller --comments'\n\
             branch: 'issue-{issue}'\n"
        ),
        "{}",
        file
    );
    assert!(text(&run.stdout).contains("issue_address: 'gh issue view {issue}"), "the file is shown first");
    // The file is read through the loader, and wb check holds with the new lines.
    assert_eq!(code(&wb(&folder.path, &["check"], "")), Some(0));
}

#[test]
fn test_WB_INIT_44_another_server_none_or_a_repository_that_is_not_plain_gets_no_address() {
    for origin in [
        Some("https://gitlab.example.com/brakes/it's.git"),
        Some("https://git.example.org/brakes/controller.git"),
        Some("git@github.com:example/it's.git"),
        None,
    ] {
        let folder = repository(origin);

        let run = wb(&folder.path, &["init", "--yes"], "");

        assert_eq!(code(&run), Some(0), "{:?}: {}", origin, text(&run.stderr));
        let file = read(&config(&folder.path));
        assert!(!file.contains("issue_address") && !file.contains("change_address"), "{:?}: {}", origin, file);
    }
}

#[test]
fn test_WB_INIT_45_the_branch_pattern_is_proposed_for_every_server() {
    for origin in [Some("https://git.example.org/brakes/controller.git"), None] {
        let folder = repository(origin);

        let run = wb(&folder.path, &["init", "--yes"], "");

        assert_eq!(code(&run), Some(0), "{:?}: {}", origin, text(&run.stderr));
        assert!(read(&config(&folder.path)).ends_with("ci: none\nclassed: []\nbranch: 'issue-{issue}'\n"), "{:?}", origin);
    }
}

// ---- the addresses show the text and the comments (#96) ----

#[test]
fn test_WB_INIT_46_a_github_address_runs_one_command_for_the_text_and_one_for_the_comments() {
    let folder = repository(Some("https://github.com/example/brake-controller.git"));

    let run = wb(&folder.path, &["init", "--yes"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let file = read(&config(&folder.path));
    assert!(
        file.contains(
            "issue_address: 'gh issue view {issue} --repo example/brake-controller; gh issue view {issue} --repo example/brake-controller --comments'\n"
        ),
        "{}",
        file
    );
    assert!(
        file.contains(
            "change_address: 'gh pr view {change} --repo example/brake-controller; gh pr view {change} --repo example/brake-controller --comments'\n"
        ),
        "{}",
        file
    );
    // Both commands of a line have the placeholder. That `wb prompt` fills it in both is shown by the
    // tests of `wb prompt`, which read these lines back.
    let line = |key: &str| file.lines().find(|l| l.starts_with(key)).unwrap().to_string();
    assert_eq!(line("issue_address").matches("{issue}").count(), 2, "{}", file);
    assert_eq!(line("change_address").matches("{change}").count(), 2, "{}", file);
}

// ---- the patterns of the starter (#84) ----

/// A pattern of `watchbill.yml` as the loader reads it back, or `None` when the file has none.
fn pattern(folder: &std::path::Path, key: &str) -> Option<String> {
    let document = watchbill::yamlfile::load(config(folder)).unwrap();
    let place = [watchbill::yamlfile::Part::from(key)];
    document.has(&place).then(|| document.text(&place).unwrap())
}

#[test]
fn test_WB_INIT_47_a_github_origin_gets_the_pattern_that_posts_a_comment_on_an_issue() {
    let folder = repository(Some("https://github.com/example/brake-controller.git"));
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(
        pattern(&folder.path, "issue_comment").as_deref(),
        Some("gh issue comment {issue} --repo example/brake-controller --body-file -")
    );
    assert_eq!(code(&wb(&folder.path, &["check"], "")), Some(0));
}

#[test]
fn test_WB_INIT_48_a_github_origin_gets_the_patterns_that_read_the_comments_and_find_the_change() {
    let folder = repository(Some("git@github.com:example/brake-controller.git"));
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(
        pattern(&folder.path, "issue_comments").as_deref(),
        Some("gh api repos/example/brake-controller/issues/{issue}/comments --paginate --jq '.[].body | split(\"\\n\")[0]'")
    );
    assert_eq!(
        pattern(&folder.path, "change_of_branch").as_deref(),
        Some("gh pr list --repo example/brake-controller --head {branch} --state open --json number --jq '.[].number'")
    );
}

#[test]
fn test_WB_INIT_53_a_github_origin_gets_the_patterns_that_post_and_read_comments_on_a_change() {
    let folder = repository(Some("https://github.com/example/brake-controller.git"));
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(
        pattern(&folder.path, "change_comment").as_deref(),
        Some("gh pr comment {change} --repo example/brake-controller --body-file -")
    );
    // The comments of a pull request are those of an issue at GitHub.
    assert_eq!(
        pattern(&folder.path, "change_comments").as_deref(),
        Some("gh api repos/example/brake-controller/issues/{change}/comments --paginate --jq '.[].body | split(\"\\n\")[0]'")
    );
}

#[test]
fn test_WB_INIT_47_another_server_gets_no_pattern_that_posts_on_an_issue() {
    no_pattern(&["issue_comment"]);
}

#[test]
fn test_WB_INIT_48_another_server_gets_no_pattern_that_reads_the_comments_or_finds_the_change() {
    no_pattern(&["issue_comments", "change_of_branch"]);
}

#[test]
fn test_WB_INIT_53_another_server_gets_no_pattern_for_the_comments_of_a_change() {
    no_pattern(&["change_comment", "change_comments"]);
}

// ---- the patterns for a GitLab origin (#123) ----

#[test]
fn test_WB_INIT_49_a_gitlab_origin_gets_the_glab_commands_with_the_repository_filled_in() {
    let folder = repository(Some("https://gitlab.example.com/brakes/controller.git"));
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let api = "glab api --hostname gitlab.example.com";
    let filter = "jq -r 'select(.system == false) | .body | split(\"\\n\")[0]'";
    let expected = [
        ("issue_address", "glab issue view {issue} --repo https://gitlab.example.com/brakes/controller --comments".to_string()),
        ("change_address", "glab mr view {change} --repo https://gitlab.example.com/brakes/controller --comments".to_string()),
        ("issue_comment", format!("{} --method POST projects/brakes%2Fcontroller/issues/{{issue}}/notes -F body=@-", api)),
        ("change_comment", format!("{} --method POST projects/brakes%2Fcontroller/merge_requests/{{change}}/notes -F body=@-", api)),
        ("issue_comments", format!("{} projects/brakes%2Fcontroller/issues/{{issue}}/notes --paginate --output ndjson | {}", api, filter)),
        ("change_comments", format!("{} projects/brakes%2Fcontroller/merge_requests/{{change}}/notes --paginate --output ndjson | {}", api, filter)),
        (
            "change_of_branch",
            "glab mr list --repo https://gitlab.example.com/brakes/controller --source-branch {branch} -F json --jq '.[].iid'".to_string(),
        ),
        ("branch", "issue-{issue}".to_string()),
    ];
    for (key, line) in expected {
        assert_eq!(pattern(&folder.path, key).as_deref(), Some(line.as_str()), "{}", key);
    }
    assert_eq!(code(&wb(&folder.path, &["check"], "")), Some(0));
}

#[test]
fn test_WB_INIT_49_a_gitlab_origin_in_a_group_is_encoded_and_one_that_is_not_plain_gets_none() {
    let folder = repository(Some("git@gitlab.example.com:team/brakes/controller.git"));
    assert_eq!(code(&wb(&folder.path, &["init", "--yes"], "")), Some(0));
    let line = pattern(&folder.path, "issue_comments").unwrap();
    assert!(line.contains("projects/team%2Fbrakes%2Fcontroller/issues/{issue}/notes"), "{}", line);

    let folder = repository(Some("https://gitlab.example.com/brakes/it's.git"));
    assert_eq!(code(&wb(&folder.path, &["init", "--yes"], "")), Some(0));
    for key in ["issue_address", "change_address", "issue_comment", "issue_comments", "change_comment", "change_comments", "change_of_branch"] {
        assert_eq!(pattern(&folder.path, key), None, "{}", key);
    }
}

#[test]
fn test_WB_INIT_55_a_gitlab_origin_keeps_its_host_and_drops_the_user_and_the_token() {
    let folder = repository(Some("https://oauth2:secret-token@gitlab.team.example/brakes/controller.git"));
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let file = read(&config(&folder.path));
    assert!(file.contains("--hostname gitlab.team.example "), "{}", file);
    assert!(file.contains("--repo https://gitlab.team.example/brakes/controller "), "{}", file);
    assert!(!file.contains("secret-token") && !file.contains("oauth2"), "{}", file);
}

#[test]
fn test_WB_INIT_55_a_gitlab_origin_with_a_port_gets_none_of_the_commands() {
    let folder = repository(Some("https://gitlab.example.com:8443/brakes/controller.git"));
    assert_eq!(code(&wb(&folder.path, &["init", "--yes"], "")), Some(0));
    for key in ["issue_address", "issue_comment", "issue_comments", "change_of_branch"] {
        assert_eq!(pattern(&folder.path, key), None, "{}", key);
    }
}

/// None of the patterns is written for another server, for none, or for a repository that is not plain.
fn no_pattern(keys: &[&str]) {
    for origin in [
        Some("https://gitlab.example.com/brakes/it's.git"),
        Some("https://git.example.org/brakes/controller.git"),
        Some("git@github.com:example/it's.git"),
        None,
    ] {
        let folder = repository(origin);
        let run = wb(&folder.path, &["init", "--yes"], "");
        assert_eq!(code(&run), Some(0), "{:?}: {}", origin, text(&run.stderr));
        for key in keys {
            assert_eq!(pattern(&folder.path, key), None, "{:?}: {}", origin, key);
        }
    }
}

// ---- the pattern that lists the open issues (#139) ----

#[test]
fn test_WB_INIT_56_a_github_origin_gets_open_issues_that_prints_the_number_a_tab_and_the_title() {
    let folder = repository(Some("https://github.com/example/brake-controller.git"));
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(
        pattern(&folder.path, "open_issues").as_deref(),
        Some("gh issue list --repo example/brake-controller --state open --limit 1000 --json number,title --jq '.[] | \"\\(.number)\\t\\(.title | gsub(\"[\\n\\r]\"; \" \"))\"'")
    );
    assert_eq!(code(&wb(&folder.path, &["check"], "")), Some(0));
}

#[test]
fn test_WB_INIT_56_a_gitlab_origin_gets_open_issues_with_the_host_of_origin() {
    let folder = repository(Some("https://gitlab.team.example/brakes/controller.git"));
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(
        pattern(&folder.path, "open_issues").as_deref(),
        Some("glab api --hostname gitlab.team.example 'projects/brakes%2Fcontroller/issues?state=opened&per_page=100' --paginate --output ndjson | jq -r '\"\\(.iid)\\t\\(.title | gsub(\"[\\n\\r]\"; \" \"))\"'")
    );
    assert_eq!(code(&wb(&folder.path, &["check"], "")), Some(0));
}

#[test]
fn test_WB_INIT_56_another_server_gets_no_open_issues() {
    no_pattern(&["open_issues"]);
}

/// What the jq of a pattern prints for a title with a line break and a tab, or `None` when `jq` is not here.
fn jq_prints(program: &str, json: &str) -> Option<String> {
    if !std::process::Command::new("jq").arg("--version").output().map(|run| run.status.success()).unwrap_or(false) {
        eprintln!("jq is not installed: not checked");
        return None;
    }
    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg(program)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    std::io::Write::write_all(&mut child.stdin.take().unwrap(), json.as_bytes()).unwrap();
    Some(text(&child.wait_with_output().unwrap().stdout))
}

#[test]
fn test_WB_INIT_57_the_github_pattern_prints_the_line_breaks_of_a_title_as_spaces() {
    let folder = repository(Some("https://github.com/example/brake-controller.git"));
    assert_eq!(code(&wb(&folder.path, &["init", "--yes"], "")), Some(0));
    let line = pattern(&folder.path, "open_issues").unwrap();
    // Only what `gh` prints is stood in for: the jq of the pattern is run on its JSON.
    let jq = line.split_once("--jq ").unwrap().1;
    if let Some(printed) = jq_prints(&format!("jq -r {}", jq), "[{\"number\":8,\"title\":\"Line\\nbreak\\r\\nand\\ttab\"}]") {
        assert_eq!(printed, "8\tLine break  and\ttab\n");
    }
}

#[test]
fn test_WB_INIT_57_the_gitlab_pattern_prints_the_line_breaks_of_a_title_as_spaces() {
    let folder = repository(Some("https://gitlab.example.com/brakes/controller.git"));
    assert_eq!(code(&wb(&folder.path, &["init", "--yes"], "")), Some(0));
    let line = pattern(&folder.path, "open_issues").unwrap();
    let jq = line.split_once(" | ").unwrap().1;
    assert!(jq.starts_with("jq -r "), "{}", jq);
    if let Some(printed) = jq_prints(jq, "{\"iid\":8,\"title\":\"Line\\nbreak\\r\\nand\\ttab\"}\n") {
        assert_eq!(printed, "8\tLine break  and\ttab\n");
    }
}

// ---- which paths are classed (#84, the owner's answer of 2026-11-19) ----

/// The list `classed` of `watchbill.yml` as the loader reads it back, or `None` when the file has none.
fn classed(folder: &std::path::Path) -> Option<Vec<String>> {
    let document = watchbill::yamlfile::load(config(folder)).unwrap();
    let place = [watchbill::yamlfile::Part::from("classed")];
    document.has(&place).then(|| document.texts(&place, true).unwrap())
}

const CLASSED_QUESTION: &str = "classed code (paths from the root, separated by commas, or [] for none): ([]) ";

#[test]
fn test_WB_INIT_54_yes_writes_classed_as_an_empty_list_after_the_ci() {
    for origin in [Some("https://github.com/example/brake-controller.git"), None] {
        let folder = repository(origin);
        let run = wb(&folder.path, &["init", "--yes"], "");
        assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
        assert_eq!(classed(&folder.path), Some(vec![]), "{:?}", origin);
        let file = read(&config(&folder.path));
        assert!(file.contains("\nci: none\nclassed: []\n") || file.contains("\nci: github-actions\nclassed: []\n"), "{}", file);
    }
}

#[test]
fn test_WB_INIT_54_the_question_proposes_an_empty_list_and_enter_keeps_it() {
    let folder = repository(None);
    // project, git server, tracker, requirements, ci, classed, then the question whether it is OK.
    let run = wb(&folder.path, &["init"], &lines(&["", "", "", "", "", "", "yes"]));
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    let ci = out.find("ci driver").unwrap();
    let asked = out.find(CLASSED_QUESTION).unwrap_or_else(|| panic!("{}", out));
    assert!(ci < asked, "{}", out);
    assert_eq!(classed(&folder.path), Some(vec![]));
}

#[test]
fn test_WB_INIT_54_an_answer_of_paths_is_written_as_the_list() {
    let folder = repository(None);
    let run = wb(&folder.path, &["init"], &lines(&["", "", "", "", "", "src/brake, firmware/safety.c, 'odd name'/x", "yes"]));
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(classed(&folder.path), Some(vec!["src/brake".to_string(), "firmware/safety.c".to_string(), "'odd name'/x".to_string()]));
    // Written in brackets too, and `none`, are taken as such.
    for (answer, list) in [("[src, docs]", vec!["src", "docs"]), ("none", vec![]), ("[]", vec![])] {
        let folder = repository(None);
        let run = wb(&folder.path, &["init"], &lines(&["", "", "", "", "", answer, "yes"]));
        assert_eq!(code(&run), Some(0), "{}: {}", answer, text(&run.stderr));
        let list: Vec<String> = list.iter().map(|path| path.to_string()).collect();
        assert_eq!(classed(&folder.path), Some(list), "{}", answer);
    }
}

#[test]
fn test_WB_INIT_54_a_path_that_wb_role_would_refuse_is_not_taken_and_asked_again() {
    for answer in ["/etc", "a/../b", "src,,docs", "src/\x07", ".", "src, "] {
        let folder = repository(None);
        let run = wb(&folder.path, &["init"], &lines(&["", "", "", "", "", answer, "", "yes"]));
        assert_eq!(code(&run), Some(0), "{:?}: {}", answer, text(&run.stderr));
        let out = text(&run.stdout);
        assert_eq!(out.matches(CLASSED_QUESTION).count(), 2, "{:?}: {}", answer, out);
        assert!(out.contains("Not taken: "), "{:?}: {}", answer, out);
        assert_eq!(classed(&folder.path), Some(vec![]), "{:?}", answer);
    }
}

/// The list `build_files` of `watchbill.yml`, or `None` when it has none.
fn build_files(folder: &std::path::Path) -> Option<Vec<String>> {
    let document = watchbill::yamlfile::load(config(folder)).unwrap();
    let place = [watchbill::yamlfile::Part::from("build_files")];
    document.has(&place).then(|| document.texts(&place, true).unwrap())
}

#[test]
fn test_WB_INIT_51_build_files_holds_the_ci_file_that_was_found_and_is_not_written_for_none() {
    let some = |paths: &[&str]| Some(paths.iter().map(|path| path.to_string()).collect::<Vec<String>>());
    for (files, list) in [
        (vec![".gitlab-ci.yml"], some(&[".gitlab-ci.yml"])),
        (vec![".github/workflows/ci.yml"], some(&[".github/workflows"])),
        (vec!["Jenkinsfile"], some(&["Jenkinsfile"])),
        (vec!["azure-pipelines.yml"], some(&["azure-pipelines.yml"])),
        (vec!["bitbucket-pipelines.yml"], some(&["bitbucket-pipelines.yml"])),
        (vec![".circleci/config.yml"], some(&[".circleci"])),
        (vec![".zuul.yaml"], some(&[".zuul.yaml"])),
        (vec!["zuul.d/jobs.yaml"], some(&["zuul.d"])),
        // Each CI file that was found, in the order of the drivers.
        (vec!["Jenkinsfile", ".gitlab-ci.yml"], some(&[".gitlab-ci.yml", "Jenkinsfile"])),
        // No CI file: no line at all.
        (vec![], None),
        (vec![".github/dependabot.yml"], None),
        (vec!["Makefile"], None),
    ] {
        let folder = repository(None);
        for file in &files {
            put(&folder.path, file, "");
        }
        let run = wb(&folder.path, &["init", "--yes"], "");
        assert_eq!(code(&run), Some(0), "{:?}: {}", files, text(&run.stderr));
        assert_eq!(build_files(&folder.path), list, "{:?}", files);
        if list.is_none() {
            assert!(!read(&config(&folder.path)).contains("build_files"), "{:?}", files);
        }
    }
}

#[test]
fn test_WB_INIT_51_build_files_is_shown_before_it_is_written_after_classed() {
    let folder = repository(None);
    put(&folder.path, ".gitlab-ci.yml", "");
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("build_files: ['.gitlab-ci.yml']"), "{}", text(&run.stdout));
    let file = read(&config(&folder.path));
    assert!(file.contains("\nclassed: []\nbuild_files: ['.gitlab-ci.yml']\n"), "{}", file);
}
