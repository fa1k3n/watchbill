//! `wb answer` (#145): the open questions of an issue shown in full in the terminal, one at a time, and the answers
//! posted in one comment per role. It is run as a person runs it, in a repository made for the test, with what
//! a person types on standard input. The patterns of `watchbill.yml` are small commands over files of a folder
//! `.tracker/` that git does not see: stand-ins, never the network.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The stand-in that posts: each comment is kept whole in a file of its own, `.tracker/posted-<on>-<n>`, and
/// it says on standard error that it posted.
const POST: &str = "n=$(ls .tracker | grep -c \"^posted-$1-\")\ncat > \".tracker/posted-$1-$n\"\necho \"POSTED $1\" >&2\n";

/// A repository set up by `wb init --yes`, with the stand-in patterns and a folder `.tracker/` that git ignores.
fn set_up() -> Folder {
    let folder = repository(None);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let patterns = "issue_comment_bodies: \"cat .tracker/{issue}.bodies\"
issue_comment: \"sh .tracker/post.sh {issue}\"
change_comment_bodies: \"cat .tracker/change-{change}.bodies\"
change_comment: \"sh .tracker/post.sh change-{change}\"
";
    let mut file = fs::OpenOptions::new().append(true).open(config(&folder.path)).unwrap();
    file.write_all(patterns.as_bytes()).unwrap();
    put(&folder.path, ".git/info/exclude", ".tracker/\n");
    put(&folder.path, ".tracker/post.sh", POST);
    git(&folder.path, &["add", "--", ".watchbill"]);
    git(&folder.path, &["commit", "-q", "-m", "Set up"]);
    folder
}

/// The whole comments of an issue (or `change-<n>`), as `issue_comment_bodies` prints them: each body and a NUL
/// byte. After every second one a line break follows, as `gh --jq` prints one.
fn bodies(folder: &Path, on: &str, all: &[&str]) {
    let mut bytes: Vec<u8> = Vec::new();
    for (at, body) in all.iter().enumerate() {
        bytes.extend_from_slice(body.as_bytes());
        bytes.push(0);
        if at % 2 == 1 {
            bytes.push(b'\n');
        }
    }
    fs::create_dir_all(folder.join(".tracker")).unwrap();
    fs::write(folder.join(".tracker").join(format!("{}.bodies", on)), bytes).unwrap();
}

/// The comments posted on an issue (or `change-<n>`), in the order they were posted.
fn posted(folder: &Path, on: &str) -> Vec<String> {
    let mut found = Vec::new();
    for n in 0.. {
        match fs::read_to_string(folder.join(".tracker").join(format!("posted-{}-{}", on, n))) {
            Ok(body) => found.push(body),
            Err(_) => break,
        }
    }
    found
}

/// Two questions of the breakdown, in one comment, each changing the issue's file.
const BREAKDOWN: &str = "[questions] 175 q1 q2
q1: Which name does the event use for where its task came from?
<details>

Why it matters: the name goes into the schema of every event.

1. `task_from`, with the values `env` and `branch`.
2. Another name.

Suggested: 1, because it is the issue's own example.

changes the issue's file: yes

</details>

q2: How is an event with no source read?
<details>

Why it matters: every older event has none.

1. As named by the session.
2. As from the branch.

Suggested: 2, because the branch is newer.

changes the issue's file: yes

</details>
";

/// One question of the tester, which does not change the issue's file.
const TESTER: &str = "[questions] 175.tester q1
q1: Is a clause with no test the tester may write left without one?
<details>

Why it matters: nothing fails if the clause breaks.

1. Let the tester read the two files of tests.
2. Leave the clause without a test.

Suggested: 1, because the clause needs a check.

changes the issue's file: no

</details>
";

const STAMP: &str = "[stamp] tester round 1\nrole: tester\nmodel: claude-opus-5-5\n";

/// Issue 175 with the questions of two roles, and a stamp between them.
fn two_roles() -> Folder {
    let folder = set_up();
    bodies(&folder.path, "175", &[BREAKDOWN, STAMP, TESTER]);
    folder
}

fn answer(folder: &Path, args: &[&str], typed: &str) -> std::process::Output {
    let mut all = vec!["answer"];
    all.extend(args);
    wb(folder, &all, typed)
}

// ---- WB-ANSWER-1: each open question shown in full, one at a time ----

#[test]
fn test_WB_ANSWER_1_each_open_question_is_shown_in_full() {
    let folder = two_roles();
    let run = answer(&folder.path, &["175"], "\n\n\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    for what in [
        "175.q1",
        "Which name does the event use for where its task came from?",
        "Why it matters: the name goes into the schema of every event.",
        "1. `task_from`, with the values `env` and `branch`.",
        "2. Another name.",
        "Suggested: 1, because it is the issue's own example.",
        "175.q2",
        "How is an event with no source read?",
        "Suggested: 2, because the branch is newer.",
        "175.tester.q1",
        "Is a clause with no test the tester may write left without one?",
        "Why it matters: nothing fails if the clause breaks.",
        "2. Leave the clause without a test.",
    ] {
        assert!(out.contains(what), "{}: {}", what, out);
    }
    // Laid out plainly: the folding of the tracker is not shown.
    assert!(!out.contains("<details>") && !out.contains("</details>"), "{}", out);
}

#[test]
fn test_WB_ANSWER_1_the_questions_are_shown_one_at_a_time_in_the_order_they_were_asked() {
    let folder = two_roles();
    // Nothing is typed: the first question is shown, and the next is not.
    let run = answer(&folder.path, &["175"], "");
    let out = text(&run.stdout);
    assert!(out.contains("Which name does the event use"), "{}", out);
    assert!(!out.contains("How is an event with no source read?"), "{}", out);
    assert!(!out.contains("Is a clause with no test"), "{}", out);
    // Answered one by one, they come in the order they were asked.
    let run = answer(&folder.path, &["175"], "\n\n\n");
    let out = text(&run.stdout);
    let at = |what: &str| out.find(what).unwrap_or_else(|| panic!("{}: {}", what, out));
    assert!(at("Which name does the event use") < at("How is an event with no source read?"), "{}", out);
    assert!(at("How is an event with no source read?") < at("Is a clause with no test"), "{}", out);
}

#[test]
fn test_WB_ANSWER_1_a_question_that_is_answered_is_not_shown() {
    let folder = set_up();
    bodies(&folder.path, "175", &[BREAKDOWN, "[answers] 175 q1\nq1: 2\n", TESTER]);
    let run = answer(&folder.path, &["175"], "\n\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(!out.contains("Which name does the event use"), "{}", out);
    assert!(out.contains("How is an event with no source read?"), "{}", out);
}

// ---- WB-ANSWER-2: Enter, a number and words ----

#[test]
fn test_WB_ANSWER_2_enter_takes_the_suggestion_a_number_that_choice_and_other_text_the_words() {
    let folder = two_roles();
    let run = answer(&folder.path, &["175"], "\n1\nonly for the first command\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(
        posted(&folder.path, "175"),
        ["[answers] 175 q1 q2\nq1: 1\nq2: 1\n", "[answers] 175.tester q1\nq1: only for the first command\n"]
    );
}

#[test]
fn test_WB_ANSWER_2_text_that_starts_with_a_number_is_words() {
    let folder = set_up();
    bodies(&folder.path, "175", &[TESTER]);
    let run = answer(&folder.path, &["175"], "2, but only for the first command\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "175"), ["[answers] 175.tester q1\nq1: 2, but only for the first command\n"]);
}

// ---- WB-ANSWER-3: a number that is not a choice is asked again ----

#[test]
fn test_WB_ANSWER_3_a_number_that_is_not_one_of_the_choices_is_asked_again() {
    let folder = set_up();
    bodies(&folder.path, "175", &[TESTER]);
    let run = answer(&folder.path, &["175"], "7\n0\n2\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains("7 is not one of the choices"), "{}", out);
    assert!(out.contains("0 is not one of the choices"), "{}", out);
    assert_eq!(posted(&folder.path, "175"), ["[answers] 175.tester q1\nq1: 2\n"]);
}

// ---- WB-ANSWER-4: what it will post is said, then one comment per role ----

#[test]
fn test_WB_ANSWER_4_it_says_what_it_will_post_before_it_posts() {
    let folder = two_roles();
    let run = answer(&folder.path, &["175"], "\n\n\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let err = text(&run.stderr);
    let said = err.find("[answers] 175 q1 q2\nq1: 1\nq2: 2\n").unwrap_or_else(|| panic!("{}", err));
    let first = err.find("POSTED 175").unwrap_or_else(|| panic!("{}", err));
    assert!(said < first, "{}", err);
    let said = err.find("[answers] 175.tester q1\nq1: 1\n").unwrap_or_else(|| panic!("{}", err));
    assert!(said < err.rfind("POSTED 175").unwrap(), "{}", err);
    assert!(err.contains("issue_comment"), "{}", err);
}

#[test]
fn test_WB_ANSWER_4_one_comment_is_posted_per_role_its_first_line_naming_the_questions() {
    let folder = two_roles();
    let run = answer(&folder.path, &["175"], "\n\n\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let all = posted(&folder.path, "175");
    assert_eq!(all.len(), 2, "{:?}", all);
    assert_eq!(all[0], "[answers] 175 q1 q2\nq1: 1\nq2: 2\n");
    assert_eq!(all[1], "[answers] 175.tester q1\nq1: 1\n");
}

// ---- WB-ANSWER-5: nothing is posted before every question is answered, nor after Ctrl-C ----

#[test]
fn test_WB_ANSWER_5_input_that_ends_before_every_question_is_answered_posts_nothing() {
    let folder = two_roles();
    let run = answer(&folder.path, &["175"], "\n\n");
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(posted(&folder.path, "175").is_empty());
    assert!(text(&run.stderr).contains("nothing is posted"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_ANSWER_5_ctrl_c_posts_nothing() {
    let folder = two_roles();
    let mut child = Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(["answer", "175"])
        .current_dir(&folder.path)
        .env_clear()
        .envs(env())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("wb could not be started");
    let shown = Arc::new(Mutex::new(Vec::new()));
    let mut out = child.stdout.take().unwrap();
    let kept = Arc::clone(&shown);
    let reader = std::thread::spawn(move || {
        let mut chunk = [0u8; 4096];
        while let Ok(n) = out.read(&mut chunk) {
            if n == 0 {
                break;
            }
            kept.lock().unwrap().extend_from_slice(&chunk[..n]);
        }
    });
    let mut stdin = child.stdin.take().unwrap();
    // Two of the three answered; then, at the third, Ctrl-C.
    stdin.write_all(b"\n\n").unwrap();
    stdin.flush().unwrap();
    let waited = Instant::now();
    while !String::from_utf8_lossy(&shown.lock().unwrap()).contains("Is a clause with no test") {
        assert!(waited.elapsed() < Duration::from_secs(20), "the third question was not shown: {}", String::from_utf8_lossy(&shown.lock().unwrap()));
        std::thread::sleep(Duration::from_millis(20));
    }
    let killed = Command::new("kill").args(["-INT", &child.id().to_string()]).status().unwrap();
    assert!(killed.success());
    let status = child.wait().unwrap();
    drop(stdin);
    reader.join().unwrap();
    use std::os::unix::process::ExitStatusExt;
    assert_eq!(status.signal(), Some(2), "{:?}", status);
    assert!(posted(&folder.path, "175").is_empty());
}

// ---- WB-ANSWER-6: --suggested ----

#[test]
fn test_WB_ANSWER_6_suggested_takes_every_suggestion_and_names_the_questions_with_none() {
    let folder = set_up();
    let none = "[questions] 175.build q1\nq1: Which file holds the stub?\n<details>\n\nWhy it matters: the tests call it.\n\n1. A file of its own.\n2. The header.\n\nchanges the issue's file: yes\n\n</details>\n";
    bodies(&folder.path, "175", &[BREAKDOWN, none, TESTER]);
    let run = answer(&folder.path, &["175", "--suggested"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "175"), ["[answers] 175 q1 q2\nq1: 1\nq2: 2\n", "[answers] 175.tester q1\nq1: 1\n"]);
    let err = text(&run.stderr);
    assert!(err.contains("175.build.q1") && err.contains("no suggested answer"), "{}", err);
    assert!(!err.contains("175.tester.q1 has no"), "{}", err);
}

#[test]
fn test_WB_ANSWER_6_a_suggestion_in_words_is_posted_in_words() {
    let folder = set_up();
    let words = "[questions] 175.review q1\nq1: What does the message say?\n<details>\n\nWhy it matters: a person reads it.\n\nSuggested: the file is not there, because that is what happened.\n\nchanges the issue's file: no\n\n</details>\n";
    bodies(&folder.path, "175", &[words]);
    let run = answer(&folder.path, &["175", "--suggested"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "175"), ["[answers] 175.review q1\nq1: the file is not there\n"]);
}

// ---- WB-ANSWER-7: one question answered without asking ----

#[test]
fn test_WB_ANSWER_7_one_question_is_answered_without_asking() {
    let folder = two_roles();
    let run = answer(&folder.path, &["175", "tester.q1", "2"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "175"), ["[answers] 175.tester q1\nq1: 2\n"]);
    assert!(!text(&run.stdout).contains("Which name does the event use"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_ANSWER_7_a_question_of_the_breakdown_is_named_by_its_number() {
    let folder = two_roles();
    let run = answer(&folder.path, &["175", "q2", "as from the branch, for now"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "175"), ["[answers] 175 q2\nq2: as from the branch, for now\n"]);
}

#[test]
fn test_WB_ANSWER_7_a_number_that_is_not_a_choice_or_a_question_not_open_is_refused() {
    let folder = two_roles();
    let run = answer(&folder.path, &["175", "tester.q1", "9"], "");
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let run = answer(&folder.path, &["175", "tester.q7", "1"], "");
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("175.tester.q7"), "{}", text(&run.stderr));
    assert!(posted(&folder.path, "175").is_empty());
}

// ---- WB-ANSWER-8: an answer that changes the issue's file ----

#[test]
fn test_WB_ANSWER_8_after_posting_it_says_the_issue_s_file_is_to_be_changed_and_accepted_again() {
    let folder = set_up();
    let changes = TESTER.replace("changes the issue's file: no", "changes the issue's file: yes");
    bodies(&folder.path, "175", &[&changes]);
    let run = answer(&folder.path, &["175"], "\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let err = text(&run.stderr);
    let said = err.find("changes the issue's file").unwrap_or_else(|| panic!("{}", err));
    assert!(err.find("POSTED 175").unwrap() < said, "{}", err);
    let after = &err[said..];
    assert!(after.contains(".watchbill/issues/175.yml"), "{}", err);
    assert!(after.contains("wb accept 175"), "{}", err);
    assert!(after.contains("wb start tester 175"), "{}", err);
}

#[test]
fn test_WB_ANSWER_8_an_answer_that_does_not_change_the_file_says_nothing_of_it() {
    let folder = set_up();
    bodies(&folder.path, "175", &[TESTER]);
    let run = answer(&folder.path, &["175"], "\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(!text(&run.stderr).contains("wb accept"), "{}", text(&run.stderr));
}

// ---- WB-ANSWER-13: a change with no issue ----

#[test]
fn test_WB_ANSWER_13_the_questions_of_a_change_are_answered_on_the_change() {
    let folder = set_up();
    let review = "[questions] 9.review q1\nq1: Is the message right?\n<details>\n\n1. Yes.\n2. No.\n\nSuggested: 1, because it says what happened.\n\nchanges the issue's file: no\n\n</details>\n";
    bodies(&folder.path, "change-9", &[review]);
    let run = answer(&folder.path, &["--change", "9"], "2\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "change-9"), ["[answers] 9.review q1\nq1: 2\n"]);
    assert!(text(&run.stderr).contains("change_comment"), "{}", text(&run.stderr));
}

// ---- WB-ANSWER-15: the questions of revision 0.0.8 ----

/// Questions as the breakdown and the tester of revision 0.0.8 posted them: one comment each.
const OLD_BREAKDOWN: &str = "[question] 175.q1

What is unclear: which name and values does the event use?

Why it matters: they go onto every event.

1. `task_from`, with the values `env` and `branch`.
2. Another name.

Suggested: 1, because it is the issue's own example.
";

const OLD_TESTER: &str = "[question] 175.tester.q8

What is unclear: may the tester read the two files of tests?

Why it matters: without a test, nothing fails.

1. Let the tester read them.
2. Leave the clause without a test.

Suggested: 2, because the clause is small.
";

#[test]
fn test_WB_ANSWER_15_a_question_of_revision_0_0_8_is_shown_and_answered() {
    let folder = set_up();
    bodies(&folder.path, "175", &[OLD_BREAKDOWN, STAMP, OLD_TESTER]);
    let run = answer(&folder.path, &["175"], "\n1\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains("What is unclear: which name and values does the event use?"), "{}", out);
    assert!(out.contains("Why it matters: without a test, nothing fails."), "{}", out);
    assert_eq!(posted(&folder.path, "175"), ["[answers] 175 q1\nq1: 1\n", "[answers] 175.tester q8\nq8: 1\n"]);
}

#[test]
fn test_WB_ANSWER_15_an_answer_of_revision_0_0_8_closes_its_question() {
    let folder = set_up();
    bodies(&folder.path, "175", &[OLD_BREAKDOWN, OLD_TESTER, "[answer] 175.tester.q8 1"]);
    let run = answer(&folder.path, &["175", "--suggested"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "175"), ["[answers] 175 q1\nq1: 1\n"]);
}

// ---- WB-ANSWER-17: what a marker took back ----

#[test]
fn test_WB_ANSWER_17_a_question_taken_back_by_a_marker_is_not_shown() {
    let folder = set_up();
    bodies(&folder.path, "175", &[STAMP, TESTER, "[revert] tester round 1\n"]);
    let run = answer(&folder.path, &["175"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(!text(&run.stdout).contains("Is a clause with no test"), "{}", text(&run.stdout));
    assert!(posted(&folder.path, "175").is_empty());
}

#[test]
fn test_WB_ANSWER_17_an_answer_taken_back_by_a_marker_leaves_its_question_open() {
    let folder = set_up();
    bodies(&folder.path, "175", &[TESTER, STAMP, "[answers] 175.tester q1\nq1: 2\n", "[revert] tester round 1\n"]);
    let run = answer(&folder.path, &["175"], "\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("Is a clause with no test"), "{}", text(&run.stdout));
    assert_eq!(posted(&folder.path, "175"), ["[answers] 175.tester q1\nq1: 1\n"]);
}

// ---- WB-ANSWER-18: a pattern that is not there ----

#[test]
fn test_WB_ANSWER_18_without_issue_comment_bodies_or_issue_comment_it_ends_with_1_and_posts_nothing() {
    for key in ["issue_comment_bodies", "issue_comment"] {
        let folder = two_roles();
        let kept: String = read(&config(&folder.path)).lines().filter(|line| !line.starts_with(&format!("{}:", key))).map(|line| format!("{}\n", line)).collect();
        fs::write(config(&folder.path), kept).unwrap();
        commit_config(&folder.path);
        let run = answer(&folder.path, &["175", "--suggested"], "");
        assert_eq!(code(&run), Some(1), "{}: {}", key, text(&run.stderr));
        assert!(text(&run.stderr).contains(&format!("`{}`", key)), "{}: {}", key, text(&run.stderr));
        assert!(posted(&folder.path, "175").is_empty(), "{}", key);
    }
}

#[test]
fn test_WB_ANSWER_18_a_failing_pattern_ends_with_1_and_posts_nothing() {
    let folder = two_roles();
    let run = answer(&folder.path, &["176", "--suggested"], "");
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("issue_comment_bodies"), "{}", text(&run.stderr));
}

// ---- WB-ANSWER-19: used wrongly ----

#[test]
fn test_WB_ANSWER_19_it_used_wrongly_ends_with_2_and_posts_nothing() {
    let folder = two_roles();
    for args in [
        vec![],
        vec!["175", "--change", "9"],
        vec!["175", "--suggested", "tester.q1"],
        vec!["175", "tester.q1"],
        vec!["175", "q1", "1", "tester.q1"],
        vec!["175", "tester.q1", "2", "more"],
        vec![".175"],
        vec!["175", "--soon"],
        vec!["175", "tester.q1", "two\nlines"],
    ] {
        let run = answer(&folder.path, &args, "\n\n\n");
        assert_eq!(code(&run), Some(2), "{:?}: {}", args, text(&run.stderr));
        assert!(text(&run.stderr).starts_with("wb answer: "), "{:?}: {}", args, text(&run.stderr));
        assert!(posted(&folder.path, "175").is_empty(), "{:?}", args);
    }
}

#[test]
fn test_WB_ANSWER_19_help_names_answer() {
    let folder = set_up();
    let run = wb(&folder.path, &["help", "answer"], "");
    assert!(text(&run.stdout).contains("wb answer <issue>"), "{}", text(&run.stdout));
}

// ---- WB-ANSWER-20: no open question ----

#[test]
fn test_WB_ANSWER_20_an_issue_with_no_open_question_ends_with_0_and_posts_nothing() {
    let folder = set_up();
    bodies(&folder.path, "175", &[TESTER, "[answers] 175.tester q1\nq1: 1\n"]);
    let run = answer(&folder.path, &["175"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("no open question"), "{}", text(&run.stderr));
    assert!(posted(&folder.path, "175").is_empty());
}

// ---- WB-ANSWER-29 to 33: --show reads the questions and answers none (#225) ----

#[test]
fn test_WB_ANSWER_29_show_prints_each_open_question_in_full_and_posts_nothing() {
    let folder = two_roles();
    let run = answer(&folder.path, &["175", "--show"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    for part in [
        "Which name does the event use for where its task came from?",
        "the name goes into the schema of every event.",
        "`task_from`, with the values `env` and `branch`.",
        "Suggested: 1, because it is the issue's own example.",
        "Is a clause with no test the tester may write left without one?",
        "Leave the clause without a test.",
    ] {
        assert!(out.contains(part), "{}: {}", part, out);
    }
    assert!(out.find("175.q1").unwrap() < out.find("175.tester.q1").unwrap(), "{}", out);
    assert!(out.contains("changes the issue's file: yes"), "{}", out);
    assert!(out.contains("changes the issue's file: no"), "{}", out);
    assert!(!out.contains("> "), "it asks nothing: {}", out);
    assert!(out.contains("\n\n175.tester.q1"), "the questions are apart by an empty line: {}", out);
    assert!(!out.contains("\n\n\n"), "and by no more than one: {}", out);
    assert!(posted(&folder.path, "175").is_empty());
}

#[test]
fn test_WB_ANSWER_30_show_prints_only_the_open_questions() {
    let folder = two_roles();
    bodies(&folder.path, "175", &[BREAKDOWN, STAMP, TESTER, "[answers] 175 q1 q2\nq1: 1\nq2: 1\n"]);
    let run = answer(&folder.path, &["175", "--show"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains("175.tester.q1"), "{}", out);
    assert!(!out.contains("175.q1"), "{}", out);
    assert!(!out.contains("task came from"), "{}", out);
    assert!(posted(&folder.path, "175").is_empty());
}

#[test]
fn test_WB_ANSWER_31_show_says_so_and_ends_with_0_when_no_question_is_open() {
    let folder = set_up();
    bodies(&folder.path, "175", &[TESTER, "[answers] 175.tester q1\nq1: 1\n"]);
    let run = answer(&folder.path, &["175", "--show"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("no open question"), "{}", text(&run.stderr));
    assert!(posted(&folder.path, "175").is_empty());
}

#[test]
fn test_WB_ANSWER_32_show_with_suggested_or_an_answer_ends_with_2() {
    let folder = two_roles();
    for args in [vec!["175", "--show", "--suggested"], vec!["175", "--show", "tester.q1", "2"], vec!["175", "tester.q1", "2", "--show"]] {
        let run = answer(&folder.path, &args, "");
        assert_eq!(code(&run), Some(2), "{:?}: {}", args, text(&run.stderr));
        assert!(text(&run.stderr).starts_with("wb answer: "), "{:?}: {}", args, text(&run.stderr));
        assert!(!text(&run.stderr).contains("is not known"), "--show is a known option: {:?}: {}", args, text(&run.stderr));
        assert!(posted(&folder.path, "175").is_empty(), "{:?}", args);
    }
}

#[test]
fn test_WB_ANSWER_33_show_with_change_prints_the_open_questions_of_the_change() {
    let folder = set_up();
    bodies(&folder.path, "change-9", &[&TESTER.replace("175.", "9.")]);
    let run = answer(&folder.path, &["--show", "--change", "9"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains("Is a clause with no test the tester may write left without one?"), "{}", out);
    assert!(posted(&folder.path, "change-9").is_empty());
}

#[test]
fn test_WB_ANSWER_34_show_needs_no_post_pattern() {
    let folder = set_up();
    bodies(&folder.path, "175", &[BREAKDOWN, STAMP, TESTER]);
    let file = config(&folder.path);
    let kept: String = fs::read_to_string(&file).unwrap().lines().filter(|line| !line.starts_with("issue_comment:")).map(|line| format!("{}\n", line)).collect();
    fs::write(&file, kept).unwrap();
    git(&folder.path, &["add", "--", ".watchbill"]);
    git(&folder.path, &["commit", "-q", "-m", "No post pattern"]);
    let run = answer(&folder.path, &["175", "--show"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("175.tester.q1"), "{}", text(&run.stdout));
    let asked = answer(&folder.path, &["175"], "\n\n\n");
    assert_eq!(code(&asked), Some(1), "answering still needs it: {}", text(&asked.stderr));
}

// ---- WB-ANSWER-35 to 38: several answers in one call (#181) ----

#[test]
fn test_WB_ANSWER_35_two_pairs_of_two_roles_post_two_comments() {
    let folder = two_roles();
    let run = answer(&folder.path, &["175", "q1", "2", "tester.q1", "as it is"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "175"), ["[answers] 175 q1\nq1: 2\n", "[answers] 175.tester q1\nq1: as it is\n"]);
    assert!(!text(&run.stdout).contains("Which name does the event use"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_ANSWER_35_two_pairs_of_one_role_post_one_comment() {
    let folder = two_roles();
    let run = answer(&folder.path, &["175", "q1", "2", "q2", "1"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "175"), ["[answers] 175 q1 q2\nq1: 2\nq2: 1\n"]);
}

#[test]
fn test_WB_ANSWER_36_suggested_with_one_pair_takes_the_suggestion_of_every_other_question() {
    let folder = two_roles();
    let run = answer(&folder.path, &["175", "--suggested", "q1", "2"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "175"), ["[answers] 175 q1 q2\nq1: 2\nq2: 2\n", "[answers] 175.tester q1\nq1: 1\n"]);
}

#[test]
fn test_WB_ANSWER_37_a_wrong_pair_posts_nothing() {
    let folder = two_roles();
    for args in [
        vec!["175", "q1", "2", "tester.q1", "9"],
        vec!["175", "q1", "2", "tester.q7", "1"],
        vec!["175", "--suggested", "q1", "2", "tester.q1", "9"],
    ] {
        let run = answer(&folder.path, &args, "");
        assert_eq!(code(&run), Some(1), "{:?}: {}", args, text(&run.stderr));
        assert!(text(&run.stderr).starts_with("wb answer: "), "{:?}: {}", args, text(&run.stderr));
        assert!(posted(&folder.path, "175").is_empty(), "{:?}", args);
    }
}

#[test]
fn test_WB_ANSWER_38_a_question_named_twice_ends_with_2_and_posts_nothing() {
    let folder = two_roles();
    for args in [
        vec!["175", "q1", "1", "q1", "2"],
        vec!["175", "q1", "1", "breakdown.q1", "2"],
        vec!["175", "q1", "1", "175.q1", "1"],
        vec!["175", "--suggested", "tester.q1", "1", "tester.q1", "2"],
        // Named twice wins over a question that is not open or a number that is not a choice.
        vec!["175", "q1", "1", "tester.q7", "1", "breakdown.q1", "2"],
        vec!["175", "q9", "1", "breakdown.q9", "2"],
        vec!["175", "q1", "9", "q1", "2"],
    ] {
        let run = answer(&folder.path, &args, "");
        assert_eq!(code(&run), Some(2), "{:?}: {}", args, text(&run.stderr));
        assert!(text(&run.stderr).contains("named twice"), "{:?}: {}", args, text(&run.stderr));
        assert!(text(&run.stderr).contains("usage:"), "{:?}: {}", args, text(&run.stderr));
        assert!(text(&run.stderr).starts_with("wb answer: "), "{:?}: {}", args, text(&run.stderr));
        assert!(posted(&folder.path, "175").is_empty(), "{:?}", args);
    }
}

// ---- WB-ANSWER-39 to 42: the role that acts on an answer (#226) ----

/// Two questions of the build, whose choices name the role that acts on each.
const ROLED: &str = "[questions] 54.build q3 q4
q3: A test of the tester's is broader than its requirement. What is changed?
<details>

Why it matters: the test fails for a case the requirement does not name.

1. (tester) Narrow the test to the function.
2. (person) Change the issue's file and accept it again.

Suggested: 1, because the requirement is the issue's own.

</details>

q4: Which message does the refusal say?
<details>

Why it matters: a person reads it.

1. (build) The file is not there.
2. The file cannot be read.

Suggested: 1, because it says what happened.

</details>
";

fn roled() -> Folder {
    let folder = set_up();
    bodies(&folder.path, "54", &[ROLED]);
    folder
}

#[test]
fn test_WB_ANSWER_39_the_role_of_the_choice_is_written_after_the_question_in_the_first_line() {
    let folder = roled();
    let run = answer(&folder.path, &["54", "build.q3", "1"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "54"), ["[answers] 54.build q3>tester\nq3: 1\n"]);
}

#[test]
fn test_WB_ANSWER_39_several_pairs_give_each_its_role_in_one_line() {
    let folder = roled();
    let run = answer(&folder.path, &["54", "build.q3", "2", "build.q4", "1"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "54"), ["[answers] 54.build q3>person q4>build\nq3: 2\nq4: 1\n"]);
}

#[test]
fn test_WB_ANSWER_39_the_questions_one_by_one_take_the_role_of_the_choice_without_asking() {
    let folder = roled();
    let run = answer(&folder.path, &["54"], "1\n1\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "54"), ["[answers] 54.build q3>tester q4>build\nq3: 1\nq4: 1\n"]);
    assert!(!text(&run.stdout).contains("which role acts"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_ANSWER_39_suggested_takes_the_role_of_the_suggested_choice() {
    let folder = roled();
    let run = answer(&folder.path, &["54", "--suggested"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "54"), ["[answers] 54.build q3>tester q4>build\nq3: 1\nq4: 1\n"]);
}

#[test]
fn test_WB_ANSWER_40_an_answer_in_words_asks_which_role_acts_and_enter_takes_the_asking_role() {
    let folder = roled();
    let run = answer(&folder.path, &["54"], "Narrow it, but only for the first command\n\n1\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("which role acts"), "{}", text(&run.stdout));
    assert_eq!(posted(&folder.path, "54"), ["[answers] 54.build q3>build q4>build\nq3: Narrow it, but only for the first command\nq4: 1\n"]);
}

#[test]
fn test_WB_ANSWER_40_the_role_that_is_typed_is_taken_and_a_word_that_is_no_role_asks_again() {
    let folder = roled();
    let run = answer(&folder.path, &["54"], "In words\nnobody\ntester\n1\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("nobody is not a role"), "{}", text(&run.stdout));
    assert_eq!(posted(&folder.path, "54"), ["[answers] 54.build q3>tester q4>build\nq3: In words\nq4: 1\n"]);
}

#[test]
fn test_WB_ANSWER_40_a_choice_with_no_role_asks_which_role_acts() {
    let folder = roled();
    let run = answer(&folder.path, &["54"], "1\n2\nreview\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "54"), ["[answers] 54.build q3>tester q4>review\nq3: 1\nq4: 2\n"]);
}

#[test]
fn test_WB_ANSWER_40_a_pair_in_words_or_with_a_choice_of_no_role_takes_the_asking_role() {
    let folder = roled();
    let run = answer(&folder.path, &["54", "build.q3", "words of mine", "build.q4", "2"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "54"), ["[answers] 54.build q3>build q4>build\nq3: words of mine\nq4: 2\n"]);
}

#[test]
fn test_WB_ANSWER_41_a_person_answer_says_the_file_is_to_be_changed_and_accepted_again() {
    let folder = roled();
    let run = answer(&folder.path, &["54", "build.q3", "2"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let err = text(&run.stderr);
    let said = err.find("changes the issue's file").unwrap_or_else(|| panic!("{}", err));
    assert!(err.find("POSTED 54").unwrap() < said, "{}", err);
    assert!(err[said..].contains("wb accept 54"), "{}", err);
}

#[test]
fn test_WB_ANSWER_41_an_answer_to_another_role_says_nothing_of_the_file() {
    let folder = roled();
    let run = answer(&folder.path, &["54", "build.q3", "1"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(!text(&run.stderr).contains("wb accept"), "{}", text(&run.stderr));
    assert!(!text(&run.stderr).contains("changes the issue's file"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_ANSWER_41_a_line_on_the_file_in_the_question_is_not_read_where_the_choices_name_roles() {
    let folder = set_up();
    let line = ROLED.replace("</details>\n\nq4", "changes the issue's file: yes\n\n</details>\n\nq4");
    bodies(&folder.path, "54", &[&line]);
    let run = answer(&folder.path, &["54", "build.q3", "1"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(!text(&run.stderr).contains("wb accept"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_ANSWER_42_a_question_of_0_10_0_is_answered_with_no_role_and_none_asked() {
    let folder = set_up();
    bodies(&folder.path, "175", &[TESTER]);
    let run = answer(&folder.path, &["175"], "In words\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "175"), ["[answers] 175.tester q1\nq1: In words\n"]);
    assert!(!text(&run.stdout).contains("which role acts"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_ANSWER_44_an_answer_that_names_a_role_closes_its_question() {
    let folder = set_up();
    let tester = ROLED.replace("54.build", "54.tester");
    bodies(&folder.path, "54", &[&tester, "[answers] 54.tester q3>tester q4>build\nq3: 1\nq4: 1\n"]);
    let run = answer(&folder.path, &["54"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("no open question"), "{}", text(&run.stderr));
    assert!(posted(&folder.path, "54").is_empty());
}

#[test]
fn test_WB_ANSWER_45_a_word_in_brackets_that_is_no_role_is_a_choice_with_no_role() {
    let folder = set_up();
    let other = ROLED.replace("1. (build) The file", "1. (optional) The file");
    bodies(&folder.path, "54", &[&other]);
    // A pair takes the asking role for it.
    let run = answer(&folder.path, &["54", "build.q3", "1", "build.q4", "1"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "54"), ["[answers] 54.build q3>tester q4>build\nq3: 1\nq4: 1\n"]);
    // One by one, the person is asked.
    let folder = set_up();
    bodies(&folder.path, "54", &[&other]);
    let run = answer(&folder.path, &["54"], "1\n1\nreview\n");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(posted(&folder.path, "54"), ["[answers] 54.build q3>tester q4>review\nq3: 1\nq4: 1\n"]);
}
