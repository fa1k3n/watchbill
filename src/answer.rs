//! `wb answer` (#145): the open questions of an issue, shown in full in the terminal one at a time, and the
//! answers posted in one comment per role.
//!
//! - **A question** is asked in one comment per session, whose first line is `[questions] <issue>.<role> q1 q2 …`
//!   (`[questions] <issue> …` for the planner, whose questions are `<issue>.q<n>`, #95). Each question is then
//!   a line `q<n>: <what is unclear>`, with the rest below it: why it matters, the choices numbered from 1, a line
//!   `Suggested: …` and a line `changes the issue's file: yes` or `no`. A question of revision 0.0.8 is a comment
//!   of its own whose first line is `[question] <name>`, and is read the same way.
//! - **An answer** is a comment whose first line is `[answers] <issue>.<role> q1 q2 …`, then a line `q<n>: <answer>`
//!   for each. The lines of revision 0.0.8, `[answer] <name> …` and `[answer] <issue> suggested`, are read too.
//! - **The role that acts** (#226): from revision 0.11.0 each choice starts with a role in brackets, `1. (tester) …`,
//!   and the first line of an answer names it after each question: `[answers] 54.build q3>tester q4>person`. An answer
//!   in words, or a choice with no role, asks which role acts (the asking role by Enter); a pair and `--suggested`
//!   take the role of the choice, and the asking role where it has none. The file is said to be changed only for
//!   `person` (to a question of the planner, with the message of the planner); a question whose choices name no role (0.10.0 and before) is answered with no role, and its line
//!   `changes the issue's file:` is read as before. The first lines of 0.10.0, `q3` alone, close and name no role.
//! - **Which count:** `open_names` decides from first lines alone, so that `wb status` and `wb answer` agree. What a
//!   `[revert]` marker took back counts for nothing (`starter::taken_back`).
//!
//! - **On the ref** (#238, #333): where the job's ref holds a stamp or a questions commit, the questions and the answers are the commits
//!   of that ref (`stamps`), and `wb answer` reads them there, with no `issue_comment_bodies`. It writes the answers as
//!   one commit for each role, signed by the person with a key that `.watchbill/signers` holds with the role `answer`,
//!   pushes each, and then posts the same text as a copy through `issue_comment` where it is set up. An `[answers]`
//!   comment at the tracker is not read for such a job, and none is open before its first questions commit. A job whose ref
//!   holds no stamp (begun before), and a change with no issue, are read and answered at the tracker as before.
//!
//! At the tracker, the whole comments are read through `issue_comment_bodies` (or `change_comment_bodies`), each body
//! followed by a NUL byte. Answers are posted through `issue_comment` (or `change_comment`), after `wb` says what it
//! will post. Nothing is posted until every question shown has its answer: an input that ends, or Ctrl-C, posts nothing.

use std::io::{BufRead, Write};

use crate::{issue, prompt, starter};

/// Where the questions are: an issue, or a change with no issue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum On<'a> {
    Issue(&'a str),
    Change(&'a str),
}

/// What `wb answer` was asked to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Asked {
    /// Show each open question and ask for its answer.
    Each,
    /// Print each open question in full, and ask and post nothing.
    Show,
    /// Take the suggestion of every open question but those the pairs (question, answer) answer.
    Suggested(Vec<(String, String)>),
    /// Answer the questions named, each with the text given, and no other.
    Pairs(Vec<(String, String)>),
}

/// A question's suggested answer: one of its choices, or words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Suggestion {
    Choice(u64),
    Words(String),
}

/// A question as it stands on the issue.
#[derive(Clone, Debug)]
pub struct Question {
    /// The part of its name before `q<n>`: `<issue>` for the planner, `<issue>.<role>` for the others.
    pub prefix: String,
    pub number: u64,
    /// What is unclear, from its line `q<n>: …`; empty for a question of revision 0.0.8.
    pub short: String,
    /// The rest of the question, as it is shown.
    pub lines: Vec<String>,
    /// The numbers of its choices.
    pub choices: Vec<u64>,
    pub suggested: Option<Suggestion>,
    /// What its line `changes the issue's file:` says, where it has one (a question of 0.10.0 or before).
    pub changes_file: Option<bool>,
    /// The role in brackets that starts a choice, as `1. (tester) …` (#226), by the number of the choice.
    pub roles: Vec<(u64, String)>,
    /// Whether a question of its comment has a role on a choice: then each of them is asked in the form of 0.11.0.
    pub comment_roled: bool,
}

impl Question {
    /// Its whole name: `175.tester.q1`, `175.q1`.
    pub fn name(&self) -> String {
        format!("{}.q{}", self.prefix, self.number)
    }

    /// Whether it is asked in the form of 0.11.0: a choice of it, or of a question of its comment, names a role.
    pub fn roled(&self) -> bool {
        !self.roles.is_empty() || self.comment_roled
    }

    /// Its role, or `planner` for a name with no role.
    pub fn role(&self) -> String {
        self.prefix.split_once('.').map_or("planner".to_string(), |(_, role)| role.to_string())
    }
}

/// The roles of the revision, and `person` for a person's own step (#226).
const ROLES: [&str; 6] = ["planner", "tester", "build", "review", "fix", "person"];

/// The role in brackets that starts the text of a choice: `(tester) Narrow it.`
fn role_of_choice(text: &str) -> Option<String> {
    let role = text.strip_prefix('(')?.split_once(')')?.0;
    // Only a role of the revision, or `person`, is a role: any other word is a choice with no role.
    ROLES.contains(&role).then(|| role.to_string())
}

/// The number of a word `q<n>`, written with a comma or a full stop after it or without.
fn number_of(word: &str) -> Option<u64> {
    let digits = word.trim_end_matches([',', '.', ';']).strip_prefix('q')?;
    (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())).then(|| digits.parse().ok()).flatten()
}

/// Whether a word is the issue, or the issue and a role, `<issue>.<role>`.
fn is_prefix(issue: &str, word: &str) -> bool {
    match word.strip_prefix(issue) {
        Some("") => true,
        Some(rest) => rest.strip_prefix('.').is_some_and(|role| !role.is_empty() && role.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') && number_of(role).is_none()),
        None => false,
    }
}

/// The prefix and the number of a whole name `<issue>.q<n>` or `<issue>.<role>.q<n>`.
fn split_name(issue: &str, name: &str) -> Option<(String, u64)> {
    let (prefix, last) = name.rsplit_once('.')?;
    let number = number_of(last).filter(|_| !last.ends_with([',', '.', ';']))?;
    is_prefix(issue, prefix).then(|| (prefix.to_string(), number))
}

/// The names a first line `[questions] …` or `[answers] …` gives, after `start`: the prefix and each `q<n>`.
fn names_after(issue: &str, start: &str, line: &str) -> Vec<String> {
    let Some(rest) = line.strip_prefix(start) else { return Vec::new() };
    let mut words = rest.split_whitespace();
    match words.next() {
        Some(prefix) if is_prefix(issue, prefix) => words.filter_map(|word| number_of(word.split('>').next().unwrap_or_default())).map(|n| format!("{}.q{}", prefix, n)).collect(),
        _ => Vec::new(),
    }
}

/// The question and the role that each word `q<n>>role` of a first line `[answers] …` gives: no role for a word
/// of 0.10.0, `q<n>` alone.
fn roles_after(issue: &str, line: &str) -> Vec<(String, Option<String>)> {
    let Some(rest) = line.strip_prefix("[answers] ") else { return Vec::new() };
    let mut words = rest.split_whitespace();
    match words.next() {
        Some(prefix) if is_prefix(issue, prefix) => words
            .filter_map(|word| {
                let (name, role) = word.split_once('>').map_or((word, None), |(name, role)| (name, Some(role)));
                let role = role.filter(|role| ROLES.contains(role));
                number_of(name).map(|n| (format!("{}.q{}", prefix, n), role.map(str::to_string)))
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The role and the question the newest answer names (#226): the last word `q<n>>role` of the last first line
/// `[answers] …` that has one, from the lines that still count. A `[stamp] <role>` after it clears it (the role has
/// acted since); `person` has no stamp.
pub fn waits_for(issue: &str, lines: &[String]) -> Option<(String, String)> {
    let (at, role, name) = lines
        .iter()
        .enumerate()
        .rev()
        .find_map(|(at, line)| roles_after(issue, line).into_iter().rev().find_map(|(name, role)| role.map(|role| (at, role, name))))?;
    let stamped = lines[at + 1..].iter().any(|line| line.strip_prefix("[stamp] ").and_then(|rest| rest.split_whitespace().next()) == Some(role.as_str()));
    (!stamped).then_some((role, name))
}

/// The names of every question asked on `issue`, open or answered, from the first lines of its comments, as
/// `open_names` reads them: `[questions] <prefix> q1 …` and `[question] <name>` (#295).
pub fn asked_names(issue: &str, lines: &[String]) -> Vec<String> {
    let mut asked: Vec<String> = Vec::new();
    for line in lines {
        let found = if line.starts_with("[questions] ") {
            names_after(issue, "[questions] ", line)
        } else if let Some(rest) = line.strip_prefix("[question] ") {
            rest.split_whitespace().next().filter(|name| split_name(issue, name).is_some()).map(str::to_string).into_iter().collect()
        } else {
            Vec::new()
        };
        for name in found {
            if !asked.contains(&name) {
                asked.push(name);
            }
        }
    }
    asked
}

/// The names of the open questions of `issue`, in the order they were asked, from the first lines of its comments
/// that still count, one to a line (`starter::counted`). A question is asked by `[questions] <prefix> q1 …` or
/// `[question] <name>`. It is closed by `[answers] <prefix> q1 …`, by `[answer] <name> …`, and by an
/// `[answer] <issue> suggested` after it. A line that starts with `[question]` or `[questions]` is never an answer.
pub fn open_names(issue: &str, lines: &[String]) -> Vec<String> {
    let mut asked: Vec<(String, bool)> = Vec::new();
    let ask = |asked: &mut Vec<(String, bool)>, name: String| {
        if !asked.iter().any(|(known, _)| *known == name) {
            asked.push((name, true));
        }
    };
    for line in lines {
        if line.starts_with("[questions] ") {
            for name in names_after(issue, "[questions] ", line) {
                ask(&mut asked, name);
            }
        } else if let Some(rest) = line.strip_prefix("[question] ") {
            let name = rest.split_whitespace().next().unwrap_or_default();
            // As `wb status` read it under revision 0.0.8 (#139): any name of the issue.
            if name.starts_with(&format!("{}.", issue)) {
                ask(&mut asked, name.to_string());
            }
        } else if line.starts_with("[answers] ") {
            for name in names_after(issue, "[answers] ", line) {
                asked.iter_mut().filter(|(known, _)| *known == name).for_each(|(_, open)| *open = false);
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

/// The comments that `issue_comment_bodies` printed: each body followed by a NUL byte. One line break straight
/// after a NUL is not part of the next body: `gh --jq` and `jq -r` print one after each result.
pub fn split_bodies(printed: &[u8]) -> Vec<String> {
    let mut found = Vec::new();
    let parts: Vec<&[u8]> = printed.split(|byte| *byte == 0).collect();
    let last = parts.len() - 1;
    for (at, part) in parts.into_iter().enumerate() {
        let part = if at > 0 { part.strip_prefix(b"\r\n").or_else(|| part.strip_prefix(b"\n")).unwrap_or(part) } else { part };
        // What follows the last NUL is a body only when the pattern printed no NUL after it.
        if at == last && part.is_empty() {
            continue;
        }
        found.push(String::from_utf8_lossy(part).to_string());
    }
    found
}

/// The first line of a body, as `issue_comments` prints it.
fn first_line(body: &str) -> String {
    body.split('\n').next().unwrap_or_default().trim_end_matches('\r').to_string()
}

/// The lines of a question as they are shown: the folding of the tracker left out, no blank line at the start or
/// the end, and never two blank lines together.
fn shown_lines(lines: &[&str]) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for line in lines {
        let line = line.trim_end_matches('\r').trim_end();
        let bare = line.trim();
        if bare == "<details>" || bare == "</details>" || (bare.starts_with("<summary") && bare.ends_with("</summary>")) {
            continue;
        }
        if bare.is_empty() && kept.last().map_or(true, |last| last.is_empty()) {
            continue;
        }
        kept.push(if bare.is_empty() { String::new() } else { line.to_string() });
    }
    while kept.last().is_some_and(|last| last.is_empty()) {
        kept.pop();
    }
    kept
}

/// Reads the choices, the suggestion and the line on the issue's file of a question from its lines.
fn read_parts(prefix: &str, number: u64, short: String, lines: Vec<String>) -> Question {
    let mut choices: Vec<u64> = Vec::new();
    let mut suggestion: Option<String> = None;
    let mut changes_file = None;
    let mut roles: Vec<(u64, String)> = Vec::new();
    for line in &lines {
        let bare = line.trim();
        let digits: String = bare.chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty() && bare[digits.len()..].starts_with(". ") {
            if let Ok(n) = digits.parse::<u64>() {
                if !choices.contains(&n) {
                    choices.push(n);
                    if let Some(role) = role_of_choice(bare[digits.len() + 2..].trim_start()) {
                        roles.push((n, role));
                    }
                }
            }
        }
        if suggestion.is_none() {
            if let Some(rest) = bare.strip_prefix("Suggested:") {
                suggestion = Some(rest.trim().to_string());
            }
        }
        let lower = bare.to_lowercase().replace('\u{2019}', "'");
        if let Some(rest) = lower.strip_prefix("changes the issue's file:") {
            changes_file = match rest.trim().trim_end_matches('.') {
                "yes" => Some(true),
                "no" => Some(false),
                _ => changes_file,
            };
        }
    }
    let suggested = suggestion.and_then(|text| {
        let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
        let after = &text[digits.len()..];
        if !digits.is_empty() && (after.is_empty() || after.starts_with([',', ' ', '.', ';'])) {
            let n: u64 = digits.parse().ok()?;
            return choices.contains(&n).then_some(Suggestion::Choice(n));
        }
        let words = text.rsplit_once(", because").map_or(text.as_str(), |(words, _)| words).trim().trim_end_matches('.').trim();
        (!words.is_empty()).then(|| Suggestion::Words(words.to_string()))
    });
    Question { prefix: prefix.to_string(), number, short, lines, choices, suggested, changes_file, roles, comment_roled: false }
}

/// The questions of one comment, in the order its first line names them. A comment of another form, or of
/// another issue, holds none.
fn questions_of(issue: &str, body: &str) -> Vec<Question> {
    let first = first_line(body);
    let lines: Vec<&str> = body.split('\n').skip(1).collect();
    if let Some(rest) = first.strip_prefix("[question] ") {
        let name = rest.split_whitespace().next().unwrap_or_default();
        let Some((prefix, number)) = split_name(issue, name) else { return Vec::new() };
        let shown = shown_lines(&lines);
        return vec![read_parts(&prefix, number, String::new(), shown)];
    }
    if !first.starts_with("[questions] ") {
        return Vec::new();
    }
    let names = names_after(issue, "[questions] ", &first);
    let Some(prefix) = first["[questions] ".len()..].split_whitespace().next().map(str::to_string) else { return Vec::new() };
    let numbers: Vec<u64> = names.iter().filter_map(|name| split_name(issue, name).map(|(_, n)| n)).collect();
    // Each question's lines: from its line `q<n>: …` up to the line of the next question named.
    let starts = |line: &str| -> Option<(u64, String)> {
        let (head, rest) = line.split_once(':')?;
        let n = number_of(head).filter(|n| numbers.contains(n) && head.trim() == head && !head.ends_with([',', '.', ';']))?;
        Some((n, rest.trim().to_string()))
    };
    let mut found: Vec<(u64, String, Vec<&str>)> = Vec::new();
    for line in &lines {
        match starts(line.trim_end_matches('\r')) {
            Some((n, short)) if !found.iter().any(|(known, _, _)| *known == n) => found.push((n, short, Vec::new())),
            _ => {
                if let Some((_, _, kept)) = found.last_mut() {
                    kept.push(line);
                }
            }
        }
    }
    let mut questions = Vec::new();
    for n in numbers {
        match found.iter().find(|(known, _, _)| *known == n) {
            Some((_, short, kept)) => questions.push(read_parts(&prefix, n, short.clone(), shown_lines(kept))),
            None => questions.push(read_parts(&prefix, n, String::new(), Vec::new())),
        }
    }
    let roled = questions.iter().any(|question| !question.roles.is_empty());
    questions.iter_mut().for_each(|question| question.comment_roled = roled);
    questions
}

/// The open questions of `issue` in the comments, in the order they were asked. A comment that a `[revert]`
/// marker took back is not read.
pub fn open_questions(issue: &str, bodies: &[String]) -> Vec<Question> {
    let firsts: Vec<String> = bodies.iter().map(|body| first_line(body)).collect();
    let gone = starter::taken_back(&firsts.join("\n"));
    let kept: Vec<usize> = (0..bodies.len()).filter(|at| !gone.get(*at).copied().unwrap_or(false)).collect();
    let counted: Vec<String> = starter::counted(&kept.iter().map(|at| firsts[*at].clone()).collect::<Vec<_>>().join("\n"));
    let open = open_names(issue, &counted);
    let mut found: Vec<Question> = Vec::new();
    for at in kept {
        for question in questions_of(issue, &bodies[at]) {
            let name = question.name();
            if open.contains(&name) && !found.iter().any(|known| known.name() == name) {
                found.push(question);
            }
        }
    }
    found
}

/// What a person's text gives for a question: the line `q<n>: …` and the role of the choice taken, if it has one,
/// or why it is asked again.
fn take(question: &Question, typed: &str) -> Result<(String, Option<String>), String> {
    let typed = typed.trim();
    let line = |answer: &str| format!("q{}: {}", question.number, answer);
    let role_of = |n: u64| question.roles.iter().find(|(choice, _)| *choice == n).map(|(_, role)| role.clone());
    if typed.is_empty() {
        return match &question.suggested {
            Some(Suggestion::Choice(n)) => Ok((line(&n.to_string()), role_of(*n))),
            Some(Suggestion::Words(words)) => Ok((line(words), None)),
            None => Err(format!("{} has no suggested answer.", question.name())),
        };
    }
    if typed.chars().all(|c| c.is_ascii_digit()) {
        let n: Option<u64> = typed.parse().ok();
        if let Some(n) = n.filter(|n| question.choices.contains(n)) {
            return Ok((line(typed), role_of(n)));
        }
        let choices: Vec<String> = question.choices.iter().map(u64::to_string).collect();
        let which = if choices.is_empty() { "it has none".to_string() } else { choices.join(", ") };
        return Err(format!("{} is not one of the choices of {} ({}).", typed, question.name(), which));
    }
    if typed.chars().any(char::is_control) {
        return Err("An answer is one line, with no control character.".to_string());
    }
    Ok((line(typed), None))
}

/// One line typed in answer to `prompt`, or `None` where the input has ended.
fn ask_line(input: &mut dyn BufRead, out: &mut dyn Write, prompt: &str) -> Option<String> {
    let _ = write!(out, "{}", prompt);
    let _ = out.flush();
    let mut typed = Vec::new();
    match input.read_until(b'\n', &mut typed) {
        Ok(0) | Err(_) => {
            let _ = writeln!(out);
            None
        }
        Ok(_) => Some(String::from_utf8_lossy(&typed).trim_end_matches('\n').trim_end_matches('\r').to_string()),
    }
}

/// Whether an answer says that the issue's file is to be changed: a `person` answer to a question whose choices
/// name roles, and the line of a question of 0.10.0 that says so (#226).
fn changes_file(question: &Question, role: &Option<String>) -> bool {
    if question.roled() {
        role.as_deref() == Some("person")
    } else {
        question.changes_file == Some(true)
    }
}

/// How a question is shown: its name, where it stands among those shown, what is unclear, and the rest indented.
fn show(question: &Question, at: usize, of: usize) -> String {
    let mut text = match question.short.is_empty() {
        true => format!("\n{} ({} of {})\n", question.name(), at, of),
        false => format!("\n{} ({} of {}): {}\n", question.name(), at, of, issue::shown(&question.short)),
    };
    if !question.lines.is_empty() {
        text.push('\n');
    }
    for line in &question.lines {
        if line.is_empty() {
            text.push('\n');
        } else {
            text.push_str(&format!("  {}\n", issue::shown(line)));
        }
    }
    let choices: Vec<String> = question.choices.iter().map(u64::to_string).collect();
    let how = match (&question.suggested, choices.is_empty()) {
        (Some(Suggestion::Choice(n)), _) => format!("Enter takes the suggestion, {}. A number takes that choice ({}). Other text is the answer in words.", n, choices.join(", ")),
        (Some(Suggestion::Words(words)), true) => format!("Enter takes the suggestion, \"{}\". Other text is the answer in words.", issue::shown(words)),
        (Some(Suggestion::Words(words)), false) => {
            format!("Enter takes the suggestion, \"{}\". A number takes that choice ({}). Other text is the answer in words.", issue::shown(words), choices.join(", "))
        }
        (None, false) => format!("There is no suggestion. A number takes that choice ({}). Other text is the answer in words.", choices.join(", ")),
        (None, true) => "There is no suggestion and no choice. Write the answer in words.".to_string(),
    };
    text.push_str(&format!("\n{}\n", how));
    text
}

/// The whole name a word gives a question: `tester.q1`, `q1` or `planner.q1` for the planner, or the whole name.
/// It looks at no comment, so that two words for one question are known before anything is read.
pub fn whole_name(on: &str, word: &str) -> String {
    if word.starts_with(&format!("{}.", on)) && split_name(on, word).is_some() {
        word.to_string()
    } else if let Some(rest) = word.strip_prefix("planner.") {
        format!("{}.{}", on, rest)
    } else {
        format!("{}.{}", on, word)
    }
}

/// The question a word names among the open ones.
fn named<'a>(on: &str, questions: &'a [Question], word: &str) -> Result<&'a Question, String> {
    let whole = whole_name(on, word);
    questions.iter().find(|question| question.name() == whole).ok_or_else(|| format!("there is no open question {} on {}", issue::shown(&whole), on))
}

/// `wb answer` in the repository at `root`: 0 the answers are posted, or there was no open question; 1 it was
/// refused, the input ended before every question was answered, or a pattern failed. What it shows and asks goes
/// to `out`; what it will post, and what it did, to `err`. It never returns 2: a question named twice, by one word or
/// by two (`whole_name`), is refused with 2 and the usage by `run_answer` before anything is read.
pub fn answer(root: &str, on: On, asked: &Asked, input: &mut dyn BufRead, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let (word, value, read, post) = match on {
        On::Issue(issue) => ("issue", issue, "issue_comment_bodies", "issue_comment"),
        On::Change(change) => ("change", change, "change_comment_bodies", "change_comment"),
    };
    if let Err(why) = starter::held(root) {
        return crate::refusal::limit(err, "wb answer", format_args!("{}.", why));
    }
    // A job whose ref holds a stamp or a questions commit (#238, #333) has its questions and answers on the ref, and the tracker is not
    // read for them: the answers are written as commits, signed by the person, and a copy is posted where a tracker is
    // set up. Any other job, and a change with no issue, is read from the tracker and answered there, as before.
    let ledger = match on {
        On::Issue(issue) if prompt::is_plain_name(issue) => match crate::stamps::read(root, issue) {
            Ok(ledger) if ledger.on_ref() => Some(ledger),
            Ok(_) => None,
            Err(why) => return crate::refusal::limit(err, "wb answer", format_args!("{}. Nothing is written.", why)),
        },
        _ => None,
    };
    let mut commands = Vec::new();
    let mut signing = None;
    // Where a copy of the answers is posted, in the way of the ref: none where no tracker is set up.
    let mut copy: Option<String> = None;
    let questions = match &ledger {
        Some(ledger) => {
            // `--show` writes nothing, so it needs no key; every other way writes commits that count only if signed.
            if *asked != Asked::Show {
                match crate::stamps::ready_for_answer(root) {
                    Ok(made) => signing = Some(made),
                    Err(why) => return crate::refusal::limit(err, "wb answer", format_args!("{}. Nothing is written and nothing is posted.", why)),
                }
            }
            if let Some(why) = &ledger.broken {
                return crate::refusal::limit(err, "wb answer", format_args!("{}. Nothing is written and nothing is posted.", why));
            }
            if let Some(why) = &ledger.missing {
                return crate::refusal::limit(err, "wb answer", format_args!("{}, so no question or answer on {} counts.", why, ledger.full));
            }
            if *asked != Asked::Show {
                match prompt::pattern_of(root, post) {
                    Ok(line) => copy = line.map(|line| line.replace(&format!("{{{}}}", word), value)),
                    Err(why) => return crate::refusal::limit(err, "wb answer", format_args!("{}", why)),
                }
            }
            let questions = open_questions(value, &ledger.comments());
            if questions.is_empty() {
                crate::refusal::inform(err, format_args!("wb answer: there is no open question on {} {} (from {}). Nothing is posted.", word, value, ledger.full));
                return 0;
            }
            questions
        }
        None => {
            // `--show` posts nothing, so it needs only the pattern that reads.
            let keys: &[&str] = if *asked == Asked::Show { &[read] } else { &[read, post] };
            for &key in keys {
                match prompt::pattern_of(root, key) {
                    Ok(Some(line)) if prompt::is_plain_name(value) => commands.push(line.replace(&format!("{{{}}}", word), value)),
                    Ok(Some(_)) => {
                        return crate::refusal::limit(err, "wb answer", format_args!("'{}' is not a plain name that fills `{}`.", value.escape_debug(), key));
                    }
                    Ok(None) => {
                        return crate::refusal::limit(err, "wb answer", format_args!("watchbill.yml has no `{}`. Add the line (see the README). Nothing is posted.", key));
                    }
                    Err(why) => {
                        return crate::refusal::limit(err, "wb answer", format_args!("{}", why));
                    }
                }
            }
            let printed = match starter::run(root, read, &commands[0], None) {
                Ok(printed) => printed,
                Err(why) => {
                    return crate::refusal::limit(err, "wb answer", format_args!("{}. Nothing is posted.", why));
                }
            };
            let questions = open_questions(value, &split_bodies(&printed));
            if questions.is_empty() {
                crate::refusal::inform(err, format_args!("wb answer: there is no open question on {} {} (from `{}` of watchbill.yml). Nothing is posted.", word, value, read));
                return 0;
            }
            questions
        }
    };
    if *asked == Asked::Show {
        let shown: Vec<String> = questions.iter().enumerate().map(|(at, question)| show(question, at + 1, questions.len())).collect();
        let _ = out.write_all(shown.concat().trim_start_matches('\n').as_bytes());
        return 0;
    }
    // Each question answered, with its line `q<n>: …`, in the order asked.
    let mut answered: Vec<(&Question, String, Option<String>)> = Vec::new();
    // Where it is not given, the role that acts on a question whose choices name roles is the one that asked.
    let settled = |question: &Question, role: Option<String>| if question.roled() { Some(role.unwrap_or_else(|| question.role())) } else { None };
    match asked {
        Asked::Pairs(pairs) | Asked::Suggested(pairs) => {
            // `run_answer` has refused a question named twice, before anything was read.
            let mut given: Vec<(&Question, &str)> = Vec::new();
            for (which, text) in pairs {
                match named(value, &questions, which) {
                    Ok(question) => given.push((question, text)),
                    Err(why) => {
                        return crate::refusal::limit(err, "wb answer", format_args!("{}. Nothing is posted.", why));
                    }
                }
            }
            let mut taken: Vec<(&Question, String, Option<String>)> = Vec::new();
            for (question, text) in &given {
                match take(question, text) {
                    Ok((line, role)) => taken.push((question, line, settled(question, role))),
                    Err(why) => {
                        return crate::refusal::limit(err, "wb answer", format_args!("{} Nothing is posted.", why));
                    }
                }
            }
            if let Asked::Suggested(_) = asked {
                for question in &questions {
                    match taken.iter().position(|(known, _, _)| known.name() == question.name()) {
                        Some(at) => answered.push(taken.remove(at)),
                        None => match take(question, "") {
                            Ok((line, role)) => answered.push((question, line, settled(question, role))),
                            Err(_) => {
                                let _ = writeln!(err, "wb answer: {} has no suggested answer, and is not answered.", question.name());
                            }
                        },
                    }
                }
                if answered.is_empty() {
                    return crate::refusal::limit(err, "wb answer", format_args!("no open question on {} {} has a suggested answer. Nothing is posted.", word, value));
                }
            } else {
                answered = taken;
            }
        }
        Asked::Show => unreachable!("--show returned above"),
        Asked::Each => {
            let _ = writeln!(out, "{} open question{} on {} {}.", questions.len(), if questions.len() == 1 { "" } else { "s" }, word, value);
            for (at, question) in questions.iter().enumerate() {
                let _ = out.write_all(show(question, at + 1, questions.len()).as_bytes());
                let ended = |out: &mut dyn Write, err: &mut dyn Write, answered: usize| {
                    let _ = writeln!(
                        err,
                        "wb answer: the input ended before every question was answered ({} of {}), so nothing is posted.",
                        answered,
                        questions.len()
                    );
                    let _ = out.flush();
                    // The person ended the input before every question was answered: not a refusal.
                    crate::refusal::answer();
                    1
                };
                loop {
                    let Some(typed) = ask_line(input, out, &format!("{}> ", question.name())) else { return ended(out, err, answered.len()) };
                    match take(question, &typed) {
                        Ok((line, mut role)) => {
                            // A choice with no role, and words, leave it to the person who answers.
                            while question.roled() && role.is_none() {
                                let asking = question.role();
                                let roles = ROLES.join(", ");
                                let prompt = format!("{}: which role acts on it? Enter takes {}. The roles: {}.\n{} role> ", question.name(), asking, roles, question.name());
                                let Some(typed) = ask_line(input, out, &prompt) else { return ended(out, err, answered.len()) };
                                let typed = typed.trim();
                                if typed.is_empty() {
                                    role = Some(asking);
                                } else if ROLES.contains(&typed) {
                                    role = Some(typed.to_string());
                                } else {
                                    let _ = writeln!(out, "{} is not a role. Answer again.", issue::shown(typed));
                                }
                            }
                            answered.push((question, line, role));
                            break;
                        }
                        Err(why) => {
                            let _ = writeln!(out, "{} Answer again.", why);
                        }
                    }
                }
            }
        }
    }
    // One comment per role, in the order the roles first asked.
    let mut comments: Vec<(String, Vec<(&Question, String, Option<String>)>)> = Vec::new();
    for (question, line, role) in answered {
        match comments.iter_mut().find(|(prefix, _)| *prefix == question.prefix) {
            Some((_, kept)) => kept.push((question, line, role)),
            None => comments.push((question.prefix.clone(), vec![(question, line, role)])),
        }
    }
    let texts: Vec<String> = comments
        .iter()
        .map(|(prefix, kept)| {
            let names: Vec<String> = kept.iter().map(|(question, _, role)| format!("q{}{}", question.number, role.as_ref().map_or(String::new(), |role| format!(">{}", role)))).collect();
            let lines: String = kept.iter().map(|(_, line, _)| format!("{}\n", line)).collect();
            format!("[answers] {} {}\n{}", prefix, names.join(" "), lines)
        })
        .collect();
    if let (Some(ledger), Some(signing)) = (&ledger, &signing) {
        // The answers are commits on the job's ref (#238), one for each role; then each copy is posted, where a tracker is
        // set up.
        let _ = write!(
            err,
            "wb answer: will write on issue {}, as {} on {} signed with the role answer, {}:\n",
            value,
            if texts.len() == 1 { "one commit".to_string() } else { format!("{} commits, one per role,", texts.len()) },
            ledger.full,
            if texts.len() == 1 { "the answers" } else { "the answers of each" }
        );
        for text in &texts {
            let _ = write!(err, "\n{}", text.split('\n').map(issue::shown).collect::<Vec<_>>().join("\n"));
        }
        let _ = writeln!(err);
        // A ref that stopped being as `wb` writes it while the answers were typed: nothing is written on it.
        match crate::stamps::not_as_written(root, value) {
            Ok(None) => {}
            Ok(Some(why)) | Err(why) => return crate::refusal::limit(err, "wb answer", format_args!("{}. Nothing is written and nothing is posted.", why)),
        }
        let mut messages: Vec<String> = Vec::new();
        for text in &texts {
            match crate::stamps::thread_message(root, value, text) {
                Ok(message) => messages.push(message),
                Err(why) => return crate::refusal::limit(err, "wb answer", format_args!("the answers are not written: {}. Nothing is written and nothing is posted.", why)),
            }
        }
        let mut written: Vec<String> = Vec::new();
        for (at, message) in messages.iter().enumerate() {
            match crate::stamps::write("wb answer", root, value, message, &crate::stamps::Held::Answers, signing, err) {
                Ok((commit, _)) => written.push(commit),
                Err(why) => {
                    let done = match at {
                        0 => "No answer commit was written".to_string(),
                        n => format!("The first {} of {} answer commits were written: {}", n, messages.len(), written.join(", ")),
                    };
                    return crate::refusal::limit(err, "wb answer", format_args!("{}. {}; nothing is posted.", why, done));
                }
            }
        }
        match &copy {
            None => {
                let _ = writeln!(err, "wb answer: watchbill.yml has no `{}`, so no copy of the answers is posted: they are the commits on {}.", post, ledger.full);
            }
            Some(line) => {
                let _ = writeln!(err, "wb answer: will post on {} {}, by `{}` of watchbill.yml, a copy of each commit:", word, value, post);
                for (text, commit) in texts.iter().zip(&written) {
                    match starter::run(root, post, line, Some(text.as_bytes())) {
                        Ok(said) => {
                            let _ = err.write_all(&said);
                        }
                        Err(why) => {
                            return crate::refusal::limit(
                                err,
                                "wb answer",
                                format_args!("{}. The answer commit {} is on {}; its copy may not be posted, and no later copy is.", why, commit, ledger.full),
                            );
                        }
                    }
                }
                crate::refusal::inform(err, format_args!("wb answer: posted."));
            }
        }
    } else {
        let _ = write!(err, "wb answer: will post on {} {}, by `{}` of watchbill.yml, {}:\n", word, value, post, if texts.len() == 1 { "one comment".to_string() } else { format!("{} comments, one per role", texts.len()) });
        for text in &texts {
            let _ = write!(err, "\n{}", text.split('\n').map(issue::shown).collect::<Vec<_>>().join("\n"));
        }
        let _ = writeln!(err);
        for (at, text) in texts.iter().enumerate() {
            match starter::run(root, post, &commands[1], Some(text.as_bytes())) {
                Ok(said) => {
                    let _ = err.write_all(&said);
                }
                Err(why) => {
                    let posted = match at {
                        0 => "No comment was posted".to_string(),
                        n => format!("The first {} of {} comments were posted", n, texts.len()),
                    };
                    return crate::refusal::limit(err, "wb answer", format_args!("{}. {}; this one may not be.", why, posted));
                }
            }
        }
        crate::refusal::inform(err, format_args!("wb answer: posted."));
    }
    // An answer that changes the issue's file is taken by a person, never by `wb`.
    for (_, kept) in &comments {
        let changing: Vec<String> = kept.iter().filter(|(question, _, role)| changes_file(question, role)).map(|(question, _, _)| question.name()).collect();
        if changing.is_empty() {
            continue;
        }
        let role = kept[0].0.role();
        let file = format!("{}/{}.yml", issue::FOLDER, value);
        // The command that accepts it: `wb plan accept` where a plan of the name is there (#290).
        let accepting = if crate::plan::is_plan(root, value) { "wb plan accept" } else { "wb accept" };
        let said = match (on, role.as_str()) {
            (On::Change(_), _) => format!(
                "the answer to {} changes the issue's file, and change {} has no issue: change its description, which is its spec, before the role is started again. wb answer does not change it.",
                changing.join(", "),
                value
            ),
            (On::Issue(_), "planner") => format!(
                "the answer to {} changes the issue's file: start `wb start planner {}` again to fold it into {}, and accept the file with `{} {}`. wb answer does not change the file.",
                changing.join(", "),
                value,
                file,
                accepting,
                value
            ),
            (On::Issue(_), _) => format!(
                "the answer to {} changes the issue's file: change {} to hold it and accept it again with `{} {}`, before `wb start {} {}` is started again. wb answer does not change the file.",
                changing.join(", "),
                file,
                accepting,
                value,
                role,
                value
            ),
        };
        let _ = writeln!(err, "wb answer: {}", said);
    }
    0
}
