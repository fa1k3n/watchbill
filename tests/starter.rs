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

/// A repository made for a test, the folder of its stand-ins, and the organisation's file.
struct Bench {
    repo: Folder,
    log: Folder,
    org: PathBuf,
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

/// A repository set up by `wb init --yes`, with every pattern a stand-in, `classed: []`, the level `public`,
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
        "classed: []".to_string(),
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
    let path = log.path.join("organisation.yml");
    fs::write(&path, org(&log.path, Some("stand-in"))).unwrap();
    Bench { repo, log, org: path }
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
    let run = wb(&bench.repo.path, &["accept", "--", issue], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    git(&bench.repo.path, &["add", "--", ".watchbill/issues"]);
    git(&bench.repo.path, &["commit", "-q", "-m", "The issue's file"]);
}

// ---- WB-BUILD-1: what `wb prompt` refuses, `wb <role>` refuses with the same message and status ----

fn same_as_prompt(bench: &Bench, role: &str, words: &[&str]) {
    let mut started = vec![role];
    started.extend(words);
    let mut prompted = vec![role];
    prompted.extend(words);
    let by_prompt = bench.prompt(&prompted);
    let by_start = bench.start(true, &started);
    assert_ne!(code(&by_prompt), Some(0), "{:?}: wb prompt did not refuse", words);
    assert_eq!(code(&by_start), code(&by_prompt), "{:?}: {}", words, text(&by_start.stderr));
    assert_eq!(text(&by_start.stdout), "");
    let said = text(&by_prompt.stderr).replace("wb prompt:", &format!("wb {}:", role));
    assert_eq!(text(&by_start.stderr), said, "{:?}", words);
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
    for words in [&["fix", "54"][..], &["fix", "54", "--round", "1"]] {
        let run = bench.start(true, words);
        refused(&bench, &run, 1, &["a fix follows a review", "no review stamp"]);
    }
}

#[test]
fn test_WB_BUILD_2_a_fix_whose_rule_cannot_be_checked_for_want_of_issue_comments_is_refused() {
    let bench = bench();
    bench.set("issue_comments", "");
    let run = bench.start(true, &["fix", "54", "--round", "1", "--change", "174"]);
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
    let run = bench.start(true, &["review", "54"]);
    ok(&run);
    assert!(bench.started());
}

// ---- WB-BUILD-3: classed code in `changes` is refused ----

#[test]
fn test_WB_BUILD_3_an_issue_whose_changes_hold_classed_code_is_refused_naming_65() {
    let bench = bench();
    issue_file(&bench, "54", "[src/brake/pedal.c]");
    bench.set("classed", "classed: [src/brake]");
    let run = bench.start(true, &["build", "54"]);
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
    let words = ["fix", "54", "--runner", "idle"];
    refused(&bench, &bench.start(false, &words), 1, &["a fix follows a review"]);
    bench.comments("issue", "54", &["[stamp] review round 1"]);
    refused(&bench, &bench.start(false, &words), 1, &["#65", "classed"]);
    bench.set("classed", "classed: []");
    refused(&bench, &bench.start(false, &words), 1, &["idle", "--prompt"]);
    refused(&bench, &bench.start(false, &["fix", "54"]), 1, &["terminal", "#65"]);
    ok(&bench.start(true, &["fix", "54"]));
    assert!(bench.started());
}

#[test]
fn test_WB_BUILD_4_a_check_of_wb_prompt_comes_before_the_rule_table() {
    let bench = bench();
    put(&bench.repo.path, ".watchbill/issues/54.yml", &BARE_ISSUE.replace("{issue}", "54"));
    // No review stamp either: the issue's file is named, not the rule.
    let run = bench.start(true, &["fix", "54", "--round", "1", "--change", "174"]);
    refused(&bench, &run, 1, &["not accepted"]);
    assert!(!text(&run.stderr).contains("a fix follows a review"), "{}", text(&run.stderr));
}

// ---- WB-BUILD-5: the stamp is said and posted by `issue_comment`, with its first line ----

#[test]
fn test_WB_BUILD_5_the_stamp_is_posted_on_the_issue_with_its_first_line_and_the_lines_of_wb_prompt_stamp() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.comments("issue", "54", &["[review] 1. A finding", "[stamp] review round 1", "[stamp] build round 1", "hello"]);
    let run = bench.start(true, &["review", "54"]);
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
    let run = bench.start(true, &["review", "54"]);
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
    let run = bench.start(true, &["build", "54"]);
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
    let run = bench.start(true, &["review", "54"]);
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
    let run = start_with(&bench.repo.path.join("src/deep"), &bench.org, true, &["build", "54"]);
    ok(&run);
    assert_eq!(read(&bench.at("start.pwd")).trim(), bench.root());
}

#[test]
fn test_WB_BUILD_6_the_breakdown_starts_with_the_model_and_effort_its_runner_is_cleared_with() {
    let bench = bench();
    ok(&bench.start(true, &["breakdown", "54"]));
    assert_eq!(read(&bench.at("start.model")), "--model=claude-opus-5-5");
    assert_eq!(read(&bench.at("start.effort")), "high");
    assert!(bench.posted("issue", "54").unwrap().starts_with("[stamp] breakdown round 1\n"));
}

#[test]
fn test_WB_BUILD_6_the_status_of_the_session_is_0_for_0_and_1_otherwise() {
    let bench = bench();
    ok(&bench.start(true, &["build", "54"]));
    for status in [1, 3] {
        bench.ends_with("start", status);
        let run = bench.start(true, &["build", "54"]);
        assert_eq!(code(&run), Some(1), "{}: {}", status, text(&run.stderr));
    }
}

#[test]
fn test_WB_BUILD_6_a_start_that_cannot_be_run_ends_with_1_and_names_it() {
    let bench = bench();
    bench.organisation(&org(&bench.log.path, Some("stand-in")).replace("/start.sh", "/no-such-program"));
    let run = bench.start(true, &["build", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("no-such-program"), "{}", text(&run.stderr));
}

// ---- WB-BUILD-7: no terminal, and a runner without `start` ----

#[test]
fn test_WB_BUILD_7_with_no_terminal_it_is_refused_with_1_naming_65() {
    let bench = bench();
    let run = bench.start(false, &["build", "54"]);
    refused(&bench, &run, 1, &["terminal", "#65"]);
}

#[test]
fn test_WB_BUILD_7_a_runner_without_start_is_refused_with_1_naming_prompt() {
    let bench = bench();
    let run = bench.start(true, &["build", "54", "--runner", "idle"]);
    refused(&bench, &run, 1, &["idle", "start", "--prompt"]);
}

#[test]
fn test_WB_BUILD_7_a_runner_the_organisation_s_file_does_not_name_is_refused_with_1() {
    let bench = bench();
    let run = bench.start(true, &["build", "54", "--runner", "other"]);
    refused(&bench, &run, 1, &["other", "stand-in", "idle"]);
}

#[test]
fn test_WB_BUILD_7_the_program_refuses_without_a_terminal() {
    // Through the program, standard input and output are pipes: the organisation's file of this machine
    // decides how far it gets, and whatever it is, nothing is started and the status is not 0.
    let bench = bench();
    let run = wb(&bench.repo.path, &["build", "54", "--runner", "stand-in"], "");
    assert_ne!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(!bench.started(), "{:?}", bench.calls());
}

// ---- WB-BUILD-8: `--prompt` prints what `wb prompt` prints ----

#[test]
fn test_WB_BUILD_8_prompt_prints_what_wb_prompt_prints_and_posts_and_starts_nothing() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    for words in [&["build", "54"][..], &["review", "54"], &["tester", "54", "--requirements", "R-1"], &["review", "--change", "174"]] {
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
        let mut started = vec!["build", "54"];
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
        let mut started = words.to_vec();
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
    ok(&bench.start(true, &["fix", "54"]));
    assert!(!bench.at("network").exists(), "git asked origin");
    assert_eq!(bench.calls(), ["comments issue 54", "changes issue-54", "post issue 54", "start"]);
}

// ---- WB-BUILD-10: the runner: `--runner`, else the organisation's `default` ----

#[test]
fn test_WB_BUILD_10_with_no_runner_and_no_default_it_is_refused_with_2_and_the_usage() {
    let bench = bench();
    bench.organisation(&org(&bench.log.path, None));
    let run = bench.start(true, &["build", "54"]);
    refused(&bench, &run, 2, &["--runner", "default"]);
    assert!(text(&run.stderr).ends_with(USAGE), "{}", text(&run.stderr));
}

#[test]
fn test_WB_BUILD_10_with_no_runner_and_no_organisation_s_file_it_is_refused_with_2_naming_the_file() {
    let bench = bench();
    fs::remove_file(&bench.org).unwrap();
    let run = bench.start(true, &["build", "54"]);
    refused(&bench, &run, 2, &["--runner", bench.org.to_str().unwrap()]);
    assert!(text(&run.stderr).ends_with(USAGE), "{}", text(&run.stderr));
}

#[test]
fn test_WB_BUILD_10_the_default_is_taken_and_a_runner_given_wins() {
    let bench = bench();
    ok(&bench.start(true, &["build", "54"]));
    assert!(bench.posted("issue", "54").unwrap().contains("\nrunner: stand-in\n"));
    // The default has no `start`; the runner given has, and wins.
    bench.organisation(&org(&bench.log.path, Some("idle")));
    let run = bench.start(true, &["build", "54"]);
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("idle"), "{}", text(&run.stderr));
    ok(&bench.start(true, &["build", "54", "--runner", "stand-in"]));
    assert!(bench.posted("issue", "54").unwrap().contains("\nrunner: stand-in\n"));
}

#[test]
fn test_WB_CONF_24_a_default_that_names_no_runner_is_refused_with_the_place() {
    let bench = bench();
    bench.organisation(&org(&bench.log.path, Some("other")));
    let run = bench.start(true, &["build", "54"]);
    refused(&bench, &run, 1, &["default", "other"]);
}

// ---- WB-BUILD-11: the round, from the stamps ----

#[test]
fn test_WB_BUILD_11_the_review_takes_one_more_than_the_review_stamps_and_the_fix_their_number() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    bench.comments("issue", "54", &["[stamp] build round 1", "[stamp] review round 1", "[review] 1. must: a finding"]);
    ok(&bench.start(true, &["review", "54"]));
    assert!(bench.posted("issue", "54").unwrap().starts_with("[stamp] review round 2\n"));
    ok(&bench.start(true, &["fix", "54"]));
    assert!(bench.posted("issue", "54").unwrap().starts_with("[stamp] fix round 1\n"));
}

#[test]
fn test_WB_BUILD_11_the_tester_the_build_and_the_breakdown_take_one_more_than_their_own_stamps() {
    let bench = bench();
    bench.comments("issue", "54", &["[stamp] breakdown round 1", "[stamp] tester round 1", "[stamp] tester round 2", "[stamp] review round 1"]);
    for (role, round) in [("tester", 3), ("build", 1), ("breakdown", 2)] {
        ok(&bench.start(true, &[role, "54"]));
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
    let run = bench.start(true, &["build", "54"]);
    refused(&bench, &run, 1, &["issue_comments", "--round"]);
    ok(&bench.start(true, &["build", "54", "--round", "1"]));
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
    let run = bench.start(true, &["build", "54"]);
    refused(&bench, &run, 1, &["issue_comments"]);
}

// ---- WB-BUILD-12: the change, from `change_of_branch` ----

#[test]
fn test_WB_BUILD_12_the_one_open_change_of_the_branch_is_taken() {
    let bench = bench();
    bench.changes("issue-54", &["174"]);
    ok(&bench.start(true, &["review", "54"]));
    assert!(bench.posted("issue", "54").unwrap().contains("\nchange: 174\n"));
    assert!(read(&bench.at("start.prompt")).contains("server 174"));
}

#[test]
fn test_WB_BUILD_12_none_or_two_open_changes_are_refused_with_1_naming_what_was_found() {
    let bench = bench();
    let run = bench.start(true, &["review", "54"]);
    refused(&bench, &run, 1, &["no open change", "issue-54", "--change"]);
    bench.changes("issue-54", &["174", "175"]);
    let run = bench.start(true, &["review", "54"]);
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
    ok(&bench.start(true, &["review", "54", "--round", "5", "--change", "180"]));
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
    let run = bench.start(true, &["review", "54"]);
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
    let run = bench.start(true, &["review", "--change", "174"]);
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
    let run = bench.start(true, &["review", "--change", "174"]);
    refused(&bench, &run, 1, &["change_comment"]);
}

#[test]
fn test_WB_BUILD_18_the_round_of_a_change_with_no_issue_is_read_from_the_stamps_on_the_change() {
    let bench = bench();
    bench.comments("change", "174", &["[stamp] review round 1", "a comment"]);
    ok(&bench.start(true, &["review", "--change", "174"]));
    assert!(bench.posted("change", "174").unwrap().starts_with("[stamp] review round 2\n"));
    ok(&bench.start(true, &["fix", "--change", "174"]));
    assert!(bench.posted("change", "174").unwrap().starts_with("[stamp] fix round 1\n"));
    let worked = bench.prompt(&["review", "--change", "174"]);
    let given = bench.prompt(&["review", "--change", "174", "--round", "2"]);
    ok(&worked);
    assert_eq!(text(&worked.stdout), text(&given.stdout));
}

#[test]
fn test_WB_BUILD_18_a_fix_of_a_change_with_no_review_stamp_on_it_is_refused() {
    let bench = bench();
    let run = bench.start(true, &["fix", "--change", "174"]);
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
        let run = bench.start(true, &["build", &issue]);
        refused(&bench, &run, 1, &["#65"]);
    }
    // A path beside a classed one, or whose name only starts the same, is not classed.
    issue_file(&bench, "70", "[src/brakes.c, src/other/a.c, docs]");
    git(&bench.repo.path, &["branch", "issue-70"]);
    let run = bench.start(true, &["build", "70"]);
    assert_eq!(code(&run), Some(1), "docs is above docs/safety.md: {}", text(&run.stderr));
    issue_file(&bench, "71", "[src/brakes.c, src/other/a.c, docs/other.md]");
    git(&bench.repo.path, &["branch", "issue-71"]);
    ok(&bench.start(true, &["build", "71"]));
}

#[test]
fn test_WB_BUILD_19_with_no_classed_every_path_is_classed() {
    let bench = bench();
    bench.set("classed", "");
    issue_file(&bench, "54", "[README.md]");
    let run = bench.start(true, &["build", "54"]);
    refused(&bench, &run, 1, &["#65", "classed"]);
    // An issue's file that changes nothing touches nothing classed.
    issue_file(&bench, "55", "[]");
    git(&bench.repo.path, &["branch", "issue-55"]);
    ok(&bench.start(true, &["build", "55"]));
}

#[test]
fn test_WB_BUILD_19_a_classed_that_is_not_a_list_of_paths_is_refused_with_its_place() {
    let bench = bench();
    for line in ["classed: src", "classed: [/etc]", "classed: [a/../b]", "classed: ['']"] {
        bench.set("classed", line);
        let run = bench.start(true, &["build", "54"]);
        refused(&bench, &run, 1, &["classed"]);
    }
}

#[test]
fn test_WB_BUILD_20_with_no_issue_s_file_a_role_is_refused_unless_classed_is_empty() {
    let bench = bench();
    for line in ["classed: [src]", ""] {
        bench.set("classed", line);
        for words in [&["build", "54"][..], &["review", "--change", "174"]] {
            let run = bench.start(true, words);
            refused(&bench, &run, 1, &["#65", "classed: []"]);
        }
        // The breakdown changes only the issue's file, and is not refused on the class.
        ok(&bench.start(true, &["breakdown", "54"]));
        // `--prompt` is not refused on the class.
        ok(&bench.start(true, &["build", "54", "--prompt"]));
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
    ok(&start_with(&bench.repo.path.join("src/deep"), &bench.org, true, &["build", "54"]));
    assert!(bench.posted("issue", "54").unwrap().starts_with("[stamp] build round 2\n"));
    assert_eq!(read(&bench.at("comments.pwd")).trim(), bench.root());
}

#[test]
fn test_WB_BUILD_21_a_value_given_that_is_not_plain_is_used_wrongly_by_its_option() {
    let bench = bench();
    for issue in ["a b", "54;true", "$(true)"] {
        let run = bench.start(true, &["build", issue, "--branch", "issue-54", "--issue_address", "x"]);
        refused(&bench, &run, 2, &["--issue"]);
    }
    let run = bench.start(true, &["review", "--change", "1 2", "--change_address", "x"]);
    refused(&bench, &run, 2, &["--change"]);
}

#[test]
fn test_WB_BUILD_21_a_branch_worked_out_that_is_not_plain_is_refused_with_1_saying_to_give_change() {
    let bench = bench();
    git(&bench.repo.path, &["branch", "-D", "issue-54"]);
    git(&bench.repo.path, &["branch", "issue-54-a/b"]);
    let run = bench.start(true, &["review", "54"]);
    refused(&bench, &run, 1, &["issue-54-a/b", "--change"]);
}

// ---- WB-BUILD-22: a round given is a whole number from 1 ----

#[test]
fn test_WB_BUILD_22_a_round_that_is_not_a_whole_number_from_1_is_used_wrongly() {
    let bench = bench();
    for round in ["0", "-1", "x", "1.5", "01", " 1"] {
        let run = bench.start(true, &["build", "54", "--round", round]);
        refused(&bench, &run, 2, &["--round"]);
        assert!(text(&run.stderr).ends_with(USAGE), "{}: {}", round, text(&run.stderr));
    }
}

// ---- WB-BUILD-23: the pattern that posts must be there ----

#[test]
fn test_WB_BUILD_23_with_no_issue_comment_nothing_is_posted_or_started() {
    let bench = bench();
    bench.set("issue_comment", "");
    let run = bench.start(true, &["build", "54"]);
    refused(&bench, &run, 1, &["issue_comment"]);
}

// ---- WB-BUILD-24: a word too long to be handed to a program ----

#[test]
fn test_WB_BUILD_24_a_prompt_of_128_kib_or_more_is_refused_before_anything_is_posted() {
    let bench = bench();
    put(&bench.repo.path, ".watchbill/project/all.md", &format!("{}\n", "a".repeat(132 * 1024)));
    let run = bench.start(true, &["build", "54"]);
    refused(&bench, &run, 1, &["--prompt"]);
}

// ---- WB-BUILD-25: an interrupt does not end `wb` while the session runs ----

#[test]
fn test_WB_BUILD_25_an_interrupt_and_a_quit_sent_while_the_session_runs_do_not_end_wb() {
    let bench = bench();
    let l = bench.log.path.to_str().unwrap();
    script(&bench.at("start.sh"), &format!("echo start >> {l}/calls\nkill -INT $PPID\nkill -QUIT $PPID\nsleep 0.2\nexit 3\n"));
    let run = bench.start(true, &["build", "54"]);
    // Still here, with the session's status.
    assert_eq!(code(&run), Some(1), "{}", text(&run.stderr));
    assert!(bench.started());
}
