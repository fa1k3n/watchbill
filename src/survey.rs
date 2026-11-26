//! `wb init --survey` (#289): what needs judgement of the project's code style (naming, the structure of comments, the
//! handling of errors) is drafted by a model on this machine alone, into `.watchbill/project/style.md`.
//!
//! At `wb init` the project states no level yet, so the most restrictive applies: the code must not leave the machine.
//! The survey runs only on a runner the organisation's file marks `local: true`, whose `reach` stays on this machine,
//! in a sandbox with a network of its own (#195). The session has a view of the whole tree of `HEAD`, read-only but
//! `.watchbill/project/`, as the planner's view is made (#202), and nothing comes back but the draft, written into the
//! working tree and committed by nobody. Once a person commits it, `wb prompt` adds it to the prompt of every role,
//! after `all.md`, as `HEAD` has it (#211). It goes into every prompt, so a line of the draft that a prompt would read
//! as a placeholder, or that holds a control character, is left out and counted, as in `all.md`.

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::confidentiality::{self, Organisation, Runner};

/// The name every line of the survey is said after.
const NAME: &str = "wb init";

/// The draft, from the root of the repository.
pub const STYLE_MD: &str = ".watchbill/project/style.md";

/// The folder the session may write, from the root of the repository.
const PROJECT: &str = ".watchbill/project";

/// The prompt of the survey. It is `wb`'s own, as the survey is no role of the revision; `{checkout}` is filled.
const PROMPT: &str = "You survey the code style of the project at {checkout} for Watchbill, so that every later session \
writes code that matches it. The whole tree of HEAD is here, read-only, but for .watchbill/project/. Nothing is \
committed and nothing is pushed: do neither.

Read a sample of the code: several files of each kind and of each part of the tree. Then write one file, \
.watchbill/project/style.md, in Markdown, with three sections:

1. Naming: how files, types, functions, variables, constants and macros are named, with the prefixes and the case.
2. Comments: the header of a file, the comment before a function (Doxygen or another form), and comments in code.
3. Errors: how a function says it failed, and how its caller handles it.

Under each, say what the code does, and give one or two short examples taken from it, each with its path. Say only \
what the code shows; where it is mixed, say so and say which is the more common. The facts that need no judgement \
(the indent, the line endings, the length of lines, the project's own style files) are in \
.watchbill/project/all.md where it is there: do not repeat them. Write no text in braces with a lower-case word \
inside, as a prompt reads it as a placeholder, and no control character: a line that holds either is left out. Write no \
other file.";

/// The warning of #289, in the owner's words: the problem, the file that sets it, and what to put there. `why` is said
/// after the file, where there is more to say of it: that it is not there, or why it is refused.
pub fn warning(organisation: &Path, why: &str) -> String {
    format!(
        "no runner is marked local in {}{},\n\
         so the code style cannot be surveyed by a model on this machine.\n\
         \n  The organisation marks a runner whose tools and model run on this\n  machine, for example:\n\
         \n    runners:\n      local-a:\n        local: true\n        ...\n\
         \n  Then run:  wb init --survey\n",
        organisation.display(),
        why
    )
}

/// The organisation's file at `organisation`, where it has a runner marked `local: true`; else the warning.
pub fn local_organisation(organisation: &Path) -> Result<Organisation, String> {
    if crate::bounded::is_absent(organisation) {
        return Err(warning(organisation, " (there is no such file)"));
    }
    match confidentiality::read(organisation) {
        Ok(read) if read.runners.iter().any(|runner| runner.local) => Ok(read),
        Ok(_) => Err(warning(organisation, "")),
        Err(why) => Err(warning(organisation, &format!(" (the file is refused: {})", why.trim_end_matches('.')))),
    }
}

/// Whether `.watchbill/project/style.md` is in the working tree, or in `HEAD`.
pub fn style_is_there(root: &str) -> Option<&'static str> {
    if fs::symlink_metadata(Path::new(root).join(STYLE_MD)).is_ok() {
        return Some("in the working tree");
    }
    let in_head = crate::signers::git(root, &["cat-file", "-e", &format!("HEAD:{}", STYLE_MD)], None).is_ok();
    in_head.then_some("in HEAD")
}

/// The warning of `wb init` and `wb check`, where it is to be said: no runner of the organisation's file is local,
/// and, for `wb check`, no `style.md` is there.
pub fn warn(name: &str, organisation: &Path, err: &mut dyn Write) {
    if let Err(words) = local_organisation(organisation) {
        let _ = write!(err, "{}: {}", name, words);
    }
}

/// The runner the survey runs on: the one named, else the default, which must be marked local and able to start.
fn runner_of<'a>(organisation: &'a Organisation, path: &Path, named: Option<&str>) -> Result<&'a Runner, String> {
    let locals: Vec<&str> = organisation.runners.iter().filter(|runner| runner.local).map(|runner| runner.name.as_str()).collect();
    let name = match (named, &organisation.default) {
        (Some(name), _) => name,
        (None, Some(name)) => name.as_str(),
        (None, None) => {
            return Err(format!(
                "the organisation's file {} names no default runner. Name the runner with --runner: those marked local are {}",
                path.display(),
                locals.join(", ")
            ))
        }
    };
    let Some(runner) = organisation.runners.iter().find(|runner| runner.name == name) else {
        return Err(format!("the runner '{}' is not in the organisation's file {}: those marked local are {}", name.escape_debug(), path.display(), locals.join(", ")));
    };
    if !runner.local {
        return Err(format!(
            "the runner '{}' is not marked `local: true` in the organisation's file {}, and the survey runs only where its tools and its model are on this machine. Those marked local are {}: name one with --runner",
            name,
            path.display(),
            locals.join(", ")
        ));
    }
    if runner.start.is_none() {
        return Err(format!("the runner '{}' has no `start` in the organisation's file {}, so no session can be started on it", name, path.display()));
    }
    if let Some(away) = runner.reach.iter().find(|endpoint| !endpoint.on_this_machine()) {
        return Err(format!(
            "the runner '{}' reaches {}, which is not on this machine, and the code of the survey must not leave it. Its `reach` in the organisation's file {} names this machine alone (localhost, a loopback address) for a survey",
            name,
            away,
            path.display()
        ));
    }
    Ok(runner)
}

/// The words of the runner's `start`, filled: `{model}` and `{effort}` from its `planner`, which a local runner needs
/// where `start` holds either, as the model table is for a vendor's runner.
fn words_of(runner: &Runner, prompt: &str) -> Result<Vec<String>, String> {
    let words = runner.start.clone().unwrap_or_default();
    let names: Vec<&str> = words.iter().flat_map(|word| crate::prompt::placeholder_names(word)).collect();
    let needed = ["model", "effort"].into_iter().find(|name| names.contains(name));
    let (model, effort) = match (&runner.planner, needed) {
        (Some(planner), _) => (planner.model.as_str(), planner.effort.as_str()),
        (None, None) => ("", ""),
        (None, Some(name)) => {
            return Err(format!(
                "the `start` of the runner '{}' holds {{{}}}, and the runner has no `planner` to give the model and the effort: write them in its `start`, or clear it for the planner",
                runner.name, name
            ))
        }
    };
    let filled = crate::starter::fill(&words, model, effort, prompt);
    if filled.iter().any(|word| word.len() + 1 > crate::starter::LONGEST_WORD) {
        return Err("a word of the runner's `start`, filled, is 128 KiB or more, which Linux does not hand a program".to_string());
    }
    Ok(filled)
}

/// Makes sure `.watchbill/project/` of the checkout is a folder, not a link, or is not there. With `make`, a folder on
/// the way that is not there is made: only when the draft is taken, so that a survey that fails leaves nothing new in
/// the working tree.
fn project_folder(root: &str, make: bool) -> Result<(), String> {
    let mut at = Path::new(root).to_path_buf();
    for part in PROJECT.split('/') {
        at.push(part);
        match fs::symlink_metadata(&at) {
            Ok(found) if found.is_dir() => {}
            Ok(_) => return Err(format!("{} is a link or not a folder, and the draft is not written through it", at.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && !make => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                fs::create_dir(&at).map_err(|error| format!("{} could not be made: {}", at.display(), error))?;
            }
            Err(error) => return Err(format!("{} cannot be looked at: {}", at.display(), error)),
        }
    }
    Ok(())
}

/// The draft as it is written: each line that does not fit a prompt (`style::fit`) left out, and how many were.
fn fitted(text: &str) -> (String, usize) {
    let (mut kept, mut left_out) = (String::new(), 0);
    for line in text.split_inclusive('\n') {
        let ended = line.ends_with('\n');
        let bare = line.strip_suffix('\n').unwrap_or(line);
        let bare = bare.strip_suffix('\r').unwrap_or(bare);
        if crate::style::fit(bare) {
            kept.push_str(bare);
            if ended {
                kept.push('\n');
            }
        } else {
            left_out += 1;
        }
    }
    (kept, left_out)
}

/// Runs `wb init --survey` in the repository at `root`, on the runner `named` or the default, on the machine `machine`
/// says. Returns the exit status.
pub fn survey(root: &str, named: Option<&str>, machine: &crate::Machine, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let config = Path::new(root).join(crate::init::CONFIG_FOLDER).join(crate::init::CONFIG_FILE);
    if fs::symlink_metadata(&config).is_err() {
        let _ = writeln!(err, "{}: {}/{} is not there. Watchbill is not set up in this repository: run wb init first.", NAME, crate::init::CONFIG_FOLDER, crate::init::CONFIG_FILE);
        return 2;
    }
    let organisation = match local_organisation(machine.organisation) {
        Ok(found) => found,
        Err(words) => return crate::refusal::limit(err, NAME, words.trim_end()),
    };
    let runner = match runner_of(&organisation, machine.organisation, named) {
        Ok(runner) => runner,
        Err(why) => return crate::refusal::limit(err, NAME, why),
    };
    if let Some(where_) = style_is_there(root) {
        return crate::refusal::limit(
            err,
            NAME,
            format_args!("{} is already there, {}. wb init does not write over it: a person edits it, or removes it and commits that, first", STYLE_MD, where_),
        );
    }
    let prompt = PROMPT.replace("{checkout}", root);
    let words = match words_of(runner, &prompt) {
        Ok(words) => words,
        Err(why) => return crate::refusal::limit(err, NAME, why),
    };
    let binds = match crate::resolve_binds(root, &runner.binds, machine) {
        Ok(binds) => binds,
        Err(why) => return crate::refusal::limit(err, NAME, why),
    };
    if let Some(why) = words.first().and_then(|program| crate::outside_the_sandbox(program, &binds)) {
        return crate::refusal::limit(err, NAME, format_args!("{}. Bind it in `binds` of the runner '{}' in the organisation's file {}", why, runner.name, machine.organisation.display()));
    }
    let Some(state) = machine.state else {
        return crate::refusal::limit(err, NAME, "neither XDG_STATE_HOME nor HOME names an absolute folder, and a view is made below the state folder of Watchbill");
    };
    // The network of its own (#195): bubblewrap is tried with every namespace of a view, the network's among them.
    if let Err(why) = crate::view::works(machine.sandbox) {
        return crate::refusal::limit(
            err,
            NAME,
            format_args!("the survey runs only in a sandbox with a network of its own (#195), so that the code stays on this machine, and none can be made here: {}", why),
        );
    }
    // Only looked at here: nothing is made in the working tree until the draft is taken.
    if let Err(why) = project_folder(root, false) {
        return crate::refusal::limit(err, NAME, why);
    }
    let seconds = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|since| since.as_secs()).unwrap_or(0);
    let base = format!("{}-survey", crate::prompt::utc(seconds).replace(':', ""));
    let below = state.join("watchbill").join("views");
    let (view, size) = match crate::view::make_whole(root, &below, &base) {
        Ok(made) => made,
        Err(why) => return crate::refusal::limit(err, NAME, format_args!("the view of HEAD could not be made: {}", why)),
    };
    let code = in_view(root, runner, &words, &binds, &view, &size, state, machine, out, err);
    crate::view::discard(&view);
    code
}

/// The part of the survey that runs in the view, which the caller removes after.
#[allow(clippy::too_many_arguments)]
fn in_view(
    root: &str,
    runner: &Runner,
    words: &[String],
    binds: &[(std::path::PathBuf, bool)],
    view: &crate::view::View,
    size: &crate::view::Size,
    state: &Path,
    machine: &crate::Machine,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    // The folder the session writes in, in the view, made where `HEAD` has none. `.watchbill` of the view is a folder
    // (`view::bind_points`); `.watchbill/project` of `HEAD` must be one too, never a link that the bind and the draft
    // would go through.
    let drafts = view.folder.join(PROJECT);
    match fs::symlink_metadata(&drafts) {
        Ok(found) if found.is_dir() => {}
        Ok(_) => {
            return crate::refusal::limit(
                err,
                NAME,
                format_args!("{} of HEAD is a link or not a folder, and the session writes its draft there. Nothing is run", PROJECT),
            )
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if let Err(error) = fs::create_dir(&drafts) {
                return crate::refusal::limit(err, NAME, format_args!("{} could not be made in the view: {}", PROJECT, error));
            }
        }
        Err(error) => return crate::refusal::limit(err, NAME, format_args!("{} of the view cannot be looked at: {}", PROJECT, error)),
    }
    let made = match crate::push::make_out(state, &view.folder) {
        Ok(made) => made,
        Err(why) => return crate::refusal::limit(err, NAME, why),
    };
    let code = run_and_take(root, runner, words, binds, view, size, &made, machine, out, err);
    crate::push::remove_out(&made);
    code
}

#[allow(clippy::too_many_arguments)]
fn run_and_take(
    root: &str,
    runner: &Runner,
    words: &[String],
    binds: &[(std::path::PathBuf, bool)],
    view: &crate::view::View,
    size: &crate::view::Size,
    made: &Path,
    machine: &crate::Machine,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let net = if runner.reach.is_empty() {
        None
    } else {
        let folder = made.join(crate::reach::SOCKET_FOLDER);
        match crate::reach::program().and_then(|exe| crate::reach::Proxy::start(&folder, &runner.reach, None).map(|proxy| (proxy, crate::reach::Net { folder, exe }))) {
            Ok(found) => Some(found),
            Err(why) => return crate::refusal::limit(err, NAME, why),
        }
    };
    // What it will write, said first.
    let _ = writeln!(out, "The survey writes {} from the draft the session leaves there, and nothing else. It commits nothing.", STYLE_MD);
    crate::refusal::inform(err, format_args!("{}: the view holds the whole tree of HEAD: {} files, {} bytes, read-only but {}/", NAME, size.files, size.bytes, PROJECT));
    let _ = writeln!(err, "{}: a change not committed is not in the view: the session sees HEAD", NAME);
    match &net {
        None => {
            let _ = writeln!(err, "{}: the session reaches no network: the runner '{}' has no reach", NAME, runner.name);
        }
        Some(_) => {
            let all: Vec<String> = runner.reach.iter().map(ToString::to_string).collect();
            let _ = writeln!(err, "{}: the session reaches only {}, on this machine, through wb's proxy (reach of the runner '{}')", NAME, all.join(", "), runner.name);
        }
    }
    let _ = writeln!(err, "{}: starts the survey on the local runner {}, in a view of {} at {}, in bubblewrap", NAME, runner.name, view.folder.display(), root);
    let organisation_folder = machine.organisation.parent().filter(|folder| folder.is_dir());
    let plan = crate::view::Plan {
        view: &view.folder,
        checkout: Path::new(root),
        binds: binds.to_vec(),
        organisation: organisation_folder,
        home: machine.home,
        new_session: crate::view::new_session(),
        words,
    };
    let (proxy, net) = match net {
        Some((proxy, net)) => (Some(proxy), Some(net)),
        None => (None, None),
    };
    let _held = crate::interrupt::hold();
    let ran = crate::view::run_reaching(machine.sandbox, &plan, crate::view::Layout::Survey, Some(made), net.as_ref(), None, NAME, err);
    if let Some(count) = proxy.map(crate::reach::Proxy::stop).filter(|count| *count > 0) {
        let _ = writeln!(err, "{}: the proxy refused {} connection(s) that the session asked for, as reach of the runner '{}' does not let them through", NAME, count, runner.name);
    }
    match ran {
        Err(why) => return crate::refusal::limit(err, NAME, format_args!("{}. Nothing written", why)),
        Ok(status) if !status.success() => {
            let _ = writeln!(err, "{}: the session did not end with 0 ({}). Nothing written.", NAME, status);
            crate::refusal::answer();
            return 1;
        }
        Ok(_) => {}
    }
    take(root, &view.folder, out, err)
}

/// Takes the draft back from the view: a plain UTF-8 file of 1 MiB or less, never through a link, written new into the
/// working tree. Nothing else of the view is looked at.
fn take(root: &str, view: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let from = view.join(STYLE_MD);
    let bytes = match crate::bounded::read_plain(&from, crate::bounded::TEXT_CAP) {
        Ok(bytes) => bytes,
        Err(crate::bounded::Fault::NotFound) => {
            let _ = writeln!(err, "{}: the session left no {}. Nothing written.", NAME, STYLE_MD);
            crate::refusal::answer();
            return 1;
        }
        Err(fault) => return crate::refusal::limit(err, NAME, format_args!("the session's {} is not taken: {}. Nothing written", STYLE_MD, fault)),
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return crate::refusal::limit(err, NAME, format_args!("the session's {} is not taken: it is not UTF-8 text. Nothing written", STYLE_MD));
    };
    // The draft goes into every prompt once committed, and a person reads it in a terminal: what would be read as a
    // placeholder, or holds a control character, is left out, as in `all.md`.
    let (kept, left_out) = fitted(text);
    // The checkout again, as the session ran: the folder is no link, made now, and nothing is written over.
    if let Err(why) = project_folder(root, true) {
        return crate::refusal::limit(err, NAME, why);
    }
    let to = Path::new(root).join(STYLE_MD);
    // `create_new`: never over a file that came to be there while the session ran.
    let mut file = match fs::OpenOptions::new().write(true).create_new(true).open(&to) {
        Ok(file) => file,
        Err(error) => return crate::refusal::limit(err, NAME, format_args!("{} could not be written: {}. Nothing written", STYLE_MD, error)),
    };
    if let Err(error) = file.write_all(kept.as_bytes()) {
        let _ = fs::remove_file(&to);
        return crate::refusal::limit(err, NAME, format_args!("{} could not be written: {}. Nothing written", STYLE_MD, error));
    }
    crate::refusal::inform(out, format_args!("Wrote {}.", STYLE_MD));
    if left_out > 0 {
        let plural = if left_out == 1 { "line" } else { "lines" };
        let _ = writeln!(out, "{} {} left out of the draft: a prompt would read it as a placeholder, or it holds a control character.", left_out, plural);
    }
    let _ = writeln!(
        out,
        "It is not committed: read it, correct it and commit it. Once it is committed, wb prompt adds it to the prompt of every role, after all.md; until then wb prompt and wb start refuse, as it is not as HEAD has it (#211)."
    );
    0
}
