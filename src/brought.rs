//! The breakdown reads its issue without a login (#295). Its session runs in a sandbox with no login to the tracker
//! (#202), so `wb` reads the issue for it, as the person and outside the sandbox, and brings it into the view; after
//! the session `wb` posts the breakdown's questions from the issue's file that came back.
//!
//! - **The issue's text** is what `issue_address` prints, with each comment that `issue_comment_bodies` prints taken
//!   out where it is found, from the last back: `.watchbill/issue/<issue>.md`. Its SHA-256 goes in the stamp, as
//!   `issue_text:`, and the breakdown writes it under `from`.
//! - **Its questions and answers** are the comments whose first line is `[questions] <issue>` or `[answers] <issue>`,
//!   or a form of revision 0.0.8, whole and in order: `.watchbill/issue/<issue>.questions`. A stamp and any other
//!   comment are not brought in.
//! - Both are read through the cap of a text file (#261), and are read-only in the sandbox: they lie under
//!   `.watchbill/`. They are never taken back.
//! - **The questions** of the issue's file, each written in full, are posted after the session in one comment
//!   `[questions] <issue> q1 q2 …`, by `issue_comment`, but for those the issue holds already.

use std::fs;
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::{answer, bounded, prompt, starter};

/// Where `wb` brings the issue into the breakdown's view, from its root.
pub const FOLDER: &str = ".watchbill/issue";

/// What was brought in: the SHA-256 of the issue's text, for the stamp.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brought {
    pub hash: String,
}

/// The line of the pattern `key`, or why there is none.
fn pattern(root: &str, key: &str) -> Result<String, String> {
    match prompt::pattern_of(root, key) {
        Ok(Some(line)) => Ok(line),
        Ok(None) => Err(format!(
            "watchbill.yml has no `{}`, and the breakdown's session has no login: wb reads the issue for it by `issue_address` and `issue_comment_bodies`. Add the line (see the README)",
            key
        )),
        Err(why) => Err(why.to_string()),
    }
}

/// Whether the first line of a comment is a question or an answer of the breakdown of `issue`: `[questions] <issue>`
/// or `[answers] <issue>` with the names after it, or `[question] <issue>.q<n>`, `[answer] <issue>.q<n>` and
/// `[answer] <issue> suggested` of revision 0.0.8.
fn asked_or_answered(issue: &str, first: &str) -> bool {
    let words: Vec<&str> = first.split_whitespace().collect();
    let named = |name: &str| name.strip_prefix(issue).and_then(|rest| rest.strip_prefix(".q")).is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
    match words.as_slice() {
        ["[questions]" | "[answers]", on, ..] => *on == issue,
        ["[answer]", on, "suggested", ..] if *on == issue => true,
        ["[question]" | "[answer]", name, ..] => named(name),
        _ => false,
    }
}

/// `text` with each of `bodies` taken out where it is found, from the last back: the comments come after the
/// issue's text, in the order they were written. A body that is not found is passed over.
fn without_comments(text: &str, bodies: &[String]) -> String {
    let mut end = text.len();
    let mut cuts: Vec<(usize, usize)> = Vec::new();
    for body in bodies.iter().rev() {
        let body = body.trim_end();
        if body.trim().is_empty() {
            continue;
        }
        if let Some(at) = text[..end].rfind(body) {
            cuts.push((at, at + body.len()));
            end = at;
        }
    }
    let mut kept = String::with_capacity(text.len());
    let mut from = 0;
    for (start, stop) in cuts.into_iter().rev() {
        kept.push_str(&text[from..start]);
        from = stop;
    }
    kept.push_str(&text[from..]);
    kept
}

/// Writes a new file of the view, never over a file that is there: a link in its place is not followed (`O_EXCL`).
fn write_new(path: &Path, text: &str) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| format!("{} could not be written in the view: {}", path.display(), error))?;
    file.write_all(text.as_bytes()).map_err(|error| format!("{} could not be written in the view: {}", path.display(), error))
}

/// Reads the issue `issue` by `issue_address` and its comments by `issue_comment_bodies`, as the person, in the root
/// of the checkout, each through the cap of a text file, and writes the issue's text and its questions and answers
/// into `.watchbill/issue/` of the view at `view`. Refused, with nothing written, where `HEAD` holds that folder, a
/// pattern is not there, fails, or prints more than the cap.
pub fn bring_in(root: &str, issue: &str, view: &Path) -> Result<Brought, String> {
    let folder = view.join(FOLDER);
    match fs::symlink_metadata(&folder) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        _ => {
            return Err(format!(
                "HEAD holds {}, and that is where wb brings the issue into the breakdown's view. Take it out of the branch",
                FOLDER
            ))
        }
    }
    let address = pattern(root, "issue_address")?;
    let comments = pattern(root, "issue_comment_bodies")?;
    let text = starter::run_capped(root, "issue_address", &address.replace("{issue}", issue), bounded::TEXT_CAP)?;
    let printed = starter::run_capped(root, "issue_comment_bodies", &comments.replace("{issue}", issue), bounded::TEXT_CAP)?;
    let bodies = answer::split_bodies(&printed);
    let text = without_comments(&String::from_utf8_lossy(&text), &bodies);
    let kept: Vec<&str> = bodies
        .iter()
        .filter(|body| asked_or_answered(issue, body.split('\n').next().unwrap_or_default().trim_end_matches('\r')))
        .map(|body| body.trim_end())
        .collect();
    let mut questions = kept.join("\n\n");
    if !questions.is_empty() {
        questions.push('\n');
    }
    fs::DirBuilder::new().mode(0o700).create(&folder).map_err(|error| format!("{} could not be made in the view: {}", FOLDER, error))?;
    write_new(&folder.join(format!("{}.md", issue)), &text)?;
    write_new(&folder.join(format!("{}.questions", issue)), &questions)?;
    let hash = Sha256::digest(text.as_bytes()).iter().map(|byte| format!("{:02x}", byte)).collect();
    Ok(Brought { hash })
}

/// One question of the issue's file: its number and the line of what is unclear, then the rest as it was written.
struct Question {
    number: u64,
    short: String,
    rest: String,
}

/// A question of the issue's file of `issue`: its first line `<issue>.q<n>: <what is unclear>`, then the rest.
fn question_of(issue: &str, text: &str) -> Result<Question, String> {
    let (first, rest) = text.split_once('\n').unwrap_or((text, ""));
    let named = first.strip_prefix(issue).and_then(|after| after.strip_prefix(".q")).and_then(|after| after.split_once(':'));
    let Some((digits, short)) = named.filter(|(digits, _)| !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())) else {
        return Err(format!(
            "the question {} of the issue's file does not start with its name, `{}.q<n>:`",
            crate::issue::shown(first),
            issue
        ));
    };
    let number = digits.parse().map_err(|_| format!("the number of the question {} is too big", crate::issue::shown(first)))?;
    let rest = rest.trim_end().trim_start_matches(['\n', '\r']).to_string();
    Ok(Question { number, short: short.trim().to_string(), rest })
}

/// The comment of questions of point 13: the first line names them, each is its line `q<n>: …` and the rest under
/// `<details>`.
fn comment_of(issue: &str, questions: &[Question]) -> String {
    let names: Vec<String> = questions.iter().map(|question| format!("q{}", question.number)).collect();
    let parts: Vec<String> = questions
        .iter()
        .map(|question| {
            let mut part = format!("q{}: {}\n", question.number, question.short);
            if !question.rest.is_empty() {
                part.push_str(&format!("<details>\n\n{}\n\n</details>\n", question.rest));
            }
            part
        })
        .collect();
    format!("[questions] {} {}\n{}", issue, names.join(" "), parts.join("\n"))
}

/// Posts the questions of the issue's file of `issue`, as the working tree of the checkout at `root` has it once the
/// breakdown's commit is taken back, that the issue does not hold yet (#295): by `issue_comment`, after it is said on
/// `err`. A file with no new question posts nothing. The error says why nothing was posted.
pub fn post_questions(name: &str, root: &str, issue: &str, err: &mut dyn Write) -> Result<(), String> {
    let path = Path::new(root).join(crate::issue::FOLDER).join(format!("{}.yml", issue));
    let document = crate::yamlfile::load(&path).map_err(|why| why.to_string())?;
    let place = [crate::yamlfile::Part::from("questions")];
    let written = if document.has(&place) { document.texts(&place, true).map_err(|why| why.to_string())? } else { Vec::new() };
    if written.is_empty() {
        return Ok(());
    }
    let questions = written.iter().map(|text| question_of(issue, text)).collect::<Result<Vec<_>, _>>()?;
    let read = pattern_to_post(root, "issue_comments")?;
    let printed = starter::run_capped(root, "issue_comments", &read.replace("{issue}", issue), bounded::TEXT_CAP)?;
    let lines = starter::counted(&String::from_utf8_lossy(&printed));
    let asked = answer::asked_names(issue, &lines);
    let new: Vec<Question> = questions.into_iter().filter(|question| !asked.contains(&format!("{}.q{}", issue, question.number))).collect();
    if new.is_empty() {
        return Ok(());
    }
    let post = pattern_to_post(root, "issue_comment")?;
    let comment = comment_of(issue, &new);
    let _ = write!(err, "{}: will post on issue {}, by `issue_comment` of watchbill.yml:\n{}", name, issue, comment);
    let said = starter::run(root, "issue_comment", &post.replace("{issue}", issue), Some(comment.as_bytes()))?;
    let _ = err.write_all(&said);
    Ok(())
}

/// The line of a pattern the questions are posted by, or why there is none.
fn pattern_to_post(root: &str, key: &str) -> Result<String, String> {
    match prompt::pattern_of(root, key) {
        Ok(Some(line)) => Ok(line),
        Ok(None) => Err(format!("watchbill.yml has no `{}`. Add the line (see the README)", key)),
        Err(why) => Err(why.to_string()),
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn test_WB_BUILD_135_each_comment_is_taken_out_once_from_the_last_back() {
        let text = "ok\nThe text.\n--\nok\n--\nA stamp\n--\n";
        let bodies = vec!["ok".to_string(), "A stamp\n".to_string()];
        assert_eq!(without_comments(text, &bodies), "ok\nThe text.\n--\n\n--\n\n--\n");
    }

    #[test]
    fn test_WB_BUILD_136_only_the_breakdown_s_questions_and_answers_of_the_issue_are_kept() {
        for first in ["[questions] 54 q1", "[answers] 54 q1>breakdown", "[question] 54.q3", "[answer] 54.q3 2", "[answer] 54 suggested"] {
            assert!(asked_or_answered("54", first), "{}", first);
        }
        for first in ["[stamp] breakdown round 1", "[questions] 54.tester q1", "[answers] 541 q1", "[question] 54.tester.q1", "[questions] 5 q1", "Thanks"] {
            assert!(!asked_or_answered("54", first), "{}", first);
        }
    }
}
