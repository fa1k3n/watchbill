//! The organisation clears a runner for the breakdown up to a level (#93): `wb prompt breakdown --runner`,
//! the organisation's file and the repository's level. The organisation's file of a test is given to
//! `watchbill::prompt` in the test process, so that no test reads the real `/etc`.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use watchbill::USAGE;

const REST: [&str; 4] = ["--branch", "issue-42", "--issue_address", "tracker 42"];

/// A repository set up by `wb init --yes`.
fn set_up() -> Folder {
    let folder = repository(None);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    folder
}

/// A repository at `public` and the file of `ORGANISATION`.
fn ready() -> (Folder, Folder, PathBuf) {
    let folder = set_up();
    let (place, path) = cleared(&folder.path);
    (folder, place, path)
}

/// A repository at the given level, or with no line when `None`, and an organisation's file of the given text.
fn with(level: Option<&str>, organisation: &str) -> (Folder, Folder, PathBuf) {
    let folder = set_up();
    if let Some(level) = level {
        let mut yml = read(&config(&folder.path));
        yml.push_str(&format!("confidentiality: {}\n", level));
        fs::write(config(&folder.path), yml).unwrap();
    }
    let place = folder_named("organisation");
    let path = place.path.join("organisation.yml");
    fs::write(&path, organisation).unwrap();
    (folder, place, path)
}

fn breakdown(folder: &Path, organisation: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["breakdown", "42"];
    args.extend(REST);
    args.extend(extra);
    prompt_with(folder, organisation, &args)
}

/// Refused with 1, nothing on standard output, and each of `named` on standard error.
fn refused(run: &Output, named: &[&str]) {
    assert_eq!(code(run), Some(1), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "");
    let err = text(&run.stderr);
    for name in named {
        assert!(err.contains(name), "{} not in: {}", name, err);
    }
}

#[test]
fn test_WB_CONF_18_a_runner_cleared_for_the_breakdown_up_to_the_repository_s_level_gets_the_prompt() {
    let (folder, _place, path) = ready();

    let run = breakdown(&folder.path, &path, &["--runner", "claude-code"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("You are the breakdown of issue 42."), "{}", text(&run.stdout));
}

#[test]
fn test_WB_CONF_20_the_stamp_of_the_breakdown_names_the_runner_and_the_model_and_effort_of_its_breakdown() {
    let (folder, _place, path) = ready();

    let run = breakdown(&folder.path, &path, &["--runner", "claude-code", "--stamp"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(&lines[..4], &["role: breakdown", "model: claude-opus-5-5", "effort: high", "runner: claude-code"], "{}", out);
}

#[test]
fn test_WB_CONF_22_parts_of_the_breakdown_add_the_level_in_force_and_the_runner() {
    let (folder, _place, path) = ready();

    let run = breakdown(&folder.path, &path, &["--runner", "claude-code", "--parts"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines[0].ends_with("  .watchbill/revision/roles/breakdown.md"), "{}", out);
    assert_eq!(&lines[lines.len() - 2..], &["level: public", "runner: claude-code"], "{}", out);
}

#[test]
fn test_WB_CONF_19_a_repository_above_the_breakdown_s_up_to_is_refused_with_both_levels_and_the_runner() {
    let (folder, _place, path) = with(Some("internal"), ORGANISATION);

    for extra in [&[][..], &["--stamp"], &["--parts"]] {
        let mut args = vec!["--runner", "claude-code"];
        args.extend(extra);
        let run = breakdown(&folder.path, &path, &args);
        refused(&run, &["internal", "public", "claude-code"]);
    }
}

#[test]
fn test_WB_CONF_18_a_runner_with_no_breakdown_is_refused() {
    let file = ORGANISATION.replace("    breakdown:\n      up_to: public\n      model: claude-opus-5-5\n      effort: high\n", "");
    let (folder, _place, path) = with(Some("public"), &file);

    let run = breakdown(&folder.path, &path, &["--runner", "claude-code"]);

    refused(&run, &["claude-code", "breakdown"]);
}

#[test]
fn test_WB_CONF_10_a_runner_the_file_does_not_name_is_refused_with_the_names_it_has() {
    let file = format!(
        "{}  local-llm:\n    tools: this machine\n    model: a local model\n    agreement: none, it stays here\n    up_to: confidential\n",
        ORGANISATION
    );
    let (folder, _place, path) = with(Some("public"), &file);

    let run = breakdown(&folder.path, &path, &["--runner", "elsewhere"]);

    refused(&run, &["elsewhere", "claude-code", "local-llm"]);
}

#[test]
fn test_WB_CONF_19_no_organisation_s_file_refuses_the_breakdown_and_names_the_path() {
    let folder = set_up();
    let place = folder_named("organisation");
    let path = place.path.join("not-there").join("organisation.yml");

    let run = breakdown(&folder.path, &path, &["--runner", "claude-code"]);

    refused(&run, &[path.to_str().unwrap()]);
}

#[test]
fn test_WB_CONF_1_the_organisation_s_file_is_a_fixed_path_and_no_environment_variable_changes_it() {
    assert_eq!(watchbill::confidentiality::ORGANISATION_FILE, "/etc/watchbill/organisation.yml");
    let source = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/confidentiality.rs")).unwrap();
    assert!(!source.contains("env::var"));
    assert!(!source.contains("WATCHBILL_ORGANISATION"));
    let lib = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs")).unwrap();
    assert!(!lib.contains("WATCHBILL_ORGANISATION"));
    assert!(lib.contains("confidentiality::ORGANISATION_FILE"));
}

#[test]
fn test_WB_CONF_17_the_breakdown_with_no_runner_is_refused_with_2_and_the_usage() {
    let folder = set_up();

    let run = wb(&folder.path, &["prompt", "breakdown", "42"], "");

    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stdout), "");
    let err = text(&run.stderr);
    assert!(err.starts_with("wb prompt: ") && err.contains("--runner") && err.ends_with(USAGE), "{}", err);
}

#[test]
fn test_WB_CONF_4_with_no_confidentiality_line_the_most_restrictive_level_applies() {
    let (folder, _place, path) = with(None, ORGANISATION);
    let run = breakdown(&folder.path, &path, &["--runner", "claude-code"]);
    refused(&run, &["confidential", "public", "claude-code"]);

    let file = ORGANISATION.replace("    up_to: internal\n", "    up_to: confidential\n").replace("      up_to: public\n", "      up_to: confidential\n");
    let (folder, _place, path) = with(None, &file);
    let run = breakdown(&folder.path, &path, &["--runner", "claude-code"]);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
}

#[test]
fn test_WB_CONF_5_a_repository_s_level_not_on_the_list_is_refused_by_name() {
    let (folder, _place, path) = with(Some("secret"), ORGANISATION);

    let run = breakdown(&folder.path, &path, &["--runner", "claude-code"]);

    refused(&run, &["secret", "confidentiality", "public, internal, confidential"]);
}

#[test]
fn test_WB_CONF_6_levels_or_runners_in_the_repository_s_settings_are_refused() {
    for line in ["levels: [public]\n", "runners:\n  mine:\n    up_to: public\n"] {
        let (folder, _place, path) = ready();
        let mut yml = read(&config(&folder.path));
        yml.push_str(line);
        fs::write(config(&folder.path), yml).unwrap();

        let run = breakdown(&folder.path, &path, &["--runner", "claude-code"]);

        let key = line.split(':').next().unwrap();
        refused(&run, &["watchbill.yml", key, "organisation"]);
    }
}

/// The breakdown refused for an organisation's file of the given text, with each of `named` said.
fn file_refused(file: &str, named: &[&str]) {
    let (folder, _place, path) = with(Some("public"), file);
    let run = breakdown(&folder.path, &path, &["--runner", "claude-code"]);
    let mut all = vec![path.to_str().unwrap()];
    all.extend(named);
    refused(&run, &all);
}

#[test]
fn test_WB_CONF_14_a_level_written_twice_is_refused_with_the_place() {
    file_refused(&ORGANISATION.replace("[public, internal, confidential]", "[public, internal, public]"), &["levels", "public"]);
}

#[test]
fn test_WB_CONF_14_an_empty_list_of_levels_is_refused_with_the_place() {
    file_refused(&ORGANISATION.replace("[public, internal, confidential]", "[]"), &["levels"]);
}

#[test]
fn test_WB_CONF_11_a_runner_with_no_up_to_is_refused_with_the_place() {
    file_refused(&ORGANISATION.replace("    up_to: internal\n", ""), &["runners.claude-code", "up_to"]);
}

#[test]
fn test_WB_CONF_2_a_runner_with_no_tools_model_or_agreement_is_refused_with_the_place() {
    for line in ["    tools: this machine\n", "    model: Anthropic, under the organisation's agreement\n", "    agreement: EU, no retention, no training\n"] {
        let key = line.trim().split(':').next().unwrap();
        file_refused(&ORGANISATION.replace(line, ""), &["runners.claude-code", key]);
    }
}

#[test]
fn test_WB_CONF_11_an_up_to_that_is_not_a_level_is_refused_with_the_place() {
    file_refused(&ORGANISATION.replace("    up_to: internal\n", "    up_to: secret\n"), &["runners.claude-code.up_to", "secret"]);
}

#[test]
fn test_WB_CONF_15_a_breakdown_up_to_above_the_runner_s_up_to_is_refused_with_the_place() {
    file_refused(
        &ORGANISATION.replace("      up_to: public\n", "      up_to: confidential\n"),
        &["runners.claude-code.breakdown.up_to", "confidential", "internal"],
    );
}

#[test]
fn test_WB_CONF_16_a_model_that_is_not_exact_is_refused_with_the_place() {
    for model in ["claude-opus", "claude-opus-latest", "Claude-Opus-5"] {
        file_refused(
            &ORGANISATION.replace("model: claude-opus-5-5", &format!("model: {}", model)),
            &["runners.claude-code.breakdown.model", model],
        );
    }
}

#[test]
fn test_WB_CONF_16_an_effort_that_is_not_known_is_refused_with_the_place() {
    file_refused(&ORGANISATION.replace("effort: high", "effort: huge"), &["runners.claude-code.breakdown.effort", "huge"]);
}

#[test]
fn test_WB_CONF_15_a_breakdown_lacking_a_field_is_refused_with_the_place() {
    for line in ["      up_to: public\n", "      model: claude-opus-5-5\n", "      effort: high\n"] {
        let key = line.trim().split(':').next().unwrap();
        file_refused(&ORGANISATION.replace(line, ""), &["runners.claude-code.breakdown", key]);
    }
}

#[test]
fn test_WB_CONF_11_a_key_not_known_at_any_depth_is_refused_with_the_place() {
    let top = format!("{}region: eu\n", ORGANISATION);
    file_refused(&top, &["region"]);
    let runner = ORGANISATION.replace("    up_to: internal\n", "    up_to: internal\n    region: eu\n");
    file_refused(&runner, &["runners.claude-code.region"]);
    let deep = ORGANISATION.replace("      effort: high\n", "      effort: high\n      region: eu\n");
    file_refused(&deep, &["runners.claude-code.breakdown.region"]);
}

#[test]
fn test_WB_CONF_11_a_file_that_is_not_yaml_of_the_loader_is_refused_with_the_place() {
    // A key written twice is refused by the one loader.
    file_refused(&format!("{}levels: [public]\n", ORGANISATION), &["levels"]);
}

#[test]
fn test_WB_CONF_23_a_runner_that_is_empty_given_twice_or_not_one_line_is_used_wrongly() {
    let (folder, _place, path) = ready();
    for extra in [
        &["--runner", ""][..],
        &["--runner", "claude-code", "--runner", "claude-code"],
        &["--runner", "claude\ncode"],
        &["--runner", " claude-code"],
        &["--runner"],
    ] {
        let run = breakdown(&folder.path, &path, extra);
        assert_eq!(code(&run), Some(2), "{:?}: {}", extra, text(&run.stderr));
        assert_eq!(text(&run.stdout), "");
        assert!(text(&run.stderr).contains("--runner") && text(&run.stderr).ends_with(USAGE), "{}", text(&run.stderr));
    }
}

#[test]
fn test_WB_CONF_21_the_four_other_roles_take_a_runner_without_a_check_and_the_stamp_names_it() {
    let folder = set_up();
    issues_ready(&folder.path, &["54"]);
    let rest = ["--branch", "issue-54", "--issue_address", "tracker 54", "--change", "34", "--round", "1", "--change_address", "server 34"];
    for role in ["tester", "build", "review", "fix"] {
        let mut args = vec!["prompt", role, "54", "--stamp"];
        args.extend(rest);
        let without = wb(&folder.path, &args, "");
        assert_eq!(code(&without), Some(0), "{}: {}", role, text(&without.stderr));
        assert!(text(&without.stdout).contains("\nrunner:\n"), "{}", text(&without.stdout));

        args.extend(["--runner", "not-in-any-file"]);
        let with = wb(&folder.path, &args, "");
        assert_eq!(code(&with), Some(0), "{}: {}", role, text(&with.stderr));
        assert!(text(&with.stdout).contains("\nrunner: not-in-any-file\n"), "{}", text(&with.stdout));

        // Without --stamp the prompt is as it was without --runner.
        let plain: Vec<&str> = args.iter().copied().filter(|word| *word != "--stamp").collect();
        let given = wb(&folder.path, &plain, "");
        let mut bare: Vec<&str> = plain.clone();
        bare.truncate(bare.len() - 2);
        let before = wb(&folder.path, &bare, "");
        assert_eq!(code(&given), Some(0), "{}", text(&given.stderr));
        assert_eq!(given.stdout, before.stdout, "{}", role);
    }
}

#[test]
fn test_WB_CONF_18_wb_prompt_breakdown_writes_nothing() {
    let (folder, place, path) = ready();
    let listing = |root: &Path| -> Vec<(PathBuf, Vec<u8>)> {
        let mut all = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(at) = stack.pop() {
            for entry in fs::read_dir(&at).unwrap() {
                let entry = entry.unwrap().path();
                if entry.is_dir() {
                    stack.push(entry);
                } else {
                    all.push((entry.clone(), fs::read(&entry).unwrap()));
                }
            }
        }
        all.sort();
        all
    };
    let before = (listing(&folder.path), listing(&place.path));

    for extra in [&["--runner", "claude-code"][..], &["--runner", "claude-code", "--stamp"], &["--runner", "elsewhere"]] {
        breakdown(&folder.path, &path, extra);
    }

    assert_eq!((listing(&folder.path), listing(&place.path)), before);
}
