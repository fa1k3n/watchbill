//! A person's acceptance is a signed tag on one exact hash (#235; `docs/revision.md`, point 15), and `wb` checks every
//! signature it relies on against the keys the project keeps, `.watchbill/signers`, as the default branch has it.
//!
//! Run as a person runs `wb`, in a repository made for the test. Each test makes its own SSH keys with `ssh-keygen`, in
//! a home folder of its own: `HOME`, `XDG_CONFIG_HOME` and `GIT_CONFIG_GLOBAL` point there, and no ssh agent is handed
//! on, so no person's key, agent or git config is ever read or touched. `wb plan add`, which `wb` as a program runs only
//! with the organisation's file of `/etc`, runs in a child of this test binary with that same environment. The patterns of `watchbill.yml` are stand-ins
//! that read and write files of a folder `.tracker/`: never the network.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use watchbill::manifest::hash_bytes;

const PATTERNS: &str = "issue_comments: \"cat .tracker/{issue}.comments\"
issue_comment: \"tee -a .tracker/{issue}.bodies | head -n 1 >> .tracker/{issue}.comments\"
";

const YML: &str = ".watchbill/issues/54.yml";
const ACCEPTED: &str = ".watchbill/issues/54.accepted";
const RECORD: &str = ".watchbill/issues/54.finished";
const SIGNERS: &str = ".watchbill/signers";

/// The options `wb prompt` of a role needs beside the issue.
const REST: [&str; 10] =
    ["--branch", "issue-54-work", "--issue_address", "tracker 54", "--change", "34", "--round", "1", "--change_address", "server 34"];

const ORG: &str = "levels: [public, internal, confidential]
runners:
  stand-in:
    tools: this machine
    model: a stand-in
    agreement: none
    up_to: internal
default: stand-in
";

/// A repository of a test, with the home folder its git and `wb` run in, and the keys made there.
struct Bench {
    repo: Folder,
    home: Folder,
}

/// The environment of a test: that of `common::env`, with the home, git's config and the ssh agent of the test's own.
fn env_in(home: &Path) -> Vec<(String, String)> {
    let own = ["HOME", "XDG_CONFIG_HOME", "SSH_AUTH_SOCK", "SSH_AGENT_PID", "GIT_CONFIG_GLOBAL", "GIT_CONFIG_NOSYSTEM", "GNUPGHOME"];
    let mut vars: Vec<(String, String)> = env().into_iter().filter(|(name, _)| !own.contains(&name.as_str())).collect();
    let at = |below: &str| home.join(below).display().to_string();
    vars.push(("HOME".to_string(), home.display().to_string()));
    vars.push(("XDG_CONFIG_HOME".to_string(), at(".config")));
    vars.push(("GIT_CONFIG_GLOBAL".to_string(), at(".gitconfig")));
    vars.push(("GIT_CONFIG_NOSYSTEM".to_string(), "1".to_string()));
    vars.push(("GNUPGHOME".to_string(), at(".gnupg")));
    vars
}

impl Bench {
    fn run(&self, program: &str, args: &[&str], dir: &Path) -> Output {
        Command::new(program).args(args).current_dir(dir).env_clear().envs(env_in(&self.home.path)).stdin(Stdio::null()).output().expect("a program could not be started")
    }

    /// `wb` as the program, in the repository.
    fn wb(&self, args: &[&str]) -> Output {
        self.run(env!("CARGO_BIN_EXE_wb"), args, &self.repo.path)
    }

    /// git in the repository, which must end with 0; what it printed.
    fn git(&self, args: &[&str]) -> String {
        let run = self.run("git", args, &self.repo.path);
        assert!(run.status.success(), "git {:?}: {}", args, text(&run.stderr));
        text(&run.stdout)
    }

    /// git in the repository, as it ended.
    fn git_run(&self, args: &[&str]) -> Output {
        self.run("git", args, &self.repo.path)
    }

    /// The private key of `name`, made the first time it is asked for.
    fn key(&self, name: &str) -> PathBuf {
        let folder = self.home.path.join("keys");
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join(name);
        if !path.exists() {
            let run = self.run("ssh-keygen", &["-q", "-t", "ed25519", "-N", "", "-C", name, "-f", path.to_str().unwrap()], &self.home.path);
            assert!(run.status.success(), "ssh-keygen: {}", text(&run.stderr));
        }
        path
    }

    /// The public key of `name`, as `<type> <base64>`.
    fn public(&self, name: &str) -> String {
        let key = self.key(name);
        let line = read(&PathBuf::from(format!("{}.pub", key.display())));
        line.split_whitespace().take(2).collect::<Vec<_>>().join(" ")
    }

    /// A line of `.watchbill/signers`: the identity, the roles and the key of `name`.
    fn line(&self, identity: &str, roles: &str, name: &str) -> String {
        format!("{} roles=\"{}\" {}\n", identity, roles, self.public(name))
    }

    /// Signs with the key of `name` from now on, as git's config of the repository.
    fn sign_with(&self, name: &str) {
        let key = self.key(name);
        self.git(&["config", "gpg.format", "ssh"]);
        self.git(&["config", "user.signingkey", key.to_str().unwrap()]);
    }

    fn put(&self, path: &str, body: &str) {
        put(&self.repo.path, path, body);
    }

    fn at(&self, path: &str) -> PathBuf {
        self.repo.path.join(path)
    }

    fn head(&self) -> String {
        self.git(&["rev-parse", "HEAD"]).trim().to_string()
    }

    /// The refs under `prefix`, one to a line.
    fn refs(&self, prefix: &str) -> Vec<String> {
        self.git(&["for-each-ref", "--format=%(refname)", prefix]).lines().map(str::to_string).collect()
    }

    fn comments(&self, lines: &[&str]) {
        let all: String = lines.iter().map(|line| format!("{}\n", line)).collect();
        self.put(".tracker/54.comments", &all);
        // Since #236 a stamp is a commit on the job's ref, signed by the starter, and `wb` reads no `[stamp]` comment:
        // each stamp goes after what the ref holds, signed with the key git signs with.
        for line in lines.iter().filter(|line| line.starts_with("[stamp] ")) {
            let parent = self.git_run(&["rev-parse", "--verify", "-q", "refs/watchbill/54"]);
            let parent = text(&parent.stdout).trim().to_string();
            let tree = self.git_input(&["mktree"], b"").trim().to_string();
            // Its job and its base, as `wb` writes them (#324): no plan here, and the commit `HEAD` is at.
            let base = format!("base: {}", self.head());
            let mut args = vec!["commit-tree", "-S", "-m", line, "-m", "issue: 54\njob: none", "-m", &base, tree.as_str()];
            if !parent.is_empty() {
                args.push("-p");
                args.push(&parent);
            }
            let commit = self.git(&args).trim().to_string();
            self.git(&["update-ref", "refs/watchbill/54", &commit]);
        }
    }

    /// Commits `.watchbill/signers` on `main`, with the lines given, and comes back to the branch it was on.
    fn signers_on_main(&self, lines: &str) {
        let back = self.git(&["symbolic-ref", "--short", "HEAD"]).trim().to_string();
        if back != "main" {
            self.git(&["checkout", "-q", "main"]);
        }
        self.put(SIGNERS, lines);
        self.git(&["add", "--", SIGNERS]);
        self.git(&["commit", "-q", "-m", "The project's keys"]);
        if back != "main" {
            self.git(&["checkout", "-q", &back]);
        }
    }

    /// The branch of job 54, from `main`, with its issue's file committed. The checkout stays there.
    fn job(&self) {
        self.git(&["checkout", "-q", "-b", "issue-54-work", "main"]);
        self.put(YML, &BARE_ISSUE.replace("{issue}", "54"));
        self.git(&["add", "--", YML]);
        self.git(&["commit", "-q", "-m", "The issue's file (54)"]);
    }

    /// `wb plan accept 54`, which must end with 0.
    fn accept(&self) -> Output {
        let run = self.wb(&["plan", "accept", "54"]);
        assert_eq!(code(&run), Some(0), "{}{}", text(&run.stdout), text(&run.stderr));
        run
    }

    /// The tag of the acceptance of 54 as `.accepted` is now.
    fn tag(&self) -> String {
        format!("refs/tags/watchbill/accept/54/{}", &hash_bytes(&fs::read(self.at(ACCEPTED)).unwrap())[..12])
    }

    /// The message of a tag, without its signature.
    fn message(&self, tag: &str) -> String {
        let raw = self.git(&["cat-file", "tag", tag]);
        let body = raw.split_once("\n\n").unwrap().1;
        body.split("-----BEGIN SSH SIGNATURE-----").next().unwrap().to_string()
    }

    /// The tag `tag` made again, with the same message and target, by `args` of `git tag` (unsigned, or another key).
    fn tag_again(&self, tag: &str, args: &[&str]) {
        let message = self.message(tag);
        let target = self.git(&["rev-parse", &format!("{}^{{}}", tag)]).trim().to_string();
        self.tag_as(tag, args, &message, &target);
    }

    /// The tag `tag` made again by `args` of `git tag`, with `message`, on `target`.
    fn tag_as(&self, tag: &str, args: &[&str], message: &str, target: &str) {
        let file = self.home.path.join("message");
        fs::write(&file, &message).unwrap();
        let short = tag.strip_prefix("refs/tags/").unwrap();
        let mut all = vec!["tag", "-f"];
        all.extend(args);
        all.extend(["--cleanup=verbatim", "-F", file.to_str().unwrap(), short, target]);
        self.git(&all);
    }

    /// git in the repository with `input` on standard input, which must end with 0; what it printed.
    fn git_input(&self, args: &[&str], input: &[u8]) -> String {
        let mut child = Command::new("git")
            .args(args)
            .current_dir(&self.repo.path)
            .env_clear()
            .envs(env_in(&self.home.path))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        let run = child.wait_with_output().unwrap();
        assert!(run.status.success(), "git {:?}: {}", args, text(&run.stderr));
        text(&run.stdout)
    }

    /// The tag object `tag`, as git prints it.
    fn raw(&self, tag: &str) -> String {
        self.git(&["cat-file", "tag", tag])
    }

    /// The tag object `tag` up to its SSH signature: what git signed.
    fn payload(&self, tag: &str) -> String {
        let raw = self.raw(tag);
        raw[..raw.find("-----BEGIN SSH SIGNATURE-----").unwrap()].to_string()
    }

    /// The ref `tag` moved to a tag object made of `raw` by `git mktag`.
    fn put_tag(&self, tag: &str, raw: &str) {
        let object = self.git_input(&["mktag"], raw.as_bytes()).trim().to_string();
        self.git(&["update-ref", tag, &object]);
    }

    fn status(&self) -> String {
        let run = self.wb(&["status", "54"]);
        text(&run.stdout)
    }

    fn prompt(&self, role: &str) -> Output {
        let mut args = vec!["prompt", role, "54"];
        args.extend(REST);
        self.wb(&args)
    }
}

/// A repository set up by `wb init --yes`, committed on `main`, with the patterns of the tracker's stand-ins, a git
/// identity of its own, and signing with Ada's key. `.watchbill/signers` is committed on `main` where it is given.
fn bench(signers: Option<&dyn Fn(&Bench) -> String>) -> Bench {
    let repo = folder_named("brake-controller");
    let home = folder_named("home");
    fs::write(home.path.join(".gitconfig"), "").unwrap();
    let bench = Bench { repo, home };
    bench.git(&["init", "-q"]);
    for (key, value) in [
        ("user.name", "Ada Lovelace"),
        ("user.email", "ada@example.org"),
        ("commit.gpgsign", "false"),
        ("tag.gpgsign", "false"),
        ("gc.auto", "0"),
        ("maintenance.auto", "false"),
    ] {
        bench.git(&["config", key, value]);
    }
    let run = bench.wb(&["init", "--yes"]);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let mut file = fs::OpenOptions::new().append(true).open(config(&bench.repo.path)).unwrap();
    file.write_all(PATTERNS.as_bytes()).unwrap();
    bench.put("src/brake.c", "int brake(void) { return 1; }\n");
    bench.put(".git/info/exclude", ".tracker/\n");
    bench.comments(&[]);
    if let Some(signers) = signers {
        let lines = signers(&bench);
        bench.put(SIGNERS, &lines);
    }
    bench.git(&["add", "--", ".watchbill", "src/brake.c"]);
    bench.git(&["commit", "-q", "-m", "Set up"]);
    bench.git(&["branch", "-M", "main"]);
    bench.sign_with("ada");
    bench
}

/// A bench whose `.watchbill/signers` on `main` is what `lines` gives.
fn bench_with(lines: &dyn Fn(&Bench) -> String) -> Bench {
    bench(Some(lines))
}

/// A bench whose `.watchbill/signers` lets Ada accept, and start a session (#236).
fn ada_accepts() -> Bench {
    bench(Some(&|bench: &Bench| bench.line("ada@example.org", "accept,start", "ada")))
}

/// Job 54 accepted by Ada, signed, on its branch.
fn accepted() -> Bench {
    let bench = ada_accepts();
    bench.job();
    bench.accept();
    bench
}

/// The paths, refs and commits a refusal must leave as they were.
fn snapshot(bench: &Bench) -> (String, String, String) {
    (bench.git(&["for-each-ref"]), bench.git(&["status", "--porcelain", "--ignored"]), bench.git(&["log", "--all", "--format=%H"]))
}

/// Checks the tag with git's own `verify-tag`, against an allowed signers file of the key of `name`.
fn git_verifies(bench: &Bench, tag: &str, name: &str) -> bool {
    let allowed = bench.home.path.join("allowed");
    fs::write(&allowed, format!("ada@example.org {}\n", bench.public(name))).unwrap();
    let setting = format!("gpg.ssh.allowedSignersFile={}", allowed.display());
    bench.git_run(&["-c", &setting, "verify-tag", tag.strip_prefix("refs/tags/").unwrap()]).status.success()
}

fn err_of(run: &Output) -> String {
    text(&run.stderr)
}

fn out_of(run: &Output) -> String {
    text(&run.stdout)
}

// ---- WB-TASK-31: the signed tag of the acceptance ----

#[test]
fn test_WB_TASK_31_plan_accept_makes_a_signed_tag_on_the_blob_of_the_issues_file_named_by_its_acceptance() {
    let bench = accepted();
    let tag = bench.tag();
    assert_eq!(bench.refs("refs/tags/watchbill/"), [tag.clone()]);
    assert_eq!(bench.git(&["cat-file", "-t", &tag]).trim(), "tag");
    let blob = bench.git(&["rev-parse", &format!("HEAD:{}", YML)]).trim().to_string();
    assert_eq!(bench.git(&["rev-parse", &format!("{}^{{}}", tag)]).trim(), blob);
    assert_eq!(bench.git(&["cat-file", "-t", &blob]).trim(), "blob");
    assert!(git_verifies(&bench, &tag, "ada"), "git verify-tag does not pass the tag");
}

#[test]
fn test_WB_TASK_31_the_message_holds_the_files_sha256_the_job_the_identity_and_the_time_of_the_acceptance() {
    let bench = accepted();
    let accepted = read(&bench.at(ACCEPTED));
    let line = |key: &str| accepted.lines().find(|line| line.starts_with(key)).unwrap().to_string();
    let file = hash_bytes(&fs::read(bench.at(YML)).unwrap());
    let expected = format!("Accept the issue's file (54)\n\nsha256: {}\njob: none\n{}\n{}\n", file, line("by: "), line("time: "));
    assert_eq!(bench.message(&bench.tag()), expected);
    assert_eq!(line("by: "), "by: Ada Lovelace <ada@example.org>");
}

#[test]
fn test_WB_TASK_31_the_message_names_the_hash_of_each_source_of_the_plan() {
    let bench = ada_accepts();
    let org = with_level(&bench);
    let added = plan_add(&bench, &org, &["add", "54", "-m", "The brakes hold."]);
    assert_eq!(added.0, 0, "{}", added.1);
    bench.job();
    bench.accept();
    let job = bench.git(&["cat-file", "blob", "refs/watchbill/54:job.yml"]);
    let hash = job.lines().find_map(|line| line.trim().strip_prefix("- hash: ")).unwrap().trim_matches('\'').trim_matches('"').to_string();
    let message = bench.message(&bench.tag());
    assert!(message.contains(&format!("\njob: {}\n", hash)), "{}", message);
}

#[test]
fn test_WB_TASK_31_it_says_the_tag_before_it_makes_it_and_says_to_push_it() {
    let bench = ada_accepts();
    bench.job();
    let run = bench.accept();
    let out = out_of(&run);
    let tag = bench.tag();
    let said = out.find(&format!("will make the signed tag {}", tag)).unwrap_or_else(|| panic!("{}", out));
    assert!(said < out.find("will write").unwrap(), "{}", out);
    assert!(out.contains(&format!("git push origin {}", tag)), "{}", out);
}

#[test]
fn test_WB_TASK_31_accept_of_the_work_makes_a_signed_tag_on_the_commit_its_record_names() {
    let bench = accepted();
    bench.put("src/work.c", "int work;\n");
    bench.git(&["add", "--", "src/work.c"]);
    bench.git(&["commit", "-q", "-m", "The work"]);
    bench.comments(&["[stamp] tester round 1", "[stamp] build round 1"]);
    let work = bench.head();
    let run = bench.wb(&["accept", "54"]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    let record = fs::read(bench.at(RECORD)).unwrap();
    let tag = format!("refs/tags/watchbill/finished/54/{}", &hash_bytes(&record)[..12]);
    assert_eq!(bench.refs("refs/tags/watchbill/finished/"), [tag.clone()]);
    assert_eq!(bench.git(&["cat-file", "-t", &tag]).trim(), "tag");
    assert_eq!(bench.git(&["rev-parse", &format!("{}^{{}}", tag)]).trim(), work);
    assert_eq!(bench.message(&tag), format!("Accept the work (54)\n\n{}", text(&record)));
    assert!(git_verifies(&bench, &tag, "ada"));
    assert!(bench.status().starts_with("Done:\n"), "{}", bench.status());
}

// ---- WB-TASK-32: no signing key ----

#[test]
fn test_WB_TASK_32_with_no_signing_key_plan_accept_ends_with_1_names_the_setting_and_writes_nothing() {
    let bench = ada_accepts();
    bench.job();
    bench.git(&["config", "--unset", "user.signingkey"]);
    let before = snapshot(&bench);
    let run = bench.wb(&["plan", "accept", "54"]);
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains("user.signingkey"), "{}", err_of(&run));
    assert!(!bench.at(ACCEPTED).exists());
    assert_eq!(snapshot(&bench), before);
    assert!(bench.refs("refs/tags/").is_empty());
}

#[test]
fn test_WB_TASK_32_where_git_does_not_sign_with_ssh_plan_accept_ends_with_1_naming_gpg_format() {
    let bench = ada_accepts();
    bench.job();
    bench.git(&["config", "--unset", "gpg.format"]);
    let before = snapshot(&bench);
    let run = bench.wb(&["plan", "accept", "54", "--no-commit"]);
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains("gpg.format"), "{}", err_of(&run));
    assert!(!bench.at(ACCEPTED).exists());
    assert_eq!(snapshot(&bench), before);
}

#[test]
fn test_WB_TASK_32_with_no_signing_key_accept_of_the_work_ends_with_1_and_writes_nothing() {
    let bench = accepted();
    bench.put("src/work.c", "int work;\n");
    bench.git(&["add", "--", "src/work.c"]);
    bench.git(&["commit", "-q", "-m", "The work"]);
    bench.comments(&["[stamp] tester round 1", "[stamp] build round 1"]);
    bench.git(&["config", "--unset", "user.signingkey"]);
    let before = snapshot(&bench);
    let run = bench.wb(&["accept", "54"]);
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains("user.signingkey"), "{}", err_of(&run));
    assert!(!bench.at(RECORD).exists());
    assert_eq!(snapshot(&bench), before);
}

// ---- WB-TASK-33 and WB-TASK-34: a signature that checks, and one that does not ----

#[test]
fn test_WB_TASK_33_a_tag_signed_by_a_key_the_list_holds_with_accept_is_an_acceptance() {
    let bench = accepted();
    let status = bench.status();
    assert!(status.starts_with("Accepted:\n  54  issue-54-work"), "{}", status);
    assert!(status.contains(&format!("{}: signed by ada@example.org, with the role accept in .watchbill/signers of main", bench.tag())), "{}", status);
    let run = bench.prompt("tester");
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
}

#[test]
fn test_WB_TASK_34_an_unsigned_tag_counts_for_nothing_and_status_says_it_is_unsigned() {
    let bench = accepted();
    let tag = bench.tag();
    bench.tag_again(&tag, &["-a", "--no-sign"]);
    assert!(!bench.git(&["cat-file", "tag", &tag]).contains("SIGNATURE"));
    let status = bench.status();
    assert!(status.starts_with("Planned:\n"), "{}", status);
    assert!(status.contains(&format!("{}: unsigned", tag)), "{}", status);
    assert!(!status.contains("accepted: the issue's file"), "{}", status);
}

#[test]
fn test_WB_TASK_34_a_tag_signed_by_a_key_not_in_the_list_counts_for_nothing_and_status_says_so() {
    let bench = accepted();
    let tag = bench.tag();
    let bob = bench.key("bob");
    bench.tag_again(&tag, &["-s", "-u", bob.to_str().unwrap()]);
    let status = bench.status();
    assert!(status.starts_with("Planned:\n"), "{}", status);
    assert!(status.contains(&format!("{}: signed by a key that is not in .watchbill/signers of main", tag)), "{}", status);
}

#[test]
fn test_WB_TASK_34_a_tag_signed_by_a_key_without_accept_counts_for_nothing_and_status_says_so() {
    let bench = accepted();
    bench.signers_on_main(&bench.line("ada@example.org", "start,answer", "ada"));
    let status = bench.status();
    assert!(status.starts_with("Planned:\n"), "{}", status);
    assert!(status.contains(&format!("{}: signed by the key of ada@example.org, which .watchbill/signers of main holds without the role accept", bench.tag())), "{}", status);
}

#[test]
fn test_WB_TASK_34_an_acceptance_with_no_tag_counts_for_nothing_where_the_list_is_there() {
    let bench = accepted();
    let tag = bench.tag();
    bench.git(&["tag", "-d", tag.strip_prefix("refs/tags/").unwrap()]);
    let status = bench.status();
    assert!(status.starts_with("Planned:\n"), "{}", status);
    assert!(status.contains(&format!("{} is not there", tag)), "{}", status);
}

#[test]
fn test_WB_TASK_34_a_work_accepted_with_an_unsigned_tag_is_not_done() {
    let bench = accepted();
    bench.put("src/work.c", "int work;\n");
    bench.git(&["add", "--", "src/work.c"]);
    bench.git(&["commit", "-q", "-m", "The work"]);
    bench.comments(&["[stamp] tester round 1", "[stamp] build round 1"]);
    let run = bench.wb(&["accept", "54"]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    let tag = bench.refs("refs/tags/watchbill/finished/").pop().unwrap();
    bench.tag_again(&tag, &["-a", "--no-sign"]);
    let status = bench.status();
    assert!(status.starts_with("Built:\n"), "{}", status);
    assert!(status.contains(&format!("{}: unsigned", tag)), "{}", status);
}

#[test]
fn test_WB_TASK_34_text_after_a_marker_of_a_signature_in_the_message_makes_it_not_the_acceptances() {
    let bench = accepted();
    let tag = bench.tag();
    let message = format!("{}-----BEGIN SSH SIGNATURE-----\nnot really\n", bench.message(&tag));
    let target = bench.git(&["rev-parse", &format!("{}^{{}}", tag)]).trim().to_string();
    bench.tag_as(&tag, &["-s"], &message, &target);
    let status = bench.status();
    assert!(status.starts_with("Planned:\n"), "{}", status);
    assert!(status.contains(&format!("{}: its message is not that of the acceptance", tag)), "{}", status);
}

/// The words `wb status 54` says of the tag of the acceptance, which must count for nothing.
fn void_by(bench: &Bench, words: &str) {
    let status = bench.status();
    assert!(status.starts_with("Planned:\n"), "{}", status);
    assert!(status.contains(&format!("{}: {}", bench.tag(), words)), "{}", status);
    assert_eq!(code(&bench.prompt("tester")), Some(1));
}

const PGP: &str = "-----BEGIN PGP SIGNATURE-----\n\niQEzBAABCAAdFiEE\n=abcd\n-----END PGP SIGNATURE-----\n";

#[test]
fn test_WB_TASK_34_a_signed_tag_of_the_name_on_another_object_counts_for_nothing() {
    let bench = accepted();
    let tag = bench.tag();
    let other = bench.git(&["rev-parse", "HEAD:src/brake.c"]).trim().to_string();
    bench.tag_as(&tag, &["-s"], &bench.message(&tag), &other);
    void_by(&bench, "it is not on the blob");
}

#[test]
fn test_WB_TASK_34_a_lightweight_tag_of_the_name_is_unsigned() {
    let bench = accepted();
    let tag = bench.tag();
    let blob = bench.git(&["rev-parse", &format!("{}^{{}}", tag)]).trim().to_string();
    bench.git(&["tag", "-f", tag.strip_prefix("refs/tags/").unwrap(), &blob]);
    void_by(&bench, "unsigned");
}

#[test]
fn test_WB_TASK_34_a_signed_tag_whose_message_differs_by_a_line_counts_for_nothing() {
    let bench = accepted();
    let tag = bench.tag();
    let target = bench.git(&["rev-parse", &format!("{}^{{}}", tag)]).trim().to_string();
    bench.tag_as(&tag, &["-s"], &format!("{}more: a line\n", bench.message(&tag)), &target);
    void_by(&bench, "its message is not that of the acceptance");
}

#[test]
fn test_WB_TASK_34_an_openpgp_signature_alone_or_after_the_ssh_one_is_a_key_not_in_the_list() {
    let bench = accepted();
    let tag = bench.tag();
    let (payload, raw) = (bench.payload(&tag), bench.raw(&tag));
    bench.put_tag(&tag, &format!("{}{}", raw, PGP));
    void_by(&bench, "signed by a key that is not in .watchbill/signers of main");
    bench.put_tag(&tag, &format!("{}{}", payload, PGP));
    void_by(&bench, "signed by a key that is not in .watchbill/signers of main");
}

#[test]
fn test_WB_TASK_34_a_tag_changed_after_it_was_signed_does_not_check() {
    let bench = accepted();
    let tag = bench.tag();
    let raw = bench.raw(&tag);
    assert!(raw.contains("\ntagger Ada Lovelace <"), "{}", raw);
    bench.put_tag(&tag, &raw.replace("\ntagger Ada Lovelace <", "\ntagger Ada Lovelacf <"));
    void_by(&bench, "its signature does not check");
}

#[test]
fn test_WB_TASK_34_a_signature_in_another_namespace_than_git_does_not_check() {
    let bench = accepted();
    let tag = bench.tag();
    let payload = bench.payload(&tag);
    let file = bench.home.path.join("payload");
    fs::write(&file, &payload).unwrap();
    let key = bench.key("ada");
    let run = bench.run("ssh-keygen", &["-q", "-Y", "sign", "-n", "file", "-f", key.to_str().unwrap(), file.to_str().unwrap()], &bench.home.path);
    assert!(run.status.success(), "{}", text(&run.stderr));
    let signature = read(&PathBuf::from(format!("{}.sig", file.display())));
    bench.put_tag(&tag, &format!("{}{}", payload, signature));
    void_by(&bench, "its signature does not check");
}

#[test]
fn test_WB_TASK_36_a_list_on_main_with_a_comma_or_a_pattern_makes_every_acceptance_count_for_nothing() {
    let bench = accepted();
    let key = bench.public("ada");
    bench.signers_on_main(&format!("eve@example.org,ada@example.org roles=\"accept\" {}\n", key));
    let status = bench.status();
    assert!(status.starts_with("Planned:\n"), "{}", status);
    assert!(status.contains(".watchbill/signers of main cannot be read: line 1:"), "{}", status);
    assert_eq!(code(&bench.prompt("tester")), Some(1));

    let bench = bench_with(&|bench: &Bench| format!("ada@* roles=\"accept\" {}\n", bench.public("ada")));
    bench.job();
    let before = snapshot(&bench);
    let run = bench.wb(&["plan", "accept", "54"]);
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains(".watchbill/signers of main cannot be read: line 1:"), "{}", err_of(&run));
    assert!(!bench.at(ACCEPTED).exists());
    assert_eq!(snapshot(&bench), before);
}

#[test]
fn test_WB_TASK_34_one_identity_with_two_keys_signs_as_accept_only_with_the_key_that_holds_it() {
    let bench = bench_with(&|bench: &Bench| {
        format!("{}{}", bench.line("ada@example.org", "start", "ada"), bench.line("ada@example.org", "accept", "ada-desk"))
    });
    bench.job();
    let before = snapshot(&bench);
    let run = bench.wb(&["plan", "accept", "54"]);
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains("which .watchbill/signers of main holds without the role accept"), "{}", err_of(&run));
    assert!(err_of(&run).contains("It is removed again"), "{}", err_of(&run));
    assert_eq!(snapshot(&bench), before);
    bench.sign_with("ada-desk");
    bench.accept();
    assert!(bench.status().contains("signed by ada@example.org, with the role accept"), "{}", bench.status());
}

#[test]
fn test_WB_TASK_34_a_tag_larger_than_its_cap_counts_for_nothing() {
    let bench = accepted();
    let tag = bench.tag();
    let target = bench.git(&["rev-parse", &format!("{}^{{}}", tag)]).trim().to_string();
    let filler = format!("{}\n", "x".repeat(100)).repeat(700);
    bench.tag_as(&tag, &["-s"], &format!("{}{}", bench.message(&tag), filler), &target);
    void_by(&bench, "it is larger than 64 KiB");
}

// ---- WB-TASK-35: no prompt of a later role ----

#[test]
fn test_WB_TASK_35_prompt_of_a_later_role_refuses_with_1_for_a_tag_unsigned_or_signed_by_a_key_not_in_the_list() {
    for role in ["tester", "build", "review", "fix"] {
        let bench = accepted();
        let tag = bench.tag();
        bench.tag_again(&tag, &["-a", "--no-sign"]);
        let run = bench.prompt(role);
        assert_eq!(code(&run), Some(1), "{}: {}", role, out_of(&run));
        assert!(out_of(&run).is_empty(), "{}", role);
        assert!(err_of(&run).contains("unsigned") && err_of(&run).contains("No prompt is handed out"), "{}: {}", role, err_of(&run));
    }
    let bench = accepted();
    bench.signers_on_main(&bench.line("bob@example.org", "accept", "bob"));
    let run = bench.prompt("tester");
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains("a key that is not in .watchbill/signers of main"), "{}", err_of(&run));
}

#[test]
fn test_WB_TASK_35_prompt_refuses_with_1_for_a_tag_signed_by_a_key_without_accept() {
    let bench = accepted();
    bench.signers_on_main(&bench.line("ada@example.org", "start", "ada"));
    let run = bench.prompt("build");
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains("without the role accept"), "{}", err_of(&run));
}

// ---- WB-TASK-36: the list as the default branch has it ----

#[test]
fn test_WB_TASK_36_a_key_added_on_the_issues_branch_alone_is_not_trusted() {
    let bench = bench(Some(&|bench: &Bench| bench.line("bob@example.org", "accept", "bob")));
    bench.job();
    // The issue's branch adds Ada's key; the default branch does not have it.
    let mut lines = read(&bench.at(SIGNERS));
    lines.push_str(&bench.line("ada@example.org", "accept", "ada"));
    bench.put(SIGNERS, &lines);
    bench.git(&["add", "--", SIGNERS]);
    bench.git(&["commit", "-q", "-m", "Ada's key"]);
    let before = snapshot(&bench);
    let run = bench.wb(&["plan", "accept", "54"]);
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains("not in .watchbill/signers of main"), "{}", err_of(&run));
    assert!(!bench.at(ACCEPTED).exists());
    assert_eq!(snapshot(&bench), before);
    assert!(bench.refs("refs/tags/").is_empty());
}

#[test]
fn test_WB_TASK_36_a_tag_checks_by_the_list_of_the_default_branch_not_by_the_one_of_the_checkout() {
    let bench = accepted();
    // Ada's key leaves the list of `main`, and stays in the working tree and on the issue's branch.
    bench.signers_on_main(&bench.line("bob@example.org", "accept", "bob"));
    assert!(read(&bench.at(SIGNERS)).contains("ada@example.org"));
    let status = bench.status();
    assert!(status.starts_with("Planned:\n"), "{}", status);
    assert!(status.contains("not in .watchbill/signers of main"), "{}", status);
}

#[test]
fn test_WB_TASK_36_check_passes_the_four_roles_and_refuses_a_role_it_does_not_know_with_1_naming_the_line() {
    let bench = bench(None);
    let mut lines = String::from("# The keys of the project.\n");
    lines.push_str(&bench.line("ada@example.org", "accept,start,handover,answer", "ada"));
    bench.put(SIGNERS, &lines);
    let run = bench.wb(&["check"]);
    assert_eq!(code(&run), Some(0), "{}{}", out_of(&run), err_of(&run));
    assert!(out_of(&run).contains("The repository is at the revision it is pinned to"), "{}", out_of(&run));
    lines.push_str(&bench.line("bob@example.org", "accept,deploy", "bob"));
    bench.put(SIGNERS, &lines);
    let run = bench.wb(&["check"]);
    assert_eq!(code(&run), Some(1), "{}", err_of(&run));
    let out = out_of(&run);
    assert!(out.contains(".watchbill/signers: line 3: the role `deploy` is not known"), "{}", out);
    assert!(!out.contains("line 2"), "{}", out);
}

#[test]
fn test_WB_TASK_36_check_names_a_line_that_is_not_a_key_of_the_list() {
    let bench = bench(None);
    let mut lines = bench.line("ada@example.org", "accept", "ada");
    lines.push_str("bob@example.org ssh-ed25519 AAAA\n");
    lines.push_str(&bench.line("ada@example.org", "start", "ada"));
    lines.push_str(&bench.line("eve@*", "accept", "bob"));
    bench.put(SIGNERS, &lines);
    let run = bench.wb(&["check"]);
    assert_eq!(code(&run), Some(1), "{}", err_of(&run));
    let out = out_of(&run);
    for line in ["line 2:", "line 3:", "line 4:"] {
        assert!(out.contains(&format!(".watchbill/signers: {}", line)), "{}: {}", line, out);
    }
    assert!(!out.contains("line 1:"), "{}", out);
}

#[test]
fn test_WB_TASK_36_a_replace_of_the_list_of_main_is_not_trusted() {
    let bench = bench(Some(&|bench: &Bench| bench.line("bob@example.org", "accept", "bob")));
    bench.job();
    // A replace ref swaps the blob of `main:.watchbill/signers` for a list that holds Ada; `main` itself does not.
    let other = bench.home.path.join("signers");
    fs::write(&other, bench.line("ada@example.org", "accept", "ada")).unwrap();
    let new = bench.git(&["hash-object", "-w", other.to_str().unwrap()]).trim().to_string();
    let old = bench.git(&["rev-parse", &format!("main:{}", SIGNERS)]).trim().to_string();
    bench.git(&["replace", &old, &new]);
    assert!(bench.git(&["show", &format!("main:{}", SIGNERS)]).contains("ada@example.org"));
    let before = snapshot(&bench);
    let run = bench.wb(&["plan", "accept", "54"]);
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains("not in .watchbill/signers of main"), "{}", err_of(&run));
    assert!(!bench.at(ACCEPTED).exists());
    assert_eq!(snapshot(&bench), before);
    assert!(bench.refs("refs/tags/").is_empty());
}

#[test]
fn test_WB_TASK_36_a_replace_of_the_issues_file_by_the_blob_accepted_is_not_followed() {
    let bench = accepted();
    let accepted_blob = bench.git(&["rev-parse", &format!("HEAD:{}", YML)]).trim().to_string();
    let edited = BARE_ISSUE.replace("{issue}", "54").replace("  none", "  fn brake()");
    bench.put(YML, &edited);
    bench.git(&["commit", "-q", "-a", "-m", "An edit"]);
    let edited_blob = bench.git(&["rev-parse", &format!("HEAD:{}", YML)]).trim().to_string();
    bench.git(&["replace", &edited_blob, &accepted_blob]);
    let status = bench.status();
    assert!(status.starts_with("Planned:\n"), "{}", status);
    assert!(!status.contains("signed by ada@example.org"), "{}", status);
}

#[test]
fn test_WB_TASK_36_a_replace_of_a_later_commit_by_the_acceptance_of_the_work_is_not_followed() {
    let bench = accepted();
    bench.put("src/work.c", "int work;\n");
    bench.git(&["add", "--", "src/work.c"]);
    bench.git(&["commit", "-q", "-m", "The work"]);
    bench.comments(&["[stamp] tester round 1", "[stamp] build round 1"]);
    let run = bench.wb(&["accept", "54"]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    let record = bench.head();
    assert!(bench.status().starts_with("Done:\n"), "{}", bench.status());
    bench.put("src/work.c", "int work = 2;\n");
    bench.git(&["commit", "-q", "-a", "-m", "Later work"]);
    let later = bench.head();
    bench.git(&["replace", &later, &record]);
    let status = bench.status();
    assert!(!status.starts_with("Done:\n"), "{}", status);
    assert!(!status.contains("accepted: the work"), "{}", status);
    let run = bench.wb(&["revert", "54", "build"]);
    assert!(!err_of(&run).contains("is done"), "{}", err_of(&run));
}

// ---- WB-TASK-56 to WB-TASK-62: no list, nothing counts (#323) ----

/// The line `wb status <issue>` says where the default branch holds no list.
const NOTHING_COUNTS: &str = "no .watchbill/signers on main: nothing is signed, so nothing counts";

/// An acceptance of 54 written by hand, as `wb plan accept` writes `.accepted`, with no tag, committed on the branch.
fn accepted_by_hand(bench: &Bench) {
    let hash = hash_bytes(&fs::read(bench.at(YML)).unwrap());
    bench.put(ACCEPTED, &format!("sha256: {}\nby: Ada Lovelace <ada@example.org>\ntime: 2026-11-25T10:00:00Z\n", hash));
    bench.git(&["add", "--", ACCEPTED]);
    bench.git(&["commit", "-q", "-m", "The acceptance (54), by hand"]);
}

/// `.watchbill/signers` taken out of `main`, and the branch of the job brought back.
fn no_list_on_main(bench: &Bench) {
    let back = bench.git(&["symbolic-ref", "--short", "HEAD"]).trim().to_string();
    bench.git(&["checkout", "-q", "main"]);
    bench.git(&["rm", "-q", "--", SIGNERS]);
    bench.git(&["commit", "-q", "-m", "No keys"]);
    bench.git(&["checkout", "-q", &back]);
}

#[test]
fn test_WB_TASK_56_with_no_list_plan_accept_ends_with_1_writes_nothing_and_names_the_file_and_how_to_add_a_key() {
    // Git signs with Ada's key: only the list is missing.
    let bench = bench(None);
    bench.job();
    let before = snapshot(&bench);
    let run = bench.wb(&["plan", "accept", "54"]);
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    let err = err_of(&run);
    assert!(err.contains(".watchbill/signers"), "{}", err);
    assert!(err.contains("roles=\"accept\""), "{}", err);
    assert!(!bench.at(ACCEPTED).exists());
    assert_eq!(snapshot(&bench), before);
    assert!(bench.refs("refs/tags/").is_empty());
    let run = bench.wb(&["plan", "accept", "54", "--no-commit"]);
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(!bench.at(ACCEPTED).exists());
    assert_eq!(snapshot(&bench), before);
}

#[test]
fn test_WB_TASK_56_with_no_list_accept_of_the_work_ends_with_1_writes_nothing_and_names_the_file() {
    let bench = bench(None);
    bench.job();
    accepted_by_hand(&bench);
    bench.put("src/work.c", "int work;\n");
    bench.git(&["add", "--", "src/work.c"]);
    bench.git(&["commit", "-q", "-m", "The work"]);
    bench.comments(&["[stamp] tester round 1", "[stamp] build round 1"]);
    let before = snapshot(&bench);
    let run = bench.wb(&["accept", "54"]);
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    let err = err_of(&run);
    assert!(err.contains(".watchbill/signers"), "{}", err);
    assert!(err.contains("roles=\"accept\""), "{}", err);
    assert!(!bench.at(RECORD).exists());
    assert_eq!(snapshot(&bench), before);
    assert!(bench.refs("refs/tags/").is_empty());
}

#[test]
fn test_WB_TASK_57_an_accepted_written_by_hand_with_no_tag_counts_for_nothing_with_no_list() {
    let bench = bench(None);
    bench.job();
    accepted_by_hand(&bench);
    let status = bench.status();
    assert!(status.starts_with("Planned:\n"), "{}", status);
    assert!(status.contains(NOTHING_COUNTS), "{}", status);
    assert!(!status.contains("accepted: the issue's file"), "{}", status);
    let run = bench.prompt("tester");
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(out_of(&run).is_empty(), "{}", out_of(&run));
    assert!(err_of(&run).contains(".watchbill/signers"), "{}", err_of(&run));
}

#[test]
fn test_WB_TASK_57_a_signed_acceptance_counts_for_nothing_once_the_list_is_gone_from_main() {
    let bench = accepted();
    assert!(bench.status().starts_with("Accepted:\n"), "{}", bench.status());
    no_list_on_main(&bench);
    let status = bench.status();
    assert!(status.starts_with("Planned:\n"), "{}", status);
    assert!(status.contains(NOTHING_COUNTS), "{}", status);
    assert_eq!(code(&bench.prompt("tester")), Some(1));
}

#[test]
fn test_WB_TASK_58_a_finished_with_no_list_counts_for_nothing_and_the_job_is_not_done() {
    let bench = accepted();
    bench.put("src/work.c", "int work;\n");
    bench.git(&["add", "--", "src/work.c"]);
    bench.git(&["commit", "-q", "-m", "The work"]);
    bench.comments(&["[stamp] tester round 1", "[stamp] build round 1"]);
    let run = bench.wb(&["accept", "54"]);
    assert_eq!(code(&run), Some(0), "{}", err_of(&run));
    assert!(bench.status().starts_with("Done:\n"), "{}", bench.status());
    no_list_on_main(&bench);
    let status = bench.status();
    assert!(!status.contains("Done:"), "{}", status);
    assert!(!status.contains("accepted: "), "{}", status);
    assert!(status.contains(NOTHING_COUNTS), "{}", status);
    // The list of the issues in the flow lists it again.
    let all = text(&bench.wb(&["status"]).stdout);
    assert!(all.contains("  54  issue-54-work"), "{}", all);
}

#[test]
fn test_WB_TASK_59_status_of_an_issue_says_the_line_once_for_an_acceptance_and_stamps_with_no_list() {
    let bench = bench(None);
    bench.job();
    accepted_by_hand(&bench);
    // Two stamps on the job's ref, signed by Ada's key, which no list holds.
    bench.comments(&["[stamp] tester round 1", "[stamp] build round 1"]);
    let status = bench.status();
    assert_eq!(status.matches(NOTHING_COUNTS).count(), 1, "{}", status);
    assert!(status.starts_with("Planned:\n"), "{}", status);
    assert!(status.contains("[stamp] tester round 1  (on refs/watchbill/54, not counted"), "{}", status);
    assert!(status.contains("[stamp] build round 1  (on refs/watchbill/54, not counted"), "{}", status);
}

#[test]
fn test_WB_TASK_60_check_with_no_list_in_the_working_tree_ends_with_1_naming_it() {
    let bench = bench(None);
    let run = bench.wb(&["check"]);
    assert_eq!(code(&run), Some(1), "{}{}", out_of(&run), err_of(&run));
    assert!(out_of(&run).contains("missing: .watchbill/signers"), "{}", out_of(&run));
    let bench = ada_accepts();
    let run = bench.wb(&["check"]);
    assert_eq!(code(&run), Some(0), "{}{}", out_of(&run), err_of(&run));
}

/// The lines of a list that holds its form and no key: a comment and an empty line.
fn no_key(_: &Bench) -> String {
    "# The keys of the project.\n\n".to_string()
}

#[test]
fn test_WB_TASK_60_check_of_a_list_with_no_key_line_ends_with_1_naming_it() {
    let bench = bench_with(&no_key);
    let run = bench.wb(&["check"]);
    assert_eq!(code(&run), Some(1), "{}{}", out_of(&run), err_of(&run));
    assert!(out_of(&run).contains(".watchbill/signers") && out_of(&run).contains("no key"), "{}", out_of(&run));
}

#[test]
fn test_WB_TASK_56_a_list_with_no_key_line_on_main_is_missing_and_plan_accept_makes_no_tag() {
    let bench = bench_with(&no_key);
    bench.job();
    let before = snapshot(&bench);
    let run = bench.wb(&["plan", "accept", "54"]);
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    // Refused before any tag is made, not after one is made and removed again.
    assert!(!out_of(&run).contains("will make the signed tag"), "{}", out_of(&run));
    let err = err_of(&run);
    assert!(err.contains(".watchbill/signers on main holds no key") && err.contains("roles=\"accept\""), "{}", err);
    assert!(!bench.at(ACCEPTED).exists());
    assert_eq!(snapshot(&bench), before);
    assert!(bench.refs("refs/tags/").is_empty());
}

#[test]
fn test_WB_TASK_59_status_says_a_list_with_no_key_line_counts_nothing() {
    let bench = bench_with(&no_key);
    bench.job();
    accepted_by_hand(&bench);
    let status = bench.status();
    assert!(status.starts_with("Planned:\n"), "{}", status);
    assert_eq!(status.matches(".watchbill/signers on main holds no key: nothing is signed, so nothing counts").count(), 1, "{}", status);
}

#[test]
fn test_WB_TASK_62_with_no_default_branch_plan_accept_is_refused_as_with_no_list() {
    let bench = ada_accepts();
    bench.job();
    // The list is on the branch that was `main`; under another name it is no default branch.
    bench.git(&["branch", "-m", "main", "trunk"]);
    let before = snapshot(&bench);
    let run = bench.wb(&["plan", "accept", "54"]);
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains("no default branch"), "{}", err_of(&run));
    assert!(err_of(&run).contains(".watchbill/signers"), "{}", err_of(&run));
    assert!(!bench.at(ACCEPTED).exists());
    assert_eq!(snapshot(&bench), before);
}

#[test]
fn test_WB_TASK_62_with_no_default_branch_a_signed_acceptance_counts_for_nothing() {
    let bench = accepted();
    assert_eq!(code(&bench.prompt("tester")), Some(0));
    bench.git(&["branch", "-m", "main", "trunk"]);
    let run = bench.prompt("tester");
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(err_of(&run).contains("no default branch"), "{}", err_of(&run));
}

// ---- WB-TASK-38: an edit after the tag ----

#[test]
fn test_WB_TASK_38_an_edit_of_the_issues_file_after_its_tag_leaves_it_not_accepted() {
    let bench = accepted();
    assert_eq!(bench.refs("refs/tags/watchbill/accept/").len(), 1);
    let edited = BARE_ISSUE.replace("{issue}", "54").replace("  none", "  fn brake()");
    bench.put(YML, &edited);
    bench.git(&["commit", "-q", "-a", "-m", "An edit"]);
    let status = bench.status();
    assert!(status.starts_with("Planned:\n"), "{}", status);
    let run = bench.prompt("tester");
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    // The tag of the file as it was stays, and does not hold for the file as it is.
    assert_eq!(bench.refs("refs/tags/watchbill/accept/").len(), 1);
}

// ---- WB-TASK-39: the notice ----

#[test]
fn test_WB_TASK_39_plan_accept_posts_the_accepted_notice_by_issue_comment_after_it_says_it() {
    let bench = ada_accepts();
    bench.job();
    let run = bench.accept();
    let posted = read(&bench.at(".tracker/54.bodies"));
    let file = hash_bytes(&fs::read(bench.at(YML)).unwrap());
    assert!(posted.starts_with("[accepted] 54\n"), "{}", posted);
    assert!(posted.contains(&format!("sha256: {}\n", file)), "{}", posted);
    assert!(posted.contains(&format!("tag: {}\n", bench.tag())), "{}", posted);
    // It names the commit of the acceptance (#240).
    assert_eq!(posted.lines().last(), Some(format!("commit: {}", bench.git(&["rev-parse", "HEAD"]).trim()).as_str()), "{}", posted);
    assert!(out_of(&run).contains("will post on issue 54, by `issue_comment` of watchbill.yml:\n[accepted] 54\n"), "{}", out_of(&run));
}

#[test]
fn test_WB_TASK_39_an_accepted_comment_is_never_read_back() {
    // An acceptance committed and signed, whose notice is posted; then its tag is gone, and the notice alone is left.
    let bench = accepted();
    let tag = bench.tag();
    let notice = read(&bench.at(".tracker/54.bodies"));
    assert!(notice.starts_with("[accepted] 54\n") && notice.contains(&format!("tag: {}\n", tag)), "{}", notice);
    assert!(read(&bench.at(".tracker/54.comments")).contains("[accepted] 54"));
    bench.git(&["tag", "-d", tag.strip_prefix("refs/tags/").unwrap()]);
    let status = bench.status();
    assert!(status.starts_with("Planned:\n"), "{}", status);
    assert!(status.contains(&format!("{} is not there", tag)), "{}", status);
    assert_eq!(code(&bench.prompt("tester")), Some(1));
}

// ---- WB-TASK-40: `.accepted` stays ----

#[test]
fn test_WB_TASK_40_the_accepted_file_is_written_beside_the_tag_and_is_still_read() {
    let bench = accepted();
    let tag = bench.tag();
    assert!(bench.at(ACCEPTED).is_file());
    assert_eq!(bench.git(&["show", "--name-only", "--format=", "HEAD"]).trim(), ACCEPTED);
    // With `.accepted` gone, the tag alone is no acceptance.
    bench.git(&["rm", "-q", "--", ACCEPTED]);
    bench.git(&["commit", "-q", "-m", "No acceptance"]);
    assert_eq!(bench.refs("refs/tags/watchbill/accept/"), [tag]);
    let status = bench.status();
    assert!(status.starts_with("Planned:\n"), "{}", status);
    assert_eq!(code(&bench.prompt("tester")), Some(1));
}

// ---- WB-TASK-41 and WB-TASK-42: text taken in after the acceptance ----

/// The variables that make this test binary, run as a child, the runner of one `wb plan add` (`plan_add`).
const CHILD_WORDS: &str = "WB_SIGNED_CHILD_WORDS";
const CHILD_ORG: &str = "WB_SIGNED_CHILD_ORG";
const CHILD_SAID: &str = "WB_SIGNED_CHILD_SAID";

/// The test that this binary runs as a child to run `wb plan add`.
const CHILD_TEST: &str = "test_WB_TASK_41_plan_add_runs_in_a_child_with_the_tests_own_home";

/// `wb plan add` through `watchbill::plan`, with the organisation's file at `org`, which `wb` as a program never takes
/// (#93). It runs in a child of this test binary, with the environment of the bench (`env_in`), so that git reads the
/// bench's home and never the person's.
fn plan_add(bench: &Bench, org: &Path, args: &[&str]) -> (i32, String) {
    let said = bench.home.path.join("plan-add-said");
    let run = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD_TEST, "--test-threads=1", "--quiet"])
        .current_dir(&bench.repo.path)
        .env_clear()
        .envs(env_in(&bench.home.path))
        .env(CHILD_WORDS, args.join("\u{1f}"))
        .env(CHILD_ORG, org)
        .env(CHILD_SAID, &said)
        .stdin(Stdio::null())
        .output()
        .expect("the test binary could not be started");
    let code = run.status.code().unwrap_or(-1);
    (code, fs::read_to_string(&said).unwrap_or_else(|_| format!("the child said nothing: {}{}", text(&run.stdout), text(&run.stderr))))
}

#[test]
fn test_WB_TASK_41_plan_add_runs_in_a_child_with_the_tests_own_home() {
    if let (Ok(words), Ok(org), Ok(said)) = (std::env::var(CHILD_WORDS), std::env::var(CHILD_ORG), std::env::var(CHILD_SAID)) {
        // The child: one `wb plan add` in the folder it was started in, and its status as the status of the process.
        let words: Vec<OsString> = words.split('\u{1f}').map(OsString::from).collect();
        let mut input = std::io::Cursor::new(Vec::new());
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let folder = std::env::current_dir().unwrap();
        let code = watchbill::plan(&words, &folder, Path::new(&org), &mut input, &mut out, &mut err);
        fs::write(said, format!("{}{}", text(&out), text(&err))).unwrap();
        std::process::exit(code);
    }
    // The parent: the plan is taken in, as the bench's identity, in a child whose home is the bench's.
    let bench = ada_accepts();
    let org = with_level(&bench);
    let (code, said) = plan_add(&bench, &org, &["add", "54", "-m", "The brakes hold."]);
    assert_eq!(code, 0, "{}", said);
    assert!(bench.git(&["cat-file", "blob", "refs/watchbill/54:job.yml"]).contains("Ada Lovelace"));
}

/// Sets the repository at the level `internal`, and gives the bench's home the organisation's file: its path.
fn with_level(bench: &Bench) -> PathBuf {
    let org = bench.home.path.join("organisation.yml");
    fs::write(&org, ORG).unwrap();
    let mut yml = read(&config(&bench.repo.path));
    yml.push_str("confidentiality: internal\n");
    fs::write(config(&bench.repo.path), yml).unwrap();
    bench.git(&["commit", "-q", "-a", "-m", "The level"]);
    org
}

/// Job 54 as a plan of one source, accepted by Ada, signed; then other text taken in under its name. The organisation's
/// file is that of the bench's home.
fn taken_in_again() -> (Bench, PathBuf) {
    let bench = ada_accepts();
    let org = with_level(&bench);
    let (code, said) = plan_add(&bench, &org, &["add", "54", "-m", "The brakes hold."]);
    assert_eq!(code, 0, "{}", said);
    bench.job();
    // A source of the acceptance's own second counts as after it (#290): the acceptance comes in the next second.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    bench.accept();
    assert!(bench.status().starts_with("Accepted:\n"), "{}", bench.status());
    let (code, said) = plan_add(&bench, &org, &["add", "54", "-m", "On a slope too."]);
    assert_eq!(code, 0, "{}", said);
    (bench, org)
}

#[test]
fn test_WB_TASK_41_after_wb_add_takes_in_changed_text_status_says_the_issue_is_not_accepted() {
    let (bench, _org) = taken_in_again();
    let status = bench.status();
    assert!(status.starts_with("Planned:\n"), "{}", status);
    assert!(status.contains(&format!("{}: names the job", bench.tag())) && status.contains("and the job is now"), "{}", status);
}

#[test]
fn test_WB_TASK_42_after_wb_add_takes_in_changed_text_prompt_of_a_later_role_refuses_with_1() {
    let (bench, _org) = taken_in_again();
    let run = bench.prompt("tester");
    assert_eq!(code(&run), Some(1), "{}", out_of(&run));
    assert!(out_of(&run).is_empty());
    assert!(err_of(&run).contains("names the job") && err_of(&run).contains("No prompt is handed out"), "{}", err_of(&run));
}

#[test]
fn test_WB_PUSH_13_the_accepted_notice_ends_with_the_hash_of_the_commit_of_the_acceptance() {
    let bench = ada_accepts();
    bench.job();
    bench.accept();
    let posted = read(&bench.at(".tracker/54.bodies"));
    let head = bench.git(&["rev-parse", "HEAD"]).trim().to_string();
    assert!(posted.starts_with("[accepted] 54\n"), "{}", posted);
    assert_eq!(posted.lines().last(), Some(format!("commit: {}", head).as_str()), "{}", posted);
}
