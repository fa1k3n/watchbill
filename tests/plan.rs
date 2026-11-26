//! `wb plan add` (#234, docs/revision.md point 15): a job is taken in. Its text is a blob in git under the hash of its
//! content, and its name is a ref, `refs/watchbill/<name>`, that points at a commit whose tree holds `job.yml` and the
//! text of each source. A plan may have several sources (#290, the owner's decision 1).
//!
//! The command is run in the test process, through `watchbill::plan`, with the organisation's file of the test. The
//! patterns of `watchbill.yml` are stand-ins that write what they were given in a folder of the test, and `origin` is an
//! address that only `ssh.sh` serves, which writes `network`: nothing reaches the network. `wb init` and `wb upgrade`
//! are run as the program.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const FETCH: &str = "+refs/watchbill/*:refs/watchbill/*";
const TEXT: &str = "The brakes hold the train on a slope of 4 %.\n";

struct Bench {
    repo: Folder,
    log: Folder,
    org: PathBuf,
    _org_folder: Folder,
}

/// What `wb plan` ended with and printed.
struct Ran {
    code: i32,
    out: String,
    err: String,
}

impl Bench {
    fn at(&self, name: &str) -> PathBuf {
        self.log.path.join(name)
    }

    /// What the stand-ins were run with, one line for each run, in order.
    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.at("calls")).unwrap_or_default().lines().map(str::to_string).collect()
    }

    /// `wb plan` with the words after `plan`, and `typed` on standard input.
    fn plan(&self, args: &[&str], typed: &str) -> Ran {
        let words: Vec<OsString> = args.iter().map(OsString::from).collect();
        let mut input = std::io::Cursor::new(typed.as_bytes().to_vec());
        let mut out = Vec::new();
        let mut err = Vec::new();
        let code = watchbill::plan(&words, &self.repo.path, &self.org, &mut input, &mut out, &mut err);
        Ran { code, out: text(&out), err: text(&err) }
    }

    /// Replaces a line of `watchbill.yml` that starts with `key:`, or takes it out when `line` is empty, and commits it.
    fn set(&self, key: &str, line: &str) {
        let yml = read(&config(&self.repo.path));
        let mut kept: Vec<String> = yml.lines().filter(|l| !l.starts_with(&format!("{}:", key))).map(str::to_string).collect();
        if !line.is_empty() {
            kept.push(line.to_string());
        }
        fs::write(config(&self.repo.path), format!("{}\n", kept.join("\n"))).unwrap();
        git(&self.repo.path, &["add", "--", ".watchbill"]);
        git(&self.repo.path, &["commit", "-q", "-m", "watchbill.yml"]);
    }

    /// What git prints, as text.
    fn git(&self, args: &[&str]) -> String {
        git_text(&self.repo.path, args)
    }

    /// Whether git ends with 0.
    fn git_ok(&self, args: &[&str]) -> bool {
        Command::new("git")
            .arg("-C")
            .arg(&self.repo.path)
            .args(args)
            .env_clear()
            .envs(env())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success()
    }

    /// The refs under `refs/watchbill/`, one to a line.
    fn refs(&self) -> String {
        self.git(&["for-each-ref", "--format=%(refname)", "refs/watchbill/"])
    }

    /// The hash git gives a text as a blob.
    fn hash_of(&self, text: &str) -> String {
        let mut child = Command::new("git")
            .arg("-C")
            .arg(&self.repo.path)
            .args(["hash-object", "--stdin"])
            .env_clear()
            .envs(env())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        use std::io::Write;
        child.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
        String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap().trim().to_string()
    }

    /// `job.yml` of the job a ref names.
    fn job_yml(&self, name: &str) -> String {
        self.git(&["cat-file", "blob", &format!("refs/watchbill/{}:job.yml", name)])
    }

    fn commit_of(&self, name: &str) -> String {
        self.git(&["rev-parse", &format!("refs/watchbill/{}", name)]).trim().to_string()
    }
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
    String::from_utf8_lossy(&run.stdout).to_string()
}

fn script(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\n{}", body)).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

const ORG: &str = "levels: [public, internal, confidential]
runners:
  stand-in:
    tools: this machine
    model: a stand-in
    agreement: none
    up_to: internal
    planner:
      up_to: internal
      model: claude-opus-5-5
      effort: high
    start: ['{log}/start.sh', '{model}', '{effort}', '{prompt}']
default: stand-in
";

/// A repository set up by `wb init --yes` and committed, at the level `internal`, with the tracker `github`, every
/// pattern a stand-in, and `issue_text` printing `The text of issue <n>.`; the organisation's file of `ORG` with `extra`.
fn bench_with(extra: &str) -> Bench {
    let repo = repository(Some("ssh://example.invalid/brakes/controller.git"));
    let log = folder_named("log");
    let l = log.path.to_str().unwrap().to_string();
    let at = |name: &str| format!("{}/{}", l, name);
    script(&log.path.join("call.sh"), &format!("echo \"$*\" >> {l}/calls\nexit 0\n"));
    script(&log.path.join("text.sh"), &format!("echo \"text $1\" >> {l}/calls\necho \"The text of issue $1.\"\nexit 0\n"));
    script(&log.path.join("start.sh"), &format!("echo start >> {l}/calls\nexit 0\n"));
    script(&log.path.join("ssh.sh"), &format!("echo ssh >> {l}/network\nexit 1\n"));
    git(&repo.path, &["config", "user.name", "Ada Lovelace"]);
    git(&repo.path, &["config", "user.email", "ada@example.org"]);
    git(&repo.path, &["config", "commit.gpgsign", "false"]);
    git(&repo.path, &["config", "core.sshCommand", &at("ssh.sh")]);
    let run = wb(&repo.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let yml: String = read(&config(&repo.path)).lines().filter(|line| !line.starts_with("tracker:")).map(|line| format!("{}\n", line)).collect();
    let mut yml = yml;
    for line in [
        format!("issue_comment: '{} post {{issue}}'", at("call.sh")),
        format!("issue_comments: '{} comments {{issue}}'", at("call.sh")),
        format!("change_comment: '{} post {{change}}'", at("call.sh")),
        format!("change_comments: '{} comments {{change}}'", at("call.sh")),
        format!("change_of_branch: '{} changes {{branch}}'", at("call.sh")),
        format!("issue_address: '{} address {{issue}}'", at("call.sh")),
        format!("issue_comment_bodies: '{} bodies {{issue}}'", at("call.sh")),
        format!("issue_text: '{} {{issue}}'", at("text.sh")),
        "tracker: github".to_string(),
        "confidentiality: internal".to_string(),
    ] {
        yml.push_str(&format!("{}\n", line));
    }
    fs::write(config(&repo.path), yml).unwrap();
    git(&repo.path, &["add", "--", ".watchbill"]);
    git(&repo.path, &["commit", "-q", "-m", "Watchbill"]);
    // Since #327 a commit of `wb plan add` is signed by a key that `.watchbill/signers` of the default branch holds
    // with `accept`: Ada's key of the test, and her line.
    signing(&repo.path);
    let org_folder = folder_named("org");
    let org = org_folder.path.join("organisation.yml");
    fs::write(&org, format!("{}{}", ORG.replace("{log}", &l), extra)).unwrap();
    Bench { repo, log, org, _org_folder: org_folder }
}

fn bench() -> Bench {
    bench_with("")
}

fn ok(ran: &Ran) {
    assert_eq!(ran.code, 0, "{}{}", ran.out, ran.err);
}

/// Refused with 1, as a limit, and nothing under `refs/watchbill/`.
fn refused(bench: &Bench, ran: &Ran) {
    assert_eq!(ran.code, 1, "{}{}", ran.out, ran.err);
    assert!(ran.err.contains("wb plan add: limit: "), "{}", ran.err);
    assert_eq!(bench.refs(), "", "a ref was written");
}

// ---- WB-JOB-1: the text from standard input is a blob under its hash ----

#[test]
fn test_WB_JOB_1_the_text_on_standard_input_is_stored_as_a_blob_under_its_hash() {
    let bench = bench();

    let ran = bench.plan(&["add", "97", "-F", "-"], TEXT);

    ok(&ran);
    let hash = bench.hash_of(TEXT);
    assert_eq!(bench.git(&["cat-file", "-t", &hash]).trim(), "blob");
    assert_eq!(bench.git(&["cat-file", "-p", &hash]), TEXT);
}

// ---- WB-JOB-2: the ref points at a commit whose tree holds job.yml and the text ----

#[test]
fn test_WB_JOB_2_the_ref_points_at_a_commit_whose_tree_holds_job_yml_and_the_text() {
    let bench = bench();

    ok(&bench.plan(&["add", "97", "-F", "-"], TEXT));

    let hash = bench.hash_of(TEXT);
    assert_eq!(bench.git(&["cat-file", "-t", "refs/watchbill/97"]).trim(), "commit");
    assert_eq!(bench.git(&["ls-tree", "-r", "--name-only", "refs/watchbill/97"]), format!("job.yml\nsources/{}\n", hash));
    assert_eq!(bench.git(&["rev-parse", &format!("refs/watchbill/97:sources/{}", hash)]).trim(), hash);
    // The first commit on the ref names none before it.
    assert_eq!(bench.git(&["rev-list", "--count", "refs/watchbill/97"]).trim(), "1");
}

#[test]
fn test_WB_JOB_2_a_name_with_a_slash_is_stored_under_its_ref() {
    let bench = bench();

    ok(&bench.plan(&["add", "github/54", "-F", "-"], TEXT));

    assert_eq!(bench.refs(), "refs/watchbill/github/54\n");
    assert!(bench.job_yml("github/54").contains("name: github/54\n"), "{}", bench.job_yml("github/54"));
}

// ---- WB-JOB-3: what job.yml holds ----

#[test]
fn test_WB_JOB_3_job_yml_holds_the_name_and_for_the_source_its_hash_address_level_person_and_time() {
    let bench = bench();

    ok(&bench.plan(&["add", "97", "-F", "-", "--source", "https://tracker.example.org/issues/97"], TEXT));

    let yml = bench.job_yml("97");
    let hash = bench.hash_of(TEXT);
    assert!(yml.contains("name: 97\n") || yml.contains("name: '97'\n"), "{}", yml);
    assert!(yml.contains(&format!("hash: {}\n", hash)) || yml.contains(&format!("hash: '{}'\n", hash)), "{}", yml);
    assert!(yml.contains("source: 'https://tracker.example.org/issues/97'\n"), "{}", yml);
    assert!(yml.contains("level: internal\n"), "{}", yml);
    assert!(yml.contains("by: 'Ada Lovelace <ada@example.org>'\n"), "{}", yml);
    let time = yml.lines().find_map(|line| line.trim().strip_prefix("time: ")).unwrap_or_else(|| panic!("no time: {}", yml));
    let time = time.trim_matches('\'');
    assert_eq!(time.len(), 20, "{}", time);
    assert!(time.starts_with("20") && time.ends_with('Z') && &time[10..11] == "T", "{}", time);
}

#[test]
fn test_WB_JOB_3_with_no_source_given_the_source_is_none() {
    let bench = bench();

    ok(&bench.plan(&["add", "97", "-m", "Brakes."], ""));

    assert!(bench.job_yml("97").contains("source: none\n"), "{}", bench.job_yml("97"));
}

// ---- WB-JOB-4: -F <file> and -m give the hash standard input gives ----

#[test]
fn test_WB_JOB_4_a_file_and_a_message_give_the_same_hash_as_standard_input() {
    let bench = bench();
    let file = bench.log.path.join("job.txt");
    fs::write(&file, TEXT).unwrap();

    ok(&bench.plan(&["add", "by-pipe", "-F", "-"], TEXT));
    ok(&bench.plan(&["add", "by-file", "-F", file.to_str().unwrap()], ""));
    ok(&bench.plan(&["add", "by-message", "-m", TEXT.trim_end()], ""));

    let hash = bench.hash_of(TEXT);
    for name in ["by-pipe", "by-file", "by-message"] {
        assert!(bench.job_yml(name).contains(&hash), "{}: {}", name, bench.job_yml(name));
    }
}

#[test]
fn test_WB_JOB_4_a_file_is_read_from_the_folder_wb_runs_in() {
    let bench = bench();
    fs::write(bench.repo.path.join("job.txt"), TEXT).unwrap();

    ok(&bench.plan(&["add", "97", "--file=job.txt"], ""));

    assert_eq!(bench.git(&["cat-file", "-p", &bench.hash_of(TEXT)]), TEXT);
}

/// `wb` run as a child, as a person runs it, under a cap of 1 GiB of memory and for at most `seconds`, with `input` as
/// its standard input: a `wb` that hangs or reads without end fails the test and does not take the machine.
fn wb_child(folder: &Path, args: &[&str], input: Stdio, seconds: u64) -> std::process::Output {
    let child = Command::new("prlimit")
        .arg("--as=1073741824")
        .arg("--")
        .arg(env!("CARGO_BIN_EXE_wb"))
        .args(args)
        .current_dir(folder)
        .env_clear()
        .envs(env())
        .stdin(input)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("wb could not be started");
    let id = child.id();
    let (send, receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = send.send(child.wait_with_output());
    });
    match receive.recv_timeout(std::time::Duration::from_secs(seconds)) {
        Ok(done) => done.expect("wb could not be waited for"),
        Err(_) => {
            // Only the child this test started is ended.
            let _ = Command::new("kill").args(["-9", &id.to_string()]).status();
            panic!("wb {:?} did not end within {} seconds", args, seconds);
        }
    }
}

#[test]
fn test_WB_JOB_4_a_fifo_a_device_or_a_folder_as_the_file_is_refused_with_1_and_not_waited_on() {
    let bench = bench();
    let fifo = bench.log.path.join("job.fifo");
    assert!(Command::new("mkfifo").arg(&fifo).status().unwrap().success());
    let folder = bench.log.path.join("a-folder");
    fs::create_dir(&folder).unwrap();

    for path in [fifo.to_str().unwrap(), "/dev/zero", "/dev/null", folder.to_str().unwrap()] {
        let run = wb_child(&bench.repo.path, &["plan", "add", "97", "-F", path], Stdio::null(), 20);

        assert_eq!(code(&run), Some(1), "{}: {}", path, text(&run.stderr));
        let err = text(&run.stderr);
        assert!(err.contains("wb plan add: limit: ") && err.contains("not a plain file") && err.contains("-F -"), "{}: {}", path, err);
    }
    assert_eq!(bench.refs(), "");
}

#[test]
fn test_WB_JOB_4_a_text_that_is_empty_not_utf_8_or_over_1_mib_is_refused_with_1() {
    let bench = bench();
    let at = |name: &str| bench.log.path.join(name).to_str().unwrap().to_string();
    fs::write(at("big.txt"), vec![b'a'; 1024 * 1024 + 1]).unwrap();
    fs::write(at("blank.txt"), " \n\t\n\n").unwrap();
    fs::write(at("bad.txt"), b"The brakes \xff hold.\n").unwrap();
    let (big, blank, bad) = (at("big.txt"), at("blank.txt"), at("bad.txt"));
    let cases: Vec<(Vec<&str>, Stdio, &str)> = vec![
        (vec!["-F", &big], Stdio::null(), "over 1 MiB"),
        (vec!["-F", "-"], Stdio::from(fs::File::open("/dev/zero").unwrap()), "over 1 MiB"),
        (vec!["-F", &blank], Stdio::null(), "empty"),
        (vec!["-F", "-"], Stdio::null(), "empty"),
        (vec!["-m", "  ", "-m", "\n"], Stdio::null(), "empty"),
        (vec!["-F", &bad], Stdio::null(), "not UTF-8"),
    ];

    for (words, input, said) in cases {
        let mut args = vec!["plan", "add", "97"];
        args.extend(&words);
        let run = wb_child(&bench.repo.path, &args, input, 20);

        assert_eq!(code(&run), Some(1), "{:?}: {}", words, text(&run.stderr));
        let err = text(&run.stderr);
        assert!(err.contains("wb plan add: limit: ") && err.contains(said), "{:?}: {}", words, err);
    }
    assert_eq!(bench.refs(), "");
}

// ---- WB-JOB-5: exactly one way of giving the text ----

#[test]
fn test_WB_JOB_5_none_or_two_of_the_ways_of_giving_the_text_end_with_2_and_write_nothing() {
    let bench = bench();
    let file = bench.log.path.join("job.txt");
    fs::write(&file, TEXT).unwrap();
    let file = file.to_str().unwrap();
    let cases: [&[&str]; 6] = [
        &["add", "97"],
        &["add", "97", "-F", "-", "-m", "Brakes."],
        &["add", "97", "-F", file, "--from", "github", "54"],
        &["add", "-m", "Brakes.", "--from", "github", "54"],
        &["add", "97", "-F", file, "-F", "-"],
        &["add", "97", "--from", "github", "54", "--from", "github", "55"],
    ];

    for args in cases {
        let ran = bench.plan(args, TEXT);
        assert_eq!(ran.code, 2, "{:?}: {}{}", args, ran.out, ran.err);
        assert!(ran.err.contains("-F") && ran.err.contains("-m") && ran.err.contains("--from"), "{:?}: {}", args, ran.err);
    }
    assert_eq!(bench.refs(), "");
    assert_eq!(bench.calls(), Vec::<String>::new());
}

// ---- WB-JOB-6: no name, the first 12 hex digits of the hash ----

#[test]
fn test_WB_JOB_6_with_no_name_the_ref_is_named_by_the_first_12_hex_digits_of_the_hash() {
    let bench = bench();

    ok(&bench.plan(&["add", "-F", "-"], TEXT));

    let hash = bench.hash_of(TEXT);
    assert_eq!(bench.refs(), format!("refs/watchbill/{}\n", &hash[..12]));
}

// ---- WB-JOB-7: the same text under the same name again ----

#[test]
fn test_WB_JOB_7_the_same_text_under_the_same_name_again_writes_no_commit_and_ends_with_0() {
    let bench = bench();
    ok(&bench.plan(&["add", "97", "-F", "-"], TEXT));
    let first = bench.commit_of("97");

    let ran = bench.plan(&["add", "97", "-F", "-"], TEXT);

    ok(&ran);
    assert_eq!(bench.commit_of("97"), first);
    assert!(ran.out.contains("Nothing written"), "{}", ran.out);
}

// ---- WB-JOB-8 and 9: other text under a name taken adds a source ----

#[test]
fn test_WB_JOB_8_other_text_under_a_name_taken_adds_a_source_as_a_new_commit_after_the_one_before() {
    let bench = bench();
    let other = "The brakes are tested at 80 km/h.\n";
    ok(&bench.plan(&["add", "97", "-F", "-"], TEXT));
    let first = bench.commit_of("97");

    ok(&bench.plan(&["add", "97", "-F", "-"], other));

    let second = bench.commit_of("97");
    assert_ne!(second, first);
    assert_eq!(bench.git(&["rev-parse", &format!("{}^", second)]).trim(), first);
    let (one, two) = (bench.hash_of(TEXT), bench.hash_of(other));
    let mut sources = [format!("sources/{}", one), format!("sources/{}", two)];
    sources.sort();
    assert_eq!(bench.git(&["ls-tree", "-r", "--name-only", "refs/watchbill/97"]), format!("job.yml\n{}\n{}\n", sources[0], sources[1]));
    let yml = bench.job_yml("97");
    assert!(yml.contains(&one) && yml.contains(&two), "{}", yml);
    assert!(yml.find(&one).unwrap() < yml.find(&two).unwrap(), "the sources are not in the order they were added: {}", yml);
}

#[test]
fn test_WB_JOB_9_other_text_under_a_name_taken_says_the_sources_there_and_the_one_added() {
    let bench = bench();
    let other = "The brakes are tested at 80 km/h.\n";
    ok(&bench.plan(&["add", "97", "-F", "-"], TEXT));

    let ran = bench.plan(&["add", "97", "-F", "-"], other);

    ok(&ran);
    assert!(ran.out.contains(&bench.hash_of(TEXT)), "{}", ran.out);
    assert!(ran.out.contains(&bench.hash_of(other)), "{}", ran.out);
}

// ---- WB-JOB-10: a name git refuses ----

#[test]
fn test_WB_JOB_10_a_name_git_refuses_ends_with_1_names_the_rule_and_writes_nothing() {
    let bench = bench();

    for name in ["github:54", "a..b", "two words", "x.lock", "a/", "a//b", "a~1", "a^", "a?", "a*", "a[1]", "a\\b", ".hidden"] {
        let ran = bench.plan(&["add", name, "-F", "-"], TEXT);
        refused(&bench, &ran);
        assert!(ran.err.contains("check-ref-format"), "{}: {}", name, ran.err);
    }
    assert!(!bench.git_ok(&["cat-file", "-e", &bench.hash_of(TEXT)]), "the blob was written");
}

#[test]
fn test_WB_JOB_10_a_name_that_meets_a_name_taken_as_a_folder_is_refused() {
    let bench = bench();
    ok(&bench.plan(&["add", "github", "-m", "One."], ""));

    let ran = bench.plan(&["add", "github/54", "-m", "Two."], "");

    assert_eq!(ran.code, 1, "{}{}", ran.out, ran.err);
    assert!(ran.err.contains("limit: "), "{}", ran.err);
    assert_eq!(bench.refs(), "refs/watchbill/github\n");
}

// ---- WB-JOB-11: the level ----

#[test]
fn test_WB_JOB_11_a_level_above_the_repositorys_ends_with_1_and_writes_nothing() {
    let bench = bench();

    let ran = bench.plan(&["add", "97", "-F", "-", "--level", "confidential"], TEXT);

    refused(&bench, &ran);
    assert!(ran.err.contains("confidential") && ran.err.contains("internal"), "{}", ran.err);
    assert!(!bench.git_ok(&["cat-file", "-e", &bench.hash_of(TEXT)]), "the blob was written");
}

#[test]
fn test_WB_JOB_11_a_level_not_on_the_organisations_list_ends_with_1() {
    let bench = bench();

    let ran = bench.plan(&["add", "97", "-F", "-", "--level", "secret"], TEXT);

    refused(&bench, &ran);
    assert!(ran.err.contains("secret"), "{}", ran.err);
}

#[test]
fn test_WB_JOB_11_a_level_at_or_below_the_repositorys_is_written() {
    let bench = bench();

    ok(&bench.plan(&["add", "97", "-F", "-", "--level", "public"], TEXT));

    assert!(bench.job_yml("97").contains("level: public\n"), "{}", bench.job_yml("97"));
}

#[test]
fn test_WB_JOB_11_with_no_organisations_file_it_ends_with_1_and_names_the_file() {
    let bench = bench();
    fs::remove_file(&bench.org).unwrap();

    let ran = bench.plan(&["add", "97", "-F", "-"], TEXT);

    refused(&bench, &ran);
    assert!(ran.err.contains(bench.org.to_str().unwrap()), "{}", ran.err);
}

// ---- WB-JOB-12 and 13: only the hash ----

#[test]
fn test_WB_JOB_12_hash_only_without_the_organisations_leave_ends_with_1_and_writes_nothing() {
    for extra in ["", "jobs:\n  hash_only: refused\n"] {
        let bench = bench_with(extra);

        let ran = bench.plan(&["add", "97", "-F", "-", "--hash-only"], TEXT);

        refused(&bench, &ran);
        assert!(ran.err.contains("hash_only"), "{}", ran.err);
        assert!(!bench.git_ok(&["cat-file", "-e", &bench.hash_of(TEXT)]), "the blob was written");
    }
}

#[test]
fn test_WB_JOB_13_hash_only_with_the_leave_writes_no_blob_and_job_yml_holds_the_hash_and_the_source() {
    let bench = bench_with("jobs:\n  hash_only: allowed\n");

    ok(&bench.plan(&["add", "97", "-F", "-", "--hash-only", "--source", "https://tracker.example.org/issues/97"], TEXT));

    let hash = bench.hash_of(TEXT);
    assert!(!bench.git_ok(&["cat-file", "-e", &hash]), "the blob of the text was written");
    assert_eq!(bench.git(&["ls-tree", "-r", "--name-only", "refs/watchbill/97"]), "job.yml\n");
    let yml = bench.job_yml("97");
    assert!(yml.contains(&hash), "{}", yml);
    assert!(yml.contains("source: 'https://tracker.example.org/issues/97'\n"), "{}", yml);
    assert!(yml.contains("text: hash only\n") || yml.contains("text: 'hash only'\n"), "{}", yml);
}

// ---- WB-JOB-14: it says what it will write first ----

#[test]
fn test_WB_JOB_14_it_says_the_blob_the_job_yml_and_the_ref_before_it_writes_them() {
    let bench = bench();

    let ran = bench.plan(&["add", "97", "-F", "-"], TEXT);

    ok(&ran);
    let hash = bench.hash_of(TEXT);
    let said = ran.out.find(&hash).unwrap_or_else(|| panic!("{}", ran.out));
    let yml = ran.out.find("job.yml:").unwrap_or_else(|| panic!("{}", ran.out));
    let wrote = ran.out.find("Wrote refs/watchbill/97").unwrap_or_else(|| panic!("{}", ran.out));
    assert!(said < wrote && yml < wrote, "{}", ran.out);
    assert!(ran.out.contains(&bench.commit_of("97")), "{}", ran.out);
}

#[test]
fn test_WB_JOB_14_a_write_that_fails_was_said_first_and_ends_with_1() {
    let bench = bench();
    // A lock of the ref that git keeps: `update-ref` cannot take it.
    let refs = bench.repo.path.join(".git/refs/watchbill");
    fs::create_dir_all(&refs).unwrap();
    fs::write(refs.join("97.lock"), "").unwrap();

    let ran = bench.plan(&["add", "97", "-F", "-"], TEXT);

    assert_eq!(ran.code, 1, "{}{}", ran.out, ran.err);
    assert!(ran.err.contains("limit: "), "{}", ran.err);
    assert!(ran.out.contains("refs/watchbill/97"), "{}", ran.out);
    assert_eq!(bench.refs(), "");
}

// ---- WB-JOB-15 and 16: no network, no prompt, no session ----

#[test]
fn test_WB_JOB_15_it_asks_nothing_of_the_network_and_runs_no_pattern() {
    let bench = bench();

    ok(&bench.plan(&["add", "97", "-F", "-"], TEXT));
    ok(&bench.plan(&["add", "97", "-m", "More."], ""));

    assert!(!bench.at("network").exists(), "git asked origin");
    assert_eq!(bench.calls(), Vec::<String>::new());
}

#[test]
fn test_WB_JOB_16_it_only_queues_the_job_no_prompt_is_run_and_no_session_started() {
    let bench = bench();

    let ran = bench.plan(&["add", "97", "-F", "-"], TEXT);

    ok(&ran);
    assert!(!bench.calls().iter().any(|call| call == "start"), "{:?}", bench.calls());
    // The prompt of no role is printed.
    assert!(!ran.out.contains("{issue}") && !ran.out.contains("# The planner"), "{}", ran.out);
    // Nothing of the working tree changed: no issue's file, no view.
    assert_eq!(bench.git(&["status", "--porcelain"]), "");
}

// ---- WB-JOB-17 to 19: the fetch line ----

fn fetch_lines(folder: &Path) -> Vec<String> {
    git_text(folder, &["config", "--get-all", "remote.origin.fetch"]).lines().map(str::to_string).collect()
}

#[test]
fn test_WB_JOB_17_init_adds_the_fetch_line_of_the_jobs_to_origin_and_says_so_first() {
    let repo = repository(Some("https://github.com/acme/brake-controller.git"));

    let run = wb(&repo.path, &["init", "--yes"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(fetch_lines(&repo.path).contains(&FETCH.to_string()), "{:?}", fetch_lines(&repo.path));
    let out = text(&run.stdout);
    let said = out.find(FETCH).unwrap_or_else(|| panic!("{}", out));
    assert!(said < out.find("Wrote ").unwrap(), "{}", out);
}

#[test]
fn test_WB_JOB_17_init_with_no_origin_writes_no_fetch_line() {
    let repo = repository(None);

    let run = wb(&repo.path, &["init", "--yes"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(git_text(&repo.path, &["config", "--get-regexp", "^remote\\."]), "");
}

/// A repository with `origin`, set up by `wb init --yes` and committed, with a git identity.
fn set_up() -> Folder {
    let folder = repository(Some("https://github.com/acme/brake-controller.git"));
    git(&folder.path, &["config", "user.name", "Ada Lovelace"]);
    git(&folder.path, &["config", "user.email", "ada@example.org"]);
    git(&folder.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    git(&folder.path, &["add", "--all", "--", ".watchbill"]);
    git(&folder.path, &["commit", "-q", "-m", "Watchbill"]);
    folder
}

#[test]
fn test_WB_JOB_18_upgrade_adds_the_fetch_line_where_it_is_missing_and_says_so_first() {
    let folder = set_up();
    git(&folder.path, &["config", "--unset-all", "remote.origin.fetch", "watchbill"]);
    assert!(!fetch_lines(&folder.path).contains(&FETCH.to_string()));

    let dry = wb(&folder.path, &["upgrade", "--dry-run"], "");
    assert_eq!(code(&dry), Some(0), "{}{}", text(&dry.stdout), text(&dry.stderr));
    assert!(text(&dry.stdout).contains(FETCH), "{}", text(&dry.stdout));
    assert!(!fetch_lines(&folder.path).contains(&FETCH.to_string()), "--dry-run wrote the line");

    let run = wb(&folder.path, &["upgrade"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.find(FETCH).unwrap_or_else(|| panic!("{}", out)) < out.find("Written").unwrap_or_else(|| panic!("{}", out)), "{}", out);
    assert_eq!(fetch_lines(&folder.path).iter().filter(|line| *line == FETCH).count(), 1);
    // The line of the branches is kept.
    assert!(fetch_lines(&folder.path).contains(&"+refs/heads/*:refs/remotes/origin/*".to_string()), "{:?}", fetch_lines(&folder.path));
}

#[test]
fn test_WB_JOB_19_upgrade_leaves_the_fetch_line_where_it_is_there() {
    let folder = set_up();

    let run = wb(&folder.path, &["upgrade"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    assert!(text(&run.stdout).contains("Nothing to do"), "{}", text(&run.stdout));
    assert_eq!(fetch_lines(&folder.path).iter().filter(|line| *line == FETCH).count(), 1);
}

// ---- WB-JOB-20: wb help plan ----

#[test]
fn test_WB_JOB_20_help_plan_says_what_add_takes_how_it_names_a_job_its_options_and_its_statuses() {
    let folder = folder();

    let run = wb(&folder.path, &["help", "plan"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let help = text(&run.stdout);
    for word in ["wb plan add", "-F <file>", "-m <text>", "--from <adapter> <number>", "--source", "--level", "--hash-only", "refs/watchbill/", "12 hex digits", "Ends with 0", "1 when", "2 when"] {
        assert!(help.contains(word), "{}: {}", word, help);
    }
}

// ---- WB-JOB-45: wb help plan says each subcommand ----

#[test]
fn test_WB_JOB_45_help_plan_says_each_subcommand_and_its_statuses() {
    let folder = folder();

    let run = wb(&folder.path, &["help", "plan"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    // The words are read across the line breaks of the usage.
    let help = text(&run.stdout).split_whitespace().collect::<Vec<_>>().join(" ");
    for word in [
        "wb plan list",
        "wb plan show <name>",
        "wb plan edit <name>",
        "wb plan accept <name> [--no-commit | -n]",
        "wb plan remove <name>",
        "GIT_EDITOR",
        "planned again after the acceptance",
        "a source added after the acceptance",
        "removed",
    ] {
        assert!(help.contains(word), "{}: {}", word, help);
    }
}

// ---- WB-JOB-21: -m is repeated as git repeats it ----

#[test]
fn test_WB_JOB_21_messages_are_joined_as_git_joins_them_each_a_paragraph() {
    let bench = bench();

    ok(&bench.plan(&["add", "97", "-m", "The brakes hold.", "-m", "They are tested."], ""));

    let joined = "The brakes hold.\n\nThey are tested.\n";
    assert_eq!(bench.git(&["cat-file", "-p", &bench.hash_of(joined)]), joined);
    assert!(bench.job_yml("97").contains(&bench.hash_of(joined)), "{}", bench.job_yml("97"));
}

#[test]
fn test_WB_JOB_21_messages_are_cleaned_up_as_git_commit_cleans_them() {
    let bench = bench();
    let messages = ["  The brakes hold.  \t", "", "\n\nThey are tested.\n\n\n\nAt 80 km/h.   \n\n"];
    // What `git commit -m` builds of them: each on a line of its own after a line break, then cleaned up by
    // `git stripspace`, its clean-up of whitespace.
    let mut built = String::new();
    for message in messages {
        if !built.is_empty() {
            built.push('\n');
        }
        built.push_str(message);
        if !built.ends_with('\n') {
            built.push('\n');
        }
    }
    let mut child = Command::new("git").arg("stripspace").env_clear().envs(env()).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().unwrap();
    use std::io::Write;
    child.stdin.take().unwrap().write_all(built.as_bytes()).unwrap();
    let expected = String::from_utf8(child.wait_with_output().unwrap().stdout).unwrap();
    assert_eq!(expected, "  The brakes hold.\n\nThey are tested.\n\nAt 80 km/h.\n");
    let mut args = vec!["add", "97"];
    for message in &messages {
        args.push("-m");
        args.push(message);
    }

    ok(&bench.plan(&args, ""));

    assert_eq!(bench.git(&["cat-file", "-p", &bench.hash_of(&expected)]), expected);
}

// ---- WB-JOB-22: one text under two names is one job ----

#[test]
fn test_WB_JOB_22_the_same_text_under_a_second_name_is_one_job_with_two_names() {
    let bench = bench();
    ok(&bench.plan(&["add", "github/54", "-F", "-"], TEXT));

    let ran = bench.plan(&["add", "gitlab/54", "-F", "-"], TEXT);

    ok(&ran);
    assert_eq!(bench.commit_of("gitlab/54"), bench.commit_of("github/54"));
    assert!(ran.out.contains("github/54"), "{}", ran.out);
    // The ref that is there is moved, and the new one is made: each is said as what it is, first and when written.
    let commit = bench.commit_of("github/54");
    assert!(ran.out.contains("  the ref refs/watchbill/github/54 moved to it from that commit\n"), "{}", ran.out);
    assert!(ran.out.contains("  the ref refs/watchbill/gitlab/54 made at it\n"), "{}", ran.out);
    assert!(ran.out.contains(&format!("Wrote refs/watchbill/github/54: moved to commit {}.\n", commit)), "{}", ran.out);
    assert!(ran.out.contains(&format!("Wrote refs/watchbill/gitlab/54: made at commit {}.\n", commit)), "{}", ran.out);
    assert!(!ran.out.contains("gitlab/54 moved"), "{}", ran.out);
    // job.yml keeps the first name and lists both.
    let yml = bench.job_yml("gitlab/54");
    assert!(yml.contains("name: github/54\nnames:\n  - github/54\n  - gitlab/54\n"), "{}", yml);
    assert_eq!(yml.matches("hash: ").count(), 1, "{}", yml);
}

#[test]
fn test_WB_JOB_22_a_source_added_under_the_second_name_moves_every_name_of_the_job() {
    let bench = bench();
    let other = "The brakes are tested at 80 km/h.\n";
    ok(&bench.plan(&["add", "github/54", "-F", "-"], TEXT));
    ok(&bench.plan(&["add", "gitlab/54", "-F", "-"], TEXT));
    let before = bench.commit_of("github/54");

    ok(&bench.plan(&["add", "gitlab/54", "-F", "-"], other));

    let after = bench.commit_of("github/54");
    assert_ne!(after, before);
    assert_eq!(bench.commit_of("gitlab/54"), after);
    assert_eq!(bench.git(&["rev-parse", &format!("{}^", after)]).trim(), before);
    for name in ["github/54", "gitlab/54"] {
        let yml = bench.job_yml(name);
        assert!(yml.contains(&bench.hash_of(TEXT)) && yml.contains(&bench.hash_of(other)), "{}: {}", name, yml);
        assert!(yml.contains("name: github/54\n"), "{}: {}", name, yml);
    }
}

#[test]
fn test_WB_JOB_22_a_ref_under_refs_watchbill_that_is_not_a_job_is_passed_over_and_said_once() {
    let bench = bench();
    let head = bench.git(&["rev-parse", "HEAD"]).trim().to_string();
    git(&bench.repo.path, &["update-ref", "refs/watchbill/not-a-job", &head]);

    let ran = bench.plan(&["add", "97", "-F", "-"], TEXT);

    ok(&ran);
    assert_eq!(ran.err.matches("refs/watchbill/not-a-job").count(), 1, "{}", ran.err);
    assert_eq!(bench.refs(), "refs/watchbill/97\nrefs/watchbill/not-a-job\n");
}

// ---- WB-JOB-23 and 24: --from ----

#[test]
fn test_WB_JOB_23_from_with_no_adapter_set_up_ends_with_2_and_says_to_pipe_the_text_in() {
    let bench = bench();

    let ran = bench.plan(&["add", "--from", "jira", "54"], "");

    assert_eq!(ran.code, 2, "{}{}", ran.out, ran.err);
    assert!(ran.err.contains("-F -"), "{}", ran.err);
    bench.set("issue_text", "");
    let ran = bench.plan(&["add", "--from", "github", "54"], "");
    assert_eq!(ran.code, 2, "{}{}", ran.out, ran.err);
    assert!(ran.err.contains("-F -"), "{}", ran.err);
    assert_eq!(bench.refs(), "");
    assert_eq!(bench.calls(), Vec::<String>::new());
}

#[test]
fn test_WB_JOB_24_from_takes_the_text_by_issue_text_and_names_the_job_adapter_slash_number() {
    let bench = bench();

    let ran = bench.plan(&["add", "--from", "github", "54"], "");

    ok(&ran);
    assert_eq!(bench.calls(), ["text 54"]);
    assert_eq!(bench.refs(), "refs/watchbill/github/54\n");
    let printed = "The text of issue 54.\n";
    assert_eq!(bench.git(&["cat-file", "-p", &bench.hash_of(printed)]), printed);
    let yml = bench.job_yml("github/54");
    assert!(yml.contains("text.sh 54"), "the source is not the command that read it: {}", yml);
}

#[test]
fn test_WB_JOB_24_from_with_a_number_that_is_not_one_or_with_a_name_ends_with_2() {
    let bench = bench();

    for args in [&["add", "--from", "github", "5x"][..], &["add", "--from", "github"][..], &["add", "54", "--from", "github", "54"][..]] {
        let ran = bench.plan(args, "");
        assert_eq!(ran.code, 2, "{:?}: {}{}", args, ran.out, ran.err);
    }
    assert_eq!(bench.calls(), Vec::<String>::new());
    assert_eq!(bench.refs(), "");
}

// ---- WB-JOB-25: the key jobs of the organisation's file ----

#[test]
fn test_WB_JOB_25_the_key_jobs_of_the_organisations_file_is_held_to_its_form() {
    for extra in ["jobs:\n  hash_only: sometimes\n", "jobs:\n  hash_only: true\n", "jobs:\n  other: allowed\n", "jobs: allowed\n"] {
        let bench = bench_with(extra);

        let ran = bench.plan(&["add", "97", "-F", "-"], TEXT);

        refused(&bench, &ran);
        assert!(ran.err.contains("jobs"), "{}: {}", extra, ran.err);
    }
    let path = folder_named("org");
    let file = path.path.join("organisation.yml");
    fs::write(&file, format!("{}jobs:\n  hash_only: allowed\n", ORG.replace("{log}", "/nowhere"))).unwrap();
    assert!(watchbill::confidentiality::read(&file).unwrap().hash_only);
    fs::write(&file, ORG.replace("{log}", "/nowhere")).unwrap();
    assert!(!watchbill::confidentiality::read(&file).unwrap().hash_only);
}

// ---- WB-JOB-26: wb plan with no subcommand ----

#[test]
fn test_WB_JOB_26_plan_with_no_subcommand_or_one_not_known_ends_with_2_naming_each() {
    let folder = folder();

    for args in [&["plan"][..], &["plan", "frob"][..], &["plan", "--from"][..]] {
        let run = wb(&folder.path, args, "");

        assert_eq!(code(&run), Some(2), "{:?}", args);
        assert_eq!(text(&run.stdout), "", "{:?}", args);
        assert!(text(&run.stderr).contains("add, list, show, edit, accept, remove"), "{:?}: {}", args, text(&run.stderr));
    }
}

#[test]
fn test_WB_JOB_26_there_is_no_command_add() {
    let folder = folder();

    let run = wb(&folder.path, &["add", "-m", "Brakes."], "");

    assert_eq!(code(&run), Some(2));
    assert!(text(&run.stderr).starts_with("wb: no command 'add'\n"), "{}", text(&run.stderr));
}

// ---- WB-VIEW-38 and 40 (#237): the fetch line of the notes that name the commits of a session ----

const NOTES_FETCH: &str = "+refs/notes/watchbill:refs/notes/watchbill";

#[test]
fn test_WB_VIEW_38_init_adds_the_fetch_line_of_the_notes_to_origin_and_says_so_first() {
    let repo = repository(Some("https://github.com/acme/brake-controller.git"));

    let run = wb(&repo.path, &["init", "--yes"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(fetch_lines(&repo.path).contains(&NOTES_FETCH.to_string()), "{:?}", fetch_lines(&repo.path));
    let out = text(&run.stdout);
    let said = out.find(NOTES_FETCH).unwrap_or_else(|| panic!("{}", out));
    assert!(said < out.find("Wrote ").unwrap(), "{}", out);
    // Both lines, each once, and the line of the branches is kept.
    assert_eq!(fetch_lines(&repo.path).iter().filter(|line| *line == NOTES_FETCH).count(), 1);
    assert!(fetch_lines(&repo.path).contains(&FETCH.to_string()));
    assert!(fetch_lines(&repo.path).contains(&"+refs/heads/*:refs/remotes/origin/*".to_string()), "{:?}", fetch_lines(&repo.path));
}

#[test]
fn test_WB_VIEW_38_upgrade_adds_the_fetch_line_of_the_notes_where_only_the_jobs_line_is_there_and_says_so_first() {
    let folder = set_up();
    git(&folder.path, &["config", "--unset-all", "remote.origin.fetch", "refs/notes"]);
    assert!(!fetch_lines(&folder.path).contains(&NOTES_FETCH.to_string()));
    assert!(fetch_lines(&folder.path).contains(&FETCH.to_string()));

    let dry = wb(&folder.path, &["upgrade", "--dry-run"], "");
    assert_eq!(code(&dry), Some(0), "{}{}", text(&dry.stdout), text(&dry.stderr));
    assert!(text(&dry.stdout).contains(NOTES_FETCH), "{}", text(&dry.stdout));
    // The line that is there is not said again.
    assert!(!text(&dry.stdout).contains(FETCH), "{}", text(&dry.stdout));
    assert!(!fetch_lines(&folder.path).contains(&NOTES_FETCH.to_string()), "--dry-run wrote the line");

    let run = wb(&folder.path, &["upgrade"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.find(NOTES_FETCH).unwrap_or_else(|| panic!("{}", out)) < out.find("Written").unwrap_or_else(|| panic!("{}", out)), "{}", out);
    assert_eq!(fetch_lines(&folder.path).iter().filter(|line| *line == NOTES_FETCH).count(), 1);
    assert_eq!(fetch_lines(&folder.path).iter().filter(|line| *line == FETCH).count(), 1);
}

#[test]
fn test_WB_VIEW_38_upgrade_adds_both_lines_where_neither_is_there() {
    let folder = set_up();
    git(&folder.path, &["config", "--unset-all", "remote.origin.fetch", "watchbill"]);

    let run = wb(&folder.path, &["upgrade"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains(FETCH) && out.contains(NOTES_FETCH), "{}", out);
    assert!(fetch_lines(&folder.path).contains(&FETCH.to_string()) && fetch_lines(&folder.path).contains(&NOTES_FETCH.to_string()), "{:?}", fetch_lines(&folder.path));
}

#[test]
fn test_WB_VIEW_40_upgrade_leaves_the_fetch_line_of_the_notes_where_it_is_there() {
    let folder = set_up();
    assert!(fetch_lines(&folder.path).contains(&NOTES_FETCH.to_string()), "{:?}", fetch_lines(&folder.path));

    let run = wb(&folder.path, &["upgrade"], "");

    assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
    assert!(text(&run.stdout).contains("Nothing to do"), "{}", text(&run.stdout));
    assert_eq!(fetch_lines(&folder.path).iter().filter(|line| *line == NOTES_FETCH).count(), 1);
}
