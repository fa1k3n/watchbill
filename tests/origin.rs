//! `wb start --list`, which names the roles a session can be started for, and `wb prompt --show-origin`, which
//! prints the whole prompt with a line before each part that says where it comes from (#291). Run as a person
//! runs them, in a repository made for the test.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

mod common;

use common::*;
use std::fs;
use std::path::Path;

const ISSUES: [&str; 2] = ["54", "T-1"];

/// The words that say where the work is: the prompt of a build or a review holds them.
const WHERE: [&str; 6] = ["--checkout", "/work/brakes", "--branch", "issue-1", "--issue_address", "tracker T-1"];

fn set_up() -> Folder {
    let folder = repository(None);
    let run = wb(&folder.path, &["init", "--yes"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    issues_ready(&folder.path, &ISSUES);
    folder
}

fn lines_of(run: &std::process::Output) -> Vec<String> {
    text(&run.stdout).lines().map(String::from).collect()
}

#[test]
fn test_WB_CMD_45_start_list_prints_the_roles_one_to_a_line_and_nothing_else() {
    let folder = set_up();
    let before = fs::read_dir(&folder.path).unwrap().count();

    let run = wb(&folder.path, &["start", "--list"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(lines_of(&run), ["planner", "tester", "build", "review", "fix"]);
    assert_eq!(text(&run.stderr), "");
    assert_eq!(fs::read_dir(&folder.path).unwrap().count(), before);
}

#[test]
fn test_WB_CMD_46_start_list_with_v_or_verbose_names_the_view_or_the_tree_and_whether_a_runner_is_cleared() {
    let folder = set_up();
    let (_place, organisation) = cleared(&folder.path);

    for flag in ["-v", "--verbose"] {
        let run = start_with(&folder.path, &organisation, false, &["start", "--list", flag]);

        assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
        let rows: Vec<Vec<String>> = lines_of(&run).iter().map(|line| line.split_whitespace().map(String::from).collect()).collect();
        let want: [(&str, &str); 5] = [("planner", "tree"), ("tester", "view"), ("build", "view"), ("review", "view"), ("fix", "view")];
        assert_eq!(rows.len(), 5, "{:?}", rows);
        for (row, (role, view)) in rows.iter().zip(want) {
            assert_eq!(row.len(), 3, "{:?}", row);
            assert_eq!((row[0].as_str(), row[1].as_str()), (role, view), "{:?}", row);
        }
        // The runner is cleared for the planner but has no `start`: no role can be started.
        for row in &rows {
            assert_eq!(row[2], "not-cleared", "{:?}", row);
        }
    }
}

/// The words `wb start --list -v` gives each role, as `(role, word)`, with one runner of the organisation's file that
/// has the lines given after its `up_to`, and the repository at `level`.
fn cleared_words(runner_lines: &str, level: &str) -> Vec<(String, String)> {
    let folder = set_up();
    let place = folder_named("organisation");
    let path = place.path.join("organisation.yml");
    let file = format!(
        "levels: [public, internal, confidential]\nrunners:\n  stand-in:\n    tools: this machine\n    model: a stand-in\n    agreement: none\n    up_to: confidential\n{}",
        runner_lines
    );
    fs::write(&path, file).unwrap();
    let mut yml = read(&config(&folder.path));
    yml.push_str(&format!("confidentiality: {}\n", level));
    fs::write(config(&folder.path), yml).unwrap();
    commit_config(&folder.path);
    let run = start_with(&folder.path, &path, false, &["start", "--list", "-v"]);
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    lines_of(&run)
        .iter()
        .map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            (words[0].to_string(), words[2].to_string())
        })
        .collect()
}

const START: &str = "    start: ['/bin/true', '{model}', '{effort}', '{prompt}']\n";
const PLANNER: &str = "    planner:\n      up_to: public\n      model: claude-opus-5-5\n      effort: high\n";

#[test]
fn test_WB_CMD_46_a_runner_with_start_alone_clears_the_four_roles_and_not_the_planner() {
    let found = cleared_words(START, "public");
    let want: Vec<(String, String)> = [("planner", "not-cleared"), ("tester", "cleared"), ("build", "cleared"), ("review", "cleared"), ("fix", "cleared")]
        .iter()
        .map(|(role, word)| (role.to_string(), word.to_string()))
        .collect();
    assert_eq!(found, want);
}

#[test]
fn test_WB_CMD_46_a_runner_with_planner_alone_clears_no_role() {
    for (role, word) in cleared_words(PLANNER, "public") {
        assert_eq!(word, "not-cleared", "{}", role);
    }
}

#[test]
fn test_WB_CMD_46_a_runner_with_start_and_planner_up_to_the_level_clears_every_role() {
    for (role, word) in cleared_words(&format!("{}{}", START, PLANNER), "public") {
        assert_eq!(word, "cleared", "{}", role);
    }
}

#[test]
fn test_WB_CMD_46_a_level_above_the_planners_up_to_leaves_the_planner_not_cleared() {
    let found = cleared_words(&format!("{}{}", START, PLANNER), "internal");
    assert_eq!(found[0], ("planner".to_string(), "not-cleared".to_string()));
    for (role, word) in &found[1..] {
        assert_eq!(word, "cleared", "{}", role);
    }
}

#[test]
fn test_WB_CMD_46_with_no_organisation_file_no_role_is_cleared() {
    let folder = set_up();
    let place = folder_named("nothing");
    let run = start_with(&folder.path, &place.path.join("organisation.yml"), false, &["start", "--list", "-v"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    for line in lines_of(&run) {
        assert!(line.ends_with("not-cleared"), "{}", line);
    }
}

#[test]
fn test_WB_CMD_47_start_list_names_the_roles_of_the_pinned_revision_an_older_one_has() {
    let folder = set_up();
    // A revision that has no fix: its manifest has no line for it and its folder has no file.
    let manifest = folder.path.join(".watchbill/revision.sha256");
    let kept: String = read(&manifest).lines().filter(|line| !line.ends_with("revision/roles/fix.md")).map(|line| format!("{}\n", line)).collect();
    fs::write(&manifest, kept).unwrap();
    fs::remove_file(folder.path.join(".watchbill/revision/roles/fix.md")).unwrap();

    let run = wb(&folder.path, &["start", "--list"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(lines_of(&run), ["planner", "tester", "build", "review"]);
}

#[test]
fn test_WB_CMD_47_start_list_writes_nothing_under_the_repository() {
    let folder = set_up();
    let snapshot = |at: &Path| -> Vec<String> {
        let run = std::process::Command::new("find").arg(at).args(["-type", "f", "-not", "-path", "*/.git/*"]).output().unwrap();
        let mut found: Vec<String> = text(&run.stdout).lines().map(String::from).collect();
        found.sort();
        found
    };
    let before = snapshot(&folder.path);
    let run = wb(&folder.path, &["start", "--list", "-v"], "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    assert_eq!(snapshot(&folder.path), before);
}

#[test]
fn test_WB_CMD_48_start_list_is_used_wrongly_with_a_role_another_word_or_an_option_it_does_not_take() {
    let folder = set_up();
    for (args, said) in [
        (vec!["start", "--list", "build"], "is not taken with --list"),
        (vec!["start", "--list", "--prompt"], "is not taken with --list"),
        (vec!["start", "--list", "--stamp"], "is not taken with --list"),
        (vec!["start", "--list", "-v", "-v"], "is given twice"),
        (vec!["start", "-v"], "taken only with --list"),
        (vec!["start", "--verbose"], "taken only with --list"),
    ] {
        let run = wb(&folder.path, &args, "");
        assert_eq!(code(&run), Some(2), "{:?}: {}", args, text(&run.stderr));
        assert_eq!(text(&run.stdout), "", "{:?}", args);
        let err = text(&run.stderr);
        assert!(err.contains(said) && err.contains("usage:"), "{:?}: {}", args, err);
    }
}

#[test]
fn test_WB_CMD_48_start_list_in_a_folder_that_is_not_set_up_ends_with_2_and_prints_nothing() {
    let folder = repository(None);
    let run = wb(&folder.path, &["start", "--list"], "");
    assert_eq!(code(&run), Some(2), "{}", text(&run.stderr));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("run wb init"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_CMD_49_the_usage_and_the_help_name_list_verbose_and_show_origin() {
    for words in [watchbill::USAGE.to_string(), text(&wb(&folder().path, &["help", "start"], "").stdout)] {
        assert!(words.contains("--list"), "{}", words);
        assert!(words.contains("--verbose"), "{}", words);
    }
    let prompt = text(&wb(&folder().path, &["help", "prompt"], "").stdout);
    assert!(prompt.contains("--show-origin"), "{}", prompt);
    assert!(watchbill::USAGE.contains("--show-origin"));
}

#[test]
fn test_WB_CMD_50_the_man_pages_and_the_completion_name_the_flags() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let start = fs::read_to_string(root.join("packaging/man/wb-start.1")).unwrap();
    let prompt = fs::read_to_string(root.join("packaging/man/wb-prompt.1")).unwrap();
    let completion = fs::read_to_string(root.join("packaging/wb.bash")).unwrap();
    for flag in ["\\-\\-list", "\\-\\-verbose"] {
        assert!(start.lines().any(|line| line.starts_with(".B ") && line.contains(flag)), "{}", flag);
    }
    assert!(prompt.contains(".B \\-\\-show\\-origin"));
    for name in ["_wb_flags_start=", "_wb_flags_prompt="] {
        let line = completion.lines().find(|line| line.starts_with(name)).unwrap();
        assert!(line.contains("--show-origin"), "{}", line);
    }
    let line = completion.lines().find(|line| line.starts_with("_wb_flags_start=")).unwrap();
    assert!(line.contains("--list") && line.contains("--verbose"), "{}", line);
}

/// One origin line of `--show-origin`, in the fields it has, split at two spaces, and the kind without the `# `.
fn origin_of(line: &str) -> Option<Vec<String>> {
    let rest = line.strip_prefix("# ")?;
    let fields: Vec<String> = rest.split("  ").map(String::from).collect();
    let kind = fields[0].as_str();
    let known = ["revision", "project", "issue", "wb"].contains(&kind)
        || (kind.starts_with('{') && kind.ends_with('}') && watchbill::prompt::PLACEHOLDERS.contains(&&kind[1..kind.len() - 1]));
    known.then_some(fields)
}

/// What `--show-origin` printed, taken apart: the origin lines in order, and the text between them joined.
fn taken_apart(output: &str) -> (Vec<Vec<String>>, String) {
    let (mut origins, mut joined) = (Vec::new(), String::new());
    for line in output.split_inclusive('\n') {
        match origin_of(line.trim_end_matches('\n')) {
            Some(fields) => origins.push(fields),
            None => joined.push_str(line),
        }
    }
    (origins, joined)
}

fn prompt_args<'a>(role: &'a str, issue: &'a str, extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec!["prompt", role, issue];
    args.extend(WHERE);
    args.extend(extra);
    args
}

#[test]
fn test_WB_PROMPT_74_each_part_is_preceded_by_an_origin_line_with_the_hash_of_its_part_line() {
    let folder = set_up();
    put_committed(&folder.path, ".watchbill/project/all.md", "Everyone: use make.\n");
    put_committed(&folder.path, ".watchbill/project/build.md", "Builders: run make test.\n");

    let shown = wb(&folder.path, &prompt_args("build", "54", &["--show-origin"]), "");
    let parts = wb(&folder.path, &prompt_args("build", "54", &["--parts"]), "");

    assert_eq!(code(&shown), Some(0), "{}", text(&shown.stderr));
    let (origins, _) = taken_apart(&text(&shown.stdout));
    let kinds: Vec<(&str, &str)> = origins.iter().filter(|fields| !fields[0].starts_with('{') && fields[0] != "wb").map(|fields| (fields[0].as_str(), fields[1].as_str())).collect();
    assert_eq!(
        kinds,
        [
            ("revision", ".watchbill/revision/roles/build.md"),
            ("issue", ".watchbill/issues/54.yml"),
            ("project", ".watchbill/project/all.md"),
            ("project", ".watchbill/project/build.md"),
        ]
    );
    // The last field of each is the hash, the one of the same line of `--parts`.
    let part_lines: Vec<(String, String)> =
        text(&parts.stdout).lines().map(|line| line.split_once("  ").map(|(hash, path)| (hash.to_string(), path.to_string())).unwrap()).collect();
    let parts_seen: Vec<(String, String)> = origins
        .iter()
        .filter(|fields| !fields[0].starts_with('{') && fields[0] != "wb")
        .map(|fields| (fields.last().unwrap().clone(), fields[1].clone()))
        .collect();
    assert_eq!(parts_seen, part_lines);
    // Each part's source: the project's text is as `HEAD` has it.
    assert!(origins.iter().any(|fields| fields[0] == "project" && fields[2] == "(HEAD)"), "{:?}", origins);
}

#[test]
fn test_WB_PROMPT_74_the_hash_of_an_origin_line_is_that_of_the_part_line_of_the_stamp() {
    let folder = set_up();
    put_committed(&folder.path, ".watchbill/project/all.md", "Everyone: use make.\n");
    let shown = wb(&folder.path, &prompt_args("tester", "T-1", &["--show-origin"]), "");
    let stamp = wb(&folder.path, &prompt_args("tester", "T-1", &["--stamp"]), "");

    assert_eq!(code(&shown), Some(0), "{}", text(&shown.stderr));
    let (origins, _) = taken_apart(&text(&shown.stdout));
    let stamped: Vec<String> = text(&stamp.stdout).lines().filter_map(|line| line.strip_prefix("part: ")).map(String::from).collect();
    let shown: Vec<String> =
        origins.iter().filter(|fields| !fields[0].starts_with('{') && fields[0] != "wb").map(|fields| format!("{}  {}", fields.last().unwrap(), fields[1])).collect();
    assert_eq!(shown, stamped);
}

#[test]
fn test_WB_PROMPT_75_the_text_between_the_origin_lines_joined_is_the_prompt_without_the_flag() {
    let folder = set_up();
    put_committed(&folder.path, ".watchbill/project/all.md", "Everyone: use make.\n");
    put_committed(&folder.path, ".watchbill/project/review.md", "Reviewers: read the diff.\n");
    for role in ["tester", "build", "review", "fix"] {
        let shown = wb(&folder.path, &prompt_args(role, "54", &["--show-origin", "--change", "9", "--round", "1"]), "");
        let plain = wb(&folder.path, &prompt_args(role, "54", &["--change", "9", "--round", "1"]), "");

        assert_eq!(code(&shown), Some(0), "{}: {}", role, text(&shown.stderr));
        let (origins, joined) = taken_apart(&text(&shown.stdout));
        assert!(!origins.is_empty(), "{}", role);
        assert_eq!(joined, text(&plain.stdout), "{}", role);
    }
}

#[test]
fn test_WB_PROMPT_75_parts_that_end_with_no_newline_still_join_to_the_prompt() {
    let folder = set_up();
    put_committed(&folder.path, ".watchbill/project/all.md", "Everyone: use make.");
    put_committed(&folder.path, ".watchbill/project/build.md", "Builders: run make test.");

    let shown = wb(&folder.path, &prompt_args("build", "54", &["--show-origin"]), "");
    let plain = wb(&folder.path, &prompt_args("build", "54", &[]), "");

    assert_eq!(code(&shown), Some(0), "{}", text(&shown.stderr));
    let (_, joined) = taken_apart(&text(&shown.stdout));
    assert_eq!(joined, text(&plain.stdout));
    assert!(joined.ends_with("Builders: run make test."));
}

#[test]
fn test_WB_PROMPT_77_for_a_change_with_no_issue_the_issue_and_its_address_say_what_wb_filled_them_with() {
    let folder = set_up();
    let run = wb(&folder.path, &["prompt", "review", "--change", "9", "--round", "1", "--branch", "b", "--checkout", "/w", "--show-origin"], "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains("# {issue}  none: the change has no issue\n"), "{}", out);
    assert!(out.contains("# {change}  given on the command line\n"), "{}", out);
    assert!(!out.contains("the command line\n# {issue}"), "{}", out);
    assert!(!out.contains("# {issue}  the command line"), "{}", out);
}

#[test]
fn test_WB_PROMPT_77_the_branch_of_the_planner_is_from_the_pattern_alone() {
    let folder = set_up();
    let (_place, organisation) = cleared(&folder.path);
    let run = prompt_with(&folder.path, &organisation, &["planner", "54", "--runner", "claude-code", "--checkout", "/w", "--issue_address", "a", "--show-origin"]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    assert!(out.contains("# {branch}  the pattern branch of watchbill.yml (HEAD)\n"), "{}", out);
}

#[test]
fn test_WB_PROMPT_76_with_no_project_text_there_is_no_project_line_and_no_fixed_line() {
    let folder = set_up();

    let run = wb(&folder.path, &prompt_args("build", "54", &["--show-origin"]), "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let (origins, joined) = taken_apart(&text(&run.stdout));
    assert!(origins.iter().all(|fields| fields[0] != "project" && fields[0] != "wb"), "{:?}", origins);
    assert!(!joined.contains(watchbill::prompt::FIXED_LINE));
}

#[test]
fn test_WB_PROMPT_76_the_fixed_line_has_an_origin_line_of_its_own_with_no_hash() {
    let folder = set_up();
    put_committed(&folder.path, ".watchbill/project/all.md", "Everyone: use make.\n");

    let run = wb(&folder.path, &prompt_args("build", "54", &["--show-origin"]), "");

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    let (origins, _) = taken_apart(&out);
    let fixed: Vec<&Vec<String>> = origins.iter().filter(|fields| fields[0] == "wb").collect();
    assert_eq!(fixed.len(), 1);
    assert_eq!(fixed[0].len(), 2, "{:?}", fixed);
    let lines: Vec<&str> = out.lines().collect();
    let at = lines.iter().position(|line| line.starts_with("# wb  ")).unwrap();
    assert_eq!(lines[at + 1], watchbill::prompt::FIXED_LINE);
}

#[test]
fn test_WB_PROMPT_77_a_placeholder_is_marked_after_the_line_of_its_part_and_what_wb_names_is_its_own_kind() {
    let folder = set_up();
    let (_place, organisation) = cleared(&folder.path);
    put_committed(&folder.path, ".watchbill/project/build.md", "Branch is {branch}.\n");

    let run = prompt_with(&folder.path, &organisation, &prompt_args("build", "54", &["--show-origin"])[1..]);

    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    let lines: Vec<&str> = out.lines().collect();
    let first = lines.iter().position(|line| line.starts_with("# revision  ")).unwrap();
    // The marks of the role's text stand right after its line, one for each placeholder it holds, in the order of the list.
    let marks: Vec<&str> = lines[first + 1..].iter().take_while(|line| origin_of(line).is_some()).copied().collect();
    assert!(marks.contains(&"# {issue}  given on the command line"), "{:?}", marks);
    assert!(marks.contains(&"# {brought}  named by wb"), "{:?}", marks);
    assert!(marks.contains(&"# {description}  named by wb"), "{:?}", marks);
    assert!(marks.contains(&"# {branch}  given on the command line"), "{:?}", marks);
    // No placeholder is marked twice for one part.
    let mut seen = marks.clone();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), marks.len());
    // The project's text is marked for its own placeholder.
    let project = lines.iter().position(|line| line.starts_with("# project  ")).unwrap();
    assert_eq!(lines[project + 1], "# {branch}  given on the command line");
}

#[test]
fn test_WB_PROMPT_77_a_value_that_wb_works_out_is_marked_with_where_it_comes_from() {
    let folder = set_up();

    let run = wb(&folder.path, &["prompt", "tester", "54", "--branch", "issue-1"], "");

    // The pattern, the checkout and the classed lists are not given: their marks say where they come from.
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let shown = wb(&folder.path, &["prompt", "tester", "54", "--branch", "issue-1", "--show-origin"], "");
    assert_eq!(code(&shown), Some(0), "{}", text(&shown.stderr));
    let out = text(&shown.stdout);
    assert!(out.contains("# {checkout}  the root of the repository\n"), "{}", out);
    assert!(out.contains("# {requirements}  the issue's file .watchbill/issues/54.yml\n"), "{}", out);
}

#[test]
fn test_WB_PROMPT_78_prompt_without_the_flag_prints_what_it_printed() {
    let folder = set_up();
    let run = wb(&folder.path, &prompt_args("build", "54", &[]), "");
    assert_eq!(code(&run), Some(0), "{}", text(&run.stderr));
    let out = text(&run.stdout);
    for line in out.lines() {
        assert!(origin_of(line).is_none(), "{}", line);
    }
    let role = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("share/watchbill/revision/roles/build.md")).unwrap();
    assert!(out.starts_with(role.lines().next().unwrap()));
    let parts = wb(&folder.path, &prompt_args("build", "54", &["--parts"]), "");
    assert_eq!(text(&parts.stdout).lines().count(), 2);
}

#[test]
fn test_WB_PROMPT_79_show_origin_refuses_as_prompt_does_and_is_used_wrongly_with_parts_or_stamp() {
    let folder = set_up();
    // A changed byte in the role's text: the check does not hold, nothing is printed.
    let role = folder.path.join(".watchbill/revision/roles/build.md");
    let mut changed = fs::read(&role).unwrap();
    changed.push(b'x');
    fs::write(&role, changed).unwrap();
    let run = wb(&folder.path, &prompt_args("build", "54", &["--show-origin"]), "");
    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("limit:"), "{}", text(&run.stderr));

    let folder = set_up();
    for other in ["--parts", "--stamp"] {
        let run = wb(&folder.path, &prompt_args("build", "54", &["--show-origin", other]), "");
        assert_eq!(code(&run), Some(2), "{}", other);
        assert_eq!(text(&run.stdout), "");
    }
    let run = wb(&folder.path, &prompt_args("build", "54", &["--show-origin", "--show-origin"]), "");
    assert_eq!(code(&run), Some(2));
}

#[test]
fn test_WB_PROMPT_79_a_project_file_that_is_not_as_head_has_it_is_refused() {
    let folder = set_up();
    put_committed(&folder.path, ".watchbill/project/all.md", "Everyone: use make.\n");
    put(&folder.path, ".watchbill/project/all.md", "Everyone: use ninja.\n");

    let run = wb(&folder.path, &prompt_args("build", "54", &["--show-origin"]), "");

    assert_eq!(code(&run), Some(1));
    assert_eq!(text(&run.stdout), "");
    assert!(text(&run.stderr).contains("all.md"), "{}", text(&run.stderr));
}

#[test]
fn test_WB_PROMPT_79_the_planner_is_shown_for_a_cleared_runner_and_start_prompt_shows_the_same() {
    let folder = set_up();
    let (_place, organisation) = cleared(&folder.path);
    let shown = prompt_with(&folder.path, &organisation, &["planner", "54", "--runner", "claude-code", "--checkout", "/w", "--branch", "b", "--issue_address", "a", "--show-origin"]);
    assert_eq!(code(&shown), Some(0), "{}", text(&shown.stderr));
    let (origins, joined) = taken_apart(&text(&shown.stdout));
    assert_eq!(origins[0][0], "revision");
    assert_eq!(origins[0][1], ".watchbill/revision/roles/planner.md");
    let plain = prompt_with(&folder.path, &organisation, &["planner", "54", "--runner", "claude-code", "--checkout", "/w", "--branch", "b", "--issue_address", "a"]);
    assert_eq!(joined, text(&plain.stdout));

    // `wb start <role> --show-origin` does what `wb prompt --show-origin` does, and starts nothing.
    let started = start_with(&folder.path, &organisation, false, &["start", "tester", "54", "--checkout", "/w", "--branch", "b", "--issue_address", "a", "--show-origin"]);
    let direct = prompt_with(&folder.path, &organisation, &["tester", "54", "--checkout", "/w", "--branch", "b", "--issue_address", "a", "--show-origin"]);
    assert_eq!(code(&started), Some(0), "{}", text(&started.stderr));
    assert_eq!(text(&started.stdout), text(&direct.stdout));
}

#[test]
fn test_WB_PROMPT_80_the_usage_the_man_page_and_the_completion_of_prompt_name_show_origin() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(watchbill::USAGE.contains("[--show-origin]"));
    let prompt = fs::read_to_string(root.join("packaging/man/wb-prompt.1")).unwrap();
    assert!(prompt.contains("\\-\\-show\\-origin"));
    let completion = fs::read_to_string(root.join("packaging/wb.bash")).unwrap();
    assert!(completion.contains("--show-origin"));
}
