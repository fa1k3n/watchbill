//! The command `wb`.

use lexopt::{Arg, Parser};
use std::ffi::OsString;
use std::io::{BufRead, Write};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub mod answer;
pub mod bounded;
pub mod brought;
pub mod check;
pub mod compare;
pub mod confidentiality;
pub mod detect;
pub mod doctor;
pub mod finished;
pub mod hooks;
pub mod init;
pub mod interrupt;
pub mod kept;
pub mod manifest;
pub mod notes;
pub mod plan;
pub mod prompt;
pub mod push;
pub mod reach;
pub mod refusal;
pub mod revert;
pub mod rules;
pub mod issue;
pub mod layout;
pub mod session_status;
pub mod sessions;
pub mod stamps;
pub mod signers;
pub mod starter;
pub mod status;
pub mod upgrade;
pub mod view;
pub mod watchdog;
mod word;
pub mod yamlfile;

/// The version of the package.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What `wb --help` prints. The words are those of the Python `wb`.
pub const USAGE: &str = "usage:
  wb init [--yes | -y] [-q | --quiet]
  wb check [-q | --quiet]
  wb doctor [-q | --quiet]
  wb upgrade [--dry-run | -n] [-q | --quiet]
  wb plan add [<name>] (-F <file> | -m <text>... | --from <adapter> <number>)
            [--source <address>] [--level <level>] [--hash-only] [-q | --quiet]
            -F is --file and -m is --message
            or list [-q | --quiet]
            or show <name> [-q | --quiet]
            or edit <name> [-q | --quiet]
            or accept <name> [--no-commit | -n] [-q | --quiet]
            or remove <name> [-q | --quiet]
  wb accept <issue> [--no-commit | -n] [-q | --quiet]
  wb start <role> [<issue>] [--runner <name>] [--prompt] [--ignore-local] [--ignore-kept]
            [--ignore-branch] [--no-push] [-q | --quiet] [the options of prompt]
            or <role> <issue> --take-back
            or --list [-v | --verbose]
  wb status [<issue>] [--stage <heading>] [-q | --quiet]
            or --session <hash> [-q | --quiet]
  wb view <issue> (<role> [--in | --out] | <path>) [-q | --quiet]
            or --kept [-v | --verbose]
  wb answer (<issue> | --change <n>) [--show | [--suggested] [<question> <answer>]...]
            [-q | --quiet]
  wb revert (<issue> | --change <n>) <role> [--round <n>] [-q | --quiet]
  wb clean [--dry-run | -n] [<view>]
            [-q | --quiet]
  wb prompt <role> [<issue>] [--issue <text>] [--change <text>]
            [--requirements <text>] [--round <text>] [--checkout <text>]
            [--branch <text>] [--issue_address <text>]
            [--change_address <text>] [--classed <text>]
            [--classed_touched <text>] [--description <text>]
            [--brought <text>] [--questions <text>]
            [--change_comment <text>] [--needs_person <text>]
            [--runner <name>] [--parts] [--stamp] [--show-origin] [-q | --quiet]
  wb revision [-q | --quiet]
  wb compare [--from <path>] <revision> <revision> [-q | --quiet]
  wb --version

  init      set Watchbill up in the git repository of this folder: propose what
            the repository says about itself (its name, the git server of
            origin, a folder of requirements, the CI driver), ask one line per
            answer, show the file and write .watchbill/watchbill.yml at the
            root of the repository. --yes (-y) takes what it found and asks
            nothing. A file that is already there is never changed. -q
            (--quiet) leaves out the lines that confirm what was written, and
            not what it says before it writes or the advice after
  check     say whether the repository still is at the revision it is pinned to:
            each file under .watchbill/revision/ against the manifest, the
            manifest against the revision line of watchbill.yml, and that
            revision against the installed wb, and the project text for a
            placeholder the revision does not know. Ends with 0 when all of it
            holds, 1 when something differs. Writes nothing. -q (--quiet)
            leaves out the line that says all holds, and no difference
  doctor    say what on this machine Watchbill needs, one line to a check, each
            ok, warning or fault: bubblewrap and user namespaces, git, the
            tools the patterns of watchbill.yml call, the organisation's file
            and its default runner, each bind of that runner, each hook of the
            runner's settings whose program the sandbox does not hold, each
            endpoint of the runner's reach, the command needs run once, and
            the ports that listen on this machine. A session in a view has a
            network of its own and reaches only the endpoints of reach,
            through the proxy of wb: an endpoint on this machine, and a port
            whose number reach names, are warnings, as wb cannot tell whether
            it has a login. A runner with no reach is a warning: a session on
            it reaches no network. It reads only and asks nothing of the
            network. Ends with 0 when no check is a fault, 1 when one
            is. -q (--quiet) leaves out the ok lines, and not a warning or a fault
  upgrade   pin the repository to the installed revision: copy its files over
            .watchbill/revision/, removing those it does not have, and its
            manifest to .watchbill/revision.sha256, set the lines watchbill:
            and revision: of watchbill.yml, and add each pattern the installed
            wb init would write that the file lacks; no other line is changed.
            It says each write first, then writes, runs the check of wb check
            and says the command that commits the change: it commits nothing.
            --dry-run (-n) says the same and writes nothing. Ends with 0 when it is
            done and the check holds, or there is nothing to do; 1 when it is
            refused (a change under .watchbill/ that is not committed, an
            installed version older than the pinned one) or the check does not
            hold; 2 when it was used wrongly or the repository is not set up.
            -q (--quiet) leaves out the lines that confirm the writes and the
            line that says there is nothing to do, and not what it says before
            it writes, a refusal, a difference of the check, or that nothing
            is committed
  plan      the plans of the jobs, kept in git (docs/revision.md, point 15).
            wb plan add takes a job in. Its text is given in one of three
            ways: -F <file>, where -F - reads standard input, a pipe or a
            here-document; -m <text>, which may be given more than once and is
            joined and cleaned up of whitespace as git commit -m does it; or
            --from <adapter> <number>, which runs the pattern issue_text of
            watchbill.yml where its tracker: is the adapter, and names the job
            <adapter>/<number>. -F is --file and -m is --message, as in git. A
            file is read only where it is a plain file of 1 MiB or less, and a
            text that is empty or not UTF-8 is refused. The text is stored as
            a git blob under its hash, and the name of the job is the ref
            refs/watchbill/<name>: a commit whose tree holds job.yml and the
            text under sources/<hash>. job.yml holds the first name, every
            name and, for each source, its hash, --source <address> or none,
            the level, the git identity of the person and the time. With no
            name the job is named by the first 12 hex digits of the hash. A
            name may hold / (github/54); a name that git check-ref-format
            refuses, such as one with :, is refused. Other text under a name
            that is there adds a source, as a new commit after the one before;
            the same text again changes nothing; text that is already a job
            under another name gives that job this name too. Every name of the
            job is moved to its new commit together, or none is. --level
            <level> is a level of the organisation's file no higher than the
            repository's, which is taken when none is given. --hash-only keeps
            the hash and the source and not the text, where the organisation's
            file allows it with jobs: hash_only: allowed. It says what it will
            write first, asks nothing of the network unless --from runs a
            pattern, and only queues: it runs no prompt and starts no session.
            wb plan add ends with 0 when the job is taken in or nothing
            changes, 1 when it is refused, 2 when it was used wrongly or the
            repository is not set up. wb plan list prints each plan once, by
            its first name, one to a line, and nothing else; a plan removed is
            not listed, and a ref under refs/watchbill/ that is not a job is
            passed over and said on standard error. A plan whose tip says
            removed, on a ref that is not as wb writes it, is said on standard
            error as unknown. wb plan show <name> prints
            the plan's names, each source with its hash, where it came from,
            whether its text is kept, its level, who added it and when, then
            the issue's file .watchbill/issues/<name>.yml, read where wb
            status reads it (the issue's branch, else the working tree), and
            says which, and its state: not planned, not accepted, not accepted
            (edited after the acceptance), not accepted (a source added after
            the acceptance), not accepted (planned again after the
            acceptance), or accepted by whom and when; the planner's stamps
            are read from the job's ref. Where the tip of a ref that is not as
            wb writes it says the plan is removed, it prints the plan's name,
            the line that names the first commit that is not as wb writes it,
            and state: unknown, and ends with 0 as it does for a bad ref with
            a live plan. A source or a [stamp] planner of the same second as
            the acceptance or later voids it,
            and no prompt of the issue is handed out until wb plan accept
            again; a stamp that a [revert] took back counts for nothing. wb
            plan edit <name> opens the issue's file in the editor git would
            use (GIT_EDITOR, core.editor, VISUAL, EDITOR), on the issue's
            branch, from the file as HEAD has it, and commits it alone with
            the message Edit the issue's file (<name>); where nothing changed
            it commits nothing, and where the editor fails, the file no longer
            holds its form or names another issue, it commits nothing and
            leaves the edit. wb plan accept <name> [--no-commit | -n] accepts
            the issue's file, also one with no plan, as above. wb plan remove
            <name> adds a commit with the marker removed after the plan's, moves
            every name of it there, and rewrites nothing; the name stays taken.
            It says first, for each ref it moves, which rounds stop counting
            and which count again.
            It is refused while a branch of any of its names holds work the
            default branch does not, or this machine keeps a view of it with
            work not taken back. A name with / has no issue's file until point
            15, and show, edit and remove of a name that is no plan are refused.
            Ends with 0 when it is done or nothing changes, 1 when it is
            refused, 2 when it was used wrongly or the repository is not set up.
            -q (--quiet) leaves out the lines that confirm what a subcommand
            wrote and the lines that say there is nothing to do, and not what it
            says before it writes, a refusal, that an acceptance holds no
            longer, or what list and show print
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
            --show-origin prints the whole prompt as it is, and before each
            part a line that says its kind (revision, issue, project), its
            path, where it was read from and its hash, the hash of its part
            line of the stamp. Below that line stands one line for each
            placeholder the part holds, and where its value came from. Not
            with --parts or --stamp.
            --runner names the runner the prompt is handed to. The planner
            needs it, and is handed out only to a runner that
            /etc/watchbill/organisation.yml clears for the planner up to the
            level of the repository, the line confidentiality: of
            watchbill.yml. Its stamp then has the model and effort of that
            file, and --parts adds the level and the runner. Ends with 0 when
            the prompt is printed, 1 when it is refused, 2 when it was used
            wrongly or the repository is not set up. Writes nothing. -q
            (--quiet) is taken and changes nothing: what it prints is what a
            script reads
  start     start a session of one role, planner, tester, build, review or
            fix: the role is the word after start. The issue is needed by
            all but the review and the fix, which may be given --change
            and no issue. It checks what prompt checks, that a fix
            follows a review, that no classed code is in its way (classed: of
            watchbill.yml), that the runner has start in the organisation's
            file, and that standard input and output are a terminal; then
            write the stamp of an issue as a commit on refs/watchbill/<issue>,
            signed by a key of .watchbill/signers with start (refused where
            the default branch has no list, or one with no key line), and push
            it to origin with no force: a push refused as not a
            fast-forward ends with 1 and starts nothing, and with no origin
            the stamp stays local, said. Its copy is posted by
            issue_comment of watchbill.yml where it is there (a review or a
            fix of a change with no issue posts the stamp by change_comment),
            and the runner's start is run in the root of the checkout. The
            runner is --runner, or the organisation's default. The round is
            worked out from the stamp commits on the job's ref (from
            change_comments for a change with no issue), the change from
            change_of_branch; --round and --change win. --prompt, --stamp and
            --parts print as prompt does, and post and start nothing. A view
            of the issue kept with work not taken back, from any role, stops the
            start with 1 before anything is posted; --ignore-kept goes on
            without it, and the stamp says so. The commits of the session come
            back onto the branch of the stamp, made where it is not there.
            Where it is there and HEAD is not its commit, the start ends with
            1 before anything is posted. A checkout on another branch, but
            the default branch before the branch of the stamp is there, stops
            the start with 1 before anything is posted, and --ignore-branch
            goes on. After the tester, the build or the fix of an issue whose
            commits all came back, and whose session ended with 0, it pushes
            the branch to origin with git, never with force, and after a build
            with no change open it opens one as a draft by draft_change, with
            the description the session wrote; without change_of_branch and
            draft_change, or with --no-push, it says the commands and runs
            neither. --take-back, given with the role
            and the issue alone, starts and posts nothing: it says what it will
            take, then takes back the commits of the newest view of that role
            and issue that was kept, through the checks of the end of a
            session; 0 when all of it is taken and the view removed, 1 when
            anything is refused or left. --list, given alone, prints the
            roles of the pinned revision that start starts, one to a line and
            nothing else. With -v (--verbose) each line adds whether the role
            runs in a view or in the whole tree, and whether the
            organisation's file clears a runner for it, for the planner a
            runner with start and planner up to the level of the repository.
            --show-origin prints as prompt does. Ends with the session: 0 when
            it ended with 0, else 1; 1 when it is refused, 2 when it was used
            wrongly or the repository is not set up. -q (--quiet) leaves out
            the size of a view of the whole tree and how to see what a commit
            changes, and not a refusal, what it says before it posts or
            writes, what is not pushed and the commands to push, or what
            --prompt, --stamp, --parts, --show-origin and --list print
  revision  print the version of the package, the hash of its revision and the
            manifest: the hash of every file the revision holds. Writes
            nothing. -q (--quiet) is taken and changes nothing: what it prints
            is what a script reads
  compare   say what changed between two revisions, each named by its version
            or its hash: one line for each file added, removed or changed, then
            a unified diff of each changed file. The installed revision is one;
            the other is that of the repository at --from, the root of a
            repository pinned to it. Ends with 0 when they are the same, 1 when
            they differ or the files of a revision are refused, 2 when a
            revision is not found or it was used wrongly. Writes nothing. -q
            (--quiet) is taken and changes nothing: what it prints is what a
            script reads
  accept    accept the work of an issue. The issue's file
            .watchbill/issues/<issue>.yml is accepted by wb plan accept, which
            does this: read it, hold it to its form, refuse it while it holds a
            question, say what will be written, and write
            .watchbill/issues/<issue>.accepted with the SHA-256 of the file, the
            git user.name and user.email, and the time in UTC. An acceptance
            that is there is never written over. Then commit it alone, on the
            issue's branch, with nothing else: it refuses first when the
            checkout is not on that branch or the issue's file is not committed
            as it is. --no-commit (-n) only writes it and says the commit. Where
            the issue's file is not accepted, a plan or not, wb accept ends with
            2 and names wb plan accept <name>. Where it is accepted as it
            stands, it accepts the work: it says first accepting the work
            <commit> (not run: <roles>), then writes
            .watchbill/issues/<issue>.finished with the head commit of the
            branch, who and when, and commits it alone. It ends with 1 when no
            role has committed work, a question is open, a view is kept with
            work not taken back, or the change touches classed code, which needs
            a person as verifier. A later commit on the branch voids it. The job
            is then done. Ends with 0 when it is accepted, 1 when it is refused,
            2 when it was used wrongly or the repository is not set up. -q
            (--quiet) leaves out the check of the lists and the lines that
            confirm the write and the commit, and not what it will write or
            commit, the command to commit or push, or a refusal
  status    list the issues in the flow under headings, as git status lists
            files: each issue under the last step it has done (Not planned,
            Planned, Accepted, Tested, Built, Reviewed, Fixed), and last the
            issues that have a question no [answers] or [answer] line has
            closed. An issue is in the flow when its branch, local or at
            origin, holds a commit that the default branch of origin does not:
            git alone says which, and fetches nothing. Its line shows the
            branch, the open change as #<n> (change_of_branch) and how many
            commits are not pushed. The stamps are the commits on the job's
            ref that count. The questions and answers of a job whose ref
            holds a stamp, a commit of wb plan or a questions commit are
            read from the ref alone, from the first of them on, and no
            issue_comments is run for it, so a project with no
            issue_comments reads it; those of a job begun before, whose ref
            holds none of them, come from issue_comments. For
            an open question read from the ref it says on
            standard error that an answer typed at the tracker is not read.
            An acceptance that a source or a planner round after it voided
            puts the issue under Planned.
            <issue> prints that issue, in the flow or not, with its title
            (issue_address) and what it was worked out from; --stage <heading>
            prints that heading only, whatever its case. A job whose work is
            accepted, or whose change is merged, is done: it is not listed,
            and <issue> prints Done and what closed it, and a line accepted:
            for each acceptance, by its object. For one issue it also prints
            a line kept: <role> round <n>, <k> commits not taken,
            <folder> for each view kept with work not taken back, and cut off:
            <role> round <n>, <folder> for each session whose wb ended before
            it did. With no issue it says on standard error how many such
            views are kept on this machine. A view of over 200 commits is said
            as the first 200 and 'and <k> more', and each view is read within
            20 seconds. Ends with 0 when every issue could be read, 1 when a
            pattern failed for one (it is listed under Not read), 2 when it
            was used wrongly or the repository is not set up. Writes nothing
            but a temporary repository in $TMPDIR, removed after, where it
            reads a kept view. --session <hash>, with no issue and no --stage,
            prints one session from git alone: the lines of its stamp, Base:
            the commit it was made from, the commits that a note names for it,
            newest last, Last: the newest of them, and Ended: how its start
            ended where this machine holds the lines of the start, else 'not
            known on this machine'. <hash> is the commit of a stamp that
            counts, on a job's ref, or a unique start of it of 12 or more hex
            digits. A note is not signed and counts for nothing: it is only
            where the commits are found. A hash that names no stamp that
            counts, or not one only, ends with 1, and fewer than 12 digits
            with 2. It only reads: it writes nothing and asks nothing of
            the network. -q (--quiet) is taken and changes nothing: what it
            prints is what a script reads
  view      show what a session of a role will see, before it starts. wb view
            <issue> <role> prints the paths of its view, one to a line, the
            list the start makes the view from, and nothing else. --in
            [<folder>] prints what of HEAD the view holds in the folder (the
            root when none is given), one level deep: each file in it, and
            <count><tab><folder>/ for each folder below it. --out prints the
            same for what the view leaves out. A folder is a path from the
            root, with or without a closing /. The level and the line view:
            are in wb prompt <role> <issue> --stamp.
            wb view --kept takes no other word but -v. It lists the views
            kept on this machine for this repository whose session does not
            run, one to a line: the folder, the role, round <n>, issue
            <issue>, and <k> commits not taken <m> paths uncommitted, or not
            read where git could not read the view. -v (--verbose) adds below
            each line why it was kept, each commit not taken, each path left
            uncommitted, and why a view was not read. It writes nothing.
            wb view <issue> <path> says for each role whether the path, from
            the root, is in its view: in, with the list and the line of the
            issue's file that hold it, or out, and why. A path named as a role
            is written ./<path>. Ends with 0 when the path is in the view of a
            role or the view is shown, 1 when it is in none, the folder is not
            in HEAD or the issue has no accepted, committed file, 2 when it
            was used wrongly (a file given as the folder too) or the
            repository is not set up. Writes nothing. -q (--quiet) is taken
            and changes nothing: what it prints is what a script reads
  revert    take an issue back to before a step, deleting nothing:
            wb revert <issue> <role> [--round <n>] writes one marker whose
            first line is [revert] <role> round <n>, for the last stamp of the
            role, or round <n>, as a commit on refs/watchbill/<issue>, signed
            and pushed to origin as a stamp is, and posts its copy by
            issue_comment of watchbill.yml where it is there. From that stamp
            up to the marker no stamp, question or answer counts, for wb start
            <role>, wb prompt and wb status. wb revert --change <n> <role>
            posts it on a change with no issue, by change_comment.
            The branch, the issue's file and its acceptance, and the change are
            not touched: taking commits back is git's. The planner is taken
            back with git, not by a marker. A job that is done is refused. Ends
            with 0 when the marker is written, 1 when it was refused or its
            writing failed, 2 when it was used wrongly or the repository is not
            set up. -q (--quiet) is taken and changes nothing: what it says,
            that the commits stay, is what a person must know
  clean     remove the views kept on this machine for this repository that
            have nothing left in them: those whose session ended and that hold
            no commit not taken back (a commit a take-back took is taken) and
            no path left uncommitted, and those of a job that is done whose
            commits are all in the default branch. It says first, on standard
            output, Removing <folder> for each view it will remove and Keeping
            <folder>: <what is left> for each it keeps, then removes.
            --dry-run (-n) says Would remove and removes nothing. <view>, the
            name of the folder of a view or its whole path, as wb view --kept
            prints it, removes that view although work is left in it, and says
            first each commit (20 at most) and each path (20 at most) it
            loses. Naming a view is the one waiver of the work left in it. A
            view that git could not read, or whose session runs, is never
            removed, named or not. It never removes the checkout, a branch or
            the lines of a session. Ends with 0 when it is done or there is
            nothing to remove, 1 when a named view is not removed, 2 when it was
            used wrongly or this folder is not in a git repository. -q (--quiet)
            leaves out the Keeping lines, and not Removing or Would remove,
            which it says before it removes
  answer    answer the open questions of an issue: wb answer <issue> reads
            the questions from the job's ref where it holds a stamp, a
            commit of wb plan or a questions commit (none is open before
            the first questions commit), else the whole comments by
            issue_comment_bodies of
            watchbill.yml, shows each open question in full, one at a time,
            and asks: Enter takes the suggestion, a number takes that choice,
            other text is the answer in words; a number that is not a choice
            is asked again. Then it says what it will write or post. For a
            job whose ref holds a stamp, a commit of wb plan or a questions
            commit it writes one commit per role on refs/watchbill/<issue>,
            signed with a key of .watchbill/signers that holds the role
            answer, pushes it, and posts a copy by issue_comment where that
            is set up, and needs neither issue_comment_bodies nor
            issue_comment; for a job begun before, whose ref holds none of
            them, it posts one [answers] comment per role by issue_comment.
            Nothing is written or posted before every question is answered,
            and nothing after Ctrl-C. --suggested takes every
            suggestion and names the questions with none. --show prints every
            open question in full, as it is laid out when asked, to standard
            output, and asks nothing, posts nothing and ends with 0; it takes
            no --suggested and no answer. <question> <answer>, as
            tester.q1 2 (q1 for the planner), answers a question
            without asking, and the pairs repeat; with --suggested the
            suggestion is taken for every question they do not answer.
            One comment or commit per role is written, or none when a pair is wrong
            (a question not open, a number not a choice: 1); a question
            named twice ends with 2. --change <n> answers on a change
            with no issue, by
            change_comment_bodies and change_comment. It never changes the
            issue's file: where an answer does, it says so. Ends with 0 when
            the answers are posted or written or no question is open, 1 when
            it was refused (also where the ref is not as wb writes it, or no
            key holds the role answer), the input ended, a pattern failed or a
            copy could not be posted, 2 when it was used wrongly or the
            repository is not set up. -q (--quiet) leaves out
            the line that says the answers are posted and the line that says
            no question is open, and not what it will post, a question or a
            refusal
";

/// Runs a command so that a fault in `wb` itself is not shown as a panic: the hook prints nothing,
/// and a panic that is caught ends the command with 1 (it did not hold) and one line on `err`.
/// `main` runs every command through it. The body is a parameter so that a test can make it panic.
pub fn guarded(body: impl FnOnce() -> i32, err: &mut dyn Write) -> i32 {
    std::panic::set_hook(Box::new(|_| {}));
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(code) => code,
        // In a test build a 1 neither said nor marked (#180) ends the program with no status, so no test passes it.
        Err(payload) if cfg!(debug_assertions) && payload.downcast_ref::<&str>() == Some(&refusal::FAULT) => std::process::abort(),
        Err(_) => {
            let _ = writeln!(err, "wb: an internal error. This is a fault in wb, not in what it was given.");
            // not a refusal: a fault in wb itself, after the command; `refusal::ended` is not reached.
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

/// What `quiet_of` made of a word.
enum Quiet {
    /// The word is not `-q` or `--quiet`.
    Not,
    /// The word is `-q` or `--quiet`, the first of it: the command is quiet.
    Taken,
    /// The word is `-q` or `--quiet` again: what to say of it, which names the form that was typed.
    Twice(String),
}

/// `-q` and `--quiet` (#285), read once for every command: the one place that says which words they are. The first
/// makes the command quiet (`refusal::quiet`): it leaves out the lines that only confirm what was done or say there
/// was nothing to do, through `refusal::inform`, and never a refusal, a warning, what it says before it writes, a
/// command a person has to run, or the output a script reads. `given` is the form that was given first, if any.
fn quiet_of(found: &Word, given: &mut Option<&'static str>) -> Quiet {
    let form = match found {
        Word::Short('q') => "-q",
        Word::Long(option) if option == "quiet" => "--quiet",
        _ => return Quiet::Not,
    };
    match *given {
        Some(first) if first == form => Quiet::Twice(format!("'{}' is given twice", form)),
        Some(_) => Quiet::Twice("'-q' and '--quiet' are both given".to_string()),
        None => {
            *given = Some(form);
            refusal::set_quiet();
            Quiet::Taken
        }
    }
}

/// Runs `wb` with the words after its name and returns the exit status:
/// 0 it holds or it is done, 1 it does not hold or it was refused, 2 it was used wrongly.
/// Every command's status passes `refusal::ended` (#180).
pub fn run(args: &[OsString], input: &mut dyn BufRead, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    refusal::begin();
    let code = run_command(args, input, out, err);
    refusal::ended(code, err)
}

fn run_command(args: &[OsString], input: &mut dyn BufRead, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
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
        "doctor" => {
            let home = home_folder();
            let path = std::env::var_os("PATH");
            let host = doctor::Host {
                organisation: Path::new(confidentiality::ORGANISATION_FILE),
                sandbox: Path::new(view::BWRAP),
                home: home.as_deref(),
                path: path.as_deref(),
                net: Path::new(doctor::NET),
            };
            run_doctor(&mut parser, None, &host, out, err)
        }
        "upgrade" => run_upgrade(&mut parser, out, err),
        "prompt" => run_prompt(&mut parser, None, Path::new(confidentiality::ORGANISATION_FILE), out, err),
        "plan" => run_plan(&mut parser, None, Path::new(confidentiality::ORGANISATION_FILE), input, out, err),
        "accept" => run_accept(&mut parser, out, err),
        "status" => run_status(&mut parser, out, err),
        "view" => run_view(&mut parser, None, Path::new(confidentiality::ORGANISATION_FILE), out, err),
        "revert" => run_revert(&mut parser, err),
        "clean" => run_clean(&mut parser, out, err),
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
        "compare" => run_compare(&mut parser, Path::new(manifest::PACKAGE_FOLDER), out, err),
        // A command of the layout that left `layout::PLANNED` and has no arm yet is refused, not run as another.
        other => {
            let _ = writeln!(err, "wb: the command '{}' has no arm in the dispatch", other);
            2
        }
    }
}

/// The commands of the layout that are built, each with an arm in `run`. A test holds that it is the
/// layout without `layout::PLANNED`, so a piece that builds a command adds it here with its arm.
pub const ARMS: [&str; 15] =
    ["init", "check", "doctor", "upgrade", "plan", "accept", "start", "status", "view", "answer", "revert", "clean", "prompt", "revision", "compare"];

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
    let mut quiet: Option<&'static str> = None;
    loop {
        let refused = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => match quiet_of(&found, &mut quiet) {
                Quiet::Taken => continue,
                Quiet::Twice(why) => return used(err, "wb init", &why),
                Quiet::Not => match found {
                    Word::Short('y') => {
                        yes = true;
                        continue;
                    }
                    Word::Long(option) if option == "yes" => {
                        yes = true;
                        continue;
                    }
                    found => named(found, parser),
                },
            },
            Err(refused) => refused,
        };
        let _ = write!(err, "wb init: '{}' is not known\n\n{}", refused, USAGE);
        return 2;
    }

    interrupt::install();
    match init::repository_root(None) {
        Err(why) => {
            crate::refusal::limit(err, "wb init", format_args!("{}. Nothing written.", why))
        }
        Ok(None) => {
            crate::refusal::limit(err, "wb init", format_args!("this folder is not in a git repository. Watchbill is set up in one."))
        }
        Ok(Some(root)) => init::init(&root, VERSION, Path::new(manifest::PACKAGE_FOLDER), yes, input, out, err, None),
    }
}

/// `wb check`: the repository against its pinned revision and the installed `wb`. It writes nothing.
fn run_check(parser: &mut Parser, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut quiet: Option<&'static str> = None;
    loop {
        let refused = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => match quiet_of(&found, &mut quiet) {
                Quiet::Taken => continue,
                Quiet::Twice(why) => return used(err, "wb check", &why),
                Quiet::Not => named(found, parser),
            },
            Err(refused) => refused,
        };
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

/// `wb doctor`, with the words after `doctor`, in the repository of `folder`, on the machine `host` says: `run` gives the
/// folder it runs in and what is fixed in `wb`; this is so that a test can give its own, as `view_command` does.
pub fn doctor(args: &[OsString], folder: &Path, host: &doctor::Host, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    refusal::begin();
    let code = run_doctor(&mut Parser::from_args(args.iter().cloned()), Some(folder), host, out, err);
    refusal::ended(code, err)
}

/// `wb doctor`: what this machine needs, one line to a check. It reads only (#178).
fn run_doctor(parser: &mut Parser, folder: Option<&Path>, host: &doctor::Host, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut quiet: Option<&'static str> = None;
    loop {
        let refused = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => match quiet_of(&found, &mut quiet) {
                Quiet::Taken => continue,
                Quiet::Twice(why) => return used(err, "wb doctor", &why),
                Quiet::Not => named(found, parser),
            },
            Err(refused) => refused,
        };
        return used(err, "wb doctor", &format!("'{}' is not known", refused));
    }
    match root_for("wb doctor", folder, err) {
        Err(code) => code,
        Ok(root) => doctor::doctor(&root, host, out, err),
    }
}

/// `wb upgrade [--dry-run]`: pins the repository to the installed revision, saying each write first (#174).
fn run_upgrade(parser: &mut Parser, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut dry_run = false;
    let mut quiet: Option<&'static str> = None;
    loop {
        let refused = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => match quiet_of(&found, &mut quiet) {
                Quiet::Taken => continue,
                Quiet::Twice(why) => return used(err, "wb upgrade", &why),
                Quiet::Not => match found {
                    Word::Short('n') => {
                        if dry_run {
                            return used(err, "wb upgrade", "'--dry-run' is given twice");
                        }
                        dry_run = true;
                        continue;
                    }
                    Word::Long(option) if option == "dry-run" => {
                        if dry_run {
                            return used(err, "wb upgrade", "'--dry-run' is given twice");
                        }
                        dry_run = true;
                        continue;
                    }
                    found => named(found, parser),
                },
            },
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
    refusal::begin();
    let code = run_prompt(&mut Parser::from_args(args.iter().cloned()), Some(folder), organisation, out, err);
    refusal::ended(code, err)
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
    refusal::begin();
    let code = start_words(args, folder, machine, out, err);
    refusal::ended(code, err)
}

fn start_words(args: &[OsString], folder: &Path, machine: &Machine, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
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
    // `-q` is taken before the role too, as git takes an option before its arguments.
    let mut quiet: Option<&'static str> = None;
    let first = loop {
        match next(parser) {
            Ok(Some(found)) => match quiet_of(&found, &mut quiet) {
                Quiet::Taken => continue,
                Quiet::Twice(why) => return used(err, "wb start", &why),
                Quiet::Not => break Ok(Some(found)),
            },
            other => break other,
        }
    };
    match first {
        Ok(Some(Word::Long(option))) if option == "list" => run_list((true, false), quiet, parser, folder, machine, out, err),
        Ok(Some(Word::Long(option))) if option == "verbose" => run_list((false, true), quiet, parser, folder, machine, out, err),
        Ok(Some(Word::Short('v'))) => run_list((false, true), quiet, parser, folder, machine, out, err),
        Ok(Some(Word::Value(value))) if value.to_str().is_some_and(|role| STARTED.contains(&role)) => {
            let role = word(value.as_bytes());
            run_start(&role, quiet, parser, folder, machine, out, err)
        }
        Ok(None) => used(err, "wb start", &format!("a role is needed: {}", roles)),
        Ok(Some(found)) => {
            let given = named(found, parser);
            used(err, "wb start", &format!("'{}' is not a role of the revision: {}", given.escape_debug(), roles))
        }
        Err(given) => used(err, "wb start", &format!("'{}' is not a role of the revision: {}", given.escape_debug(), roles)),
    }
}

/// `wb start --list [-v | --verbose]` (#291): the roles of the pinned revision that `wb start` starts a session of,
/// in the order of `STARTED`, one to a line and nothing else. With `-v` each line holds two words more: whether the
/// role runs in a view of its lists (`view`) or of the whole tree (`tree`, #202), and whether the organisation's file
/// clears a runner for it (`cleared`, else `not-cleared`): for the planner one runner with both `start` and `planner`
/// up to the level of the repository, as `wb start planner` needs, for the others a runner with `start`. It reads the manifest and the organisation's file, writes
/// nothing and asks nothing of the network. `given` is what the first word was: (`--list`, `--verbose`).
fn run_list(given: (bool, bool), mut quiet: Option<&'static str>, parser: &mut Parser, folder: Option<&Path>, machine: &Machine, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let name = "wb start";
    let (mut list, mut verbose) = given;
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(given) => return used(err, name, &format!("'{}' is not taken with --list", given.escape_debug())),
        };
        // `-q` is taken with `--list` and changes nothing: the roles are what a script reads.
        match quiet_of(&found, &mut quiet) {
            Quiet::Taken => continue,
            Quiet::Twice(why) => return used(err, name, &why),
            Quiet::Not => {}
        }
        match found {
            Word::Long(option) if option == "list" || option == "verbose" => {
                let flag = if option == "list" { &mut list } else { &mut verbose };
                if *flag {
                    return used(err, name, &format!("'--{}' is given twice", option));
                }
                *flag = true;
            }
            Word::Short('v') => {
                if verbose {
                    return used(err, name, "'-v' is given twice");
                }
                verbose = true;
            }
            found => {
                let given = named(found, parser);
                return used(err, name, &format!("'{}' is not taken with --list", given.escape_debug()));
            }
        }
    }
    if !list {
        return used(err, name, "'-v' and '--verbose' are taken only with --list");
    }
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
    let have = match prompt::role_names(Path::new(&root)) {
        Ok(have) => have,
        Err(why) => {
            let _ = writeln!(err, "{}: {}. Watchbill is not set up here: run wb init.", name, why);
            return 2;
        }
    };
    let roles: Vec<&str> = STARTED.iter().copied().filter(|role| have.iter().any(|one| one == role)).collect();
    // The organisation's file is read for `-v` alone. One that is not there, or does not hold, clears no runner.
    let organisation = if verbose { confidentiality::read(machine.organisation).ok() } else { None };
    let level = organisation.as_ref().and_then(|found| confidentiality::repository_level(Path::new(&root), found).ok());
    let width = roles.iter().map(|role| role.len()).max().unwrap_or(0);
    let mut lines = String::new();
    for role in roles {
        if !verbose {
            lines.push_str(&format!("{}\n", role));
            continue;
        }
        let view = if issue::ROLES.contains(&role) { "view" } else { "tree" };
        let cleared = organisation.as_ref().is_some_and(|found| {
            found.runners.iter().any(|runner| match (role, &runner.planner, &level) {
                // A session of the planner needs one runner that has `start` and is cleared for it up to the level.
                ("planner", Some(planner), Some(level)) => {
                    runner.start.is_some() && matches!((found.rank(level), found.rank(&planner.up_to)), (Some(held), Some(up_to)) if held <= up_to)
                }
                ("planner", _, _) => false,
                _ => runner.start.is_some(),
            })
        });
        lines.push_str(&format!("{:width$} {} {}\n", role, view, if cleared { "cleared" } else { "not-cleared" }, width = width));
    }
    let _ = out.write_all(lines.as_bytes());
    0
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
const STARTED: [&str; 5] = ["planner", "tester", "build", "review", "fix"];

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
    /// `--show-origin`: print the prompt with a line before each part that says where it comes from (#291).
    show_origin: bool,
    /// `--prompt` of `wb start <role>`: print the prompt, as `wb prompt` does.
    print: bool,
    runner: Option<String>,
    /// `--ignore-local` of `wb start <role>`: a local change under `reads` does not stop the start (#172).
    ignore_local: bool,
    /// `--ignore-kept` of `wb start <role>`: a view kept with work not taken back does not stop the start (#222).
    ignore_kept: bool,
    /// `--take-back` of `wb start <role> <issue>`: take back the kept view of that role and issue (#223).
    take_back: bool,
    /// `--ignore-branch` of `wb start <role>`: a checkout on a branch that is neither the stamp's nor the default
    /// branch does not stop the start (#296).
    ignore_branch: bool,
    /// `--no-push` of `wb start <role>`: the branch is not pushed and no change is opened after the session; the
    /// commands are said, as before #151.
    no_push: bool,
}

impl Words {
    fn get(&self, name: &str) -> Option<&str> {
        value_of(&self.values, name)
    }

    /// Only what `wb prompt` does: print, and post and start nothing.
    fn prints(&self) -> bool {
        self.print || self.stamp || self.parts || self.show_origin
    }

    /// A review or a fix of a change with no issue (#116).
    fn alone(&self) -> bool {
        self.get("issue").is_none() && prompt::ALONE.contains(&self.role.as_str()) && self.get("change").is_some()
    }
}

/// Reads the words of `wb prompt`, where the first word is the role, or of `wb start <role>`, where `role` is given
/// and `--prompt` is taken too. What `wb prompt` refuses as used wrongly is refused here, in its order, with
/// its words after `name`. The status of a refusal is the error.
fn read_words(parser: &mut Parser, mut quiet: Option<&'static str>, name: &str, role: Option<&str>, err: &mut dyn Write) -> Result<Words, i32> {
    let starter = role.is_some();
    let mut role: Option<String> = role.map(str::to_string);
    let mut issue_word: Option<String> = None;
    let mut values: Vec<(&'static str, String)> = Vec::new();
    let (mut parts, mut stamp, mut print, mut ignore_local, mut ignore_kept, mut take_back) = (false, false, false, false, false, false);
    let (mut ignore_branch, mut no_push, mut show_origin) = (false, false, false);
    let mut runner: Option<String> = None;
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(refused) => return Err(used(err, name, &format!("'{}' is not known", refused))),
        };
        match quiet_of(&found, &mut quiet) {
            Quiet::Taken => continue,
            Quiet::Twice(why) => return Err(used(err, name, &why)),
            Quiet::Not => {}
        }
        match found {
            Word::Long(option) if option == "parts" || option == "stamp" || option == "show-origin" || (starter && (option == "prompt" || option == "ignore-local" || option == "ignore-kept" || option == "ignore-branch" || option == "take-back" || option == "no-push")) => {
                let flag = match option.as_str() {
                    "parts" => &mut parts,
                    "stamp" => &mut stamp,
                    "show-origin" => &mut show_origin,
                    "ignore-local" => &mut ignore_local,
                    "ignore-kept" => &mut ignore_kept,
                    "ignore-branch" => &mut ignore_branch,
                    "no-push" => &mut no_push,
                    "take-back" => &mut take_back,
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
    if show_origin && (stamp || parts) {
        let other = if stamp { "--stamp" } else { "--parts" };
        return Err(used(err, name, &format!("'--show-origin' and '{}' are not used together", other)));
    }
    // A session that `wb start <role>` starts takes the organisation's default runner, when none is given (#84).
    let prints = !starter || print || stamp || parts || show_origin;
    if role == "planner" && runner.is_none() && prints {
        return Err(used(
            err,
            name,
            "the planner is handed out only to a runner that the organisation's file clears for it: give '--runner <name>'",
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
    Ok(Words { role, values, parts, stamp, show_origin, print, runner, ignore_local, ignore_kept, take_back, ignore_branch, no_push })
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
    /// The hash of each source of the job (#236), none where there is no plan; and the commit of `HEAD` it is handed out
    /// from, which the view is made from.
    job: Vec<String>,
    base: String,
}

/// Everything `wb prompt` checks and does before it prints, for the words given: the check of `wb check`,
/// the role, the planner's runner (#93), the issue's file (#76, #97), the values derived (#72) and those
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
    let refused = |err: &mut dyn Write, why: &dyn std::fmt::Display| refusal::limit(err, name, why);
    // `wb prompt` and a start read the patterns of `watchbill.yml`, and only while it is as `HEAD` has it (#186).
    if let Err(why) = starter::held(&root) {
        return Err(refused(err, &format!("{}.", why)));
    }
    // The project's text goes into the prompt only as `HEAD` has it (#211): every file that is not is named, before
    // any pattern runs and before the check.
    let refusals = prompt::project_refusals(Path::new(&root), role);
    if !refusals.is_empty() {
        let mut code = 0;
        for why in &refusals {
            code = refusal::limit(err, name, why);
        }
        return Err(code);
    }
    // What `wb check` says is for standard error here: standard output holds the prompt or nothing.
    let (mut said, mut why) = (Vec::new(), Vec::new());
    let status = check::check(&root, VERSION, Path::new(manifest::PACKAGE_FOLDER), &mut said, &mut why);
    if status != 0 {
        refusal::limit(err, name, "nothing printed, the check of `wb check` does not hold.");
        let _ = err.write_all(&said);
        let _ = err.write_all(&why);
        return Err(status);
    }
    // A role the revision does not have is refused before anything is derived for it.
    if let Err(why) = prompt::require_role(Path::new(&root), role) {
        return Err(refused(err, &why));
    }
    // The planner goes only to a runner the organisation cleared for it, up to the repository's level (#93).
    let mut cleared = None;
    if role == "planner" {
        match confidentiality::clear_planner(organisation, Path::new(&root), words.runner.as_deref().unwrap_or_default()) {
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
    // A planner round after the acceptance voids it (#290), as `wb status` reads it: from the stamps on the job's ref
    // (#236), with or without a plan.
    if let (Some(file), Some(issue)) = (&file, words.get("issue")) {
        if let Err(why) = issue::planner_gate(Path::new(&root), issue, file) {
            return Err(refused(err, &why));
        }
    }
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
            let why = format!(
                "the {} was derived from {} and holds a control character or a line break, or white space at its start or end, and a stamp holds it as one line. Give --{}.",
                name_of, source, name_of
            );
            return Err(refusal::limit(err, name, why));
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
    // The level goes in the stamp of every session, as every session runs in a view (#209, #202), from the same place
    // `--parts` has it.
    let level = {
        let (level, why) = stamp_level(&root, organisation);
        if let Some(why) = why {
            let _ = writeln!(err, "{}: the level of the stamp is {}: {}", name, level, why);
        }
        Some(level)
    };
    // The job and the base the stamp names (#236): read with no replace ref, as what counts is read.
    let job = value_of(&values, "issue").map(|issue| plan::source_hashes(&root, issue)).unwrap_or_default();
    let base = signers::git(&root, &["rev-parse", "--verify", "-q", "HEAD^{commit}"], None)
        .map(|printed| String::from_utf8_lossy(&printed).trim().to_string())
        .unwrap_or_default();
    Ok(Handed { root, values, found, cleared, file, requirements_given, stamps: worked.stamps, level, job, base })
}

/// The model and effort a role is handed out with: for the planner those its runner is cleared with, for
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
        job: &handed.job,
        base: &handed.base,
    };
    let given: Vec<(&str, &str)> = handed.values.iter().map(|(name, value)| (*name, value.as_str())).collect();
    Ok(prompt::stamp(&said, &given, &handed.found))
}

/// The local changes the view of a session of `role` would miss (#172): over the whole checkout for the planner
/// (#202), its issue's file as its `changes`, and for a role with no issue's file, which `--ignore-local` passes.
fn local_of(role: &str, handed: &Handed, root: &str) -> Option<Result<Vec<view::Local>, String>> {
    if role == "planner" {
        let issue = value_of(&handed.values, "issue")?;
        return Some(view::local_changes_whole(root, Some(&issue_file_of(issue))));
    }
    match handed.file.as_ref() {
        Some(file) if issue::ROLES.contains(&role) => Some(view::local_changes(root, &file.issue, role)),
        _ => Some(view::local_changes_whole(root, None)),
    }
}

/// Why the runner's `program`, where it is named by its path, cannot be found inside the sandbox, if it cannot: it
/// lies, as written or as resolved, under none of the system's folders the sandbox holds (`view::SYSTEM`) and none of
/// the runner's `binds`.
fn outside_the_sandbox(program: &str, binds: &[(std::path::PathBuf, bool)]) -> Option<String> {
    if !program.starts_with('/') {
        return None;
    }
    let held: Vec<String> = view::SYSTEM.iter().map(|folder| folder.to_string()).chain(binds.iter().map(|(path, _)| path.display().to_string())).collect();
    let inside = |path: &str| {
        let parts = starter::parts(path);
        held.iter().any(|folder| {
            let folder = starter::parts(folder);
            !folder.is_empty() && parts.len() >= folder.len() && parts[..folder.len()] == folder[..]
        })
    };
    let real = std::fs::canonicalize(program).map(|path| path.display().to_string()).unwrap_or_else(|_| program.to_string());
    (!inside(program) || !inside(&real)).then(|| {
        format!(
            "the runner's program {} lies outside what the sandbox holds ({} and the runner's `binds`), so no session can be started with it",
            program.escape_debug(),
            view::SYSTEM.join(", ")
        )
    })
}

/// The issue's file of `issue`, from the root, as the planner writes it.
fn issue_file_of(issue: &str) -> String {
    format!("{}/{}.yml", issue::FOLDER, issue)
}

/// What may come back from the view of a session of `role` (#202): for the planner its issue's file alone, for a
/// role with an issue's file its `changes`, and for any other session every path but `.watchbill/` and
/// `build_files`.
fn allowed_of(role: &str, file: Option<&issue::Used>, issue: Option<&str>) -> view::Allowed {
    if role == "planner" {
        return view::Allowed::IssueFile(issue_file_of(issue.unwrap_or_default()));
    }
    match file {
        Some(file) if issue::ROLES.contains(&role) => view::Allowed::Changes(file.issue.changes.clone()),
        _ => view::Allowed::Whole,
    }
}

/// The paths of the view a session of `role` runs in (#126): those of the issue's file for the role, when the
/// role is one the file lists paths for and a file is used. `None`: the session runs in a view of every file of
/// `HEAD` (#202), as the planner, which reads the whole code by its role, and a role started with no issue's file
/// (#116) do.
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
            Err(why) => return refusal::limit(err, name, why),
        }
    } else if words.parts {
        let mut lines: String = handed.found.parts.iter().map(|part| format!("{}  {}\n", part.hash, part.path)).collect();
        if let (Some(cleared), Some(runner)) = (&handed.cleared, &words.runner) {
            lines.push_str(&format!("level: {}\nrunner: {}\n", cleared.level, runner));
        }
        let _ = out.write_all(lines.as_bytes());
    } else if words.show_origin {
        let _ = out.write_all(show_origin(words, handed).as_bytes());
    } else {
        let _ = out.write_all(handed.found.text.as_bytes());
    }
    0
}

/// Where the value of a placeholder in a prompt came from (#291).
fn placeholder_source(name: &str, words: &Words, handed: &Handed) -> String {
    if words.get(name).is_some() {
        return "given on the command line".to_string();
    }
    if prompt::NAMED_BY_WB.contains(&name) {
        return "named by wb".to_string();
    }
    match name {
        "checkout" => "the root of the repository".to_string(),
        "requirements" => match &handed.file {
            Some(file) => format!("the issue's file {}", file.path),
            None => "the default text, as no issue's file is used".to_string(),
        },
        "issue" | "issue_address" if words.alone() => "none: the change has no issue".to_string(),
        "issue_address" | "change_address" => format!("the pattern {} of watchbill.yml (HEAD)", name),
        "branch" if words.alone() => "the branch checked out".to_string(),
        "branch" if words.role == "planner" => "the pattern branch of watchbill.yml (HEAD)".to_string(),
        "branch" => "the pattern branch of watchbill.yml (HEAD) and the branches of the repository".to_string(),
        "round" => "worked out from the comments on the issue or the change".to_string(),
        "change" => "worked out from the branch of the issue".to_string(),
        "classed" => "classed: of watchbill.yml (HEAD)".to_string(),
        "classed_touched" => "the diff of the change's branch, and classed: of watchbill.yml (HEAD)".to_string(),
        _ => "given on the command line".to_string(),
    }
}

/// The prompt with, before each part, one line that says its kind, its path, where it was read from and its hash (the
/// hash of its `part:` line), and below it one line for each placeholder the part holds, `{name}` and where its value
/// came from (#291). The lines of the parts, put one after the other, are the prompt.
fn show_origin(words: &Words, handed: &Handed) -> String {
    let mut shown = String::new();
    for segment in &handed.found.segments {
        match &segment.part {
            Some(part) => {
                let source = match segment.kind {
                    "project" => "  (HEAD)",
                    "issue" => "  (accepted)",
                    _ => "",
                };
                shown.push_str(&format!("# {}  {}{}  {}\n", segment.kind, part.path, source, part.hash));
            }
            None => shown.push_str("# wb  the line between the revision's text and the project's\n"),
        }
        for name in &segment.used {
            shown.push_str(&format!("# {{{}}}  {}\n", name, placeholder_source(name, words, handed)));
        }
        shown.push_str(&segment.text);
    }
    shown
}

/// `wb prompt <role> [--<placeholder> X]... [--runner <name>] [--parts]`: the prompt of a role, filled from
/// one option for each placeholder of the list (`prompt::PLACEHOLDERS`), or with `--parts` the path and hash
/// of each of its parts. Nothing is printed unless the check of `wb check` holds and the prompt can be put
/// together. The planner is handed out only to a runner the organisation's file clears for it (#93). It
/// writes nothing.
fn run_prompt(parser: &mut Parser, folder: Option<&Path>, organisation: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let name = "wb prompt";
    let words = match read_words(parser, None, name, None, err) {
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
fn run_start(role: &str, quiet: Option<&'static str>, parser: &mut Parser, folder: Option<&Path>, machine: &Machine, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let (organisation, terminal, state) = (machine.organisation, machine.terminal, machine.state);
    let name = format!("wb start {}", role);
    let name = name.as_str();
    let mut words = match read_words(parser, quiet, name, Some(role), err) {
        Ok(words) => words,
        Err(status) => return status,
    };
    if words.take_back {
        return run_take_back(name, role, &words, folder, state, err);
    }
    if words.prints() {
        return match hand_out(name, &words, folder, organisation, false, err) {
            Ok(handed) => print(name, &words, &handed, out, err),
            Err(status) => status,
        };
    }
    // The file of the change's description is named by `wb` (#151): a session it starts writes it where `wb` reads it.
    if words.get("description").is_some() {
        return used(err, name, "'--description' is taken only with --prompt, --stamp or --parts: the file of the change's description is named by wb");
    }
    // So are the folder of what `wb` brings in, and the files it posts from (#153).
    if let Some(given) = prompt::NAMED_BY_WB.iter().find(|placeholder| words.get(placeholder).is_some()) {
        let why = format!("'--{}' is taken only with --prompt, --stamp or --parts: the files a session reads and writes are named by wb", given);
        return used(err, name, &why);
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
            Err(why) => return refusal::limit(err, name, why),
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
    // A refusal at a check of the start, said and kept in the lines.
    let said = |lines: &mut sessions::Lines, err: &mut dyn Write, check: &'static str, message: String| {
        let code = refusal::say(err, name, &message);
        lines.close(stamp.as_deref().ok(), sessions::Outcome::Refused { check, status: code, message }, err);
        code
    };
    let refused = |lines: &mut sessions::Lines, err: &mut dyn Write, check: &'static str, why: &str| said(lines, err, check, refusal::limit_text(why));
    // 1a. The stamp of an issue is a commit on its job's ref (#236): git must sign it with a key that `.watchbill/signers`
    // of the default branch holds with `start`, and with no list the start is refused (#323). Before anything is written.
    // A review or a fix of a change with no issue has no job: its stamp is posted on the change, as before.
    let job = words.get("issue").map(str::to_string);
    let signing = match &job {
        None => None,
        Some(_) => match stamps::ready(root) {
            Ok(signing) => Some(signing),
            Err(why) => return refused(&mut lines, err, "sign", &format!("{}. Nothing is written, and no session is started", why)),
        },
    };
    // A job's ref that is not as `wb` writes it (#327): nothing on it counts, and no stamp is written after it until a
    // person repairs it.
    if let Some(issue) = &job {
        match stamps::not_as_written(root, issue) {
            Ok(None) => {}
            Ok(Some(why)) | Err(why) => return refused(&mut lines, err, "sign", &format!("{}. Nothing is written, and no session is started", why)),
        }
    }
    // 1b. A view kept with work not taken back (#222): what a session left there is not on the branch, and the next
    // session would not see it. Before anything is posted. Only `--ignore-kept` passes it.
    if !kept_views.is_empty() && kept_ignored.is_none() {
        let issue = words.get("issue").unwrap_or_default();
        return said(&mut lines, err, "kept", refusal::guard_text("--ignore-kept", kept::refusal(issue, &kept_views)));
    }
    // 1c. The branch checked out (#296, #298): the commits come back onto the stamp's branch, whichever is checked
    // out. Where that branch is there, the view is its tree or nothing: a checkout whose `HEAD` is not its commit is a
    // limit. A checkout on another branch is said, for the person may mean that branch; the default branch passes
    // only where the stamp's branch is not there yet, as for a planner. Only `--ignore-branch` passes the guard.
    if let Some(stamped) = value_of(&handed.values, "branch") {
        match view::on_branch(root, stamped) {
            view::OnBranch::Fine => {}
            view::OnBranch::Limit(why) => return refused(&mut lines, err, "branch", &why),
            view::OnBranch::Guard(_) if words.ignore_branch => {}
            view::OnBranch::Guard(why) => return said(&mut lines, err, "branch", refusal::guard_text("--ignore-branch", why)),
        }
    }
    // 2. The rule table: a fix follows a review. The number of rounds is no rule for a person (#84).
    if role == "fix" {
        let stamps = match (&handed.stamps, &job) {
            (Some(stamps), _) => stamps.clone(),
            // From the job's ref (#236), where the round was given.
            (None, Some(issue)) => match stamps::read(root, issue) {
                Ok(ledger) => ledger.as_stamps(),
                Err(why) => return refused(&mut lines, err, "rule", &why),
            },
            (None, None) => {
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
    // The limits of the session (#267): the rule table's, or lower ones of `watchbill.yml`, never higher.
    let limits = match watchdog::limits_for(root, role) {
        Ok(limits) => limits,
        Err(why) => return refused(&mut lines, err, "rule", &why),
    };
    // 3. The class: no classed code is worked on here before #65. The planner changes only the issue's file.
    if role != "planner" {
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
    // In the sandbox a program that is not there is said only on the terminal, by `env`: one named by its path is
    // looked for first, before anything is posted (#202).
    if let Some(program) = filled.first().filter(|program| program.starts_with('/')) {
        if !Path::new(program).is_file() {
            let why = format!(
                "the runner's program {} is not there, so no session can be started with it. Install it, or change `start` of the runner '{}' in the organisation's file {}",
                program.escape_debug(),
                runner,
                organisation.display()
            );
            return refused(&mut lines, err, "start", &why);
        }
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
    // 7. The view (#126): every session runs in one, in bubblewrap (#202): of the paths of its issue's file, or of
    // every file of `HEAD`. A local change to a file of it stops the start (#172) before its binds, the sandbox and
    // the view itself are made sure of; none of it is posted.
    match &local {
        Some(Err(why)) => return refused(&mut lines, err, "local", &format!("the local changes to the view could not be looked at: {}", why)),
        Some(Ok(found)) if !found.is_empty() && ignored.is_none() => {
            // Only a change under `reads` is passed, by `--ignore-local`: a change under `changes` is a limit.
            let message = view::local_message(found, words.ignore_local);
            let message = if found.iter().any(|local| local.changes) { refusal::limit_text(message) } else { refusal::guard_text("--ignore-local", message) };
            return said(&mut lines, err, "local", message);
        }
        _ => {}
    }
    let paths = view_paths(role, &handed);
    let layout = match (role, &paths) {
        ("planner", _) => view::Layout::Planner,
        (_, Some(_)) => view::Layout::Listed,
        (_, None) => view::Layout::Whole,
    };
    // The commits come back onto the stamp's branch (#296), or onto the branch checked out where it names none.
    let onto = value_of(&handed.values, "branch");
    let prepared = match prepare_view(root, role, words.get("issue"), paths.as_deref(), onto, &chosen.binds, machine) {
        Ok(found) => found,
        Err(why) => return refused(&mut lines, err, "view", &why),
    };
    // A view made and not run in is removed, whatever ends the start, and the folder of what the session writes for
    // `wb` with it (#151).
    let discard = |prepared: &Prepared| {
        view::discard(&prepared.view);
        push::remove_out(&prepared.out);
    };
    // A program named by its path lies under what the sandbox holds, as it is there: the system's folders and the
    // runner's binds, once they are made sure of (#202).
    if let Some(why) = filled.first().and_then(|program| outside_the_sandbox(program, &prepared.binds)) {
        discard(&prepared);
        let why = format!("{}. Bind it in `binds` of the runner '{}' in the organisation's file {}", why, runner, organisation.display());
        return refused(&mut lines, err, "start", &why);
    }
    // The planner has no login (#295): its issue, and the questions and answers under it, are read here, as the
    // person and outside the sandbox, and brought into its view before anything is posted. The stamp holds the hash.
    let brought = match (role, value_of(&handed.values, "issue")) {
        ("planner", Some(issue)) => match brought::bring_in(root, issue, &prepared.view.folder) {
            Ok(found) => {
                // A job with no stamp on its ref yet: what the tracker holds is said, before the stamp is written (#333).
                if let Some(notice) = &found.notice {
                    let _ = writeln!(err, "{}: {}", name, notice);
                }
                Some(found)
            }
            Err(why) => {
                discard(&prepared);
                return refused(&mut lines, err, "issue", &why);
            }
        },
        _ => None,
    };
    // Every other role has no login either (#153): what it reads is read here, as the person and outside the sandbox,
    // and brought into the sandbox outside its view, before anything is posted.
    let session_of = || brought::Session {
        role,
        issue: value_of(&handed.values, "issue"),
        change: value_of(&handed.values, "change"),
        branch: value_of(&handed.values, "branch"),
        paths: paths.as_deref(),
    };
    if brought::BROUGHT_FOR.contains(&role) {
        match brought::bring_in_for(root, &session_of(), &prepared.out) {
            Ok(Some(notice)) => {
                let _ = writeln!(err, "{}: {}", name, notice);
            }
            Ok(None) => {}
            Err(why) => {
                discard(&prepared);
                return refused(&mut lines, err, "issue", &why);
            }
        }
    }
    let stamp: Result<String, String> = match (&stamp, &brought) {
        (Ok(text), Some(found)) => Ok(prompt::with_issue_text(text, &found.hash)),
        (Ok(text), None) => Ok(text.clone()),
        (Err(why), _) => Err(why.clone()),
    };
    // What the session reaches (#195): the stamp of a start says it, before its parts.
    let stamp: Result<String, String> = stamp.map(|text| prompt::with_reach(&text, &chosen.reach));
    // The pattern that posts the stamp. From here, a fault leaves the stamp not posted, and no view.
    let not_posted = |lines: &mut sessions::Lines, err: &mut dyn Write, why: &str| {
        discard(&prepared);
        let message = refusal::limit_text(why);
        let code = refusal::say(err, name, &message);
        lines.close(stamp.as_deref().ok(), sessions::Outcome::NotPosted { status: code, message }, err);
        code
    };
    // The stamp of an issue is a commit on its job's ref (#236), and the comment only a copy of it, posted where a
    // tracker is set up; the stamp of a change with no issue is the comment, as before.
    let line = match (prompt::pattern_of(root, post), &job) {
        (Ok(Some(line)), _) => Some(line),
        (Ok(None), Some(_)) => None,
        (Ok(None), None) => {
            let why = format!("watchbill.yml has no `{}`, so the stamp cannot be posted, and no session is started without it. Add the line (see the README), or use --prompt.", post);
            return not_posted(&mut lines, err, &why);
        }
        (Err(why), _) => return not_posted(&mut lines, err, &why.to_string()),
    };
    let stamp = match &stamp {
        Ok(stamp) => stamp,
        Err(why) => return not_posted(&mut lines, err, why),
    };
    // The stamp of an issue names it, whatever the prompt holds (the review of #322): the commit, its copy and the lines
    // of the start hold the same text.
    let named_stamp = match &job {
        Some(issue) => stamps::with_issue(stamp, issue),
        None => stamp.clone(),
    };
    let stamp = &named_stamp;
    lines.open(Some(stamp), err);
    // `outcome` says `running` first (#222), before `folder` is written (#176): a view is never without the mark of a
    // `wb` that is alive, so a `wb clean` that is told its name does not take the view of a session that is starting.
    lines.running(err);
    if let Some(paths) = &paths {
        lines.write("view", &view::list(paths), err);
    }
    lines.write("reach", &reach_list(&chosen.reach), err);
    // Where the view is, and what it was made from (#222): a `wb` ended before its session leaves no `back`, and the
    // view is found by this file.
    if let Some(text) = kept::folder_text(&prepared.view, words.get("issue")) {
        lines.write("folder", &text, err);
    }
    if let (Some(Ok(found)), Some(_)) = (&local, ignored) {
        let named: String = found.iter().map(|local| format!("{}\n", issue::shown(&local.path))).collect();
        lines.write("local", &named, err);
    }
    // The way out of the view (#195), for a runner with `reach`: the proxy listens before the stamp is posted, so that a
    // proxy that cannot be started leaves the stamp not posted.
    let net = if chosen.reach.is_empty() {
        None
    } else {
        let folder = prepared.out.join(reach::SOCKET_FOLDER);
        let started = reach::program().and_then(|exe| reach::Proxy::start(&folder, &chosen.reach, lines.file("refused").as_deref()).map(|proxy| (proxy, exe)));
        match started {
            Ok((proxy, exe)) => Some((proxy, reach::Net { folder, exe })),
            Err(why) => return not_posted(&mut lines, err, &why),
        }
    };
    // The stamp of an issue: a commit on the job's ref, pushed with no force before the session (#236). A push that
    // another start went before is refused, and nothing is started: the push is the lock.
    // Where origin took the stamp, what a later failure says: the stamp holds the round (the review of #322).
    let mut pushed_stamp: Option<String> = None;
    // The commit of the stamp, which the note on each commit taken back names (#237): none for a stamp that is a comment.
    let mut stamp_commit: Option<String> = None;
    if let (Some(issue), Some(signing)) = (&job, &signing) {
        let view_list = paths.as_ref().map(|paths| view::list(paths));
        let file = handed.file.as_ref().and_then(|_| signers::git(root, &["cat-file", "blob", &format!("HEAD:{}", issue_file_of(issue))], None).ok());
        let held = stamps::Held::Stamp { view: view_list.as_deref(), issue: file.as_deref() };
        match stamps::write(name, root, issue, stamp, &held, signing, err) {
            Ok((commit, local)) => {
                if !local {
                    pushed_stamp = Some(stamps::round_held(issue, &commit, stamp));
                }
                stamp_commit = Some(commit);
            }
            Err(why) => {
                discard(&prepared);
                let message = refusal::limit_text(why);
                let code = refusal::say(err, name, &message);
                lines.close(None, sessions::Outcome::NotPosted { status: code, message }, err);
                return code;
            }
        }
    }
    let posted = match &line {
        None => {
            let _ = writeln!(err, "{}: watchbill.yml has no `{}`, so no copy of the stamp is posted: the stamp is the commit on {}.", name, post, stamps::full(&on));
            Ok(Vec::new())
        }
        Some(line) => {
            let _ = write!(err, "{}: will post on {} {}, by `{}` of watchbill.yml:\n{}", name, word, on, post, stamp);
            starter::run(root, post, &line.replace(&format!("{{{}}}", word), &on), Some(stamp.as_bytes()))
        }
    };
    match posted {
        Ok(said) => {
            let _ = err.write_all(&said);
        }
        Err(why) => {
            discard(&prepared);
            let held = pushed_stamp.as_deref().map(|said| format!(" {}", said)).unwrap_or_default();
            let message = refusal::limit_text(format!("{}. The stamp may not be posted, and no session is started.{}", why, held));
            let code = refusal::say(err, name, &message);
            lines.close(None, sessions::Outcome::NotPosted { status: code, message }, err);
            return code;
        }
    }
    // A runner's hooks (#160) are looked at before the signals are held, so that a slow look can be ended (#317). The
    // folders of the organisation's file are covered in the sandbox, as `view::arguments_out` covers them.
    let named = {
        let path = std::env::var_os("PATH");
        let fixed = Path::new(confidentiality::ORGANISATION_FILE).parent().filter(|folder| folder.is_dir());
        let given = organisation.parent().filter(|folder| folder.is_dir());
        let covered: Vec<std::path::PathBuf> = [fixed, given].into_iter().flatten().map(Path::to_path_buf).collect();
        let sandbox = hooks::Sandbox { checkout: Path::new(root), view: &prepared.view.folder, home: machine.home, binds: &prepared.binds, covered: &covered, path: path.as_deref() };
        hooks::said(filled.first().map(String::as_str).unwrap_or_default(), root, &sandbox)
    };
    let began = std::time::Instant::now();
    // The signals stay held until the commits are taken back and `back` and `outcome` are written (#221).
    let _held = interrupt::hold();
    // A view of the whole tree (#202) is said: what it holds, and for a role with no issue's file what it is not.
    if let Some(size) = &prepared.size {
        refusal::inform(err, format_args!("{}: the view holds the whole tree of HEAD: {} files, {} bytes", name, size.files, size.bytes));
        if layout == view::Layout::Whole {
            let _ = writeln!(
                err,
                "{}: there is no issue's file, so the session runs in a view of every file of HEAD, with .watchbill/ read-only and no history: a change not committed is not in it, and the limits of its prompt on what it reads rest on the prompt alone",
                name
            );
        }
    }
    // A runner's hooks (#160): each that will fail in the view is named before the session, which starts all the same.
    for line in &named {
        let _ = writeln!(err, "{}: {}", name, line);
    }
    if !named.is_empty() {
        let _ = writeln!(
            err,
            "{}: the session starts all the same. A hook's program and what it reads go in `binds` of the runner '{}' in the organisation's file {}, read-only (see the README, A runner's hooks)",
            name,
            runner,
            organisation.display()
        );
    }
    // What the session reaches (#195), said before it starts.
    if chosen.reach.is_empty() {
        let _ = writeln!(err, "{}: the session reaches no network: the runner '{}' has no reach", name, runner);
    } else {
        let all: Vec<String> = chosen.reach.iter().map(ToString::to_string).collect();
        let _ = writeln!(err, "{}: the session reaches only {}, through wb's proxy (reach of the runner '{}')", name, all.join(", "), runner);
    }
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
    // The `wb` that waits is the watchdog (#267): what the view holds now is where its quiet starts from.
    let mut watch = watchdog::Watchdog::new(limits, &prepared.view.folder, paths.as_deref().unwrap_or(&[]), lines.folder());
    let (proxy, net) = match net {
        Some((proxy, net)) => (Some(proxy), Some(net)),
        None => (None, None),
    };
    let ran = view::run_reaching(machine.sandbox, &plan, layout, Some(&prepared.out), net.as_ref(), Some(&mut watch), name, err);
    // The proxy ends with the session (#195), before anything is taken back, and its refusals are said once.
    if let Some(count) = proxy.map(reach::Proxy::stop).filter(|count| *count > 0) {
        let named = match lines.file("refused").filter(|file| file.is_file()) {
            Some(file) => format!("each is named in {}", file.display()),
            None => "the lines of this start were not written, so they are not named".to_string(),
        };
        let _ = writeln!(
            err,
            "{}: the proxy refused {} {} that the session asked for, as reach of the runner '{}' does not let {} through; {}",
            name,
            count,
            if count == 1 { "connection" } else { "connections" },
            runner,
            if count == 1 { "it" } else { "them" },
            named
        );
    }
    let status = match ran {
        Ok(status) => status,
        Err(why) => {
            push::remove_out(&prepared.out);
            let message = refusal::limit_text(&why);
            let code = refusal::say(err, name, &message);
            let back = view::Back { failed: Some(why), ..Default::default() };
            let kept = view::tidy(name, &prepared.view, &back, err);
            lines.write("back", &back.text(kept.as_deref()), err);
            lines.close(None, sessions::Outcome::Refused { check: "start", status: code, message }, err);
            return code;
        }
    };
    let seconds = began.elapsed().as_secs();
    if !status.success() {
        refusal::answer();
    }
    // not a refusal: a session that did not end with 0 is an answer, marked above, and it is said below.
    let mut code = if status.success() { 0 } else { 1 };
    if code != 0 {
        let _ = writeln!(err, "{}: the session did not end with 0", name);
    }
    // A session ended at `max_minutes` (#267) ends `wb` with 1 however it ended: the limit was said when it was met. It
    // did not end well, so nothing that waits on a good end follows from it.
    let limit = watch.ended().then_some("max_minutes");
    if limit.is_some() {
        // not a refusal: the limit is said by `view::run_watched` as a limit, when it is met.
        code = 1;
    }
    let ended_well = status.success() && limit.is_none();
    // What the session committed in its view comes back onto the branch, as far as it keeps to what may come back.
    let allowed = allowed_of(role, handed.file.as_ref(), words.get("issue"));
    let list = lines.file("back");
    let back = view::take_back(name, root, &prepared.view, prepared.at.as_deref(), &allowed, &prepared.build_files, list.as_deref(), err);
    let kept = view::tidy(name, &prepared.view, &back, err);
    lines.write("back", &back.text(kept.as_deref()), err);
    if back.refused_any() {
        // not a refusal: what was refused at the take-back is said by `view::take_back` as a limit.
        code = 1;
    }
    // What the session wrote for `wb` (#151), read before its folder goes.
    let description = push::read_description(&prepared.out);
    let written = brought::read_written(&prepared.out);
    push::remove_out(&prepared.out);
    // How to look at what was taken (#159): the commits keep who wrote them, the tests, the checks.
    for (commit, _) in &back.taken {
        refusal::inform(err, format_args!("{}: to see what the commit changes, run:\n  git show --stat {}", name, commit));
    }
    // The push and the change (#151): by `wb`, as the person, outside the sandbox, or said where `wb` does not push.
    let ended = push::Ended {
        name,
        role,
        root,
        issue: value_of(&handed.values, "issue"),
        branch: prepared.view.branch_name(),
        back: &back,
        ended_well,
        no_push: words.no_push,
        description,
    };
    let pushed = push::after(&ended, err);
    if let Some(text) = &pushed.lines {
        lines.write("push", text, err);
    }
    if pushed.code != 0 {
        // not a refusal: what stopped the push or the change is said by `push::after` as a limit.
        code = 1;
    }
    // A note on each commit taken back, naming the stamp, and the notes pushed after the branch (#237).
    let taken: Vec<String> = back.taken.iter().map(|(commit, _)| commit.clone()).collect();
    let noted = notes::after(
        &notes::Session {
            name,
            root,
            role,
            stamp: stamp_commit.as_deref().ok_or("the stamp of a change with no issue is a comment, not a commit, so there is no commit to name"),
            taken: &taken,
            held: notes_held(words.no_push, pushed.lines.as_deref()),
        },
        err,
    );
    if noted != 0 {
        // not a refusal: what stopped the notes is said by `notes::after` as a limit.
        code = 1;
    }
    if !back.taken.is_empty() {
        // The planner posts nothing (#295): its new questions, written in full in the issue's file, are posted here.
        // They are read from the last commit taken back, not from the working tree, which need not hold it (#296).
        // They are written as a commit on the job's ref after the stamp, and the copy is posted (#238).
        if let (true, Some(issue), Some((commit, _)), Some(signing)) = (role == "planner", value_of(&handed.values, "issue"), back.taken.last(), signing.as_ref()) {
            if let Err(why) = brought::post_questions(name, root, issue, commit, signing, err) {
                refusal::limit(err, name, format_args!("the questions of the issue's file are not posted: {}", why));
                // not a refusal: the limit is said just above, after the session; the code says it too.
                code = 1;
            }
        }
    }
    // What the session wrote for `wb` to post (#153): its questions, the review's findings or the fix's answers, and
    // what only a person can settle. After the commits are taken back and the push.
    let posted = brought::post_written(
        &brought::Ended {
            name,
            root,
            session: session_of(),
            round: value_of(&handed.values, "round"),
            ended_well,
            held: fix_held_back(&back, value_of(&handed.values, "issue"), pushed.lines.as_deref()),
            signing: signing.as_ref(),
        },
        written,
        err,
    );
    if posted != 0 {
        // not a refusal: what could not be posted is said by `brought::post_written` as a limit.
        code = 1;
    }
    lines.close(None, sessions::Outcome::Ended { status, seconds, limit }, err);
    code
}

/// Why the notes are not pushed after a session (#237), or none: they go only where `wb` pushed the branch in this
/// start, and nowhere that it holds the branch back (WB-PUSH-4, 10, 15, 16).
fn notes_held(no_push: bool, pushed: Option<&str>) -> Option<&'static str> {
    if no_push {
        return Some("--no-push was given");
    }
    if pushed.is_some_and(|text| text.starts_with("pushed: ")) {
        return None;
    }
    Some("the branch was not pushed by this start, and the notes go only where the branch does")
}

/// Why the fix's answers are held back after its session (#153), or none: only where a commit came back, taken or
/// refused, and the branch was not pushed. A fix that commits nothing answers with reasons alone, and they are posted.
fn fix_held_back(back: &view::Back, issue: Option<&str>, pushed: Option<&str>) -> Option<String> {
    if back.taken.is_empty() && !back.refused_any() {
        return None;
    }
    if pushed.is_some_and(|text| text.starts_with("pushed: ")) {
        return None;
    }
    Some(match issue {
        None => "there is no issue, so there is no issue's branch to push the fix's commits to, and its answers would name commits the change does not hold".to_string(),
        Some(_) => "the fix's branch was not pushed, so its answers would name commits the change does not hold".to_string(),
    })
}

/// `wb start <role> <issue> --take-back` (#223): takes back the commits of the newest kept view of that role and
/// issue, through the checks of the end of a session (`view::take_back_later`), and starts and posts nothing. What it
/// will take is said first. Ends with 0 when all of the view was taken and it is removed, 1 when anything was refused
/// or left, 2 when it is used wrongly.
fn run_take_back(name: &str, role: &str, words: &Words, folder: Option<&Path>, state: Option<&Path>, err: &mut dyn Write) -> i32 {
    let others = words.print || words.stamp || words.parts || words.runner.is_some() || words.ignore_local || words.ignore_kept || words.ignore_branch || words.no_push || words.values.iter().any(|(given, _)| *given != "issue");
    if others {
        return used(err, name, "'--take-back' is used with the role and the issue alone: it starts and posts nothing");
    }
    let Some(issue) = words.get("issue") else {
        return used(err, name, "'--take-back' needs the issue: give it as the word after the role, or as '--issue'");
    };
    if !prompt::is_plain_name(issue) {
        return used(err, name, &prompt::not_plain("issue"));
    }
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
    let refuse = |err: &mut dyn Write, why: &str| refusal::limit(err, name, why);
    let Some(state) = state else {
        return refuse(err, "neither XDG_STATE_HOME nor HOME names an absolute folder, and the views are kept below the state folder of Watchbill");
    };
    let kept = match kept::find(state, role, issue) {
        Ok(kept) => kept,
        Err(why) => return refuse(err, &why),
    };
    // What may come back, as at the end of a session: by the issue's accepted file, or by the rule of the planner
    // or of a role with no issue's file (#202); never the build files of watchbill.yml. The rule is the one the
    // session ran under, as its stamp's `view:` says it: where HEAD now gives another, nothing is taken.
    let file = if role == "planner" {
        None
    } else {
        match issue::for_prompt(Path::new(&root), issue) {
            Ok(file) => file,
            Err(why) => return refuse(err, &why),
        }
    };
    let stamp_text = crate::bounded::read_plain(&kept.session.join("stamp"), crate::bounded::TEXT_CAP).ok().and_then(|bytes| String::from_utf8(bytes).ok());
    let stamped = stamp_text.as_deref().and_then(|text| text.lines().find_map(|line| line.strip_prefix("view: ").map(str::to_string)));
    let now = view_hash(role, file.as_ref());
    match stamped {
        Some(stamped) if stamped == now => {}
        Some(stamped) => {
            return refuse(
                err,
                &format!(
                    "the stamp of the kept view says `view: {}`, and the issue's file in HEAD gives `view: {}`: the rule its session ran under is not the one HEAD gives, so nothing is taken back",
                    issue::shown(&stamped),
                    now
                ),
            )
        }
        None => return refuse(err, "the stamp of the kept view has no line `view:` that can be read, so what may come back is not known, and nothing is taken back"),
    }
    let allowed = allowed_of(role, file.as_ref(), Some(issue));
    let build_files = match starter::settings(&root).and_then(|settings| issue::build_files_of(&settings)) {
        Ok(found) => found.unwrap_or_default(),
        Err(why) => return refuse(err, &why),
    };
    // What will be taken, said first: the view, its role and round, each commit with its subject.
    let shown_view = issue::shown(&kept.view.folder.display().to_string());
    let taken = kept::taken_of(&kept.session);
    let (commits, more, left) = match view::work_left(&root, &kept.view, &taken) {
        Ok(found) => (found.commits, found.more, found.uncommitted),
        Err(why) => return refuse(err, &format!("the view {} was not read, and nothing is taken back: {}", shown_view, issue::shown(&why))),
    };
    // The count is the one `work_left` made: the commits named and the ones more.
    let total = commits.len() + more;
    match view::objects_readable(&root, &kept.view) {
        Ok(()) => {}
        Err(why) => return refuse(err, &format!("the view {} was not read, and nothing is taken back: {}", shown_view, issue::shown(&why))),
    };
    let _ = writeln!(err, "{}: will take back {}, kept by the {} round {}, onto {}:", name, shown_view, issue::shown(role), kept.round, kept.view.branch_name());
    for (commit, subject) in &commits {
        let _ = writeln!(err, "{}:   {} {}", name, view::short(commit), subject);
    }
    if commits.is_empty() {
        let _ = writeln!(err, "{}:   no commit{}", name, if taken.is_empty() { "" } else { ": every commit of the view was taken before" });
    }
    if total > commits.len() {
        let _ = writeln!(err, "{}:   and {} more", name, total - commits.len());
    }
    for path in &left {
        let _ = writeln!(err, "{}:   uncommitted, and not taken: {}", name, issue::shown(path));
    }
    let list = kept.session.join("back");
    let back = view::take_back_later_of(name, &root, &kept.view, &allowed, &build_files, total, &taken, Some(&list), err);
    let still = view::tidy(name, &kept.view, &back, err);
    let written = sessions::append_back(&kept.session, &format!("take-back\n{}", back.text(still.as_deref())));
    if let Err(error) = &written {
        let held = if back.taken.is_empty() { "" } else { "the commits are on the branch, but " };
        refusal::limit(err, name, format_args!("{}the lines of this take-back could not be written in {}: {}", held, kept.session.display(), error));
    }
    for (commit, _) in &back.taken {
        refusal::inform(err, format_args!("{}: to see what the commit changes, run:\n  git show --stat {}", name, commit));
    }
    // A note on each commit taken back, naming the stamp that the kept session wrote (#237): the commit of the job's ref
    // whose message is the text of the session's `stamp`. Where none is found, no note is written.
    let taken_now: Vec<String> = back.taken.iter().map(|(commit, _)| commit.clone()).collect();
    let stamp_commit = stamp_text.as_deref().and_then(|text| stamps::commit_of(&root, issue, text));
    let noted = notes::after(
        &notes::Session {
            name,
            root: &root,
            role,
            stamp: stamp_commit.as_deref().ok_or("no stamp on the job's ref has the text of the kept session's stamp, so there is no commit to name"),
            taken: &taken_now,
            held: Some("wb pushes at the end of a session, and --take-back is not one"),
        },
        err,
    );
    if !back.taken.is_empty() {
        // A take-back is not the end of a session (#151): the person, who is there, pushes.
        let _ = writeln!(err, "{}: nothing is pushed: wb pushes at the end of a session, and --take-back is not one.", name);
        push::say_commands(name, role, &root, kept.view.branch_name(), err);
    }
    if back.all_taken() && written.is_ok() && noted == 0 {
        0
    } else {
        // not a refusal: each thing not taken, the lines not written and a note not written is said above as a limit.
        1
    }
}

/// A view made for a session, before it is posted: the view, the runner's binds as paths, `build_files` of
/// `watchbill.yml`, and for a view of the whole tree (#202) its files and bytes.
struct Prepared {
    view: view::View,
    binds: Vec<(std::path::PathBuf, bool)>,
    build_files: Vec<String>,
    size: Option<view::Size>,
    /// The commit of the branch the commits go back onto, as the view was made, or none where it was not there (#296).
    at: Option<String>,
    /// The folder of what the session writes for `wb`, bound at `view::OUT_FOLDER` (#151).
    out: std::path::PathBuf,
}

/// Makes sure of what a session in a view needs, and makes the view (#126): the runner's binds, each from the
/// root and holding neither the checkout, the folder of the organisation's file nor the state folder; a state
/// folder to make the view below; bubblewrap that can make a sandbox; `build_files`. The view is of `paths`, or of
/// every file of `HEAD` where they are none (#202). The error says why not.
fn prepare_view(
    root: &str,
    role: &str,
    issue: Option<&str>,
    paths: Option<&[String]>,
    onto: Option<&str>,
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
        // No login of the code host is in the sandbox (#151): `wb` pushes and opens the change outside it.
        if let Some(login) = push::login_of(path, machine.home, &|name| std::env::var_os(name)) {
            return Err(format!(
                "the runner's bind {} holds or lies in {}, a login of the code host, and no session holds one: wb pushes the branch and opens the change outside the sandbox. Take it out of `binds` in the organisation's file {}",
                path.display(),
                login.display(),
                machine.organisation.display()
            ));
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
    let below = state.join("watchbill").join("views");
    let (made, size, at) = match paths {
        Some(paths) => view::make_onto(root, paths, &below, &base, onto).map(|(made, at)| (made, None, at))?,
        None => view::make_whole_onto(root, &below, &base, onto).map(|(made, size, at)| (made, Some(size), at))?,
    };
    let out = match push::make_out(state, &made.folder) {
        Ok(out) => out,
        Err(why) => {
            view::discard(&made);
            return Err(why);
        }
    };
    Ok(Prepared { view: made, binds: resolved, build_files, size, at, out })
}

/// The endpoints of `reach`, one to a line, as the file `reach` of the lines holds them, or `none` (#195).
fn reach_list(endpoints: &[reach::Endpoint]) -> String {
    if endpoints.is_empty() {
        return "none\n".to_string();
    }
    endpoints.iter().map(|endpoint| format!("{}\n", endpoint)).collect()
}

/// `wb plan`, with the words after `plan`, in the repository of `folder`, with the organisation's file at `organisation`
/// and `input` as standard input. `run` gives the folder it runs in and the path fixed in `wb`; this is so that a test
/// can give its own, as `prompt` does. A relative `-F` is read from `folder`.
pub fn plan(args: &[OsString], folder: &Path, organisation: &Path, input: &mut dyn BufRead, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    refusal::begin();
    let code = run_plan(&mut Parser::from_args(args.iter().cloned()), Some(folder), organisation, input, out, err);
    refusal::ended(code, err)
}

/// The subcommands of `wb plan` (#290), in the order the usage gives them.
const PLAN_SUBCOMMANDS: &str = "add, list, show, edit, accept, remove";

/// `wb plan list`, `show`, `edit`, `accept` and `remove` (#290): the words after the subcommand, a name for all but
/// `list`, and `--no-commit` for `accept`.
fn run_plan_of(sub: &str, mut quiet: Option<&'static str>, parser: &mut Parser, folder: Option<&Path>, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let name = format!("wb plan {}", sub);
    let mut given: Option<String> = None;
    let mut commit = true;
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(refused) => return used(err, &name, &format!("'{}' is not known", refused)),
        };
        match quiet_of(&found, &mut quiet) {
            Quiet::Taken => continue,
            Quiet::Twice(why) => return used(err, &name, &why),
            Quiet::Not => {}
        }
        match found {
            Word::Long(option) if option == "no-commit" && sub == "accept" && commit => commit = false,
            Word::Short('n') if sub == "accept" && commit => commit = false,
            Word::Value(value) if sub != "list" && given.is_none() => given = Some(word(value.as_bytes())),
            found => return used(err, &name, &format!("'{}' is not known", named(found, parser).escape_debug())),
        }
    }
    if sub != "list" && given.is_none() {
        return used(err, &name, "the name of a plan is needed: wb plan list lists them");
    }
    let root = match root_for(&name, folder, err) {
        Ok(root) => root,
        Err(code) => return code,
    };
    if !Path::new(&root).join(init::CONFIG_FOLDER).join(init::CONFIG_FILE).is_file() {
        let _ = writeln!(err, "{}: {}/{} is not there. Watchbill is not set up in this repository: wb init sets it up.", name, init::CONFIG_FOLDER, init::CONFIG_FILE);
        return 2;
    }
    let plan_name = given.unwrap_or_default();
    match sub {
        "list" => plan::list(&root, out, err),
        "show" => plan::show(&root, &plan_name, out, err),
        "edit" => plan::edit(&root, &plan_name, out, err),
        "accept" => plan::accept(&root, &plan_name, commit, out, err),
        _ => plan::remove(&root, &plan_name, sessions::state_folder().as_deref(), out, err),
    }
}

/// `wb plan <subcommand>` (#290): the subcommand is the word after `plan`.
fn run_plan(parser: &mut Parser, folder: Option<&Path>, organisation: &Path, input: &mut dyn BufRead, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    // `-q` is taken before the subcommand too, as git takes an option before its arguments.
    let mut quiet: Option<&'static str> = None;
    let first = loop {
        match next(parser) {
            Ok(Some(found)) => match quiet_of(&found, &mut quiet) {
                Quiet::Taken => continue,
                Quiet::Twice(why) => return used(err, "wb plan", &why),
                Quiet::Not => break Ok(Some(found)),
            },
            other => break other,
        }
    };
    match first {
        Ok(Some(Word::Value(value))) if value == "add" => {}
        Ok(Some(Word::Value(value))) if ["list", "show", "edit", "accept", "remove"].contains(&value.to_string_lossy().as_ref()) => {
            let sub = value.to_string_lossy().to_string();
            return run_plan_of(&sub, quiet, parser, folder, out, err);
        }
        Ok(None) => return used(err, "wb plan", &format!("a subcommand is needed: {}", PLAN_SUBCOMMANDS)),
        Ok(Some(found)) => {
            let given = named(found, parser);
            return used(err, "wb plan", &format!("'{}' is not a subcommand of plan: {}", given.escape_debug(), PLAN_SUBCOMMANDS));
        }
        Err(given) => return used(err, "wb plan", &format!("'{}' is not a subcommand of plan: {}", given.escape_debug(), PLAN_SUBCOMMANDS)),
    }
    let name = "wb plan add";
    let ways = "the text is given by exactly one of -F <file>, -m <text> and --from <adapter> <number>";
    let mut given: Option<String> = None;
    let mut files: Vec<String> = Vec::new();
    let mut messages: Vec<String> = Vec::new();
    let mut froms: Vec<(String, String)> = Vec::new();
    let mut source: Option<String> = None;
    let mut level: Option<String> = None;
    let mut hash_only = false;
    let wanted = |parser: &mut Parser, flag: &str| -> Result<String, String> {
        parser.value().map(|value| word(value.as_bytes())).map_err(|_| format!("{} needs a value", flag))
    };
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(refused) => return used(err, name, &format!("'{}' is not known", refused)),
        };
        match quiet_of(&found, &mut quiet) {
            Quiet::Taken => continue,
            Quiet::Twice(why) => return used(err, name, &why),
            Quiet::Not => {}
        }
        let taken: Result<(), String> = match found {
            Word::Short('F') => wanted(parser, "-F").map(|value| files.push(value)),
            Word::Long(option) if option == "file" => wanted(parser, "--file").map(|value| files.push(value)),
            Word::Short('m') => wanted(parser, "-m").map(|value| messages.push(value)),
            Word::Long(option) if option == "message" => wanted(parser, "--message").map(|value| messages.push(value)),
            Word::Long(option) if option == "from" => match (wanted(parser, "--from"), wanted(parser, "--from")) {
                (Ok(adapter), Ok(number)) => {
                    froms.push((adapter, number));
                    Ok(())
                }
                _ => Err("--from needs an adapter and a number: --from <adapter> <number>".to_string()),
            },
            Word::Long(option) if option == "source" && source.is_none() => wanted(parser, "--source").map(|value| source = Some(value)),
            Word::Long(option) if option == "level" && level.is_none() => wanted(parser, "--level").map(|value| level = Some(value)),
            Word::Long(option) if option == "hash-only" && !hash_only => {
                hash_only = true;
                Ok(())
            }
            Word::Value(value) if given.is_none() => {
                given = Some(word(value.as_bytes()));
                Ok(())
            }
            found => Err(format!("'{}' is not known", named(found, parser).escape_debug())),
        };
        if let Err(why) = taken {
            return used(err, name, &why);
        }
    }
    let text = match (files.len(), messages.is_empty(), froms.len()) {
        (1, true, 0) => plan::Text::File(files.remove(0)),
        (0, false, 0) => plan::Text::Messages(messages),
        (0, true, 1) => {
            let (adapter, number) = froms.remove(0);
            if adapter.is_empty() || !adapter.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                return used(err, name, &format!("'{}' is not an adapter: an adapter is letters, digits, '-' and '_'", adapter.escape_debug()));
            }
            if number.is_empty() || number.len() > 20 || !number.chars().all(|c| c.is_ascii_digit()) {
                return used(err, name, &format!("'{}' is not a number of the tracker: a number is digits", number.escape_debug()));
            }
            if given.is_some() {
                return used(err, name, "--from names the job <adapter>/<number>, and takes no name");
            }
            if source.is_some() {
                return used(err, name, "--from gives the source, and takes no --source");
            }
            plan::Text::From(adapter, number)
        }
        _ => return used(err, name, ways),
    };
    if let Some(value) = &source {
        if let Some(why) = plan::source_fault(value) {
            return used(err, name, &format!("the --source {}", why));
        }
    }
    let (root, base) = match folder {
        Some(folder) => (init::repository_root_of(folder), folder.to_path_buf()),
        None => (init::repository_root(None), std::env::current_dir().unwrap_or_default()),
    };
    let root = match root {
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
    let add = plan::Add { name: given, text, source, level, hash_only };
    plan::add(&root, &base, organisation, &add, input, out, err)
}

/// `wb accept <issue>`: accepts the work of an issue whose file is accepted (`finished::accept`); a file not accepted
/// ends with 2, naming `wb plan accept` (#282).
fn run_accept(parser: &mut Parser, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let used = |err: &mut dyn Write, why: String| {
        let _ = write!(err, "wb accept: {}\n\n{}", why, USAGE);
        2
    };
    let mut issue: Option<String> = None;
    let mut commit = true;
    let mut quiet: Option<&'static str> = None;
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(refused) => return used(err, format!("'{}' is not known", refused)),
        };
        match quiet_of(&found, &mut quiet) {
            Quiet::Taken => continue,
            Quiet::Twice(why) => return used(err, why),
            Quiet::Not => {}
        }
        match found {
            Word::Long(option) if option == "no-commit" => {
                if !commit {
                    return used(err, "'--no-commit' is given twice".to_string());
                }
                commit = false;
            }
            Word::Short('n') => {
                if !commit {
                    return used(err, "'--no-commit' is given twice".to_string());
                }
                commit = false;
            }
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
        Ok(Some(root)) => {
            // Without `.watchbill/signers` on the default branch no acceptance counts (#323): refused with 1 before the
            // acceptance of the file is looked at, where Watchbill is set up.
            let set_up = !crate::bounded::is_absent(&Path::new(&root).join(".watchbill").join("watchbill.yml"));
            if let Some(why) = signers::missing(&signers::load(&root), "accept").filter(|_| set_up) {
                return refusal::limit(err, "wb accept", format_args!("{}. Nothing written.", why));
            }
            // An issue's file not accepted is accepted with `wb plan accept`, plan or not (#290, #282); `wb accept`
            // accepts the work.
            match plan::accept_word(&root, &issue, err) {
                Some(code) => code,
                None => finished::accept(&root, &issue, commit, sessions::state_folder().as_deref(), out, err),
            }
        }
    }
}

/// `wb status [<issue>] [--stage <heading>]` (#139): where each issue stands, and `wb status --session <hash>` (#239):
/// what one session was given, how it ended and what came back. It only reads.
fn run_status(parser: &mut Parser, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let used = |err: &mut dyn Write, why: String| {
        let _ = write!(err, "wb status: {}\n\n{}", why, USAGE);
        2
    };
    let mut issue: Option<String> = None;
    let mut stage: Option<&'static str> = None;
    let mut session: Option<String> = None;
    let mut quiet: Option<&'static str> = None;
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(refused) => return used(err, format!("'{}' is not known", refused)),
        };
        match quiet_of(&found, &mut quiet) {
            Quiet::Taken => continue,
            Quiet::Twice(why) => return used(err, why),
            Quiet::Not => {}
        }
        match found {
            Word::Value(value) if issue.is_none() => issue = Some(word(value.as_bytes())),
            Word::Long(option) if option == "session" && session.is_some() => return used(err, "'--session' is given once".to_string()),
            Word::Long(option) if option == "session" => {
                let Ok(value) = parser.value() else {
                    return used(err, "'--session' needs a hash".to_string());
                };
                let given = word(value.as_bytes());
                if !session_status::is_given_rightly(&given) {
                    return used(err, session_status::NEEDS.to_string());
                }
                session = Some(given);
            }
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
    if session.is_some() && (issue.is_some() || stage.is_some()) {
        return used(err, "'--session' takes no issue and no --stage".to_string());
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
        Ok(Some(root)) => match &session {
            // One session, from git alone (#239): the repository is set up as for every `wb status`.
            Some(hash) => match status::read_only(&root, err) {
                Ok(_guard) => session_status::run(&root, hash, sessions::state_folder().as_deref(), out, err),
                Err(code) => code,
            },
            None => status::status(&root, issue.as_deref(), stage, sessions::state_folder().as_deref(), out, err),
        },
    }
}

/// The repository root of `folder`, or of the folder `wb` runs in: for a command that says its own refusal.
fn root_for(name: &str, folder: Option<&Path>, err: &mut dyn Write) -> Result<String, i32> {
    let found = match folder {
        None => init::repository_root(None),
        Some(folder) => init::repository_root_of(folder),
    };
    match found {
        Err(why) => {
            let _ = writeln!(err, "{}: {}.", name, why);
            Err(2)
        }
        Ok(None) => {
            let _ = writeln!(err, "{}: this folder is not in a git repository. Watchbill is set up in one.", name);
            Err(2)
        }
        Ok(Some(root)) => Ok(root),
    }
}

/// `wb view --kept` (#176): the views kept on this machine for this repository, each with why it was kept and what
/// is left in it. Only the state folder and the objects of a view are read.
fn run_view_kept(folder: Option<&Path>, verbose: bool, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let root = match root_for("wb view", folder, err) {
        Ok(root) => root,
        Err(code) => return code,
    };
    let Some(state) = sessions::state_folder() else { return 0 };
    for entry in kept::entries(&root, &state).iter().filter(|entry| !entry.runs) {
        let _ = writeln!(out, "{}", entry.line());
        if verbose {
            for line in entry.details() {
                let _ = writeln!(out, "{}", line);
            }
        }
    }
    0
}

/// `wb clean [--dry-run | -n] [<view>]` (#176): removes the views kept on this machine for this repository that have
/// nothing left in them, saying first which. A view that is named is removed with what is left in it.
fn run_clean(parser: &mut Parser, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let name = "wb clean";
    let mut dry = false;
    let mut view: Option<String> = None;
    let mut quiet: Option<&'static str> = None;
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(refused) => return used(err, name, &format!("'{}' is not known", refused)),
        };
        match quiet_of(&found, &mut quiet) {
            Quiet::Taken => continue,
            Quiet::Twice(why) => return used(err, name, &why),
            Quiet::Not => {}
        }
        match found {
            Word::Short('n') => {
                if dry {
                    return used(err, name, "'--dry-run' is given twice");
                }
                dry = true;
            }
            Word::Long(option) if option == "dry-run" => {
                if dry {
                    return used(err, name, "'--dry-run' is given twice");
                }
                dry = true;
            }
            Word::Value(value) if view.is_none() => view = Some(word(value.as_bytes())),
            found => {
                let refused = named(found, parser);
                return used(err, name, &format!("'{}' is not known", refused));
            }
        }
    }
    let root = match root_for(name, None, err) {
        Ok(root) => root,
        Err(code) => return code,
    };
    let Some(state) = sessions::state_folder() else { return 0 };
    let entries = kept::entries(&root, &state);
    let say = if dry { "Would remove" } else { "Removing" };
    // What is to be removed, and the lines that say so first.
    let mut chosen: Vec<&kept::Entry> = Vec::new();
    let mut lines: Vec<String> = Vec::new();
    match &view {
        Some(given) => {
            let found = entries.iter().find(|entry| entry.folder.as_os_str() == given.as_str() || entry.folder.file_name().is_some_and(|last| last == given.as_str()));
            let Some(entry) = found else {
                return refusal::limit(err, name, format_args!("'{}' is not a view kept on this machine for this repository; `wb view --kept` lists them.", issue::shown(given)));
            };
            if let Some(why) = kept::not_removable_named(entry) {
                return refusal::limit(err, name, format_args!("{} is not removed: {}.", issue::shown(&entry.folder.display().to_string()), issue::shown(&why)));
            }
            let left = entry.left();
            let folder = issue::shown(&entry.folder.display().to_string());
            lines.push(if left.is_empty() { format!("{} {}", say, folder) } else { format!("{} {}: {}", say, folder, left) });
            lines.extend(entry.lost());
            chosen.push(entry);
        }
        None => {
            for entry in entries.iter().filter(|entry| !entry.runs) {
                let folder = issue::shown(&entry.folder.display().to_string());
                if kept::removable(entry) {
                    lines.push(if entry.merged { format!("{} {}: its job is done, and what it holds is in the default branch", say, folder) } else { format!("{} {}", say, folder) });
                    chosen.push(entry);
                } else if !refusal::quiet() {
                    // A view that is kept is said as it was, unless `-q` was given (#285): it only informs.
                    lines.push(format!("Keeping {}: {}", folder, entry.left()));
                }
            }
        }
    }
    for line in &lines {
        let _ = writeln!(out, "{}", line);
    }
    let _ = out.flush();
    if dry {
        return 0;
    }
    let mut code = 0;
    for entry in chosen {
        if let Err(error) = view::remove(&entry.folder) {
            code = refusal::limit(err, name, format_args!("{} could not be removed: {}.", issue::shown(&entry.folder.display().to_string()), error));
        }
    }
    code
}

/// `wb view`, with the words after `view`, in the repository of `folder`, with the organisation's file at
/// `organisation`: `run` gives the folder it runs in and the path fixed in `wb`; this is so that a test can give
/// its own, as `prompt` does.
pub fn view_command(args: &[OsString], folder: &Path, organisation: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    refusal::begin();
    let code = run_view(&mut Parser::from_args(args.iter().cloned()), Some(folder), organisation, out, err);
    refusal::ended(code, err)
}

/// `wb view <issue> <role> [--in | --out]` and `wb view <issue> <path>` (#173): what a session of a role will see,
/// and for one path whether it is in the view of each role, and why. It only reads: it makes no view, runs no
/// pattern, writes nothing and asks nothing of the network.
fn run_view(parser: &mut Parser, folder: Option<&Path>, _organisation: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let name = "wb view";
    let mut words: Vec<String> = Vec::new();
    let (mut inside, mut outside, mut kept, mut verbose) = (false, false, false, false);
    let mut quiet: Option<&'static str> = None;
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(refused) => return used(err, name, &format!("'{}' is not known", refused)),
        };
        match quiet_of(&found, &mut quiet) {
            Quiet::Taken => continue,
            Quiet::Twice(why) => return used(err, name, &why),
            Quiet::Not => {}
        }
        match found {
            Word::Long(option) if option == "verbose" => {
                if verbose {
                    return used(err, name, "'--verbose' is given twice");
                }
                verbose = true;
            }
            Word::Short('v') => {
                if verbose {
                    return used(err, name, "'--verbose' is given twice");
                }
                verbose = true;
            }
            Word::Long(option) if option == "kept" => {
                if kept {
                    return used(err, name, "'--kept' is given twice");
                }
                kept = true;
            }
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
    if kept {
        if inside || outside || !words.is_empty() {
            return used(err, name, "'--kept' takes no other word but '-v'");
        }
        return run_view_kept(folder, verbose, out, err);
    }
    if verbose {
        return used(err, name, "'--verbose' goes with '--kept'");
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
        // Not set up is 2, and no refusal; a file of `HEAD` over its bound is a limit (#261).
        Err(not_read) if not_read.status == 2 => {
            let _ = writeln!(err, "{}: {}.", name, not_read.why);
            return not_read.status;
        }
        Err(not_read) => return refusal::limit(err, name, format_args!("{}.", not_read.why)),
        Ok((guard, differs)) => {
            if differs {
                let _ = writeln!(err, "{}", starter::drift_line(name));
            }
            guard
        }
    };
    let refused = |err: &mut dyn Write, why: &str| refusal::limit(err, name, why);
    if role == Some("planner") {
        return refused(err, "the planner runs in a view of every file of HEAD, writable, with .watchbill/ read-only but for .watchbill/issues/: it reads the whole code by its role, and has no list of paths");
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
                "the issue {} has no file {}/{}.yml, so {} would run in a view of every file of HEAD, with .watchbill/ read-only and no history",
                issue,
                issue::FOLDER,
                issue,
                who
            );
            return refused(err, &why);
        }
        Err(why) => return refused(err, &why.replace(not_started, "wb start refuses it, so no session of it runs.")),
    };
    // A planner round after the acceptance is not read here: `wb view` runs no pattern (WB-VIEW-31); `wb start` and
    // `wb prompt` hold it (#290).
    if let Err(why) = issue::committed(Path::new(&root), issue) {
        return refused(err, &why.replace(not_started, "wb start refuses it, so no session of it runs."));
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
        if !any {
            refusal::answer();
        }
        // not a refusal: in no view is the answer, marked above.
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

/// The level of the repository, as `--parts` of the planner gives it: by the organisation's file where it is
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
    let mut quiet: Option<&'static str> = None;
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(refused) => return used(err, format!("'{}' is not known", refused)),
        };
        match quiet_of(&found, &mut quiet) {
            Quiet::Taken => continue,
            Quiet::Twice(why) => return used(err, why),
            Quiet::Not => {}
        }
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
            if bounded::is_absent(&Path::new(&root).join(init::CONFIG_FOLDER).join(init::CONFIG_FILE)) {
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
    let mut quiet: Option<&'static str> = None;
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(refused) => return used(err, format!("'{}' is not known", refused)),
        };
        match quiet_of(&found, &mut quiet) {
            Quiet::Taken => continue,
            Quiet::Twice(why) => return used(err, why),
            Quiet::Not => {}
        }
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
            if bounded::is_absent(&Path::new(&root).join(init::CONFIG_FOLDER).join(init::CONFIG_FILE)) {
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
    refusal::begin();
    let code = run_revision(&mut Parser::from_args(args.iter().cloned()), base, out, err);
    refusal::ended(code, err)
}

fn run_revision(parser: &mut Parser, base: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut quiet: Option<&'static str> = None;
    loop {
        let refused = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => match quiet_of(&found, &mut quiet) {
                Quiet::Taken => continue,
                Quiet::Twice(why) => return used(err, "wb revision", &why),
                Quiet::Not => named(found, parser),
            },
            Err(refused) => refused,
        };
        let _ = write!(err, "wb revision: '{}' is not known\n\n{}", refused, USAGE);
        return 2;
    }
    match manifest::manifest(base) {
        Err(why) => {
            crate::refusal::limit(err, "wb revision", format_args!("{}", why))
        }
        Ok(text) => {
            let _ = write!(out, "watchbill {}\nrevision {}\n\n{}", VERSION, manifest::hash_of(&text), text);
            0
        }
    }
}

/// `wb compare [--from <path>] <revision> <revision>` (#179): what changed between two revisions. It writes
/// nothing. `package` is the folder that holds the installed `revision/`.
fn run_compare(parser: &mut Parser, package: &Path, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let mut from: Option<std::path::PathBuf> = None;
    let mut names: Vec<String> = Vec::new();
    let mut quiet: Option<&'static str> = None;
    loop {
        let found = match next(parser) {
            Ok(None) => break,
            Ok(Some(found)) => found,
            Err(refused) => return used(err, "wb compare", &format!("'{}' is not known", refused)),
        };
        match quiet_of(&found, &mut quiet) {
            Quiet::Taken => continue,
            Quiet::Twice(why) => return used(err, "wb compare", &why),
            Quiet::Not => {}
        }
        match found {
            Word::Value(value) if names.len() < 2 => names.push(word(value.as_bytes())),
            Word::Long(option) if option == "from" && from.is_none() => match parser.value() {
                Ok(value) if !value.is_empty() => from = Some(std::path::PathBuf::from(value)),
                _ => return used(err, "wb compare", "'--from' needs the path of a repository"),
            },
            Word::Long(option) if option == "from" => return used(err, "wb compare", "'--from' is given twice"),
            found => {
                let refused = named(found, parser);
                return used(err, "wb compare", &format!("'{}' is not known", refused));
            }
        }
    }
    if names.len() != 2 {
        return used(err, "wb compare", "two revisions are needed, each a version or a hash");
    }
    compare::compare(package, VERSION, from.as_deref(), &names[0], &names[1], out, err)
}
