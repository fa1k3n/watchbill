//! `wb answer` (#145): the open questions of an issue, shown in full in the terminal one at a time, and the
//! answers posted in one comment per role.
//!
//! - **A question** is asked in one comment per session, whose first line is `[questions] <issue>.<role> q1 q2 …`
//!   (`[questions] <issue> …` for the breakdown, whose questions are `<issue>.q<n>`, #95). Each question is then
//!   a line `q<n>: <what is unclear>`, with the rest below it: why it matters, the choices numbered from 1, a line
//!   `Suggested: …` and a line `changes the issue's file: yes` or `no`. A question of revision 0.0.8 is a comment
//!   of its own whose first line is `[question] <name>`, and is read the same way.
//! - **An answer** is a comment whose first line is `[answers] <issue>.<role> q1 q2 …`, then a line `q<n>: <answer>`
//!   for each. The lines of revision 0.0.8, `[answer] <name> …` and `[answer] <issue> suggested`, are read too.
//! - **Which count:** `open_names` decides from first lines alone, so that `wb status` and `wb answer` agree. What a
//!   `[revert]` marker took back counts for nothing (`starter::taken_back`).
//!
//! The whole comments are read through `issue_comment_bodies` (or `change_comment_bodies`), each body followed by
//! a NUL byte. Answers are posted through `issue_comment` (or `change_comment`), after `wb` says what it will post.
//! Nothing is posted until every question shown has its answer: an input that ends, or Ctrl-C, posts nothing.

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
    /// Take every suggestion.
    Suggested,
    /// Answer the question named, with the text given.
    One(String, String),
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
    /// The part of its name before `q<n>`: `<issue>` for the breakdown, `<issue>.<role>` for the others.
    pub prefix: String,
    pub number: u64,
    /// What is unclear, from its line `q<n>: …`; empty for a question of revision 0.0.8.
    pub short: String,
    /// The rest of the question, as it is shown.
    pub lines: Vec<String>,
    /// The numbers of its choices.
    pub choices: Vec<u64>,
    pub suggested: Option<Suggestion>,
    /// What its line `changes the issue's file:` says, where it has one.
    pub changes_file: Option<bool>,
}

impl Question {
    /// Its whole name: `175.tester.q1`, `175.q1`.
    pub fn name(&self) -> String {
        format!("{}.q{}", self.prefix, self.number)
    }

    /// Its role, or `breakdown` for a name with no role.
    pub fn role(&self) -> String {
        self.prefix.split_once('.').map_or("breakdown".to_string(), |(_, role)| role.to_string())
    }
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
        Some(prefix) if is_prefix(issue, prefix) => words.filter_map(number_of).map(|n| format!("{}.q{}", prefix, n)).collect(),
        _ => Vec::new(),
    }
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
    for line in &lines {
        let bare = line.trim();
        let digits: String = bare.chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty() && bare[digits.len()..].starts_with(". ") {
            if let Ok(n) = digits.parse::<u64>() {
                if !choices.contains(&n) {
                    choices.push(n);
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
    Question { prefix: prefix.to_string(), number, short, lines, choices, suggested, changes_file }
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

/// What a person's text gives for a question: the line `q<n>: …`, or why it is asked again.
fn take(question: &Question, typed: &str) -> Result<String, String> {
    let typed = typed.trim();
    let line = |answer: &str| format!("q{}: {}", question.number, answer);
    if typed.is_empty() {
        return match &question.suggested {
            Some(Suggestion::Choice(n)) => Ok(line(&n.to_string())),
            Some(Suggestion::Words(words)) => Ok(line(words)),
            None => Err(format!("{} has no suggested answer.", question.name())),
        };
    }
    if typed.chars().all(|c| c.is_ascii_digit()) {
        let n: Option<u64> = typed.parse().ok();
        if n.is_some_and(|n| question.choices.contains(&n)) {
            return Ok(line(typed));
        }
        let choices: Vec<String> = question.choices.iter().map(u64::to_string).collect();
        let which = if choices.is_empty() { "it has none".to_string() } else { choices.join(", ") };
        return Err(format!("{} is not one of the choices of {} ({}).", typed, question.name(), which));
    }
    if typed.chars().any(char::is_control) {
        return Err("An answer is one line, with no control character.".to_string());
    }
    Ok(line(typed))
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

/// The question a word names: `tester.q1`, `q1` or `breakdown.q1` for the breakdown, or the whole name.
fn named<'a>(on: &str, questions: &'a [Question], word: &str) -> Result<&'a Question, String> {
    let whole = if word.starts_with(&format!("{}.", on)) && split_name(on, word).is_some() {
        word.to_string()
    } else if let Some(rest) = word.strip_prefix("breakdown.") {
        format!("{}.{}", on, rest)
    } else {
        format!("{}.{}", on, word)
    };
    questions.iter().find(|question| question.name() == whole).ok_or_else(|| format!("there is no open question {} on {}", issue::shown(&whole), on))
}

/// `wb answer` in the repository at `root`: 0 the answers are posted, or there was no open question; 1 it was
/// refused, the input ended before every question was answered, or a pattern failed. What it shows and asks goes
/// to `out`; what it will post, and what it did, to `err`.
pub fn answer(root: &str, on: On, asked: &Asked, input: &mut dyn BufRead, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let (word, value, read, post) = match on {
        On::Issue(issue) => ("issue", issue, "issue_comment_bodies", "issue_comment"),
        On::Change(change) => ("change", change, "change_comment_bodies", "change_comment"),
    };
    let mut commands = Vec::new();
    for key in [read, post] {
        match prompt::pattern_of(root, key) {
            Ok(Some(line)) if prompt::is_plain_name(value) => commands.push(line.replace(&format!("{{{}}}", word), value)),
            Ok(Some(_)) => {
                let _ = writeln!(err, "wb answer: '{}' is not a plain name that fills `{}`.", value.escape_debug(), key);
                return 1;
            }
            Ok(None) => {
                let _ = writeln!(err, "wb answer: watchbill.yml has no `{}`. Add the line (see the README). Nothing is posted.", key);
                return 1;
            }
            Err(why) => {
                let _ = writeln!(err, "wb answer: {}", why);
                return 1;
            }
        }
    }
    let printed = match starter::run(root, read, &commands[0], None) {
        Ok(printed) => printed,
        Err(why) => {
            let _ = writeln!(err, "wb answer: {}. Nothing is posted.", why);
            return 1;
        }
    };
    let questions = open_questions(value, &split_bodies(&printed));
    if questions.is_empty() {
        let _ = writeln!(err, "wb answer: there is no open question on {} {} (from `{}` of watchbill.yml). Nothing is posted.", word, value, read);
        return 0;
    }
    // Each question answered, with its line `q<n>: …`, in the order asked.
    let mut answered: Vec<(&Question, String)> = Vec::new();
    match asked {
        Asked::One(which, text) => {
            let question = match named(value, &questions, which) {
                Ok(question) => question,
                Err(why) => {
                    let _ = writeln!(err, "wb answer: {}. Nothing is posted.", why);
                    return 1;
                }
            };
            match take(question, text) {
                Ok(line) => answered.push((question, line)),
                Err(why) => {
                    let _ = writeln!(err, "wb answer: {} Nothing is posted.", why);
                    return 1;
                }
            }
        }
        Asked::Suggested => {
            for question in &questions {
                match take(question, "") {
                    Ok(line) => answered.push((question, line)),
                    Err(_) => {
                        let _ = writeln!(err, "wb answer: {} has no suggested answer, and is not answered.", question.name());
                    }
                }
            }
            if answered.is_empty() {
                let _ = writeln!(err, "wb answer: no open question on {} {} has a suggested answer. Nothing is posted.", word, value);
                return 1;
            }
        }
        Asked::Each => {
            let _ = writeln!(out, "{} open question{} on {} {}.", questions.len(), if questions.len() == 1 { "" } else { "s" }, word, value);
            for (at, question) in questions.iter().enumerate() {
                let _ = out.write_all(show(question, at + 1, questions.len()).as_bytes());
                loop {
                    let _ = write!(out, "{}> ", question.name());
                    let _ = out.flush();
                    let mut typed = Vec::new();
                    match input.read_until(b'\n', &mut typed) {
                        Ok(0) | Err(_) => {
                            let _ = writeln!(out);
                            let _ = writeln!(
                                err,
                                "wb answer: the input ended before every question was answered ({} of {}), so nothing is posted.",
                                answered.len(),
                                questions.len()
                            );
                            return 1;
                        }
                        Ok(_) => {}
                    }
                    let typed = String::from_utf8_lossy(&typed);
                    let typed = typed.trim_end_matches('\n').trim_end_matches('\r');
                    match take(question, typed) {
                        Ok(line) => {
                            answered.push((question, line));
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
    let mut comments: Vec<(String, Vec<(&Question, String)>)> = Vec::new();
    for (question, line) in answered {
        match comments.iter_mut().find(|(prefix, _)| *prefix == question.prefix) {
            Some((_, kept)) => kept.push((question, line)),
            None => comments.push((question.prefix.clone(), vec![(question, line)])),
        }
    }
    let texts: Vec<String> = comments
        .iter()
        .map(|(prefix, kept)| {
            let names: Vec<String> = kept.iter().map(|(question, _)| format!("q{}", question.number)).collect();
            let lines: String = kept.iter().map(|(_, line)| format!("{}\n", line)).collect();
            format!("[answers] {} {}\n{}", prefix, names.join(" "), lines)
        })
        .collect();
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
                let _ = writeln!(err, "wb answer: {}. {}; this one may not be.", why, posted);
                return 1;
            }
        }
    }
    let _ = writeln!(err, "wb answer: posted.");
    // An answer that changes the issue's file is taken by a person, never by `wb`.
    for (_, kept) in &comments {
        let changing: Vec<String> = kept.iter().filter(|(question, _)| question.changes_file == Some(true)).map(|(question, _)| question.name()).collect();
        if changing.is_empty() {
            continue;
        }
        let role = kept[0].0.role();
        let file = format!("{}/{}.yml", issue::FOLDER, value);
        let said = match (on, role.as_str()) {
            (On::Change(_), _) => format!(
                "the answer to {} changes the issue's file, and change {} has no issue: change its description, which is its spec, before the role is started again. wb answer does not change it.",
                changing.join(", "),
                value
            ),
            (On::Issue(_), "breakdown") => format!(
                "the answer to {} changes the issue's file: start `wb breakdown {}` again to fold it into {}, and accept the file with `wb accept {}`. wb answer does not change the file.",
                changing.join(", "),
                value,
                file,
                value
            ),
            (On::Issue(_), _) => format!(
                "the answer to {} changes the issue's file: change {} to hold it and accept it again with `wb accept {}`, before `wb {} {}` is started again. wb answer does not change the file.",
                changing.join(", "),
                file,
                value,
                role,
                value
            ),
        };
        let _ = writeln!(err, "wb answer: {}", said);
    }
    0
}
