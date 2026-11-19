//! `wb status` (#139): the issues in the flow, under headings as `git status` lists files under theirs, each
//! issue under the last step it has done. It only reads. It says what is, never what to do.
//!
//! What it reads: the open issues from `open_issues` of `watchbill.yml` (or, without it, the branches and the
//! files), the comments of each issue from `issue_comments`, and the issue's file and its acceptance from the
//! branch of the issue with `git show`, never by switching branches.
//!
//! `issue_comments` prints only the first line of each comment and no author. An `[answer]` therefore cannot be
//! told from a person's: until the program reads who wrote a comment (#83), every `[answer]` line counts as a
//! person's. A line that starts with `[question]` is never an answer. An answer written below the first line of
//! a comment is not seen, and neither is a choice number that is not one of the choices.

use std::io::Write;
use std::path::Path;

use crate::{issue, prompt, starter};

/// The headings of the steps, in the order of the flow.
pub const STEPS: [&str; 7] = ["Not broken down", "Broken down", "Accepted", "Tested", "Built", "Reviewed", "Fixed"];

/// The heading after the steps, that lists the questions no person has answered.
pub const QUESTIONS: &str = "Questions open";

/// The last heading, for an issue that could not be read.
pub const NOT_READ: &str = "Not read";

/// The heading that a word names, matched without regard to case, or `None` when it is no heading.
pub fn heading_of(word: &str) -> Option<&'static str> {
    STEPS.iter().chain([&QUESTIONS, &NOT_READ]).find(|heading| heading.eq_ignore_ascii_case(word)).copied()
}

/// An issue as it was read.
struct Found {
    number: String,
    title: Option<String>,
    step: &'static str,
    questions: Vec<String>,
    /// The lines it was worked out from, for `wb status <issue>`.
    from: Vec<String>,
}

/// A branch or a remote branch: how it is read with `git show`, and the name it has.
struct Branch {
    full: String,
    name: String,
}

fn git(root: &str, args: &[&str]) -> Option<Vec<u8>> {
    let mut all = vec!["-C", root];
    all.extend(args);
    crate::detect::git_output(&all, None).filter(|run| run.status.success()).map(|run| run.stdout)
}

/// The local branches and those of `origin`, local first.
fn branches(root: &str) -> Vec<Branch> {
    let Some(listed) = git(root, &["for-each-ref", "--format=%(refname)", "refs/heads", "refs/remotes/origin"]) else {
        return Vec::new();
    };
    let listed = String::from_utf8_lossy(&listed).to_string();
    let mut found: Vec<Branch> = Vec::new();
    for full in listed.lines() {
        let name = full.strip_prefix("refs/heads/").or_else(|| full.strip_prefix("refs/remotes/origin/"));
        if let Some(name) = name.filter(|name| *name != "HEAD") {
            found.push(Branch { full: full.to_string(), name: name.to_string() });
        }
    }
    found.sort_by_key(|branch| !branch.full.starts_with("refs/heads/"));
    found
}

/// Whether a branch is the issue's: it is the pattern `branch` with the issue filled, or starts with it and a `-`.
fn is_of(pattern: &str, issue: &str, name: &str) -> bool {
    let start = pattern.replace("{issue}", issue);
    pattern.contains("{issue}") && (name == start || name.starts_with(&format!("{}-", start)))
}

/// The issue that a branch is of, by the pattern: what stands where `{issue}` stands, up to the digits' end.
fn issue_of(pattern: &str, name: &str) -> Option<String> {
    let (before, _) = pattern.split_once("{issue}")?;
    let rest = name.strip_prefix(before)?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    let after = &rest[digits.len()..];
    (!digits.is_empty() && (after.is_empty() || after.starts_with('-'))).then_some(digits)
}

/// The names of the questions, in the order they were asked, that no answer closes, from the first lines of the
/// comments. A question is `[question] <issue>.…`; an answer is `[answer] <name> …` and closes the question of
/// that name; `[answer] <issue> suggested` closes every question asked before it.
fn open_questions(issue: &str, lines: &[String]) -> Vec<String> {
    let mine = format!("{}.", issue);
    let mut asked: Vec<(String, bool)> = Vec::new();
    for line in lines {
        if let Some(rest) = line.strip_prefix("[question] ") {
            let name = rest.split_whitespace().next().unwrap_or_default();
            if name.starts_with(&mine) && !asked.iter().any(|(known, _)| known == name) {
                asked.push((name.to_string(), true));
            }
        } else if let Some(rest) = line.strip_prefix("[answer] ") {
            let mut words = rest.split_whitespace();
            match (words.next(), words.next(), words.next()) {
                (Some(name), Some("suggested"), None) if name == issue => asked.iter_mut().for_each(|(_, open)| *open = false),
                (Some(name), _, _) => asked.iter_mut().filter(|(known, _)| known == name).for_each(|(_, open)| *open = false),
                _ => {}
            }
        }
    }
    asked.into_iter().filter(|(_, open)| *open).map(|(name, _)| name).collect()
}

/// The issue's file and whether its acceptance holds, on the first branch of the issue that has them.
fn on_branches(root: &str, number: &str, pattern: &str, all: &[Branch], from: &mut Vec<String>) -> (bool, bool) {
    let file = format!("{}/{}.yml", issue::FOLDER, number);
    let accepted = format!("{}/{}.accepted", issue::FOLDER, number);
    let mine: Vec<&Branch> = all.iter().filter(|branch| is_of(pattern, number, &branch.name)).collect();
    if mine.is_empty() {
        from.push(format!("no branch of the issue, from the pattern `branch`: {}", pattern));
    }
    let mut has_file = false;
    for branch in mine {
        let Some(bytes) = git(root, &["show", &format!("{}:{}", branch.full, file)]) else {
            from.push(format!("{}: no {}", branch.full, file));
            continue;
        };
        has_file = true;
        let hash = crate::manifest::hash_bytes(&bytes);
        let held = git(root, &["show", &format!("{}:{}", branch.full, accepted)])
            .map(|text| String::from_utf8_lossy(&text).lines().find_map(|line| line.strip_prefix("sha256: ").map(str::to_string)) == Some(hash.clone()))
            .unwrap_or(false);
        from.push(if held {
            format!("{}: {} and {}, which holds for it", branch.full, file, accepted)
        } else {
            format!("{}: {}, and {} is not there or does not hold for it", branch.full, file, accepted)
        });
        if held {
            return (true, true);
        }
    }
    (has_file, false)
}

/// Reads one issue, or says why it could not be read.
fn read_issue(root: &str, number: &str, title: Option<String>, comments: Option<&str>, pattern: &str, all: &[Branch]) -> Result<Found, String> {
    let mut from = Vec::new();
    let mut lines = Vec::new();
    if let Some(command) = comments {
        if !prompt::is_plain_name(number) {
            return Err(format!("{} is not a plain name that fills `issue_comments`", issue::shown(number)));
        }
        let printed = starter::run(root, "issue_comments", &command.replace("{issue}", number), None)?;
        lines = String::from_utf8_lossy(&printed).lines().map(|line| line.trim_end_matches('\r').to_string()).collect();
    }
    let stamps = starter::stamps_in(&lines.join("\n"));
    let (file, accepted) = on_branches(root, number, pattern, all, &mut from);
    let has = |role: &str| stamps.iter().any(|(name, _)| name == role);
    let step = match stamps.last().map(|(name, _)| name.as_str()) {
        Some("fix") => "Fixed",
        Some("review") => "Reviewed",
        _ if has("build") => "Built",
        _ if has("tester") => "Tested",
        _ if accepted => "Accepted",
        _ if file => "Broken down",
        _ => "Not broken down",
    };
    for line in &lines {
        if ["[stamp] ", "[question] ", "[answer] "].iter().any(|start| line.starts_with(start)) {
            from.push(issue::shown(line));
        }
    }
    let questions = open_questions(number, &lines);
    Ok(Found { number: number.to_string(), title, step, questions, from })
}

/// The open issues that `open_issues` prints: the number, a tab and the title, to a line.
fn open_issues(root: &str, command: &str) -> Result<Vec<(String, Option<String>)>, String> {
    let printed = starter::run(root, "open_issues", command, None)?;
    Ok(String::from_utf8_lossy(&printed)
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| !line.trim().is_empty())
        .map(|line| match line.split_once('\t') {
            Some((number, title)) => (number.trim().to_string(), Some(title.to_string())),
            None => (line.trim().to_string(), None),
        })
        .collect())
}

/// The issues with no `open_issues`: those with a branch of the pattern, and those with a file under
/// `.watchbill/issues/`.
fn from_the_repository(root: &str, pattern: &str, all: &[Branch]) -> Vec<(String, Option<String>)> {
    let mut numbers: Vec<String> = all.iter().filter_map(|branch| issue_of(pattern, &branch.name)).collect();
    if let Ok(entries) = std::fs::read_dir(Path::new(root).join(issue::FOLDER)) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(stem) = name.strip_suffix(".yml").filter(|stem| issue::is_name(stem)) {
                numbers.push(stem.to_string());
            }
        }
    }
    numbers.into_iter().map(|number| (number, None)).collect()
}

/// Numbers in the order of their value, then the other names in the order of their letters.
fn order(number: &str) -> (u8, u64, String) {
    match number.parse::<u64>() {
        Ok(value) => (0, value, String::new()),
        Err(_) => (1, 0, number.to_string()),
    }
}

/// `wb status [<issue>] [--stage <heading>]` in the repository at `root`: 0 every issue could be read, 1 a
/// pattern failed, 2 the repository is not set up. `stage`, when given, is a heading that `heading_of` knows.
pub fn status(root: &str, only: Option<&str>, stage: Option<&'static str>, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    if !Path::new(root).join(crate::init::CONFIG_FOLDER).join(crate::init::CONFIG_FILE).is_file() {
        let _ = writeln!(err, "wb status: .watchbill/watchbill.yml is not there. Watchbill is not set up in this repository.");
        return 2;
    }
    let pattern = |key: &str| prompt::pattern_of(root, key);
    let (branch, listing, comments) = match (pattern("branch"), pattern("open_issues"), pattern("issue_comments")) {
        (Ok(branch), Ok(listing), Ok(comments)) => (branch.unwrap_or_else(|| "issue-{issue}".to_string()), listing, comments),
        (Err(why), _, _) | (_, Err(why), _) | (_, _, Err(why)) => {
            let _ = writeln!(err, "wb status: {}", why);
            return 2;
        }
    };
    let all = branches(root);
    let mut issues = match &listing {
        Some(command) => match open_issues(root, command) {
            Ok(issues) => issues,
            Err(why) => {
                let _ = writeln!(err, "wb status: {}", why);
                return 1;
            }
        },
        None => from_the_repository(root, &branch, &all),
    };
    if let Some(one) = only {
        issues.retain(|(number, _)| number == one);
        if issues.is_empty() {
            issues.push((one.to_string(), None));
        }
    }
    // A line of `open_issues` whose first field is not a plain name cannot fill a pattern: it is not read.
    let mut failed: Vec<(String, String)> = Vec::new();
    let (plain, bad): (Vec<_>, Vec<_>) = issues.into_iter().partition(|(number, _)| prompt::is_plain_name(number));
    {
        for (number, _) in bad {
            failed.push((issue::shown(&number), "the first field of this line of `open_issues` is not a plain issue name".to_string()));
        }
    }
    let mut issues = plain;
    if comments.is_none() {
        let _ = writeln!(err, "wb status: watchbill.yml has no `issue_comments`, so no stamp, question or answer is read: an issue is at Accepted or below.");
    }
    issues.sort_by_key(|(number, _)| order(number));
    issues.dedup_by(|a, b| a.0 == b.0);

    let mut read: Vec<Found> = Vec::new();
    for (number, title) in issues {
        match read_issue(root, &number, title, comments.as_deref(), &branch, &all) {
            Ok(found) => read.push(found),
            Err(why) => failed.push((issue::shown(&number), issue::shown(&why))),
        }
    }

    let show = |heading: &str| stage.map_or(true, |wanted| wanted == heading);
    let mut sections: Vec<String> = Vec::new();
    let title = |found: &Found| match &found.title {
        Some(title) => format!("  {}  {}\n", issue::shown(&found.number), issue::shown(title)),
        None => format!("  {}\n", issue::shown(&found.number)),
    };
    for heading in STEPS.iter().filter(|heading| show(heading)) {
        let lines: String = read.iter().filter(|found| found.step == *heading).map(title).collect();
        if !lines.is_empty() {
            sections.push(format!("{}:\n{}", heading, lines));
        }
    }
    if show(QUESTIONS) {
        let lines: String =
            read.iter().filter(|found| !found.questions.is_empty()).map(|found| format!("  {}  {}\n", issue::shown(&found.number), found.questions.iter().map(|name| issue::shown(name)).collect::<Vec<_>>().join(", "))).collect();
        if !lines.is_empty() {
            sections.push(format!("{}:\n{}", QUESTIONS, lines));
        }
    }
    if show(NOT_READ) && !failed.is_empty() {
        let lines: String = failed.iter().map(|(number, why)| format!("  {}  {}\n", number, why)).collect();
        sections.push(format!("{}:\n{}", NOT_READ, lines));
    }
    // One issue: the lines it was worked out from, below what was printed of it.
    if let (Some(_), false, Some(found)) = (only, sections.is_empty(), read.first()) {
        let lines: String = found.from.iter().map(|line| format!("  {}\n", line)).collect();
        sections.push(format!("Worked out from:\n{}", lines));
    }
    let _ = out.write_all(sections.join("\n").as_bytes());
    if failed.is_empty() {
        0
    } else {
        1
    }
}
