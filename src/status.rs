//! `wb status` (#139, #155): the issues in the flow, under headings as `git status` lists files under theirs, each
//! issue under the last step it has done. It only reads, and writes nothing but a temporary repository in $TMPDIR, removed after, to read a kept view (#222). It says what is, never what to do.
//!
//! The issues in the flow are those whose branch, by the pattern `branch`, local or at `origin`, holds a commit
//! that the default branch of `origin` does not. They are found with git alone, from the refs as they are:
//! nothing is fetched. An issue that is done is not listed: its branch is merged into the default branch or
//! deleted, or (as the owner squash-merges) its whole change is one commit on the default branch, by
//! `git patch-id --stable`, or its upstream at `origin` is gone. `wb status <issue>` reads it all the same, and
//! says that it is not in the flow.
//!
//! What it reads besides: the comments of each issue from `issue_comments`, the open change of its branch from
//! `change_of_branch`, the title of one issue from `issue_address`, and the issue's file and its acceptance from
//! the branch of the issue with `git show`, never by switching branches.
//!
//! `issue_comments` prints only the first line of each comment and no author. An answer therefore cannot be told
//! from a person's: until the program reads who wrote a comment (#83), every `[answers]` and `[answer]` line counts
//! as a person's. A line that starts with `[question]` or `[questions]` is never an answer. The questions and the
//! answers are read from first lines alone (`answer::open_names`, #145): `[questions] <issue>.<role> q1 q2` names
//! every question of its comment, as `[answers]` names every answer. A choice number that is not one of the
//! choices is not seen.

use std::io::Write;

use crate::{answer, issue, prompt, starter};

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
    /// The branch of the issue in the flow that is shown on its line, or `None` when it is not in the flow.
    branch: Option<String>,
    /// The open changes of that branch.
    changes: Vec<String>,
    /// How many commits of the local branch the branch at `origin` does not hold.
    unpushed: u64,
    /// The title, for `wb status <issue>` alone.
    title: Option<String>,
    step: &'static str,
    questions: Vec<String>,
    /// The role and the question the newest answer that names a role waits for (#226).
    waits: Option<(String, String)>,
    /// The lines it was worked out from, for `wb status <issue>`.
    from: Vec<String>,
}

/// A branch or a remote branch: how it is read with `git show`, and the name it has.
struct Branch {
    full: String,
    name: String,
}

pub(crate) fn git(root: &str, args: &[&str]) -> Option<Vec<u8>> {
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

/// The ref of the default branch of `origin`: `refs/remotes/origin/HEAD`, else `origin/main`, `main`,
/// `origin/master`, `master`. `None` when none is there.
pub(crate) fn default_branch(root: &str) -> Option<String> {
    let exists = |full: &str| git(root, &["rev-parse", "--verify", "--quiet", &format!("{}^{{commit}}", full)]).is_some();
    if let Some(target) = git(root, &["symbolic-ref", "--quiet", "refs/remotes/origin/HEAD"]) {
        let target = String::from_utf8_lossy(&target).trim().to_string();
        if exists(&target) {
            return Some(target);
        }
    }
    ["refs/remotes/origin/main", "refs/heads/main", "refs/remotes/origin/master", "refs/heads/master"].iter().find(|full| exists(full)).map(|full| full.to_string())
}

/// The patch ids (`git patch-id --stable`) of a diff or of the output of `git log -p`, one for each commit.
fn patch_ids(root: &str, input: Vec<u8>) -> Vec<String> {
    use std::process::{Command, Stdio};
    let Ok(mut child) = Command::new("git").args(["-C", root, "patch-id", "--stable"]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() else {
        return Vec::new();
    };
    let Some(mut stdin) = child.stdin.take() else {
        return Vec::new();
    };
    // The input is written from a thread, so that a long one cannot wait for the output to be read.
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&input);
    });
    let output = child.wait_with_output();
    let _ = writer.join();
    match output {
        Ok(run) => String::from_utf8_lossy(&run.stdout).lines().filter_map(|line| line.split_whitespace().next()).map(str::to_string).collect(),
        Err(_) => Vec::new(),
    }
}

/// Whether the whole change of the branch is a commit on the default branch: the patch id of the diff from their
/// merge base to the branch is that of a commit between the merge base and the default branch. This finds a
/// squash merge, whose commit is never an ancestor of the branch. A squash whose diff differs from the
/// branch's is not found.
fn squashed(root: &str, default: &str, branch: &str) -> bool {
    let Some(base) = git(root, &["merge-base", default, branch]) else {
        return false;
    };
    let base = String::from_utf8_lossy(&base).trim().to_string();
    let Some(diff) = git(root, &["diff", "--no-ext-diff", &base, branch]).filter(|diff| !diff.is_empty()) else {
        return false;
    };
    let Some(mine) = patch_ids(root, diff).into_iter().next() else {
        return false;
    };
    let Some(log) = git(root, &["log", "-p", "--no-ext-diff", &format!("{}..{}", base, default)]) else {
        return false;
    };
    patch_ids(root, log).contains(&mine)
}

/// The local branches whose upstream at `origin` is gone, as after `git fetch --prune`.
fn upstream_gone(root: &str) -> Option<Vec<String>> {
    let listed = git(root, &["for-each-ref", "--format=%(refname) %(upstream:track)", "refs/heads"])?;
    Some(String::from_utf8_lossy(&listed).lines().filter_map(|line| line.strip_suffix(" [gone]")).map(str::to_string).collect())
}

/// The refs of the branches in the flow: those whose tip holds a commit that the default branch does not, less
/// the branches of an issue that are done (their upstream is gone, or their whole change is on the default
/// branch). `Err` when git could not list the refs.
fn in_flow(root: &str, default: &str, pattern: &str) -> Result<Vec<String>, String> {
    let fail = || "git could not list the branches, so no issue is read".to_string();
    let listed = git(root, &["for-each-ref", "--no-merged", default, "--format=%(refname)", "refs/heads", "refs/remotes/origin"]).ok_or_else(fail)?;
    let gone = upstream_gone(root).ok_or_else(fail)?;
    let mut refs: Vec<String> = Vec::new();
    for full in String::from_utf8_lossy(&listed).lines() {
        let name = full.strip_prefix("refs/heads/").or_else(|| full.strip_prefix("refs/remotes/origin/")).unwrap_or(full);
        if issue_of(pattern, name).is_some() && (gone.iter().any(|branch| branch == full) || squashed(root, default, full)) {
            continue;
        }
        refs.push(full.to_string());
    }
    Ok(refs)
}

/// How many commits `local` holds that `other` does not.
fn ahead(root: &str, local: &str, other: &str) -> u64 {
    git(root, &["rev-list", "--count", local, &format!("^{}", other)]).and_then(|count| String::from_utf8_lossy(&count).trim().parse().ok()).unwrap_or(0)
}

/// Whether a branch is the issue's: it is the pattern `branch` with the issue filled, or starts with it and a `-`.
pub(crate) fn is_of(pattern: &str, issue: &str, name: &str) -> bool {
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
fn read_issue(root: &str, number: &str, patterns: &Patterns, all: &[Branch], flow: &[String], default: &str, with_title: bool) -> Result<Found, String> {
    let (comments, pattern) = (patterns.comments.as_deref(), patterns.branch.as_str());
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
    // What a `[revert]` marker took back (#142) counts for no step and no question.
    let counted = starter::counted(&lines.join("\n"));
    let (file, accepted) = on_branches(root, number, pattern, all, &mut from);
    // The branch that is shown: the first of the issue's branches in the flow, a local one before one at `origin`.
    let shown = all.iter().find(|branch| is_of(pattern, number, &branch.name) && flow.contains(&branch.full));
    let (mut changes, mut unpushed) = (Vec::new(), 0);
    let mut not_in_flow = None;
    match shown {
        None => not_in_flow = Some(format!("not in the flow: no branch of the issue holds a commit that {} does not", default)),
        Some(branch) => {
            if branch.full.starts_with("refs/heads/") {
                let at_origin = format!("refs/remotes/origin/{}", branch.name);
                let against = if all.iter().any(|other| other.full == at_origin) { at_origin } else { default.to_string() };
                unpushed = ahead(root, &branch.full, &against);
            }
            if let Some(command) = &patterns.change_of_branch {
                if !prompt::is_plain_name(&branch.name) {
                    return Err(format!("the branch {} is not a plain name that fills `change_of_branch`", issue::shown(&branch.name)));
                }
                changes = starter::open_changes(root, &command.replace("{branch}", &branch.name))?;
            }
        }
    }
    if let Some(line) = not_in_flow {
        from.insert(0, line);
    }
    let title = match (&patterns.issue_address, with_title) {
        (Some(command), true) => {
            let printed = starter::run(root, "issue_address", &command.replace("{issue}", number), None)?;
            String::from_utf8_lossy(&printed).lines().next().map(|line| line.trim_end_matches('\r').to_string())
        }
        _ => None,
    };
    // A new breakdown takes the issue back: only the stamps after the last `[stamp] breakdown` decide the step.
    let since = stamps.iter().rposition(|(name, _)| name == "breakdown").map_or(0, |at| at + 1);
    let stamps = &stamps[since..];
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
    let gone = starter::taken_back(&lines.join("\n"));
    for (line, gone) in lines.iter().zip(gone) {
        if ["[stamp] ", "[question] ", "[questions] ", "[answer] ", "[answers] ", "[revert] "].iter().any(|start| line.starts_with(start)) {
            from.push(if gone { format!("{}  (taken back by a [revert] marker)", issue::shown(line)) } else { issue::shown(line) });
        }
    }
    let questions = answer::open_names(number, &counted);
    let waits = answer::waits_for(number, &counted);
    Ok(Found { number: number.to_string(), branch: shown.map(|branch| branch.name.clone()), changes, unpushed, title, step, questions, waits, from })
}

/// The patterns of `watchbill.yml` that `wb status` reads.
struct Patterns {
    branch: String,
    comments: Option<String>,
    change_of_branch: Option<String>,
    issue_address: Option<String>,
}

/// Numbers in the order of their value, then the other names in the order of their letters.
fn order(number: &str) -> (u8, u64, String) {
    match number.parse::<u64>() {
        Ok(value) => (0, value, String::new()),
        Err(_) => (1, 0, number.to_string()),
    }
}
/// The line of an issue: its number, its branch, its open changes as `#<n>`, how many commits are not pushed and,
/// for one issue, its title. Every part is written with its control characters escaped.
fn line_of(found: &Found) -> String {
    let mut parts = vec![issue::shown(&found.number)];
    if let Some(branch) = &found.branch {
        parts.push(issue::shown(branch));
    }
    parts.extend(found.changes.iter().map(|change| format!("#{}", issue::shown(change))));
    if found.unpushed > 0 {
        parts.push(format!("{} not pushed", found.unpushed));
    }
    if let Some(title) = &found.title {
        parts.push(issue::shown(title));
    }
    format!("  {}\n", parts.join("  "))
}

/// `wb status [<issue>] [--stage <heading>]` in the repository at `root`: 0 every issue could be read, 1 a
/// pattern failed, 2 the repository is not set up or has no default branch. `stage`, when given, is a heading
/// that `heading_of` knows.
pub fn status(root: &str, only: Option<&str>, stage: Option<&'static str>, state: Option<&std::path::Path>, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    // `wb status` only reads (#254): a file that is not as `HEAD` has it is said, and every pattern is read from
    // the file of `HEAD`. Where the file is not in `HEAD` at all, the repository is not set up.
    let _read_only = match starter::read_only(root) {
        // Not set up is 2, and no refusal; a file of `HEAD` over its bound is a limit (#261).
        Err(not_read) if not_read.status == 2 => {
            let _ = writeln!(err, "wb status: {}.", not_read.why);
            return not_read.status;
        }
        Err(not_read) => return crate::refusal::limit(err, "wb status", format_args!("{}.", not_read.why)),
        Ok((guard, differs)) => {
            if differs {
                let _ = writeln!(err, "{}", starter::drift_line("wb status"));
            }
            guard
        }
    };
    let pattern = |key: &str| prompt::pattern_of(root, key);
    // `issue_address` is read for one issue only: the list never runs it.
    let address = if only.is_some() { pattern("issue_address") } else { Ok(None) };
    let patterns = match (pattern("branch"), pattern("issue_comments"), pattern("change_of_branch"), address) {
        (Ok(branch), Ok(comments), Ok(change_of_branch), Ok(issue_address)) => {
            Patterns { branch: branch.unwrap_or_else(|| "issue-{issue}".to_string()), comments, change_of_branch, issue_address }
        }
        (Err(why), ..) | (_, Err(why), ..) | (_, _, Err(why), _) | (.., Err(why)) => {
            let _ = writeln!(err, "wb status: {}", why);
            return 2;
        }
    };
    let Some(default) = default_branch(root) else {
        let _ = writeln!(err, "wb status: no default branch was found: not refs/remotes/origin/HEAD, nor main nor master, at origin or here. The issues in the flow are those whose branch holds a commit that it does not.");
        return 2;
    };
    let all = branches(root);
    let flow = match in_flow(root, &default, &patterns.branch) {
        Ok(flow) => flow,
        Err(why) => {
            let _ = writeln!(err, "wb status: {}", why);
            return 2;
        }
    };
    let mut issues: Vec<String> = match only {
        Some(one) => vec![one.to_string()],
        None => flow.iter().filter_map(|full| full.strip_prefix("refs/heads/").or_else(|| full.strip_prefix("refs/remotes/origin/"))).filter_map(|name| issue_of(&patterns.branch, name)).collect(),
    };
    if patterns.comments.is_none() {
        let _ = writeln!(err, "wb status: watchbill.yml has no `issue_comments`, so no stamp, question or answer is read: an issue is at Accepted or below.");
    }
    issues.sort_by_key(|number| order(number));
    issues.dedup();

    let mut failed: Vec<(String, String)> = Vec::new();
    let mut read: Vec<Found> = Vec::new();
    for number in issues {
        match read_issue(root, &number, &patterns, &all, &flow, &default, only.is_some()) {
            Ok(found) => read.push(found),
            Err(why) => failed.push((issue::shown(&number), issue::shown(&why))),
        }
    }

    let show = |heading: &str| stage.map_or(true, |wanted| wanted == heading);
    let mut sections: Vec<String> = Vec::new();
    for heading in STEPS.iter().filter(|heading| show(heading)) {
        let lines: String = read.iter().filter(|found| found.step == *heading).map(line_of).collect();
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
    // One issue: who the newest answer waits for (#226). It is a fact about the issue, not a command.
    if let (Some(_), None, Some(Found { waits: Some((role, name)), .. })) = (only, stage, read.first()) {
        sections.push(format!("waits for: {} ({})\n", issue::shown(role), issue::shown(name)));
    }
    // One issue: the lines it was worked out from, below what was printed of it.
    if let (Some(_), false, Some(found)) = (only, sections.is_empty(), read.first()) {
        let lines: String = found.from.iter().map(|line| format!("  {}\n", line)).collect();
        sections.push(format!("Worked out from:\n{}", lines));
    }
    // What the sessions of this machine left (#222): the views kept with work not taken back, and the sessions cut
    // off, of one issue below what was printed of it; for the list, only the count of the views, on standard error.
    // Only the state folder of this machine and the objects of a view are read, and the view's settings never.
    if let Some(state) = state {
        match (only, stage) {
            (Some(number), None) => {
                let mut lines: Vec<String> = crate::kept::scan(root, state, number).iter().map(|kept| kept.line()).collect();
                lines.extend(crate::kept::cut_off(state, number).iter().map(|cut| cut.line()));
                if !lines.is_empty() {
                    sections.push(lines.iter().map(|line| format!("{}\n", line)).collect());
                }
            }
            (None, _) => {
                let count = crate::kept::scan_all(root, state).len();
                if count > 0 {
                    let _ = writeln!(
                        err,
                        "wb status: {} kept {} with work not taken back on this machine. `wb status <issue>` names them.",
                        count,
                        if count == 1 { "view" } else { "views" }
                    );
                }
            }
            _ => {}
        }
    }
    let _ = out.write_all(sections.join("\n").as_bytes());
    if failed.is_empty() {
        0
    } else {
        // not a refusal: the issue is listed under Not read, and the rest is printed.
        1
    }
}
