//! The command layout (#171): every command exists, a planned one says its release and does nothing,
//! and `wb help` shows the groups.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;

const GROUPS: [(&str, &[&str]); 5] = [
    ("set up", &["init", "check", "doctor", "upgrade"]),
    ("work", &["plan", "accept", "start", "run"]),
    ("see", &["status", "view", "log", "answer"]),
    ("take back", &["revert", "clean"]),
    ("plumbing", &["prompt", "revision", "compare"]),
];

/// The commands not built yet: the command, the release and the issue. The releases have the names the owner
/// gave them after 0.0.9: 0.10.0, 0.11.0, 0.12.0. `view` (#173), `upgrade` (#174), `compare` (#179), `clean` (#176) and `doctor` (#178) and `log` (#177) are built. The four of
/// 0.11.0 moved to 0.12.0 with the plan of 2026-11-22 (#230).
const PLANNED: [(&str, &str, &str); 1] = [
    ("run", "0.12.0", "#183"),
];

fn built() -> Vec<&'static str> {
    let planned: Vec<&str> = PLANNED.iter().map(|row| row.0).collect();
    GROUPS.iter().flat_map(|group| group.1.iter().copied()).filter(|name| !planned.contains(name)).collect()
}

#[test]
fn test_WB_CMD_2_a_planned_command_ends_with_2_and_writes_nothing() {
    for (name, _, _) in PLANNED {
        let folder = folder();
        for extra in [&[][..], &["--any", "thing"][..]] {
            let mut args = vec![name];
            args.extend_from_slice(extra);
            let run = wb(&folder.path, &args, "");

            assert_eq!(code(&run), Some(2), "{:?}", args);
            assert_eq!(text(&run.stdout), "", "{:?}", args);
            assert_eq!(fs::read_dir(&folder.path).unwrap().count(), 0, "{:?}", args);
            // Nor the state folder.
            assert!(!std::path::Path::new(NO_STATE).exists(), "{:?}", args);
        }
    }
}

#[test]
fn test_WB_CMD_3_a_planned_command_says_its_release_and_its_issue_on_standard_error() {
    for (name, release, issue) in PLANNED {
        let folder = folder();
        let run = wb(&folder.path, &[name], "");

        assert_eq!(text(&run.stderr), format!("wb {} is planned for {} ({})\n", name, release, issue), "{}", name);
    }
}

#[test]
fn test_WB_CMD_1_every_command_of_the_layout_is_known() {
    let folder = folder();
    for (_, names) in GROUPS {
        for name in names {
            let run = wb(&folder.path, &["help", name], "");
            assert_eq!(code(&run), Some(0), "{}: {}", name, text(&run.stderr));
        }
    }
    // A role is not a command, since `wb start <role>` (#175).
    let run = wb(&folder.path, &["tester"], "");
    assert_eq!(code(&run), Some(2));
}

#[test]
fn test_WB_CMD_4_help_holds_each_group_in_order_and_each_command_under_its_group() {
    let folder = folder();
    let run = wb(&folder.path, &["help"], "");
    let help = text(&run.stdout);

    assert_eq!(code(&run), Some(0));
    assert_eq!(text(&run.stderr), "");
    let mut from = 0;
    for (group, names) in GROUPS {
        let heading = format!("\n{}\n", group);
        let at = from + help[from..].find(&heading).unwrap_or_else(|| panic!("{}: {}", group, help));
        let start = at + heading.len();
        let end = help[start..].find("\n\n").map(|n| start + n).unwrap_or(help.len());
        let found: Vec<&str> = help[start..end].lines().map(|line| line.split_whitespace().next().unwrap()).collect();
        assert_eq!(&found, names, "{}", group);
        from = end;
    }
}

#[test]
fn test_WB_CMD_5_help_names_the_release_of_each_planned_command() {
    let folder = folder();
    let help = text(&wb(&folder.path, &["help"], "").stdout);
    let line_of = |name: &str| {
        help.lines().find(|line| line.split_whitespace().next() == Some(name)).unwrap_or_else(|| panic!("{}", name)).to_string()
    };

    for (name, release, issue) in PLANNED {
        let line = line_of(name);
        assert!(line.contains(&format!("planned for {}, {})", release, issue)), "{}", line);
        assert!(!line.contains("(planned for") || !line.contains("))"), "nested parentheses: {}", line);
    }
    for name in built() {
        let line = line_of(name);
        assert!(!line.contains("planned"), "{}", line);
    }
}

#[test]
fn test_WB_CMD_6_help_of_a_built_command_prints_its_usage_line_and_its_help() {
    let folder = folder();
    for name in built() {
        let run = wb(&folder.path, &["help", name], "");
        let help = text(&run.stdout);

        assert_eq!(code(&run), Some(0), "{}", name);
        assert!(help.starts_with("usage:\n"), "{}: {}", name, help);
        assert!(help.contains(&format!("\n  wb {}", name)), "{}: {}", name, help);
        assert!(help.contains(&format!("\n  {} ", name)), "{}: {}", name, help);
        // Not the help of another command.
        for other in built().into_iter().filter(|other| *other != name) {
            assert!(!help.contains(&format!("\n  {}  ", other)), "{} holds {}", name, other);
        }
    }
}

#[test]
fn test_WB_CMD_6_help_of_a_planned_command_prints_its_release() {
    let folder = folder();
    for (name, release, issue) in PLANNED {
        let run = wb(&folder.path, &["help", name], "");

        assert_eq!(code(&run), Some(0), "{}", name);
        assert!(text(&run.stdout).contains(&format!("wb {} is planned for {} ({})", name, release, issue)), "{}", name);
    }
}

#[test]
fn test_WB_CMD_7_help_of_a_command_that_is_not_known_ends_with_2() {
    let folder = folder();
    for word in ["nothing", "Init", "--version"] {
        let run = wb(&folder.path, &["help", word], "");

        assert_eq!(code(&run), Some(2), "{}", word);
        assert_eq!(text(&run.stdout), "", "{}", word);
        assert!(text(&run.stderr).starts_with(&format!("wb help: no command '{}'\n", word)), "{}", text(&run.stderr));
    }
}

#[test]
fn test_WB_CMD_8_dash_dash_help_and_no_command_print_what_help_prints() {
    let folder = folder();
    let help = text(&wb(&folder.path, &["help"], "").stdout);

    for args in [&["--help"][..], &["-h"][..], &[][..]] {
        let run = wb(&folder.path, args, "");
        assert_eq!(text(&run.stdout), help, "{:?}", args);
        assert_eq!(text(&run.stderr), "", "{:?}", args);
        // `wb` with no command ends with 2, as before (#25).
        assert_eq!(code(&run), Some(if args.is_empty() { 2 } else { 0 }), "{:?}", args);
    }
}

#[test]
fn test_WB_CMD_9_the_usage_the_help_and_the_commands_come_from_one_list() {
    // Each built command has its usage line and its paragraph in the usage; no other command has.
    let usage = watchbill::USAGE;
    for name in built() {
        assert!(usage.contains(&format!("\n  wb {}", name)), "{}", name);
        assert!(usage.contains(&format!("\n  {} ", name)), "{}", name);
    }
    for line in usage.lines().filter(|line| line.starts_with("  wb ")) {
        let name = line.split_whitespace().nth(1).unwrap();
        assert!(name.starts_with("--") || built().contains(&name), "{}", line);
    }
    for (name, _, _) in PLANNED {
        assert!(!usage.contains(&format!("\n  wb {}", name)), "{}", name);
    }
    // The list in the library is the layout.
    let layout: Vec<(&str, Vec<&str>)> = watchbill::layout::GROUPS
        .iter()
        .map(|group| (*group, watchbill::layout::COMMANDS.iter().filter(|c| c.group == *group).map(|c| c.name).collect()))
        .collect();
    let expected: Vec<(&str, Vec<&str>)> = GROUPS.iter().map(|group| (group.0, group.1.to_vec())).collect();
    assert_eq!(layout, expected);
}

#[test]
fn test_WB_CMD_9_each_command_of_the_layout_that_is_built_has_an_arm_in_the_dispatch() {
    // A command that leaves the planned list without an arm must be seen here, not run another command.
    let planned: Vec<&str> = PLANNED.iter().map(|row| row.0).collect();
    for command in watchbill::layout::COMMANDS.iter() {
        let built = !watchbill::layout::PLANNED.iter().any(|row| row.0 == command.name);
        assert_eq!(built, watchbill::ARMS.contains(&command.name), "{}", command.name);
        assert_eq!(built, !planned.contains(&command.name), "{}", command.name);
    }
}

#[test]
fn test_WB_CMD_11_help_with_a_word_more_ends_with_2_and_names_it() {
    let folder = folder();
    for args in [&["help", "init", "extra"][..], &["help", "init", "--yes"][..], &["help", "nothing", "extra"][..]] {
        let run = wb(&folder.path, args, "");

        assert_eq!(code(&run), Some(2), "{:?}", args);
        assert_eq!(text(&run.stdout), "", "{:?}", args);
        let named = args[2];
        assert!(text(&run.stderr).contains(named), "{:?}: {}", args, text(&run.stderr));
    }
}

#[test]
fn test_WB_CMD_12_dash_dash_help_reads_the_word_after_it_as_help_does() {
    let folder = folder();
    let of_init = wb(&folder.path, &["help", "init"], "");
    for args in [&["--help", "init"][..], &["-h", "init"][..]] {
        let run = wb(&folder.path, args, "");
        assert_eq!(code(&run), Some(0), "{:?}", args);
        assert_eq!(text(&run.stdout), text(&of_init.stdout), "{:?}", args);
    }
    let run = wb(&folder.path, &["--help", "nothing"], "");
    assert_eq!(code(&run), Some(2));
    assert!(text(&run.stderr).starts_with("wb help: no command 'nothing'\n"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_CMD_13_help_of_help_prints_its_usage() {
    let folder = folder();
    let run = wb(&folder.path, &["help", "help"], "");

    assert_eq!(code(&run), Some(0));
    assert!(text(&run.stdout).starts_with("usage:\n  wb help"), "{}", text(&run.stdout));
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_CMD_10_help_lists_status_in_the_group_see_and_help_status_prints_its_usage() {
    let folder = folder();
    let help = text(&wb(&folder.path, &["help"], "").stdout);
    let see = help.split("\nsee\n").nth(1).unwrap().split("\n\n").next().unwrap();
    assert!(see.lines().any(|line| line.split_whitespace().next() == Some("status")), "{}", see);

    let run = wb(&folder.path, &["help", "status"], "");
    assert_eq!(code(&run), Some(0));
    assert!(text(&run.stdout).contains("wb status [<issue>] [--stage <heading>]"), "{}", text(&run.stdout));
}

#[test]
fn test_WB_CMD_14_the_usage_names_each_built_command_once_at_the_start_of_a_line() {
    let usage = watchbill::USAGE;
    let lines: Vec<&str> = usage.lines().take_while(|line| !line.is_empty()).collect();
    for name in built() {
        let opens = lines.iter().filter(|line| **line == format!("  wb {}", name) || line.starts_with(&format!("  wb {} ", name))).count();
        assert_eq!(opens, 1, "{}: {}", name, usage);
    }
}

#[test]
fn test_WB_CMD_15_the_usage_lists_the_built_commands_in_the_order_of_help() {
    let usage = watchbill::USAGE;
    let listed: Vec<&str> = usage
        .lines()
        .take_while(|line| !line.is_empty())
        .filter(|line| line.starts_with("  wb "))
        .map(|line| line.split_whitespace().nth(1).unwrap())
        .filter(|name| !name.starts_with("--"))
        .collect();
    assert_eq!(listed, built(), "{}", usage);
}
