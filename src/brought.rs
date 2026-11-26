//! The planner reads its issue without a login (#295). Its session runs in a sandbox with no login to the tracker
//! (#202), so `wb` reads the issue for it, as the person and outside the sandbox, and brings it into the view; after
//! the session `wb` posts the planner's questions from the issue's file that came back.
//!
//! - **The issue's text** is what `issue_text` prints, the issue's text alone, whole and byte for byte:
//!   `.watchbill/issue/<issue>.md`. Nothing is cut from it (the second look at #297): no rule can tell the end of an
//!   issue's text from a tracker's lines around a comment. Its SHA-256 goes in the stamp, as
//!   `issue_text:`, and the planner writes it under `from`.
//! - **Its questions and answers** are the comments whose first line is `[questions] <issue>` or `[answers] <issue>`,
//!   or a form of revision 0.0.8, whole and in order: `.watchbill/issue/<issue>.questions`. A stamp and any other
//!   comment are not brought in.
//! - Both are read through the cap of a text file (#261), and are read-only in the sandbox: they lie under
//!   `.watchbill/`. They are never taken back.
//! - **The questions** of the issue's file, each written in full, are posted after the session in one comment
//!   `[questions] <issue> q1 q2 …`, by `issue_comment`, but for those the issue holds already. The file is read from
//!   the last commit taken back. A question that holds a character a prompt never holds (`issue::is_unsafe`) is
//!   refused, and nothing is posted: what the session wrote never reaches the terminal or the tracker raw.

use std::fs;
use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::{answer, bounded, prompt, starter};

/// Where `wb` brings the issue into the planner's view, from its root.
pub const FOLDER: &str = ".watchbill/issue";

/// What was brought in: the SHA-256 of the issue's text, for the stamp.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brought {
    pub hash: String,
    /// What the tracker holds of the job that is not read once its first stamp is on the ref (#333), where the job had
    /// none yet: said before the stamp is written.
    pub notice: Option<String>,
}

/// The line of the pattern `key`, or why there is none.
fn pattern(root: &str, key: &str) -> Result<String, String> {
    match prompt::pattern_of(root, key) {
        Ok(Some(line)) => Ok(line),
        Ok(None) if key == "issue_text" => Err(
            "watchbill.yml has no `issue_text`, the pattern that prints the issue's text alone, and the planner's session has no login: wb reads the issue for it. `wb upgrade` adds the line `issue_text`; or add it by hand (see the README)"
                .to_string(),
        ),
        Ok(None) => Err(format!(
            "watchbill.yml has no `{}`, and the planner's session has no login: wb reads the issue for it by `issue_text` and `issue_comment_bodies`. Add the line (see the README)",
            key
        )),
        Err(why) => Err(why.to_string()),
    }
}

/// Whether the first line of a comment is a question or an answer of the planner of `issue`: `[questions] <issue>`
/// or `[answers] <issue>` with the names after it, or `[question] <issue>.q<n>`, `[answer] <issue>.q<n>` and
/// `[answer] <issue> suggested` of revision 0.0.8.
fn asked_or_answered(issue: &str, first: &str) -> bool {
    asked_or_answered_by(issue, issue, first)
}

/// The same for the questions named `<prefix>.q<n>` on `on`, the issue or the change (#153): `<prefix>` is
/// `<on>.<role>` for every role but the planner.
fn asked_or_answered_by(on: &str, prefix: &str, first: &str) -> bool {
    let words: Vec<&str> = first.split_whitespace().collect();
    let named = |name: &str| name.strip_prefix(prefix).and_then(|rest| rest.strip_prefix(".q")).is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
    match words.as_slice() {
        ["[questions]" | "[answers]", named_on, ..] => *named_on == prefix,
        ["[answer]", named_on, "suggested", ..] if *named_on == on => true,
        ["[question]" | "[answer]", name, ..] => named(name),
        _ => false,
    }
}

/// Writes a new file in `place`, the view or the folder of what `wb` brings in, never over a file that is there: a
/// link in its place is not followed (`O_EXCL`).
fn write_new(path: &Path, bytes: &[u8], place: &str) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| format!("{} could not be written in {}: {}", path.display(), place, error))?;
    file.write_all(bytes).map_err(|error| format!("{} could not be written in {}: {}", path.display(), place, error))
}

/// Reads the issue's text of `issue` by `issue_text` and its comments by `issue_comment_bodies`, as the person, in the root
/// of the checkout, each through the cap of a text file, and writes the issue's text and its questions and answers
/// into `.watchbill/issue/` of the view at `view`. Refused, with nothing written, where `HEAD` holds that folder, a
/// pattern is not there, fails, or prints more than the cap.
pub fn bring_in(root: &str, issue: &str, view: &Path) -> Result<Brought, String> {
    let folder = view.join(FOLDER);
    match fs::symlink_metadata(&folder) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        _ => {
            return Err(format!(
                "HEAD holds {}, and that is where wb brings the issue into the planner's view. Take it out of the branch",
                FOLDER
            ))
        }
    }
    // A plan of the name brings every source of it, and `issue_text` is not run (#290, decision 1).
    let planned = crate::plan::planner_text(root, issue)?;
    let address = if planned.is_none() { Some(pattern(root, "issue_text")?) } else { None };
    // Where the job's ref holds a stamp, a questions commit or a commit of `wb plan`, its questions and answers are read
    // from the ref, and the tracker is not read for them (#238, #333): a project with no tracker, and so no
    // `issue_comment_bodies`, runs the planner's first round of a plan, which brings in no question. Any other job is
    // read from the tracker, where a missing pattern is refused.
    let ledger = crate::stamps::read(root, issue)?;
    let comments = if ledger.on_ref() { None } else { Some(pattern(root, "issue_comment_bodies")?) };
    let text = match planned {
        Some(text) => text,
        None => starter::run_capped(root, "issue_text", &address.unwrap_or_default().replace("{issue}", issue), bounded::TEXT_CAP)?,
    };
    let bodies = match &comments {
        Some(comments) => answer::split_bodies(&starter::run_capped(root, "issue_comment_bodies", &comments.replace("{issue}", issue), bounded::TEXT_CAP)?),
        None => ledger.thread(),
    };
    // A job with no stamp on its ref yet is read from the tracker for the last time: what it holds is said. A plan has a
    // ref of its commits and no stamp yet: where `issue_comment_bodies` is set, it is read once, for the notice alone
    // (#333), and nothing of it is brought in or counted.
    let notice = if comments.is_some() {
        tracker_notice(issue, &bodies)
    } else if ledger.plan_commit && ledger.entries.is_empty() && ledger.broken.is_none() {
        notice_alone(root, issue)
    } else {
        None
    };
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
    // The issue's text as `issue_text` printed it, byte for byte.
    write_new(&folder.join(format!("{}.md", issue)), &text, "the view")?;
    write_new(&folder.join(format!("{}.questions", issue)), questions.as_bytes(), "the view")?;
    let hash = Sha256::digest(&text).iter().map(|byte| format!("{:02x}", byte)).collect();
    Ok(Brought { hash, notice })
}

/// The notice of `tracker_notice` for a plan with no stamp on its ref yet, from one read of `issue_comment_bodies` where it
/// is set. A pattern that is not there, fails or prints too much gives no notice and no refusal: what it prints reaches
/// nothing but this notice.
fn notice_alone(root: &str, issue: &str) -> Option<String> {
    let line = prompt::pattern_of(root, "issue_comment_bodies").ok().flatten()?;
    let printed = starter::run_capped(root, "issue_comment_bodies", &line.replace("{issue}", issue), bounded::TEXT_CAP).ok()?;
    tracker_notice(issue, &answer::split_bodies(&printed))
}

/// What the tracker holds of `issue` that is not read once the first stamp of its job is on the ref (#333): every question
/// of any role still open, and every answer that waits for a role, from the first lines of `bodies` (the whole comments of
/// the issue), counted as `wb status` counts them. None where nothing is open or waits. It is a notice and no refusal: a
/// refusal would leave no way on.
fn tracker_notice(issue: &str, bodies: &[String]) -> Option<String> {
    let firsts: Vec<String> = bodies.iter().map(|body| body.split('\n').next().unwrap_or_default().trim_end_matches('\r').to_string()).collect();
    let counted = starter::counted(&firsts.join("\n"));
    let open = answer::open_names(issue, &counted);
    let waiting = answer::waiting(issue, &counted);
    if open.is_empty() && waiting.is_empty() {
        return None;
    }
    let mut held: Vec<String> = open.iter().map(|name| format!("{} (open at the tracker)", crate::issue::shown(name))).collect();
    held.extend(waiting.iter().map(|(role, name)| format!("{} (answered at the tracker, waits for {})", crate::issue::shown(name), crate::issue::shown(role))));
    Some(format!(
        "the tracker holds questions and answers of issue {issue} that are not read from this start on: {held}. This start writes the first stamp of the job on {full}, and from it on the tracker is not read for {issue}. The questions are asked again by the next round of their role, and are answered with wb answer {issue}.",
        issue = crate::issue::shown(issue),
        held = held.join(", "),
        full = crate::stamps::full(issue)
    ))
}

/// One question of the issue's file: its number and the line of what is unclear, then the rest as it was written.
struct Question {
    number: u64,
    short: String,
    rest: String,
}

/// A question of the issue's file of `issue`: its first line `<issue>.q<n>: <what is unclear>`, then the rest.
fn question_of(issue: &str, text: &str) -> Result<Question, String> {
    question_led(&format!("{}.", issue), "of the issue's file", text)
}

/// A question whose first line is `<lead>q<n>: <what is unclear>`, then the rest; `whose` says where it was read.
fn question_led(lead: &str, whose: &str, text: &str) -> Result<Question, String> {
    let (first, rest) = text.split_once('\n').unwrap_or((text, ""));
    let named = first.strip_prefix(lead).and_then(|after| after.strip_prefix('q')).and_then(|after| after.split_once(':'));
    let Some((digits, short)) = named.filter(|(digits, _)| !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())) else {
        return Err(format!("the question {} {} does not start with its name, `{}q<n>:`", crate::issue::shown(first), whose, lead));
    };
    let number = digits.parse().map_err(|_| format!("the number of the question {} is too big", crate::issue::shown(first)))?;
    // What the session wrote goes to the terminal and the tracker: never a character a prompt never holds (the review
    // of #297).
    if text.chars().any(|c| crate::issue::is_unsafe(c, true)) {
        return Err(format!(
            "the question {} {} holds a control character, a line separator or a bidirectional control: {}",
            crate::issue::shown(first),
            whose,
            crate::issue::shown(text)
        ));
    }
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

/// The issue's file of `issue` in `commit` of the checkout at `root`, read through the cap of an issue's file.
fn file_in(root: &str, commit: &str, issue: &str) -> Result<Vec<u8>, String> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    let place = format!("{}/{}.yml", crate::issue::FOLDER, issue);
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args(["--no-replace-objects", "cat-file", "blob", &format!("{}:{}", commit, place)]);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("git could not be run to read {}: {}", place, error))?;
    let mut bytes = Vec::new();
    let read = match child.stdout.take() {
        Some(out) => out.take(crate::issue::MAX_BYTES + 1).read_to_end(&mut bytes).map(|_| ()),
        None => Ok(()),
    };
    if bytes.len() as u64 > crate::issue::MAX_BYTES {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!("{} in the commit {} is over 1 MiB", place, crate::view::short(commit)));
    }
    let status = child.wait().map_err(|error| format!("git could not be waited for: {}", error))?;
    read.map_err(|error| format!("{} could not be read: {}", place, error))?;
    if !status.success() {
        return Err(format!("{} is not a file of the commit {} taken back", place, crate::view::short(commit)));
    }
    Ok(bytes)
}

/// Posts the questions of the issue's file of `issue`, as `commit` holds it, the last commit taken back from the
/// planner's view, that the issue does not hold yet (#295): by `issue_comment`, after it is said on `err`. A file
/// with no new question posts nothing. The error says why nothing was posted; it holds nothing raw.
pub fn post_questions(name: &str, root: &str, issue: &str, commit: &str, signing: &crate::stamps::Signing, err: &mut dyn Write) -> Result<(), String> {
    let path = Path::new(root).join(crate::issue::FOLDER).join(format!("{}.yml", issue));
    let document = crate::yamlfile::load_bytes(&path, file_in(root, commit, issue)?).map_err(|why| crate::issue::shown(&why.to_string()))?;
    let place = [crate::yamlfile::Part::from("questions")];
    let written = if document.has(&place) { document.texts(&place, true).map_err(|why| why.to_string())? } else { Vec::new() };
    if written.is_empty() {
        return Ok(());
    }
    let questions = written.iter().map(|text| question_of(issue, text)).collect::<Result<Vec<_>, _>>()?;
    // What is asked already is what the job's ref holds (#238): the tracker is not read for it.
    let ledger = crate::stamps::read(root, issue)?;
    let asked = answer::asked_names(issue, &starter::counted(&ledger.lines()));
    let new: Vec<Question> = questions.into_iter().filter(|question| !asked.contains(&format!("{}.q{}", issue, question.number))).collect();
    if new.is_empty() {
        return Ok(());
    }
    record_questions(name, root, issue, &comment_of(issue, &new), signing, &ledger, err)
}

/// Writes `comment`, the questions of a session in the form of their copy, as a commit on the job's ref of `issue` after
/// the stamp (#238), then posts the copy by `issue_comment` where it is set up. Where the commit cannot be written, the
/// error holds the questions whole, so that they are not lost with the session's files.
fn record_questions(name: &str, root: &str, issue: &str, comment: &str, signing: &crate::stamps::Signing, ledger: &crate::stamps::Ledger, err: &mut dyn Write) -> Result<(), String> {
    let held_back = |why: String| format!("{}. The questions are not written, and read:\n{}", why, comment);
    if let Some(why) = &ledger.broken {
        return Err(held_back(why.clone()));
    }
    let message = crate::stamps::thread_message(root, issue, comment).map_err(|why| held_back(format!("the questions are not written: {}", why)))?;
    let (commit, _) = crate::stamps::write(name, root, issue, &message, &crate::stamps::Held::Questions, signing, err).map_err(held_back)?;
    match prompt::pattern_of(root, "issue_comment") {
        Err(why) => Err(format!("{}. The questions are the commit {} on {}; no copy of them is posted", why, commit, ledger.full)),
        Ok(None) => {
            let _ = writeln!(err, "{}: watchbill.yml has no `issue_comment`, so no copy of the questions is posted: they are the commit {} on {}.", name, commit, ledger.full);
            Ok(())
        }
        Ok(Some(line)) => {
            let _ = write!(err, "{}: will post on issue {}, by `issue_comment` of watchbill.yml:\n{}", name, issue, comment);
            let said = starter::run(root, "issue_comment", &line.replace("{issue}", issue), Some(comment.as_bytes()))
                .map_err(|why| format!("{}. The questions are the commit {} on {}; their copy may not be posted", why, commit, ledger.full))?;
            let _ = err.write_all(&said);
            Ok(())
        }
    }
}

/// The line of a pattern the questions are posted by, or why there is none.
fn pattern_to_post(root: &str, key: &str) -> Result<String, String> {
    match prompt::pattern_of(root, key) {
        Ok(Some(line)) => Ok(line),
        Ok(None) => Err(format!("watchbill.yml has no `{}`. Add the line (see the README)", key)),
        Err(why) => Err(why.to_string()),
    }
}

// ---- #153: every other role reads and writes without a login, as the planner does ----

/// The roles `wb` brings the issue in for, and posts the questions of, through the folder of its own (#153). The
/// planner has its files in its view (#295).
pub const BROUGHT_FOR: [&str; 4] = ["tester", "build", "review", "fix"];

/// The cap of a file a session writes for `wb` to post: as the description's (#151).
pub const WRITTEN_CAP: u64 = crate::push::DESCRIPTION_CAP;

/// What a session of a role is about, as `wb start` worked it out.
pub struct Session<'a> {
    pub role: &'a str,
    pub issue: Option<&'a str>,
    pub change: Option<&'a str>,
    pub branch: Option<&'a str>,
    /// The paths of `changes` and the role's `reads`, or none where there is no issue's file.
    pub paths: Option<&'a [String]>,
}

impl Session<'_> {
    /// Where the role's comments stand: the issue, or the change where the issue is none. The word that names it, and
    /// the patterns that read the whole comments, read their first lines, and post one.
    fn on(&self) -> Option<On<'_>> {
        match (self.issue, self.change) {
            (Some(issue), _) => Some(On { word: "issue", value: issue, bodies: "issue_comment_bodies", lines: "issue_comments", post: "issue_comment" }),
            (None, Some(change)) => Some(On { word: "change", value: change, bodies: "change_comment_bodies", lines: "change_comments", post: "change_comment" }),
            _ => None,
        }
    }

    /// The first words of the role's names: `<on>` for the planner, which keeps the names #95 gave, else `<on>.<role>`.
    fn prefix(&self, on: &On) -> String {
        if self.role == "planner" {
            on.value.to_string()
        } else {
            format!("{}.{}", on.value, self.role)
        }
    }
}

/// Where a role's comments stand, and the patterns of it.
struct On<'a> {
    word: &'static str,
    value: &'a str,
    bodies: &'static str,
    lines: &'static str,
    post: &'static str,
}

/// The line of the pattern `key`, for a session of `role`, or why there is none.
fn needed(root: &str, key: &str, role: &str) -> Result<String, String> {
    match prompt::pattern_of(root, key) {
        Ok(Some(line)) => Ok(line),
        Ok(None) if key == "issue_text" || key == "change_text" => Err(format!(
            "watchbill.yml has no `{}`, and the {}'s session has no login: wb reads it for the session. `wb upgrade` adds the line `{}`; or add it by hand (see the README)",
            key, role, key
        )),
        Ok(None) => Err(format!("watchbill.yml has no `{}`, and the {}'s session has no login: wb reads it for the session. Add the line (see the README)", key, role)),
        Err(why) => Err(why.to_string()),
    }
}

/// Runs the pattern `key` with `{word}` filled with `value`, as the person, through the cap of a text file.
fn read_by(root: &str, key: &str, role: &str, word: &str, value: &str) -> Result<Vec<u8>, String> {
    let line = needed(root, key, role)?;
    starter::run_capped(root, key, &line.replace(&format!("{{{}}}", word), value), bounded::TEXT_CAP)
}

/// The comments of `bodies` whose first line `keep` keeps, whole and in order, each ended by a blank line.
fn kept_of(bodies: &[String], keep: &dyn Fn(&str) -> bool) -> String {
    let kept: Vec<&str> = bodies.iter().filter(|body| keep(body.split('\n').next().unwrap_or_default().trim_end_matches('\r'))).map(|body| body.trim_end()).collect();
    let mut text = kept.join("\n\n");
    if !text.is_empty() {
        text.push('\n');
    }
    text
}

/// The diff of `branch` against its merge base with the default branch, limited to `paths` where they are given,
/// through the cap of a text file. Run with git at the root of the checkout, as the person; nothing of the network.
/// Its first line is `wb`'s own: the ref and the commit the diff ends at, and its base, so that a branch this checkout
/// holds behind the server's can be seen. An empty list of paths gives that line alone: git is not asked for a diff,
/// as `git diff` with no path after `--` is the diff of every path.
fn diff_of(root: &str, branch: Option<&str>, paths: Option<&[String]>) -> Result<Vec<u8>, String> {
    let Some(branch) = branch else {
        return Err("the branch of the change is not known, and wb brings in its diff".to_string());
    };
    let Some(default) = crate::status::default_branch(root) else {
        return Err("there is no default branch to take the diff of the change against".to_string());
    };
    let git = |args: &[&str]| crate::status::git(root, args);
    let tip = [format!("refs/heads/{}", branch), format!("refs/remotes/origin/{}", branch)]
        .into_iter()
        .find(|full| git(&["rev-parse", "--verify", "--quiet", &format!("{}^{{commit}}", full)]).is_some())
        .ok_or_else(|| format!("the branch {} is not there, and wb brings in the diff of the change", branch))?;
    let base = git(&["merge-base", &default, &tip]).ok_or_else(|| format!("the branch {} has no merge base with {}, and wb brings in the diff of the change", branch, default))?;
    let base = String::from_utf8_lossy(&base).trim().to_string();
    let at = git(&["rev-parse", "--verify", "--quiet", &format!("{}^{{commit}}", tip)]).ok_or_else(|| format!("the branch {} is not there, and wb brings in the diff of the change", branch))?;
    let at = String::from_utf8_lossy(&at).trim().to_string();
    let mut bytes = format!("wb: the diff of {} at {} against {}, its merge base with {}, as this checkout holds them; nothing is fetched\n", tip, at, base, default).into_bytes();
    if paths.is_some_and(|paths| paths.is_empty()) {
        return Ok(bytes);
    }
    let mut args: Vec<String> =
        ["--no-replace-objects", "diff", "--no-ext-diff", "--no-textconv", "--submodule=short", "--no-color", "--no-renames", &base, &at].iter().map(|word| word.to_string()).collect();
    if let Some(paths) = paths {
        args.push("--".to_string());
        args.extend(paths.iter().map(|path| format!(":(literal){}", path)));
    }
    let cap = bounded::TEXT_CAP.saturating_sub(bytes.len() as u64);
    bytes.extend(git_capped(root, &args, cap, "the diff of the change")?);
    Ok(bytes)
}

/// What git prints for `args` at `root`, with no `GIT_*` of this process, through `cap`. The error names `what`.
fn git_capped(root: &str, args: &[String], cap: u64, what: &str) -> Result<Vec<u8>, String> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    let mut command = Command::new("git");
    command.arg("-C").arg(root).args(args);
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("GIT_") {
            command.env_remove(key);
        }
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("git could not be run to make {}: {}", what, error))?;
    let mut bytes = Vec::new();
    let read = match child.stdout.take() {
        Some(out) => out.take(cap + 1).read_to_end(&mut bytes).map(|_| ()),
        None => Ok(()),
    };
    if bytes.len() as u64 > cap {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!("{} is more than 1 MiB, and no more of it is read", what));
    }
    let status = child.wait().map_err(|error| format!("git could not be waited for: {}", error))?;
    read.map_err(|error| format!("{} could not be read: {}", what, error))?;
    if !status.success() {
        return Err(format!("git did not make {}", what));
    }
    Ok(bytes)
}

/// Reads, as the person and outside the sandbox, what a session of the tester, the build, the review or the fix
/// reads (#153), and writes it into the new folder `in` of `out`, which the sandbox holds read-only at
/// `view::BROUGHT`: `issue.text`, what `issue_text` prints; `questions`, the role's own questions and answers; and for
/// the review and the fix `change.text`, what `change_text` prints, `change.comments`, the comments of the review and
/// the fix, and `change.diff`. Everything is read before anything is written. Refused, with nothing written, where a
/// pattern is not there, fails, or prints more than the cap, or the diff cannot be had.
pub fn bring_in_for(root: &str, session: &Session, out: &Path) -> Result<Option<String>, String> {
    let role = session.role;
    let mut files: Vec<(&str, Vec<u8>)> = Vec::new();
    let mut notice = None;
    if let Some(issue) = session.issue {
        files.push(("issue.text", read_by(root, "issue_text", role, "issue", issue)?));
    }
    let change_bodies = match (crate::prompt::ALONE.contains(&role), session.change) {
        (true, Some(change)) => {
            files.push(("change.text", read_by(root, "change_text", role, "change", change)?));
            Some(answer::split_bodies(&read_by(root, "change_comment_bodies", role, "change", change)?))
        }
        (true, None) => return Err(format!("the change of the {} is not known, and wb brings in its description", role)),
        _ => None,
    };
    if let Some(on) = session.on() {
        // Where the job's ref holds a stamp or a questions commit, the questions and answers are those of the ref, and the
        // tracker is not read for them (#238, #333).
        let on_ref = match (on.word, session.issue) {
            ("issue", Some(issue)) => Some(crate::stamps::read(root, issue)?).filter(|ledger| ledger.on_ref()),
            _ => None,
        };
        let bodies = match (&on_ref, &change_bodies, on.word) {
            (Some(ledger), _, _) => ledger.thread(),
            (None, Some(bodies), "change") => bodies.clone(),
            _ => answer::split_bodies(&read_by(root, on.bodies, role, on.word, on.value)?),
        };
        // An issue with no stamp on its ref yet is read from the tracker for the last time: what it holds is said (#333).
        if on_ref.is_none() && on.word == "issue" {
            notice = tracker_notice(on.value, &bodies);
        }
        let prefix = session.prefix(&on);
        files.push(("questions", kept_of(&bodies, &|first| asked_or_answered_by(on.value, &prefix, first)).into_bytes()));
        if let Some(bodies) = &change_bodies {
            let ours = |first: &str| ["[review]", "[fix]"].iter().any(|kind| first.starts_with(&format!("{} {} round", kind, on.value)));
            files.push(("change.comments", kept_of(bodies, &ours).into_bytes()));
            files.push(("change.diff", diff_of(root, session.branch, session.paths)?));
        }
    }
    let folder = out.join(crate::view::BROUGHT_FOLDER);
    fs::DirBuilder::new().mode(0o700).create(&folder).map_err(|error| format!("the folder {} could not be made: {}", folder.display(), error))?;
    for (name, bytes) in files {
        write_new(&folder.join(name), &bytes, "the folder of what wb brings in")?;
    }
    Ok(notice)
}

/// What a session wrote for `wb` to post, each read before the folder goes: none where the file is not there or
/// holds only white space.
pub struct Written {
    pub questions: Result<Option<String>, String>,
    pub change_comment: Result<Option<String>, String>,
    pub needs_person: Result<Option<String>, String>,
}

/// Reads the files a session wrote in `out` for `wb` to post (#153).
pub fn read_written(out: &Path) -> Written {
    Written {
        questions: read_one(out, crate::view::QUESTIONS),
        change_comment: read_one(out, crate::view::CHANGE_COMMENT),
        needs_person: read_one(out, crate::view::NEEDS_PERSON),
    }
}

/// The file at `place` in the sandbox, read from `out`, as the description is (#151): a plain file of the cap, never
/// through a link, UTF-8, with no character `issue::is_unsafe` refuses. The error says why, and holds nothing of it.
fn read_one(out: &Path, place: &str) -> Result<Option<String>, String> {
    let name = Path::new(place).file_name().unwrap_or_default();
    let bytes = match bounded::read_plain(&out.join(name), WRITTEN_CAP) {
        Ok(bytes) => bytes,
        Err(bounded::Fault::NotFound) => return Ok(None),
        Err(fault) => return Err(format!("the file {} was not read: {}", place, fault)),
    };
    let text = String::from_utf8(bytes).map_err(|_| format!("the file {} is not UTF-8 text", place))?;
    if text.chars().any(|c| crate::issue::is_unsafe(c, true)) {
        return Err(format!("the file {} holds a control character, a line separator or a bidirectional control", place));
    }
    Ok(Some(text).filter(|text| !text.trim().is_empty()))
}

/// The end of a session, as the posting of what it wrote sees it.
pub struct Ended<'a> {
    pub name: &'a str,
    pub root: &'a str,
    pub session: Session<'a>,
    pub round: Option<&'a str>,
    /// Whether the session ended with 0.
    pub ended_well: bool,
    /// Why the fix's answers are held back, or none: only where a commit of the session came back and was not pushed,
    /// as the answers would name commits the change does not hold.
    pub held: Option<String>,
    /// How a commit on the job's ref is signed, for the questions of an issue (#238); none for a change with no issue.
    pub signing: Option<&'a crate::stamps::Signing>,
}

/// Posts what a session wrote for `wb` (#153), each after it is said: the new questions of `{questions}` for the
/// tester, the build, the review and the fix; `{change_comment}` of the review and the fix on the change; and
/// `{needs_person}` of any role. The status it adds: 1 where anything could not be posted, said as a limit.
pub fn post_written(ended: &Ended, written: Written, err: &mut dyn Write) -> i32 {
    let mut code = 0;
    let mut limit = |err: &mut dyn Write, why: String| {
        code = crate::refusal::limit(err, ended.name, &why);
    };
    let role = ended.session.role;
    if BROUGHT_FOR.contains(&role) {
        match written.questions {
            Ok(None) => {}
            Ok(Some(text)) => {
                if let Err(why) = post_questions_of(ended, &text, err) {
                    limit(err, format!("the questions of {} are not posted: {}", crate::view::QUESTIONS, why));
                }
            }
            Err(why) => limit(err, format!("the questions are not posted: {}", why)),
        }
    }
    if crate::prompt::ALONE.contains(&role) {
        match written.change_comment {
            Ok(None) => {}
            Ok(Some(text)) => match post_change_comment(ended, &text, err) {
                Ok(()) => {}
                Err(Held::Back(why)) => {
                    let _ = write!(err, "{}: the comment of the {} in {} is not posted: {}. It reads:\n{}", ended.name, role, crate::view::CHANGE_COMMENT, why, with_end(&text));
                }
                Err(Held::Failed(why)) => limit(err, format!("the comment of the {} in {} is not posted: {}", role, crate::view::CHANGE_COMMENT, why)),
            },
            Err(why) => limit(err, format!("the comment on the change is not posted: {}", why)),
        }
    }
    match written.needs_person {
        Ok(None) => {}
        Ok(Some(text)) if says_none(&text) => {}
        Ok(Some(text)) => {
            if let Err(why) = post_needs_person(ended, &text, err) {
                limit(err, format!("what {} holds is not posted: {}", crate::view::NEEDS_PERSON, why));
            }
        }
        Err(why) => limit(err, format!("needs_person is not posted: {}", why)),
    }
    code
}

/// Whether `text` says there is nothing for a person: `none` in any case, trimmed of white space, backticks and a final
/// full stop.
fn says_none(text: &str) -> bool {
    let trimmed = |text: &str| text.trim_matches(|c: char| c.is_whitespace() || c == '`').to_string();
    let text = trimmed(text);
    trimmed(text.strip_suffix('.').unwrap_or(&text)).eq_ignore_ascii_case("none")
}

/// `text` with a line break at its end.
fn with_end(text: &str) -> String {
    let mut text = text.trim_end().to_string();
    text.push('\n');
    text
}

/// Says `comment` and posts it on `on` by its pattern, as the person.
fn say_and_post(ended: &Ended, on: &On, key: &str, comment: &str, err: &mut dyn Write) -> Result<(), String> {
    let line = pattern_to_post(ended.root, key)?;
    let _ = write!(err, "{}: will post on {} {}, by `{}` of watchbill.yml:\n{}", ended.name, on.word, on.value, key, comment);
    let said = starter::run(ended.root, key, &line.replace(&format!("{{{}}}", on.word), on.value), Some(comment.as_bytes()))?;
    let _ = err.write_all(&said);
    Ok(())
}

/// Posts the questions of `text` that `on` does not hold yet, in one comment, as #295 posts the planner's.
fn post_questions_of(ended: &Ended, text: &str, err: &mut dyn Write) -> Result<(), String> {
    let on = ended.session.on().ok_or("there is no issue and no change to post them on")?;
    let prefix = ended.session.prefix(&on);
    let questions = split_questions(text)?.iter().map(|one| question_led("", &format!("in {}", crate::view::QUESTIONS), one)).collect::<Result<Vec<_>, _>>()?;
    // The questions of an issue are commits on its job's ref (#238), and what is asked already is what the ref holds; the
    // questions of a change with no issue are comments, read from the tracker as before.
    if let (Some(signing), "issue") = (ended.signing, on.word) {
        let ledger = crate::stamps::read(ended.root, on.value)?;
        let asked = answer::asked_names(on.value, &starter::counted(&ledger.lines()));
        let new: Vec<Question> = questions.into_iter().filter(|question| !asked.contains(&format!("{}.q{}", prefix, question.number))).collect();
        if new.is_empty() {
            return Ok(());
        }
        return record_questions(ended.name, ended.root, on.value, &comment_of(&prefix, &new), signing, &ledger, err);
    }
    let read = pattern_to_post(ended.root, on.lines)?;
    let printed = starter::run_capped(ended.root, on.lines, &read.replace(&format!("{{{}}}", on.word), on.value), bounded::TEXT_CAP)?;
    let lines = starter::counted(&String::from_utf8_lossy(&printed));
    let asked = answer::asked_names(on.value, &lines);
    let new: Vec<Question> = questions.into_iter().filter(|question| !asked.contains(&format!("{}.q{}", prefix, question.number))).collect();
    if new.is_empty() {
        return Ok(());
    }
    say_and_post(ended, &on, on.post, &comment_of(&prefix, &new), err)
}

/// The questions of a file: each from a line `q<n>: …` to the next. Text before the first is refused.
fn split_questions(text: &str) -> Result<Vec<String>, String> {
    let starts = |line: &str| line.strip_prefix('q').and_then(|rest| rest.split_once(':')).is_some_and(|(digits, _)| !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()));
    let mut found: Vec<String> = Vec::new();
    for line in text.split('\n') {
        if starts(line) {
            found.push(String::new());
        }
        match found.last_mut() {
            Some(current) => {
                current.push_str(line);
                current.push('\n');
            }
            None if line.trim().is_empty() => {}
            None => return Err(format!("{} does not start with a question, `q<n>: <what is unclear>`, but with {}", crate::view::QUESTIONS, crate::issue::shown(line))),
        }
    }
    Ok(found)
}

/// Why the comment on the change is not posted: held back, as the person may post it, or failed.
enum Held {
    Back(String),
    Failed(String),
}

/// Posts the review's findings or the fix's answers on the change, under a first line of `wb`'s own.
fn post_change_comment(ended: &Ended, text: &str, err: &mut dyn Write) -> Result<(), Held> {
    let role = ended.session.role;
    if !ended.ended_well {
        return Err(Held::Back("the session did not end with 0".to_string()));
    }
    if let (true, Some(why)) = (role == "fix", &ended.held) {
        return Err(Held::Back(why.clone()));
    }
    let Some(change) = ended.session.change else {
        return Err(Held::Back("the change is not known".to_string()));
    };
    let on = ended.session.on().map(|on| on.value).unwrap_or(change);
    let round = ended.round.unwrap_or("1");
    let comment = format!("[{}] {} round {}\n{}", role, on, round, with_end(text));
    let change_on = On { word: "change", value: change, bodies: "change_comment_bodies", lines: "change_comments", post: "change_comment" };
    say_and_post(ended, &change_on, "change_comment", &comment, err).map_err(Held::Failed)
}

/// Posts what only a person can settle on the issue, or on the change where the issue is none, under a first line
/// of `wb`'s own.
fn post_needs_person(ended: &Ended, text: &str, err: &mut dyn Write) -> Result<(), String> {
    let on = ended.session.on().ok_or("there is no issue and no change to post it on")?;
    let comment = format!("[needs_person] {}\n{}", ended.session.prefix(&on), with_end(text));
    say_and_post(ended, &on, on.post, &comment, err)
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use super::*;

    #[test]
    fn test_WB_BUILD_136_only_the_planner_s_questions_and_answers_of_the_issue_are_kept() {
        for first in ["[questions] 54 q1", "[answers] 54 q1>planner", "[question] 54.q3", "[answer] 54.q3 2", "[answer] 54 suggested"] {
            assert!(asked_or_answered("54", first), "{}", first);
        }
        for first in ["[stamp] planner round 1", "[questions] 54.tester q1", "[answers] 541 q1", "[question] 54.tester.q1", "[questions] 5 q1", "Thanks"] {
            assert!(!asked_or_answered("54", first), "{}", first);
        }
    }
}
