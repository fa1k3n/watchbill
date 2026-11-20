//! `wb <role>`: the starter (#84). It checks what `wb prompt` checks, then the rule table, the class, the
//! runner's `start` and the terminal; it posts the stamp by `issue_comment` and starts the session by `start`.
//!
//! The commands it runs are stand-ins: small scripts the tests write, which put what they were given in a
//! folder of the test. Nothing reaches the network. The starter is run in the test process, through
//! `watchbill::start`, with the organisation's file of the test and the answer to whether standard input and
//! output are a terminal: the Fedora image the package is built in has no `script` to give `wb` a terminal.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;
use watchbill::USAGE;

/// A repository made for a test, the folder of its stand-ins, and the organisation's file, in a folder of its
/// own: a session in a view (#126) has the folder of the stand-ins bound, and never the organisation's.
struct Bench {
    repo: Folder,
    log: Folder,
    org: PathBuf,
    _org_folder: Folder,
}

impl Bench {
    fn root(&self) -> String {
        fs::canonicalize(&self.repo.path).unwrap().to_str().unwrap().to_string()
    }

    fn at(&self, name: &str) -> PathBuf {
        self.log.path.join(name)
    }

    /// What the stand-ins were run with, one line for each run, in order.
    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.at("calls")).unwrap_or_default().lines().map(str::to_string).collect()
    }

    fn posted(&self, kind: &str, name: &str) -> Option<String> {
        fs::read_to_string(self.at(&format!("posted-{}-{}", kind, name))).ok()
    }

    /// The comments on an issue or a change, as `issue_comments` or `change_comments` prints them.
    fn comments(&self, kind: &str, name: &str, lines: &[&str]) {
        let text: String = lines.iter().map(|line| format!("{}\n", line)).collect();
        fs::write(self.at(&format!("comments-{}-{}", kind, name)), text).unwrap();
    }

    /// The open changes of a branch, as `change_of_branch` prints them.
    fn changes(&self, branch: &str, numbers: &[&str]) {
        let text: String = numbers.iter().map(|line| format!("{}\n", line)).collect();
        fs::write(self.at(&format!("changes-{}", branch)), text).unwrap();
    }

    /// The status a stand-in ends with: `post`, `comments` or `start`.
    fn ends_with(&self, which: &str, status: i32) {
        fs::write(self.at(&format!("{}.status", which)), format!("{}\n", status)).unwrap();
    }

    fn started(&self) -> bool {
        self.calls().iter().any(|call| call == "start")
    }

    fn any_posted(&self) -> bool {
        self.calls().iter().any(|call| call.starts_with("post "))
    }

    fn start(&self, terminal: bool, args: &[&str]) -> Output {
        start_with(&self.repo.path, &self.org, terminal, args)
    }

    fn prompt(&self, args: &[&str]) -> Output {
        prompt_with(&self.repo.path, &self.org, args)
    }

    /// Replaces a line of `watchbill.yml` that starts with `key:`, or takes it out when `line` is empty.
    fn set(&self, key: &str, line: &str) {
        let yml = read(&config(&self.repo.path));
        let mut kept: Vec<String> = yml.lines().filter(|l| !l.starts_with(&format!("{}:", key))).map(str::to_string).collect();
        if !line.is_empty() {
            kept.push(line.to_string());
        }
        fs::write(config(&self.repo.path), format!("{}\n", kept.join("\n"))).unwrap();
    }

    fn organisation(&self, text: &str) {
        fs::write(&self.org, text).unwrap();
    }
}

fn script(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\n{}", body)).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// The organisation's file of the bench: `stand-in` has `start` and is cleared for the breakdown; `idle`
/// has no `start`. `{default}` is the line of `default`, or nothing.
const ORG: &str = "levels: [public, internal, confidential]
{default}runners:
  stand-in:
    tools: this machine
    model: a stand-in
    agreement: none
    up_to: internal
    breakdown:
      up_to: public
      model: claude-opus-5-5
      effort: high
    start: ['{log}/start.sh', '--model={model}', '{effort}', '{prompt}']
    binds: [{writable: '{log}'}]
  idle:
    tools: this machine
    model: a stand-in
    agreement: none
    up_to: internal
";

fn org(log: &Path, default: Option<&str>) -> String {
    let line = default.map(|name| format!("default: {}\n", name)).unwrap_or_default();
    ORG.replace("{log}", log.to_str().unwrap()).replace("{default}", &line)
}

/// A repository set up by `wb init --yes`, which writes `classed: []`, with every pattern a stand-in, the level `public`,
/// committed on `main`, with `origin/HEAD` naming `main`, the branch `issue-54`, and `feature-x` checked out.
/// `origin` is an address no stand-in serves; git reaches it only through `ssh.sh`, which writes `network`.
fn bench() -> Bench {
    let repo = repository(Some("ssh://example.invalid/brakes/controller.git"));
    let log = folder_named("log");
    let at = |name: &str| log.path.join(name).to_str().unwrap().to_string();
    let l = log.path.to_str().unwrap();
    script(&log.path.join("post.sh"), &format!("echo \"post $1 $2\" >> {l}/calls\ncat > {l}/posted-$1-$2\nexit $(cat {l}/post.status 2>/dev/null || echo 0)\n"));
    script(
        &log.path.join("comments.sh"),
        &format!("echo \"comments $1 $2\" >> {l}/calls\npwd > {l}/comments.pwd\ncat {l}/comments-$1-$2 2>/dev/null\nexit $(cat {l}/comments.status 2>/dev/null || echo 0)\n"),
    );
    script(&log.path.join("changes.sh"), &format!("echo \"changes $1\" >> {l}/calls\ncat {l}/changes-$1 2>/dev/null\nexit 0\n"));
    script(
        &log.path.join("start.sh"),
        &format!(
            "echo start >> {l}/calls\npwd > {l}/start.pwd\nprintf '%s' \"$1\" > {l}/start.model\nprintf '%s' \"$2\" > {l}/start.effort\nprintf '%s' \"$3\" > {l}/start.prompt\necho $# > {l}/start.count\nexit $(cat {l}/start.status 2>/dev/null || echo 0)\n"
        ),
    );
    script(&log.path.join("ssh.sh"), &format!("echo ssh >> {l}/network\nexit 1\n"));
    git(&repo.path, &["config", "user.name", "Ada Lovelace"]);
    git(&repo.path, &["config", "user.email", "ada@example.org"]);
    git(&repo.path, &["config", "commit.gpgsign", "false"]);
    git(&repo.path, &["config", "core.sshCommand", &at("ssh.sh")]);
    let run = wb(&repo.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let mut yml = read(&config(&repo.path));
    for line in [
        format!("issue_comment: '{} issue {{issue}}'", at("post.sh")),
        format!("issue_comments: '{} issue {{issue}}'", at("comments.sh")),
        format!("change_comment: '{} change {{change}}'", at("post.sh")),
        format!("change_comments: '{} change {{change}}'", at("comments.sh")),
        format!("change_of_branch: '{} {{branch}}'", at("changes.sh")),
        "issue_address: 'tracker {issue}'".to_string(),
        "change_address: 'server {change}'".to_string(),
        "confidentiality: public".to_string(),
    ] {
        yml.push_str(&format!("{}\n", line));
    }
    fs::write(config(&repo.path), yml).unwrap();
    git(&repo.path, &["add", "--", ".watchbill"]);
    git(&repo.path, &["commit", "-q", "-m", "Watchbill"]);
    git(&repo.path, &["branch", "-M", "main"]);
    git(&repo.path, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(&repo.path, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"]);
    git(&repo.path, &["branch", "issue-54"]);
    git(&repo.path, &["checkout", "-q", "-b", "feature-x"]);
    let org_folder = folder_named("org");
    let path = org_folder.path.join("organisation.yml");
    fs::write(&path, org(&log.path, Some("stand-in"))).unwrap();
    Bench { repo, log, org: path, _org_folder: org_folder }
}

/// The model and effort of the model table for a role.
fn table(role: &str) -> (String, String) {
    let rules = watchbill::rules::load_revision().unwrap();
    let model = &rules.models[role];
    (model.model.clone(), model.effort.clone())
}

/// A stamp without its line `time:`, which differs from one call to the next.
fn timeless(stamp: &str) -> String {
    stamp.lines().filter(|line| !line.starts_with("time:")).map(|line| format!("{}\n", line)).collect()
}

fn ok(run: &Output) {
    assert_eq!(code(run), Some(0), "{}", text(&run.stderr));
}

/// A start of an issue with an accepted file, which runs in a view (#126): it ends with 0 where bubblewrap
/// can make a sandbox, and where it cannot, it is refused at the view, after every check before it.
fn ok_in_view(run: &Output) {
    match watchbill::view::works(Path::new(watchbill::view::BWRAP)) {
        Ok(()) => ok(run),
        Err(_) => {
            assert_eq!(code(run), Some(1), "{}", text(&run.stderr));
            assert!(text(&run.stderr).contains("bubblewrap"), "{}", text(&run.stderr));
            assert!(!text(&run.stderr).contains("#65"), "{}", text(&run.stderr));
        }
    }
}

/// Refused with `status`, with each of `named` on standard error, and nothing posted or started.
fn refused(bench: &Bench, run: &Output, status: i32, named: &[&str]) {
    assert_eq!(code(run), Some(status), "{}", text(&run.stderr));
    let err = text(&run.stderr);
    for name in named {
        assert!(err.contains(name), "{} not in: {}", name, err);
    }
    assert!(!bench.any_posted(), "posted: {:?}", bench.calls());
    assert!(!bench.started(), "started: {:?}", bench.calls());
}

/// An issue's file with the given `changes`, accepted, and committed with its acceptance.
fn issue_file(bench: &Bench, issue: &str, changes: &str) {
    let file = BARE_ISSUE.replace("{issue}", issue).replace("changes: []", &format!("changes: {}", changes));
    put(&bench.repo.path, &format!(".watchbill/issues/{}.yml", issue), &file);
    let run = wb(&bench.repo.path, &["accept", "--no-commit", "--", issue], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    git(&bench.repo.path, &["add", "--", ".watchbill/issues"]);
    git(&bench.repo.path, &["commit", "-q", "-m", "The issue's file"]);
}

// ---- WB-BUILD-1: what `wb prompt` refuses, `wb <role>` refuses with the same message and status ----

fn same_as_prompt(bench: &Bench, role: &str, words: &[&str]) {
    let mut started = vec!["start", role];
    started.extend(words);
    let mut prompted = vec![role];
    prompted.extend(words);
    let by_prompt = bench.prompt(&prompted);
    let by_start = bench.start(true, &started);
    assert_ne!(code(&by_prompt), Some(0), "{:?}: wb prompt did not refuse", words);
    assert_eq!(code(&by_start), code(&by_prompt), "{:?}: {}", words, text(&by_start.stderr));
    assert_eq!(text(&by_start.stdout), "");
    let said = text(&by_prompt.stderr).replace("wb prompt:", &format!("wb {}:", role));
    // The line that names the folder of the start's lines is the one WB-MON-2 asks for, in addition (#124).
    let folder = format!("wb {}: will write the lines of this start in ", role);
    let by_start: String = text(&by_start.stderr).lines().filter(|line| !line.starts_with(&folder)).map(|line| format!("{}\n", line)).collect();
    assert_eq!(by_start, said, "{:?}", words);
    assert!(!bench.any_posted() && !bench.started(), "{:?}: {:?}", words, bench.calls());
}

#[test]
fn test_WB_BUILD_1_a_check_that_does_not_hold_is_refused_as_wb_prompt_refuses_it() {
    let bench = bench();
    let role = bench.repo.path.join(".watchbill/revision/roles/build.md");
    let mut changed = read(&role);
    changed.push_str("\nAnother line.\n");
    fs::write(&role, changed).unwrap();
    same_as_prompt(&bench, "build", &["54"]);
}

#[test]
fn test_WB_BUILD_1_an_issue_s_file_that_is_not_accepted_is_refused_as_wb_prompt_refuses_it() {
    let bench = bench();
    put(&bench.repo.path, ".watchbill/issues/54.yml", &BARE_ISSUE.replace("{issue}", "54"));
    same_as_prompt(&bench, "build", &["54"]);
}

#[test]
fn test_WB_BUILD_1_a_breakdown_runner_that_is_not_cleared_is_refused_as_wb_prompt_refuses_it() {
    let bench = bench();
    same_as_prompt(&bench, "breakdown", &["54", "--runner", "idle"]);
}

#[test]
fn test_WB_BUILD_1_words_used_wrongly_are_refused_as_wb_prompt_refuses_them() {
    let bench = bench();
    same_as_prompt(&bench, "build", &["54", "--issue", "54"]);
    same_as_prompt(&bench, "review", &["54", "--nonsense"]);
    same_as_prompt(&bench, "tester", &[]);
    same_as_prompt(&bench, "build", &["54", "--stamp", "--parts"]);
}

// ---- WB-BUILD-2: the rule table: a fix follows a review ----

#[test]
fn test_WB_BUILD_2_a_fix_with_no_review_stamp_is_refused_and_names_the_rule_also_with_a_round_given() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.comments("issue", "54", &["[stamp] build round 1"]);
    for words in [&["start", "fix", "54"][..], &["start", "fix", "54", "--round", "1"]] {
        let run = bench.start(true, words);
        refused(&bench, &run, 1, &["a fix follows a review", "no review stamp"]);
    }
}

#[test]
fn test_WB_BUILD_2_a_fix_whose_rule_cannot_be_checked_for_want_of_issue_comments_is_refused() {
    let bench = bench();
    bench.set("issue_comments", "");
    let run = bench.start(true, &["start", "fix", "54", "--round", "1", "--change", "174"]);
    refused(&bench, &run, 1, &["a fix follows a review", "issue_comments"]);
}

#[test]
fn test_WB_BUILD_2_the_number_of_review_rounds_is_no_rule_for_a_person() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    // The rule table's greatest number of rounds is 2: a person who starts a third has decided to.
    let rounds = watchbill::rules::load_revision().unwrap().review_rounds;
    let lines: Vec<String> = (1..=rounds).map(|n| format!("[stamp] review round {}", n)).collect();
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    bench.comments("issue", "54", &lines);
    let run = bench.start(true, &["start", "review", "54"]);
    ok(&run);
    assert!(bench.started());
}

// ---- WB-BUILD-3: classed code in `changes` is refused ----

#[test]
fn test_WB_BUILD_3_an_issue_whose_changes_hold_classed_code_is_refused_naming_65() {
    let bench = bench();
    issue_file(&bench, "54", "[src/brake/pedal.c]");
    bench.set("classed", "classed: [src/brake]");
    let run = bench.start(true, &["start", "build", "54"]);
    refused(&bench, &run, 1, &["#65", "src/brake/pedal.c"]);
}

// ---- WB-BUILD-4: the order of the checks, and nothing posted or started on a fault ----

#[test]
fn test_WB_BUILD_4_the_checks_are_made_in_order_and_the_first_fault_ends_it() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.set("classed", "classed: [src]");
    bench.comments("issue", "54", &["[stamp] build round 1"]);
    // The rule, then the class, then the runner's `start`, then the terminal.
    let words = ["start", "fix", "54", "--runner", "idle"];
    refused(&bench, &bench.start(false, &words), 1, &["a fix follows a review"]);
    bench.comments("issue", "54", &["[stamp] review round 1"]);
    refused(&bench, &bench.start(false, &words), 1, &["#65", "classed"]);
    bench.set("classed", "classed: []");
    refused(&bench, &bench.start(false, &words), 1, &["idle", "--prompt"]);
    refused(&bench, &bench.start(false, &["start", "fix", "54"]), 1, &["terminal", "#65"]);
    ok(&bench.start(true, &["start", "fix", "54"]));
    assert!(bench.started());
}

#[test]
fn test_WB_BUILD_4_a_check_of_wb_prompt_comes_before_the_rule_table() {
    let bench = bench();
    put(&bench.repo.path, ".watchbill/issues/54.yml", &BARE_ISSUE.replace("{issue}", "54"));
    // No review stamp either: the issue's file is named, not the rule.
    let run = bench.start(true, &["start", "fix", "54", "--round", "1", "--change", "174"]);
    refused(&bench, &run, 1, &["not accepted"]);
    assert!(!text(&run.stderr).contains("a fix follows a review"), "{}", text(&run.stderr));
}

// ---- WB-BUILD-5: the stamp is said and posted by `issue_comment`, with its first line ----

#[test]
fn test_WB_BUILD_5_the_stamp_is_posted_on_the_issue_with_its_first_line_and_the_lines_of_wb_prompt_stamp() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.comments("issue", "54", &["[review] 1. A finding", "[stamp] review round 1", "[stamp] build round 1", "hello"]);
    let run = bench.start(true, &["start", "review", "54"]);
    ok(&run);
    let posted = bench.posted("issue", "54").expect("nothing was posted on issue 54");
    let (first, rest) = posted.split_once('\n').unwrap();
    assert_eq!(first, "[stamp] review round 2");
    let stamp = bench.prompt(&["review", "54", "--change", "174", "--round", "2", "--runner", "stand-in", "--stamp"]);
    ok(&stamp);
    assert_eq!(timeless(rest), timeless(&text(&stamp.stdout)));
    // Posted once, then started once.
    let calls = bench.calls();
    assert_eq!(calls.iter().filter(|call| call.starts_with("post ")).collect::<Vec<_>>(), ["post issue 54"]);
    let posted_at = calls.iter().position(|call| call == "post issue 54").unwrap();
    assert_eq!(calls.iter().position(|call| call == "start"), Some(posted_at + 1), "{:?}", calls);
}

#[test]
fn test_WB_BUILD_5_what_will_be_posted_is_said_on_standard_error() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    let run = bench.start(true, &["start", "review", "54"]);
    ok(&run);
    let err = text(&run.stderr);
    let posted = bench.posted("issue", "54").unwrap();
    assert!(err.contains("will post"), "{}", err);
    assert!(err.contains(&posted), "{}", err);
}

#[test]
fn test_WB_BUILD_5_a_posting_that_fails_starts_nothing_and_ends_with_1() {
    let bench = bench();
    bench.ends_with("post", 1);
    let run = bench.start(true, &["start", "build", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("issue_comment"), "{}", text(&run.stderr));
    assert!(bench.any_posted());
    assert!(!bench.started(), "{:?}", bench.calls());
}

// ---- WB-BUILD-6: the session is started by `start`, with the model, the effort and the prompt ----

#[test]
fn test_WB_BUILD_6_start_is_run_once_with_the_model_the_effort_and_the_prompt_in_the_root() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    let run = bench.start(true, &["start", "review", "54"]);
    ok(&run);
    let (model, effort) = table("review");
    assert_eq!(read(&bench.at("start.model")), format!("--model={}", model));
    assert_eq!(read(&bench.at("start.effort")), effort);
    let prompt = bench.prompt(&["review", "54", "--change", "174", "--round", "1"]);
    ok(&prompt);
    assert_eq!(read(&bench.at("start.prompt")), text(&prompt.stdout));
    // The prompt is one word, also with its line breaks.
    assert_eq!(read(&bench.at("start.count")).trim(), "3");
    assert_eq!(read(&bench.at("start.pwd")).trim(), bench.root());
    assert_eq!(bench.calls().iter().filter(|call| *call == "start").count(), 1);
}

#[test]
fn test_WB_BUILD_6_the_session_is_started_from_a_folder_inside_the_checkout_in_its_root() {
    let bench = bench();
    fs::create_dir_all(bench.repo.path.join("src/deep")).unwrap();
    let run = start_with(&bench.repo.path.join("src/deep"), &bench.org, true, &["start", "build", "54"]);
    ok(&run);
    assert_eq!(read(&bench.at("start.pwd")).trim(), bench.root());
}

#[test]
fn test_WB_BUILD_6_the_breakdown_starts_with_the_model_and_effort_its_runner_is_cleared_with() {
    let bench = bench();
    ok(&bench.start(true, &["start", "breakdown", "54"]));
    assert_eq!(read(&bench.at("start.model")), "--model=claude-opus-5-5");
    assert_eq!(read(&bench.at("start.effort")), "high");
    assert!(bench.posted("issue", "54").unwrap().starts_with("[stamp] breakdown round 1\n"));
}

#[test]
fn test_WB_BUILD_6_the_status_of_the_session_is_0_for_0_and_1_otherwise() {
    let bench = bench();
    ok(&bench.start(true, &["start", "build", "54"]));
    for status in [1, 3] {
        bench.ends_with("start", status);
        let run = bench.start(true, &["start", "build", "54"]);
        assert_eq!(code(&run), Some(1), "{}: {}", status, text(&run.stderr));
    }
}

#[test]
fn test_WB_BUILD_6_a_start_that_cannot_be_run_ends_with_1_and_names_it() {
    let bench = bench();
    bench.organisation(&org(&bench.log.path, Some("stand-in")).replace("/start.sh", "/no-such-program"));
    let run = bench.start(true, &["start", "build", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("no-such-program"), "{}", text(&run.stderr));
}

// ---- WB-BUILD-7: no terminal, and a runner without `start` ----

#[test]
fn test_WB_BUILD_7_with_no_terminal_it_is_refused_with_1_naming_65() {
    let bench = bench();
    let run = bench.start(false, &["start", "build", "54"]);
    refused(&bench, &run, 1, &["terminal", "#65"]);
}

#[test]
fn test_WB_BUILD_7_a_runner_without_start_is_refused_with_1_naming_prompt() {
    let bench = bench();
    let run = bench.start(true, &["start", "build", "54", "--runner", "idle"]);
    refused(&bench, &run, 1, &["idle", "start", "--prompt"]);
}

#[test]
fn test_WB_BUILD_7_a_runner_the_organisation_s_file_does_not_name_is_refused_with_1() {
    let bench = bench();
    let run = bench.start(true, &["start", "build", "54", "--runner", "other"]);
    refused(&bench, &run, 1, &["other", "stand-in", "idle"]);
}

#[test]
fn test_WB_BUILD_7_the_program_refuses_without_a_terminal() {
    // Through the program, standard input and output are pipes: the organisation's file of this machine
    // decides how far it gets, and whatever it is, nothing is started and the status is not 0.
    let bench = bench();
    let run = wb(&bench.repo.path, &["start", "build", "54", "--runner", "stand-in"], "");
    assert_ne!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(!bench.started(), "{:?}", bench.calls());
}

// ---- WB-BUILD-8: `--prompt` prints what `wb prompt` prints ----

#[test]
fn test_WB_BUILD_8_prompt_prints_what_wb_prompt_prints_and_posts_and_starts_nothing() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    for words in [&["start", "build", "54"][..], &["start", "review", "54"], &["start", "tester", "54", "--requirements", "R-1"], &["start", "review", "--change", "174"]] {
        let mut started = words.to_vec();
        started.push("--prompt");
        let by_start = bench.start(false, &started);
        let by_prompt = bench.prompt(words);
        ok(&by_start);
        assert_eq!(text(&by_start.stdout), text(&by_prompt.stdout), "{:?}", words);
        assert_eq!(text(&by_start.stderr), text(&by_prompt.stderr).replace("wb prompt:", &format!("wb {}:", words[0])), "{:?}", words);
    }
    assert!(!bench.any_posted() && !bench.started(), "{:?}", bench.calls());
}

#[test]
fn test_WB_BUILD_8_stamp_and_parts_print_what_wb_prompt_prints() {
    let bench = bench();
    for extra in [&["--stamp"][..], &["--prompt", "--stamp"], &["--parts"], &["--prompt", "--parts"]] {
        let mut started = vec!["start", "build", "54"];
        started.extend(extra);
        let by_start = bench.start(true, &started);
        let mut prompted = vec!["build", "54"];
        prompted.extend(extra.iter().filter(|word| **word != "--prompt"));
        let by_prompt = bench.prompt(&prompted);
        ok(&by_start);
        assert_eq!(timeless(&text(&by_start.stdout)), timeless(&text(&by_prompt.stdout)), "{:?}", extra);
    }
    assert!(!bench.any_posted() && !bench.started(), "{:?}", bench.calls());
}

#[test]
fn test_WB_BUILD_8_the_program_takes_each_role_as_a_command() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    for words in [&["build", "54"][..], &["tester", "54"], &["review", "54", "--round", "1"], &["fix", "54", "--round", "1"]] {
        let mut started = vec!["start"];
        started.extend(words);
        started.push("--prompt");
        let by_start = wb(&bench.repo.path, &started, "");
        let mut prompted = vec!["prompt"];
        prompted.extend(words);
        let by_prompt = wb(&bench.repo.path, &prompted, "");
        ok(&by_start);
        assert_eq!(text(&by_start.stdout), text(&by_prompt.stdout), "{:?}", words);
    }
    assert!(!bench.any_posted() && !bench.started(), "{:?}", bench.calls());
}

#[test]
fn test_WB_BUILD_8_the_usage_names_the_commands_of_the_roles() {
    for role in ["breakdown", "tester", "build", "review", "fix"] {
        assert!(USAGE.contains(&format!("wb {} ", role)), "{}", role);
    }
}

// ---- WB-BUILD-9: the network is asked only through the patterns and `start` ----

#[test]
fn test_WB_BUILD_9_a_whole_start_reaches_no_remote_of_git_and_runs_only_the_stand_ins() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.comments("issue", "54", &["[stamp] review round 1"]);
    ok(&bench.start(true, &["start", "fix", "54"]));
    assert!(!bench.at("network").exists(), "git asked origin");
    assert_eq!(bench.calls(), ["comments issue 54", "changes issue-54", "post issue 54", "start"]);
}

// ---- WB-BUILD-10: the runner: `--runner`, else the organisation's `default` ----

#[test]
fn test_WB_BUILD_10_with_no_runner_and_no_default_it_is_refused_with_2_and_the_usage() {
    let bench = bench();
    bench.organisation(&org(&bench.log.path, None));
    let run = bench.start(true, &["start", "build", "54"]);
    refused(&bench, &run, 2, &["--runner", "default"]);
    assert!(text(&run.stderr).ends_with(USAGE), "{}", text(&run.stderr));
}

#[test]
fn test_WB_BUILD_10_with_no_runner_and_no_organisation_s_file_it_is_refused_with_2_naming_the_file() {
    let bench = bench();
    fs::remove_file(&bench.org).unwrap();
    let run = bench.start(true, &["start", "build", "54"]);
    refused(&bench, &run, 2, &["--runner", bench.org.to_str().unwrap()]);
    assert!(text(&run.stderr).ends_with(USAGE), "{}", text(&run.stderr));
}

#[test]
fn test_WB_BUILD_10_the_default_is_taken_and_a_runner_given_wins() {
    let bench = bench();
    ok(&bench.start(true, &["start", "build", "54"]));
    assert!(bench.posted("issue", "54").unwrap().contains("\nrunner: stand-in\n"));
    // The default has no `start`; the runner given has, and wins.
    bench.organisation(&org(&bench.log.path, Some("idle")));
    let run = bench.start(true, &["start", "build", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("idle"), "{}", text(&run.stderr));
    ok(&bench.start(true, &["start", "build", "54", "--runner", "stand-in"]));
    assert!(bench.posted("issue", "54").unwrap().contains("\nrunner: stand-in\n"));
}

#[test]
fn test_WB_CONF_24_a_default_that_names_no_runner_is_refused_with_the_place() {
    let bench = bench();
    bench.organisation(&org(&bench.log.path, Some("other")));
    let run = bench.start(true, &["start", "build", "54"]);
    refused(&bench, &run, 1, &["default", "other"]);
}

// ---- WB-BUILD-11: the round, from the stamps ----

#[test]
fn test_WB_BUILD_11_the_review_takes_one_more_than_the_review_stamps_and_the_fix_their_number() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.comments("issue", "54", &["[stamp] build round 1", "[stamp] review round 1", "[review] 1. must: a finding"]);
    ok(&bench.start(true, &["start", "review", "54"]));
    assert!(bench.posted("issue", "54").unwrap().starts_with("[stamp] review round 2\n"));
    ok(&bench.start(true, &["start", "fix", "54"]));
    assert!(bench.posted("issue", "54").unwrap().starts_with("[stamp] fix round 1\n"));
}

#[test]
fn test_WB_BUILD_11_the_tester_the_build_and_the_breakdown_take_one_more_than_their_own_stamps() {
    let bench = bench();
    bench.comments("issue", "54", &["[stamp] breakdown round 1", "[stamp] tester round 1", "[stamp] tester round 2", "[stamp] review round 1"]);
    for (role, round) in [("tester", 3), ("build", 1), ("breakdown", 2)] {
        ok(&bench.start(true, &["start", role, "54"]));
        let posted = bench.posted("issue", "54").unwrap();
        assert!(posted.starts_with(&format!("[stamp] {} round {}\n", role, round)), "{}", posted);
    }
}

#[test]
fn test_WB_BUILD_11_wb_prompt_refuses_a_fix_with_no_review_stamp() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.comments("issue", "54", &["[stamp] build round 1"]);
    let run = bench.prompt(&["fix", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("no review stamp"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_BUILD_11_wb_prompt_works_out_the_same_round_by_the_same_function() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.comments("issue", "54", &["[stamp] review round 1"]);
    for (role, round) in [("review", "2"), ("fix", "1")] {
        let worked = bench.prompt(&[role, "54"]);
        let given = bench.prompt(&[role, "54", "--change", "174", "--round", round]);
        ok(&worked);
        assert_eq!(text(&worked.stdout), text(&given.stdout), "{}", role);
    }
}

#[test]
fn test_WB_BUILD_11_with_no_issue_comments_and_no_round_wb_role_is_refused_naming_both() {
    let bench = bench();
    bench.set("issue_comments", "");
    let run = bench.start(true, &["start", "build", "54"]);
    refused(&bench, &run, 1, &["issue_comments", "--round"]);
    ok(&bench.start(true, &["start", "build", "54", "--round", "1"]));
}

#[test]
fn test_WB_BUILD_11_wb_prompt_asks_for_no_round_where_its_text_holds_none() {
    let bench = bench();
    ok(&bench.prompt(&["build", "54"]));
    ok(&bench.prompt(&["tester", "54"]));
    assert_eq!(bench.calls(), Vec::<String>::new());
}

#[test]
fn test_WB_BUILD_11_a_comments_command_that_fails_is_refused_with_1_and_named() {
    let bench = bench();
    bench.ends_with("comments", 1);
    let run = bench.start(true, &["start", "build", "54"]);
    refused(&bench, &run, 1, &["issue_comments"]);
}

// ---- WB-BUILD-12: the change, from `change_of_branch` ----

#[test]
fn test_WB_BUILD_12_the_one_open_change_of_the_branch_is_taken() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    ok(&bench.start(true, &["start", "review", "54"]));
    assert!(bench.posted("issue", "54").unwrap().contains("\nchange: 174\n"));
    assert!(read(&bench.at("start.prompt")).contains("server 174"));
}

#[test]
fn test_WB_BUILD_12_none_or_two_open_changes_are_refused_with_1_naming_what_was_found() {
    let bench = bench();
    let run = bench.start(true, &["start", "review", "54"]);
    refused(&bench, &run, 1, &["no open change", "issue-54", "--change"]);
    bench.changes("issue-54", &["174", "175"]);
    let run = bench.start(true, &["start", "review", "54"]);
    refused(&bench, &run, 1, &["174", "175", "issue-54", "--change"]);
    let run = bench.prompt(&["review", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("--change"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_BUILD_12_given_values_win_and_nothing_is_worked_out_for_them() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.comments("issue", "54", &["[stamp] review round 1"]);
    ok(&bench.start(true, &["start", "review", "54", "--round", "5", "--change", "180"]));
    let posted = bench.posted("issue", "54").unwrap();
    assert!(posted.starts_with("[stamp] review round 5\n"), "{}", posted);
    assert!(posted.contains("\nchange: 180\n") && posted.contains("\nround: 5\n"), "{}", posted);
    assert_eq!(bench.calls(), ["post issue 54", "start"]);
}

// ---- WB-BUILD-13: what was worked out is said first ----

#[test]
fn test_WB_BUILD_13_the_round_and_the_change_and_where_they_came_from_are_said_before_the_stamp() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.comments("issue", "54", &["[stamp] review round 1"]);
    let run = bench.start(true, &["start", "review", "54"]);
    ok(&run);
    let err = text(&run.stderr);
    let worked = err.find("round 2 (1 review stamp on issue 54)").unwrap_or_else(|| panic!("{}", err));
    let change = err.find("change 174 (the one open change from issue-54)").unwrap_or_else(|| panic!("{}", err));
    let post = err.find("will post").unwrap();
    assert!(worked < post && change < post, "{}", err);
}

// ---- WB-BUILD-17 and 18: a review or a fix with no issue (#116) ----

#[test]
fn test_WB_BUILD_17_a_review_with_no_issue_posts_its_stamp_on_the_change_by_change_comment() {
    let bench = bench();
    let run = bench.start(true, &["start", "review", "--change", "174"]);
    ok(&run);
    let posted = bench.posted("change", "174").expect("nothing posted on change 174");
    assert!(posted.starts_with("[stamp] review round 1\n"), "{}", posted);
    assert!(posted.contains("\nissue:\n"), "{}", posted);
    assert!(!bench.calls().iter().any(|call| call.starts_with("post issue")), "{:?}", bench.calls());
    assert!(bench.started());
}

#[test]
fn test_WB_BUILD_17_with_no_change_comment_a_review_with_no_issue_is_refused() {
    let bench = bench();
    bench.set("change_comment", "");
    let run = bench.start(true, &["start", "review", "--change", "174"]);
    refused(&bench, &run, 1, &["change_comment"]);
}

#[test]
fn test_WB_BUILD_18_the_round_of_a_change_with_no_issue_is_read_from_the_stamps_on_the_change() {
    let bench = bench();
    bench.comments("change", "174", &["[stamp] review round 1", "a comment"]);
    ok(&bench.start(true, &["start", "review", "--change", "174"]));
    assert!(bench.posted("change", "174").unwrap().starts_with("[stamp] review round 2\n"));
    ok(&bench.start(true, &["start", "fix", "--change", "174"]));
    assert!(bench.posted("change", "174").unwrap().starts_with("[stamp] fix round 1\n"));
    let worked = bench.prompt(&["review", "--change", "174"]);
    let given = bench.prompt(&["review", "--change", "174", "--round", "2"]);
    ok(&worked);
    assert_eq!(text(&worked.stdout), text(&given.stdout));
}

#[test]
fn test_WB_BUILD_18_a_fix_of_a_change_with_no_review_stamp_on_it_is_refused() {
    let bench = bench();
    let run = bench.start(true, &["start", "fix", "--change", "174"]);
    refused(&bench, &run, 1, &["no review stamp", "change 174"]);
}

// ---- WB-BUILD-19 and 20: which paths are classed ----

#[test]
fn test_WB_BUILD_19_a_path_below_a_classed_folder_or_a_folder_above_a_classed_path_is_classed() {
    let bench = bench();
    bench.set("classed", "classed: [src/brake, docs/safety.md]");
    for (n, changes) in ["[src/brake/pedal.c]", "[src]", "[docs/safety.md]", "[src/brake]"].iter().enumerate() {
        let issue = format!("{}", 60 + n);
        issue_file(&bench, &issue, changes);
        git(&bench.repo.path, &["branch", &format!("issue-{}", issue)]);
        let run = bench.start(true, &["start", "build", &issue]);
        refused(&bench, &run, 1, &["#65"]);
    }
    // A path beside a classed one, or whose name only starts the same, is not classed.
    issue_file(&bench, "70", "[src/brakes.c, src/other/a.c, docs]");
    git(&bench.repo.path, &["branch", "issue-70"]);
    let run = bench.start(true, &["start", "build", "70"]);
    assert_eq!(code(&run), Some(1), "docs is above docs/safety.md: {}", text(&run.stderr));
    issue_file(&bench, "71", "[src/brakes.c, src/other/a.c, docs/other.md]");
    git(&bench.repo.path, &["branch", "issue-71"]);
    ok_in_view(&bench.start(true, &["start", "build", "71"]));
}

#[test]
fn test_WB_BUILD_19_with_no_classed_every_path_is_classed() {
    let bench = bench();
    bench.set("classed", "");
    issue_file(&bench, "54", "[README.md]");
    let run = bench.start(true, &["start", "build", "54"]);
    refused(&bench, &run, 1, &["#65", "classed"]);
    // An issue's file that changes nothing touches nothing classed.
    issue_file(&bench, "55", "[]");
    git(&bench.repo.path, &["branch", "issue-55"]);
    ok_in_view(&bench.start(true, &["start", "build", "55"]));
}

#[test]
fn test_WB_BUILD_19_a_classed_that_is_not_a_list_of_paths_is_refused_with_its_place() {
    let bench = bench();
    for line in ["classed: src", "classed: [/etc]", "classed: [a/../b]", "classed: ['']"] {
        bench.set("classed", line);
        let run = bench.start(true, &["start", "build", "54"]);
        refused(&bench, &run, 1, &["classed"]);
    }
}

#[test]
fn test_WB_BUILD_20_with_no_issue_s_file_a_role_is_refused_unless_classed_is_empty() {
    let bench = bench();
    for line in ["classed: [src]", ""] {
        bench.set("classed", line);
        for words in [&["start", "build", "54"][..], &["start", "review", "--change", "174"]] {
            let run = bench.start(true, words);
            refused(&bench, &run, 1, &["#65", "classed: []"]);
        }
        // The breakdown changes only the issue's file, and is not refused on the class.
        ok(&bench.start(true, &["start", "breakdown", "54"]));
        // `--prompt` is not refused on the class.
        ok(&bench.start(true, &["start", "build", "54", "--prompt"]));
        fs::remove_file(bench.at("calls")).unwrap();
    }
}

// ---- WB-BUILD-21: the patterns are lines for the shell, filled with plain values ----

#[test]
fn test_WB_BUILD_21_a_pattern_is_run_by_the_shell_in_the_root_of_the_checkout() {
    let bench = bench();
    let comments = bench.at("comments.sh");
    bench.set("issue_comments", &format!("issue_comments: 'true && {} issue {{issue}} | cat'", comments.display()));
    bench.comments("issue", "54", &["[stamp] build round 1"]);
    fs::create_dir_all(bench.repo.path.join("src/deep")).unwrap();
    ok(&start_with(&bench.repo.path.join("src/deep"), &bench.org, true, &["start", "build", "54"]));
    assert!(bench.posted("issue", "54").unwrap().starts_with("[stamp] build round 2\n"));
    assert_eq!(read(&bench.at("comments.pwd")).trim(), bench.root());
}

#[test]
fn test_WB_BUILD_21_a_value_given_that_is_not_plain_is_used_wrongly_by_its_option() {
    let bench = bench();
    for issue in ["a b", "54;true", "$(true)"] {
        let run = bench.start(true, &["start", "build", issue, "--branch", "issue-54", "--issue_address", "x"]);
        refused(&bench, &run, 2, &["--issue"]);
    }
    let run = bench.start(true, &["start", "review", "--change", "1 2", "--change_address", "x"]);
    refused(&bench, &run, 2, &["--change"]);
}

#[test]
fn test_WB_BUILD_21_a_branch_worked_out_that_is_not_plain_is_refused_with_1_saying_to_give_change() {
    let bench = bench();
    git(&bench.repo.path, &["branch", "-D", "issue-54"]);
    git(&bench.repo.path, &["branch", "issue-54-a/b"]);
    let run = bench.start(true, &["start", "review", "54"]);
    refused(&bench, &run, 1, &["issue-54-a/b", "--change"]);
}

// ---- WB-BUILD-22: a round given is a whole number from 1 ----

#[test]
fn test_WB_BUILD_22_a_round_that_is_not_a_whole_number_from_1_is_used_wrongly() {
    let bench = bench();
    for round in ["0", "-1", "x", "1.5", "01", " 1"] {
        let run = bench.start(true, &["start", "build", "54", "--round", round]);
        refused(&bench, &run, 2, &["--round"]);
        assert!(text(&run.stderr).ends_with(USAGE), "{}: {}", round, text(&run.stderr));
    }
}

// ---- WB-BUILD-23: the pattern that posts must be there ----

#[test]
fn test_WB_BUILD_23_with_no_issue_comment_nothing_is_posted_or_started() {
    let bench = bench();
    bench.set("issue_comment", "");
    let run = bench.start(true, &["start", "build", "54"]);
    refused(&bench, &run, 1, &["issue_comment"]);
}

// ---- WB-BUILD-24: a word too long to be handed to a program ----

#[test]
fn test_WB_BUILD_24_a_prompt_of_128_kib_or_more_is_refused_before_anything_is_posted() {
    let bench = bench();
    put(&bench.repo.path, ".watchbill/project/all.md", &format!("{}\n", "a".repeat(132 * 1024)));
    let run = bench.start(true, &["start", "build", "54"]);
    refused(&bench, &run, 1, &["--prompt"]);
}

// ---- WB-BUILD-25: an interrupt does not end `wb` while the session runs ----

#[test]
fn test_WB_BUILD_25_an_interrupt_and_a_quit_sent_while_the_session_runs_do_not_end_wb() {
    let bench = bench();
    let l = bench.log.path.to_str().unwrap();
    script(&bench.at("start.sh"), &format!("echo start >> {l}/calls\nkill -INT $PPID\nkill -QUIT $PPID\nsleep 0.2\nexit 3\n"));
    let run = bench.start(true, &["start", "build", "54"]);
    // Still here, with the session's status.
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(bench.started());
}

// ---- WB-MON: the lines a start leaves on the machine (#124) ----

/// Runs `wb <role>` on the bench with `state` as its state folder.
fn start_at(bench: &Bench, state: &Path, terminal: bool, args: &[&str]) -> Output {
    start_in(&bench.repo.path, &bench.org, terminal, Some(state), args)
}

/// The folders of the starts' lines below `state`, by name.
fn sessions(state: &Path) -> Vec<PathBuf> {
    let Ok(found) = fs::read_dir(state.join("watchbill/sessions")) else { return Vec::new() };
    let mut found: Vec<PathBuf> = found.map(|entry| entry.unwrap().path()).collect();
    found.sort();
    found
}

/// The one folder below `state`.
fn only(state: &Path) -> PathBuf {
    let found = sessions(state);
    assert_eq!(found.len(), 1, "{:?}", found);
    found[0].clone()
}

/// The lines of a file of a folder of a start.
fn lines_of(folder: &Path, name: &str) -> Vec<String> {
    read(&folder.join(name)).lines().map(str::to_string).collect()
}

/// Everything below a folder, files and folders.
fn everything(folder: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut open = vec![folder.to_path_buf()];
    while let Some(at) = open.pop() {
        for entry in fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                open.push(path.clone());
            }
            found.push(path);
        }
    }
    found
}

/// The time of a stamp as the name of a folder holds it: the line `time:`, with `:` left out.
fn time_of(stamp: &str) -> String {
    let line = stamp.lines().find(|line| line.starts_with("time: ")).expect("the stamp has no time");
    line["time: ".len()..].replace(':', "")
}

/// The name of a folder.
fn name_of(folder: &Path) -> String {
    folder.file_name().unwrap().to_str().unwrap().to_string()
}

#[test]
fn test_WB_MON_1_the_folder_is_below_the_state_folder_named_after_the_time_the_role_and_the_issue() {
    let bench = bench();
    let state = folder_named("state");
    ok(&start_at(&bench, &state.path, true, &["start", "build", "54"]));
    let folder = only(&state.path);
    let stamp = read(&folder.join("stamp"));
    assert_eq!(folder.parent().unwrap(), state.path.join("watchbill/sessions"));
    assert_eq!(name_of(&folder), format!("{}-build-54", time_of(&stamp)));
    // UTC, as in the stamp: `2026-11-19T094959Z`.
    let time = time_of(&stamp);
    assert_eq!(time.len(), "2026-11-19T094959Z".len(), "{}", time);
    assert!(time.ends_with('Z') && time.contains('T') && !time.contains(':'), "{}", time);
}

#[test]
fn test_WB_MON_1_a_role_started_with_no_issue_has_none_for_its_issue() {
    let bench = bench();
    let state = folder_named("state");
    ok(&start_at(&bench, &state.path, true, &["start", "review", "--change", "174"]));
    let folder = only(&state.path);
    assert!(name_of(&folder).ends_with("-review-none"), "{}", folder.display());
}

#[test]
fn test_WB_MON_1_two_starts_in_the_same_second_give_two_folders_and_none_is_written_over() {
    let bench = bench();
    let state = folder_named("state");
    // The folders a start could be named for the next seconds are there already, each with a file.
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let below = state.path.join("watchbill/sessions");
    for second in now..now + 20 {
        put(&below, &format!("{}-build-54/keep", watchbill::prompt::utc(second).replace(':', "")), "kept\n");
    }
    ok(&start_at(&bench, &state.path, true, &["start", "build", "54"]));
    let new: Vec<PathBuf> = sessions(&state.path).into_iter().filter(|path| !path.join("keep").exists()).collect();
    assert_eq!(new.len(), 1, "{:?}", sessions(&state.path));
    assert_eq!(name_of(&new[0]), format!("{}-build-54-2", time_of(&read(&new[0].join("stamp")))));
    // A second start in the same second as the first is `-3`, or `-2` of the next second.
    ok(&start_at(&bench, &state.path, true, &["start", "build", "54"]));
    let new: Vec<PathBuf> = sessions(&state.path).into_iter().filter(|path| !path.join("keep").exists()).collect();
    assert_eq!(new.len(), 2, "{:?}", sessions(&state.path));
    for folder in &new {
        let time = time_of(&read(&folder.join("stamp")));
        assert!([format!("{}-build-54-2", time), format!("{}-build-54-3", time)].contains(&name_of(folder)), "{}", folder.display());
    }
    // Every folder that was there holds what it held.
    for folder in sessions(&state.path).iter().filter(|path| path.join("keep").exists()) {
        assert_eq!(read(&folder.join("keep")), "kept\n");
        assert_eq!(everything(folder).len(), 1, "{}", folder.display());
    }
}

#[test]
fn test_WB_MON_1_two_starts_one_after_the_other_give_two_folders() {
    let bench = bench();
    let state = folder_named("state");
    ok(&start_at(&bench, &state.path, true, &["start", "build", "54"]));
    ok(&start_at(&bench, &state.path, true, &["start", "build", "54"]));
    assert_eq!(sessions(&state.path).len(), 2);
}

/// Runs the program, as a person does, with the environment of the tests and the variables given.
fn wb_with(folder: &Path, args: &[&str], set: &[(&str, &Path)], unset: &[&str]) -> Output {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_wb"));
    command.args(args).current_dir(folder).env_clear().envs(env());
    for name in unset {
        command.env_remove(name);
    }
    for (name, value) in set {
        command.env(name, value);
    }
    command.stdin(std::process::Stdio::null()).output().expect("wb could not be started")
}

#[test]
fn test_WB_MON_1_the_program_writes_below_xdg_state_home_when_it_is_set_and_else_below_the_home_folder() {
    // Through the program, standard input and output are not a terminal: the start is refused, at the
    // runner's `start` or at the terminal, and leaves its folder.
    let bench = bench();
    let (state, home) = (folder_named("state"), folder_named("home"));
    let words = ["start", "build", "54", "--runner", "stand-in"];
    let run = wb_with(&bench.repo.path, &words, &[("XDG_STATE_HOME", &state.path), ("HOME", &home.path)], &[]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert_eq!(lines_of(&only(&state.path), "outcome")[0], "outcome: refused");
    assert!(everything(&home.path).is_empty(), "{:?}", everything(&home.path));
    // With no XDG_STATE_HOME, or one that is not an absolute path, the home folder's `.local/state`.
    let run = wb_with(&bench.repo.path, &words, &[("HOME", &home.path)], &["XDG_STATE_HOME"]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert_eq!(sessions(&home.path.join(".local/state")).len(), 1);
    let run = wb_with(&bench.repo.path, &words, &[("HOME", &home.path), ("XDG_STATE_HOME", Path::new("state"))], &[]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert_eq!(sessions(&home.path.join(".local/state")).len(), 2);
    assert!(!bench.repo.path.join("state").exists());
    assert_eq!(sessions(&state.path).len(), 1);
}

#[test]
fn test_WB_MON_2_the_folder_is_said_on_standard_error_before_the_stamp_is_posted() {
    let bench = bench();
    let state = folder_named("state");
    let run = start_at(&bench, &state.path, true, &["start", "build", "54"]);
    ok(&run);
    let line = format!("wb start build: will write the lines of this start in {}", only(&state.path).display());
    let err = text(&run.stderr);
    assert!(err.lines().any(|said| said == line), "{}", err);
    assert!(err.find(&line).unwrap() < err.find("will post").unwrap(), "{}", err);
    // And for a start that is refused.
    let other = folder_named("state");
    let run = start_at(&bench, &other.path, false, &["start", "build", "54"]);
    let line = format!("wb start build: will write the lines of this start in {}", only(&other.path).display());
    assert!(text(&run.stderr).lines().any(|said| said == line), "{}", text(&run.stderr));
}

#[test]
fn test_WB_MON_3_the_stamp_is_byte_for_byte_what_issue_comment_was_given() {
    let bench = bench();
    let state = folder_named("state");
    bench.changes("issue-54", &["174"]);
    bench.comments("issue", "54", &["[stamp] review round 1"]);
    ok(&start_at(&bench, &state.path, true, &["start", "fix", "54"]));
    let posted = fs::read(bench.at("posted-issue-54")).unwrap();
    assert_eq!(fs::read(only(&state.path).join("stamp")).unwrap(), posted);
}

#[test]
fn test_WB_MON_3_a_posting_that_fails_holds_the_stamp_that_was_given() {
    let bench = bench();
    let state = folder_named("state");
    bench.ends_with("post", 1);
    start_at(&bench, &state.path, true, &["start", "build", "54"]);
    let posted = fs::read(bench.at("posted-issue-54")).unwrap();
    assert_eq!(fs::read(only(&state.path).join("stamp")).unwrap(), posted);
}

#[test]
fn test_WB_MON_3_a_start_refused_after_the_checks_of_wb_prompt_holds_the_stamp_that_would_have_been_posted() {
    let bench = bench();
    let state = folder_named("state");
    let run = start_at(&bench, &state.path, false, &["start", "build", "54"]);
    assert_eq!(code(&run), Some(1));
    let stamp = read(&only(&state.path).join("stamp"));
    let (first, rest) = stamp.split_once('\n').unwrap();
    assert_eq!(first, "[stamp] build round 1");
    let by_prompt = bench.prompt(&["build", "54", "--round", "1", "--runner", "stand-in", "--stamp"]);
    ok(&by_prompt);
    assert_eq!(timeless(rest), timeless(&text(&by_prompt.stdout)));
    assert!(!bench.any_posted());
}

#[test]
fn test_WB_MON_3_a_start_refused_at_the_checks_of_wb_prompt_holds_no_stamp() {
    let bench = bench();
    let state = folder_named("state");
    put(&bench.repo.path, ".watchbill/issues/54.yml", &BARE_ISSUE.replace("{issue}", "54"));
    let run = start_at(&bench, &state.path, true, &["start", "build", "54"]);
    assert_eq!(code(&run), Some(1));
    let folder = only(&state.path);
    assert!(!folder.join("stamp").exists());
    assert_eq!(lines_of(&folder, "outcome")[..2], ["outcome: refused", "check: prompt"]);
}

/// The outcome of a start refused at `check`: status 1, and a message that starts with `message`, the first
/// line of what was said on standard error.
fn refused_at(bench: &Bench, run: &Output, state: &Path, check: &str, message: &str) {
    assert_eq!(code(run), Some(1), "{}: {}", check, text(&run.stderr));
    assert!(!bench.any_posted() && !bench.started(), "{}: {:?}", check, bench.calls());
    let outcome = lines_of(&only(state), "outcome");
    assert_eq!(outcome.len(), 4, "{}: {:?}", check, outcome);
    assert_eq!(outcome[..3], ["outcome: refused".to_string(), format!("check: {}", check), "status: 1".to_string()]);
    let said = outcome[3].strip_prefix("message: ").unwrap_or_else(|| panic!("{}: {:?}", check, outcome));
    assert!(said.starts_with(message), "{}: {:?}", check, outcome);
    let err = text(&run.stderr);
    assert!(err.lines().any(|line| line.ends_with(&format!(": {}", said))), "{}: {}", check, err);
}

#[test]
fn test_WB_MON_4_a_start_refused_at_the_checks_of_wb_prompt_names_prompt() {
    let bench = bench();
    let state = folder_named("state");
    let role = bench.repo.path.join(".watchbill/revision/roles/build.md");
    let mut changed = read(&role);
    changed.push_str("\nAnother line.\n");
    fs::write(&role, changed).unwrap();
    let run = start_at(&bench, &state.path, true, &["start", "build", "54"]);
    refused_at(&bench, &run, &state.path, "prompt", "nothing printed, the check of `wb check` does not hold");
}

#[test]
fn test_WB_MON_4_a_start_refused_at_the_rule_table_names_rule() {
    let bench = bench();
    let state = folder_named("state");
    bench.changes("issue-54", &["174"]);
    bench.comments("issue", "54", &["[stamp] build round 1"]);
    let run = start_at(&bench, &state.path, true, &["start", "fix", "54", "--round", "1"]);
    refused_at(&bench, &run, &state.path, "rule", "a fix follows a review");
}

#[test]
fn test_WB_MON_4_a_start_refused_at_the_class_names_class() {
    let bench = bench();
    let state = folder_named("state");
    issue_file(&bench, "54", "[src/brake/pedal.c]");
    bench.set("classed", "classed: [src/brake]");
    let run = start_at(&bench, &state.path, true, &["start", "build", "54"]);
    refused_at(&bench, &run, &state.path, "class", "the issue's `changes` hold classed code");
}

#[test]
fn test_WB_MON_4_a_start_refused_at_the_runner_s_start_names_start() {
    let bench = bench();
    let state = folder_named("state");
    let run = start_at(&bench, &state.path, true, &["start", "build", "54", "--runner", "idle"]);
    refused_at(&bench, &run, &state.path, "start", "the runner 'idle' has no `start`");
}

#[test]
fn test_WB_MON_4_a_start_refused_at_the_terminal_names_terminal() {
    let bench = bench();
    let state = folder_named("state");
    let run = start_at(&bench, &state.path, false, &["start", "build", "54"]);
    refused_at(&bench, &run, &state.path, "terminal", "standard input or standard output is not a terminal");
}

#[test]
fn test_WB_MON_4_a_posting_that_fails_gives_not_posted_and_no_session() {
    let bench = bench();
    let state = folder_named("state");
    bench.ends_with("post", 1);
    let run = start_at(&bench, &state.path, true, &["start", "build", "54"]);
    assert_eq!(code(&run), Some(1));
    assert!(!bench.started(), "{:?}", bench.calls());
    let outcome = lines_of(&only(&state.path), "outcome");
    assert_eq!(outcome.len(), 3, "{:?}", outcome);
    assert_eq!(outcome[..2], ["outcome: not posted", "status: 1"], "{:?}", outcome);
    assert!(outcome[2].starts_with("message: `issue_comment` of watchbill.yml ended with 1"), "{:?}", outcome);
}

#[test]
fn test_WB_MON_4_a_session_that_ends_with_0_or_3_gives_ended_with_that_status_and_its_seconds() {
    for status in [0, 3] {
        let bench = bench();
        let state = folder_named("state");
        bench.ends_with("start", status);
        let run = start_at(&bench, &state.path, true, &["start", "build", "54"]);
        assert_eq!(code(&run), Some(if status == 0 { 0 } else { 1 }));
        let outcome = lines_of(&only(&state.path), "outcome");
        assert_eq!(outcome.len(), 3, "{:?}", outcome);
        assert_eq!(outcome[..2], ["outcome: ended".to_string(), format!("status: {}", status)]);
        let seconds = outcome[2].strip_prefix("seconds: ").unwrap();
        assert!(seconds.parse::<u64>().unwrap() < 60, "{:?}", outcome);
    }
}

#[test]
fn test_WB_MON_4_the_seconds_are_those_the_session_ran() {
    let bench = bench();
    let state = folder_named("state");
    let l = bench.log.path.to_str().unwrap();
    script(&bench.at("start.sh"), &format!("echo start >> {l}/calls\nsleep 2\nexit 0\n"));
    ok(&start_at(&bench, &state.path, true, &["start", "build", "54"]));
    let outcome = lines_of(&only(&state.path), "outcome");
    let seconds: u64 = outcome[2].strip_prefix("seconds: ").unwrap().parse().unwrap();
    assert!((2..10).contains(&seconds), "{:?}", outcome);
}

#[test]
fn test_WB_MON_5_a_state_folder_that_cannot_be_written_changes_neither_the_start_nor_the_status() {
    for status in [0, 3] {
        let bench = bench();
        let place = folder_named("state");
        // A file where the state folder would be: nothing can be made below it.
        let state = place.path.join("a-file");
        fs::write(&state, "").unwrap();
        bench.ends_with("start", status);
        let run = start_at(&bench, &state, true, &["start", "build", "54"]);
        assert_eq!(code(&run), Some(if status == 0 { 0 } else { 1 }), "{}", text(&run.stderr));
        assert!(bench.any_posted() && bench.started(), "{:?}", bench.calls());
        let err = text(&run.stderr);
        assert!(err.contains("could not be written") && err.contains(state.to_str().unwrap()), "{}", err);
        assert_eq!(read(&state), "");
    }
}

#[test]
fn test_WB_MON_5_a_state_folder_that_cannot_be_written_changes_no_refusal() {
    let bench = bench();
    let place = folder_named("state");
    let state = place.path.join("a-file");
    fs::write(&state, "").unwrap();
    let run = start_at(&bench, &state, false, &["start", "build", "54"]);
    assert_eq!(code(&run), Some(1));
    assert!(text(&run.stderr).contains("not a terminal"), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("could not be written"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_MON_5_with_no_state_folder_known_the_session_still_starts() {
    let bench = bench();
    let run = start_in(&bench.repo.path, &bench.org, true, None, &["start", "build", "54"]);
    ok(&run);
    assert!(bench.started());
    assert!(text(&run.stderr).contains("no lines of this start are written"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_MON_6_wb_role_prompt_stamp_and_parts_write_nothing() {
    let bench = bench();
    let state = folder_named("state");
    for extra in [&["--prompt"][..], &["--stamp"], &["--parts"], &["--prompt", "--stamp"]] {
        let mut words = vec!["start", "build", "54"];
        words.extend(extra);
        ok(&start_at(&bench, &state.path, true, &words));
        ok(&start_at(&bench, &state.path, false, &words));
    }
    assert!(everything(&state.path).is_empty(), "{:?}", everything(&state.path));
}

#[test]
fn test_WB_MON_6_the_program_s_wb_prompt_wb_check_and_wb_role_prompt_write_nothing_anywhere() {
    let bench = bench();
    let (state, home) = (folder_named("state"), folder_named("home"));
    let set = [("XDG_STATE_HOME", state.path.as_path()), ("HOME", home.path.as_path())];
    for words in [&["prompt", "build", "54"][..], &["start", "build", "54", "--prompt"], &["start", "build", "54", "--parts"], &["check"]] {
        ok(&wb_with(&bench.repo.path, words, &set, &[]));
    }
    assert!(everything(&state.path).is_empty(), "{:?}", everything(&state.path));
    assert!(everything(&home.path).is_empty(), "{:?}", everything(&home.path));
    ok(&wb_with(&bench.repo.path, &["start", "build", "54", "--prompt"], &[("HOME", home.path.as_path())], &["XDG_STATE_HOME"]));
    assert!(everything(&home.path).is_empty(), "{:?}", everything(&home.path));
}

// ---- WB-BUILD-27: the stamp's `round:` line holds the round of its first line, for every role ----

/// The first line of a posted stamp is `[stamp] <role> round <n>` and its line `round:` says `n`.
fn agree(posted: &str, role: &str, round: usize) {
    let (first, rest) = posted.split_once('\n').unwrap();
    assert_eq!(first, format!("[stamp] {} round {}", role, round), "{}", posted);
    assert!(rest.lines().any(|line| line == format!("round: {}", round)), "{}", posted);
}

#[test]
fn test_WB_BUILD_27_the_round_line_agrees_with_the_first_line_for_each_role_with_the_round_worked_out() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.comments("issue", "54", &["[stamp] breakdown round 1", "[stamp] tester round 1", "[stamp] tester round 2", "[stamp] review round 1", "[stamp] review round 2"]);
    for (role, round) in [("breakdown", 2), ("tester", 3), ("build", 1), ("review", 3), ("fix", 2)] {
        ok(&bench.start(true, &["start", role, "54"]));
        agree(&bench.posted("issue", "54").unwrap(), role, round);
    }
}

#[test]
fn test_WB_BUILD_27_the_round_line_agrees_with_the_first_line_for_a_build_with_a_round_given() {
    let bench = bench();
    ok(&bench.start(true, &["start", "build", "54", "--round", "1"]));
    agree(&bench.posted("issue", "54").unwrap(), "build", 1);
    let run = bench.prompt(&["build", "54", "--round", "1", "--stamp"]);
    ok(&run);
    assert!(text(&run.stdout).lines().any(|line| line == "round: 1"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_BUILD_27_the_round_line_agrees_with_the_first_line_for_a_review_of_a_change_with_no_issue() {
    let bench = bench();
    bench.comments("change", "174", &["[stamp] review round 1"]);
    ok(&bench.start(true, &["start", "review", "--change", "174"]));
    agree(&bench.posted("change", "174").unwrap(), "review", 2);
}

// ---- WB-PROMPT-55: `--stamp` holds the round for every role, where there is one ----

#[test]
fn test_WB_PROMPT_55_a_stamp_keeps_change_empty_for_a_role_whose_text_has_no_change() {
    let bench = bench();
    let run = bench.prompt(&["build", "54", "--round", "1", "--change", "9", "--stamp"]);
    ok(&run);
    let out = text(&run.stdout);
    assert!(out.lines().any(|line| line == "change:") && out.lines().any(|line| line == "round: 1"), "{}", out);
}

#[test]
fn test_WB_PROMPT_55_a_stamp_of_the_build_without_a_round_leaves_it_empty_and_runs_no_pattern() {
    let bench = bench();
    bench.comments("issue", "54", &["[stamp] build round 1"]);
    let run = bench.prompt(&["build", "54", "--stamp"]);
    ok(&run);
    assert!(text(&run.stdout).lines().any(|line| line == "round:"), "{}", text(&run.stdout));
    assert!(!bench.calls().iter().any(|call| call.starts_with("comments")), "{:?}", bench.calls());
}

// ---- WB-BUILD-14 to 16 and WB-MON-8: the review starts only on a change whose CI is green (#127) ----

impl Bench {
    /// A stand-in `change_checks`: it logs the change it was run for, prints `said`, and ends with `status`.
    fn checks(&self, said: &str, status: i32) {
        let l = self.log.path.to_str().unwrap();
        script(&self.at("checks.sh"), &format!("echo \"checks $1\" >> {l}/calls\nprintf '%s' '{said}'\nexit {status}\n"));
        self.set("change_checks", &format!("change_checks: '{} {{change}}'", self.at("checks.sh").display()));
    }

    fn checks_read(&self) -> Vec<String> {
        self.calls().into_iter().filter(|call| call.starts_with("checks ")).collect()
    }
}

#[test]
fn test_WB_BUILD_14_a_green_ci_starts_the_review_and_is_read_for_the_change_before_the_stamp_is_posted() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.checks("green\n", 0);
    ok(&bench.start(true, &["start", "review", "54"]));
    let calls = bench.calls();
    let at = |call: &str| calls.iter().position(|one| one == call).unwrap_or_else(|| panic!("{} not in {:?}", call, calls));
    assert!(at("checks 174") < at("post issue 54") && at("post issue 54") < at("start"), "{:?}", calls);
}

#[test]
fn test_WB_BUILD_14_a_pending_ci_is_refused_with_1_saying_to_wait() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.checks("pending\n", 0);
    let run = bench.start(true, &["start", "review", "54"]);
    refused(&bench, &run, 1, &["pending", "wait"]);
    assert_eq!(bench.checks_read(), ["checks 174"]);
}

#[test]
fn test_WB_BUILD_14_a_red_ci_is_refused_with_1_saying_it_goes_to_a_person_and_the_lists_or_stubs_may_be_wrong() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.checks("red\n", 0);
    let run = bench.start(true, &["start", "review", "54"]);
    refused(&bench, &run, 1, &["red", "goes to a person", "lists or the stubs"]);
}

#[test]
fn test_WB_BUILD_14_a_review_of_a_change_with_no_issue_reads_the_ci_of_the_change_given() {
    let bench = bench();
    bench.checks("red\n", 0);
    let run = bench.start(true, &["start", "review", "--change", "175"]);
    refused(&bench, &run, 1, &["red"]);
    assert_eq!(bench.checks_read(), ["checks 175"]);
}

#[test]
fn test_WB_BUILD_14_only_the_review_reads_the_ci() {
    let bench = bench();
    bench.checks("red\n", 0);
    ok(&bench.start(true, &["start", "build", "54"]));
    assert!(bench.checks_read().is_empty(), "{:?}", bench.calls());
}

#[test]
fn test_WB_BUILD_14_review_with_prompt_reads_no_ci() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.checks("red\n", 0);
    ok(&bench.start(true, &["start", "review", "54", "--prompt"]));
    assert!(bench.checks_read().is_empty(), "{:?}", bench.calls());
}

#[test]
fn test_WB_BUILD_15_a_word_other_than_green_red_or_pending_is_refused_naming_change_checks() {
    for said in ["", "passed\n", "GREEN\n", "green red\n", "green\ngreen\n", "greenish\n"] {
        let bench = bench();
        bench.changes("issue-54", &["174"]);
        bench.checks(said, 0);
        let run = bench.start(true, &["start", "review", "54"]);
        refused(&bench, &run, 1, &["change_checks", "green, red or pending"]);
    }
}

#[test]
fn test_WB_BUILD_15_a_check_that_ends_with_anything_but_0_is_refused_naming_change_checks_also_when_it_printed_green() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.checks("green\n", 3);
    let run = bench.start(true, &["start", "review", "54"]);
    refused(&bench, &run, 1, &["change_checks", "ended with 3"]);
}

#[test]
fn test_WB_BUILD_15_a_word_with_spaces_or_a_line_end_around_it_is_read() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.checks(" green \r\n", 0);
    ok(&bench.start(true, &["start", "review", "54"]));
    assert_eq!(bench.checks_read(), ["checks 174"]);
}

#[test]
fn test_WB_BUILD_16_with_no_change_checks_the_review_starts_and_says_the_ci_was_not_read() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    let run = bench.start(true, &["start", "review", "54"]);
    ok(&run);
    assert!(bench.started());
    assert!(text(&run.stderr).contains("the CI was not read"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_BUILD_16_a_start_that_reads_the_ci_does_not_say_it_was_not_read() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.checks("green\n", 0);
    let run = bench.start(true, &["start", "review", "54"]);
    ok(&run);
    assert!(!text(&run.stderr).contains("was not read"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_MON_8_the_folder_of_a_review_holds_ci_with_the_word_printed() {
    for word in ["green", "red", "pending"] {
        let bench = bench();
        let state = folder_named("state");
        bench.changes("issue-54", &["174"]);
        bench.checks(&format!("{}\n", word), 0);
        let run = start_at(&bench, &state.path, true, &["start", "review", "54"]);
        assert_eq!(code(&run), Some(if word == "green" { 0 } else { 1 }), "{}", text(&run.stderr));
        let folder = only(&state.path);
        assert_eq!(read(&folder.join("ci")), format!("ci: {}\n", word));
        // The stamp that would have been posted is there too, whatever the outcome.
        assert!(folder.join("stamp").exists());
    }
}

#[test]
fn test_WB_MON_8_a_review_with_no_change_checks_leaves_ci_not_read_and_another_role_leaves_no_ci() {
    let bench = bench();
    let state = folder_named("state");
    bench.changes("issue-54", &["174"]);
    ok(&start_at(&bench, &state.path, true, &["start", "review", "54"]));
    assert_eq!(read(&only(&state.path).join("ci")), "ci: not read\n");
    let other = folder_named("other");
    ok(&start_at(&bench, &other.path, true, &["start", "build", "54"]));
    assert!(!only(&other.path).join("ci").exists());
}

#[test]
fn test_WB_MON_8_a_check_that_fails_leaves_ci_with_the_fault() {
    let bench = bench();
    let state = folder_named("state");
    bench.changes("issue-54", &["174"]);
    bench.checks("passed\n", 0);
    assert_eq!(code(&start_at(&bench, &state.path, true, &["start", "review", "54"])), Some(1));
    assert_eq!(read(&only(&state.path).join("ci")), "ci: unread\n");
}

#[test]
fn test_WB_MON_4_a_review_refused_at_the_ci_leaves_check_ci_and_status_1_in_outcome() {
    let bench = bench();
    let state = folder_named("state");
    bench.changes("issue-54", &["174"]);
    bench.checks("red\n", 0);
    assert_eq!(code(&start_at(&bench, &state.path, true, &["start", "review", "54"])), Some(1));
    let lines = lines_of(&only(&state.path), "outcome");
    assert_eq!(&lines[..3], ["outcome: refused", "check: ci", "status: 1"]);
    assert!(lines[3].starts_with("message: the CI of change 174 is red"), "{:?}", lines);
}

#[test]
fn test_WB_BUILD_14_a_review_refused_at_the_ci_makes_no_view() {
    let bench = bench();
    let state = folder_named("state");
    issue_file(&bench, "54", "[]");
    bench.changes("issue-54", &["174"]);
    bench.checks("red\n", 0);
    let run = start_at(&bench, &state.path, true, &["start", "review", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    let folder = only(&state.path);
    assert!(!folder.join("view").exists() && !folder.join("back").exists(), "{:?}", everything(&folder));
    assert!(!text(&run.stderr).contains("bubblewrap"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_BUILD_14_the_prompt_of_a_started_review_carries_nothing_of_the_ci() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.checks("green\n", 0);
    let printed = bench.start(true, &["start", "review", "54", "--prompt"]);
    ok(&printed);
    ok(&bench.start(true, &["start", "review", "54"]));
    assert_eq!(read(&bench.at("start.prompt")), text(&printed.stdout));
}

// ---- WB-BUILD-36 to 38: `wb start <role> <issue>` starts one role (#175) ----

#[test]
fn test_WB_BUILD_36_wb_start_build_does_what_wb_build_did_the_stamp_and_the_start() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    ok(&bench.start(true, &["start", "build", "54"]));
    assert!(bench.any_posted() && bench.started(), "{:?}", bench.calls());
}

#[test]
fn test_WB_BUILD_36_wb_start_build_makes_the_checks_it_made_and_a_fault_ends_it_before_the_stamp() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.set("classed", "classed: [src]");
    let run = bench.start(true, &["start", "build", "54"]);
    refused(&bench, &run, 1, &["#65", "classed"]);
}

#[test]
fn test_WB_BUILD_36_wb_start_review_with_a_change_and_no_issue_works_as_before() {
    let bench = bench();
    ok(&bench.start(true, &["start", "review", "--change", "178"]));
    assert!(bench.any_posted() && bench.started(), "{:?}", bench.calls());
}

#[test]
fn test_WB_BUILD_37_the_old_form_ends_with_2_and_says_the_new_form_with_the_same_words() {
    let bench = bench();
    for (words, form) in [
        (&["build", "54"][..], "use: wb start build 54"),
        (&["breakdown", "54"], "use: wb start breakdown 54"),
        (&["tester", "54", "--runner", "stand-in"], "use: wb start tester 54 --runner stand-in"),
        (&["review", "--change", "178"], "use: wb start review --change 178"),
        (&["fix", "54", "--prompt"], "use: wb start fix 54 --prompt"),
    ] {
        let run = bench.start(true, words);
        refused(&bench, &run, 2, &[form]);
        assert_eq!(text(&run.stdout), "", "{:?}", words);
        // The program says the same, as a person meets it.
        let run = wb(&bench.repo.path, words, "");
        assert_eq!(code(&run), Some(2), "{:?}: {}", words, text(&run.stderr));
        assert!(text(&run.stderr).contains(form), "{:?}: {}", words, text(&run.stderr));
        assert!(!bench.any_posted() && !bench.started(), "{:?}", bench.calls());
    }
}

#[test]
fn test_WB_BUILD_38_a_role_that_is_not_known_after_start_ends_with_2_and_names_the_roles() {
    let bench = bench();
    for words in [&["start", "nothing", "54"][..], &["start"], &["start", "--prompt"], &["start", "prompt", "54"]] {
        let run = bench.start(true, words);
        refused(&bench, &run, 2, &["breakdown", "tester", "build", "review", "fix"]);
        let run = wb(&bench.repo.path, words, "");
        assert_eq!(code(&run), Some(2), "{:?}: {}", words, text(&run.stderr));
        // The first line is the message of `wb start`; the usage follows it.
        let err = text(&run.stderr);
        let first = err.lines().next().unwrap();
        assert!(first.starts_with("wb start: "), "{:?}: {}", words, err);
        for role in ["breakdown", "tester", "build", "review", "fix"] {
            assert!(first.contains(role), "{:?}: {}", words, err);
        }
        assert!(!bench.any_posted() && !bench.started(), "{:?}", bench.calls());
    }
}
