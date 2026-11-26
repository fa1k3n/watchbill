//! The Rust `wb`, run as the command a person types: a process, its text and its exit status.
//! The words are those of the first `wb`, which was written in Python.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::{Command, Output};

const USAGE: &str = "usage:
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
            with 2. It writes nothing and asks nothing of the network. -q
            (--quiet) is taken and changes nothing: what
            it prints is what a script reads
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

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn wb(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_wb"))
        .args(args)
        .output()
        .expect("wb could not be started")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("wb wrote text that is not UTF-8")
}

/// The version of the crate: the `version` line of the `[package]` table.
fn crate_version() -> String {
    let file = fs::read_to_string(root().join("Cargo.toml")).expect("Cargo.toml is not there");
    let mut in_package = false;
    for line in file.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
        } else if in_package {
            if let Some(rest) = line.strip_prefix("version") {
                let value = rest.trim_start().strip_prefix('=').expect("version has no =").trim();
                return value.trim_matches('"').to_string();
            }
        }
    }
    panic!("Cargo.toml has no version in [package]");
}

#[test]
fn test_WB_INIT_26_version_prints_the_version_of_the_package() {
    let run = wb(&["--version"]);

    assert_eq!(run.status.code(), Some(0));
    assert_eq!(text(&run.stdout), format!("{}\n", crate_version()));
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_INIT_36_dash_v_does_as_dash_dash_version() {
    let run = wb(&["-v"]);

    assert_eq!(run.status.code(), Some(0));
    assert_eq!(text(&run.stdout), format!("{}\n", crate_version()));
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_INIT_27_help_prints_the_usage_and_ends_with_0() {
    let run = wb(&["--help"]);

    assert_eq!(run.status.code(), Some(0));
    assert_eq!(text(&run.stdout), text(&wb(&["help"]).stdout));
    assert!(text(&run.stdout).starts_with("usage: wb <command>"), "{}", text(&run.stdout));
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_INIT_35_help_and_h_do_as_dash_dash_help() {
    for word in ["help", "-h"] {
        let run = wb(&[word]);

        assert_eq!(run.status.code(), Some(0), "{}", word);
        assert_eq!(text(&run.stdout), text(&wb(&["help"]).stdout), "{}", word);
        assert!(text(&run.stdout).starts_with("usage: wb <command>"), "{}", word);
        assert_eq!(text(&run.stderr), "", "{}", word);
    }
}

#[test]
fn test_WB_INIT_25_no_command_ends_with_2_and_the_usage() {
    let run = wb(&[]);

    assert_eq!(run.status.code(), Some(2));
    assert_eq!(text(&run.stdout), text(&wb(&["help"]).stdout));
    assert!(text(&run.stdout).starts_with("usage: wb <command>"), "{}", text(&run.stdout));
    assert_eq!(text(&run.stderr), "");
}

#[test]
fn test_WB_INIT_25_a_command_that_is_not_known_ends_with_2_and_the_usage_on_standard_error() {
    for word in ["begin", "--force", "", "Help", "--version=1"] {
        let run = wb(&[word]);

        assert_eq!(run.status.code(), Some(2), "{:?}", word);
        assert_eq!(text(&run.stdout), "", "{:?}", word);
        assert_eq!(text(&run.stderr), format!("wb: no command '{}'\n\n{}", word, USAGE), "{:?}", word);
    }
}

#[test]
fn test_WB_INIT_25_only_the_first_word_is_the_command() {
    let run = wb(&["begin", "--version"]);

    assert_eq!(run.status.code(), Some(2));
    assert_eq!(text(&run.stderr), format!("wb: no command 'begin'\n\n{}", USAGE));

    // Words after a known option change nothing, as in the Python `wb`.
    let run = wb(&["--version", "begin"]);

    assert_eq!(run.status.code(), Some(0));
    assert_eq!(text(&run.stdout), format!("{}\n", crate_version()));
}

#[test]
fn test_WB_INIT_25_the_commands_that_are_not_built_yet_are_not_known() {
    // `init` is built (#22), `revision` is built (#24) and `check` is built (#8): none is left.
    for word in [] as [&str; 0] {
        let run = wb(&[word]);

        assert_eq!(run.status.code(), Some(2), "{}", word);
        assert_eq!(text(&run.stdout), "", "{}", word);
        assert_eq!(text(&run.stderr), format!("wb: no command '{}'\n\n{}", word, USAGE), "{}", word);
    }
}

#[test]
fn test_WB_PKG_9_wb_version_prints_the_version_in_cargo_toml() {
    let run = wb(&["--version"]);

    assert_eq!(run.status.code(), Some(0));
    assert_eq!(text(&run.stdout), format!("{}\n", crate_version()));
    assert_ne!(crate_version(), "");
}

#[test]
fn test_WB_PKG_6_the_source_of_crates_has_the_name_the_macros_give_it() {
    // `packaging/rpm.sh` builds in a folder that may be inside the checkout. `%cargo_prep` then
    // writes a config that cargo merges with this one, and two names for the one folder are refused.
    let text = fs::read_to_string(root().join(".cargo/config.toml")).expect(".cargo/config.toml is not there");
    assert!(text.contains("[source.local-registry]"), "the source is not named local-registry");
    assert!(text.contains("directory = \"/usr/share/cargo/registry\""));
    assert!(text.contains("replace-with = \"local-registry\""));
}

#[test]
fn test_WB_INIT_25_a_word_that_is_not_utf_8_is_written_as_python_writes_it() {
    let run = Command::new(env!("CARGO_BIN_EXE_wb"))
        .arg(OsStr::from_bytes(b"a\xffb"))
        .output()
        .expect("wb could not be started");

    assert_eq!(run.status.code(), Some(2));
    assert_eq!(text(&run.stdout), "");
    assert_eq!(text(&run.stderr), format!("wb: no command 'a\\udcffb'\n\n{}", USAGE));
}

// A fault in `wb` itself: the panic hook is quiet, and the command ends with 1 and one line.

const CHILD: &str = "WB_TEST_PANIC_CHILD";

/// Runs only in the child process of the test below: it makes the program panic through
/// `guarded`, which is how `main` runs a command, and prints what `guarded` returned.
#[test]
fn test_WB_PKG_10_helper_a_panic_in_the_command_is_caught() {
    if std::env::var_os(CHILD).is_none() {
        return;
    }
    let mut err = std::io::stderr();
    let code = watchbill::guarded(|| panic!("a fault in wb"), &mut err);
    println!("code {}", code);
}

#[test]
fn test_WB_PKG_10_a_panic_in_the_command_ends_with_1_and_one_line_and_no_panic_message() {
    let run = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "test_WB_PKG_10_helper_a_panic_in_the_command_is_caught", "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .output()
        .expect("the test program could not be started");
    let (out, err) = (text(&run.stdout), text(&run.stderr));

    assert!(out.contains("code 1\n"), "{}", out);
    assert!(err.contains("wb: an internal error. This is a fault in wb, not in what it was given.\n"), "{}", err);
    assert!(!err.contains("panicked"), "{}", err);
    assert!(!err.contains("a fault in wb\n"), "{}", err);
}

#[test]
fn test_WB_PKG_10_main_runs_the_command_guarded() {
    let main = fs::read_to_string(root().join("src/main.rs")).unwrap();
    assert!(main.contains("watchbill::guarded("));
}
