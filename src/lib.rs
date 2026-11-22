//! The command `wb`.

use lexopt::{Arg, Parser};
use std::ffi::OsString;
use std::io::{BufRead, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub mod answer;
pub mod check;
pub mod confidentiality;
pub mod detect;
pub mod init;
pub mod interrupt;
pub mod kept;
pub mod manifest;
pub mod prompt;
pub mod revert;
pub mod rules;
pub mod issue;
pub mod layout;
pub mod sessions;
pub mod starter;
pub mod status;
pub mod upgrade;
pub mod view;
mod word;
pub mod yamlfile;

/// The version of the package.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What `wb --help` prints. The words are those of the Python `wb`.
pub const USAGE: &str = "usage:
  wb init [--yes]
  wb check
  wb upgrade [--dry-run]
  wb accept <issue> [--no-commit]
  wb start <role> [<issue>] [--runner <name>] [--prompt] [--ignore-local] [--ignore-kept] [the options of prompt]
  wb status [<issue>] [--stage <heading>]
  wb view <issue> (<role> [--in | --out] | <path>)
  wb answer (<issue> | --change <n>) [--show | [--suggested] [<question> <answer>]...]
  wb revert (<issue> | --change <n>) <role> [--round <n>]
  wb prompt <role> [<issue>] [--issue <text>] [--change <text>]
            [--requirements <text>] [--round <text>] [--checkout <text>]
            [--branch <text>] [--issue_address <text>]
            [--change_address <text>] [--classed <text>]
            [--classed_touched <text>] [--runner <name>] [--parts] [--stamp]
  wb revision
  wb --version

  init      set Watchbill up in the git repository of this folder: propose what
            the repository says about itself (its name, the git server of
            origin, a folder of requirements, the CI driver), ask one line per
            answer, show the file and write .watchbill/watchbill.yml at the
            root of the repository. --yes (-y) takes what it found and asks
            nothing. A file that is already there is never changed
  check     say whether the repository still is at the revision it is pinned to:
            each file under .watchbill/revision/ against the manifest, the
            manifest against the revision line of watchbill.yml, and that
            revision against the installed wb, and the project text for a
            placeholder the revision does not know. Ends with 0 when all of it
            holds, 1 when something differs. Writes nothing
  upgrade   pin the repository to the installed revision: copy its files over
            .watchbill/revision/, removing those it does not have, and its
            manifest to .watchbill/revision.sha256, set the lines watchbill:
            and revision: of watchbill.yml, and add each pattern the installed
            wb init would write that the file lacks; no other line is changed.
            It says each write first, then writes, runs the check of wb check
            and says the command that commits the change: it commits nothing.
            --dry-run says the same and writes nothing. Ends with 0 when it is
            done and the check holds, or there is nothing to do; 1 when it is
            refused (a change under .watchbill/ that is not committed, an
            installed version older than the pinned one) or the check does not
            hold; 2 when it was used wrongly or the repository is not set up
  prompt    print the prompt of a role on standard output and nothing else: the
            role's text of the pinned revision with each placeholder filled
            from the option of its name, such as --issue for {issue}, then the
            project's own text. The issue is the word after the role, or
            --issue. What no option gives is taken from the repository and
            watchbill.yml: the checkout, the branch, the two addresses and
            the requirements; an option always wins. It runs the check of wb
            check first and prints nothing when that fails. --parts prints
            the path and the SHA-256 of each part, in the form sha256sum
            prints, in place of the text.
            --stamp prints, in place of the text, what the assignment is: the
            role, its model and effort, the runner, the version and revision,
            the issue, the change, the round, the branch, the checkout, the
            time, and each part as --parts gives it. Not with --parts.
            --runner names the runner the prompt is handed to. The breakdown
            needs it, and is handed out only to a runner that
            /etc/watchbill/organisation.yml clears for the breakdown up to the
            level of the repository, the line confidentiality: of
            watchbill.yml. Its stamp then has the model and effort of that
            file, and --parts adds the level and the runner. Ends with 0 when
            the prompt is printed, 1 when it is refused, 2 when it was used
            wrongly or the repository is not set up. Writes nothing
  start     start a session of one role, breakdown, tester, build, review or
            fix: the role is the word after start. The issue is needed by
            all but the review and the fix, which may be given --change
            and no issue. It checks what prompt checks, that a fix
            follows a review, that no classed code is in its way (classed: of
            watchbill.yml), that the runner has start in the organisation's
            file, and that standard input and output are a terminal; then say
            and post the stamp by issue_comment of watchbill.yml (by
            change_comment for a review or a fix of a change with no issue),
            and run the runner's start in the root of the checkout. The runner
            is --runner, or the organisation's default. The round and the
            change are worked out from issue_comments, change_comments and
            change_of_branch; --round and --change win. --prompt, --stamp and
            --parts print as prompt does, and post and start nothing. A view
            of the issue kept with work not taken back, from any role, stops the
            start with 1 before anything is posted; --ignore-kept goes on
            without it, and the stamp says so. Ends with
            the session: 0 when it ended with 0, else 1; 1 when it is refused,
            2 when it was used wrongly or the repository is not set up
  revision  print the version of the package, the hash of its revision and the
            manifest: the hash of every file the revision holds. Writes nothing
  accept    accept the issue's file .watchbill/issues/<issue>.yml: read it,
            hold it to its form, refuse it while it holds a question, say what
            will be written, and write .watchbill/issues/<issue>.accepted with
            the SHA-256 of the file, the git user.name and user.email, and the
            time in UTC. An acceptance that is there is never written over.
            Then commit it alone, on the issue's branch, with nothing else:
            it refuses first when the checkout is not on that branch or the
            issue's file is not committed as it is. --no-commit only writes
            it and says the commit. Ends with 0 when it is accepted, 1 when it
            is refused, 2 when it was used wrongly or the repository is not
            set up
  status    list the issues in the flow under headings, as git status lists
            files: each issue under the last step it has done (Not broken
            down, Broken down, Accepted, Tested, Built, Reviewed, Fixed), and
            last the issues that have a question no [answers] or [answer] line
            has closed. An issue is in the flow when its branch, local or at
            origin, holds a commit that the default branch of origin does not:
            git alone says which, and fetches nothing. Its line shows the
            branch, the open change as #<n> (change_of_branch) and how many
            commits are not pushed. The comments come from issue_comments.
            <issue> prints that issue, in the flow or not, with its title
            (issue_address) and what it was worked out from; --stage <heading>
            prints that heading only, whatever its case. For one issue it also
            prints a line kept: <role> round <n>, <k> commits not taken,
            <folder> for each view kept with work not taken back, and cut off:
            <role> round <n>, <folder> for each session whose wb ended before
            it did. With no issue it says on standard error how many such
            views are kept on this machine. Ends with 0 when
            every issue could be read, 1 when a pattern failed for one (it is
            listed under Not read), 2 when it was used wrongly or the
            repository is not set up. Writes nothing but a temporary index in
            $TMPDIR, removed after, where it reads a kept view
  view      show what a session of a role will see, before it starts. wb view
            <issue> <role> prints the paths of its view, one to a line, the
            list the start makes the view from, and nothing else. --in
            [<folder>] prints what of HEAD the view holds in the folder (the
            root when none is given), one level deep: each file in it, and
            <count><tab><folder>/ for each folder below it. --out prints the
            same for what the view leaves out. A folder is a path from the
            root, with or without a closing /. The level and the line view:
            are in wb prompt <role> <issue> --stamp.
            wb view <issue> <path> says for each role whether the path, from
            the root, is in its view: in, with the list and the line of the
            issue's file that hold it, or out, and why. A path named as a role
            is written ./<path>. Ends with 0 when the path is in the view of a
            role or the view is shown, 1 when it is in none, the folder is not
            in HEAD or the issue has no accepted, committed file, 2 when it
            was used wrongly (a file given as the folder too) or the
            repository is not set up. Writes nothing
  revert    take an issue back to before a step, deleting nothing:
            wb revert <issue> <role> [--round <n>] posts, by issue_comment of
            watchbill.yml, one comment whose first line is
            [revert] <role> round <n>, for the last stamp of the role, or round
            <n>. From that stamp up to the marker no stamp, question or answer
            counts, for wb start <role>, wb prompt and wb status. wb revert
            --change <n> <role> posts it on a change with no issue, by
            change_comment.
            The branch, the issue's file and its acceptance, and the change are
            not touched: taking commits back is git's. The breakdown is taken
            back with git, not by a marker. Ends with 0 when the marker is
            posted, 1 when it was refused or the posting failed, 2 when it was
            used wrongly or the repository is not set up
  answer    answer the open questions of an issue: wb answer <issue> reads
            the whole comments by issue_comment_bodies of watchbill.yml, shows
            each open question in full, one at a time, and asks: Enter takes
            the suggestion, a number takes that choice, other text is the
            answer in words; a number that is not a choice is asked again.
            Then it says what it will post and posts one [answers] comment per
            role by issue_comment. Nothing is posted before every question is
            answered, and nothing after Ctrl-C. --suggested takes every
            suggestion and names the questions with none. --show prints every
            open question in full, as it is laid out when asked, to standard
            output, and asks nothing, posts nothing and ends with 0; it takes
            no --suggested and no answer. <question> <answer>, as
            tester.q1 2 (q1 for the breakdown), answers a question
            without asking, and the pairs repeat; with --suggested the
            suggestion is taken for every question they do not answer.
            One comment per role is posted, or none when a pair is wrong
            (a question not open, a number not a choice: 1); a question
            named twice ends with 2. --change <n> answers on a change
            with no issue, by
            change_comment_bodies and change_comment. It never changes the
            issue's file: where an answer does, it says so. Ends with 0 when
            the answers are posted or no question is open, 1 when it was
            refused, the input ended or a pattern failed, 2 when it was used
            wrongly or the repository is not set up
";

/// Runs a command so that a fault in `wb` itself is not shown as a panic: the hook prints nothing,
/// and a panic that is caught ends the command with 1 (it did not hold) and one line on `err`.
/// `main` runs every command through it. The body is a parameter so that a test can make it panic.
pub fn guarded(body: impl FnOnce() -> i32, err: &mut dyn Write) -> i32 {
    std::panic::set_hook(Box::new(|_| {}));
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(code) => code,
        Err(_) => {
            let _ = writeln!(err, "wb: an internal error. This is a fault in wb, not in what it was given.");
            1
        }
    }
}

/// A word of the command line as text. What is not UTF-8 is written as the Python `wb` writes
/// it: each byte that is not part of a character as `\udcXX`.
pub fn word(bytes: &[u8]) -> String {
    let mut text = String::new();
    let mut rest = bytes;
    while !rest.is_empty() {
        match std::str::from_utf8(rest) {
            Ok(valid) => {
                text.push_str(valid);
                break;
            }
            Err(error) => {
                let (valid, after) = rest.split_at(error.valid_up_to());
                text.push_str(std::str::from_utf8(valid).unwrap_or(""));
                text.push_str(&format!("\\udc{:02x}", after[0]));
                rest = &after[1..];
            }
        }
    }
    text
}

/// A word of the command line as `lexopt` read it, held apart from the parser.
enum Word {
    Short(char),
    Long(String),
    Value(OsString),
}

/// The next option or word. An option given a value it does not take, as `--yes=x` or `-y=x`, is
/// refused here, and the error is the word to name: the option with its value.
fn next(parser: &mut Parser) -> Result<Option<Word>, String> {
    match parser.next() {
        Ok(None) => Ok(None),
        Ok(Some(Arg::Short(option))) => Ok(Some(Word::Short(option))),
        Ok(Some(Arg::Long(option))) => Ok(Some(Word::Long(option.to_string()))),
        Ok(Some(Arg::Value(value))) => Ok(Some(Word::Value(value))),
        Err(lexopt::Error::UnexpectedValue { option, value }) => Err(format!("{}={}", option, word(value.as_bytes()))),
        // `next` of lexopt 0.3 gives no other error. Were a later one to, the text of `lexopt` is
        // not shown: the word is named empty and the command is still refused with 2.
        Err(_) => Err(String::new()),
    }
}

/// A word a command does not take, as `wb` names it: an option alone, with the value written
/// after it, and a value as it was given.
fn named(found: Word, parser: &mut Parser) -> String {
    match found {
        // `-x=1`: the value is named with the option. Otherwise what follows `-x` in the word is
        // other options, and the first that is not known is named alone.
        Word::Short(option) => match parser.next() {
            Err(lexopt::Error::UnexpectedValue { value, .. }) => format!("-{}={}", option, word(value.as_bytes())),
            _ => format!("-{}", option),
        },
        Word::Long(option) => match parser.optional_value() {
            Some(value) => format!("--{}={}", option, word(value.as_bytes())),
            None => format!("--{}", option),
        },
        Word::Value(value) => word(value.as_bytes()),
    }
}

/// Runs `wb` with the words after its name and returns the exit status:
/// 0 it holds or it is done, 1 it does not hold or it was refused, 2 it was used wrongly.
pub fn run(args: &[OsString], input: &mut dyn BufRead, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    // A failed write, such as a closed pipe, does not change what the command ended with.
    let mut parser = Parser::from_args(args.iter().cloned());
    // The first word is the command. `-h` and `-v` are taken only alone, so an option there that is
    // not taken was the whole first word, and it is named as it was given.
    let first = || word(args.first().map(|arg| arg.as_bytes()).unwrap_or_default());
    let command = match next(&mut parser) {
        Ok(None) => {
            let _ = out.write_all(layout::help().as_bytes());
            return 2;
        }
        Ok(Some(Word::Short('h'))) if parser.optional_value().is_none() => "help".to_string(),
        Ok(Some(Word::Long(option))) if option == "help" && parser.optional_value().is_none() => "help".to_string(),
        Ok(Some(Word::Short('v'))) if parser.optional_value().is_none() => "version".to_string(),
        Ok(Some(Word::Long(option))) if option == "version" && parser.optional_value().is_none() => {
            "version".to_string()
        }
        Ok(Some(Word::Value(value))) => match value.to_str() {
            Some(name) if name == "help" || layout::is_command(name) || STARTED.contains(&name) => name.to_string(),
            _ => {
                let _ = write!(err, "wb: no command '{}'\n\n{}", word(value.as_bytes()), USAGE);
                return 2;
            }
        },
        _ => {
            let _ = write!(err, "wb: no command '{}'\n\n{}", first(), USAGE);
            return 2;
        }
    };
    // `--help` and `-h` are the command `help`, so a word after them is read as `wb help` reads it
    // (`wb -h init` prints the help of `init`, as git does), and one that is not a command ends with 2.
    // Words after `--version` change nothing.
    match command.as_str() {
        "help" => run_help(&mut parser, out, err),
        name if layout::planned(name).is_some() => {
            let _ = writeln!(err, "{}", layout::planned(name).unwrap_or_default());
            2
        }
        "version" => {
            let _ = writeln!(out, "{}", VERSION);
            0
        }
        "init" => run_init(&mut parser, input, out, err),
        "check" => run_check(&mut parser, out, err),
        "upgrade" => run_upgrade(&mut parser, out, err),
        "prompt" => run_prompt(&mut parser, None, Path::new(confidentiality::ORGANISATION_FILE), out, err),
        "accept" => run_accept(&mut parser, out, err),
        "status" => run_status(&mut parser, out, err),
        "view" => run_view(&mut parser, None, Path::new(confidentiality::ORGANISATION_FILE), out, err),
        "revert" => run_revert(&mut parser, err),
        "answer" => run_answer(&mut parser, input, out, err),
        "start" => {
            use std::io::IsTerminal;
            let terminal = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
            let state = sessions::state_folder();
            let home = home_folder();
            let machine = Machine {
                organisation: Path::new(confidentiality::ORGANISATION_FILE),
                terminal,
                state: state.as_deref(),
                sandbox: Path::new(view::BWRAP),
                home: home.as_deref(),
            };
            run_start_word(&mut parser, None, &machine, out, err)
        }
        // `wb <role>` was the form before `wb start <role>` (#175).
        _ if STARTED.contains(&command.as_str()) => old_form(args, err),
        "revision" => run_revision(&mut parser, Path::new(manifest::PACKAGE_FOLDER), out, err),
        // A command of the layout that left `layout::PLANNED` and has no arm yet is refused, not run as another.
        other => {
            let _ = writeln!(err, "wb: the command '{}' has no arm in the dispatch", other);
            2
        }
    }
}

/// The commands of the layout that are built, each with an arm in `run`. A test holds that it is the
/// layout without `layout::PLANNED`, so a piece that builds a command adds it here with its arm.
pub const ARMS: [&str; 11] =
    ["init", "check", "upgrade", "accept", "start", "status", "view", "answer", "revert", "prompt", "revision"];

/// `wb help`, and `wb help <command>`. `--help` and `-h` are this command too, and read the words after them.
/// A word after the command is refused with 2.
fn run_help(parser: &mut Parser, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let name = match next(parser) {
        Ok(None) => {
            let _ = out.write_all(layout::help().as_bytes());
            return 0;
        }
        Ok(Some(Word::Value(value))) => word(value.as_bytes()),
        Ok(Some(other)) => named(other, parser),
        Err(refused) => refused,
    };
    match next(parser) {
        Ok(None) => {}
        Ok(Some(found)) => {
            let extra = named(found, parser);
            let _ = write!(err, "wb help: no word is taken after '{}': '{}'\n\n{}", name, extra, layout::help());
            return 2;
        }
        Err(refused) => {
            let _ = write!(err, "wb help: no word is taken after '{}': '{}'\n\n{}", name, refused, layout::help());
            return 2;
        }
    }
    match layout::help_of(&name, USAGE) {
        Some(text) => {
            let _ = out.write_all(text.as_bytes());
            0
        }
        None => {
            let _ = write!(err, "wb help: no command '{}'\n\n{}", name, layout::help());
            2
        }
    }
}

/// `wb init [--yes]`
fn run_init(parser: &mut Parser, input: &mut dyn BufRead, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut yes = false;
    loop {
        let refused = match next(parser) {
            Ok(None) => break,
            Ok(Some(Word::Short('y'))) => {
                yes = true;
                continue;
            }
            Ok(Some(Word::Long(option))) if option == "yes" => {
                yes = true;
                continue;
            }
            Ok(Some(found)) => named(found, parser),
            Err(refused) => refused,
        };
        let _ = write!(err, "wb init: '{}' is not known\n\n{}", refused, USAGE);
        return 2;
    }

    interrupt::install();
    match init::repository_root(None) {
        Err(why) => {
            let _ = writeln!(err, "wb init: {}. Nothing written.", why);
            1
        }
        Ok(None) => {
            let _ = writeln!(err, "wb init: this folder is not in a git repository. Watchbill is set up in one.");
            1
        }
        Ok(Some(root)) => init::init(&root, VERSION, Path::new(manifest::PACKAGE_FOLDER), yes, input, out, err, None),
    }
}

/// `wb check`: the repository against its pinned revision and the installed `wb`. It writes nothing.
fn run_check(parser: &mut Parser, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let refused = match next(parser) {
        Ok(None) => None,
        Ok(Some(found)) => Some(named(found, parser)),
        Err(refused) => Some(refused),
    };
    if let Some(refused) = refused {
        let _ = write!(err, "wb check: '{}' is not known\n\n{}", refused, USAGE);
        return 2;
    }
    match init::repository_root(None) {
        Err(why) => {
            let _ = writeln!(err, "wb check: {}.", why);
            2
        }
        Ok(None) => {
            let _ = writeln!(err, "wb check: this folder is not in a git repository. Watchbill is set up in one.");
            2
        }
        Ok(Some(root)) => check::check(&root, VERSION, Path::new(manifest::PACKAGE_FOLDER), out, err),
    }
}

/// `wb upgrade [--dry-run]`: pins the repository to the installed revision, saying each write first (#174).
fn run_upgrade(parser: &mut Parser, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut dry_run = false;
    loop {
        let refused = match next(parser) {
            Ok(None) => break,
            Ok(Some(Word::Long(option))) if option == "dry-run" => {
                if dry_run {
                    return used(err, "wb upgrade", "'--dry-run' is given twice");
                }
                dry_run = true;
                continue;
            }
            Ok(Some(found)) => named(found, parser),
            Err(refused) => refused,
        };
        return used(err, "wb upgrade", &format!("'{}' is not known", refused));
    }
    match init::repository_root(None) {
        Err(why) => {
            let _ = writeln!(err, "wb upgrade: {}. Nothing written.", why);
            2
        }
        Ok(None) => {
            let _ = writeln!(err, "wb upgrade: this folder is not in a git repository. Watchbill is set up in one, by wb init.");
            2
        }
        Ok(Some(root)) => {
            interrupt::install();
            upgrade::upgrade(&root, VERSION, Path::new(manifest::PACKAGE_FOLDER), dry_run, out, err)
        }
    }
}

/// `wb prompt`, with the words after `prompt`, in the repository of `folder`, and the organisation's file at
/// `organisation`. `run` gives the folder it runs in and the path fixed in `wb`
/// (`confidentiality::ORGANISATION_FILE`); this is so that a test can give its own, as `revision` does (#93).
/// The variables `GIT_*` of this process are not handed to git here: `folder` alone says the repository.
pub fn prompt(args: &[OsString], folder: &Path, organisation: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    run_prompt(&mut Parser::from_args(args.iter().cloned()), Some(folder), organisation, out, err)
}

/// `wb start <role>`, with the role and the words after it, in the repository of `folder`, with the organisation's
/// file at `organisation`, and `terminal` the answer to whether standard input and output are a terminal
/// (#84), and `state` the state folder the lines of the start are written below, or none (#124). `run` gives
/// the folder it runs in, the path fixed in `wb`, what the terminal says, and `sessions::state_folder`; this
/// is so that a test can give its own, as `prompt` does.
pub fn start(
    args: &[OsString],
    folder: &Path,
    organisation: &Path,
    terminal: bool,
    state: Option<&Path>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let home = home_folder();
    let machine = Machine { organisation, terminal, state, sandbox: Path::new(view::BWRAP), home: home.as_deref() };
    start_on(args, folder, &machine, out, err)
}

/// What `wb start <role>` takes from the machine it runs on. `run` gives the path of the organisation's file fixed in
/// `wb`, what the terminal says, `sessions::state_folder`, bubblewrap at `view::BWRAP` and `$HOME`; a test gives
/// its own.
pub struct Machine<'a> {
    /// The organisation's file (#93).
    pub organisation: &'a Path,
    /// Whether standard input and output are a terminal (#84).
    pub terminal: bool,
    /// The state folder the lines of the start (#124) and the views (#126) are written below, or none.
    pub state: Option<&'a Path>,
    /// bubblewrap, which a session in a view runs in (#126).
    pub sandbox: &'a Path,
    /// The home folder that `~/` of a runner's `binds` names, or none.
    pub home: Option<&'a Path>,
}

/// The home folder of the person who runs `wb`: `$HOME` when it is an absolute path.
fn home_folder() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from).filter(|path| path.is_absolute())
}

/// `wb start <role>`, as `start` runs it, on the machine `machine` says.
pub fn start_on(args: &[OsString], folder: &Path, machine: &Machine, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut parser = Parser::from_args(args.iter().cloned());
    match next(&mut parser) {
        Ok(Some(Word::Value(value))) if value.to_str() == Some("start") => run_start_word(&mut parser, Some(folder), machine, out, err),
        Ok(Some(Word::Value(value))) if value.to_str().is_some_and(|role| STARTED.contains(&role)) => old_form(args, err),
        _ => {
            let _ = write!(err, "wb: no command '{}'\n\n{}", word(args.first().map(|arg| arg.as_bytes()).unwrap_or_default()), USAGE);
            2
        }
    }
}

/// `wb start <role> …`: the word after `start` is the role, or the start is refused with 2, naming the roles.
fn run_start_word(parser: &mut Parser, folder: Option<&Path>, machine: &Machine, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let roles = STARTED.join(", ");
    match next(parser) {
        Ok(Some(Word::Value(value))) if value.to_str().is_some_and(|role| STARTED.contains(&role)) => {
            let role = word(value.as_bytes());
            run_start(&role, parser, folder, machine, out, err)
        }
        Ok(None) => used(err, "wb start", &format!("a role is needed: {}", roles)),
        Ok(Some(found)) => {
            let given = named(found, parser);
            used(err, "wb start", &format!("'{}' is not a role of the revision: {}", given.escape_debug(), roles))
        }
        Err(given) => used(err, "wb start", &format!("'{}' is not a role of the revision: {}", given.escape_debug(), roles)),
    }
}

/// A word as a shell takes it back as one word: as it is where it holds only plain characters, else in single quotes.
fn quoted(word: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c);
    if !word.is_empty() && word.chars().all(plain) {
        word.to_string()
    } else {
        format!("'{}'", word.replace('\'', "'\\''"))
    }
}

/// `wb start <role> …`, which `wb start <role> …` replaced (#175): ends with 2 and says the new form, with the words
/// as they were given.
fn old_form(args: &[OsString], err: &mut dyn Write) -> i32 {
    let words: Vec<String> = args.iter().map(|arg| quoted(&word(arg.as_bytes()))).collect();
    let _ = writeln!(err, "wb: {} is not a command any more: use: wb start {}", words[0], words.join(" "));
    2
}

/// The roles that `wb start` starts a session of (#84, #175).
const STARTED: [&str; 5] = ["breakdown", "tester", "build", "review", "fix"];

/// Says that a command was used wrongly, then the usage, and gives 2.
fn used(err: &mut dyn Write, name: &str, why: &str) -> i32 {
    let _ = write!(err, "{}: {}\n\n{}", name, why, USAGE);
    2
}

/// The value of a placeholder among the values of a prompt.
fn value_of<'a>(values: &'a [(&'static str, String)], name: &str) -> Option<&'a str> {
    values.iter().find(|(given, _)| *given == name).map(|(_, value)| value.as_str())
}

/// What `wb prompt <role> …` or `wb start <role> …` was given.
struct Words {
    role: String,
    /// The value of each placeholder given, the issue word among them as `issue`.
    values: Vec<(&'static str, String)>,
    parts: bool,
    stamp: bool,
    /// `--prompt` of `wb start <role>`: print the prompt, as `wb prompt` does.
    print: bool,
    runner: Option<String>,
    /// `--ignore-local` of `wb start <role>`: a local change under `reads` does not stop the start (#172).
    ignore_local: bool,
    /// `--ignore-kept` of `wb start <role>`: a view kept with work not taken back does not stop the start (#222).
    ignore_kept: bool,
}

impl Words {
    fn get(&self, name: &str) -> Option<&str> {
        value_of(&self.values, name)
    }

    /// Only what `wb prompt` does: print, and post and start nothing.
    fn prints(&self) -> bool {
        self.print || self.stamp || self.parts
    }

    /// A review or a fix of a change with no issue (#116).
    fn alone(&self) -> bool {
        self.get("issue").is_none() && prompt::ALONE.contains(&self.role.as_str()) && self.get("change").is_some()
    }
}

/// Reads the words of `wb prompt`, where the first word is the role, or of `wb start <role>`, where `role` is given
/// and `--prompt` is taken too. What `wb prompt` refuses as used wrongly is refused here, in its order, with
/// its words after `name`. The status of a refusal is the error.
fn read_words(parser: &mut Parser, name: &str, role: Option<&str>, err: &mut dyn Write) -> Result<Words, i32> {
    let starter = role.is_some();
    let mut role: Option<String> = role.map(str::to_string);
    let mut issue_word: Option<String> = None;
    let mut values: Vec<(&'static str, String)> = Vec::new();
    let (mut parts, mut stamp, mut print, mut ignore_local, mut ignore_kept) = (false, false, false, false, false);
    let mut runner: Option<String> = None;
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(refused) => return Err(used(err, name, &format!("'{}' is not known", refused))),
        };
        match found {
            Word::Long(option) if option == "parts" || option == "stamp" || (starter && (option == "prompt" || option == "ignore-local" || option == "ignore-kept")) => {
                let flag = match option.as_str() {
                    "parts" => &mut parts,
                    "stamp" => &mut stamp,
                    "ignore-local" => &mut ignore_local,
                    "ignore-kept" => &mut ignore_kept,
                    _ => &mut print,
                };
                if *flag {
                    return Err(used(err, name, &format!("'--{}' is given twice", option)));
                }
                *flag = true;
            }
            Word::Long(option) if option == "runner" => {
                let Ok(value) = parser.value() else {
                    return Err(used(err, name, "'--runner' needs a value"));
                };
                let Ok(value) = value.into_string() else {
                    return Err(used(err, name, "the value of '--runner' is not UTF-8 text"));
                };
                if value.is_empty() {
                    return Err(used(err, name, "the value of '--runner' is empty"));
                }
                if runner.is_some() {
                    return Err(used(err, name, "'--runner' is given twice"));
                }
                if !one_line(&value) {
                    return Err(used(
                        err,
                        name,
                        "the value of '--runner' holds a control character or a line break, or white space at its start or end, and a stamp holds it as one line",
                    ));
                }
                runner = Some(value);
            }
            Word::Long(option) if prompt::PLACEHOLDERS.contains(&option.as_str()) => {
                let placeholder = prompt::PLACEHOLDERS.iter().find(|known| **known == option).unwrap();
                // A value is whatever follows, also when it starts with `-`.
                let value = match parser.value() {
                    Ok(value) => value,
                    Err(_) => return Err(used(err, name, &format!("'--{}' needs a value", option))),
                };
                let Ok(value) = value.into_string() else {
                    return Err(used(err, name, &format!("the value of '--{}' is not UTF-8 text", option)));
                };
                if value.is_empty() {
                    return Err(used(err, name, &format!("the value of '--{}' is empty", option)));
                }
                if values.iter().any(|(given, _)| given == placeholder) {
                    return Err(used(err, name, &format!("'--{}' is given twice", option)));
                }
                values.push((placeholder, value));
            }
            Word::Value(value) if role.is_none() => role = Some(word(value.as_bytes())),
            Word::Value(value) if issue_word.is_none() => issue_word = Some(word(value.as_bytes())),
            found => {
                let refused = named(found, parser);
                return Err(used(err, name, &format!("'{}' is not known", refused)));
            }
        }
    }
    let Some(role) = role else {
        return Err(used(err, name, "a role is needed"));
    };
    if stamp && parts {
        return Err(used(err, name, "'--stamp' and '--parts' are not used together"));
    }
    // A session that `wb start <role>` starts takes the organisation's default runner, when none is given (#84).
    let prints = !starter || print || stamp || parts;
    if role == "breakdown" && runner.is_none() && prints {
        return Err(used(
            err,
            name,
            "the breakdown is handed out only to a runner that the organisation's file clears for it: give '--runner <name>'",
        ));
    }
    if let Some(word) = issue_word {
        if values.iter().any(|(given, _)| *given == "issue") {
            return Err(used(err, name, "the issue is given as a word and as '--issue'"));
        }
        if word.is_empty() {
            return Err(used(err, name, "the issue word is empty"));
        }
        values.push(("issue", word));
    }
    // Only the review and the fix take a change alone, with no issue (#116). The tester and the build
    // work for an issue, and are used wrongly without one.
    let has_issue = values.iter().any(|(given, _)| *given == "issue");
    if !has_issue && (role == "tester" || role == "build") {
        return Err(used(err, name, &format!("the {} is handed out for an issue: give it as the word after the role, or as '--issue'", role)));
    }
    // A stamp printed, or posted by `wb start <role>`, holds each value as one line.
    if stamp || !prints {
        if let Some(why) = stamp_fault(&values) {
            return Err(used(err, name, &why));
        }
    }
    Ok(Words { role, values, parts, stamp, print, runner, ignore_local, ignore_kept })
}

/// A prompt handed out: the values it was filled with, what was put together, and what was read for it.
struct Handed {
    root: String,
    values: Vec<(&'static str, String)>,
    found: prompt::Prompt,
    cleared: Option<confidentiality::Cleared>,
    file: Option<issue::Used>,
    requirements_given: bool,
    stamps: Option<starter::Stamps>,
    /// The level of the repository, for a session that runs in a view (#209): the stamp holds it.
    level: Option<String>,
}

/// Everything `wb prompt` checks and does before it prints, for the words given: the check of `wb check`,
/// the role, the breakdown's runner (#93), the issue's file (#76, #97), the values derived (#72) and those
/// worked out from the patterns (#84), and the prompt put together. `starting` is `wb start <role>` that starts a
/// session: it always needs the round, and a pattern it needs that is not there is refused. What is
/// refused is said after `name`, and its status is the error.
fn hand_out(name: &str, words: &Words, folder: Option<&Path>, organisation: &Path, starting: bool, err: &mut dyn Write) -> Result<Handed, i32> {
    let role = words.role.as_str();
    let found_root = match folder {
        None => init::repository_root(None),
        Some(folder) => init::repository_root_of(folder),
    };
    let root = match found_root {
        Err(why) => {
            let _ = writeln!(err, "{}: {}.", name, why);
            return Err(2);
        }
        Ok(None) => {
            let _ = writeln!(err, "{}: this folder is not in a git repository. Watchbill is set up in one.", name);
            return Err(2);
        }
        Ok(Some(root)) => root,
    };
    let refused = |err: &mut dyn Write, why: &dyn std::fmt::Display| {
        let _ = writeln!(err, "{}: {}", name, why);
        1
    };
    // `wb prompt` and a start read the patterns of `watchbill.yml`, and only while it is as `HEAD` has it (#186).
    if let Err(why) = starter::held(&root) {
        return Err(refused(err, &format!("{}.", why)));
    }
    // What `wb check` says is for standard error here: standard output holds the prompt or nothing.
    let (mut said, mut why) = (Vec::new(), Vec::new());
    let status = check::check(&root, VERSION, Path::new(manifest::PACKAGE_FOLDER), &mut said, &mut why);
    if status != 0 {
        let _ = writeln!(err, "{}: nothing printed, the check of `wb check` does not hold.", name);
        let _ = err.write_all(&said);
        let _ = err.write_all(&why);
        return Err(status);
    }
    // A role the revision does not have is refused before anything is derived for it.
    if let Err(why) = prompt::require_role(Path::new(&root), role) {
        return Err(refused(err, &why));
    }
    // The breakdown goes only to a runner the organisation cleared for it, up to the repository's level (#93).
    let mut cleared = None;
    if role == "breakdown" {
        match confidentiality::clear_breakdown(organisation, Path::new(&root), words.runner.as_deref().unwrap_or_default()) {
            Ok(found) => cleared = Some(found),
            Err(why) => return Err(refused(err, &why)),
        }
    }
    // The file of the issue (#76): used when it is accepted as it is, refused when it is there and is
    // not, and left out when there is none. An option for the requirements wins over the file.
    let mut values = words.values.clone();
    let mut file = None;
    let mut requirements_given = false;
    if let (true, Some(issue)) = (issue::ROLES.contains(&role), words.get("issue")) {
        match issue::for_prompt(Path::new(&root), issue) {
            Ok(found) => file = found,
            Err(why) => return Err(refused(err, &why)),
        }
    }
    if let Some(file) = &file {
        requirements_given = words.get("requirements").is_some();
        if !requirements_given {
            values.push(("requirements", file.requirements()));
        }
    }
    // The round and the change are worked out where the prompt holds them, and the round always for a
    // session that `wb start <role>` starts: its stamp's first line holds it (#84).
    let holds = prompt::used_by(Path::new(&root), role);
    let need = prompt::Need {
        round: starting || holds.contains(&"round"),
        change: holds.contains(&"change"),
        classed: holds.contains(&"classed") || holds.contains(&"classed_touched"),
        strict: starting,
    };
    let before = values.len();
    let (values, worked) = match prompt::derive_with(&root, role, &values, need) {
        Ok(found) => found,
        Err(prompt::Derive::Used(why)) => return Err(used(err, name, &why)),
        Err(prompt::Derive::Refused(why)) => return Err(refused(err, &why)),
    };
    // What was worked out is said first, so that a person sees it (#84).
    if !worked.said.is_empty() {
        let _ = writeln!(err, "{}: {}", name, worked.said.join(", "));
    }
    // The change of a review or a fix, once, and from where (#162): shown to the person, never asked of them.
    // It is from the branch where `--change` was not given and the change was worked out. `wb prompt` says it
    // only for a change worked out, as before: a change given to it is not repeated.
    if (role == "review" || role == "fix") && (starting || words.get("change").is_none()) {
        if let Some(change) = value_of(&values, "change") {
            let from = match (words.get("change"), value_of(&values, "branch")) {
                (Some(_), _) => "given".to_string(),
                (None, Some(branch)) => format!("from the branch {}", branch),
                (None, None) => "worked out".to_string(),
            };
            let _ = match words.get("issue") {
                Some(issue) => writeln!(err, "{}: issue {}, change {} ({})", name, issue, change, from),
                None => writeln!(err, "{}: change {}", name, change),
            };
        }
    }
    // An issue's file that is there is used only when it and its acceptance are committed (#97). An issue
    // with no file is handed out as it is: a person may start any role alone (#116).
    if let (Some(_), Some((_, issue))) = (&file, values.iter().find(|(given, _)| *given == "issue")) {
        if let Err(why) = issue::committed(Path::new(&root), issue) {
            return Err(refused(err, &why));
        }
    }
    // A value derived that is not one plain line is not the caller's fault: refused, with where it came from.
    if words.stamp || starting {
        let derived = &values[before..];
        if let Some(name_of) = stamp_fault_in(derived) {
            let source = match name_of {
                "checkout" => "the root of the repository",
                _ => "the pattern of watchbill.yml and the branches of the repository",
            };
            let _ = writeln!(
                err,
                "{}: the {} was derived from {} and holds a control character or a line break, or white space at its start or end, and a stamp holds it as one line. Give --{}.",
                name, name_of, source, name_of
            );
            return Err(1);
        }
    }
    // The text of a review or a fix of a change with no issue says what to do where the issue is `none`
    // (#116). Only the text has it: the stamp is made from `values`, and its `issue:` is empty.
    let mut filled: Vec<(&str, &str)> = values.iter().map(|(given, value)| (*given, value.as_str())).collect();
    if words.alone() {
        for no in ["issue", "issue_address"] {
            if !filled.iter().any(|(given, _)| *given == no) {
                filled.push((no, prompt::NO_ISSUE));
            }
        }
    }
    let text = file.as_ref().map(|file| file.text(role));
    let extra = file.as_ref().zip(text.as_deref()).map(|(file, text)| prompt::Extra { path: &file.path, hash: &file.hash, text });
    let found = match prompt::assemble_with(Path::new(&root), role, &filled, extra.as_ref()) {
        Ok(found) => found,
        Err(why) => return Err(refused(err, &why)),
    };
    // The level goes in the stamp of a session that runs in a view (#209), from the same place `--parts` has it.
    let level = match view_paths_of(role, file.as_ref()) {
        Some(_) => {
            let (level, why) = stamp_level(&root, organisation);
            if let Some(why) = why {
                let _ = writeln!(err, "{}: the level of the stamp is {}: {}", name, level, why);
            }
            Some(level)
        }
        None => None,
    };
    Ok(Handed { root, values, found, cleared, file, requirements_given, stamps: worked.stamps, level })
}

/// The model and effort a role is handed out with: for the breakdown those its runner is cleared with, for
/// the others the model table's.
fn model_of(role: &str, cleared: Option<&confidentiality::Cleared>) -> Result<rules::Model, String> {
    if let Some(cleared) = cleared {
        return Ok(rules::Model { model: cleared.model.clone(), effort: cleared.effort.clone() });
    }
    let rules = rules::load_revision().map_err(|why| why.to_string())?;
    rules.models.get(role).cloned().ok_or_else(|| format!("the model table has no row for '{}'", role))
}

/// The seconds since 1970 by the clock of this machine. A clock that fails, or is before 1970, gives no
/// stamp: a false time is worse than none.
fn now() -> Result<u64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs())
        .map_err(|_| "the clock of this machine reads before 1970 or cannot be read. No stamp.".to_string())
}

/// The stamp of what is handed out (`docs/revision.md`, point 12), as `wb prompt --stamp` prints it, with the
/// time `seconds`, as `now` gives it.
fn stamp_of(role: &str, handed: &Handed, runner: &str, local: Option<usize>, kept: Option<usize>, seconds: Result<u64, String>) -> Result<String, String> {
    let model = model_of(role, handed.cleared.as_ref())?;
    let revision = manifest::revision_hash(manifest::PACKAGE_FOLDER).map_err(|why| why.to_string())?;
    let time = prompt::utc(seconds?);
    let view = view_hash(role, handed.file.as_ref());
    let said = prompt::Stamp {
        view: &view,
        level: handed.level.as_deref(),
        local,
        kept,
        role,
        model: &model.model,
        effort: &model.effort,
        runner,
        version: VERSION,
        revision: &revision,
        time: &time,
        requirements_given: handed.requirements_given,
    };
    let given: Vec<(&str, &str)> = handed.values.iter().map(|(name, value)| (*name, value.as_str())).collect();
    Ok(prompt::stamp(&said, &given, &handed.found))
}

/// The local changes the view of a session of `role` would miss (#172): none looked for where the session runs
/// without a view.
fn local_of(role: &str, handed: &Handed, root: &str) -> Option<Result<Vec<view::Local>, String>> {
    let file = handed.file.as_ref()?;
    issue::ROLES.contains(&role).then(|| view::local_changes(root, &file.issue, role))
}

/// The paths of the view a session of `role` runs in (#126): those of the issue's file for the role, when the
/// role is one the file lists paths for and a file is used. `None`: the session runs without a view, as the
/// breakdown, which reads the whole code by its role, and a role started with no issue's file (#116) do.
fn view_paths(role: &str, handed: &Handed) -> Option<Vec<String>> {
    view_paths_of(role, handed.file.as_ref())
}

/// The paths of the view of `role` for the issue's file used, as `view_paths` gives them.
fn view_paths_of(role: &str, file: Option<&issue::Used>) -> Option<Vec<String>> {
    let file = file?;
    issue::ROLES.contains(&role).then(|| view::paths(&file.issue, role))
}

/// The stamp's `view:` (#126): the SHA-256 of the list of paths of the view, or `none` where the session runs
/// without one. `wb view` prints it by this function (#173), so the two cannot differ.
fn view_hash(role: &str, file: Option<&issue::Used>) -> String {
    match view_paths_of(role, file) {
        Some(paths) => view::hash(&paths),
        None => "none".to_string(),
    }
}

/// Prints what was handed out, as `wb prompt` prints it: the stamp, the parts, or the text.
fn print(name: &str, words: &Words, handed: &Handed, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    if words.stamp {
        match stamp_of(&words.role, handed, words.runner.as_deref().unwrap_or_default(), None, None, now()) {
            Ok(text) => {
                let _ = out.write_all(text.as_bytes());
            }
            Err(why) => {
                let _ = writeln!(err, "{}: {}", name, why);
                return 1;
            }
        }
    } else if words.parts {
        let mut lines: String = handed.found.parts.iter().map(|part| format!("{}  {}\n", part.hash, part.path)).collect();
        if let (Some(cleared), Some(runner)) = (&handed.cleared, &words.runner) {
            lines.push_str(&format!("level: {}\nrunner: {}\n", cleared.level, runner));
        }
        let _ = out.write_all(lines.as_bytes());
    } else {
        let _ = out.write_all(handed.found.text.as_bytes());
    }
    0
}

/// `wb prompt <role> [--<placeholder> X]... [--runner <name>] [--parts]`: the prompt of a role, filled from
/// one option for each placeholder of the list (`prompt::PLACEHOLDERS`), or with `--parts` the path and hash
/// of each of its parts. Nothing is printed unless the check of `wb check` holds and the prompt can be put
/// together. The breakdown is handed out only to a runner the organisation's file clears for it (#93). It
/// writes nothing.
fn run_prompt(parser: &mut Parser, folder: Option<&Path>, organisation: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let name = "wb prompt";
    let words = match read_words(parser, name, None, err) {
        Ok(words) => words,
        Err(status) => return status,
    };
    match hand_out(name, &words, folder, organisation, false, err) {
        Ok(handed) => print(name, &words, &handed, out, err),
        Err(status) => status,
    }
}

/// Whether a round given to `wb start <role>` can stand in the first line of a stamp, to be read back: a whole
/// number from 1, with no leading zero.
fn is_round(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_digit()) && !value.starts_with('0')
}

/// Standard error, with what is said on it kept, so that the lines of a start (#124) hold what `wb prompt`
/// refused, said as it says it.
struct Kept<'a> {
    inner: &'a mut dyn Write,
    kept: Vec<u8>,
}

impl Write for Kept<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(bytes)?;
        self.kept.extend_from_slice(&bytes[..written]);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// The refusal in what `name` said: the last line that starts with `<name>: `, without it. What was worked
/// out is said before a refusal, and what `wb check` says follows its own.
fn refusal_in(name: &str, said: &[u8]) -> String {
    let said = String::from_utf8_lossy(said);
    let start = format!("{}: ", name);
    said.lines().filter_map(|line| line.strip_prefix(&start)).last().or_else(|| said.lines().next()).unwrap_or_default().to_string()
}

/// `wb start <role> [<issue>] [--runner <name>] [--prompt] …` (#84): the checks of `wb prompt`, then that the rule
/// table lets the role start, that no classed code is in its way, that the runner has `start`, and that it
/// runs at a terminal, and for an issue with an accepted file that its view can be made (#126); then the stamp
/// is posted by `issue_comment` (or `change_comment`), and the session is started by the runner's `start`, in
/// its view in bubblewrap where it has one, and its commits are taken back. The first fault ends it, and nothing
/// is posted or started. With `--prompt`, `--stamp` or `--parts` it does what `wb prompt` does, and writes
/// nothing.
///
/// From the checks on, each start leaves its lines in a new folder below `state` (#124): the stamp, posted
/// or as it would have been, the outcome (`sessions`), and for a view `view` and `back`.
fn run_start(role: &str, parser: &mut Parser, folder: Option<&Path>, machine: &Machine, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let (organisation, terminal, state) = (machine.organisation, machine.terminal, machine.state);
    let name = format!("wb start {}", role);
    let name = name.as_str();
    let mut words = match read_words(parser, name, Some(role), err) {
        Ok(words) => words,
        Err(status) => return status,
    };
    if words.prints() {
        return match hand_out(name, &words, folder, organisation, false, err) {
            Ok(handed) => print(name, &words, &handed, out, err),
            Err(status) => status,
        };
    }
    // The stamp's first line is read back by the next start: the round is a whole number.
    if let Some(round) = words.get("round") {
        if !is_round(round) {
            return used(err, name, "the value of '--round' is a whole number from 1, written without a leading zero");
        }
    }
    // The issue, or the change of a review or a fix with no issue, fills the pattern that posts the stamp.
    let alone = words.alone();
    let (post, word, on) = match (words.get("issue"), words.get("change")) {
        (Some(issue), _) => ("issue_comment", "issue", issue.to_string()),
        (None, Some(change)) if alone => ("change_comment", "change", change.to_string()),
        _ => ("issue_comment", "issue", String::new()),
    };
    if !on.is_empty() && !prompt::is_plain_name(&on) {
        return used(err, name, &prompt::not_plain(word));
    }
    // The runner: the one given, else the organisation's `default`.
    if words.runner.is_none() {
        match confidentiality::read(organisation) {
            Ok(found) => match found.default {
                Some(default) => words.runner = Some(default),
                None => {
                    let why = format!(
                        "no runner is given, and the organisation's file {} names no `default`: give '--runner <name>'",
                        organisation.display()
                    );
                    return used(err, name, &why);
                }
            },
            Err(_) if !organisation.exists() => {
                let why = format!(
                    "no runner is given, and there is no organisation's file {} to name a `default`: give '--runner <name>'",
                    organisation.display()
                );
                return used(err, name, &why);
            }
            Err(why) => {
                let _ = writeln!(err, "{}: {}", name, why);
                return 1;
            }
        }
    }
    let runner = words.runner.clone().unwrap_or_default();
    // The lines of this start (#124), from here on. The clock is read once: the folder's time is the stamp's.
    let seconds = now();
    let mut lines = sessions::Lines::new(name, state, seconds.clone(), role, words.get("issue"));
    // 1. Everything `wb prompt` checks. What it refuses is said as `wb prompt` says it, and kept for the lines.
    let mut kept = Kept { inner: err, kept: Vec::new() };
    let handed = hand_out(name, &words, folder, organisation, true, &mut kept);
    let said = kept.kept;
    let handed = match handed {
        Ok(handed) => handed,
        Err(status) => {
            let message = refusal_in(name, &said);
            lines.close(None, sessions::Outcome::Refused { check: "prompt", status, message }, err);
            return status;
        }
    };
    let root = handed.root.as_str();
    // The local changes the view would miss (#172), looked at now so that the stamp says what is ignored; they stop
    // the start at the view's check, below.
    let local = local_of(role, &handed, root);
    let ignored = match &local {
        Some(Ok(found)) if words.ignore_local && !found.is_empty() && found.iter().all(|local| !local.changes) => Some(found.len()),
        _ => None,
    };
    // The views of this issue kept with work not taken back (#222), from any role: they stop the start, below, unless
    // `--ignore-kept` passes them, and then the stamp says how many.
    let kept_views = match (words.get("issue"), state) {
        (Some(issue), Some(state)) if !words.ignore_kept => kept::scan(root, state, issue),
        _ => Vec::new(),
    };
    // `--ignore-kept` runs no git on any view: it counts them, for the stamp.
    let kept_ignored = match (words.get("issue"), state) {
        (Some(issue), Some(state)) if words.ignore_kept => Some(kept::count(root, state, issue)).filter(|count| *count > 0),
        _ => None,
    };
    // The stamp, as it will be posted; made now, so that a start refused before it is posted leaves it.
    let round = value_of(&handed.values, "round").map(str::to_string);
    let stamp = match &round {
        Some(round) => stamp_of(role, &handed, &runner, ignored, kept_ignored, seconds).map(|stamp| format!("[stamp] {} round {}\n{}", role, round, stamp)),
        None => Err("no round was given or worked out. Give --round".to_string()),
    };
    let refused = |lines: &mut sessions::Lines, err: &mut dyn Write, check: &'static str, why: &str| {
        let _ = writeln!(err, "{}: {}", name, why);
        lines.close(stamp.as_deref().ok(), sessions::Outcome::Refused { check, status: 1, message: why.to_string() }, err);
        1
    };
    // 1b. A view kept with work not taken back (#222): what a session left there is not on the branch, and the next
    // session would not see it. Before anything is posted. Only `--ignore-kept` passes it.
    if !kept_views.is_empty() && kept_ignored.is_none() {
        let issue = words.get("issue").unwrap_or_default();
        return refused(&mut lines, err, "kept", &kept::refusal(issue, &kept_views));
    }
    // 2. The rule table: a fix follows a review. The number of rounds is no rule for a person (#84).
    if role == "fix" {
        let stamps = match &handed.stamps {
            Some(stamps) => stamps.clone(),
            None => {
                let (key, word) = if alone { ("change_comments", "change") } else { ("issue_comments", "issue") };
                let line = match prompt::pattern_of(root, key) {
                    Ok(Some(line)) => line,
                    Ok(None) => {
                        let why = format!(
                            "a fix follows a review (the rule table), and that is checked from the stamps that `{}` of watchbill.yml prints, and it has none. Add the line (see the README)",
                            key
                        );
                        return refused(&mut lines, err, "rule", &why);
                    }
                    Err(why) => return refused(&mut lines, err, "rule", &why.to_string()),
                };
                let at = format!("{} {}", word, on);
                match starter::read_stamps(root, key, &line.replace(&format!("{{{}}}", word), &on), &at) {
                    Ok(stamps) => stamps,
                    Err(why) => return refused(&mut lines, err, "rule", &why),
                }
            }
        };
        if stamps.count("review") == 0 {
            return refused(&mut lines, err, "rule", &starter::no_review(&stamps.on, &stamps.key));
        }
    }
    // 3. The class: no classed code is worked on here before #65. The breakdown changes only the issue's file.
    if role != "breakdown" {
        let changes = handed.file.as_ref().map(|file| file.issue.changes.as_slice());
        if let Err(why) = starter::class(root, changes) {
            return refused(&mut lines, err, "class", &why);
        }
    }
    // 4. The runner's `start`, filled, and each word one that a program can be handed.
    let organisation_file = match confidentiality::read(organisation) {
        Ok(found) => found,
        Err(why) => return refused(&mut lines, err, "start", &why),
    };
    let Some(chosen) = organisation_file.runners.iter().find(|one| one.name == runner) else {
        let names: Vec<&str> = organisation_file.runners.iter().map(|one| one.name.as_str()).collect();
        let why = format!(
            "the organisation's file {} names no runner '{}'. Its runners are: {}.",
            organisation.display(),
            runner.escape_debug(),
            names.join(", ")
        );
        return refused(&mut lines, err, "start", &why);
    };
    let Some(start) = &chosen.start else {
        let why = format!(
            "the runner '{}' has no `start` in the organisation's file {}, so wb starts no session on it. Use --prompt, and start the session by hand.",
            runner,
            organisation.display()
        );
        return refused(&mut lines, err, "start", &why);
    };
    let model = match model_of(role, handed.cleared.as_ref()) {
        Ok(model) => model,
        Err(why) => return refused(&mut lines, err, "start", &why),
    };
    let filled = starter::fill(start, &model.model, &model.effort, &handed.found.text);
    if let Some((index, _)) = filled.iter().enumerate().find(|(_, word)| word.len() + 1 > starter::LONGEST_WORD) {
        let why = format!(
            "word {} of the runner's `start` is {} bytes once filled, and a program is handed no word of 128 KiB or more. Use --prompt, and hand the prompt to the session another way.",
            index,
            filled[index].len()
        );
        return refused(&mut lines, err, "start", &why);
    }
    // 5. A terminal: no session is started out of a person's sight before the sandbox (#65).
    if !terminal {
        return refused(
            &mut lines,
            err,
            "terminal",
            "standard input or standard output is not a terminal, and no session is started out of a person's sight before a session sees only its files (#65). Use --prompt.",
        );
    }
    // 6. The CI (#127): a review starts only on a change whose CI is green. The word is kept in the lines of the
    // start, whatever it is. The review's prompt does not get it, and never the CI's log.
    if role == "review" {
        let change = value_of(&handed.values, "change");
        let found = starter::ci(root, change);
        lines.open(stamp.as_deref().ok(), err);
        let kept = match &found {
            Ok(Some(ci)) => ci.word(),
            Ok(None) => "not read",
            Err(_) => "unread",
        };
        lines.write("ci", &format!("ci: {}\n", kept), err);
        match found {
            Err(why) => return refused(&mut lines, err, "ci", &why),
            Ok(None) => {
                let _ = writeln!(err, "{}: watchbill.yml has no `change_checks`, so the CI was not read. The review starts on a change whose CI nobody has looked at.", name);
            }
            Ok(Some(ci)) => {
                if let Some(why) = ci.refusal(change.unwrap_or_default()) {
                    return refused(&mut lines, err, "ci", &why);
                }
            }
        }
    }
    // 7. The view (#126): a session of an issue with an accepted file runs in one, in bubblewrap. A local change to
    // a file of it stops the start (#172) before its binds, the sandbox and the view itself are made sure of; none
    // of it is posted.
    match &local {
        Some(Err(why)) => return refused(&mut lines, err, "local", &format!("the local changes to the view could not be looked at: {}", why)),
        Some(Ok(found)) if !found.is_empty() && ignored.is_none() => {
            return refused(&mut lines, err, "local", &view::local_message(found, words.ignore_local));
        }
        _ => {}
    }
    let paths = view_paths(role, &handed);
    let mut prepared: Option<Prepared> = None;
    if let Some(paths) = &paths {
        match prepare_view(root, role, words.get("issue"), paths, &chosen.binds, machine) {
            Ok(found) => prepared = Some(found),
            Err(why) => return refused(&mut lines, err, "view", &why),
        }
    }
    // A view made and not run in is removed, whatever ends the start.
    let discard = |prepared: &Option<Prepared>| {
        if let Some(prepared) = prepared {
            view::discard(&prepared.view);
        }
    };
    // The pattern that posts the stamp. From here, a fault leaves the stamp not posted, and no view.
    let not_posted = |lines: &mut sessions::Lines, err: &mut dyn Write, why: &str| {
        discard(&prepared);
        let _ = writeln!(err, "{}: {}", name, why);
        lines.close(stamp.as_deref().ok(), sessions::Outcome::NotPosted { status: 1, message: why.to_string() }, err);
        1
    };
    let line = match prompt::pattern_of(root, post) {
        Ok(Some(line)) => line,
        Ok(None) => {
            let why = format!("watchbill.yml has no `{}`, so the stamp cannot be posted, and no session is started without it. Add the line (see the README), or use --prompt.", post);
            return not_posted(&mut lines, err, &why);
        }
        Err(why) => return not_posted(&mut lines, err, &why.to_string()),
    };
    let stamp = match &stamp {
        Ok(stamp) => stamp,
        Err(why) => return not_posted(&mut lines, err, why),
    };
    lines.open(Some(stamp), err);
    if let Some(paths) = &paths {
        lines.write("view", &view::list(paths), err);
    }
    // Where the view is, and what it was made from (#222): a `wb` ended before its session leaves no `back`, and the
    // view is found by this file.
    if let Some(prepared) = &prepared {
        if let Some(text) = kept::folder_text(&prepared.view, words.get("issue")) {
            lines.write("folder", &text, err);
        }
    }
    if let (Some(Ok(found)), Some(_)) = (&local, ignored) {
        let named: String = found.iter().map(|local| format!("{}\n", issue::shown(&local.path))).collect();
        lines.write("local", &named, err);
    }
    let _ = write!(err, "{}: will post on {} {}, by `{}` of watchbill.yml:\n{}", name, word, on, post, stamp);
    match starter::run(root, post, &line.replace(&format!("{{{}}}", word), &on), Some(stamp.as_bytes())) {
        Ok(said) => {
            let _ = err.write_all(&said);
        }
        Err(why) => {
            discard(&prepared);
            let why = format!("{}. The stamp may not be posted, and no session is started.", why);
            let _ = writeln!(err, "{}: {}", name, why);
            lines.close(None, sessions::Outcome::NotPosted { status: 1, message: why }, err);
            return 1;
        }
    }
    // Where the checkout's branch stands, to see after a session with no view whether it committed (#145).
    let before = if prepared.is_none() { view::branch_of(root).ok() } else { None };
    let began = std::time::Instant::now();
    // The signals stay held until the commits are taken back and `back` and `outcome` are written (#221).
    let _held = interrupt::hold();
    // `outcome` says `running` first (#222): a `wb` that is ended before its session leaves it so.
    lines.running(err);
    let ran = match &prepared {
        None => {
            // A role started with no issue's file (#116) runs as in revision 0.0.8, and the person is told (#126).
            if role != "breakdown" {
                let _ = writeln!(
                    err,
                    "{}: there is no issue's file, so the session runs without a view: it sees the whole checkout and its history, and the limits of its prompt rest on the prompt alone",
                    name
                );
            }
            let _ = writeln!(err, "{}: starts {} on the runner {}, in {}", name, role, runner, root);
            starter::session(root, &filled)
        }
        Some(prepared) => {
            let _ = writeln!(
                err,
                "{}: starts {} on the runner {}, in a view of {} at {}, in bubblewrap",
                name,
                role,
                runner,
                prepared.view.folder.display(),
                root
            );
            let organisation_folder = organisation.parent().filter(|folder| folder.is_dir());
            let plan = view::Plan {
                view: &prepared.view.folder,
                checkout: Path::new(root),
                binds: prepared.binds.clone(),
                organisation: organisation_folder,
                home: machine.home,
                new_session: view::new_session(),
                words: &filled,
            };
            view::run(machine.sandbox, &plan)
        }
    };
    let status = match ran {
        Ok(status) => status,
        Err(why) => {
            let _ = writeln!(err, "{}: {}", name, why);
            if let Some(prepared) = &prepared {
                let back = view::Back { failed: Some(why.clone()), ..Default::default() };
                let kept = view::tidy(name, &prepared.view, &back, err);
                lines.write("back", &back.text(kept.as_deref()), err);
            }
            lines.close(None, sessions::Outcome::Refused { check: "start", status: 1, message: why }, err);
            return 1;
        }
    };
    let seconds = began.elapsed().as_secs();
    let mut code = if status.success() { 0 } else { 1 };
    if code != 0 {
        let _ = writeln!(err, "{}: the session did not end with 0", name);
    }
    // What the session committed in its view comes back onto the branch, as far as it keeps to `changes`.
    if let (Some(prepared), Some(file)) = (&prepared, &handed.file) {
        let list = lines.file("back");
        let back = view::take_back(name, root, &prepared.view, &file.issue.changes, &prepared.build_files, list.as_deref(), err);
        let kept = view::tidy(name, &prepared.view, &back, err);
        lines.write("back", &back.text(kept.as_deref()), err);
        if back.refused_any() {
            code = 1;
        }
        if !back.taken.is_empty() {
            // How to look at what was taken (#159): the commits keep who wrote them, the tests, the checks.
            for (commit, _) in &back.taken {
                let _ = writeln!(err, "{}: to see what the commit changes, run:\n  git show --stat {}", name, commit);
            }
            say_push(name, role, root, prepared.view.branch_name(), err);
        }
    }
    // Without a view (#116), the tester, the build and the fix commit in the checkout: the same commands are said
    // when the branch checked out has new commits (#145).
    if let (None, Some((branch, head)), true) = (&prepared, &before, COMMITTED.contains(&role)) {
        if let Ok((now_branch, now)) = view::branch_of(root) {
            if now_branch == *branch && now != *head {
                say_push(name, role, root, branch.strip_prefix("refs/heads/").unwrap_or(branch), err);
            }
        }
    }
    lines.close(None, sessions::Outcome::Ended { status, seconds }, err);
    code
}

/// The roles whose sessions commit on the branch, and push nothing (#145).
const COMMITTED: [&str; 3] = ["tester", "build", "fix"];

/// Says the command that pushes `branch` and, after a build, the command that opens the change as a draft: the
/// pattern `draft_change` of `watchbill.yml`, filled. `wb` runs neither (#145): a person does.
fn say_push(name: &str, role: &str, root: &str, branch: &str, err: &mut dyn Write) {
    let _ = writeln!(err, "{}: nothing is pushed. To push the branch, run:\n  git push origin {}", name, branch);
    if role != "build" {
        return;
    }
    match prompt::pattern_of(root, "draft_change") {
        Ok(Some(line)) if prompt::is_plain_name(branch) => {
            let _ = writeln!(err, "{}: to open the change as a draft, where none is open yet, run:\n  {}", name, line.replace("{branch}", branch));
        }
        Ok(Some(_)) => {
            let _ = writeln!(err, "{}: the branch '{}' is not a plain name that fills `draft_change`: open the change as a draft at the git server.", name, branch.escape_debug());
        }
        Ok(None) => {
            let _ = writeln!(err, "{}: watchbill.yml has no `draft_change`, so the command that opens the change is not known: open it as a draft at the git server, where none is open yet.", name);
        }
        Err(why) => {
            let _ = writeln!(err, "{}: {}", name, why);
        }
    }
}

/// A view made for a session, before it is posted: the view, the runner's binds as paths, and `build_files` of
/// `watchbill.yml`.
struct Prepared {
    view: view::View,
    binds: Vec<(std::path::PathBuf, bool)>,
    build_files: Vec<String>,
}

/// Makes sure of what a session in a view needs, and makes the view (#126): the runner's binds, each from the
/// root and holding neither the checkout, the folder of the organisation's file nor the state folder; a state
/// folder to make the view below; bubblewrap that can make a sandbox; `build_files`. The error says why not.
fn prepare_view(
    root: &str,
    role: &str,
    issue: Option<&str>,
    paths: &[String],
    binds: &[confidentiality::Bind],
    machine: &Machine,
) -> Result<Prepared, String> {
    let mut resolved = Vec::new();
    for bind in binds {
        let path = match (bind.path.strip_prefix("~/"), machine.home) {
            (None, _) => std::path::PathBuf::from(&bind.path),
            (Some(rest), Some(home)) => home.join(rest),
            (Some(_), None) => {
                return Err(format!(
                    "the runner's bind {} is in the home folder, and HOME names no absolute folder. Set HOME, or take it out of `binds` in the organisation's file {}",
                    bind.path,
                    machine.organisation.display()
                ))
            }
        };
        resolved.push((path, bind.writable));
    }
    let mut kept: Vec<(std::path::PathBuf, &str)> = vec![(std::path::PathBuf::from(root), "the checkout")];
    if let Some(folder) = machine.organisation.parent().filter(|folder| !folder.as_os_str().is_empty()) {
        kept.push((folder.to_path_buf(), "the folder of the organisation's file"));
    }
    if let Some(state) = machine.state {
        kept.push((state.join("watchbill"), "the state folder of Watchbill, which holds the views"));
    }
    let forms = |path: &Path| -> Vec<String> {
        let mut found = vec![path.display().to_string()];
        if let Ok(real) = std::fs::canonicalize(path) {
            found.push(real.display().to_string());
        }
        found
    };
    for (path, _) in &resolved {
        for (other, what) in &kept {
            let meets = forms(path).iter().any(|one| forms(other).iter().any(|two| starter::meet(one, two)));
            if meets {
                return Err(format!(
                    "the runner's bind {} holds or lies in {}, {}, which a view never shows. Take it out of `binds` in the organisation's file {}",
                    path.display(),
                    what,
                    other.display(),
                    machine.organisation.display()
                ));
            }
        }
    }
    let state = machine
        .state
        .ok_or("neither XDG_STATE_HOME nor HOME names an absolute folder, and a view is made below the state folder of Watchbill")?;
    view::works(machine.sandbox)?;
    let settings = starter::settings(root)?;
    let build_files = issue::build_files_of(&settings)?.unwrap_or_default();
    let time = now().map(|seconds| prompt::utc(seconds).replace(':', ""))?;
    let base = format!("{}-{}-{}", time, role, issue.unwrap_or("none"));
    let made = view::make(root, paths, &state.join("watchbill").join("views"), &base)?;
    Ok(Prepared { view: made, binds: resolved, build_files })
}

/// `wb accept <issue>`: accepts the issue's file (`issue::accept`).
fn run_accept(parser: &mut Parser, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let used = |err: &mut dyn Write, why: String| {
        let _ = write!(err, "wb accept: {}\n\n{}", why, USAGE);
        2
    };
    let mut issue: Option<String> = None;
    let mut commit = true;
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(refused) => return used(err, format!("'{}' is not known", refused)),
        };
        match found {
            Word::Long(option) if option == "no-commit" => commit = false,
            Word::Value(value) if issue.is_none() => issue = Some(word(value.as_bytes())),
            found => {
                let refused = named(found, parser);
                return used(err, format!("'{}' is not known", refused));
            }
        }
    }
    let Some(issue) = issue else {
        return used(err, "an issue is needed".to_string());
    };
    if !issue::is_name(&issue) {
        return used(err, format!("'{}' is not an issue: an issue is letters, digits, '.', '_' and '-', and does not start with '.'", issue));
    }
    match init::repository_root(None) {
        Err(why) => {
            let _ = writeln!(err, "wb accept: {}.", why);
            2
        }
        Ok(None) => {
            let _ = writeln!(err, "wb accept: this folder is not in a git repository. Watchbill is set up in one.");
            2
        }
        Ok(Some(root)) => issue::accept(&root, &issue, commit, out, err),
    }
}

/// `wb status [<issue>] [--stage <heading>]` (#139): where each issue stands. It only reads.
fn run_status(parser: &mut Parser, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let used = |err: &mut dyn Write, why: String| {
        let _ = write!(err, "wb status: {}\n\n{}", why, USAGE);
        2
    };
    let mut issue: Option<String> = None;
    let mut stage: Option<&'static str> = None;
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(refused) => return used(err, format!("'{}' is not known", refused)),
        };
        match found {
            Word::Value(value) if issue.is_none() => issue = Some(word(value.as_bytes())),
            Word::Long(option) if option == "stage" && stage.is_none() => {
                let Ok(value) = parser.value() else {
                    return used(err, "'--stage' needs a heading".to_string());
                };
                let given = word(value.as_bytes());
                match status::heading_of(&given) {
                    Some(heading) => stage = Some(heading),
                    None => {
                        let names: Vec<&str> = status::STEPS.iter().chain([&status::QUESTIONS, &status::NOT_READ]).copied().collect();
                        return used(err, format!("'{}' is not a heading of `wb status`: {}", given.escape_debug(), names.join(", ")));
                    }
                }
            }
            found => {
                let refused = named(found, parser);
                return used(err, format!("'{}' is not known", refused));
            }
        }
    }
    if let Some(issue) = &issue {
        if !issue::is_name(issue) {
            return used(err, format!("'{}' is not an issue: an issue is letters, digits, '.', '_' and '-', and does not start with '.'", issue.escape_debug()));
        }
    }
    match init::repository_root(None) {
        Err(why) => {
            let _ = writeln!(err, "wb status: {}.", why);
            2
        }
        Ok(None) => {
            let _ = writeln!(err, "wb status: this folder is not in a git repository. Watchbill is set up in one.");
            2
        }
        Ok(Some(root)) => status::status(&root, issue.as_deref(), stage, sessions::state_folder().as_deref(), out, err),
    }
}

/// `wb view`, with the words after `view`, in the repository of `folder`, with the organisation's file at
/// `organisation`: `run` gives the folder it runs in and the path fixed in `wb`; this is so that a test can give
/// its own, as `prompt` does.
pub fn view_command(args: &[OsString], folder: &Path, organisation: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    run_view(&mut Parser::from_args(args.iter().cloned()), Some(folder), organisation, out, err)
}

/// `wb view <issue> <role> [--in | --out]` and `wb view <issue> <path>` (#173): what a session of a role will see,
/// and for one path whether it is in the view of each role, and why. It only reads: it makes no view, runs no
/// pattern, writes nothing and asks nothing of the network.
fn run_view(parser: &mut Parser, folder: Option<&Path>, _organisation: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let name = "wb view";
    let mut words: Vec<String> = Vec::new();
    let (mut inside, mut outside) = (false, false);
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(refused) => return used(err, name, &format!("'{}' is not known", refused)),
        };
        match found {
            Word::Long(option) if option == "in" || option == "out" => {
                let flag = if option == "in" { &mut inside } else { &mut outside };
                if *flag {
                    return used(err, name, &format!("'--{}' is given twice", option));
                }
                *flag = true;
            }
            // The third word is the folder of `--in` or `--out`, and is checked below.
            Word::Value(value) if words.len() < 3 => words.push(word(value.as_bytes())),
            found => {
                let refused = named(found, parser);
                return used(err, name, &format!("'{}' is not known", refused));
            }
        }
    }
    if inside && outside {
        return used(err, name, "'--in' and '--out' are not used together");
    }
    let (issue, second, folder_given) = match words.as_slice() {
        [issue, second] => (issue, second, None),
        [issue, second, folder] if inside || outside => (issue, second, Some(folder.as_str())),
        [_, _, third] => return used(err, name, &format!("'{}' is not known: a folder goes with '--in' or '--out'", third.escape_debug())),
        _ => return used(err, name, "an issue and a role or a path are needed"),
    };
    if !issue::is_name(issue) {
        return used(err, name, &format!("'{}' is not an issue: an issue is letters, digits, '.', '_' and '-', and does not start with '.'", issue.escape_debug()));
    }
    let role = STARTED.contains(&second.as_str()).then_some(second.as_str());
    if role.is_none() {
        if inside || outside {
            return used(err, name, "'--in' and '--out' go with a role, not with a path");
        }
        let found = starter::parts(second);
        if second.starts_with('/') || found.contains(&"..") || found.is_empty() {
            return used(
                err,
                name,
                &format!("'{}' is not a path from the root of the repository: it does not start with '/', holds no '..', and is not the root", second.escape_debug()),
            );
        }
    }
    let at = match folder_given {
        None => String::new(),
        Some(given) => {
            let found = starter::parts(given);
            if given.starts_with('/') || found.contains(&"..") {
                return used(
                    err,
                    name,
                    &format!("'{}' is not a folder from the root of the repository: it does not start with '/' and holds no '..'", given.escape_debug()),
                );
            }
            found.join("/")
        }
    };
    let found_root = match folder {
        None => init::repository_root(None),
        Some(folder) => init::repository_root_of(folder),
    };
    let root = match found_root {
        Err(why) => {
            let _ = writeln!(err, "{}: {}.", name, why);
            return 2;
        }
        Ok(None) => {
            let _ = writeln!(err, "{}: this folder is not in a git repository. Watchbill is set up in one.", name);
            return 2;
        }
        Ok(Some(root)) => root,
    };
    // `wb view` only reads (#254): a file that differs from `HEAD` is said, and `HEAD`'s file is the one read.
    let _read_only = match starter::read_only(&root) {
        Err(why) => {
            let _ = writeln!(err, "{}: {}.", name, why);
            return 2;
        }
        Ok((guard, differs)) => {
            if differs {
                let _ = writeln!(err, "{}", starter::drift_line(name));
            }
            guard
        }
    };
    let refused = |err: &mut dyn Write, why: &str| {
        let _ = writeln!(err, "{}: {}", name, why);
        1
    };
    if role == Some("breakdown") {
        return refused(err, "the breakdown runs without a view: it reads the whole code by its role");
    }
    let settings = match starter::settings(&root) {
        Ok(settings) => settings,
        Err(why) => return refused(err, &why),
    };
    let who = match role {
        Some(role) => format!("a session of the {}", role),
        None => "a session of any role".to_string(),
    };
    let not_started = "No prompt is handed out.";
    let file = match issue::for_prompt(Path::new(&root), issue) {
        Ok(Some(file)) => file,
        Ok(None) => {
            let why = format!(
                "the issue {} has no file {}/{}.yml, so {} would run without a view: it sees the whole checkout and its history",
                issue,
                issue::FOLDER,
                issue,
                who
            );
            return refused(err, &why);
        }
        Err(why) => return refused(err, &why.replace(not_started, "wb start refuses it, so no session of it runs, in a view or without.")),
    };
    if let Err(why) = issue::committed(Path::new(&root), issue) {
        return refused(err, &why.replace(not_started, "wb start refuses it, so no session of it runs, in a view or without."));
    }
    let Some(role) = role else {
        // `wb view <issue> <path>`: for each role with a view, in or out, and why.
        let build_files = match issue::build_files_of(&settings) {
            Ok(found) => found.unwrap_or_default(),
            Err(why) => return refused(err, &why),
        };
        let facts = match view::facts(&root, second) {
            Ok(facts) => facts,
            Err(why) => return refused(err, &why),
        };
        let mut any = false;
        let mut text = String::new();
        for role in issue::ROLES {
            match view::judge(&root, &file, role, &facts, &build_files) {
                Ok(view::Verdict::In { list, line, entry }) => {
                    any = true;
                    text.push_str(&format!("{}: in: {}, line {} of {}: {}\n", role, list, line, file.path, entry));
                }
                Ok(view::Verdict::Out(why)) => text.push_str(&format!("{}: out: {}\n", role, why)),
                Err(why) => return refused(err, &why),
            }
        }
        let _ = out.write_all(text.as_bytes());
        return if any { 0 } else { 1 };
    };
    let paths = view::paths(&file.issue, role);
    if !inside && !outside {
        // Only the paths: why a file is in or out is `wb view <issue> <path>`, the level and `view:` are in the stamp.
        let _ = out.write_all(view::list(&paths).as_bytes());
        return 0;
    }
    match view::by_folder(&root, &paths, &at, inside) {
        Ok(view::Folder::Listed(text)) => {
            let _ = out.write_all(text.as_bytes());
            0
        }
        Ok(view::Folder::Missing) => refused(err, &format!("'{}' is not in HEAD, so it is not a folder to look into.", issue::shown(&at))),
        Ok(view::Folder::File) => used(
            err,
            name,
            &format!("'{}' is a file, not a folder. Use `wb view {} {}` to see whether it is in a view.", issue::shown(&at), issue, issue::shown(&at)),
        ),
        Err(why) => refused(err, &why),
    }
}

/// The level of the repository, as `--parts` of the breakdown gives it: by the organisation's file where it is
/// there; else the line `confidentiality:` of `watchbill.yml`, which the organisation's list cannot be held to.
/// The error says why it is not known.
fn level_of(root: &str, settings: &yamlfile::Document, organisation: &Path) -> Result<String, String> {
    if organisation.exists() {
        let found = confidentiality::read(organisation)?;
        return confidentiality::repository_level(Path::new(root), &found);
    }
    let place = [yamlfile::Part::from("confidentiality")];
    if settings.has(&place) {
        return settings.text(&place).map_err(|why| why.to_string());
    }
    Err(format!(
        "there is no organisation's file {}, and watchbill.yml has no line confidentiality:, so the level is the most restrictive of a list that is not there",
        organisation.display()
    ))
}

/// `wb answer <issue> [--show | [--suggested] [<question> <answer>]...]` and `wb answer --change <n> […]` (#145).
/// The level the stamp holds for a session in a view (#209), and why where it is not the level the repository
/// gives. Where the organisation's file can be read, the rule decides: a `watchbill.yml` that gives no level
/// that holds (a level not on the list, or `levels` or `runners`) is the organisation's most restrictive level,
/// and the reason is returned to be said. Where the organisation's file is not there, the line `confidentiality:`
/// of `watchbill.yml` is the level; with none there is no list to take the most restrictive of, so the level is
/// `not known`, which says nothing on standard error. A file of the organisation that cannot be read is `not known`
/// too, with its reason.
fn stamp_level(root: &str, organisation: &Path) -> (String, Option<String>) {
    let not_known = "not known".to_string();
    let settings = match starter::settings(root) {
        Ok(settings) => settings,
        Err(why) => return (not_known, Some(why)),
    };
    if !organisation.exists() {
        return match level_of(root, &settings, organisation) {
            Ok(level) => (level, None),
            Err(_) => (not_known, None),
        };
    }
    match confidentiality::read(organisation) {
        Err(why) => (not_known, Some(why)),
        Ok(found) => match confidentiality::repository_level(Path::new(root), &found) {
            Ok(level) => (level, None),
            Err(why) => (found.strictest().to_string(), Some(why)),
        },
    }
}

/// `wb answer <issue> [--show | --suggested | <question> <answer>]` and `wb answer --change <n> […]` (#145).
fn run_answer(parser: &mut Parser, input: &mut dyn BufRead, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let used = |err: &mut dyn Write, why: String| {
        let _ = write!(err, "wb answer: {}\n\n{}", why, USAGE);
        2
    };
    let mut values: Vec<String> = Vec::new();
    let mut change: Option<String> = None;
    let mut suggested = false;
    let mut show = false;
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(refused) => return used(err, format!("'{}' is not known", refused)),
        };
        match found {
            Word::Value(value) => values.push(word(value.as_bytes())),
            Word::Long(option) if option == "change" && change.is_none() => match parser.value() {
                Ok(value) => change = Some(word(value.as_bytes())),
                Err(_) => return used(err, "'--change' needs a change".to_string()),
            },
            Word::Long(option) if option == "suggested" && !suggested => suggested = true,
            Word::Long(option) if option == "show" && !show => show = true,
            found => {
                let refused = named(found, parser);
                return used(err, format!("'{}' is not known", refused));
            }
        }
    }
    let (on_value, rest) = match (&change, values.as_slice()) {
        (Some(change), rest) => (change.clone(), rest.to_vec()),
        (None, [issue, rest @ ..]) => (issue.clone(), rest.to_vec()),
        (None, []) => return used(err, "an issue is needed, or --change and a change".to_string()),
    };
    if show && suggested {
        return used(err, "--show reads the questions and answers none, so it takes no --suggested".to_string());
    }
    if show && !rest.is_empty() {
        return used(err, "--show reads the questions and answers none, so it takes no question and answer".to_string());
    }
    if rest.len() % 2 == 1 {
        let why = match rest.len() {
            1 => "a question is named with its answer: wb answer <issue> <question> <answer>".to_string(),
            _ => "a question and its answer are two words, and the pairs repeat: put an answer of several words in quotes".to_string(),
        };
        return used(err, why);
    }
    let mut pairs: Vec<(String, String)> = Vec::new();
    for pair in rest.chunks(2) {
        if pair[1].chars().any(char::is_control) {
            return used(err, "an answer is one line, with no control character".to_string());
        }
        if pairs.iter().any(|(known, _)| answer::whole_name(&on_value, known) == answer::whole_name(&on_value, &pair[0])) {
            return used(err, format!("the question '{}' is named twice", pair[0].escape_debug()));
        }
        pairs.push((pair[0].clone(), pair[1].clone()));
    }
    let asked = match (suggested, show, pairs.is_empty()) {
        (_, true, _) => answer::Asked::Show,
        (true, _, _) => answer::Asked::Suggested(pairs),
        (false, _, true) => answer::Asked::Each,
        (false, _, false) => answer::Asked::Pairs(pairs),
    };
    if change.is_none() && !issue::is_name(&on_value) {
        return used(err, format!("'{}' is not an issue: an issue is letters, digits, '.', '_' and '-', and does not start with '.'", on_value.escape_debug()));
    }
    if change.is_some() && !prompt::is_plain_name(&on_value) {
        return used(err, format!("'{}' is not a change: a change is letters, digits, '.', '_' and '-', and does not start with '.' or '-'", on_value.escape_debug()));
    }
    let on = match &change {
        Some(change) => answer::On::Change(change),
        None => answer::On::Issue(&on_value),
    };
    match init::repository_root(None) {
        Err(why) => {
            let _ = writeln!(err, "wb answer: {}.", why);
            2
        }
        Ok(None) => {
            let _ = writeln!(err, "wb answer: this folder is not in a git repository. Watchbill is set up in one.");
            2
        }
        Ok(Some(root)) => {
            if !Path::new(&root).join(init::CONFIG_FOLDER).join(init::CONFIG_FILE).is_file() {
                let _ = writeln!(err, "wb answer: .watchbill/watchbill.yml is not there. Watchbill is not set up in this repository.");
                return 2;
            }
            answer::answer(&root, on, &asked, input, out, err)
        }
    }
}

/// `wb revert <issue> <role> [--round <n>]` and `wb revert --change <n> <role> [--round <n>]` (#142).
fn run_revert(parser: &mut Parser, err: &mut dyn Write) -> i32 {
    let used = |err: &mut dyn Write, why: String| {
        let _ = write!(err, "wb revert: {}\n\n{}", why, USAGE);
        2
    };
    let mut values: Vec<String> = Vec::new();
    let mut change: Option<String> = None;
    let mut round: Option<u64> = None;
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(refused) => return used(err, format!("'{}' is not known", refused)),
        };
        match found {
            Word::Value(value) if values.len() < 2 => values.push(word(value.as_bytes())),
            Word::Long(option) if option == "change" && change.is_none() => match parser.value() {
                Ok(value) => change = Some(word(value.as_bytes())),
                Err(_) => return used(err, "'--change' needs a change".to_string()),
            },
            Word::Long(option) if option == "round" && round.is_none() => {
                let Ok(value) = parser.value() else {
                    return used(err, "'--round' needs a round".to_string());
                };
                let given = word(value.as_bytes());
                match given.parse::<u64>() {
                    Ok(n) if n >= 1 && given.chars().all(|c| c.is_ascii_digit()) => round = Some(n),
                    _ => return used(err, format!("the value of '--round' is a whole number from 1, not '{}'", given.escape_debug())),
                }
            }
            found => {
                let refused = named(found, parser);
                return used(err, format!("'{}' is not known", refused));
            }
        }
    }
    let (issue, role) = match (&change, values.as_slice()) {
        (Some(_), [role]) => (None, role.clone()),
        (Some(_), _) => return used(err, "a change is reverted with a role and no issue".to_string()),
        (None, [issue, role]) => (Some(issue.clone()), role.clone()),
        (None, _) => return used(err, "an issue and a role are needed, or --change and a role".to_string()),
    };
    if !STARTED.contains(&role.as_str()) {
        return used(err, format!("'{}' is not a role of the revision: {}", role.escape_debug(), STARTED.join(", ")));
    }
    if let Some(issue) = &issue {
        if !issue::is_name(issue) {
            return used(err, format!("'{}' is not an issue: an issue is letters, digits, '.', '_' and '-', and does not start with '.'", issue.escape_debug()));
        }
    }
    if let Some(change) = &change {
        if !prompt::is_plain_name(change) {
            return used(err, format!("'{}' is not a change: a change is letters, digits, '.', '_' and '-', and does not start with '.' or '-'", change.escape_debug()));
        }
    }
    let on = match (&issue, &change) {
        (Some(issue), _) => revert::On::Issue(issue),
        (None, Some(change)) => revert::On::Change(change),
        (None, None) => return used(err, "an issue and a role are needed, or --change and a role".to_string()),
    };
    match init::repository_root(None) {
        Err(why) => {
            let _ = writeln!(err, "wb revert: {}.", why);
            2
        }
        Ok(None) => {
            let _ = writeln!(err, "wb revert: this folder is not in a git repository. Watchbill is set up in one.");
            2
        }
        Ok(Some(root)) => {
            if !Path::new(&root).join(init::CONFIG_FOLDER).join(init::CONFIG_FILE).is_file() {
                let _ = writeln!(err, "wb revert: .watchbill/watchbill.yml is not there. Watchbill is not set up in this repository.");
                return 2;
            }
            revert::revert(&root, on, &role, round, err)
        }
    }
}

/// Whether a stamp can hold a value as one plain line: it holds no control character, no line or
/// paragraph separator, and no white space at its start or end.
fn one_line(value: &str) -> bool {
    let broken = value.chars().any(|c| c.is_control() || c == '\u{2028}' || c == '\u{2029}');
    !broken && value.trim() == value
}

/// The name of the first value that a stamp cannot hold as one plain line, or `None`.
fn stamp_fault_in(values: &[(&'static str, String)]) -> Option<&'static str> {
    values.iter().find_map(|(name, value)| (prompt::STAMPED.contains(name) && !one_line(value)).then_some(*name))
}

/// One field of a stamp stays one line, and one that a comment may trim. The fault is said by name.
fn stamp_fault(values: &[(&'static str, String)]) -> Option<String> {
    stamp_fault_in(values).map(|name| {
        format!(
            "the value of '--{}' holds a control character or a line break, or white space at its start or end, and a stamp holds it as one line",
            name
        )
    })
}

/// `wb revision`: the version of the package, the hash of the revision and the manifest. It writes
/// nothing. The folder that holds `revision/` is an argument so that a test can give its own; `run`
/// gives the one fixed when `wb` was built.
pub fn revision(args: &[OsString], base: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    run_revision(&mut Parser::from_args(args.iter().cloned()), base, out, err)
}

fn run_revision(parser: &mut Parser, base: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let refused = match next(parser) {
        Ok(None) => None,
        Ok(Some(found)) => Some(named(found, parser)),
        Err(refused) => Some(refused),
    };
    if let Some(refused) = refused {
        let _ = write!(err, "wb revision: '{}' is not known\n\n{}", refused, USAGE);
        return 2;
    }
    match manifest::manifest(base) {
        Err(why) => {
            let _ = writeln!(err, "wb revision: {}", why);
            1
        }
        Ok(text) => {
            let _ = write!(out, "watchbill {}\nrevision {}\n\n{}", VERSION, manifest::hash_of(&text), text);
            0
        }
    }
}
