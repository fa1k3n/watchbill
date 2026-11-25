//! `wb doctor` (#178): what this machine needs to run Watchbill for this repository, one line to a check, before a
//! session fails on it. It reads only and asks nothing of the network.
//!
//! The command is run in the test process through `watchbill::doctor`, with the machine it looks at given by the
//! test: the path of the organisation's file, bubblewrap, the home folder, `PATH` and the folder that stands in for
//! `/proc/net`. Each check has a stand-in for what it looks at.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Output;

use watchbill::doctor::Host;

fn script(path: &Path, body: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, format!("#!/bin/sh\n{}", body)).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// The organisation's file: `stand-in` is the default runner, starts `session` of `PATH`, and binds the folder of the
/// tools and the person's `~/.claude`.
const ORG: &str = "levels: [public, internal]
default: stand-in
runners:
  stand-in:
    tools: this machine
    model: a stand-in
    agreement: none
    up_to: internal
    start: ['session', '{model}', '{effort}', '{prompt}']
    binds: ['{tools}', '~/.claude']
";

/// A repository set up by `wb init --yes` with its patterns committed, the folder of the stand-in tools that is
/// `PATH`, the organisation's file, a home folder with `~/.claude`, a bubblewrap that works, and a folder that stands
/// in for `/proc/net` with no listening socket.
struct Bench {
    repo: Folder,
    tools: Folder,
    org: Folder,
    home: Folder,
    net: Folder,
}

fn bench() -> Bench {
    let repo = repository(None);
    let tools = folder_named("tools");
    let org = folder_named("org");
    let home = folder_named("home");
    let net = folder_named("net");
    for tool in ["git", "gh", "jq", "session"] {
        script(&tools.path.join(tool), "exit 0\n");
    }
    script(&tools.path.join("bwrap"), "exit 0\n");
    fs::create_dir_all(home.path.join(".claude")).unwrap();
    fs::write(net.path.join("tcp"), HEADER).unwrap();
    fs::write(net.path.join("tcp6"), HEADER).unwrap();
    fs::write(org.path.join("organisation.yml"), ORG.replace("{tools}", tools.path.to_str().unwrap())).unwrap();
    git(&repo.path, &["config", "user.name", "Ada Lovelace"]);
    git(&repo.path, &["config", "user.email", "ada@example.org"]);
    git(&repo.path, &["config", "commit.gpgsign", "false"]);
    let run = wb(&repo.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    commit_config(&repo.path);
    Bench { repo, tools, org, home, net }
}

impl Bench {
    fn organisation(&self) -> PathBuf {
        self.org.path.join("organisation.yml")
    }

    fn bwrap(&self) -> PathBuf {
        self.tools.path.join("bwrap")
    }

    /// Adds lines to `watchbill.yml` and commits it.
    fn add(&self, lines: &str) {
        let mut yml = read(&config(&self.repo.path));
        yml.push_str(lines);
        fs::write(config(&self.repo.path), yml).unwrap();
        commit_config(&self.repo.path);
    }

    /// `wb doctor` with the given words, on this machine.
    fn doctor_with(&self, args: &[&str], path: &Path, organisation: &Path, sandbox: &Path) -> Output {
        let args: Vec<std::ffi::OsString> = args.iter().map(std::ffi::OsString::from).collect();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let host = Host { organisation, sandbox, home: Some(&self.home.path), path: Some(path.as_os_str()), net: &self.net.path };
        let status = watchbill::doctor(&args, &self.repo.path, &host, &mut out, &mut err);
        Output { status: std::process::ExitStatus::from_raw(status << 8), stdout: out, stderr: err }
    }

    fn doctor(&self) -> Output {
        self.doctor_with(&[], &self.tools.path, &self.organisation(), &self.bwrap())
    }
}

/// The first line of `/proc/net/tcp`.
const HEADER: &str = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n";

/// A line of `/proc/net/tcp` for a socket in `state` at `address` (as the kernel writes it).
fn socket(address: &str, state: &str) -> String {
    format!("   0: {} 00000000:0000 {} 00000000:00000000 00:00000000 00000000  1000        0 12345 1 0000000000000000 100 0 0 10 0\n", address, state)
}

/// The lines of the output that say the check `name`.
fn lines_of(run: &Output, name: &str) -> Vec<String> {
    let marker = format!(": {}: ", name);
    text(&run.stdout).lines().filter(|line| line.contains(&marker)).map(str::to_string).collect()
}

/// The one line of the check `name`.
fn line_of(run: &Output, name: &str) -> String {
    let found = lines_of(run, name);
    assert_eq!(found.len(), 1, "{}: {:?}\n{}", name, found, text(&run.stdout));
    found[0].clone()
}

fn status_of(line: &str) -> &str {
    line.split(':').next().unwrap()
}

fn all_hold(run: &Output) {
    let out = text(&run.stdout);
    assert_eq!(code(run), Some(0), "{}\n{}", out, text(&run.stderr));
    assert!(out.lines().all(|line| line.starts_with("ok: ") || line.starts_with("warning: ")), "{}", out);
}

#[test]
fn test_WB_CMD_51_q_leaves_out_the_ok_lines_and_keeps_a_warning_and_a_fault() {
    let bench = bench();
    bench.add("issue_comments: 'nowhere-wb-tool x'\n");

    let run = bench.doctor_with(&["-q"], &bench.tools.path, &bench.organisation(), &bench.bwrap());
    let long = bench.doctor_with(&["--quiet"], &bench.tools.path, &bench.organisation(), &bench.bwrap());
    let twice = bench.doctor_with(&["-q", "--quiet"], &bench.tools.path, &bench.organisation(), &bench.bwrap());

    let out = text(&run.stdout);
    assert_eq!(code(&run), Some(1), "{}", out);
    assert!(out.lines().all(|line| !line.starts_with("ok: ")) && out.contains("fault: tools: nowhere-wb-tool"), "{}", out);
    assert_eq!(text(&long.stdout), out);
    assert_eq!(code(&twice), Some(2), "{}", text(&twice.stderr));
    assert!(text(&twice.stderr).contains("'-q' and '--quiet' are both given"), "{}", text(&twice.stderr));
}

#[test]
fn test_WB_CMD_51_it_ends_with_0_when_all_holds_and_says_one_line_to_a_check() {
    let bench = bench();

    let run = bench.doctor();

    all_hold(&run);
    let out = text(&run.stdout);
    for name in ["bubblewrap", "user namespaces", "git", "organisation", "default runner", "binds", "hooks", "needs", "ports"] {
        assert!(!lines_of(&run, name).is_empty(), "{}: {}", name, out);
    }
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_CMD_51_it_ends_with_1_when_a_check_is_a_fault_and_goes_on_to_the_others() {
    let bench = bench();
    fs::remove_file(bench.organisation()).unwrap();

    let run = bench.doctor();

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    assert_eq!(status_of(&line_of(&run, "organisation")), "fault");
    assert_eq!(status_of(&line_of(&run, "git")), "ok");
    assert_eq!(status_of(&line_of(&run, "needs")), "ok");
}

#[test]
fn test_WB_CMD_51_a_warning_is_not_a_fault() {
    let bench = bench();
    fs::write(bench.net.path.join("tcp"), format!("{}{}", HEADER, socket("0100007F:1F90", "0A"))).unwrap();

    let run = bench.doctor();

    all_hold(&run);
    assert!(text(&run.stdout).lines().any(|line| line.starts_with("warning: ")));
}

#[test]
fn test_WB_CMD_51_a_word_is_used_wrongly_and_ends_with_2() {
    let bench = bench();

    for args in [&["now"][..], &["--force"][..], &["-x"][..]] {
        let run = bench.doctor_with(args, &bench.tools.path, &bench.organisation(), &bench.bwrap());

        assert_eq!(code(&run), Some(2), "{:?}", args);
        assert_eq!(text(&run.stdout), "", "{:?}", args);
        assert!(text(&run.stderr).starts_with("wb doctor: "), "{}", text(&run.stderr));
    }
}

#[test]
fn test_WB_CMD_51_outside_a_repository_and_where_not_set_up_it_ends_with_2() {
    let folder = folder();
    let run = wb(&folder.path, &["doctor"], "");
    assert_eq!(code(&run), Some(2), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("not in a git repository"), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "");

    let repo = repository(None);
    let run = wb(&repo.path, &["doctor"], "");
    assert_eq!(code(&run), Some(2), "{}", text(&run.stderr));
    assert!(text(&run.stderr).contains("not set up"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_CMD_51_the_program_runs_the_command() {
    // The program has no organisation's file here: it is a fault, and the fixed path is not changed by the test.
    let bench = bench();

    let run = wb(&bench.repo.path, &["doctor"], "");

    assert!(matches!(code(&run), Some(0) | Some(1)), "{}", text(&run.stderr));
    assert!(text(&run.stdout).lines().any(|line| line.contains(": bubblewrap: ")), "{}", text(&run.stdout));
}

#[test]
fn test_WB_CMD_52_bubblewrap_that_is_not_there_is_a_fault_that_names_the_package() {
    let bench = bench();

    let run = bench.doctor_with(&[], &bench.tools.path, &bench.organisation(), &bench.tools.path.join("no-bwrap"));

    assert_eq!(code(&run), Some(1));
    let line = line_of(&run, "bubblewrap");
    assert!(line.starts_with("fault: ") && line.contains("bubblewrap") && line.contains("Install"), "{}", line);
}

#[test]
fn test_WB_CMD_52_user_namespaces_that_are_off_are_a_fault_with_what_bubblewrap_said() {
    let bench = bench();
    script(&bench.tools.path.join("bwrap-off"), "echo 'bwrap: No permissions to create new namespace' >&2\nexit 1\n");

    let run = bench.doctor_with(&[], &bench.tools.path, &bench.organisation(), &bench.tools.path.join("bwrap-off"));

    assert_eq!(code(&run), Some(1));
    assert_eq!(status_of(&line_of(&run, "bubblewrap")), "ok");
    let line = line_of(&run, "user namespaces");
    assert!(line.starts_with("fault: ") && line.contains("No permissions to create new namespace"), "{}", line);
}

#[test]
fn test_WB_CMD_52_bubblewrap_that_works_is_two_ok_lines() {
    let bench = bench();

    let run = bench.doctor();

    assert_eq!(status_of(&line_of(&run, "bubblewrap")), "ok");
    assert_eq!(status_of(&line_of(&run, "user namespaces")), "ok");
}

#[test]
fn test_WB_CMD_53_git_off_the_path_is_a_fault() {
    let bench = bench();
    fs::remove_file(bench.tools.path.join("git")).unwrap();

    let run = bench.doctor();

    assert_eq!(code(&run), Some(1));
    let line = line_of(&run, "git");
    assert!(line.starts_with("fault: ") && line.contains("PATH"), "{}", line);
}

#[test]
fn test_WB_CMD_53_a_tool_a_pattern_calls_that_is_not_on_the_path_is_a_fault_naming_the_keys() {
    let bench = bench();
    bench.add("issue_comment: 'gh issue comment {issue} --body-file -'\nissue_comments: 'gh api x | jq -r .a'\nchange_comment: 'nowhere-wb-tool {change}'\nchange_text: 'nowhere-wb-tool {change} | jq .b'\n");

    let run = bench.doctor();

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    let lines = lines_of(&run, "tools");
    let gh = lines.iter().find(|line| line.contains(": gh ")).unwrap_or_else(|| panic!("{:?}", lines));
    assert!(gh.starts_with("ok: ") && gh.contains("issue_comment") && gh.contains("issue_comments"), "{}", gh);
    let jq = lines.iter().find(|line| line.contains(": jq ")).unwrap_or_else(|| panic!("{:?}", lines));
    assert!(jq.starts_with("ok: ") && jq.contains("issue_comments") && jq.contains("change_text"), "{}", jq);
    let lost: Vec<&String> = lines.iter().filter(|line| line.contains("nowhere-wb-tool")).collect();
    assert_eq!(lost.len(), 1, "{:?}", lines);
    assert!(lost[0].starts_with("fault: ") && lost[0].contains("change_comment") && lost[0].contains("change_text"), "{}", lost[0]);
}

#[test]
fn test_WB_CMD_53_what_is_in_quotes_or_cannot_be_told_is_not_taken_for_a_tool() {
    let bench = bench();
    bench.add("issue_comments: 'gh api x --jq ''.[].body | split(\"a\") | nowhere-in-quotes'''\nchange_comment: '$TOOL {change}'\nissue_comment: 'gh issue comment {issue} --body \"$(nowhere-in-substitution)\"'\n");

    let run = bench.doctor();

    all_hold(&run);
    let said = lines_of(&run, "tools").join("\n");
    assert!(!said.contains("nowhere") && !said.contains("split") && !said.contains("$TOOL"), "{}", said);
}

#[test]
fn test_WB_CMD_53_an_option_after_command_and_a_loop_variable_are_not_taken_for_a_tool() {
    let bench = bench();
    bench.add("issue_comments: 'command -v jq >/dev/null; time -p gh; for f in a b; do cat $f; done; select s in a; do gh; done; case $x in a) gh;; esac'\n");

    let run = bench.doctor();

    let said = lines_of(&run, "tools").join("\n");
    for word in [": -v", ": -p", ": f ", ": s ", ": in "] {
        assert!(!said.contains(word), "{} in {}", word, said);
    }
}

#[test]
fn test_WB_CMD_53_no_pattern_is_an_ok_line() {
    let bench = bench();

    let run = bench.doctor();

    let line = line_of(&run, "tools");
    assert!(line.starts_with("ok: ") && line.contains("no pattern"), "{}", line);
}

#[test]
fn test_WB_CMD_54_no_organisation_file_is_a_fault_and_a_file_that_does_not_read_is_a_fault() {
    let bench = bench();
    fs::remove_file(bench.organisation()).unwrap();
    let run = bench.doctor();
    assert_eq!(code(&run), Some(1));
    let line = line_of(&run, "organisation");
    assert!(line.starts_with("fault: ") && line.contains("organisation.yml"), "{}", line);

    fs::write(bench.organisation(), "levels: [public]\nrunners: nonsense\n").unwrap();
    let run = bench.doctor();
    assert_eq!(code(&run), Some(1));
    let line = line_of(&run, "organisation");
    assert!(line.starts_with("fault: ") && line.contains("runners"), "{}", line);
}

#[test]
fn test_WB_CMD_54_a_file_without_a_default_runner_is_a_warning_and_the_runner_checks_are_not_made() {
    let bench = bench();
    let org = read(&bench.organisation()).replace("default: stand-in\n", "");
    fs::write(bench.organisation(), org).unwrap();

    let run = bench.doctor();

    all_hold(&run);
    let line = line_of(&run, "default runner");
    assert!(line.starts_with("warning: ") && line.contains("--runner") && line.contains("not looked at"), "{}", line);
    for name in ["binds", "hooks"] {
        assert!(lines_of(&run, name).iter().all(|line| line.contains("not looked at")), "{}: {:?}", name, lines_of(&run, name));
    }
}

#[test]
fn test_WB_CMD_54_a_runner_with_no_start_is_a_warning() {
    let bench = bench();
    let org = read(&bench.organisation()).replace("    start: ['session', '{model}', '{effort}', '{prompt}']\n", "");
    fs::write(bench.organisation(), org).unwrap();

    let run = bench.doctor();

    all_hold(&run);
    let line = line_of(&run, "default runner");
    assert!(line.starts_with("warning: ") && line.contains("no `start`"), "{}", line);
}

#[test]
fn test_WB_CMD_54_a_start_program_that_is_not_on_the_path_is_a_fault() {
    let bench = bench();
    fs::remove_file(bench.tools.path.join("session")).unwrap();

    let run = bench.doctor();

    assert_eq!(code(&run), Some(1));
    let line = line_of(&run, "default runner");
    assert!(line.starts_with("fault: ") && line.contains("session") && line.contains("PATH"), "{}", line);
}

#[test]
fn test_WB_CMD_54_a_start_program_outside_what_the_sandbox_holds_is_a_fault() {
    let bench = bench();
    let outside = folder_named("outside");
    script(&outside.path.join("session"), "exit 0\n");
    let org = read(&bench.organisation()).replace("'session'", &format!("'{}/session'", outside.path.display()));
    fs::write(bench.organisation(), org).unwrap();

    let run = bench.doctor();

    assert_eq!(code(&run), Some(1));
    let line = line_of(&run, "default runner");
    assert!(line.starts_with("fault: ") && line.contains("outside what the sandbox holds"), "{}", line);
}

#[test]
fn test_WB_CMD_54_a_good_default_runner_is_ok_and_names_the_runner() {
    let bench = bench();

    let run = bench.doctor();

    let line = line_of(&run, "default runner");
    assert!(line.starts_with("ok: ") && line.contains("stand-in") && line.contains("session"), "{}", line);
    assert_eq!(status_of(&line_of(&run, "organisation")), "ok");
}

#[test]
fn test_WB_CMD_55_a_bind_that_is_not_there_is_a_fault_naming_it() {
    let bench = bench();
    fs::remove_dir(bench.home.path.join(".claude")).unwrap();

    let run = bench.doctor();

    assert_eq!(code(&run), Some(1));
    let line = lines_of(&run, "binds").into_iter().find(|line| line.starts_with("fault: ")).unwrap();
    assert!(line.contains(&bench.home.path.join(".claude").display().to_string()) && line.contains("not there"), "{}", line);
}

#[test]
fn test_WB_CMD_55_a_bind_that_holds_a_login_of_the_code_host_is_a_fault() {
    let bench = bench();
    fs::create_dir_all(bench.home.path.join(".config/gh")).unwrap();
    let org = read(&bench.organisation()).replace("'~/.claude'", "'~/.claude', '~/.config/gh'");
    fs::write(bench.organisation(), org).unwrap();

    let run = bench.doctor();

    assert_eq!(code(&run), Some(1));
    let line = lines_of(&run, "binds").into_iter().find(|line| line.starts_with("fault: ")).unwrap();
    assert!(line.contains(".config/gh") && line.contains("login"), "{}", line);
}

#[test]
fn test_WB_CMD_55_binds_that_are_there_are_ok() {
    let bench = bench();

    let run = bench.doctor();

    let lines = lines_of(&run, "binds");
    assert!(!lines.is_empty() && lines.iter().all(|line| line.starts_with("ok: ")), "{:?}", lines);
}

/// The person's settings of the runner, `~/.claude/settings.json`, with a hook of `command`; the runner's program
/// is made `claude`, for `wb` reads the settings of that runner alone.
fn with_hook(bench: &Bench, command: &str) {
    script(&bench.tools.path.join("claude"), "exit 0\n");
    let org = read(&bench.organisation()).replace("'session'", "'claude'");
    fs::write(bench.organisation(), org).unwrap();
    let json = format!("{{\"hooks\": {{\"Stop\": [{{\"hooks\": [{{\"type\": \"command\", \"command\": \"{}\"}}]}}]}}}}", command);
    fs::write(bench.home.path.join(".claude/settings.json"), json).unwrap();
}

#[test]
fn test_WB_CMD_56_a_backslash_in_a_hook_command_is_shown_escaped_once() {
    let bench = bench();
    with_hook(&bench, "/nowhere/b\\\\c hook");

    let run = bench.doctor();

    let line = lines_of(&run, "hooks").into_iter().find(|line| line.starts_with("fault: ")).unwrap_or_else(|| panic!("{}", text(&run.stdout)));
    assert!(line.contains("b\\\\c") && !line.contains("\\\\\\"), "{}", line);
}

#[test]
fn test_WB_CMD_56_a_hook_whose_program_the_sandbox_does_not_hold_is_a_fault_naming_it() {
    let bench = bench();
    with_hook(&bench, "/nonexistent-wb-test/recorder hook");

    let run = bench.doctor();

    assert_eq!(code(&run), Some(1), "{}", text(&run.stdout));
    let line = lines_of(&run, "hooks").into_iter().find(|line| line.starts_with("fault: ")).unwrap_or_else(|| panic!("{}", text(&run.stdout)));
    assert!(line.contains("/nonexistent-wb-test/recorder hook") && line.contains("Stop") && line.contains("will fail in the view"), "{}", line);
}

#[test]
fn test_WB_CMD_56_a_hook_whose_program_the_sandbox_holds_is_ok() {
    let bench = bench();
    with_hook(&bench, &format!("{}/claude --hook", bench.tools.path.display()));

    let run = bench.doctor();

    all_hold(&run);
    let line = line_of(&run, "hooks");
    assert!(line.starts_with("ok: "), "{}", line);
}

#[test]
fn test_WB_CMD_57_the_needs_command_is_run_once_and_a_good_one_is_ok() {
    let bench = bench();
    let log = bench.tools.path.join("needs.log");
    script(&bench.tools.path.join("needs.sh"), &format!("echo \"$1\" >> {}\nexit 0\n", log.display()));
    bench.add(&format!("needs: '{}/needs.sh {{file}}'\n", bench.tools.path.display()));

    let run = bench.doctor();

    all_hold(&run);
    assert!(line_of(&run, "needs").starts_with("ok: "));
    assert_eq!(read(&log).lines().count(), 1, "{}", read(&log));
}

#[test]
fn test_WB_CMD_57_a_needs_command_that_ends_with_anything_but_0_is_a_warning() {
    let bench = bench();
    script(&bench.tools.path.join("needs.sh"), "exit 3\n");
    bench.add(&format!("needs: '{}/needs.sh {{file}}'\n", bench.tools.path.display()));

    let run = bench.doctor();

    assert_eq!(code(&run), Some(0), "{}", text(&run.stdout));
    let line = line_of(&run, "needs");
    assert!(line.starts_with("warning: ") && line.contains("ended with 3"), "{}", line);
}

#[test]
fn test_WB_CMD_57_no_needs_is_an_ok_line_and_nothing_is_run() {
    let bench = bench();

    let run = bench.doctor();

    let line = line_of(&run, "needs");
    assert!(line.starts_with("ok: ") && line.contains("no `needs`"), "{}", line);
}

#[test]
fn test_WB_CMD_58_each_listening_port_is_a_warning_that_says_it_cannot_tell_whether_it_has_a_login() {
    let bench = bench();
    let tcp = format!("{}{}{}{}", HEADER, socket("0100007F:1F90", "0A"), socket("00000000:0016", "0A"), socket("0100007F:C350", "01"));
    let tcp6 = format!("{}{}{}", HEADER, socket("00000000000000000000000001000000:0050", "0A"), socket("00000000000000000000000000000000:0277", "0A"));
    fs::write(bench.net.path.join("tcp"), tcp).unwrap();
    fs::write(bench.net.path.join("tcp6"), tcp6).unwrap();

    let run = bench.doctor();

    all_hold(&run);
    let lines = lines_of(&run, "ports");
    assert_eq!(lines.len(), 4, "{:?}", lines);
    assert!(lines.iter().all(|line| line.starts_with("warning: ")), "{:?}", lines);
    let find = |address: &str| lines.iter().find(|line| line.contains(&format!(": ports: {} ", address))).unwrap_or_else(|| panic!("{}: {:?}", address, lines)).clone();
    assert!(find("127.0.0.1:8080").contains("loopback"));
    assert!(find("0.0.0.0:22").contains("every address"));
    assert!(find("[::1]:80").contains("loopback"));
    assert!(find("[::]:631").contains("every address"));
    for line in &lines {
        assert!(line.contains("cannot tell") && line.contains("login") && line.contains("a session in a view can reach it") && line.contains("#195"), "{}", line);
    }
    // A socket that is not listening (state 01) is none.
    assert!(!lines.iter().any(|line| line.contains(":50000")), "{:?}", lines);
}

#[test]
fn test_WB_CMD_58_no_listening_port_is_an_ok_line() {
    let bench = bench();

    let run = bench.doctor();

    let line = line_of(&run, "ports");
    assert!(line.starts_with("ok: ") && line.contains("no port"), "{}", line);
}

#[test]
fn test_WB_CMD_58_a_port_that_listens_twice_is_named_once_and_twenty_are_named_at_most() {
    let bench = bench();
    let mut tcp = String::from(HEADER);
    tcp.push_str(&socket("0100007F:03E8", "0A"));
    tcp.push_str(&socket("0100007F:03E8", "0A"));
    for port in 1..=25 {
        tcp.push_str(&socket(&format!("0100007F:{:04X}", 2000 + port), "0A"));
    }
    fs::write(bench.net.path.join("tcp"), tcp).unwrap();

    let run = bench.doctor();

    all_hold(&run);
    let lines = lines_of(&run, "ports");
    assert_eq!(lines.iter().filter(|line| line.contains("127.0.0.1:1000")).count(), 1, "{:?}", lines);
    assert_eq!(lines.iter().filter(|line| line.contains(" listens ")).count(), 20, "{:?}", lines);
    assert!(lines.iter().any(|line| line.contains("and 6 more ports")), "{:?}", lines);
}

#[test]
fn test_WB_CMD_58_a_listing_over_the_cap_is_read_no_further_and_says_so() {
    let bench = bench();
    let mut tcp = String::from(HEADER);
    tcp.push_str(&socket("0100007F:1F90", "0A"));
    let filler = socket("0100007F:C350", "01");
    while tcp.len() < 1024 * 1024 + 1 {
        tcp.push_str(&filler);
    }
    fs::write(bench.net.path.join("tcp"), tcp).unwrap();

    let run = bench.doctor();

    all_hold(&run);
    let lines = lines_of(&run, "ports");
    assert!(lines.iter().any(|line| line.starts_with("warning: ") && line.contains("tcp") && line.contains("over 1 MiB")), "{:?}", lines);
}

#[test]
fn test_WB_CMD_58_a_listing_that_is_not_there_is_said_as_a_warning() {
    let bench = bench();
    fs::remove_file(bench.net.path.join("tcp6")).unwrap();

    let run = bench.doctor();

    all_hold(&run);
    assert!(lines_of(&run, "ports").iter().any(|line| line.starts_with("warning: ") && line.contains("tcp6") && line.contains("not read")));
}

/// Every plain file under a folder, with its bytes, sorted by path.
fn snapshot(folder: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(base: &Path, at: &Path, found: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(base, &path, found);
            } else {
                found.push((path.strip_prefix(base).unwrap().to_str().unwrap().to_string(), fs::read(&path).unwrap()));
            }
        }
    }
    let mut found = Vec::new();
    walk(folder, folder, &mut found);
    found.sort();
    found
}

#[test]
fn test_WB_CMD_59_it_writes_nothing() {
    let bench = bench();
    let before = (snapshot(&bench.repo.path), snapshot(&bench.home.path), snapshot(&bench.org.path), snapshot(&bench.net.path));

    let run = bench.doctor();

    assert!(matches!(code(&run), Some(0) | Some(1)));
    let after = (snapshot(&bench.repo.path), snapshot(&bench.home.path), snapshot(&bench.org.path), snapshot(&bench.net.path));
    assert!(before == after, "doctor wrote a file");
}

#[test]
fn test_WB_CMD_59_it_runs_no_pattern_but_needs_and_reads_the_file_as_head_has_it() {
    let bench = bench();
    let log = bench.tools.path.join("calls.log");
    script(&bench.tools.path.join("post.sh"), &format!("echo post >> {}\nexit 0\n", log.display()));
    bench.add(&format!("issue_comment: '{}/post.sh {{issue}}'\n", bench.tools.path.display()));
    // A pattern changed in the working tree alone is not run, and not looked at.
    let mut yml = read(&config(&bench.repo.path));
    yml.push_str("needs: 'nowhere-wb-tool {file}'\n");
    fs::write(config(&bench.repo.path), yml).unwrap();

    let run = bench.doctor();

    assert!(!log.exists(), "a pattern was run");
    assert!(!text(&run.stdout).contains("nowhere-wb-tool"), "{}", text(&run.stdout));
    assert!(text(&run.stderr).contains("differs from HEAD"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_CMD_59_it_asks_nothing_of_the_network() {
    let source = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/doctor.rs")).unwrap();

    for word in ["TcpStream", "TcpListener", "UdpSocket", "ToSocketAddrs", "Command::new", "std::process", "\"curl\"", "\"gh\"", "\"glab\"", "\"ssh\""] {
        assert!(!source.contains(word), "src/doctor.rs names {}", word);
    }
}

#[test]
fn test_WB_CMD_60_the_usage_and_the_help_name_the_command_and_it_takes_q() {
    let folder = folder();

    let run = wb(&folder.path, &["help", "doctor"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.starts_with("usage:\n  wb doctor [-q | --quiet]\n"), "{}", out);
    assert!(out.contains("ports"), "{}", out);
    let help = text(&wb(&folder.path, &["help"], "").stdout);
    assert!(help.contains("doctor    say what on this machine Watchbill needs\n"), "{}", help);
    assert!(!help.contains("doctor (planned"), "{}", help);
    let usage = text(&wb(&folder.path, &["nonsense"], "").stderr);
    assert!(usage.lines().any(|line| line == "  wb doctor [-q | --quiet]"), "{}", usage);
}

#[test]
fn test_WB_CMD_60_the_page_the_completion_the_readme_and_the_words_name_it() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let page = fs::read_to_string(root.join("packaging/man/wb-doctor.1")).unwrap();
    assert!(page.contains("wb\\-doctor") && page.contains("EXIT STATUS"), "{}", page);
    let wb_1 = fs::read_to_string(root.join("packaging/man/wb.1")).unwrap();
    assert!(wb_1.contains(".BR wb\\-doctor (1)"), "{}", wb_1);
    let spec = fs::read_to_string(root.join("packaging/watchbill.spec")).unwrap();
    assert!(spec.contains("packaging/man/wb-doctor.1"), "the spec does not install the page");
    let completion = fs::read_to_string(root.join("packaging/wb.bash")).unwrap();
    assert!(completion.contains("_wb_commands=\"init check doctor upgrade"), "{}", completion);
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    assert!(readme.contains("`wb doctor`") && !readme.contains("`doctor` (planned"), "README.md");
    let words = fs::read_to_string(root.join("docs/words.md")).unwrap();
    assert!(!words.contains("`doctor` (planned"), "docs/words.md");
}
