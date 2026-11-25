//! `wb init`: sets Watchbill up in a repository, the way `git init` starts one.
//!
//! It proposes what the repository says about itself, asks one line per answer,
//! shows the files and writes them: `.watchbill/watchbill.yml`, and the revision the package carries,
//! its files under `.watchbill/revision/` and its manifest as `.watchbill/revision.sha256`
//! (docs/revision.md, point 7). A file that is already there is never changed, and when one is
//! there nothing at all is written.

use std::fs;
use std::io::{self, BufRead, ErrorKind, Write};
use std::path::{Path, PathBuf};

use crate::detect::{detect, origin_host, strip, universal_newlines, Setup, NONE};
use crate::interrupt;
use crate::manifest;

pub const CONFIG_FOLDER: &str = ".watchbill";
pub const CONFIG_FILE: &str = "watchbill.yml";
const REVISION_FOLDER: &str = "revision";

const QUESTIONS: [(&str, &str); 6] = [
    ("project", "project"),
    ("git_server", "git server (github, gitlab, gerrit, bitbucket or none)"),
    ("repository", "repository at the git server"),
    ("tracker", "tracker (github, gitlab, jira or none)"),
    ("requirements", "requirements (a path to files, the name of a tool, or none)"),
    ("ci", "ci driver (github-actions, gitlab-ci, jenkins or none)"),
];

/// A project name is lower-case letters, digits, `.`, `_` and `-`, and starts with a letter or a digit.
fn is_project_name(answer: &str) -> bool {
    let mut chars = answer.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "._-".contains(c))
}

/// Why an answer cannot be taken, or `None` when it can.
fn fault(key: &str, answer: &str) -> Option<&'static str> {
    if answer.chars().count() > 200 {
        return Some("that is over 200 characters");
    }
    // A control character would break the line of the file.
    if answer.chars().any(|c| c <= '\x1f' || c == '\x7f') {
        return Some("that holds a character that cannot be written");
    }
    if key == "project" && !is_project_name(answer) {
        return Some(
            "a project name is lower-case letters, digits, '.', '_' and '-', and starts with a letter or a digit",
        );
    }
    None
}

/// A word that starts with a letter, and is not one of the words YAML reads as a truth value or as
/// nothing: plain is sure to be read back as the same text.
fn is_plain_word(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || "_./*-".contains(c))
        && !["y", "n", "yes", "no", "true", "false", "on", "off", "null"].contains(&value.to_ascii_lowercase().as_str())
}

/// A version of three numbers or more, which no YAML number or date has the form of.
fn is_version(value: &str) -> bool {
    let digits = |text: &str| text.find(|c: char| !c.is_ascii_digit()).unwrap_or(text.len());
    let mut rest = value;
    for _ in 0..2 {
        let n = digits(rest);
        if n == 0 || !rest[n..].starts_with('.') {
            return false;
        }
        rest = &rest[n + 1..];
    }
    digits(rest) > 0 && rest.chars().all(|c| c.is_ascii_alphanumeric() || "._+-".contains(c))
}

/// A value as YAML reads it back unchanged: plain when that is certain, else in single quotes.
pub fn yaml_value(value: &str) -> String {
    if is_version(value) || is_plain_word(value) {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}

/// The question of the classed paths (#84), asked after the CI driver. What stands in brackets is `[]`.
const CLASSED_QUESTION: &str = "classed code (paths from the root, separated by commas, or [] for none)";

/// The classed paths of an answer, or why it is not taken. `[]`, `none` and nothing are no classed code. The
/// paths are separated by commas, in brackets or not, and each is one that `wb start <role>` takes: from the root of
/// the repository, not starting with `/`, with no `..`, and not the root.
fn classed_answer(answer: &str) -> Result<Vec<String>, &'static str> {
    let inner = answer.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')).unwrap_or(answer).trim();
    if inner.is_empty() || inner == NONE {
        return Ok(Vec::new());
    }
    if answer.chars().count() > 200 {
        return Err("that is over 200 characters");
    }
    if answer.chars().any(|c| c <= '\x1f' || c == '\x7f') {
        return Err("that holds a character that cannot be written");
    }
    let mut paths = Vec::new();
    for path in inner.split(',').map(str::trim) {
        let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty() && *part != ".").collect();
        if path.is_empty() || path.starts_with('/') || parts.is_empty() || parts.contains(&"..") {
            return Err("a classed path is a file or a folder from the root of the repository, not `/` at its start, with no `..`, and not the root; paths are separated by commas");
        }
        paths.push(path.to_string());
    }
    Ok(paths)
}

/// The file `wb init` writes, with no classed code and no file of the build.
pub fn render(revision: &str, hash: &str, setup: &Setup) -> String {
    render_with(revision, hash, setup, &[], &[], "")
}

/// The file `wb init` writes, with the classed paths given as `classed:` after the CI driver (#84), and the
/// files of the build as `build_files:` after them, where there are any (#125).
pub fn render_with(revision: &str, hash: &str, setup: &Setup, classed: &[String], build_files: &[String], host: &str) -> String {
    let rows = [
        ("watchbill", revision),
        ("revision", hash),
        ("project", &setup.project),
        ("git_server", &setup.git_server),
        ("repository", &setup.repository),
        ("tracker", &setup.tracker),
        ("requirements", &setup.requirements),
        ("ci", &setup.ci),
    ];
    let mut text = String::from(
        "# How Watchbill is set up in this repository. Written by `wb init`.\n\
         # `watchbill` is the version of the package the repository is set up for.\n",
    );
    for (key, value) in rows {
        text.push_str(&format!("{}: {}\n", key, yaml_value(value)));
    }
    // Which paths are classed (#84): a person says it here. With no line, `wb start <role>` takes every path as classed.
    let listed: Vec<String> = classed.iter().map(|path| yaml_value(path)).collect();
    text.push_str(&format!("classed: [{}]\n", listed.join(", ")));
    // The files of the build (#125): `wb accept` refuses an issue's file whose `changes` holds one. The CI
    // files found are proposed; a person adds the rest of the build, such as `Makefile`.
    if !build_files.is_empty() {
        let listed: Vec<String> = build_files.iter().map(|path| yaml_value(path)).collect();
        text.push_str(&format!("build_files: [{}]\n", listed.join(", ")));
    }
    // The patterns `wb prompt` fills (#72). The addresses are known for GitHub only, and only from a
    // repository that is plain text; the branch does not depend on the server.
    let plain = !setup.repository.is_empty()
        && setup.repository != NONE
        && setup.repository.chars().all(|c| c.is_ascii_alphanumeric() || "._-/".contains(c));
    if setup.git_server == "github" && plain {
        // The patterns of the starter (#84): post a comment from standard input, print the first line of each
        // comment, and print the numbers of the open pull requests from a branch. A pull request's comments
        // are those of an issue at GitHub, so `change_comments` reads them the same way.
        let repository = &setup.repository;
        let first_lines = |word: &str| {
            format!("gh api repos/{}/issues/{{{}}}/comments --paginate --jq '.[].body | split(\"\\n\")[0]'", repository, word)
        };
        let whole = |word: &str| format!("gh api repos/{}/issues/{{{}}}/comments --paginate --jq '.[] | .body + \"\\u0000\"'", repository, word);
        let patterns = [
            ("issue_comment", format!("gh issue comment {{issue}} --repo {} --body-file -", repository)),
            ("issue_comments", first_lines("issue")),
            ("change_comment", format!("gh pr comment {{change}} --repo {} --body-file -", repository)),
            ("change_comments", first_lines("change")),
            ("change_of_branch", format!("gh pr list --repo {} --head {{branch}} --state open --json number --jq '.[].number'", repository)),
            // The CI of a change (#127), as one word: `red` where a check failed or was cancelled, `pending` where one has
            // not ended, has a bucket that is not known, or none is reported, else `green`: only `pass` and `skipping` are. `gh` ends with 1 where a pull request has no checks.
            (
                "change_checks",
                format!(
                    "gh pr checks {{change}} --repo {} --json bucket --jq '[.[].bucket] | if length == 0 then \"pending\" elif any(. == \"fail\" or . == \"cancel\") then \"red\" elif all(. == \"pass\" or . == \"skipping\") then \"green\" else \"pending\" end'",
                    repository
                ),
            ),
            // The whole comments, for `wb answer` (#145): each body and a NUL byte. `gh --jq` prints a line break
            // after each, which `wb` does not read as part of the next body.
            ("issue_comment_bodies", whole("issue")),
            ("change_comment_bodies", whole("change")),
            // The change's description alone, which `wb start` brings in for the review and the fix (#153).
            ("change_text", format!("gh pr view {{change}} --repo {} --json body --jq .body", repository)),
            // The command that opens the change as a draft, which `wb start build` says and a person runs (#145). The
            // description is filled from the commits: the build writes it in the message of its last commit.
            ("draft_change", format!("gh pr create --repo {} --head {{branch}} --draft --fill --body-file -", repository)),
        ];
        for (key, line) in patterns {
            text.push_str(&format!("{}: {}\n", key, yaml_value(&line)));
        }
        let address = |command: &str, word: &str| {
            // One command shows the text or the description, the other the comments: `--comments` alone hides the first (#96).
            let show = format!("gh {} view {{{}}} --repo {}", command, word, setup.repository);
            format!("{}; {} --comments", show, show)
        };
        text.push_str(&format!("issue_address: {}\n", yaml_value(&address("issue", "issue"))));
        // The issue's text alone, which `wb start planner` brings into its view (#295): nothing is cut from it.
        text.push_str(&format!("issue_text: {}\n", yaml_value(&format!("gh issue view {{issue}} --repo {} --json body --jq .body", setup.repository))));
        text.push_str(&format!("change_address: {}\n", yaml_value(&address("pr", "change"))));
    }
    if setup.git_server == "gitlab" && plain && !host.is_empty() {
        // The same patterns for GitLab (#123, #55), with `glab`. They name the host of `origin`: without it
        // `glab` asks gitlab.com. The comments are posted and read through `glab api`, which reads a whole
        // text from standard input (`-F body=@-`); `glab issue note` would prompt. `glab api` has no `--jq`,
        // so the comments are filtered by `jq`, which keeps the notes a person or an account wrote and no
        // system note. The project is in the path of the API as its repository with `/` written `%2F`.
        let repository = &setup.repository;
        let api = format!("glab api --hostname {}", host);
        let project = format!("projects/{}", repository.replace('/', "%2F"));
        let url = format!("https://{}/{}", host, repository);
        let notes = |kind: &str, word: &str| format!("{}/{}/{{{}}}/notes", project, kind, word);
        let first_lines = |kind: &str, word: &str| {
            format!(
                "{} {} --paginate --output ndjson | jq -r 'select(.system == false) | .body | split(\"\\n\")[0]'",
                api,
                notes(kind, word)
            )
        };
        let whole = |kind: &str, word: &str| {
            format!(
                "{} '{}?sort=asc&order_by=created_at' --paginate --output ndjson | jq -j 'select(.system == false) | .body + \"\\u0000\"'",
                api,
                notes(kind, word)
            )
        };
        let patterns = [
            ("issue_comment", format!("{} --method POST {} -F body=@-", api, notes("issues", "issue"))),
            ("issue_comments", first_lines("issues", "issue")),
            ("change_comment", format!("{} --method POST {} -F body=@-", api, notes("merge_requests", "change"))),
            ("change_comments", first_lines("merge_requests", "change")),
            ("change_of_branch", format!("glab mr list --repo {} --source-branch {{branch}} -F json --jq '.[].iid'", url)),
            // The CI of a merge request (#127), as one word: the status of its head pipeline. Only `success` is green; a
            // merge request with no pipeline, or one that is running, waiting or blocked, is pending.
            (
                "change_checks",
                format!(
                    "{} {}/merge_requests/{{change}} | jq -r 'if (.iid | type) != \"number\" then error(\"not a merge request\") else (.head_pipeline.status // \"none\") as $s | if $s == \"success\" then \"green\" elif $s == \"failed\" or $s == \"canceled\" then \"red\" else \"pending\" end end'",
                    api, project
                ),
            ),
            ("issue_address", format!("glab issue view {{issue}} --repo {} --comments", url)),
            // The issue's text alone, for the planner (#295): the description of the issue, as `glab` gives it in JSON.
            ("issue_text", format!("glab issue view {{issue}} --repo {} -F json | jq -r .description", url)),
            ("change_address", format!("glab mr view {{change}} --repo {} --comments", url)),
            // The whole comments, for `wb answer` (#145): each body of a note that is no system note, and a NUL
            // byte, oldest first. The query is quoted: it holds `&`.
            ("issue_comment_bodies", whole("issues", "issue")),
            ("change_comment_bodies", whole("merge_requests", "change")),
            // The change's description alone, for the review and the fix (#153), as `glab` gives it in JSON.
            ("change_text", format!("glab mr view {{change}} --repo {} -F json | jq -r .description", url)),
            // The command that opens the change as a draft (#145, #151). `--fill` gives the title from the commits;
            // `--push=false` keeps `glab` from pushing, which `--fill` does by default: `wb`'s push is the only push.
            // A given `--description` is kept over the one `--fill` would make.
            ("draft_change", format!("glab mr create --repo {} --source-branch {{branch}} --draft --fill --push=false --yes --description \"$(cat)\"", url)),
        ];
        for (key, line) in patterns {
            text.push_str(&format!("{}: {}\n", key, yaml_value(&line)));
        }
    }
    text.push_str(&format!("branch: {}\n", yaml_value("issue-{issue}")));
    text
}

/// What is said, with 1, when reading the answers failed: an answer that is not UTF-8 text, or an
/// error of reading, which is told as it is.
fn read_failure(error: &io::Error) -> String {
    if error.kind() == ErrorKind::InvalidData {
        "what was typed is not UTF-8 text. Nothing written.".to_string()
    } else {
        format!("cannot read standard input: {}. Nothing written.", error)
    }
}

/// The revision of the package, as it is to be written: each file with its bytes, the manifest and
/// its hash. Paths are relative to `.watchbill/`.
pub(crate) struct Pin {
    pub(crate) files: Vec<(String, Vec<u8>)>,
    pub(crate) hash: String,
}

/// Reads the revision from the package. Each file is read once, and what was read must be what the
/// manifest says of it, so that what is written is what is hashed.
pub(crate) fn read_pin(package: &Path) -> Result<Pin, String> {
    let manifest = manifest::manifest(package).map_err(|why| why.to_string())?;
    let mut files = Vec::new();
    for line in manifest.lines() {
        let (hash, path) = line.split_once("  ").ok_or("the manifest is not in the form of sha256sum")?;
        let bytes = fs::read(package.join(path)).map_err(|error| format!("{}: cannot be read ({})", path, error))?;
        let same = std::str::from_utf8(&bytes).map(|text| manifest::hash_of(text) == hash).unwrap_or(false);
        if !same {
            return Err(format!("{}: changed while it was read", path));
        }
        files.push((path.to_string(), bytes));
    }
    let hash = manifest::hash_of(&manifest);
    files.push((manifest::MANIFEST_FILE.to_string(), manifest.into_bytes()));
    Ok(Pin { files, hash })
}

/// What is in the way of the files, said as a sentence, or `None`. It is a file that is there
/// already; a folder on the way to a file that is a link or not a folder (a link would take the files
/// to where it points); a target that is not below the folder; or `.watchbill/revision` of any kind,
/// so that no other file stays beside the revision (WB-PIN-4, 7, 8 and 10).
fn in_the_way(folder: &str, targets: &[String]) -> Option<String> {
    let said = |path: &str, why: &str| Some(format!("{} {}. wb init does not change it. Nothing written.", path, why));
    for target in targets {
        let mut path = PathBuf::from(folder);
        let below = match Path::new(target).strip_prefix(folder).ok().and_then(|rest| rest.parent()) {
            Some(below) => below,
            None => return said(target, &format!("is not below {}", folder)),
        };
        let mut names = vec![path.clone()];
        for part in below {
            path.push(part);
            names.push(path.clone());
        }
        // The folder above is the one at fault when a path below it cannot be looked at.
        let mut above = &names[0];
        for each in &names {
            match fs::symlink_metadata(each) {
                Ok(meta) if meta.is_dir() => {}
                Err(error) if error.kind() == ErrorKind::NotFound => break,
                Ok(meta) => {
                    let why = if meta.file_type().is_symlink() { "is a link" } else { "is not a folder" };
                    return said(&each.to_string_lossy(), why);
                }
                Err(_) => return said(&above.to_string_lossy(), "cannot be looked at"),
            }
            above = each;
        }
    }
    if let Some(file) = targets.iter().find(|path| fs::symlink_metadata(path).is_ok()) {
        return said(file, "is already there");
    }
    let revision = format!("{}/{}", folder, REVISION_FOLDER);
    match fs::symlink_metadata(&revision) {
        Ok(_) => said(&revision, "is already there"),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(_) => said(folder, "cannot be looked at"),
    }
}

/// The place of the answers: where the questions go and where the lines come from.
struct Talk<'a> {
    input: &'a mut dyn BufRead,
    out: &'a mut dyn Write,
}

impl Talk<'_> {
    fn say(&mut self, text: &str) {
        // A failed write, such as a closed pipe, does not change what the command ended with.
        let _ = writeln!(self.out, "{}", text);
    }

    /// A line that only informs, after the files are written: `-q` leaves it out (#285).
    fn inform(&mut self, text: &str) {
        crate::refusal::inform(self.out, format_args!("{}", text));
    }

    /// A question and its answer, one line from standard input: `Ok(None)` when there are no more.
    /// A line that is not UTF-8 is an error of the kind `InvalidData`.
    fn ask(&mut self, prompt: &str) -> io::Result<Option<String>> {
        let _ = write!(self.out, "{}", prompt);
        let _ = self.out.flush();
        let mut line = Vec::new();
        self.input.read_until(b'\n', &mut line)?;
        if line.is_empty() {
            // No more input: end the line of the question, so what follows starts on its own.
            let _ = writeln!(self.out);
            return Ok(None);
        }
        if line.last() == Some(&b'\n') {
            line.pop();
        }
        match String::from_utf8(line) {
            Ok(text) => Ok(Some(text)),
            Err(_) => {
                let _ = writeln!(self.out);
                Err(io::Error::new(ErrorKind::InvalidData, "not UTF-8"))
            }
        }
    }

    fn answers(&mut self, found: &Setup) -> io::Result<Setup> {
        let mut setup = found.clone();
        for (key, text) in QUESTIONS {
            if key == "repository" && setup.git_server == NONE {
                setup.repository = NONE.to_string();
                continue;
            }
            loop {
                let current = match key {
                    "project" => &setup.project,
                    "git_server" => &setup.git_server,
                    "repository" => &setup.repository,
                    "tracker" => &setup.tracker,
                    "requirements" => &setup.requirements,
                    _ => &setup.ci,
                };
                let line = self.ask(&format!("{}: ({}) ", text, current))?;
                let answer = strip(line.as_deref().unwrap_or("")).to_string();
                if answer.is_empty() {
                    break;
                }
                match fault(key, &answer) {
                    None => {
                        match key {
                            "project" => setup.project = answer,
                            "git_server" => setup.git_server = answer,
                            "repository" => setup.repository = answer,
                            "tracker" => setup.tracker = answer,
                            "requirements" => setup.requirements = answer,
                            _ => setup.ci = answer,
                        }
                        break;
                    }
                    Some(why) => self.say(&format!("Not taken: {}.", why)),
                }
            }
        }
        Ok(setup)
    }

    /// The classed paths (#84): asked after the CI driver, with `[]` proposed. An empty line, or input that has
    /// ended, keeps it; an answer that is not taken is said, and the question is asked again.
    fn classed(&mut self) -> io::Result<Vec<String>> {
        loop {
            let line = self.ask(&format!("{}: ([]) ", CLASSED_QUESTION))?;
            match classed_answer(strip(line.as_deref().unwrap_or(""))) {
                Ok(paths) => return Ok(paths),
                Err(why) => self.say(&format!("Not taken: {}.", why)),
            }
        }
    }
}

/// Runs `wb init` in the repository at `root`. Returns the exit status.
pub fn init(
    root: &str,
    revision: &str,
    package: &Path,
    yes: bool,
    input: &mut dyn BufRead,
    out: &mut dyn Write,
    err: &mut dyn Write,
    env: Option<&[(String, String)]>,
) -> i32 {
    let mut talk = Talk { input, out };
    let folder = if root.ends_with('/') { format!("{}{}", root, CONFIG_FOLDER) } else { format!("{}/{}", root, CONFIG_FOLDER) };
    let file = format!("{}/{}", folder, CONFIG_FILE);
    let pin = match read_pin(package) {
        Ok(pin) => pin,
        Err(why) => {
            return crate::refusal::limit(err, "wb init", format_args!("the revision of the package cannot be read: {}. Nothing written.", why));
        }
    };
    // The files, as paths to write: the manifest and the files of the revision, then the config last,
    // so that a repository that holds it is one that was set up whole.
    let mut targets: Vec<String> = pin.files.iter().map(|(path, _)| format!("{}/{}", folder, path)).collect();
    targets.push(format!("{}/project/README.md", folder));
    targets.push(file.clone());
    if let Some(message) = in_the_way(&folder, &targets) {
        return crate::refusal::limit(err, "wb init", message);
    }

    let mut setup = match detect(root, env) {
        Ok(setup) => setup,
        Err(why) => {
            return crate::refusal::limit(err, "wb init", format_args!("{}. Nothing written.", why));
        }
    };
    let host = origin_host(root, env);
    let mut classed = Vec::new();
    if !yes {
        talk.say(&format!("This sets Watchbill {} up in {}.", revision, root));
        talk.say(&format!("It asks a few things and writes {}/{}. What it found in the", CONFIG_FOLDER, CONFIG_FILE));
        talk.say("repository stands in brackets: Enter keeps it. Ctrl-C leaves at any time.");
        talk.say("");
        setup = match talk.answers(&setup) {
            Ok(setup) => setup,
            Err(error) => {
                return crate::refusal::limit(err, "wb init", read_failure(&error));
            }
        };
        classed = match talk.classed() {
            Ok(paths) => paths,
            Err(error) => {
                return crate::refusal::limit(err, "wb init", read_failure(&error));
            }
        };
    }

    let build_files = crate::detect::ci_files(Path::new(root));
    let text = render_with(revision, &pin.hash, &setup, &classed, &build_files, &host);
    talk.say(&format!("\nAbout to write {}:\n\n{}", file, text));
    talk.say("and these files: those of the revision, byte for byte as the package has them, and the README of the project's own text:\n");
    for path in &targets[..targets.len() - 1] {
        talk.say(&format!("  {}", path));
    }
    // The fetch line of the jobs (#234, point 15), where the remote `origin` is there and lacks it.
    let fetch = crate::plan::fetch_missing(root).unwrap_or(false);
    if fetch {
        talk.say(&format!("\nand add {}.", crate::plan::fetch_said()));
    }
    talk.say("");
    if !yes {
        match talk.ask("Is this OK? (yes) ") {
            Ok(None) => {
                // No answer at all, because the input ended, is no yes.
                talk.say("Nothing written.");
                // The person gave no yes: not a refusal.
                crate::refusal::answer();
                return 1;
            }
            Ok(Some(answer)) => {
                if !["", "y", "yes"].contains(&strip(&answer).to_lowercase().as_str()) {
                    talk.say("Nothing written.");
                    // The person answered no: not a refusal.
                    crate::refusal::answer();
                    return 1;
                }
            }
            Err(error) => {
                return crate::refusal::limit(err, "wb init", read_failure(&error));
            }
        }
    }

    // From here an interrupt no longer leaves nothing written.
    interrupt::writing();
    let mut contents: Vec<&[u8]> = pin.files.iter().map(|(_, bytes)| bytes.as_slice()).collect();
    contents.push(crate::prompt::PROJECT_README.as_bytes());
    contents.push(text.as_bytes());
    if let Err((path, error)) = write_all(&targets, &contents) {
        if error.kind() == ErrorKind::AlreadyExists {
            return crate::refusal::limit(err, "wb init", format_args!("{} is already there. wb init does not change it. Nothing written.", path.display()));
        }
        return crate::refusal::limit(err, "wb init", format_args!("cannot write {}: {}. Nothing written.", path.display(), error));
    }
    for path in &targets {
        talk.inform(&format!("Wrote {}.", path));
    }
    talk.say("Keep them under version control with the code.");
    if fetch {
        if let Err(why) = crate::plan::add_fetch(root) {
            return crate::refusal::limit(err, "wb init", format_args!("the fetch line {} could not be added to .git/config: {}. The files are written.", crate::plan::FETCH, why));
        }
        talk.inform(&format!("Added the fetch line {} to the remote origin in .git/config.", crate::plan::FETCH));
    }
    // The project's keys are not written: wb cannot know them (#323). Said with -q too, as what the person must do.
    talk.say(&format!(
        "{} is not written: wb cannot know your key, and until the default branch holds it nothing that needs a signature counts.",
        crate::signers::PATH
    ));
    talk.say("Write it before the first acceptance, one line for each person's SSH key, and commit it on the default branch:");
    talk.say("  <identity> roles=\"accept,start\" <key type> <key>");
    talk.say("with the key as its public key file has it (as ~/.ssh/id_ed25519.pub).");
    0
}

/// Writes every file, the last one last. Nothing is written over a file that is there. When one
/// fails, what this call made is removed again, so a failure leaves no half set-up repository: the
/// error is the path that failed and why.
fn write_all(files: &[String], contents: &[&[u8]]) -> Result<(), (PathBuf, io::Error)> {
    let mut made: Vec<(PathBuf, bool)> = Vec::new();
    let result = (|| {
        for (file, bytes) in files.iter().zip(contents) {
            let file = Path::new(file);
            // The folders above it, from the outside in, each noted when this call made it.
            let mut missing = Vec::new();
            let mut above = file.parent();
            while let Some(path) = above {
                if path.exists() || path.as_os_str().is_empty() {
                    break;
                }
                missing.push(path);
                above = path.parent();
            }
            for path in missing.into_iter().rev() {
                fs::create_dir(path).map_err(|error| (path.to_path_buf(), error))?;
                made.push((path.to_path_buf(), true));
            }
            // `create_new`: never over a file that came to be there meanwhile.
            let mut out = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(file)
                .map_err(|error| (file.to_path_buf(), error))?;
            made.push((file.to_path_buf(), false));
            out.write_all(bytes).map_err(|error| (file.to_path_buf(), error))?;
        }
        Ok(())
    })();
    if result.is_err() {
        for (path, is_folder) in made.into_iter().rev() {
            let _ = if is_folder { fs::remove_dir(&path) } else { fs::remove_file(&path) };
        }
    }
    result
}

/// The root of the git repository the command runs in: `Ok(None)` outside one. An error is a path
/// that is not UTF-8.
pub fn repository_root(env: Option<&[(String, String)]>) -> Result<Option<String>, String> {
    match crate::detect::git_output(&["rev-parse", "--show-toplevel"], env) {
        Some(run) if run.status.success() => match String::from_utf8(run.stdout) {
            Ok(text) => {
                let root = strip(&universal_newlines(&text)).to_string();
                Ok(if root.is_empty() { None } else { Some(root) })
            }
            Err(_) => Err("the path of the repository is not UTF-8 text".to_string()),
        },
        _ => Ok(None),
    }
}

/// The root of the git repository that holds `folder`, as `repository_root` gives it. The variables `GIT_*`
/// of this process are not handed to git, so that `folder` alone says which repository it is.
pub fn repository_root_of(folder: &Path) -> Result<Option<String>, String> {
    let Some(shown) = folder.to_str() else {
        return Err("the path of the folder is not UTF-8 text".to_string());
    };
    let env: Vec<(String, String)> = std::env::vars().filter(|(name, _)| !name.starts_with("GIT_")).collect();
    match crate::detect::git_output(&["-C", shown, "rev-parse", "--show-toplevel"], Some(&env)) {
        Some(run) if run.status.success() => match String::from_utf8(run.stdout) {
            Ok(text) => {
                let root = strip(&universal_newlines(&text)).to_string();
                Ok(if root.is_empty() { None } else { Some(root) })
            }
            Err(_) => Err("the path of the repository is not UTF-8 text".to_string()),
        },
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(non_snake_case)]
    fn test_WB_PIN_10_a_target_outside_the_folder_is_in_the_way_and_named() {
        let found = in_the_way("/nowhere/.watchbill", &["/elsewhere/file.md".to_string()]);

        let message = found.expect("a target outside the folder was taken as nothing in the way");
        assert!(message.contains("/elsewhere/file.md is not below /nowhere/.watchbill"), "{}", message);
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_WB_PIN_7_a_folder_that_cannot_be_looked_at_is_named_by_the_folder_above_it() {
        // A name of 300 bytes cannot be looked at, as root too.
        let folder = std::env::temp_dir().join(format!("wb53-unit-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let name = "a".repeat(300);
        let target = format!("{}/{}/file.md", folder.display(), name);

        let found = in_the_way(folder.to_str().unwrap(), &[target]);

        fs::remove_dir_all(&folder).unwrap();
        let message = found.expect("a folder that cannot be looked at was taken as nothing in the way");
        assert!(message.starts_with(&format!("{} cannot be looked at", folder.display())), "{}", message);
    }
}
