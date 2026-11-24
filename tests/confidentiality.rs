//! The organisation clears a runner for the planner up to a level (#93): `wb prompt planner --runner`,
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
    commit_config(&folder.path);
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
        commit_config(&folder.path);
    }
    let place = folder_named("organisation");
    let path = place.path.join("organisation.yml");
    fs::write(&path, organisation).unwrap();
    (folder, place, path)
}

fn planner(folder: &Path, organisation: &Path, extra: &[&str]) -> Output {
    let mut args = vec!["planner", "42"];
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
fn test_WB_CONF_18_a_runner_cleared_for_the_planner_up_to_the_repository_s_level_gets_the_prompt() {
    let (folder, _place, path) = ready();

    let run = planner(&folder.path, &path, &["--runner", "claude-code"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert!(text(&run.stdout).contains("You are the planner of issue 42."), "{}", text(&run.stdout));
}

#[test]
fn test_WB_CONF_20_the_stamp_of_the_planner_names_the_runner_and_the_model_and_effort_of_its_planner() {
    let (folder, _place, path) = ready();

    let run = planner(&folder.path, &path, &["--runner", "claude-code", "--stamp"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(&lines[..4], &["role: planner", "model: claude-opus-5-5", "effort: high", "runner: claude-code"], "{}", out);
}

#[test]
fn test_WB_CONF_22_parts_of_the_planner_add_the_level_in_force_and_the_runner() {
    let (folder, _place, path) = ready();

    let run = planner(&folder.path, &path, &["--runner", "claude-code", "--parts"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines[0].ends_with("  .watchbill/revision/roles/planner.md"), "{}", out);
    assert_eq!(&lines[lines.len() - 2..], &["level: public", "runner: claude-code"], "{}", out);
}

#[test]
fn test_WB_CONF_19_a_repository_above_the_planner_s_up_to_is_refused_with_both_levels_and_the_runner() {
    let (folder, _place, path) = with(Some("internal"), ORGANISATION);

    for extra in [&[][..], &["--stamp"], &["--parts"]] {
        let mut args = vec!["--runner", "claude-code"];
        args.extend(extra);
        let run = planner(&folder.path, &path, &args);
        refused(&run, &["internal", "public", "claude-code"]);
    }
}

#[test]
fn test_WB_CONF_18_a_runner_with_no_planner_is_refused() {
    let file = ORGANISATION.replace("    planner:\n      up_to: public\n      model: claude-opus-5-5\n      effort: high\n", "");
    let (folder, _place, path) = with(Some("public"), &file);

    let run = planner(&folder.path, &path, &["--runner", "claude-code"]);

    refused(&run, &["claude-code", "planner"]);
}

#[test]
fn test_WB_CONF_10_a_runner_the_file_does_not_name_is_refused_with_the_names_it_has() {
    let file = format!(
        "{}  local-llm:\n    tools: this machine\n    model: a local model\n    agreement: none, it stays here\n    up_to: confidential\n",
        ORGANISATION
    );
    let (folder, _place, path) = with(Some("public"), &file);

    let run = planner(&folder.path, &path, &["--runner", "elsewhere"]);

    refused(&run, &["elsewhere", "claude-code", "local-llm"]);
}

#[test]
fn test_WB_CONF_19_no_organisation_s_file_refuses_the_planner_and_names_the_path() {
    let folder = set_up();
    let place = folder_named("organisation");
    let path = place.path.join("not-there").join("organisation.yml");

    let run = planner(&folder.path, &path, &["--runner", "claude-code"]);

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
fn test_WB_CONF_17_the_planner_with_no_runner_is_refused_with_2_and_the_usage() {
    let folder = set_up();

    let run = wb(&folder.path, &["prompt", "planner", "42"], "");

    assert_eq!(code(&run), Some(2));
    assert_eq!(text(&run.stdout), "");
    let err = text(&run.stderr);
    assert!(err.starts_with("wb prompt: ") && err.contains("--runner") && err.ends_with(USAGE), "{}", err);
}

#[test]
fn test_WB_CONF_4_with_no_confidentiality_line_the_most_restrictive_level_applies() {
    let (folder, _place, path) = with(None, ORGANISATION);
    let run = planner(&folder.path, &path, &["--runner", "claude-code"]);
    refused(&run, &["confidential", "public", "claude-code"]);

    let file = ORGANISATION.replace("    up_to: internal\n", "    up_to: confidential\n").replace("      up_to: public\n", "      up_to: confidential\n");
    let (folder, _place, path) = with(None, &file);
    let run = planner(&folder.path, &path, &["--runner", "claude-code"]);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
}

#[test]
fn test_WB_CONF_5_a_repository_s_level_not_on_the_list_is_refused_by_name() {
    let (folder, _place, path) = with(Some("secret"), ORGANISATION);

    let run = planner(&folder.path, &path, &["--runner", "claude-code"]);

    refused(&run, &["secret", "confidentiality", "public, internal, confidential"]);
}

#[test]
fn test_WB_CONF_6_levels_or_runners_in_the_repository_s_settings_are_refused() {
    for line in ["levels: [public]\n", "runners:\n  mine:\n    up_to: public\n"] {
        let (folder, _place, path) = ready();
        let mut yml = read(&config(&folder.path));
        yml.push_str(line);
        fs::write(config(&folder.path), yml).unwrap();
        commit_config(&folder.path);

        let run = planner(&folder.path, &path, &["--runner", "claude-code"]);

        let key = line.split(':').next().unwrap();
        refused(&run, &["watchbill.yml", key, "organisation"]);
    }
}

/// The planner refused for an organisation's file of the given text, with each of `named` said.
fn file_refused(file: &str, named: &[&str]) {
    let (folder, _place, path) = with(Some("public"), file);
    let run = planner(&folder.path, &path, &["--runner", "claude-code"]);
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
fn test_WB_CONF_15_a_planner_up_to_above_the_runner_s_up_to_is_refused_with_the_place() {
    file_refused(
        &ORGANISATION.replace("      up_to: public\n", "      up_to: confidential\n"),
        &["runners.claude-code.planner.up_to", "confidential", "internal"],
    );
}

#[test]
fn test_WB_CONF_16_a_model_that_is_not_exact_is_refused_with_the_place() {
    for model in ["claude-opus", "claude-opus-latest", "Claude-Opus-5"] {
        file_refused(
            &ORGANISATION.replace("model: claude-opus-5-5", &format!("model: {}", model)),
            &["runners.claude-code.planner.model", model],
        );
    }
}

#[test]
fn test_WB_CONF_16_an_effort_that_is_not_known_is_refused_with_the_place() {
    file_refused(&ORGANISATION.replace("effort: high", "effort: huge"), &["runners.claude-code.planner.effort", "huge"]);
}

#[test]
fn test_WB_CONF_15_a_planner_lacking_a_field_is_refused_with_the_place() {
    for line in ["      up_to: public\n", "      model: claude-opus-5-5\n", "      effort: high\n"] {
        let key = line.trim().split(':').next().unwrap();
        file_refused(&ORGANISATION.replace(line, ""), &["runners.claude-code.planner", key]);
    }
}

#[test]
fn test_WB_CONF_11_a_key_not_known_at_any_depth_is_refused_with_the_place() {
    let top = format!("{}region: eu\n", ORGANISATION);
    file_refused(&top, &["region"]);
    let runner = ORGANISATION.replace("    up_to: internal\n", "    up_to: internal\n    region: eu\n");
    file_refused(&runner, &["runners.claude-code.region"]);
    let deep = ORGANISATION.replace("      effort: high\n", "      effort: high\n      region: eu\n");
    file_refused(&deep, &["runners.claude-code.planner.region"]);
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
        let run = planner(&folder.path, &path, extra);
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
fn test_WB_CONF_18_wb_prompt_planner_writes_nothing() {
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
        planner(&folder.path, &path, extra);
    }

    assert_eq!((listing(&folder.path), listing(&place.path)), before);
}

// ---- WB-CONF-24: a runner's `start`, and the `default` runner (#84) ----

/// `ORGANISATION` with a `start` for `claude-code`, written as `start`, and a line `default`, when given.
fn with_start(start: &str, default: Option<&str>) -> String {
    let file = ORGANISATION.replace("    up_to: internal\n", &format!("    up_to: internal\n    start: {}\n", start));
    match default {
        Some(line) => format!("{}{}\n", file, line),
        None => file,
    }
}

/// Reads an organisation's file of the given text with `watchbill::confidentiality::read`. The path of the
/// file is written `<the file>` in what is refused.
fn read_organisation(file: &str) -> Result<watchbill::confidentiality::Organisation, String> {
    let place = folder_named("organisation");
    let path = place.path.join("organisation.yml");
    fs::write(&path, file).unwrap();
    watchbill::confidentiality::read(&path).map_err(|why| why.replace(path.to_str().unwrap(), "<the file>"))
}

#[test]
fn test_WB_CONF_24_a_start_of_plain_words_and_a_default_that_names_a_runner_are_taken() {
    let found = read_organisation(&with_start(
        "[claude, --model, '{model}', --effort, '{effort}', '{prompt}']",
        Some("default: claude-code"),
    ))
    .unwrap();
    assert_eq!(found.default.as_deref(), Some("claude-code"));
    let words = found.runners[0].start.clone().unwrap();
    assert_eq!(words, ["claude", "--model", "{model}", "--effort", "{effort}", "{prompt}"]);
    // A placeholder may stand inside a word, and a word may hold braces that are no placeholder.
    let found = read_organisation(&with_start("[env, 'X={effort}', run, '--text={prompt}', '{Model}', '${HOME}']", None)).unwrap();
    assert_eq!(found.default, None);
    assert_eq!(found.runners[0].start.as_ref().unwrap()[1], "X={effort}");
    // `start` is not needed.
    assert_eq!(read_organisation(ORGANISATION).unwrap().runners[0].start, None);
}

#[test]
fn test_WB_CONF_24_a_start_that_is_not_a_list_of_plain_words_is_refused_with_the_file_and_the_place() {
    for (start, place) in [
        ("claude", "runners.claude-code.start"),
        ("[]", "runners.claude-code.start"),
        ("[claude, 5]", "runners.claude-code.start.1"),
        ("[claude, '']", "runners.claude-code.start.1"),
        ("[claude, \"a\\tb\"]", "runners.claude-code.start.1"),
        ("[claude, \"a\\nb\"]", "runners.claude-code.start.1"),
        ("[claude, [a]]", "runners.claude-code.start.1"),
        ("{run: claude}", "runners.claude-code.start"),
    ] {
        let why = read_organisation(&with_start(start, None)).expect_err(start);
        assert!(why.starts_with("<the file>"), "{}: {}", start, why);
        assert!(why.contains(place), "{}: {}", start, why);
    }
}

#[test]
fn test_WB_CONF_24_a_placeholder_that_is_not_known_or_one_in_the_first_word_is_refused_with_the_place() {
    for (start, place, named) in [
        ("[claude, '{issue}']", "runners.claude-code.start.1", "{issue}"),
        ("[claude, '--x={runner}']", "runners.claude-code.start.1", "{runner}"),
        ("['{model}', run]", "runners.claude-code.start.0", "{model}"),
        ("['/opt/{effort}/bin/agent', run]", "runners.claude-code.start.0", "{effort}"),
    ] {
        let why = read_organisation(&with_start(start, None)).expect_err(start);
        assert!(why.starts_with("<the file>") && why.contains(place) && why.contains(named), "{}: {}", start, why);
    }
}

#[test]
fn test_WB_CONF_24_a_default_that_names_no_runner_or_is_not_a_name_is_refused_with_the_file_and_the_place() {
    for line in ["default: other", "default: [claude-code]", "default: ''"] {
        let why = read_organisation(&with_start("[claude]", Some(line))).expect_err(line);
        assert!(why.starts_with("<the file>") && why.contains("default"), "{}: {}", line, why);
    }
    let why = read_organisation(&with_start("[claude]", Some("default: other"))).unwrap_err();
    assert!(why.contains("other") && why.contains("claude-code"), "{}", why);
}

#[test]
fn test_WB_BUILD_103_an_organisation_file_kept_as_a_link_is_read() {
    let (_folder, place, path) = ready();
    let link = place.path.join("linked.yml");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    let read = watchbill::confidentiality::read(&link);
    assert!(read.is_ok(), "{:?}", read.err());
}
