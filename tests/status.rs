//! `wb status` (#139, #155): where each issue in the flow stands, under the last step it has done, run as a person runs it, in a
//! repository made for the test. The patterns of `watchbill.yml` are small commands that read files of a folder
//! `.tracker/` in the repository: stand-ins, never the network.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// What a stand-in tracker says: the first lines of the comments of each issue.
const PATTERNS: &str = "issue_comments: \"cat .tracker/{issue}.comments\"
";

/// A repository set up by `wb init --yes`, with a git identity, the stand-in patterns and the folder `.tracker/`.
fn set_up() -> Folder {
    set_up_with(PATTERNS)
}

/// The same, with the patterns the test gives.
fn set_up_with(patterns: &str) -> Folder {
    let folder = repository(None);
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let mut file = fs::OpenOptions::new().append(true).open(config(&folder.path)).unwrap();
    file.write_all(patterns.as_bytes()).unwrap();
    git(&folder.path, &["add", "--", ".watchbill"]);
    git(&folder.path, &["commit", "-q", "-m", "Set up"]);
    // An acceptance and a stamp count only against `.watchbill/signers` of the default branch (#323).
    signing(&folder.path);
    folder
}

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
    text(&run.stdout)
}

/// Puts the issues in the flow: each gets a branch `issue-<issue>-work` that holds a commit the default branch
/// does not, and the same branch at `origin`, unless it has a branch already.
fn open(folder: &Path, issues: &[&str]) {
    for issue in issues {
        let name = format!("issue-{}-work", issue);
        if !git_text(folder, &["branch", "--list", &name]).trim().is_empty() {
            continue;
        }
        let back = git_text(folder, &["rev-parse", "--abbrev-ref", "HEAD"]).trim().to_string();
        git(folder, &["checkout", "-q", "-b", &name]);
        git(folder, &["commit", "-q", "--allow-empty", "-m", "Work"]);
        git(folder, &["checkout", "-q", &back]);
        at_origin(folder, &name);
    }
}

/// Makes the branch of `origin` of that name as the local one is.
fn at_origin(folder: &Path, name: &str) {
    let tip = git_text(folder, &["rev-parse", name]);
    git(folder, &["update-ref", &format!("refs/remotes/origin/{}", name), tip.trim()]);
}

/// The first lines of the comments of an issue.
fn comments(folder: &Path, issue: &str, lines: &[&str]) {
    let all: String = lines.iter().map(|line| format!("{}\n", line)).collect();
    put(folder, &format!(".tracker/{}.comments", issue), &all);
    // An issue whose lines hold a question or an answer is one begun before this revision (#333): its job has no stamp
    // on its ref, and its questions and answers are read from the tracker, the `[stamp]` and `[revert]` lines of its
    // comments placing them. Any other has its stamps and markers as commits on its job's ref (#236), and `wb` reads no
    // `[stamp]` comment; a job with a stamp on its ref is read from the ref alone, so no question of the tracker counts.
    let asks = |line: &&str| ["[question] ", "[questions] ", "[answer] ", "[answers] "].iter().any(|start| line.starts_with(start));
    if !lines.iter().any(|line| asks(&line)) {
        stamps_from_lines(folder, issue, lines, &[]);
    }
}

/// Makes the branch `issue-<issue>-work` with the issue's file, and its acceptance when `accepted`, and goes
/// back to the branch it was on.
fn branch(folder: &Path, issue: &str, accepted: bool) {
    let back = git_text(folder, &["rev-parse", "--abbrev-ref", "HEAD"]).trim().to_string();
    let name = format!("issue-{}-work", issue);
    let exists = !git_text(folder, &["branch", "--list", &name]).trim().is_empty();
    git(folder, &if exists { vec!["checkout", "-q", &name] } else { vec!["checkout", "-q", "-b", &name] });
    put(folder, &format!(".watchbill/issues/{}.yml", issue), &BARE_ISSUE.replace("{issue}", issue));
    let mut added = vec![format!(".watchbill/issues/{}.yml", issue)];
    if accepted {
        let run = wb(folder, &["plan", "accept", "--no-commit", "--", issue], "");
        assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
        added.push(format!(".watchbill/issues/{}.accepted", issue));
    }
    let mut add = vec!["add", "--"];
    add.extend(added.iter().map(String::as_str));
    git(folder, &add);
    git(folder, &["commit", "-q", "-m", "The issue's file"]);
    git(folder, &["checkout", "-q", &back]);
    at_origin(folder, &name);
}

/// Seven issues, one at each step, and the stamps that put them there.
fn at_every_step() -> Folder {
    let folder = set_up();
    open(&folder.path, &["1", "2", "3", "4", "5", "6", "7"]);
    branch(&folder.path, "2", false);
    for issue in ["3", "4", "5", "6", "7"] {
        branch(&folder.path, issue, true);
    }
    comments(&folder.path, "4", &["[stamp] tester round 1"]);
    comments(&folder.path, "5", &["[stamp] tester round 1", "[stamp] build round 1"]);
    comments(&folder.path, "6", &["[stamp] tester round 1", "[stamp] build round 1", "[stamp] review round 1"]);
    comments(&folder.path, "7", &["[stamp] build round 1", "[stamp] review round 1", "[stamp] fix round 1"]);
    for issue in ["1", "2", "3"] {
        comments(&folder.path, issue, &[]);
    }
    folder
}

/// The names of the open questions of an issue, as `wb status <issue>` says them, or none.
fn open_names(folder: &Path, issue: &str) -> String {
    let (out, _) = stdout(folder, &["status", issue]);
    out.lines().find_map(|line| line.strip_prefix("questions open: ")).unwrap_or("").to_string()
}

fn stdout(folder: &Path, args: &[&str]) -> (String, Option<i32>) {
    let run = wb(folder, args, "");
    (text(&run.stdout), code(&run))
}

#[test]
fn test_WB_STATUS_2_each_issue_is_under_the_last_step_it_has_done() {
    let folder = at_every_step();
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    let expected = "Jobs waiting for you:
  (use \"wb plan accept <name>...\" to accept the plan)
\t2  issue-2-work  planned

Jobs in the flow:
\t3  issue-3-work  accepted
\t4  issue-4-work  tested
\t5  issue-5-work  built
\t6  issue-6-work  reviewed
\t7  issue-7-work  fixed
";
    assert_eq!(out, expected);
}

#[test]
fn test_WB_STATUS_11_a_heading_with_no_issue_is_not_printed() {
    let folder = at_every_step();
    let (out, _) = stdout(&folder.path, &["status", "--stage", "fixed"]);
    assert_eq!(out, "Jobs in the flow:\n\t7  issue-7-work  fixed\n");
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", &[]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "");
    for heading in ["planned", "accepted", "tested", "built", "reviewed", "fixed", "question", "needs a person", "Not read", "Jobs"] {
        assert!(!out.contains(heading), "{} in {}", heading, out);
    }
}

#[test]
fn test_WB_STATUS_12_no_line_says_a_round_a_command_or_a_step_going_on() {
    let folder = at_every_step();
    let (out, _) = stdout(&folder.path, &["status"]);
    // The hint lines (#284) say what a person may run; the lines of the jobs say no round and no command.
    let jobs: Vec<&str> = out.lines().filter(|line| line.starts_with('\t')).collect();
    assert_eq!(jobs.len(), 6, "{}", out);
    // Every other line is a heading, a blank, or a hint, and a hint is in git's parenthesised form.
    for line in out.lines().filter(|line| !line.starts_with('\t') && !line.is_empty()) {
        assert!(line.ends_with(':') || (line.starts_with("  (use \"wb ") && line.ends_with(')')), "{}", line);
    }
    for line in jobs {
        for word in ["round", "running", "going on", "next", "wb ", "run "] {
            assert!(!line.contains(word), "{} in {}", word, line);
        }
    }
}

#[test]
fn test_WB_STATUS_13_the_jobs_are_printed_by_stage_and_then_by_number_the_person_s_move_first() {
    // The numbers are not in the order of the stages: 5 is reviewed, 6 and 7 are accepted, 8 is tested.
    let folder = set_up();
    open(&folder.path, &["5", "6", "7", "8"]);
    comments(&folder.path, "5", &["[stamp] tester round 1", "[stamp] build round 1", "[stamp] review round 1"]);
    comments(&folder.path, "6", &[]);
    comments(&folder.path, "7", &[]);
    comments(&folder.path, "8", &["[stamp] tester round 1"]);
    for issue in ["5", "6", "7", "8"] {
        branch(&folder.path, issue, true);
    }
    let (out, _) = stdout(&folder.path, &["status"]);
    let jobs: Vec<&str> = out.lines().filter(|line| line.starts_with('\t')).map(|line| line.split_whitespace().next().unwrap()).collect();
    assert_eq!(jobs, ["6", "7", "8", "5"], "{}", out);
    // The short form keeps the order of the numbers, as git sorts by path.
    let (out, _) = said(&folder.path, &["-s"]);
    let numbers: Vec<&str> = out.lines().map(|line| line.split_whitespace().nth(1).unwrap()).collect();
    assert_eq!(numbers, ["5", "6", "7", "8"], "{}", out);
}

#[test]
fn test_WB_STATUS_2_an_issue_file_that_is_not_accepted_as_it_stands_is_planned_only() {
    let folder = set_up();
    open(&folder.path, &["3"]);
    comments(&folder.path, "3", &[]);
    branch(&folder.path, "3", true);
    // The file is changed on the branch after its acceptance: the hash no longer holds.
    git(&folder.path, &["checkout", "-q", "issue-3-work"]);
    put(&folder.path, ".watchbill/issues/3.yml", &format!("{}# changed\n", BARE_ISSUE.replace("{issue}", "3")));
    git(&folder.path, &["commit", "-q", "-a", "-m", "Changed"]);
    git(&folder.path, &["checkout", "-q", "-"]);
    at_origin(&folder.path, "issue-3-work");
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Jobs waiting for you:\n  (use \"wb plan accept <name>...\" to accept the plan)\n\t3  issue-3-work  planned\n");
}

#[test]
fn test_WB_STATUS_2_the_last_stamp_decides_between_reviewed_and_fixed() {
    let folder = set_up();
    open(&folder.path, &["8"]);
    comments(&folder.path, "8", &["[stamp] review round 1", "[stamp] fix round 1", "[stamp] review round 2"]);
    branch(&folder.path, "8", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Jobs in the flow:\n\t8  issue-8-work  reviewed\n");
}

#[test]
fn test_WB_STATUS_18_the_files_are_read_from_the_origin_branch_when_there_is_no_local_one() {
    let folder = set_up();
    open(&folder.path, &["9"]);
    comments(&folder.path, "9", &[]);
    branch(&folder.path, "9", false);
    // Make the branch a remote branch only.
    let tip = git_text(&folder.path, &["rev-parse", "issue-9-work"]);
    git(&folder.path, &["update-ref", "refs/remotes/origin/issue-9-work", tip.trim()]);
    git(&folder.path, &["branch", "-q", "-D", "issue-9-work"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Jobs waiting for you:\n  (use \"wb plan accept <name>...\" to accept the plan)\n\t9  issue-9-work  planned\n");
}

#[test]
fn test_WB_STATUS_18_an_acceptance_that_is_written_and_not_committed_gives_planned() {
    let folder = set_up();
    open(&folder.path, &["2"]);
    comments(&folder.path, "2", &[]);
    branch(&folder.path, "2", false);
    git(&folder.path, &["checkout", "-q", "issue-2-work"]);
    let run = wb(&folder.path, &["plan", "accept", "--no-commit", "--", "2"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(folder.path.join(".watchbill/issues/2.accepted").exists());
    git(&folder.path, &["checkout", "-q", "-"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Jobs waiting for you:\n  (use \"wb plan accept <name>...\" to accept the plan)\n\t2  issue-2-work  planned\n");
}

#[test]
fn test_WB_STATUS_9_questions_open_is_printed_after_the_steps() {
    let folder = set_up();
    open(&folder.path, &["175", "176", "177"]);
    // 176 was begun before (#333): its questions are at the tracker, and its job has no stamp on its ref. 175 is built.
    comments(
        &folder.path,
        "176",
        &["[stamp] build round 1", "[question] 176.build.q1", "[question] 176.build.q2", "[answer] 176.build.q1 2"],
    );
    comments(&folder.path, "175", &["[stamp] build round 1"]);
    comments(&folder.path, "177", &[]);
    branch(&folder.path, "175", true);
    branch(&folder.path, "176", true);
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    // 177 is not broken down and has no line; 176 waits for a person, and its open question is named by `wb status 176`.
    assert_eq!(
        out,
        "Jobs waiting for you:\n  (use \"wb answer <issue>...\" to answer its questions)\n\t176  issue-176-work  accepted, 1 question open\n\nJobs in the flow:\n\t175  issue-175-work  built\n"
    );
    assert_eq!(open_names(&folder.path, "176"), "176.build.q2");
}

#[test]
fn test_WB_STATUS_22_an_issue_with_an_open_question_stays_under_its_step() {
    let folder = set_up();
    open(&folder.path, &["176"]);
    // The job's ref holds a stamp (#333), so its question is a commit on the ref too, and not a comment.
    comments(&folder.path, "176", &["[stamp] build round 1"]);
    stamps_on_ref(&folder.path, "176", &["[questions] 176.build q1\n\nq1: Which one?\n"]);
    branch(&folder.path, "176", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert!(out.starts_with("Jobs waiting for you:\n  (use \"wb answer <issue>...\" to answer its questions)\n\t176  issue-176-work  built, 1 question open\n"), "{}", out);
    assert_eq!(open_names(&folder.path, "176"), "176.build.q1");
    let (out, _) = stdout(&folder.path, &["status", "--stage", "built"]);
    assert_eq!(out, "Jobs waiting for you:\n  (use \"wb answer <issue>...\" to answer its questions)\n\t176  issue-176-work  built, 1 question open\n");
}

#[test]
fn test_WB_STATUS_9_the_open_questions_of_an_issue_are_named_in_the_order_they_were_asked() {
    let folder = set_up();
    open(&folder.path, &["176"]);
    comments(&folder.path, "176", &["[question] 176.q1", "[question] 176.q2"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Jobs waiting for you:\n  (use \"wb answer <issue>...\" to answer its questions)\n\t176  issue-176-work  2 questions open\n");
    assert_eq!(open_names(&folder.path, "176"), "176.q1, 176.q2");
}

#[test]
fn test_WB_STATUS_3_a_question_is_closed_by_an_answer_to_its_name_and_by_nothing_else() {
    let folder = set_up();
    open(&folder.path, &["1"]);
    // Another question's answer, another issue's answer, and lines that only mention the name.
    comments(
        &folder.path,
        "1",
        &["[question] 1.q1", "[question] 1.q2", "[answer] 1.q3 2", "[answer] 2.q1 2", "[comment] [answer] 1.q1", "[question] [answer] 1.q2"],
    );
    let (out, _) = stdout(&folder.path, &["status"]);
    assert!(out.ends_with("\t1  issue-1-work  2 questions open\n"), "{}", out);
    assert_eq!(open_names(&folder.path, "1"), "1.q1, 1.q2");
}

#[test]
fn test_WB_STATUS_3_an_answer_in_words_closes_its_question() {
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", &["[question] 1.q1", "[answer] 1.q1 only for the first command"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "");
}

#[test]
fn test_WB_STATUS_14_a_line_that_starts_with_question_is_never_an_answer() {
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", &["[question] 1.q1", "[question] [answer] 1.q1", "[question] 1.q1 [answer] 1.q1"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert!(out.ends_with("\t1  issue-1-work  1 question open\n"), "{}", out);
    assert_eq!(open_names(&folder.path, "1"), "1.q1");
}

#[test]
fn test_WB_STATUS_15_every_answer_line_counts_as_a_persons_until_wb_reads_who_wrote_a_comment() {
    // `issue_comments` prints no author: an answer from an account that is not a person's cannot be told apart.
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", &["[question] 1.q1", "[answer] 1.q1 2"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "");
}

#[test]
fn test_WB_STATUS_30_an_answer_closes_its_question_whatever_follows_the_name() {
    // A choice number is not held against the choices of the question: only first lines are read.
    let folder = set_up();
    open(&folder.path, &["5"]);
    comments(&folder.path, "5", &["[question] 5.q1", "[answer] 5.q1 7"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "");
}

#[test]
fn test_WB_STATUS_29_an_answer_of_suggested_closes_the_questions_before_it_and_not_those_after() {
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", &["[question] 1.q1", "[answer] 1 suggested", "[question] 1.q2"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert!(out.ends_with("\t1  issue-1-work  1 question open\n"), "{}", out);
    assert_eq!(open_names(&folder.path, "1"), "1.q2");
}

#[test]
fn test_WB_STATUS_4_a_branch_of_another_issue_whose_number_starts_the_same_is_not_taken() {
    let folder = set_up();
    open(&folder.path, &["1", "12"]);
    comments(&folder.path, "1", &[]);
    comments(&folder.path, "12", &[]);
    branch(&folder.path, "12", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Jobs in the flow:\n\t12  issue-12-work  accepted\n");
}

#[test]
fn test_WB_STATUS_5_one_issue_prints_its_heading_and_its_line_and_what_they_were_worked_out_from() {
    let folder = set_up();
    open(&folder.path, &["176", "177", "178"]);
    // 176 has a stamp on its ref (#333), so its question is a commit on the ref; 178 was begun before, and its question
    // is a comment.
    comments(&folder.path, "176", &["[stamp] tester round 1", "[stamp] build round 1"]);
    stamps_on_ref(&folder.path, "176", &["[questions] 176.build q1\n\nq1: Which one?\n"]);
    comments(&folder.path, "177", &[]);
    comments(&folder.path, "178", &["[question] 178.build.q1"]);
    branch(&folder.path, "176", true);
    let (out, status) = stdout(&folder.path, &["status", "176"]);
    assert_eq!(status, Some(0));
    assert!(out.starts_with("Jobs waiting for you:\n  (use \"wb answer <issue>...\" to answer its questions)\n\t176  issue-176-work  built, 1 question open\n"), "{}", out);
    assert!(out.contains("\nquestions open: 176.build.q1\n"), "{}", out);
    assert!(!out.contains("177"), "{}", out);
    for line in [
        "[stamp] tester round 1",
        "[stamp] build round 1",
        "[questions] 176.build q1",
        "issue-176-work",
        ".watchbill/issues/176.yml",
        ".watchbill/issues/176.accepted",
    ] {
        assert!(out.contains(line), "{} not in {}", line, out);
    }
    let (out, _) = stdout(&folder.path, &["status", "178"]);
    assert!(out.contains("[question] 178.build.q1"), "{}", out);
}

#[test]
fn test_WB_STATUS_24_one_issue_prints_the_heading_worked_out_from_and_its_lines_indented() {
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", &["[stamp] tester round 1"]);
    let (out, _) = stdout(&folder.path, &["status", "1"]);
    // Since #323 a stamp counts only signed by a key of `.watchbill/signers`, and the line says by whom.
    assert!(out.starts_with("Jobs in the flow:\n\t1  issue-1-work  tested\n\nSessions:\n  tester round 1, 2000-01-01T00:00:00Z, "), "{}", out);
    assert!(out.contains("\n\nWorked out from:\n  refs/heads/issue-1-work: no .watchbill/issues/1.yml\n  refs/remotes/origin/issue-1-work: no .watchbill/issues/1.yml\n  [stamp] tester round 1  (on refs/watchbill/1, signed by ada@example.org)\n"), "{}", out);
    // The full list has no such heading.
    let (out, _) = stdout(&folder.path, &["status"]);
    assert!(!out.contains("Worked out from"), "{}", out);
}

#[test]
fn test_WB_STATUS_6_stage_prints_only_that_heading_matched_without_regard_to_case() {
    let folder = at_every_step();
    // Only the jobs of that stage word, whatever the case of the word.
    let (out, status) = stdout(&folder.path, &["status", "--stage", "ACCEPTED"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Jobs in the flow:\n\t3  issue-3-work  accepted\n");
    let (out, _) = stdout(&folder.path, &["status", "--stage", "planned"]);
    assert_eq!(out, "Jobs waiting for you:\n  (use \"wb plan accept <name>...\" to accept the plan)\n\t2  issue-2-work  planned\n");
}

#[test]
fn test_WB_STATUS_16_a_stage_that_is_not_known_is_used_wrongly() {
    let folder = at_every_step();
    let run = wb(&folder.path, &["status", "--stage", "running"], "");
    assert_eq!(code(&run), Some(2));
    assert!(text(&run.stdout).is_empty());
    assert!(text(&run.stderr).contains("running"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_STATUS_20_a_stage_without_a_value_and_an_option_not_known_are_used_wrongly() {
    let folder = at_every_step();
    // Only `wb status` says `wb status:`; a command that is not known says `wb: no command`.
    for args in [vec!["status", "--stage"], vec!["status", "--all"], vec!["status", "../x"], vec!["status", "1", "2"]] {
        let run = wb(&folder.path, &args, "");
        assert_eq!(code(&run), Some(2), "{:?}", args);
        assert!(text(&run.stderr).starts_with("wb status:"), "{:?}: {}", args, text(&run.stderr));
    }
}

#[test]
fn test_WB_STATUS_6_one_issue_names_its_open_questions_and_stage_takes_no_heading_of_before() {
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", &["[question] 1.q1"]);
    let (out, _) = stdout(&folder.path, &["status", "1"]);
    assert!(out.contains("\nquestions open: 1.q1\n"), "{}", out);
    let run = wb(&folder.path, &["status", "--stage", "questions open"], "");
    assert_eq!(code(&run), Some(2));
}

#[test]
fn test_WB_STATUS_7_a_pattern_that_fails_for_one_issue_lists_it_under_not_read_and_the_others_are_printed() {
    let folder = set_up();
    open(&folder.path, &["1", "2"]);
    comments(&folder.path, "1", &[]);
    branch(&folder.path, "1", true);
    // No comments file for issue 2: `cat` fails.
    let run = wb(&folder.path, &["status"], "");
    assert_eq!(code(&run), Some(1));
    let out = text(&run.stdout);
    assert!(out.starts_with("Jobs in the flow:\n\t1  issue-1-work  accepted\n"), "{}", out);
    assert!(out.contains("\nNot read:\n  2  `issue_comments` of watchbill.yml ended with 1\n"), "{}", out);
}

/// Every file of the working tree but `.git`, with its size.
fn tree(folder: &Path) -> Vec<(String, u64)> {
    let mut found = Vec::new();
    let mut todo = vec![folder.to_path_buf()];
    while let Some(path) = todo.pop() {
        for entry in fs::read_dir(&path).unwrap() {
            let entry = entry.unwrap();
            let meta = entry.metadata().unwrap();
            if entry.file_name() == ".git" {
                continue;
            }
            if meta.is_dir() {
                todo.push(entry.path());
            } else {
                found.push((entry.path().display().to_string(), meta.len()));
            }
        }
    }
    found.sort();
    found
}

fn repository_state(folder: &Path) -> [String; 4] {
    [
        git_text(folder, &["rev-parse", "--abbrev-ref", "HEAD"]),
        git_text(folder, &["status", "--porcelain", "-uall"]),
        git_text(folder, &["for-each-ref"]),
        git_text(folder, &["reflog"]),
    ]
}

#[test]
fn test_WB_STATUS_8_it_writes_nothing() {
    let folder = at_every_step();
    let before = repository_state(&folder.path);
    let files = tree(&folder.path);
    for args in [vec!["status"], vec!["status", "7"], vec!["status", "--stage", "fixed"]] {
        assert_eq!(code(&wb(&folder.path, &args, "")), Some(0));
    }
    // The branches, the reflog and the index are as they were, and so is every file of the tree.
    assert_eq!(before, repository_state(&folder.path));
    assert_eq!(files, tree(&folder.path));
}

#[test]
fn test_WB_STATUS_17_it_switches_no_branch() {
    let folder = at_every_step();
    git(&folder.path, &["checkout", "-q", "issue-4-work"]);
    let before = git_text(&folder.path, &["rev-parse", "--abbrev-ref", "HEAD"]);
    assert_eq!(before.trim(), "issue-4-work");
    assert_eq!(code(&wb(&folder.path, &["status"], "")), Some(0));
    assert_eq!(git_text(&folder.path, &["rev-parse", "--abbrev-ref", "HEAD"]), before);
    assert!(git_text(&folder.path, &["reflog", "-1"]).contains("checkout: moving from"), "no checkout was made by wb");
}

#[test]
fn test_WB_STATUS_21_a_folder_that_is_not_set_up_or_not_a_repository_ends_with_2() {
    let folder = repository(None);
    let run = wb(&folder.path, &["status"], "");
    assert_eq!(code(&run), Some(2));
    assert!(text(&run.stderr).starts_with("wb status:") && text(&run.stderr).contains("watchbill.yml"), "{}", text(&run.stderr));
    let plain = folder_named("plain");
    let run = wb(&plain.path, &["status"], "");
    assert_eq!(code(&run), Some(2));
    assert!(text(&run.stderr).starts_with("wb status:"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_STATUS_28_it_runs_from_a_folder_below_the_root() {
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", &[]);
    fs::create_dir_all(folder.path.join("src/deep")).unwrap();
    let (out, status) = stdout(&folder.path.join("src/deep"), &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "");
}

#[test]
fn test_WB_STATUS_26_without_issue_comments_it_says_so_once_on_standard_error() {
    let folder = set_up();
    let file = config(&folder.path);
    let kept: String = read(&file).lines().filter(|line| !line.starts_with("issue_comments:")).map(|line| format!("{}\n", line)).collect();
    fs::write(&file, kept).unwrap();
    commit_config(&folder.path);
    open(&folder.path, &["1", "2", "3"]);
    for issue in ["1", "2", "3"] {
        branch(&folder.path, issue, true);
    }
    let run = wb(&folder.path, &["status"], "");
    assert_eq!(code(&run), Some(0));
    assert_eq!(text(&run.stdout), "Jobs in the flow:\n\t1  issue-1-work  accepted\n\t2  issue-2-work  accepted\n\t3  issue-3-work  accepted\n");
    let said = text(&run.stderr);
    assert_eq!(said.lines().count(), 1, "{}", said);
    assert!(said.starts_with("wb status:") && said.contains("issue_comments"), "{}", said);
}

#[test]
fn test_WB_STATUS_31_stamps_before_the_last_planner_stamp_no_longer_decide_the_step() {
    let folder = set_up();
    open(&folder.path, &["1", "2", "3"]);
    let before = ["[stamp] planner round 1", "[stamp] tester round 1", "[stamp] build round 1", "[stamp] review round 1", "[stamp] fix round 1"];
    let with = |issue: &str, after: &[&str]| {
        let all: Vec<&str> = before.iter().copied().chain(["[stamp] planner round 2"]).chain(after.iter().copied()).collect();
        comments(&folder.path, issue, &all);
    };
    with("1", &[]);
    with("2", &[]);
    with("3", &["[stamp] build round 1"]);
    branch(&folder.path, "1", true);
    branch(&folder.path, "2", false);
    branch(&folder.path, "3", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Jobs waiting for you:\n  (use \"wb plan accept <name>...\" to accept the plan)\n\t2  issue-2-work  planned\n\nJobs in the flow:\n\t1  issue-1-work  accepted\n\t3  issue-3-work  built\n");
}

// ---- The questions folded into one comment per session, and the answers in one (#145) ----

/// The names of the open questions that `wb status 1` says for issue 1, with these first lines.
fn open_of(lines: &[&str]) -> String {
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", lines);
    let (out, status) = stdout(&folder.path, &["status", "1"]);
    assert_eq!(status, Some(0), "{}", out);
    out.lines().find_map(|line| line.strip_prefix("questions open: ")).unwrap_or("").to_string()
}

#[test]
fn test_WB_ANSWER_11_a_question_of_a_questions_comment_is_open_until_an_answers_comment_names_it() {
    assert_eq!(open_of(&["[questions] 1.tester q1 q2"]), "1.tester.q1, 1.tester.q2");
    assert_eq!(open_of(&["[questions] 1.tester q1 q2", "[answers] 1.tester q1"]), "1.tester.q2");
    assert_eq!(open_of(&["[questions] 1.tester q1 q2", "[answers] 1.tester q1 q2"]), "");
    // The planner names its questions with the issue alone.
    assert_eq!(open_of(&["[questions] 1 q1 q2", "[answers] 1 q2"]), "1.q1");
}

#[test]
fn test_WB_ANSWER_11_an_answers_comment_closes_only_the_questions_of_its_own_role_and_issue() {
    let out = open_of(&["[questions] 1.tester q1", "[questions] 1.build q1", "[questions] 1 q1", "[answers] 1.build q1", "[answers] 2.tester q1"]);
    assert_eq!(out, "1.tester.q1, 1.q1");
}

#[test]
fn test_WB_ANSWER_11_words_after_the_role_that_are_no_question_name_close_nothing() {
    // The form the owner wrote in the trial of 0.0.8, which no prompt gives.
    assert_eq!(open_of(&["[questions] 1 q1", "[answers] 1 as you suggested"]), "1.q1");
    // A line that only names the form is no answer.
    assert_eq!(open_of(&["[questions] 1 q1", "[comment] [answers] 1 q1", "[questions] [answers] 1 q1"]), "1.q1");
}

// The forms of revision 0.0.8 alone are read as #139 read them: the tests of WB-STATUS-3, 9, 14, 29 and 30 keep them.

#[test]
fn test_WB_ANSWER_11_the_old_and_the_new_forms_answer_each_other() {
    let out = open_of(&["[question] 1.tester.q1", "[questions] 1.tester q2 q3", "[answers] 1.tester q1", "[answer] 1.tester.q2 1"]);
    assert_eq!(out, "1.tester.q3");
    // `[answer] <issue> suggested` closes the questions of a `[questions]` comment before it.
    assert_eq!(open_of(&["[questions] 1.tester q1", "[answer] 1 suggested"]), "");
}

#[test]
fn test_WB_ANSWER_16_a_questions_comment_from_a_reverted_stamp_up_to_its_marker_does_not_count() {
    let out = open_of(&["[questions] 1 q1", "[stamp] tester round 1", "[questions] 1.tester q1", "[revert] tester round 1"]);
    assert_eq!(out, "1.q1");
}

#[test]
fn test_WB_ANSWER_16_an_answers_comment_from_a_reverted_stamp_up_to_its_marker_does_not_count() {
    let out = open_of(&["[questions] 1 q1", "[stamp] tester round 1", "[answers] 1 q1", "[revert] tester round 1"]);
    assert_eq!(out, "1.q1");
}

// ---- Only the issues in the flow (#155) ----

/// The default branch of the repository made by `set_up`.
fn default_branch(folder: &Path) -> String {
    git_text(folder, &["rev-parse", "--abbrev-ref", "HEAD"]).trim().to_string()
}

/// Adds lines to `watchbill.yml`.
fn add_patterns(folder: &Path, lines: &str) {
    let mut file = fs::OpenOptions::new().append(true).open(config(folder)).unwrap();
    file.write_all(lines.as_bytes()).unwrap();
    commit_config(folder);
}

/// Commits a file on the branch, with the checkout back where it was.
fn commit_on(folder: &Path, name: &str, file: &str) {
    let back = default_branch(folder);
    git(folder, &["checkout", "-q", name]);
    put(folder, file, "work\n");
    git(folder, &["add", "--", file]);
    git(folder, &["commit", "-q", "-m", "More work"]);
    git(folder, &["checkout", "-q", &back]);
}

#[test]
fn test_WB_STATUS_32_an_issue_is_listed_when_its_branch_holds_a_commit_the_default_branch_does_not() {
    let folder = set_up();
    comments(&folder.path, "1", &[]);
    branch(&folder.path, "1", true);
    comments(&folder.path, "2", &[]);
    // A branch with nothing that the default branch lacks is not the flow.
    git(&folder.path, &["branch", "issue-2-work"]);
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Jobs in the flow:\n\t1  issue-1-work  accepted\n");
}

#[test]
fn test_WB_STATUS_32_an_issue_that_is_merged_or_deleted_is_not_listed_with_its_file_on_the_default_branch() {
    let folder = set_up();
    for issue in ["41", "42", "43"] {
        comments(&folder.path, issue, &["[stamp] fix round 1"]);
        branch(&folder.path, issue, true);
    }
    // 41 is merged and its branch kept, 42 is merged and its branches deleted, 43 is still open.
    for issue in ["41", "42"] {
        git(&folder.path, &["merge", "-q", "--no-edit", &format!("issue-{}-work", issue)]);
    }
    for gone in ["refs/heads/issue-42-work", "refs/remotes/origin/issue-42-work"] {
        git(&folder.path, &["update-ref", "-d", gone]);
    }
    assert!(folder.path.join(".watchbill/issues/42.yml").exists());
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Jobs in the flow:\n\t43  issue-43-work  fixed\n");
}

#[test]
fn test_WB_STATUS_33_the_issues_are_found_with_git_alone_and_no_pattern() {
    // Nothing but `issue_comments` is in `watchbill.yml` of the repository, and its issues are found.
    let folder = set_up();
    comments(&folder.path, "7", &[]);
    open(&folder.path, &["7"]);
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "");
}

#[test]
fn test_WB_STATUS_34_an_issue_whose_branch_holds_no_issue_file_has_no_line_in_the_list() {
    let folder = set_up();
    comments(&folder.path, "5", &[]);
    open(&folder.path, &["5"]);
    assert_eq!(stdout(&folder.path, &["status"]), (String::new(), Some(0)));
    // Asked by name it is read, as a job of an agent's move with no stage word.
    let (out, _) = stdout(&folder.path, &["status", "5"]);
    assert!(out.starts_with("Jobs in the flow:\n\t5  issue-5-work\n\nWorked out from:\n"), "{}", out);
}

#[test]
fn test_WB_STATUS_35_one_issue_that_is_not_in_the_flow_is_read_and_said_not_to_be() {
    let folder = set_up();
    comments(&folder.path, "41", &["[stamp] build round 1", "[stamp] review round 1", "[stamp] fix round 1"]);
    branch(&folder.path, "41", true);
    git(&folder.path, &["merge", "-q", "--no-edit", "issue-41-work"]);
    git(&folder.path, &["update-ref", "-d", "refs/heads/issue-41-work"]);
    git(&folder.path, &["update-ref", "-d", "refs/remotes/origin/issue-41-work"]);
    let (out, status) = stdout(&folder.path, &["status", "41"]);
    assert_eq!(status, Some(0));
    assert!(out.starts_with("Jobs in the flow:\n\t41  fixed\n\n"), "{}", out);
    assert!(out.contains("\nWorked out from:\n  not in the flow: no branch of the issue holds a commit that "), "{}", out);
    assert!(out.contains("[stamp] fix round 1"), "{}", out);
    // The list does not have it.
    assert_eq!(stdout(&folder.path, &["status"]), (String::new(), Some(0)));
}

#[test]
fn test_WB_STATUS_36_a_line_is_the_number_and_the_branch_and_no_title() {
    let folder = set_up();
    add_patterns(&folder.path, "issue_address: \"cat .tracker/{issue}.address\"\n");
    put(&folder.path, ".tracker/129.address", "Title of the issue\nThe rest\n");
    comments(&folder.path, "129", &[]);
    branch(&folder.path, "129", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Jobs in the flow:\n\t129  issue-129-work  accepted\n");
}

#[test]
fn test_WB_STATUS_37_one_issue_prints_the_first_line_of_issue_address_as_its_title_escaped() {
    let folder = set_up();
    add_patterns(&folder.path, "issue_address: \"cat .tracker/{issue}.address\"\n");
    put(&folder.path, ".tracker/5.address", "Bell \u{7} and \u{1b}[31mred\nThe rest\n");
    comments(&folder.path, "5", &[]);
    open(&folder.path, &["5"]);
    let (out, status) = stdout(&folder.path, &["status", "5"]);
    assert_eq!(status, Some(0));
    assert!(out.starts_with("Jobs in the flow:\n\t5  issue-5-work  Bell \\u{7} and \\u{1b}[31mred\n"), "{}", out);
    assert!(!out.contains("The rest"), "{}", out);
}

#[test]
fn test_WB_STATUS_37_a_failing_issue_address_puts_the_issue_under_not_read() {
    let folder = set_up();
    add_patterns(&folder.path, "issue_address: \"cat .tracker/{issue}.address\"\n");
    comments(&folder.path, "5", &[]);
    open(&folder.path, &["5"]);
    let run = wb(&folder.path, &["status", "5"], "");
    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stdout).contains("Not read:\n  5  `issue_address` of watchbill.yml ended with 1"), "{}", text(&run.stdout));
}

/// The calls a stand-in pattern made, as the lines it wrote, sorted.
fn calls(folder: &Path, name: &str) -> Vec<String> {
    let mut found: Vec<String> = fs::read_to_string(folder.join(".calls").join(name)).map(|text| text.lines().map(str::to_string).collect()).unwrap_or_default();
    found.sort();
    found
}

#[test]
fn test_WB_STATUS_38_it_runs_issue_comments_and_change_of_branch_once_for_each_issue_listed_and_open_issues_never() {
    let folder = set_up();
    let file = config(&folder.path);
    let kept: String = read(&file).lines().filter(|line| !line.starts_with("issue_comments:")).map(|line| format!("{}\n", line)).collect();
    fs::write(&file, kept).unwrap();
    commit_config(&folder.path);
    add_patterns(
        &folder.path,
        "issue_comments: \"mkdir -p .calls && echo {issue} >> .calls/issue_comments && cat .tracker/{issue}.comments\"
open_issues: \"mkdir -p .calls && echo x >> .calls/open_issues\"
change_of_branch: \"mkdir -p .calls && echo {branch} >> .calls/change_of_branch\"
issue_address: \"mkdir -p .calls && echo {issue} >> .calls/issue_address && echo Title\"
issue_comment: \"mkdir -p .calls && echo x >> .calls/issue_comment\"
change_comment: \"mkdir -p .calls && echo x >> .calls/change_comment\"
change_comments: \"mkdir -p .calls && echo x >> .calls/change_comments\"
change_address: \"mkdir -p .calls && echo x >> .calls/change_address\"
",
    );
    for issue in ["1", "2", "3"] {
        comments(&folder.path, issue, &[]);
    }
    open(&folder.path, &["1", "2", "3"]);
    assert_eq!(code(&wb(&folder.path, &["status"], "")), Some(0));
    assert_eq!(calls(&folder.path, "issue_comments"), ["1", "2", "3"]);
    assert_eq!(calls(&folder.path, "change_of_branch"), ["issue-1-work", "issue-2-work", "issue-3-work"]);
    for never in ["open_issues", "issue_address", "issue_comment", "change_comment", "change_comments", "change_address"] {
        assert!(calls(&folder.path, never).is_empty(), "{} was run", never);
    }
    // One issue: `issue_address` once as well.
    fs::remove_dir_all(folder.path.join(".calls")).unwrap();
    assert_eq!(code(&wb(&folder.path, &["status", "2"], "")), Some(0));
    assert_eq!(calls(&folder.path, "issue_comments"), ["2"]);
    assert_eq!(calls(&folder.path, "change_of_branch"), ["issue-2-work"]);
    assert_eq!(calls(&folder.path, "issue_address"), ["2"]);
    assert!(calls(&folder.path, "open_issues").is_empty());
}

#[test]
fn test_WB_STATUS_38_it_runs_issue_comments_once_for_each_issue_with_no_stamp_on_its_ref_and_no_issue_comment_bodies_the_planner_s_time_is_read_from_the_ref() {
    let folder = set_up();
    let file = config(&folder.path);
    let kept: String = read(&file).lines().filter(|line| !line.starts_with("issue_comments:")).map(|line| format!("{}\n", line)).collect();
    fs::write(&file, kept).unwrap();
    commit_config(&folder.path);
    add_patterns(
        &folder.path,
        "issue_comments: \"mkdir -p .calls && echo {issue} >> .calls/issue_comments && cat .tracker/{issue}.comments\"
issue_comment_bodies: \"mkdir -p .calls && echo {issue} >> .calls/issue_comment_bodies && cat .tracker/{issue}.bodies\"
change_of_branch: \"mkdir -p .calls && echo {branch} >> .calls/change_of_branch\"
",
    );
    // 1 is not accepted, 2 is accepted with a planner's stamp before the acceptance, 3 is accepted with none.
    open(&folder.path, &["1"]);
    branch(&folder.path, "2", true);
    branch(&folder.path, "3", true);
    comments(&folder.path, "1", &["[stamp] planner round 1"]);
    comments(&folder.path, "2", &["[stamp] planner round 1"]);
    comments(&folder.path, "3", &[]);
    put(&folder.path, ".tracker/2.bodies", "[stamp] planner round 1\nrole: planner\ntime: 2000-01-01T00:00:00Z\n\0");

    let run = wb(&folder.path, &["status"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("\t2  issue-2-work  accepted\n"), "{}", text(&run.stdout));
    // A job whose ref holds a stamp (1 and 2) is read from its ref, and the tracker is not (#333): only 3 is read.
    assert_eq!(calls(&folder.path, "issue_comments"), ["3"]);
    // Since #236 the planner's stamps and their times are read from the job's ref: no whole comment is read.
    assert!(calls(&folder.path, "issue_comment_bodies").is_empty(), "{:?}", calls(&folder.path, "issue_comment_bodies"));
}

#[test]
fn test_WB_STATUS_39_the_default_branch_is_that_of_origin_else_main_else_master_and_none_ends_with_2() {
    let folder = set_up();
    comments(&folder.path, "1", &[]);
    open(&folder.path, &["1"]);
    let own = default_branch(&folder.path);
    // `main` or `master`, whichever there is, is found with no `origin/HEAD`.
    assert_eq!(stdout(&folder.path, &["status"]), ("".to_string(), Some(0)));
    // `origin/HEAD` wins: here it names a branch that holds the issue's commit, so the issue is done.
    git(&folder.path, &["update-ref", "refs/remotes/origin/trunk", "issue-1-work"]);
    git(&folder.path, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/trunk"]);
    assert_eq!(stdout(&folder.path, &["status"]), (String::new(), Some(0)));
    git(&folder.path, &["symbolic-ref", "--delete", "refs/remotes/origin/HEAD"]);
    git(&folder.path, &["update-ref", "-d", "refs/remotes/origin/trunk"]);
    // No default branch: the name is neither of them.
    git(&folder.path, &["branch", "-q", "-m", &own, "trunk"]);
    let run = wb(&folder.path, &["status"], "");
    assert_eq!(code(&run), Some(2));
    assert!(text(&run.stdout).is_empty(), "{}", text(&run.stdout));
    assert!(text(&run.stderr).starts_with("wb status:") && text(&run.stderr).contains("default branch"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_STATUS_40_a_local_branch_ahead_of_origin_shows_how_many_commits_are_not_pushed() {
    let folder = set_up();
    for issue in ["1", "2", "3"] {
        comments(&folder.path, issue, &[]);
        branch(&folder.path, issue, true);
    }
    // 1 is two commits ahead of its branch at origin, 2 is level, 3 is at origin only, and 4 has no branch at origin.
    commit_on(&folder.path, "issue-1-work", "a.txt");
    commit_on(&folder.path, "issue-1-work", "b.txt");
    git(&folder.path, &["update-ref", "-d", "refs/heads/issue-3-work"]);
    comments(&folder.path, "4", &[]);
    branch(&folder.path, "4", true);
    git(&folder.path, &["update-ref", "-d", "refs/remotes/origin/issue-4-work"]);
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Jobs in the flow:\n\t1  issue-1-work  accepted, 2 not pushed\n\t2  issue-2-work  accepted\n\t3  issue-3-work  accepted\n\t4  issue-4-work  accepted, 1 not pushed\n");
}

#[test]
fn test_WB_STATUS_41_it_reads_the_refs_as_they_are_and_fetches_nothing() {
    let folder = set_up();
    // `origin` is a place that is not there: a fetch would fail and say so.
    git(&folder.path, &["remote", "add", "origin", "/nonexistent/origin.git"]);
    comments(&folder.path, "1", &[]);
    open(&folder.path, &["1"]);
    let run = wb(&folder.path, &["status"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stderr).is_empty(), "{}", text(&run.stderr));
    assert!(!folder.path.join(".git/FETCH_HEAD").exists());
}

#[test]
fn test_WB_STATUS_42_the_line_shows_the_change_that_change_of_branch_prints() {
    let folder = set_up();
    add_patterns(&folder.path, "change_of_branch: \"cat .tracker/{branch}.change\"\n");
    put(&folder.path, ".tracker/issue-129-work.change", "178\n");
    put(&folder.path, ".tracker/issue-130-work.change", "");
    for issue in ["129", "130", "131"] {
        comments(&folder.path, issue, &[]);
        branch(&folder.path, issue, true);
    }
    commit_on(&folder.path, "issue-129-work", "a.txt");
    let run = wb(&folder.path, &["status"], "");
    let out = text(&run.stdout);
    // 131 has no file for its `change_of_branch`: it is not read.
    assert_eq!(code(&run), Some(1));
    assert!(out.starts_with("Jobs in the flow:\n\t129  issue-129-work  #178  accepted, 1 not pushed\n\t130  issue-130-work        accepted\n"), "{}", out);
    assert!(out.contains("\nNot read:\n  131  `change_of_branch` of watchbill.yml ended with 1\n"), "{}", out);
    assert!(!out.contains("131  issue-131-work"), "{}", out);
}

#[test]
fn test_WB_STATUS_42_an_open_change_does_not_decide_the_step() {
    let folder = set_up();
    add_patterns(&folder.path, "change_of_branch: \"echo 7\"\n");
    comments(&folder.path, "3", &["[stamp] build round 1"]);
    branch(&folder.path, "3", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Jobs in the flow:\n\t3  issue-3-work  #7  built\n");
}

// ---- An issue that is done under squash merges (#155, the owner's decision of 2026-11-20) ----

/// Squash-merges the issue's branch into the checked-out default branch, and keeps the branch.
fn squash(folder: &Path, issue: &str) {
    git(folder, &["merge", "-q", "--squash", &format!("issue-{}-work", issue)]);
    git(folder, &["commit", "-q", "-m", &format!("Issue {} (#{})", issue, issue)]);
}

#[test]
fn test_WB_STATUS_32_a_branch_whose_whole_change_is_a_squash_commit_on_the_default_branch_is_done() {
    let folder = set_up();
    for issue in ["41", "42", "43"] {
        comments(&folder.path, issue, &[]);
        branch(&folder.path, issue, true);
    }
    // 41 is squashed and its branches kept, 42 is squashed and then has one more commit, 43 is not merged.
    squash(&folder.path, "41");
    squash(&folder.path, "42");
    commit_on(&folder.path, "issue-42-work", "later.txt");
    at_origin(&folder.path, "issue-42-work");
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Jobs in the flow:\n\t42  issue-42-work  accepted\n\t43  issue-43-work  accepted\n");
}

#[test]
fn test_WB_STATUS_32_a_squash_with_another_diff_than_the_branchs_is_not_found_and_the_issue_stays() {
    let folder = set_up();
    comments(&folder.path, "41", &[]);
    branch(&folder.path, "41", true);
    // The squash commit holds one more file than the branch: its change is not the branch's.
    git(&folder.path, &["merge", "-q", "--squash", "issue-41-work"]);
    put(&folder.path, "extra.txt", "edited in the web page\n");
    git(&folder.path, &["add", "--", "extra.txt"]);
    git(&folder.path, &["commit", "-q", "-m", "Issue 41 (#41)"]);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert_eq!(out, "Jobs in the flow:\n\t41  issue-41-work  accepted\n");
}

#[test]
fn test_WB_STATUS_32_a_local_branch_whose_upstream_is_gone_is_done() {
    let folder = set_up();
    for issue in ["5", "6", "7"] {
        comments(&folder.path, issue, &[]);
        branch(&folder.path, issue, true);
    }
    git(&folder.path, &["remote", "add", "origin", "/nonexistent/origin.git"]);
    // 5 and 6 follow a branch at origin; at 5 it was deleted (as after `git fetch --prune`), 6 still has it, 7 follows none.
    for issue in ["5", "6"] {
        let name = format!("issue-{}-work", issue);
        git(&folder.path, &["config", &format!("branch.{}.remote", name), "origin"]);
        git(&folder.path, &["config", &format!("branch.{}.merge", name), &format!("refs/heads/{}", name)]);
    }
    git(&folder.path, &["update-ref", "-d", "refs/remotes/origin/issue-5-work"]);
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Jobs in the flow:\n\t6  issue-6-work  accepted\n\t7  issue-7-work  accepted\n");
}

#[test]
fn test_WB_STATUS_32_a_refs_listing_that_fails_ends_with_2_and_lists_nothing() {
    let folder = set_up();
    comments(&folder.path, "1", &[]);
    open(&folder.path, &["1"]);
    fs::write(folder.path.join(".git/packed-refs"), "garbage\n").unwrap();
    let run = wb(&folder.path, &["status"], "");
    assert_eq!(code(&run), Some(2), "{}", text(&run.stdout));
    assert!(text(&run.stdout).is_empty(), "{}", text(&run.stdout));
    assert!(text(&run.stderr).starts_with("wb status:"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_STATUS_37_a_malformed_issue_address_does_not_fail_the_list_that_never_runs_it() {
    let folder = set_up();
    add_patterns(&folder.path, "issue_address: [one, two]\n");
    comments(&folder.path, "1", &[]);
    open(&folder.path, &["1"]);
    let (out, status) = stdout(&folder.path, &["status"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "");
}

// ---- WB-STATUS-53 to 55: who the newest answer waits for (#226) ----

#[test]
fn test_WB_STATUS_53_one_issue_says_which_role_the_newest_answer_waits_for() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[stamp] build round 1", "[questions] 54.build q3 q4", "[answers] 54.build q3>tester q4>build"]);
    branch(&folder.path, "54", true);
    let (out, status) = stdout(&folder.path, &["status", "54"]);
    assert_eq!(status, Some(0));
    assert!(out.contains("\nwaits for: build (54.build.q4)\n"), "{}", out);
    assert!(!out.contains("waits for: tester"), "{}", out);
}

#[test]
fn test_WB_STATUS_53_the_newest_answer_that_names_a_role_decides() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(
        &folder.path,
        "54",
        &["[stamp] build round 1", "[questions] 54.build q3 q4", "[answers] 54.build q3>tester", "[answers] 54.build q4"],
    );
    branch(&folder.path, "54", true);
    let (out, _) = stdout(&folder.path, &["status", "54"]);
    assert!(out.contains("\nwaits for: tester (54.build.q3)\n"), "{}", out);
}

#[test]
fn test_WB_STATUS_53_an_answer_for_a_person_says_person() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[stamp] build round 1", "[questions] 54.build q3", "[answers] 54.build q3>person"]);
    branch(&folder.path, "54", true);
    let (out, _) = stdout(&folder.path, &["status", "54"]);
    assert!(out.contains("\nwaits for: person (54.build.q3)\n"), "{}", out);
}

#[test]
fn test_WB_STATUS_53_an_answer_of_the_planner_names_its_question_without_a_role_in_the_name() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[questions] 54 q1", "[answers] 54 q1>planner"]);
    let (out, _) = stdout(&folder.path, &["status", "54"]);
    assert!(out.contains("\nwaits for: planner (54.q1)\n"), "{}", out);
}

#[test]
fn test_WB_STATUS_53_the_list_prints_no_waits_for_line() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[stamp] build round 1", "[questions] 54.build q3", "[answers] 54.build q3>tester"]);
    branch(&folder.path, "54", true);
    let (out, _) = stdout(&folder.path, &["status"]);
    assert!(!out.contains("waits for"), "{}", out);
}

#[test]
fn test_WB_STATUS_54_an_answer_of_0_10_0_closes_its_question_and_names_no_role() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[stamp] build round 1", "[questions] 54.build q3", "[answers] 54.build q3"]);
    branch(&folder.path, "54", true);
    let (out, status) = stdout(&folder.path, &["status", "54"]);
    assert_eq!(status, Some(0));
    assert!(!out.contains("waits for"), "{}", out);
    assert!(!out.contains("questions open"), "{}", out);
}

#[test]
fn test_WB_STATUS_55_the_comments_are_read_once_for_each_issue_with_the_role_too() {
    let folder = set_up_with("issue_comments: \"echo run >> .tracker/count; cat .tracker/{issue}.comments\"\n");
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[stamp] build round 1", "[questions] 54.build q3", "[answers] 54.build q3>tester"]);
    branch(&folder.path, "54", true);
    let (out, status) = stdout(&folder.path, &["status", "54"]);
    assert_eq!(status, Some(0));
    assert!(out.contains("waits for: tester (54.build.q3)"), "{}", out);
    assert_eq!(fs::read_to_string(folder.path.join(".tracker/count")).unwrap().lines().count(), 1);
}

#[test]
fn test_WB_STATUS_56_a_stamp_of_the_role_newer_than_the_answer_clears_waits_for() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[stamp] build round 1", "[questions] 54.build q3", "[answers] 54.build q3>tester", "[stamp] tester round 2"]);
    branch(&folder.path, "54", true);
    let (out, _) = stdout(&folder.path, &["status", "54"]);
    assert!(!out.contains("waits for"), "{}", out);
}

#[test]
fn test_WB_STATUS_56_a_stamp_of_another_role_or_before_the_answer_does_not() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[stamp] tester round 1", "[questions] 54.build q3", "[answers] 54.build q3>tester", "[stamp] build round 2"]);
    branch(&folder.path, "54", true);
    let (out, _) = stdout(&folder.path, &["status", "54"]);
    assert!(out.contains("\nwaits for: tester (54.build.q3)\n"), "{}", out);
}

#[test]
fn test_WB_STATUS_56_person_is_never_cleared_by_a_stamp() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[questions] 54.build q3", "[answers] 54.build q3>person", "[stamp] build round 2", "[stamp] tester round 2"]);
    branch(&folder.path, "54", true);
    let (out, _) = stdout(&folder.path, &["status", "54"]);
    assert!(out.contains("\nwaits for: person (54.build.q3)\n"), "{}", out);
}

#[test]
fn test_WB_STATUS_57_a_word_that_is_no_role_closes_the_question_and_names_none() {
    let folder = set_up();
    open(&folder.path, &["54"]);
    comments(&folder.path, "54", &["[stamp] build round 1", "[questions] 54.build q3", "[answers] 54.build q3>optional"]);
    branch(&folder.path, "54", true);
    let (out, _) = stdout(&folder.path, &["status", "54"]);
    assert!(!out.contains("waits for"), "{}", out);
    assert!(!out.contains("questions open"), "{}", out);
}

// ---- #300: a stage only where its work came back, and a round that brought nothing back is said ----

/// A folder of the lines of one start, as `wb start` leaves it, below `state`: its stamp, its outcome, and `back` where
/// one is given.
fn session(state: &Path, role: &str, issue: &str, round: u32, outcome: &str, back: Option<&str>) {
    let folder = state.join("watchbill/sessions").join(format!("2026-11-24T10{:02}00Z-{}-{}", round, role, issue));
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("stamp"), format!("[stamp] {} round {}\n", role, round)).unwrap();
    fs::write(folder.join("outcome"), outcome).unwrap();
    if let Some(back) = back {
        fs::write(folder.join("back"), back).unwrap();
    }
}

const ENDED: &str = "outcome: ended\nstatus: 0\nseconds: 5\n";
const TAKEN: &str = "taken: 1a2b3c4 Write the tests\nview: removed\n";
const NOTHING: &str = "view: removed\n";

/// `wb status` with the state folder `state`.
fn status_in(folder: &Path, state: &Path, args: &[&str]) -> (String, Option<i32>) {
    let run = Command::new(env!("CARGO_BIN_EXE_wb"))
        .arg("status")
        .args(args)
        .current_dir(folder)
        .env_clear()
        .envs(env().into_iter().filter(|(name, _)| name != "XDG_STATE_HOME"))
        .env("XDG_STATE_HOME", state)
        .stdin(Stdio::null())
        .output()
        .expect("wb could not be started");
    (text(&run.stdout), code(&run))
}

/// Issue 3 with a tester round and a build round stamped, and a state folder.
fn stamped_3() -> (Folder, Folder) {
    let folder = set_up();
    open(&folder.path, &["3"]);
    comments(&folder.path, "3", &["[stamp] tester round 1", "[stamp] build round 1"]);
    branch(&folder.path, "3", true);
    (folder, common::folder())
}

#[test]
fn test_WB_STATUS_79_a_build_round_that_brought_nothing_back_leaves_the_issue_at_tested() {
    let (folder, state) = stamped_3();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "build", "3", 1, ENDED, Some(NOTHING));
    let (out, status) = status_in(&folder.path, &state.path, &[]);
    assert_eq!(status, Some(0));
    assert_eq!(out, "Jobs in the flow:\n\t3  issue-3-work  tested\n");
}

#[test]
fn test_WB_STATUS_79_a_tester_round_that_brought_nothing_back_leaves_the_issue_at_accepted() {
    let (folder, state) = stamped_3();
    comments(&folder.path, "3", &["[stamp] tester round 1"]);
    session(&state.path, "tester", "3", 1, ENDED, Some("failed: the branch moved\nview: kept /somewhere\n"));
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    assert_eq!(out, "Jobs in the flow:\n\t3  issue-3-work  accepted\n");
}

#[test]
fn test_WB_STATUS_80_a_build_round_that_brought_a_commit_back_is_built() {
    let (folder, state) = stamped_3();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "build", "3", 1, ENDED, Some(TAKEN));
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    assert_eq!(out, "Jobs in the flow:\n\t3  issue-3-work  built\n");
}

#[test]
fn test_WB_STATUS_80_a_take_back_later_that_took_a_commit_counts_for_its_round() {
    let (folder, state) = stamped_3();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "build", "3", 1, ENDED, Some("not taken: 1a2b3c4 Build it\nview: kept /somewhere\ntake-back\ntaken: 5d6e7f8 Build it\nview: removed\n"));
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    assert_eq!(out, "Jobs in the flow:\n\t3  issue-3-work  built\n");
}

#[test]
fn test_WB_STATUS_81_a_round_with_no_record_on_this_machine_counts_by_its_stamp() {
    let (folder, state) = stamped_3();
    // No folder of a start at all.
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    assert_eq!(out, "Jobs in the flow:\n\t3  issue-3-work  built\n");
    // A folder with no `back`.
    session(&state.path, "build", "3", 1, "outcome: ended\nstatus: 1\nseconds: 5\n", None);
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    assert_eq!(out, "Jobs in the flow:\n\t3  issue-3-work  built\n");
    // A session that still runs: its process is one that is read as `wb` (a copy of `sleep` named `wb`).
    let program = state.path.join("wb");
    let mut running = stand_in("/usr/bin/sleep", &program, &["60"]);
    fs::write(state.path.join("watchbill/sessions/2026-11-24T100100Z-build-3/outcome"), format!("outcome: running\npid: {}\n", running.id())).unwrap();
    fs::write(state.path.join("watchbill/sessions/2026-11-24T100100Z-build-3/back"), NOTHING).unwrap();
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    let _ = running.kill();
    let _ = running.wait();
    assert_eq!(out, "Jobs in the flow:\n\t3  issue-3-work  built\n");
}

#[test]
fn test_WB_STATUS_81_a_start_of_another_issue_or_another_round_says_nothing_of_this_one() {
    let (folder, state) = stamped_3();
    session(&state.path, "build", "33", 1, ENDED, Some(NOTHING));
    session(&state.path, "build", "3", 2, ENDED, Some(NOTHING));
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    assert_eq!(out, "Jobs in the flow:\n\t3  issue-3-work  built\n");
}

#[test]
fn test_WB_STATUS_82_one_issue_says_the_round_that_brought_nothing_back() {
    let (folder, state) = stamped_3();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "build", "3", 1, ENDED, Some(NOTHING));
    let (out, _) = status_in(&folder.path, &state.path, &["3"]);
    assert!(out.starts_with("Jobs in the flow:\n\t3  issue-3-work  tested\n"), "{}", out);
    assert!(out.lines().any(|line| line == "build round 1: no commit came back"), "{}", out);
    assert!(!out.contains("tester round 1: no commit"), "{}", out);
    // The list says nothing of it.
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    assert!(!out.contains("no commit came back"), "{}", out);
}

#[test]
fn test_WB_STATUS_83_a_round_with_work_kept_in_a_view_names_the_view_beside_it() {
    let (folder, state) = stamped_3();
    let view = state.path.join("watchbill/views/view-1");
    fs::create_dir_all(&view).unwrap();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "build", "3", 1, ENDED, Some(&format!("uncommitted: src/a.c\nuncommitted: src/b.c\nview: kept {}\n", view.display())));
    let (out, _) = status_in(&folder.path, &state.path, &["3"]);
    let line = format!("build round 1: no commit came back; kept: 2 paths uncommitted, {}", view.display());
    assert!(out.lines().any(|one| one == line), "{}", out);
}

#[test]
fn test_WB_STATUS_84_a_review_round_is_not_held_to_a_commit() {
    let folder = set_up();
    open(&folder.path, &["3"]);
    comments(&folder.path, "3", &["[stamp] tester round 1", "[stamp] build round 1", "[stamp] review round 1"]);
    branch(&folder.path, "3", true);
    let state = common::folder();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "build", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "review", "3", 1, ENDED, Some(NOTHING));
    let (out, _) = status_in(&folder.path, &state.path, &[]);
    assert_eq!(out, "Jobs in the flow:\n\t3  issue-3-work  reviewed\n");
    let (out, _) = status_in(&folder.path, &state.path, &["3"]);
    assert!(!out.contains("no commit came back"), "{}", out);
}

#[test]
fn test_WB_STATUS_82_a_round_before_the_last_planner_stamp_is_not_said() {
    let folder = set_up();
    open(&folder.path, &["3"]);
    comments(&folder.path, "3", &["[stamp] build round 1", "[stamp] planner round 2", "[stamp] tester round 1"]);
    branch(&folder.path, "3", true);
    let state = common::folder();
    session(&state.path, "build", "3", 1, ENDED, Some(NOTHING));
    session(&state.path, "tester", "3", 1, ENDED, Some(NOTHING));
    let (out, _) = status_in(&folder.path, &state.path, &["3"]);
    assert!(!out.contains("build round 1: no commit came back"), "{}", out);
    assert!(out.contains("tester round 1: no commit came back"), "{}", out);
}

#[test]
fn test_WB_STATUS_82_a_round_that_a_revert_took_back_is_not_said() {
    let folder = set_up();
    open(&folder.path, &["3"]);
    comments(&folder.path, "3", &["[stamp] tester round 1", "[stamp] build round 1", "[revert] build round 1"]);
    branch(&folder.path, "3", true);
    let state = common::folder();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "build", "3", 1, ENDED, Some(NOTHING));
    let (out, _) = status_in(&folder.path, &state.path, &["3"]);
    assert!(!out.contains("no commit came back"), "{}", out);
}

#[test]
fn test_WB_STATUS_83_a_view_named_beside_its_round_is_not_named_again_on_a_kept_line() {
    let (folder, state) = stamped_3();
    let view = state.path.join("watchbill/views/view-1");
    fs::create_dir_all(&view).unwrap();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "build", "3", 1, ENDED, Some(&format!("uncommitted: src/a.c\nview: kept {}\n", view.display())));
    let (out, _) = status_in(&folder.path, &state.path, &["3"]);
    assert_eq!(out.matches(&view.display().to_string()).count(), 1, "{}", out);
    assert!(!out.contains("kept: build round 1"), "{}", out);
}

// ---- #284: `wb status` as `git status`: by whose move, coloured, with git's short and porcelain forms ----

/// `wb status` run in a home of its own: its `HOME`, `XDG_CONFIG_HOME` and `GIT_CONFIG_GLOBAL` are the test's, and no
/// terminal is on standard output, as with a pipe.
fn run_status(folder: &Path, args: &[&str]) -> std::process::Output {
    run_status_in(folder, None, args)
}

fn run_status_in(folder: &Path, state: Option<&Path>, args: &[&str]) -> std::process::Output {
    let home = common::folder_named("home");
    let mut command = Command::new(env!("CARGO_BIN_EXE_wb"));
    command
        .arg("status")
        .args(args)
        .current_dir(folder)
        .env_clear()
        .envs(env().into_iter().filter(|(name, _)| name != "HOME" && name != "XDG_CONFIG_HOME" && name != "XDG_STATE_HOME"))
        .env("HOME", &home.path)
        .env("XDG_CONFIG_HOME", home.path.join(".config"))
        .env("GIT_CONFIG_GLOBAL", home.path.join("gitconfig"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("XDG_STATE_HOME", state.unwrap_or(Path::new(NO_STATE)))
        .env_remove("NO_COLOR")
        .stdin(Stdio::null());
    command.output().expect("wb could not be started")
}

fn said(folder: &Path, args: &[&str]) -> (String, Option<i32>) {
    let run = run_status(folder, args);
    (text(&run.stdout), code(&run))
}

/// The mock-up of the issue: 176 has a question open, 184 is planned and waits for its acceptance, and the others are
/// an agent's move.
fn mock_up() -> Folder {
    let folder = set_up();
    open(&folder.path, &["175", "176", "177", "179", "183", "184"]);
    comments(&folder.path, "175", &[]);
    comments(&folder.path, "176", &["[stamp] build round 1"]);
    stamps_on_ref(&folder.path, "176", &["[questions] 176.build q1\n\nq1: Which one?\n"]);
    comments(&folder.path, "177", &["[stamp] tester round 1", "[stamp] build round 1", "[stamp] review round 1"]);
    comments(&folder.path, "179", &["[stamp] tester round 1", "[stamp] build round 1"]);
    comments(&folder.path, "183", &["[stamp] tester round 1"]);
    comments(&folder.path, "184", &[]);
    for issue in ["175", "176", "177", "179", "183"] {
        branch(&folder.path, issue, true);
    }
    branch(&folder.path, "184", false);
    folder
}

const WAITING: &str = "Jobs waiting for you:\n  (use \"wb plan accept <name>...\" to accept the plan)\n  (use \"wb answer <issue>...\" to answer its questions)\n\t184  issue-184-work  planned\n\t176  issue-176-work  built, 1 question open\n";
const FLOW: &str = "Jobs in the flow:\n\t175  issue-175-work  accepted\n\t183  issue-183-work  tested\n\t179  issue-179-work  built\n\t177  issue-177-work  reviewed\n";

#[test]
fn test_WB_STATUS_103_jobs_are_under_the_heading_of_whose_move_it_is() {
    let folder = mock_up();
    let (out, status) = said(&folder.path, &[]);
    assert_eq!(status, Some(0));
    assert_eq!(out, format!("{}\n{}", WAITING, FLOW));
}

#[test]
fn test_WB_STATUS_103_a_heading_with_no_job_is_not_printed() {
    let folder = set_up();
    open(&folder.path, &["3", "4"]);
    comments(&folder.path, "3", &[]);
    comments(&folder.path, "4", &["[stamp] tester round 1"]);
    branch(&folder.path, "3", true);
    branch(&folder.path, "4", true);
    let (out, _) = said(&folder.path, &[]);
    assert_eq!(out, "Jobs in the flow:\n\t3  issue-3-work  accepted\n\t4  issue-4-work  tested\n");
    let folder = set_up();
    open(&folder.path, &["3"]);
    comments(&folder.path, "3", &[]);
    branch(&folder.path, "3", false);
    let (out, _) = said(&folder.path, &[]);
    assert_eq!(out, "Jobs waiting for you:\n  (use \"wb plan accept <name>...\" to accept the plan)\n\t3  issue-3-work  planned\n");
}

#[test]
fn test_WB_STATUS_104_each_line_ends_with_the_stage_word_and_no_letter_stands_for_it() {
    let folder = at_every_step();
    let (out, status) = said(&folder.path, &[]);
    assert_eq!(status, Some(0));
    for (issue, word) in [("2", "planned"), ("3", "accepted"), ("4", "tested"), ("5", "built"), ("6", "reviewed"), ("7", "fixed")] {
        let line = format!("\t{}  issue-{}-work  {}\n", issue, issue, word);
        assert!(out.contains(&line), "{:?} not in {}", line, out);
    }
    // The old words of the steps are no heading any more.
    for old in ["Planned:", "Accepted:", "Tested:", "Built:", "Reviewed:", "Fixed:", "Not planned:", "Questions open:", "Needs a person:"] {
        assert!(!out.contains(old), "{} in {}", old, out);
    }
}

#[test]
fn test_WB_STATUS_105_a_job_not_broken_down_is_not_listed() {
    let folder = set_up();
    open(&folder.path, &["1"]);
    comments(&folder.path, "1", &[]);
    let run = run_status(&folder.path, &[]);
    assert_eq!(code(&run), Some(0));
    assert_eq!(text(&run.stdout), "");
    assert_eq!(said(&folder.path, &["--short"]).0, "");
    assert_eq!(said(&folder.path, &["--porcelain"]).0, "");
}

#[test]
fn test_WB_STATUS_105_a_job_not_broken_down_that_waits_for_a_person_is_listed_with_no_stage_word() {
    let folder = set_up();
    open(&folder.path, &["178"]);
    comments(&folder.path, "178", &["[question] 178.q1", "[question] 178.q2"]);
    let (out, _) = said(&folder.path, &[]);
    assert_eq!(out, "Jobs waiting for you:\n  (use \"wb answer <issue>...\" to answer its questions)\n\t178  issue-178-work  2 questions open\n");
}

#[test]
fn test_WB_STATUS_106_the_hints_are_in_gits_form_and_only_for_what_a_job_of_the_heading_can_do() {
    let folder = set_up();
    open(&folder.path, &["5"]);
    comments(&folder.path, "5", &["[question] 5.q1"]);
    let (out, _) = said(&folder.path, &[]);
    assert!(out.contains("\n  (use \"wb answer <issue>...\" to answer its questions)\n"), "{}", out);
    assert!(!out.contains("wb plan accept"), "{}", out);
    // Every hint line is two spaces, a bracket, `use "...", to ...` and a bracket; the flow has none.
    let folder = mock_up();
    let (out, _) = said(&folder.path, &[]);
    for line in out.lines().filter(|line| line.contains("(use ")) {
        assert!(line.starts_with("  (use \"wb ") && line.ends_with(')') && line.contains("\" to "), "{}", line);
    }
    let flow = out.split("Jobs in the flow:").nth(1).unwrap();
    assert!(!flow.contains("(use"), "{}", out);
}

#[test]
fn test_WB_STATUS_107_status_hints_false_in_watchbill_yml_turns_the_hints_off() {
    let folder = mock_up();
    add_patterns(&folder.path, "status_hints: false\n");
    let (out, status) = said(&folder.path, &[]);
    assert_eq!(status, Some(0));
    assert!(!out.contains("(use"), "{}", out);
    assert!(out.contains("\t176  issue-176-work  built, 1 question open\n"), "{}", out);
    assert_eq!(said(&folder.path, &["176"]).0.contains("(use"), false);
}

#[test]
fn test_WB_STATUS_107_status_hints_true_keeps_them_and_a_value_that_is_no_boolean_is_refused() {
    let folder = mock_up();
    add_patterns(&folder.path, "status_hints: true\n");
    assert_eq!(said(&folder.path, &[]).0, format!("{}\n{}", WAITING, FLOW));
    let folder = mock_up();
    add_patterns(&folder.path, "status_hints: sometimes\n");
    // A value that is no boolean ends with 1, as every value of `watchbill.yml` that the loader refuses, in every form.
    for args in [&[][..], &["-s"], &["--porcelain"]] {
        let run = run_status(&folder.path, args);
        assert_eq!(code(&run), Some(1), "{:?}", args);
        assert!(text(&run.stdout).is_empty());
        assert!(text(&run.stderr).contains("status_hints"), "{}", text(&run.stderr));
    }
    // `wb check` names it.
    let check = wb(&folder.path, &["check"], "");
    assert_eq!(code(&check), Some(1), "{}", text(&check.stdout));
    assert!(text(&check.stdout).contains("status_hints") || text(&check.stderr).contains("status_hints"), "{}{}", text(&check.stdout), text(&check.stderr));
    // A word that YAML may read as a boolean is refused too: only `true` and `false` are written.
    let folder = mock_up();
    add_patterns(&folder.path, "status_hints: no\n");
    assert_eq!(code(&run_status(&folder.path, &[])), Some(1));
}

#[test]
fn test_WB_STATUS_108_colour_is_red_for_waiting_and_green_for_the_flow_as_git_colours_what_is_not_staged_and_what_is() {
    let folder = mock_up();
    let (out, status) = said(&folder.path, &["--color=always"]);
    assert_eq!(status, Some(0));
    let expected = format!("{}\n{}", WAITING, FLOW);
    let red = |line: &str| format!("\t\x1b[31m{}\x1b[m\n", line.trim_start_matches('\t'));
    let green = |line: &str| format!("\t\x1b[32m{}\x1b[m\n", line.trim_start_matches('\t'));
    let mut want = expected.clone();
    for line in expected.lines().filter(|line| line.starts_with('\t')) {
        let coloured = if line.contains("176") || line.contains("184") { red(line) } else { green(line) };
        want = want.replace(&format!("{}\n", line), &coloured);
    }
    assert_eq!(out, want);
    // The headings and the hints are not coloured: whose move it is is said by the heading too.
    assert!(out.contains("Jobs waiting for you:\n  (use "), "{:?}", out);
}

#[test]
fn test_WB_STATUS_108_there_is_no_colour_in_a_pipe_nor_with_never_and_color_alone_is_always() {
    let folder = mock_up();
    let plain = format!("{}\n{}", WAITING, FLOW);
    assert_eq!(said(&folder.path, &[]).0, plain);
    assert_eq!(said(&folder.path, &["--color=auto"]).0, plain);
    assert_eq!(said(&folder.path, &["--color=never"]).0, plain);
    assert!(said(&folder.path, &["--color"]).0.contains("\x1b[32m"));
    assert_eq!(said(&folder.path, &["--color=always"]).0, said(&folder.path, &["--color"]).0);
    let run = run_status(&folder.path, &["--color=sometimes"]);
    assert_eq!(code(&run), Some(2));
    assert!(text(&run.stderr).contains("sometimes"), "{}", text(&run.stderr));
    assert_eq!(code(&run_status(&folder.path, &["--color=never", "--color=always"])), Some(2));
}

const SHORT: &str = "A  175 issue-175-work\nBQ 176 issue-176-work\nR  177 issue-177-work\nB  179 issue-179-work\nT  183 issue-183-work\nPP 184 issue-184-work\n";

#[test]
fn test_WB_STATUS_109_short_is_one_line_a_job_with_two_columns_as_git_has_them() {
    let folder = mock_up();
    for flag in ["--short", "-s"] {
        let (out, status) = said(&folder.path, &[flag]);
        assert_eq!(status, Some(0), "{}", flag);
        assert_eq!(out, SHORT, "{}", flag);
    }
    // Coloured as git does: the first column green, the second red, and a blank column not at all.
    let (out, _) = said(&folder.path, &["-s", "--color=always"]);
    assert!(out.contains("\x1b[32mA\x1b[m  175 issue-175-work\n"), "{:?}", out);
    assert!(out.contains("\x1b[32mB\x1b[m\x1b[31mQ\x1b[m 176 issue-176-work\n"), "{:?}", out);
    assert!(out.contains("\x1b[32mP\x1b[m\x1b[31mP\x1b[m 184 issue-184-work\n"), "{:?}", out);
    // No hint and no heading.
    assert!(!said(&folder.path, &["-s"]).0.contains("(use"));
}

#[test]
fn test_WB_STATUS_110_porcelain_is_the_same_fixed_lines_with_no_colour_and_no_hint() {
    let folder = mock_up();
    let (out, status) = said(&folder.path, &["--porcelain"]);
    assert_eq!(status, Some(0));
    assert_eq!(out, SHORT);
    assert_eq!(said(&folder.path, &["--porcelain", "--color=always"]).0, SHORT);
    // Of `--short` and `--porcelain` the one given last wins, as in git.
    assert_eq!(said(&folder.path, &["--short", "--porcelain", "--color=always"]).0, SHORT);
    assert_eq!(said(&folder.path, &["-s", "--porcelain", "--color=always"]).0, SHORT);
    let coloured = said(&folder.path, &["--porcelain", "-s", "--color=always"]).0;
    assert!(coloured.contains("\x1b[32mA\x1b[m  175 issue-175-work\n"), "{:?}", coloured);
    assert_eq!(said(&folder.path, &["--porcelain", "--short", "--color=never"]).0, SHORT);
    add_patterns(&folder.path, "status_hints: true\n");
    assert_eq!(said(&folder.path, &["--porcelain"]).0, SHORT);
}

#[test]
fn test_WB_STATUS_111_short_and_porcelain_of_one_issue_and_of_a_stage_print_the_lines_that_match() {
    let folder = mock_up();
    assert_eq!(said(&folder.path, &["-s", "176"]).0, "BQ 176 issue-176-work\n");
    assert_eq!(said(&folder.path, &["--porcelain", "184"]).0, "PP 184 issue-184-work\n");
    assert_eq!(said(&folder.path, &["-s", "--stage", "built"]).0, "BQ 176 issue-176-work\nB  179 issue-179-work\n");
    // A job that is not in the list has no line, as a clean path has none in git.
    comments(&folder.path, "999", &[]);
    let (out, status) = said(&folder.path, &["-s", "999"]);
    assert_eq!((out.as_str(), status), ("", Some(0)));
}

#[test]
fn test_WB_STATUS_112_one_issue_lists_its_acceptances_and_each_session_from_its_stamp() {
    let folder = mock_up();
    let (out, status) = said(&folder.path, &["179"]);
    assert_eq!(status, Some(0));
    assert!(out.contains("\naccepted: the issue's file "), "{}", out);
    let tip = git_text(&folder.path, &["rev-list", "--reverse", "refs/watchbill/179"]);
    let stamps: Vec<&str> = tip.lines().collect();
    let first: String = stamps[stamps.len() - 2].chars().take(12).collect();
    let second: String = stamps[stamps.len() - 1].chars().take(12).collect();
    let want = format!(
        "Sessions:\n  tester round 1, 2000-01-01T00:00:00Z, {}\n    ended: not known on this machine\n    came back: not known on this machine\n  build round 1, 2000-01-01T00:00:00Z, {}\n    ended: not known on this machine\n    came back: not known on this machine\n",
        first, second
    );
    assert!(out.contains(&want), "{:?} not in {}", want, out);
    // The list names no session.
    assert!(!said(&folder.path, &[]).0.contains("Sessions"));
}

#[test]
fn test_WB_STATUS_112_a_session_says_how_it_ended_from_the_lines_of_its_start_on_this_machine() {
    let (folder, state) = stamped_3();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    // The start's own copy of the stamp is its whole message, as the commit has it.
    let first = git_text(&folder.path, &["rev-list", "--reverse", "refs/watchbill/3"]).lines().next().unwrap().to_string();
    let message = git_text(&folder.path, &["log", "-1", "--format=%B", &first]);
    fs::write(state.path.join("watchbill/sessions/2026-11-24T100100Z-tester-3/stamp"), message).unwrap();
    let run = run_status_in(&folder.path, Some(&state.path), &["3"]);
    let out = text(&run.stdout);
    assert!(out.contains("Sessions:\n  tester round 1, 2000-01-01T00:00:00Z, "), "{}", out);
    assert!(out.contains("    ended: outcome: ended; status: 0; seconds: 5\n    came back: taken: 1a2b3c4 Write the tests; view: removed\n"), "{}", out);
    assert!(out.contains("  build round 1, 2000-01-01T00:00:00Z, "), "{}", out);
}

#[test]
fn test_WB_STATUS_112_a_note_that_names_a_stamp_changes_nothing_of_what_its_session_says() {
    let (folder, _state) = stamped_3();
    let before = said(&folder.path, &["3"]);
    let stamp = git_text(&folder.path, &["rev-list", "--reverse", "refs/watchbill/3"]).lines().next().unwrap().to_string();
    let head = git_text(&folder.path, &["rev-parse", "issue-3-work"]).trim().to_string();
    git(&folder.path, &["notes", "--ref=refs/notes/watchbill", "add", "-f", "-m", &format!("stamp: {}\nrole: tester", stamp), &head]);
    // A note is not signed: what came back is read from the lines of the start, never from a note.
    assert_eq!(said(&folder.path, &["3"]), before);
}

#[test]
fn test_WB_STATUS_113_a_kept_view_has_a_hint_that_says_how_to_start_again() {
    let (folder, state) = stamped_3();
    let view = state.path.join("watchbill/views/view-1");
    fs::create_dir_all(&view).unwrap();
    session(&state.path, "tester", "3", 1, ENDED, Some(TAKEN));
    session(&state.path, "build", "3", 1, ENDED, Some(&format!("{}uncommitted: src/a.c\nuncommitted: src/b.c\nview: kept {}\n", TAKEN, view.display())));
    let run = run_status_in(&folder.path, Some(&state.path), &["3"]);
    let out = text(&run.stdout);
    let kept = format!("kept: build round 1, 2 paths left uncommitted, {}\n  (use \"wb start build 3 --ignore-kept\" to start again, or remove the folder)\n", view.display());
    assert!(out.contains(&kept), "{:?} not in {}", kept, out);
    // The setting turns it off, as every hint.
    add_patterns(&folder.path, "status_hints: false\n");
    let run = run_status_in(&folder.path, Some(&state.path), &["3"]);
    let out = text(&run.stdout);
    assert!(out.contains("kept: build round 1, 2 paths left uncommitted"), "{}", out);
    assert!(!out.contains("(use"), "{}", out);
}

#[test]
fn test_WB_STATUS_114_stage_is_matched_on_the_stage_words_without_regard_to_case() {
    let folder = mock_up();
    let (out, status) = said(&folder.path, &["--stage", "BUILT"]);
    assert_eq!(status, Some(0));
    assert_eq!(
        out,
        "Jobs waiting for you:\n  (use \"wb answer <issue>...\" to answer its questions)\n\t176  issue-176-work  built, 1 question open\n\nJobs in the flow:\n\t179  issue-179-work  built\n"
    );
    assert_eq!(said(&folder.path, &["--stage", "planned"]).0, "Jobs waiting for you:\n  (use \"wb plan accept <name>...\" to accept the plan)\n\t184  issue-184-work  planned\n");
    let run = run_status(&folder.path, &["--stage"]);
    assert_eq!(code(&run), Some(2));
    assert!(text(&run.stderr).contains("'--stage' needs a word"), "{}", text(&run.stderr));
    assert_eq!(said(&folder.path, &["--stage", "fixed"]), (String::new(), Some(0)));
    assert_eq!(said(&folder.path, &["--stage", "not read"]), (String::new(), Some(0)));
    // The headings of before are no stage word: they are used wrongly.
    for old in ["questions open", "needs a person", "not planned", "done", "jobs in the flow"] {
        let run = run_status(&folder.path, &["--stage", old]);
        assert_eq!(code(&run), Some(2), "{}", old);
        assert!(text(&run.stdout).is_empty());
    }
}

#[test]
fn test_WB_STATUS_115_the_columns_are_as_wide_as_the_widest_across_both_headings() {
    let folder = set_up();
    add_patterns(&folder.path, "change_of_branch: \"cat .tracker/{branch}.change\"\n");
    put(&folder.path, ".tracker/issue-9-work.change", "178\n");
    put(&folder.path, ".tracker/issue-10-work.change", "");
    for issue in ["9", "10"] {
        comments(&folder.path, issue, &[]);
    }
    branch(&folder.path, "9", true);
    branch(&folder.path, "10", false);
    let (out, status) = said(&folder.path, &[]);
    assert_eq!(status, Some(0));
    assert_eq!(
        out,
        "Jobs waiting for you:\n  (use \"wb plan accept <name>...\" to accept the plan)\n\t10  issue-10-work        planned\n\nJobs in the flow:\n\t9   issue-9-work   #178  accepted\n"
    );
}

#[test]
fn test_WB_STATUS_116_a_job_that_could_not_be_read_is_under_not_read_and_in_short_forms_has_two_stars() {
    let folder = set_up();
    open(&folder.path, &["1", "2", "3"]);
    comments(&folder.path, "1", &[]);
    comments(&folder.path, "3", &[]);
    branch(&folder.path, "3", true);
    let run = run_status(&folder.path, &[]);
    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "Jobs in the flow:\n\t3  issue-3-work  accepted\n\nNot read:\n  2  `issue_comments` of watchbill.yml ended with 1\n");
    for flag in ["-s", "--porcelain"] {
        let run = run_status(&folder.path, &[flag]);
        assert_eq!(code(&run), Some(1), "{}", flag);
        assert_eq!(text(&run.stdout), "A  3 issue-3-work\n** 2\n", "{}", flag);
        assert!(text(&run.stderr).contains("2"), "{}", text(&run.stderr));
    }
}

#[test]
fn test_WB_STATUS_117_one_issue_prints_its_job_under_its_heading_with_the_hints_and_the_title_last() {
    let folder = mock_up();
    add_patterns(&folder.path, "issue_address: \"cat .tracker/{issue}.address\"\n");
    put(&folder.path, ".tracker/176.address", "Which one\nThe rest\n");
    put(&folder.path, ".tracker/175.address", "Another\n");
    let (out, status) = said(&folder.path, &["176"]);
    assert_eq!(status, Some(0));
    assert!(out.starts_with("Jobs waiting for you:\n  (use \"wb answer <issue>...\" to answer its questions)\n\t176  issue-176-work  built, 1 question open  Which one\n\n"), "{}", out);
    assert!(out.contains("\nWorked out from:\n"), "{}", out);
    let (out, _) = said(&folder.path, &["175"]);
    assert!(out.starts_with("Jobs in the flow:\n\t175  issue-175-work  accepted  Another\n\n"), "{}", out);
    assert!(out.contains("\nWorked out from:\n"), "{}", out);
    assert!(!out.contains("(use"), "{}", out);
}

#[test]
fn test_WB_STATUS_118_a_job_that_needs_a_person_or_waits_for_one_says_so_on_its_line() {
    let folder = set_up();
    open(&folder.path, &["54", "55", "56"]);
    comments(&folder.path, "54", &["[stamp] build round 1"]);
    stamps_on_ref(&folder.path, "54", &["[needs_person] 54.build\n\nThe CRC part is not there.\n"]);
    comments(&folder.path, "55", &["[stamp] build round 1"]);
    stamps_on_ref(&folder.path, "55", &["[needs_person] 55.build\n\n"]);
    comments(&folder.path, "56", &["[stamp] build round 1", "[questions] 56.build q3", "[answers] 56.build q3>person"]);
    branch(&folder.path, "54", true);
    branch(&folder.path, "55", true);
    branch(&folder.path, "56", true);
    let (out, status) = said(&folder.path, &[]);
    assert_eq!(status, Some(0));
    assert!(out.contains("\t54  issue-54-work  built, needs a person: The CRC part is not there.\n"), "{}", out);
    assert!(out.contains("\t55  issue-55-work  built, needs a person\n"), "{}", out);
    assert!(out.contains("\t56  issue-56-work  accepted, waits for person (56.build.q3)\n"), "{}", out);
    assert!(out.starts_with("Jobs waiting for you:\n"), "{}", out);
    assert!(!out.contains("Jobs in the flow"), "{}", out);
    assert_eq!(said(&folder.path, &["-s"]).0, "BN 54 issue-54-work\nBN 55 issue-55-work\nAW 56 issue-56-work\n");
}

#[test]
fn test_WB_STATUS_119_the_usage_the_manual_and_the_completion_name_the_new_flags() {
    let folder = set_up();
    let run = run_status(&folder.path, &["--nosuch"]);
    assert_eq!(code(&run), Some(2));
    let usage = text(&run.stderr);
    for word in ["--short", "-s", "--porcelain", "--color"] {
        assert!(usage.contains(word), "{} not in {}", word, usage);
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (file, needle) in [
        ("packaging/man/wb-status.1", "--short"),
        ("packaging/man/wb-status.1", "--porcelain"),
        ("packaging/man/wb-status.1", "--color"),
        ("packaging/man/wb-status.1", "status_hints"),
        ("packaging/wb.bash", "--short"),
        ("packaging/wb.bash", "--porcelain"),
        ("packaging/wb.bash", "--color"),
    ] {
        // A hyphen is written `\-` in roff.
        let needle = if file.ends_with(".1") { needle.replace('-', "\\-") } else { needle.to_string() };
        assert!(read(&root.join(file)).contains(&needle), "{} lacks {}", file, needle);
    }
    // The paragraph of the usage has no line wider than the others.
    let usage = watchbill::USAGE;
    let block = &usage[usage.find("\n  status    ").unwrap()..usage.find("\n  view      ").unwrap()];
    for line in block.lines() {
        assert!(line.chars().count() <= 80, "{}", line);
    }
    // The values of --color are completed.
    assert!(read(&root.join("packaging/wb.bash")).contains("always"), "wb.bash lacks the values of --color");
}

#[test]
fn test_WB_STATUS_120_the_readme_says_the_hints_the_setting_and_the_forms() {
    let readme = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("README.md"));
    for needle in ["Jobs waiting for you", "Jobs in the flow", "status_hints", "--porcelain", "--short", "--color", "(use \"wb answer <issue>...\" to answer its questions)"] {
        assert!(readme.contains(needle), "the README lacks {}", needle);
    }
    for needle in ["`git diff` and `git log`", "** <number>", "color.ui", "the one given last wins"] {
        assert!(readme.contains(needle), "the README lacks {}", needle);
    }
    // The rule of before is gone.
    assert!(!readme.contains("says what is, never what to do"), "the README keeps the rule of before");
}

/// The job 7 with a branch in the flow and no issue's file, begun by `wb start`: the ref holds a planner's stamp.
fn started_without_file() -> Folder {
    let folder = set_up();
    open(&folder.path, &["7"]);
    comments(&folder.path, "7", &["[stamp] planner round 1"]);
    folder
}

const ONLY_ONE_BRANCH: &str = "Jobs waiting for you:\n\t7  issue-7-work  no issue's file yet\n";

#[test]
fn test_WB_STATUS_105_a_job_started_with_no_issue_file_is_listed_for_a_person_to_look_at() {
    let folder = started_without_file();
    assert_eq!(said(&folder.path, &[]), (ONLY_ONE_BRANCH.to_string(), Some(0)));
    assert_eq!(said(&folder.path, &["-s"]), (" L 7 issue-7-work\n".to_string(), Some(0)));
    assert_eq!(said(&folder.path, &["--porcelain"]), (" L 7 issue-7-work\n".to_string(), Some(0)));
}

#[test]
fn test_WB_STATUS_105_a_job_with_a_commit_not_pushed_and_no_issue_file_is_listed_for_a_person_to_look_at() {
    let folder = set_up();
    open(&folder.path, &["7"]);
    comments(&folder.path, "7", &[]);
    commit_on(&folder.path, "issue-7-work", "a.txt");
    let want = "Jobs waiting for you:\n\t7  issue-7-work  no issue's file yet, 1 not pushed\n";
    assert_eq!(said(&folder.path, &[]), (want.to_string(), Some(0)));
    assert_eq!(said(&folder.path, &["-s"]).0, " L 7 issue-7-work\n");
    assert_eq!(said(&folder.path, &["--porcelain"]).0, " L 7 issue-7-work\n");
}

#[test]
fn test_WB_STATUS_116_a_job_whose_ref_is_broken_is_under_not_read_with_its_reason_in_every_form() {
    let folder = set_up();
    open(&folder.path, &["7"]);
    comments(&folder.path, "7", &[]);
    // An unsigned needs_person, with an escape in its reason: the ref is not as `wb` writes it, and nothing on it counts.
    unsigned_stamps_on_ref(&folder.path, "7", &["[needs_person] 7.build\n\nA reason \u{1b}[31mred\n"]);
    for (args, want) in [(&[][..], "Not read:\n  7  "), (&["-s"][..], "** 7\n"), (&["--porcelain"][..], "** 7\n")] {
        let run = run_status(&folder.path, args);
        assert_eq!(code(&run), Some(1), "{:?}", args);
        let out = text(&run.stdout);
        assert!(out.contains(want) && out.contains("not as wb writes it") == (args.is_empty()), "{:?}: {}", args, out);
        assert!(!out.contains('\u{1b}'), "{:?}", out);
        assert!(text(&run.stderr).contains("issue 7 was not read") && text(&run.stderr).contains("not as wb writes it"), "{:?}: {}", args, text(&run.stderr));
    }
}

/// Job 7 with an accepted issue's file and a ref that an unsigned stamp breaks, or one that a forged needs_person breaks.
fn broken_7(kind: usize) -> Folder {
    let folder = set_up();
    open(&folder.path, &["7"]);
    comments(&folder.path, "7", &[]);
    branch(&folder.path, "7", true);
    let forged = ["[stamp] build round 1\ntime: 2000-01-01T00:00:00Z\n", "[needs_person] 7.build\n\nA reason \u{1b}[31mred\n"];
    unsigned_stamps_on_ref(&folder.path, "7", &[forged[kind]]);
    folder
}

#[test]
fn test_WB_STATUS_116_a_job_asked_for_by_name_whose_ref_is_broken_is_not_read_as_in_the_list() {
    for kind in 0..2 {
        let folder = broken_7(kind);
        for (args, want) in [(&["7"][..], "Not read:\n  7  "), (&["-s", "7"][..], "** 7\n"), (&["--porcelain", "7"][..], "** 7\n")] {
            let run = run_status(&folder.path, args);
            let out = text(&run.stdout);
            assert_eq!(code(&run), Some(1), "{} {:?}: {}", kind, args, out);
            assert!(out.starts_with(want), "{} {:?}: {}", kind, args, out);
            // Nothing read from a ref that does not count reaches the output: no stage, no hint, no escape, no line of its stamps.
            for word in ["accepted", "built", "planned", "(use", "Jobs", "Worked out", "Sessions", "waits for", "\u{1b}", "issue-7-work"] {
                assert!(!out.contains(word), "{} {:?} has {:?}: {}", kind, args, word, out);
            }
            assert_eq!(out.contains("not as wb writes it"), args.len() == 1, "{} {:?}: {}", kind, args, out);
            assert!(text(&run.stderr).contains("issue 7 was not read") && text(&run.stderr).contains("not as wb writes it"), "{} {:?}: {}", kind, args, text(&run.stderr));
        }
    }
}

#[test]
fn test_WB_STATUS_116_with_stage_a_job_that_is_not_read_still_says_why_on_standard_error() {
    let folder = broken_7(0);
    for args in [&["--stage", "built"][..], &["-s", "--stage", "built"], &["--stage", "not read"]] {
        let run = run_status(&folder.path, args);
        assert_eq!(code(&run), Some(1), "{:?}", args);
        assert!(text(&run.stderr).contains("issue 7 was not read") && text(&run.stderr).contains("not as wb writes it"), "{:?}: {}", args, text(&run.stderr));
        assert_eq!(text(&run.stdout).is_empty(), args[args.len() - 1] == "built", "{:?}: {}", args, text(&run.stdout));
    }
}
