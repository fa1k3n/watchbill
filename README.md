<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/logo-dark.svg">
    <img src="docs/logo-light.svg" width="96" height="96" alt="The mark of Watchbill: a grid of three by three rounded squares, the diagonal filled in another colour">
  </picture>
</p>

# Watchbill

A harness for AI agents in embedded software development, deployed at a fixed revision onto a project that already exists.

**Status: 0.10.0. Built: `wb init`, `wb check`, `wb upgrade`, `wb prompt`, `wb accept`, `wb status`, `wb view`, `wb answer`, `wb revert`, `wb clean`, `wb revision` and `wb start`, which starts a session of a role at a person's terminal. For an issue with an accepted file the session runs in a view: only the files its work needs, and none of the history. A person starts each role. Nothing hands work from one role to the next yet. A command that is planned is marked with its release.**

## What Watchbill is for

Watchbill is for teams that build software under a standard, such as IEC 62304 for medical devices or ISO 26262 in automotive, or under rules about which source code may leave the premises.

AI agents give such a team three problems:

- **Roles.** The standard, or the team's process under it, says who may specify, implement, verify and release. One agent session does all four unless something stops it.
- **Confidentiality.** A hosted model sees every file its agent opens. Some code may not leave the premises.
- **Assessment.** An assessor asks what the agents are told to do, and what stops them from doing anything else. A prompt typed into a terminal answers neither.

Watchbill sits between the team's tools and its agents, and is built to answer each:

- **Roles.** The work is split into roles: planner, tests, build, review and fix. Each role has a fixed, versioned prompt. Where a limit of a role can be checked, the program checks it and refuses. No agent checks its own work, and no agent merges.
- **Confidentiality.** A session works in a view: a folder that holds only the files its work needs, and none of the history. The organisation says which tools and models may have a view of which code. Code that must stay on the premises is worked on there, by a model that runs there. One role, the planner, reads the whole code to make the lists of files, and runs only where the organisation has cleared it. A view is not a barrier against sending: the files in it go to the model of its runner ([What still leaks](#what-still-leaks)).
- **Assessment.** The prompts and the rules are one pinned revision in the repository, the same on every machine and checked by hash. That is what the assessor reads.

Not all of this is built. [Where it stands](#where-it-stands) says what holds today.

Watchbill is set up in an existing repository with one command, and uses the tracker, the git server and the CI the team already has.

## Goals

1. **It fits the team's workflow, not the other way round.** Agents work in the tracker, the git server and the CI the team already has, by the team's own conventions. Nothing is replaced. What Watchbill holds fixed is stated in one place: the roles, their order, their prompts and their rules ([docs/revision.md](docs/revision.md)).
2. **Everyone can read exactly what the agents are told.** Each role's instructions are fixed and versioned, the same on every machine. What a team adds of its own is in the repository beside them.
3. **The rules are enforced, not requested.** Where a rule can be checked, software checks it and stops what is not allowed. Where a rule cannot be checked, that is said.
4. **People keep the decisions that are theirs.** A person decides what is built, accepts the reading of it before work starts, and merges the result. An agent does none of these.
5. **Duties are separated.** No work is checked by the one who did it. The tests are written from the requirement before the code, by another role than the one that builds.
6. **An agent sees only the code it needs.** For each piece of work the agent is given the files that work needs. The rest of the code is not there for it to read.
7. **Code goes only where it is allowed to go.** The organisation, not the team, says which tools and models may see which code. Code that must stay on the premises is worked on there, by a model that runs there.
8. **A team starts small.** It can use one role alone, such as a review of what a person wrote, and add the rest when it is ready.
9. **A team starts strict and opens up as it learns.** It can begin with everything protected and a model on the premises. As it sees how the work goes, it finds the parts of the code that do not need full protection, and the organisation opens those parts, and only those. Where nothing is said, the strictest applies.
10. **The process can be shown to an assessor.** What the agents are told, and what holds each rule, can be read in one place. A team's process with Watchbill in it can be assessed. Watchbill is not certified, and nothing here says it is.
11. **It needs no other product.** One package to install. `wb` asks nothing of the network unless the command is about a system on the network.
12. **Work is handed from one role to the next by the program.** When a session ends, the program checks the rule table and the next role is started, without a person starting each step. The rules, the gates and the stamps are the same as when a person starts it, and a person still accepts, answers and merges.

### When it works

A team takes an issue from its text to a merged change, and a person only accepts what is to be built, starts each role, answers questions and merges.

### Where it stands

At 0.12.0. An issue that changes a verdict changes its row.

| Goal | Today | What will hold it |
|---|---|---|
| 1. Fits the workflow | Partly: any tracker and git server with a command line, by a pattern the team writes. `wb init` writes the patterns for GitHub and GitLab only. The CI and the requirements tool are names and nothing more. `wb` installs on Fedora and Enterprise Linux only | |
| 2. What the agents are told | Held for what `wb` hands out: the pinned revision, `wb check`. `wb start <role>` posts the stamp and starts the session with that prompt itself. A session started by hand is the word of the one who started it, and so is the effort of a runner whose `start` cannot set it | |
| 3. Rules enforced | Partly: the pinned revision, the acceptance, the planner's runner and the placeholders are checked in code. What a session of an issue with an accepted file reads and changes is held by its view, and what the planner and a role with no issue's file change by theirs (#202). No other limit of the rule table is | #28 names each limit and what holds it; then #88 and the pieces #28 writes |
| 4. People keep the decisions | Partly: `wb accept` asks for a person's word, but cannot tell a person from an agent under one account. That an agent cannot merge rests on the git server's settings | The roles' accounts, #83, and the acceptance read at the tracker, #88 (both planned for 0.12.0). No issue yet checks that a role's account cannot merge |
| 5. Duties separated | Partly: the rule table says it, and nothing knows who holds which role | #83 and #88, for work that is handed on (both planned for 0.12.0). No issue yet for a session a person starts |
| 6. Sees only the code it needs | Partly: a session that `wb start <role>` starts for an issue with an accepted file runs in a view, in bubblewrap. It sees the paths of its lists and no history, and its commits come back only where `changes` allows ([The view](#the-view)). `wb accept` checks that the lists hold what each file needs to build, by the team's own command `needs` ([below](#the-lists-checked-at-acceptance)). The planner and a session for an issue with no file run in a view of every file of `HEAD`, with no history (#202). A session started by hand sees the whole checkout. What still leaks is [said below](#what-still-leaks) | No issue yet for a session started by hand |
| 7. Code goes only where allowed | Partly: checked for the planner | The other roles, #11. No issue yet for a local model for them |
| 8. Starts small | Held: `wb start <role>` starts any role alone, for an issue with no file, and a review or a fix of a change with no issue, and posts the stamp on the issue or the change ([one role at a time](#taking-it-on-one-role-at-a-time)) | |
| 9. Starts strict, opens up | Not yet: one level for the whole repository, and no level given means the strictest | #11, #65, and a level for each path, which has no issue yet |
| 10. Shown to an assessor | Partly: what the agents are told can be read. The rule table says more than the program enforces | #28 |
| 11. Needs no other product | Held: one RPM, `git`, and a build with the network off | |
| 12. Work is handed on by the program | Not yet: a person starts each role. `wb status` says where each issue stands, and nothing more | `wb handover` (#88), the roles' accounts (#83), `wb listen` (#85), the officer (#86) and `wb run` (#183) for 0.12.0 |

### Not goals

- Reports. `wb` keeps its state in git (decided, not built: [point 15](docs/revision.md#15-identity-history-and-trust)), and a report of it is for the team's own tools or a recorder.
- A backlog or priorities. What is built next, and in which order, is a person's.
- Assignment of work to people, and discussion. Those stay at the tracker; `wb` posts notices there.
- A dashboard.
- A model or an agent of its own. Watchbill ships neither.
- Deciding what is built. That is a person's.

### Later

- Hardware in the loop: tests that run on the real device.

## The commands

`wb` works as git does: run in the folder, one command per job, an exit status that scripts read. The words of `wb`, and how each command and thing is like git and unlike it, are in [docs/words.md](docs/words.md). `wb help` lists the commands by group, and `wb help <command>` says more of one. A command that is not built yet is in the layout from 0.10.0: it ends with 2, writes nothing, asks nothing of the network, and says on standard error the release it is planned for and the issue that builds it (#171).

| Group | Commands |
|---|---|
| set up | `init`, `check`, `doctor` (planned for 0.12.0, #178), `upgrade` |
| work | `plan` ([Taking a job in](#taking-a-job-in)), `accept`, `start`, `run` (planned for 0.12.0, #183) |
| see | `status`, `view` ([Seeing a view first](#seeing-a-view-first)), `log` (planned for 0.12.0, #177), `answer` |
| take back | `revert`, `clean` ([Taking away what is done](#taking-away-what-is-done)) |
| plumbing | `prompt`, `revision`, `compare` |

After 0.0.9 the releases are 0.10.0, 0.11.0, 0.12.0 and 1.0.0. A man page for each command is planned for 0.11.0.

### The rules of the layout

The owner set these on 2026-11-20 (#180). They hold in every command.

- **Every refusal is a limit or a guard, and says which.** A refusal ends the command with 1, and is written on standard error as `<command>: limit: <why>` or `<command>: guard: <why>`; a message of several lines has the label on its first. A limit is never passed: the view, classed code, the acceptance, the rule table, a file that is not as `HEAD` has it (#186, #211, #254), a file over its bound (#261), the sandbox (#202) and the checks of what comes back from a view; a fault `wb` cannot go past, such as a pattern that failed, is a limit too. A guard protects a person's time and names the flag that passes it.
- **No general `--force`.** Each flag that passes a guard is named for what it passes, and passes that alone: `--ignore-local` a local change under `reads` (`wb start`), `--ignore-kept` a view kept with work not taken back (`wb start`), `--ignore-branch` a checkout on a branch that is neither the stamp's nor the default branch (`wb start`), `--no-commit` the checks of the commit of `wb accept` and `wb plan accept`, which then write the acceptance and commit nothing. A fault that stops the commit at all, such as no pattern `branch`, is a limit, which `--no-commit` skips with the commit. After `--no-commit` the limit of #97 still holds: no prompt is handed out and no session started until the issue's file and its acceptance are committed.
- **Not every 1 is a refusal.** An answer that does not hold ends with 1 too, with no label: `wb check` finds a difference, a path is in no view, an issue is listed under Not read, a session did not end with 0. So does a person's own no, or input that ended. An end with 2 is used wrongly or not set up, and has no label.
- **The plumbing keeps fixed output.** The plumbing is `prompt`, `revision`, `compare` and the output of `check`. They print on standard output as before, and a refusal of `prompt` or `revision` prints nothing there: the label is on standard error alone. The rest is porcelain, and its words may change.
- **A command that posts refuses everything when any part is wrong.** `wb start` checks everything before it posts its stamp, `wb revert` before its marker, and `wb answer` takes every answer before it posts the first comment.
- **No `wb config`.** The files are edited by hand.
- **No auto-accept flag.** If one is ever added, the organisation's file allows it, at the lowest level only, never for classed code, and the stamp says so.

## Setting it up in a repository

The command is `wb`. Watchbill is set up in a repository the way `git init` starts one: a single command, run in the folder.

```sh
cd brake-controller
wb init
```

`wb init` proposes what the repository says about itself, asks one line per answer, shows the files and writes them:

```
project: (brake-controller)
git server (github, gitlab, gerrit, bitbucket or none): (gitlab)
repository at the git server: (brakes/controller)
tracker (github, gitlab, jira or none): (gitlab) jira
requirements (a path to files, the name of a tool, or none): (requirements/*.md)
ci driver (github-actions, gitlab-ci, jenkins or none): (gitlab-ci)
classed code (paths from the root, separated by commas, or [] for none): ([])
```

Enter keeps what stands in brackets. `wb init --yes` takes all of it and asks nothing; it shows the files before it writes them. Where the repository has the remote `origin`, `wb init` also adds the fetch line `+refs/watchbill/*:refs/watchbill/*` to it in `.git/config`, and says so first, so that a clone fetches the jobs ([Taking a job in](#taking-a-job-in)). Input that ends before a yes, and Ctrl-C, write nothing and end with 1. The result is the folder `.watchbill/` at the root of the repository, to be kept under version control with the code:

```
.watchbill/
├── watchbill.yml          the answers, the version and the hash of the revision
├── revision.sha256        the manifest of the revision
├── project/
│   └── README.md          what the project's own files are for
└── revision/              the files of the revision, byte for byte as the package has them
    ├── models.yml
    ├── rules.yml
    └── roles/
```

`watchbill.yml` reads:

```yaml
# How Watchbill is set up in this repository. Written by `wb init`.
# `watchbill` is the version of the package the repository is set up for.
watchbill: 0.0.1
revision: <the SHA-256 of the manifest>
project: brake-controller
git_server: gitlab
repository: brakes/controller
tracker: jira
requirements: requirements/*.md
ci: gitlab-ci
classed: []
build_files: ['.gitlab-ci.yml']
```

- **`watchbill` is the version of the package** the repository is set up for: the version of the `wb` that wrote the file. A revision is the version and the hash of its files together ([docs/revision.md](docs/revision.md)); the hash is the line `revision`, and is the one `wb revision` prints.
- **What it finds, it finds in the repository:** the name of the folder, the address of `origin` (only its host and path; a user name or a token in it is dropped), a folder of requirements, and the file a CI driver is known by. It asks nothing of the network.
- **The revision is copied, never edited.** The files under `.watchbill/revision/` are written by `wb` and never by hand. `cd .watchbill && sha256sum -c revision.sha256` checks them without `wb`.
- **A file that is already there is never changed.** When any file `wb init` would write is there, or a folder on the way to one is a link or not a folder, or `.watchbill/revision` is there, it writes nothing at all and names the path. Bringing a repository to a newer revision is `wb upgrade` ([Upgrading it](#upgrading-it)). A write that fails part way removes what it had written.
- **The project's own text goes in `.watchbill/project/`.** `all.md` is added to the prompt of every role and `<role>.md` (`build.md`, `review.md`, `fix.md`, `tester.md`) to that of one role. Both are optional. They are the project's files: not part of the revision, not in the manifest, and `wb check` does not fail on them, except for a placeholder that the revision does not know (see `wb check`). A prompt is the role's text of the revision, then one fixed line, then `all.md`, then `<role>.md`. This text says how things are done here: the commands, the board, the words. It never lifts a limit of a role. `wb init` leaves `project/README.md`, which says so. The placeholders of `revision/roles/placeholders.md` (`{issue}`, `{branch}` and the others) are filled in every part, and any other placeholder is refused. A placeholder is a lower-case word in braces: `{`, a letter a to z, letters a to z or `_`, then `}`, with no `$` just before it. Anything else in braces is text and stays as it is, such as `${BUILD_DIR}`, `{Issue}`, `{issue-id}` and `{ issue }`. A value that is filled in is never itself read for placeholders. The command that prints a prompt comes later.
- **`build_files` is the build** ([The lists checked at acceptance](#the-lists-checked-at-acceptance)). `wb init` writes the file or folder of each CI driver it found in the repository, whatever the answer for the driver: `.github/workflows`, `.gitlab-ci.yml`, `Jenkinsfile`, `azure-pipelines.yml`, `bitbucket-pipelines.yml`, `.circleci`, `.zuul.yaml` and `zuul.d`. Where it found none, it writes no line. The rest of the build, such as `Makefile` or `CMakeLists.txt`, a person adds.
- **`classed` is the code that is classed** ([Classed code](#classed-code)). `wb init` proposes `[]`, none, and `--yes` takes that. A person who answers gives paths from the root of the repository, separated by commas; a path that starts with `/`, holds `..` or is the root is not taken, and the question is asked again. A repository set up before this has no line `classed`, so every path counts as classed: add the line by hand.
- **The answers are only written down.** No adapter exists yet, so a git server or a tracker is a name and nothing more. For a GitHub or a GitLab `origin`, `wb init` also writes the patterns that `wb prompt` and `wb start <role>` run ([below](#starting-a-session)).

## Checking it

`wb check` says whether a repository still is at the revision it is pinned to, and whether the installed `wb` is that revision. Run it anywhere inside the repository. It writes nothing and asks nothing of the network.

```
$ wb check
changed: .watchbill/revision/roles/build.md
1 difference. The repository is not at the revision it is pinned to.
```

It compares these, and names each file that differs and how:

- each file under `.watchbill/revision/` with the manifest `.watchbill/revision.sha256`: `changed` (also a folder or a link where a file goes), `missing`, or `not of the revision` when the manifest has no line for it, or a folder it does not account for, even an empty one;
- the manifest with the line `revision:` of `watchbill.yml`, which is the hash of the manifest;
- the version and the revision the repository is pinned to with those of the installed `wb`. When they differ, both are named.
- the project's own text, `.watchbill/project/all.md` and `.watchbill/project/<role>.md` for each role of the revision, for a placeholder that the revision does not know. The file, the line and the placeholder are named, and the known ones are listed, in the words `wb prompt` uses: one function judges the text for both commands. `project/README.md` is not read, and a file that is not there is not a difference. A file that `wb prompt` would refuse (a link, a folder, text that is not UTF-8, or `project` as a link) is named as `wb prompt` names it, and is never followed. When only the project's text differs, the last line says that it cannot be used with the revision, when only `watchbill.yml`'s settings differ, that `watchbill.yml` does not fit it (or pins no revision, with no line `revision:`), and when only both differ, that they do not fit it; none of these says that the repository is not at the revision. That line is said only when something about the revision itself differs: a file of it, the manifest, the pinned hash or the version.

A link is never followed, as `wb init` refuses one on the way: a link for `.watchbill`, `.watchbill/revision`, a folder below it, the manifest or `watchbill.yml` is named as a link, once, and what is behind it is not looked at.

The exit status:

- **0**: all of it holds.
- **1**: something differs, a placeholder in the project's text is not known, or the file `watchbill.yml` is refused. Each difference is on standard output.
- **2**: `wb check` was used wrongly, the folder is not in a git repository, or `.watchbill/watchbill.yml` is not there, so Watchbill is not set up. The reason is on standard error.

## Upgrading it

`wb upgrade` pins a repository to the installed revision, in one command (#174). Run it anywhere inside the repository, after the new package is installed. It asks nothing of the network and commits nothing.

```
$ wb upgrade
wb upgrade: the repository is pinned to version 0.0.9, revision badac62e…; the installed wb is version 0.10.0, revision d31965c0….
It writes these, in this order:
  change  .watchbill/revision/roles/planner.md
  change  .watchbill/revision.sha256
  set     .watchbill/watchbill.yml: watchbill: 0.10.0 (was 0.0.9)
  set     .watchbill/watchbill.yml: revision: d31965c0… (was 'badac62e…')

Written.

The check of wb check:
The repository is at the revision it is pinned to, and the installed wb is that revision.

Nothing is committed. Commit the change alone, with nothing else in the commit; until .watchbill/watchbill.yml is committed, wb runs none of its patterns (#186):
  git add --all -- ':/.watchbill' && git commit -m 'Watchbill 0.10.0' -- ':/.watchbill'
```

- **What it writes.** The files of the installed revision over `.watchbill/revision/`, removing each file and folder there that the installed revision does not have, and its manifest as `.watchbill/revision.sha256`. In `watchbill.yml`, the lines `watchbill:` and `revision:`, set to the installed version and hash as `wb init` writes them, and each pattern that the installed `wb init` would write for this repository (from its `git_server` and `repository`, and for GitLab the host of `origin`) and that the file does not have. A line that is there is never written over: a pattern with another value stays as it is. `classed` and `build_files` are settings a person decides, and are never added.
- **It says each write first:** each file it adds, changes or removes, and each line it sets or adds, in the order it writes them. Then it writes, runs the check of `wb check` and says its result. `wb upgrade --dry-run` says the same and writes nothing.
- **What it writes over** is only the revision's own: the files under `.watchbill/revision/`, the manifest, and the two lines. It does so only when git holds everything under `.watchbill/` as committed, so git gives back every byte it wrote over. A write that fails part way puts back what it had written.
- **Refused with 1, writing nothing:** a path under `.watchbill/` that is changed, staged, not tracked or ignored by git; an installed version older than the one the repository is pinned to; a version that is not numbers separated by dots; a link under `.watchbill/revision/`, or for `.watchbill`, the manifest or `watchbill.yml`, which is never followed; a `watchbill.yml` the loader refuses, or one whose two lines are not one line each. Versions compare as numbers: 0.10.0 is newer than 0.0.9.
- **The fetch line of the jobs.** Where the remote `origin` is there and has no fetch line `+refs/watchbill/*:refs/watchbill/*`, it adds it in `.git/config`, said with the other writes (#234). Where that line is all there is to write, it writes it and runs no check and says no commit: nothing under `.watchbill/` changed.
- **Nothing to do:** a repository at the installed revision, its files, its manifest, its two lines, its patterns and the fetch line, ends with 0 and writes nothing. That is so right after an upgrade too, before the commit. The same version with another hash is upgraded.

The exit status:

- **0**: it wrote and the check holds, `--dry-run` said what it would write, or there was nothing to do.
- **1**: it was refused and wrote nothing, or it wrote and the check does not hold. The differences are said, and what was written stays.
- **2**: `wb upgrade` was used wrongly, the folder is not in a git repository, or `.watchbill/watchbill.yml` is not there; `wb init` sets Watchbill up.

## Taking a job in

A job is the text of what is to be built ([docs/revision.md](docs/revision.md), point 15). `wb plan add` takes one in: its text is stored in git as a blob, under the hash of its content, and its name is a ref, `refs/watchbill/<name>` (#234). The pipe is the way in, so `wb` needs no tracker:

```sh
gh issue view 54 | wb plan add github/54 -F - --source https://github.com/acme/brake-controller/issues/54
wb plan add -F job.txt
wb plan add brakes -m 'The brakes hold the train on a slope of 4 %.' -m 'They are tested at 80 km/h.'
wb plan add --from github 54
```

- **The text** is given in exactly one way: `-F <file>` (`--file`), `-F -` for standard input (a pipe or a here-document), `-m <text>` (`--message`), or `--from <adapter> <number>`. `-m` may be given more than once, and the texts are joined and cleaned up as `git commit -m` does it: each a paragraph, the whitespace at the end of each line taken off, runs of blank lines made one, and blank lines at the start and the end taken out. A file is read only where it is a plain file: a FIFO, a device or a folder is refused with 1, and a pipe is read with `-F -`. A text that is empty, not UTF-8, or over 1 MiB is refused with 1.
- **The name** is the word after `add`. A name may hold `/`, so that a job from each tracker has its own namespace: `github/54`, `gitlab/54`. A name that `git check-ref-format` refuses, such as `github:54`, is refused with 1, and so is a name that meets another as a folder (`github` and `github/54`). With no name, the job is named by the first 12 hex digits of its hash.
- **What it writes:** the blob of the text, `job.yml`, a tree of `job.yml` and the text under `sources/<hash>`, and a commit on the ref, moved from the value it had. It says each first. `job.yml` holds the name it was first taken in under, all its names (`names`) and, for each source, the hash, whether the text is kept, the source (`--source <address>`, or `none`), the level, the person's git identity and the time.
- **Several sources:** other text under a name that is there is added as a source, a new commit after the one before, and it says the hashes there and the one it adds. The same text again changes nothing. Text that is already a job under another name gives that job the new name too: `job.yml` lists it, and both refs point at one new commit. Each later commit of the job moves every ref that points at the commit before, in one transaction of `git update-ref` with the values they had, so the job stays one job under all its names. Looking for the text, a ref under `refs/watchbill/` that is not a job is passed over, and said once on standard error.
- **`--from <adapter> <number>`** runs the pattern `issue_text` of `watchbill.yml` for the number, as the person, takes what it prints as `-F -` takes it, names the job `<adapter>/<number>`, and keeps the command as the source. Until adapters are built, the adapter is set up where `tracker:` of `watchbill.yml` is its name and the file has `issue_text`; otherwise it ends with 2 and says to pipe the text in.
- **The level** of the job is `--level <level>`, or the repository's ([Confidentiality](#confidentiality)). The text is in every clone and cannot be deleted from them, so a level above the repository's is refused with 1, and so is a run on a machine with no organisation's file. `--hash-only` keeps the hash and the source, not the text, where the organisation's file has `jobs: hash_only: allowed`; otherwise it is refused with 1.
- **It only queues.** It runs no prompt and starts no session. With `-F` and `-m` it asks nothing of the network and runs no pattern. It pushes nothing: the refs are pushed with git, and the fetch line that `wb init` and `wb upgrade` add brings them to each clone.
- `wb plan` with no subcommand, or one that is not known, ends with 2 and names `add, list, show, edit, accept, remove`.

The exit status: 0 the job is taken in or nothing changes; 1 it was refused, and nothing is written; 2 it was used wrongly, no adapter is set up for `--from`, or the repository is not set up.

## Managing a plan

A plan is a job that `wb plan add` took in, with the issue's file the planner writes for it, `.watchbill/issues/<name>.yml` (#290). `wb plan` manages it as git manages a remote or a note:

```sh
wb plan add 54 -F spec.md     # bring the document in
wb start planner 54           # the planner reads every source and writes the issue's file
wb plan show 54               # read it
wb plan edit 54               # change it, if need be
wb plan accept 54             # agree to it
wb start tester 54            # the work
```

- **`wb plan list`** prints each plan once, by its first name, one to a line, and nothing else, for a script to read. A plan removed is not listed. A ref under `refs/watchbill/` that is not a job is passed over and said once on standard error.
- **`wb plan show <name>`** prints the plan's names, each source with its hash, where it came from, whether its text is kept, its level, who added it and when; then the issue's file and its state, and the file itself. It reads the file and its acceptance where `wb status` reads them: on the issue's branch, by the pattern `branch`, local before `origin`; with no branch of it, in the working tree. It says which:

  ```
  plan 54
  names: 54
  source 1: 3b18e512dba79e4c8300dd08aeb37f8e728b8dad  from none  kept  level internal  by Ada Lovelace <ada@example.org>  2026-11-24T09:12:40Z
  issue's file: .watchbill/issues/54.yml
  state: accepted by Ada Lovelace <ada@example.org> at 2026-11-24T10:02:11Z
  ```

  The state is `not planned` (no issue's file yet), `not accepted`, `not accepted (edited after the acceptance)`, `not accepted (a source added after the acceptance)`, `not accepted (planned again after the acceptance)`, or `accepted by <who> at <time>`. A plan whose name holds `/` (`github/54`) has no issue's file until point 15 of [docs/revision.md](docs/revision.md), and is `not planned`.
- **After the acceptance** is decided by time, to the second, against `time:` of `.accepted`. A source added in the same second or later voids it, as an edit does: a planner round lasts longer than a second, so a source of the acceptance's own second was never read by the plan that was accepted. A planner round voids it too, even where the file comes out the same, as a new round is a person asking for the plan to be looked at again: a `[stamp] planner` whose `time:` is the same as the acceptance's or later, read whole by `issue_comment_bodies`. A stamp that a `[revert]` took back counts for nothing. `wb plan show` and `wb status` say so, and `wb start` and `wb prompt` hand out no prompt of the issue and start no session of it. They read the stamps as `wb status` does: from the first lines they read for the round, else, for a plan, by `issue_comments`; then the times by `issue_comment_bodies` where a `[stamp] planner` is there. Without `issue_comment_bodies` the times are not read, the acceptance stands, and the command says so. `wb prompt` of an issue with no plan, given its round (or of a role with no round), reads no comment, so it does not see a planner round after the acceptance; `wb start` and `wb status` do, and `wb prompt` will when the stamps are in git (#236). `wb view` runs no pattern, so it does not read them; it hands out no prompt and starts no session. To accept again, remove `.accepted` and run `wb plan accept`: an acceptance is never written over. The order rests on the clocks of the machines that wrote the times, until point 15.
- **`wb plan edit <name>`** opens the issue's file in the editor git would use (`GIT_EDITOR`, `core.editor`, `VISUAL`, `EDITOR`), and commits the edit alone, as the person, with the message `Edit the issue's file (<name>)`. As `wb plan accept`, it needs the checkout on the issue's branch and the file as `HEAD` has it, and says so before it opens anything. Where nothing changed, it commits nothing. Where the editor fails, or the file no longer holds its form or names another issue, it commits nothing and leaves the edit in the working tree. Where the file was accepted, it says that the acceptance holds no longer.
- **`wb plan accept <name> [--no-commit]`** is the acceptance of the issue's file ([The issue's file and its acceptance](#the-issues-file-and-its-acceptance)), with its checks, its `.accepted` and its commit, said as `wb plan accept`. `wb accept <name>` of a plan not accepted, also one whose acceptance a source or a planner round after it voided, ends with 2 and names `wb plan accept <name>`: `wb accept` is the acceptance of the work ([A job is done](#a-job-is-done)). For a name with no plan whose file is not accepted, `wb accept` accepts the issue's file as before, until point 15.
- **`wb plan remove <name>`** takes a plan out by a marker: a commit after the plan's whose tree adds `removed`, with the person's git identity and the time, and every name of the plan moved to it in one transaction. Nothing is rewritten, and the name stays taken: `wb plan add` under it is refused, and its text is not taken as that plan's again. It is refused while a role's work on the plan is not done: a branch of any of its names, by the pattern `branch`, holds a commit the default branch does not, or this machine keeps a view of one of them with work not taken back.
- **The planner reads every source.** `wb start planner <name>` of a plan brings its sources into `.watchbill/issue/<name>.md` of the view, in the order they were added, and runs no `issue_text`. One source is its text as it is; with more, each is under a line `--- source <n>: <hash>, from <source> ---`, and a source kept as its hash alone says so.

The exit status: 0 it is done, or nothing changes; 1 it was refused (a name that is no plan, or a plan removed, among others), and nothing is written; 2 it was used wrongly or the repository is not set up.

## The prompt of an assignment

`wb prompt <role>` prints the prompt of a role on standard output, and nothing else. Whatever starts an agent's session, in any language, calls it as a process. Run it anywhere inside the repository. It writes nothing. `wb start <role>` ([below](#starting-a-session)) does all that `wb prompt` does, then posts the stamp and starts the session.

The prompt is the role's text of the pinned revision, then the project's own text (`.watchbill/project/all.md` and `.watchbill/project/<role>.md`, where they are). The project's text is taken as `HEAD` has it (#211): `wb prompt` and `wb start` refuse with 1, naming each such file, before any pattern runs, where a file of the role's prompt differs from `HEAD`, is in the working tree and not in `HEAD`, or is in `HEAD` and not in the working tree. A person commits the change, or takes it back with git. No flag lifts this. Every `part:` hash of a stamp is then the hash of a file in a commit. A file of another role does not count, and `wb check` is unchanged: it only reads the text for placeholders. A link, a FIFO, a folder or a file over 1 MiB in the place of a file counts as differing and is never opened. A file over 1 MiB in `HEAD` is refused as too big. A call asks only for what `wb` cannot know on the machine: the issue, and for the review and the fix the change and the round. The change and the round live at the tracker and the git server. Where `watchbill.yml` has the patterns for them, `wb prompt` works them out by running those (#84); otherwise they are given.

```sh
wb prompt tester 54
wb prompt build 54
wb prompt review 54                                 # the change and the round worked out
wb prompt fix 54 --change 168 --round 1             # given, as a repository with no patterns needs
wb prompt review --change 174                       # a change with no issue
```

The issue is the word after the role, or `--issue`; both together end with 2. The tester and the build with no issue end with 2 and the usage. Only the review and the fix take a change alone, with no issue (see [Taking it on one role at a time](#taking-it-on-one-role-at-a-time)). The placeholders are filled from the options, one for each, and what no option gives is derived. An option given always wins over the derived value, and the stamp holds each value as it was handed out:

- `--issue <text>`, `--change <text>`, `--requirements <text>`, `--round <text>`, `--checkout <text>`, `--branch <text>`, `--issue_address <text>`, `--change_address <text>`, `--classed <text>`, `--classed_touched <text>`, `--description <text>`, `--brought <text>`, `--questions <text>`, `--change_comment <text>`, `--needs_person <text>`. What each is, is in `.watchbill/revision/roles/placeholders.md`.
- `checkout` is the root of the repository `wb` runs in.
- `description` is `/run/watchbill/description`, the file in the sandbox that the build writes the change's description to (#151). `wb start` takes `--description` only where it prints.
- `brought` is `/run/watchbill/in`, the folder in the sandbox of what `wb start` brings in for a session (#153); `questions`, `change_comment` and `needs_person` are `/run/watchbill/questions`, `/run/watchbill/change_comment` and `/run/watchbill/needs_person`, the files a session writes for `wb` to post. `wb start` takes their options only where it prints. No role's text names `{issue_address}` or `{change_address}` since #153: they fill the project's own text.
- `classed` and `classed_touched` are worked out for a prompt that holds them: see [Classed code](#classed-code).
- `issue_address` and `change_address` are the patterns of those names in `.watchbill/watchbill.yml`, with `{issue}` and `{change}` filled in. `wb init` proposes them for a GitHub `origin` (`gh issue view {issue} --repo <repository>; gh issue view {issue} --repo <repository> --comments`, `gh pr view {change} --repo <repository>; gh pr view {change} --repo <repository> --comments`: each shows the text or the description first, then the comments, as `--comments` alone hides the first). For a GitLab `origin` it writes one command each, with `--comments`. For another server, or none, there is no pattern and the option must be given.
- `branch` is the pattern of that name in `watchbill.yml`, proposed as `issue-{issue}`, with `{issue}` filled in. The planner makes the branch, so it gets the pattern as it is. For the tester, the build, the review and the fix it is the one local branch (from the refs of the repository) whose name is the pattern, or starts with it and a `-`. None, or more than one, is refused with 1, the names found are said on standard error, and `--branch` must be given.
- For a review or a fix of a change with no issue, `branch` is the branch checked out. `wb` asks nothing of the network, so it cannot see which branch the change is from: it refuses, with 2 and the words to give `--branch`, a HEAD on no branch, the branch that `origin/HEAD` names (where changes go), and any branch where `origin/HEAD` is not there.
- `round` and `change`, where the role's text holds them (the review and the fix) and they are not given, are worked out as `wb start <role>` works them out ([the round and the change](#the-round-and-the-change)), when `watchbill.yml` has the patterns. Without them the options are needed, as before. What was worked out is said on standard error, for example `wb prompt: round 2 (1 review stamp on issue 41)`, then `wb prompt: issue 41, change 174 (from the branch issue-41)`. This, and nothing else of `wb prompt`, asks the network, through the team's own patterns. The tester and the build ask nothing.
- `requirements` is the text "the numbered requirements of issue {issue}" when the issue has no file, and "what the description of change {change} says the change does" for a review or a fix with no issue. When the issue has a file, see "An issue with a file" below.
- With no issue, the text of the review and the fix has `none` for `{issue}` and `{issue_address}`, and says what to do where the issue is none. The stamp's `issue:` is empty.
- A repository set up before these patterns have no lines for them in `watchbill.yml`. It works as it did: the options are needed, and `wb check` does not fail on it. A pattern that is not a text is refused by the loader, and that shows at `wb prompt` when it needs the pattern, not at `wb check`, which does not read the patterns.
- From revision 0.0.6 the thing at the tracker is an issue, where earlier revisions said task (#78). `--task` and `--task_address` are not known, and end with 2 and the usage. A `watchbill.yml` that still has the key `task_address`, or `{task}` in a pattern, is refused by name with 1, with what to write in its place: `issue_address`, `{issue}`. A repository set up before 0.0.6 is brought to it by hand.
- From revision 0.0.8 the key `breakdown` of `watchbill.yml` is read by nothing (#116). A `watchbill.yml` that still has it is refused by `wb prompt` and `wb check`, with 1, the line named and the words to remove it.
- The issue in the branch and in the issue's address, and the change in the change's address, are filled into text that an agent runs, so each must be plain: letters, digits, `.`, `_` and `-`, with no `..`, not starting with `.` or `-`, not ending with `.` or `.lock`. Anything else is refused by its option name, with 2 and nothing on standard output. A value that fills no pattern (the option for it is given) is taken as it is given.
- A value is taken as it is given, also when it starts with `-` (`--issue -x` or `--issue=-x`) or holds a line break.
- A placeholder that the prompt uses and that got no value, given or derived, is refused by name. An option for one it does not use is ignored. An empty value (`--issue ""`, or a variable that is not set) is refused by name, as an option with no value is: a prompt is never handed over with a hole in it. Only an empty value is refused: a value of white space is taken as it is given.
- `--parts` prints, in place of the text, the SHA-256 and the path of each part, one line each, in the form `sha256sum` prints. Run `sha256sum -c` on them at the root of the repository to see that the files are as they were.
- `--runner <name>` names the runner the prompt is handed to. The planner needs it: see [Confidentiality](#confidentiality). For the tester, the build, the review and the fix it is taken without a check in this revision, and the stamp names it. A value that is empty, given twice, or not one plain line is used wrongly.
- `--stamp` prints, in place of the text, the stamp of the assignment: what was handed out ([docs/revision.md](docs/revision.md), point 12). It is lines of `name: value` in this order: `role`, `model`, `effort`, `runner`, `version`, `revision`, `issue`, `change`, `round`, `branch`, `checkout`, `time`, `view`, `level` (only for a role that runs in a view, #209), then one line `part: <sha256>  <path>` for each part. `model` and `effort` are those of the model table for the role, and for the planner those its runner is cleared with. `runner` is the value of `--runner`, and empty when none is given. `version` and `revision` are those of the pinned revision. `time` is UTC, as `2026-11-17T18:04:05Z`. A value that is empty, or that the prompt of the role does not hold (`--change` for `build`), is written as its name and the colon alone: it was not handed out. The round is the exception: `round:` holds the round that `--round` gives, for every role. Where it is not given, it is worked out only for a role whose text holds `{round}`, as above; for the tester and the build `round:` is then empty, because `wb prompt` asks nothing of the network for them. `wb start <role>` always fills it, and it agrees with the first line it posts (#136). A value of `--issue`, `--change`, `--round`, `--branch` or `--checkout` that holds a control character, a line or paragraph separator, or white space at its start or end is refused by name, as one field must stay one line that a comment can trim. `--stamp` with `--parts`, and either given twice, are used wrongly.
- `--show-origin` prints the whole prompt as it is, and before each part one line `# <kind>  <path>  [(<source>)  ]<hash>`: the kind is `revision` (the role's text of the pinned revision), `issue` (the issue's file, `(accepted)`) or `project` (the project's text as `HEAD` has it, `(HEAD)`), and the hash is that of the part's `part:` line of the stamp. The fixed line between the revision's text and the project's has the line `# wb  …`, with no hash. Right after a part's line stands one line `# {<name>}  <where its value came from>` for each placeholder the part holds: given on the command line, named by `wb` (`{brought}` and the files a session writes), or worked out from the repository, `watchbill.yml` or the issue's file. The lines that are not origin lines, put together, are what `wb prompt` prints without the flag (#291). It is not used with `--parts` or `--stamp`, and `wb start <role> --show-origin` does the same.

For a GitHub `origin`, `wb init` writes the addresses so that each shows the issue's text or the change's description first, then the comments. Here for `example/brake-controller`:

```yaml
issue_address: 'gh issue view {issue} --repo example/brake-controller; gh issue view {issue} --repo example/brake-controller --comments'
change_address: 'gh pr view {change} --repo example/brake-controller; gh pr view {change} --repo example/brake-controller --comments'
branch: 'issue-{issue}'
```

An address that ends in `--comments` alone shows the comments and hides both the issue's text and the change's description. A repository set up with such addresses keeps its `watchbill.yml`, as `wb` never writes over it: change the two lines by hand.

### An issue with a file

The flow of an issue is: the planner reads the issue and writes `.watchbill/issues/<issue>.yml`; a person accepts it with `wb plan accept <issue>` where it is a plan of `wb plan add`, else with `wb accept <issue>`; then the four roles are handed out: `wb prompt tester <issue>`, `build`, `review` and `fix`. For these four, `wb prompt` looks for the issue's file ([below](#the-issues-file-and-its-acceptance)):

- **No file:** the role is handed out, and works from the issue's own numbered items (#116). A person may start any role alone. The chain's gates bind once a file is there.
- **A file that is accepted and committed:** the file and its acceptance must both be committed in `HEAD` and be the same in the working tree, or `wb prompt` prints nothing on standard output, names the file, says to commit it, and ends with 1. The history then holds what the issue was read as before the work it allows. `{requirements}` is its IDs and texts, one to a line (`- 54.1: <text>`). Two parts follow the role's text, before the fixed line and the project's own text: the interface, and what the role may change (`changes`) and read (its own list of `reads`). The file is a part of the prompt: `--parts` and `--stamp` give its SHA-256 and path (`.watchbill/issues/<issue>.yml`) as the second part, after the role. Text of the file is put in as it is and is never read for placeholders.
- **A file that is there and not accepted, or changed since its acceptance:** nothing is printed on standard output, the reason is on standard error, and the status is 1. No prompt of the issue is handed out until a person accepts the file again. The same holds for a file that does not hold to its form, or holds a question.
- A value of the file that reaches a prompt is refused, by the name of its field and with status 1, before a prompt is handed out, when it holds a control character, a line or paragraph separator (U+2028, U+2029) or a bidirectional control (U+202A to U+202E, among them the right-to-left override, and U+2066 to U+2069). The values are the interface, each requirement ID and text, and the paths. A line break and a tab are allowed in the interface only. The file is read once and is refused when it is over 1 MiB.
- An option `--requirements` given still wins over the file. The interface and the lists are still handed out, and the stamp says so with the line `requirements: given`, before its part lines.
- The text of the fix does not hold `{requirements}`, because the fix answers findings, not requirements. It gets the two parts and no requirement.

`wb prompt` runs the check of `wb check` first. When the repository is not at its revision, it prints nothing on standard output and says why on standard error.

The exit status:

- **0**: the prompt, or with `--stamp` the stamp, is printed.
- **1**: it was refused and nothing is printed: the planner's runner is not cleared for it (see [Confidentiality](#confidentiality)), the check does not hold, the clock reads before 1970 (`--stamp`), the role is not one of the revision (the roles it has are named), a project file is not UTF-8, a placeholder is not known or got no value, the branch is not found or not one, the issue's file is there and is not accepted (or changed since), `watchbill.yml` holds `task_address`, `{task}` in a pattern or the key `breakdown`, a value derived for the stamp (the checkout, the branch) is not one plain line, a pattern run to work out the round or the change ends with anything but 0, a fix has no review stamp to answer, or the branch has no open change or more than one.
- **2**: `wb prompt` was used wrongly (no role, a third word or an option that is not known, the issue given as a word and as `--issue`, an option given twice, with no value or with an empty one, `--stamp` with `--parts`, an option given twice, a stamped value that is not one plain line, an issue or change that fills a pattern and is not plain, the planner with no `--runner`, the tester or the build with no issue, a review or a fix with no issue whose branch is not given and is not the branch checked out), the folder is not in a git repository, or Watchbill is not set up. The usage or the reason is on standard error.

Each session has its stamp on the issue, one comment for each session, the first line `[stamp] <role> round <n>`, and the lines of the stamp after it. A review or a fix with no issue has its stamp on the change. `wb start <role>` posts it ([below](#starting-a-session)). Where a session is started by hand, the person posts the stamp as it was printed. A session with no stamp on its issue, or on its change where it has no issue, is not taken as the work of a role.

```
[stamp] build round 1
role: build
model: claude-sonnet-5-5
effort: medium
runner:
version: 0.12.0
revision: 3f0c1d9e…
issue: 12
change:
round:
branch: issue-12
checkout: /work/brakes
time: 2026-11-17T18:04:05Z
part: 9a41…  .watchbill/revision/roles/build.md
part: 51be…  .watchbill/project/build.md
```

For a runner that `wb` cannot start, four lines of shell that start a session with it, and record what the session was given. The files are written to a folder of their own and moved only when both calls succeed, so a failed call leaves none:

```
t=$(mktemp -d) || exit 1; trap 'rm -rf "$t"' EXIT
set -- build --issue 12 --requirements WB-X-1 --checkout "$PWD" --branch issue-12 --issue_address "tracker 12"
wb prompt "$@" --parts > "$t/session.sha256" && wb prompt "$@" > "$t/session.md" || exit 1
mv "$t/session.sha256" "$t/session.md" . && your-agent --prompt-file session.md
```

### Confidentiality

Confidentiality is three questions, and the view ([below](#the-view)) is the answer to the first and the third.

1. **What is in the view?** The paths the issue's file lists for the role. The planner writes the lists, `wb accept` checks them, and a person accepts them.
2. **Who may have the view?** The organisation's file says which runner is cleared for which level. In this revision code checks it for the planner only; the other roles are #11.
3. **What comes back?** The commits that keep to `changes`. Nothing else is taken.

A view decides how much is seen. The level decides by whom. The network stays open, so every file in a view goes to the model of its runner: a view is not a barrier against sending.

Every session that `wb start` starts runs in a view, in bubblewrap (#202). The planner and a role started for an issue with no file have a view of every file of `HEAD`, with no history ([A view of the whole tree](#the-view)). A session started by hand, outside `wb`, runs as the person and sees the whole checkout.

Where an assignment may run is the organisation's to say, not the project's ([docs/revision.md](docs/revision.md), point 11). In this revision it is checked for the planner, which reads more of the code than any other role.

The organisation lays out one file on each machine, by its own means: `/etc/watchbill/organisation.yml`. `wb` reads it from there and from nowhere else: no option and no environment variable changes the path, so a team cannot clear a runner for itself. It is read through the same loader as every YAML file of `wb`.

```yaml
levels: [public, internal, confidential]   # from the least to the most restrictive
default: claude-code                       # the runner `wb start <role>` takes when none is given
runners:
  claude-code:
    tools: this machine
    model: Anthropic, under the organisation's agreement
    agreement: <region, retention, training>
    up_to: internal                        # the highest level it may be given
    planner:                             # only when the runner is cleared for the planner
      up_to: public                        # the highest level of an assignment it may break down
      model: claude-opus-5-5
      effort: high
    start: [claude, --model, '{model}', --effort, '{effort}', '{prompt}']   # how `wb start <role>` starts a session
    binds: ['~/.local/bin/claude', '~/.local/share/claude', {writable: '~/.claude'}, {writable: '~/.claude.json'}]
```

- `levels` is at least one name, each once, from the least to the most restrictive.
- A runner has `tools`, `model`, `agreement` and `up_to`, which is one of the levels. `planner` is there only when the runner is cleared for the planner: its `up_to` is no higher than the runner's own, its `model` is an exact model ID and its `effort` is one of the model table's.
- `start` is there only when `wb start <role>` may start sessions on the runner ([Starting a session](#starting-a-session)). It is a list of one word or more, each a text of one line. `{model}`, `{effort}` and `{prompt}` are the only placeholders, and the first word, the program, holds none.
- `binds` is there only when the runner needs paths of its own in a view ([The view](#the-view)): its program, its settings, its login. Each is a path from the root, `/…`, or from the home folder of the person who runs `wb`, `~/…`, with no `..`, and not the root or the home folder itself. It is read-only, unless it is written `{writable: <path>}`. A path that is not there is left out. The paths above are where Claude Code keeps itself on one machine; they are an example, not a promise.
- `default` is the name of one of the runners.
- `jobs` is there only when the organisation lets `wb plan add --hash-only` keep a job's hash without its text: `jobs: {hash_only: allowed}`. Its one key is `hash_only`, `allowed` or `refused` ([Taking a job in](#taking-a-job-in)).
- A key that is not known, at any depth, a level written twice, a field missing or a value of the wrong type is refused, with the file and the place named.

The repository states its own level, as the line `confidentiality:` of `.watchbill/watchbill.yml`. Where the line is missing, the level is the most restrictive of the list. A level that is not on the list is refused by name, and so are `levels` and `runners` in `watchbill.yml`: they are the organisation's.

`wb prompt planner <issue> --runner <name>` hands out the prompt only when the organisation's file is there, names the runner, the runner has `planner`, and the repository's level is no higher than the planner's `up_to`. Otherwise it prints nothing on standard output, names on standard error the file it looked for, the runner, or the level and the `up_to`, and ends with 1. With no `--runner` the planner ends with 2 and the usage. Its stamp has `runner: <name>` and the `model` and `effort` of the runner's `planner`, and `--parts` adds two lines after the parts: `level: <the repository's level>` and `runner: <name>`.

What it does not do yet (#11): the tester, the build, the review and the fix are not refused on a runner or a missing file, and the level of an issue is not read. It cannot stop a person from pasting code into another tool, or running a session with a prompt that `wb` did not hand out. The stamp on the issue is what shows where the work was handed out.

## The view

Three questions settle confidentiality ([above](#confidentiality)). The view answers the first and the third.

A session of the tester, the build, the review or the fix of an issue with an accepted, committed file sees only the files its issue's file lists for it (#126). The program keeps this limit, not the prompt. Every other session that `wb start` starts runs in a view of every file of `HEAD`, in bubblewrap too (#202): the planner, and a role with no issue's file. No session that `wb start` starts runs as the person with the checkout writable.

- **What it holds.** A new folder, `<state>/watchbill/views/<time>-<role>-<issue>/`, made by `wb` from the paths of `changes` and of the role's `reads`, as they are in `HEAD` of the checkout. It is a new repository with one commit of those files, on a branch of the checkout's name, with no remote and no history of the repository. A path of `changes` that is not there yet is not made: the session may make it. A worktree or a sparse checkout would not do: `git show` reads every file from the history.
- **A local change stops the start** (#172). The view is made from `HEAD`, so a path of it that differs from `HEAD` in the index or in the working tree (changed, staged, deleted; a rename is named by both its paths), or a file marked `assume-unchanged` or `skip-worktree` that is changed, or any untracked file under a path of `changes`, would not be seen by the session, and its commits might not come back over it. `wb` names each such path under its list, `changes` or `reads`, and ends with 1, before the stamp is posted and before the view is made. Where every such path is under `reads` only, the refusal is a guard (`guard:`), and `--ignore-local` passes it: the session sees `HEAD`'s file. A change under `changes` is a limit (`limit:`), never passed, and the message says so. A start that went on with it has the line `local: ignored <n>` in its stamp, after `view:`, and the file `local` in its session's folder; without it the stamp has no such line. An untracked file under `changes` is named also when it is ignored, by the checkout's rules or by the person's own: the fast-forward at the end writes over an ignored file without a word, so the most restrictive applies. An untracked file under `reads` is not in the view and is not named. More than 20 paths of one list are said as the first 20 and a count. The planner and a role with no issue's file are checked over the whole tree ([A view of the whole tree](#the-view), below).
- **Where it runs.** In bubblewrap (`/usr/bin/bwrap`, from the package `bubblewrap`): the view at the path of the checkout, writable, so that `{checkout}` of the prompt is right. `/usr`, `/etc` and `/sys` are there read-only, `/tmp` and `/run` are its own, and the home folder is an empty one of its own, with the runner's `binds` in it. The checkout's `.git` and `.watchbill/`, the folder of the organisation's file and the person's other folders are not there. The variables `GIT_*` are not handed in. The network is the machine's, as the runner calls its model over it: the view limits the code that is there to send, not where it can be sent ([What still leaks](#what-still-leaks)). A network of its own is #195, planned for 0.12.0.
- **A terminal.** Where the kernel lets a program type into its terminal (`TIOCSTI`), a session could type into the person's shell after it ends. `wb` gives a session in a view no controlling terminal then. Where `/proc/sys/dev/tty/legacy_tiocsti` is `0`, as on Fedora, the kernel refuses it, and the session keeps its terminal.
- **What comes back.** When the session ends, `wb` takes each commit of the view, in order. A commit that touches a path outside `changes`, under `.watchbill/` or under `build_files` of `watchbill.yml`, a merge or a submodule is refused, with the paths named: nothing of it or after it is taken, and `wb` ends with 1. The commits before it are put on the branch the stamp names, with their messages and authors (#296), never on whatever branch is checked out. Where that branch is checked out, they go on by a fast-forward of the checkout. Where it is not, only its ref is moved, by `git update-ref` with the value it had, and `wb` says so: the branch checked out, the index and the working tree are as they were. Where it is not there, it is made at the commit of the checkout the view was made from; where it is there, its commit is that one or a descendant of it whose changes touch no path the commits change. The view's own branch has the stamp's branch's name, and the session commits on it. Where the branch was made, moved or deleted while the session ran, or is checked out in another worktree, nothing is taken, `wb` ends with 1, and the view is kept. Before the session, where the stamp's branch is there and the checkout's `HEAD` is not its commit, the start stops with 1 (`limit:`): the view is made from `HEAD`, and the session would run in another tree than its branch's. Check out the issue's branch. A checkout on another branch at the same commit, or on a branch that is neither the stamp's nor the default branch where the stamp's branch is not there yet (the planner's case), stops the start with 1 (`guard:`), for the person may mean that branch; `--ignore-branch` goes on, and the commits still come back onto the stamp's branch. Just before it moves a branch, `wb` reads the branch checked out again, and moves nothing where it changed. A file entry of `changes` covers that path only, never a path under it; a folder covers the paths under it (#192). `wb` decides it from `HEAD` of the checkout, never from the view, and asks `HEAD` first: an entry that is a folder there is a folder, and one that is a file there is a file, also when it is written with a `/`. For an entry that is not there, the `/` at its end decides: with it a folder, without it a file, so a new folder is written `docs/`. A path with a part `.git` in any case, or one git refuses to check out (the forms NTFS and HFS+ read as `.git`, `.gitmodules` as a link), is refused like a path outside `changes`. Before a commit is put on the branch, `wb` refuses a commit with a tree that is not in the form git writes (a name twice, a mode git rewrites such as `100664`, entries not in git's order), and checks that what the commit it made changes is what the view's commit changed, path for path, mode and file; a commit that differs is refused, and none after it is taken. Work left uncommitted in the view is named and not taken. What the project's git ignores is not work left (#159): `wb` judges it by every `.gitignore` of the checkout's `HEAD` and the checkout's `.git/info/exclude`, read from the checkout and never from the view, so a session cannot widen them, and a `node_modules/` that the project ignores neither keeps the view nor is listed. The person's global ignore file is not read. Where more than 20 paths are left, `wb` says how many, the first 20, and that the whole list is in the file `back` of the session's folder ([The lines it leaves](#the-lines-it-leaves)). A path from the view, and a commit's subject, is said and written escaped (#167: control characters, line and paragraph separators, bidi controls and the backslash), so a file name cannot add a line or send escape codes to the terminal.
- **The push is the program's** (#151). A view has no remote, and the session holds no login to the code host: the prompts of the planner, the tester, the build and the fix say to commit and not to push or open a change, and no role's row of the rule table allows a push (the planner's since #202). When commits were taken onto the branch, `wb` says on standard error how to look at each commit taken, `git show --stat <commit>`. Then, after the tester, the build or the fix of an issue (also one with no issue's file), it pushes, as the person and outside the sandbox, where all of these hold: every commit of the view came back and nothing was left uncommitted, the session ended with 0, `--no-push` was not given, and `watchbill.yml` as `HEAD` has it has both `change_of_branch` and `draft_change` ([The patterns](#the-patterns)).
  - **What it pushes.** The stamp's branch alone, at the last commit taken back, to `origin`, never with force: `git push origin <commit>:refs/heads/<branch>`, which it says first. The person's own git settings and login push it. A push that git refuses (one that is not a fast-forward among them) ends `wb` with 1, with what git said, and no change is opened.
  - **The change.** After a build that was pushed, `wb` runs `change_of_branch`. Where it prints no change, `wb` opens one as a draft by `draft_change`, with `{branch}` filled and the description on standard input, after it says the command and the description whole; then it reads `change_of_branch` again for the change's number. Where it prints one, or more than one, `wb` opens none. After the tester and the fix it only pushes. A pattern that fails ends `wb` with 1.
  - **The description** is written by the build's session to `{description}`, the file `/run/watchbill/description` in the sandbox: a new empty folder of `wb`'s own, `<state>/watchbill/out/<the view's name>/`, bound writable at `/run/watchbill`, outside the view's tree, so that it is never a commit. `wb` reads it when the session ends, as a plain file of 64 KiB at most, never through a link, and then removes the folder. A file that is missing or holds only white space gives a description that names the issue and says that the session wrote none. A file that is not a plain file, is larger, is not UTF-8, holds a control character (but a line break or a tab), a line separator or a bidirectional control, or holds `-` alone, opens no change: a limit, and `wb` ends with 1. The cap is well below the 128 KiB that Linux holds one word of a command to, as GitLab's `draft_change` passes the description to `glab` as one word; `-` alone would have `glab` open an editor. `--description` is taken by `wb start` only with `--prompt`, `--stamp` or `--parts`; otherwise it is used wrongly (2).
  - **Where `wb` does not push** it says why, then the commands that would, and runs none of them: `git push origin <branch>`, and after a build `draft_change` with `{branch}` filled, or that the line is not there. So it does after the planner, the review, a review or a fix of a change with no issue, a session that did not end with 0, a commit refused, work left uncommitted, `--no-push`, a `watchbill.yml` without both lines, and `--take-back`, which is not the end of a session.
  - **Whose account.** The person's own login of `gh`, `glab` and git, outside the sandbox. The account of each role is #83. The session never sees either: the variables `GH_TOKEN`, `GITHUB_TOKEN`, `GH_ENTERPRISE_TOKEN`, `GITHUB_ENTERPRISE_TOKEN`, `GH_CONFIG_DIR`, `GITLAB_TOKEN`, `GLAB_TOKEN`, `GITLAB_ACCESS_TOKEN`, `GLAB_CONFIG_DIR`, `OAUTH_TOKEN`, `GLAB_CLIENT_CERT_PASSWORD`, `CI_JOB_TOKEN`, `CI_REGISTRY_PASSWORD`, `CI_DEPENDENCY_PROXY_PASSWORD`, `CI_JOB_JWT`, `CI_JOB_JWT_V2`, `SSH_AUTH_SOCK`, `SSH_ASKPASS` and `DBUS_SESSION_BUS_ADDRESS` (the session bus leads to the keyring `gh` keeps its login in) are not handed in. They are named one by one, never by an ending such as `_TOKEN`, which would take the runner's own login of its model too. A runner's bind is refused with 1 before anything is posted where it holds or lies in `~/.config/gh`, `~/.config/glab-cli`, `~/.ssh`, `~/.git-credentials`, `~/.config/git/credentials`, `~/.netrc`, the same three of `gh`, `glab` and git below `XDG_CONFIG_HOME`, the folder `GH_CONFIG_DIR` or `GLAB_CONFIG_DIR` names, `keyring`, `gnupg`, `gcr` or `bus` below `XDG_RUNTIME_DIR`, or the folder of the socket `SSH_AUTH_SOCK` names, and where it is `XDG_RUNTIME_DIR` or a folder that holds it. A folder below `XDG_RUNTIME_DIR` that is none of those is let through. The network is not unshared, so an abstract Unix socket of the host is reachable from the sandbox; the address of the session bus is taken out for that reason too.
- **What is left.** The view is removed when all of it was taken back. It is kept, and named, when anything was refused or left uncommitted.
- **Taking a kept view back** (#223). `wb start <role> <issue> --take-back` takes back the newest kept view of that role and issue, and starts and posts nothing; the word is for the owner to settle. It is the end of a start that was cut off, and it goes through the same path as the end of a session (`view::take_back`): each commit in order, the same refusals, the same `back` lines, the same way onto the branch of the view's stamp, the view's objects read through a repository of `wb`'s own, and no hook, setting or attribute of the view run. It says first the view, the role and round of its stamp and each commit with its subject, as `wb view` and `wb status` read it, then takes it. The commits go onto the branch of the view's stamp, whichever branch is checked out (#296): its commit must be the one the view was made from or a descendant of it whose changes touch no path that a commit of the view changes; otherwise it ends with 1, naming the path, and takes nothing. The commits then go on top of it. Where it is not there, it is made at the commit the view was made from. Work left uncommitted is named and not taken, and the view is kept. A session that still runs, with its `wb` alive, is not taken back. Before it takes, it reads the header of every object the commits add, in `wb`'s own store where the take reads them, and counts the commits, each git run within 3 seconds, so a view that git would wait on (a FIFO for an object) ends with 1 within seconds and takes nothing. During the take each git run ends within 60 seconds and the whole take-back within 300 seconds. It says every commit it will take, or the first 200 and `and <k> more`, and takes exactly that many: if the count found at the take differs, it ends with 1 and takes nothing. Where the lines of the take-back cannot be written it ends with 1 and says that the commits are on the branch. Every git run in `wb`'s temporary store reads no setting of the system or of the person. What it did is appended to the file `back` of that start's folder, after a line `take-back`, and `wb status` reads what is left from the lines after the last one. It ends with 0 when all of the view was taken and removed, 1 when anything was refused or left, and 2 when it is given with any other option of `start`, or with no issue. A review or a fix started with `--change` and no issue has no view to take back this way.
- **A kept view is seen** (#222). A session whose commits were never taken back, because `wb` was ended before it or the commits were refused, leaves its view with the work in it. The next `wb start` of the same issue, from any role, ends with 1 before anything is posted and names each such view: its folder, the role and round of its stamp, each commit not taken as `<commit> <subject>`, and each path left uncommitted. It says how to take it back, `wb start <role> <issue> --take-back`, and that the folder can be removed. The refusal is a guard (`guard:`): `--ignore-kept` goes on, passes this check alone, and the stamp says `kept: ignored <n>`, after `local:`. `<n>` counts every view that the file `folder` of a start of the issue names, with work in it or not, so it can be more than the start without the flag would have named. A view of more than 200 commits is said as the first 200 and `and <k> more`, in `wb status` and in the refusal. `wb status <issue>` prints `kept: <role> round <n>, <k> commits not taken, <folder>` for each, or `kept: <role> round <n>, <k> paths left uncommitted, <folder>` where no commit is not taken (its commits were taken, or it made none), and `cut off: <role> round <n>, <folder>` for a session whose `outcome` still says `running` and whose `wb` is gone; `wb status` with no issue says on standard error how many views are kept on this machine. A stamp of the tester, the build or the fix puts an issue at its step only where a commit of its round came back (#300): a round whose start on this machine left a file `back` with no line `taken:` counts for no step, and `wb status <issue>` says `<role> round <n>: no commit came back`, with `; kept: <what it holds>, <folder>` where a view is kept for it. A round with no start on this machine, or with a start that left no `back`, counts by its stamp: `wb` writes no trailer on the commits it takes back, so another machine cannot tell a round from the branch. The review and the planner are not held to a commit. A view with no commit not taken and nothing left uncommitted is not named. They read the state folder of this machine only, and the view as described next. They write nothing but that temporary repository.
- **How it is read.** Once the session has run, the `.git` of the view is the session's. `wb` never runs git with its settings, which would run outside the sandbox a command the session wrote there: it reads the view's objects through a repository of its own, and its `HEAD` from the files. The same holds for `wb start` and `wb status` when they look at a kept view: they read nothing in it without a bound, open nothing that is not a plain file, run no hook and use no setting of the view; every git run on a view ends after 3 seconds, and the view then counts as `not read`. The commits are read through the checkout's own repository with the view's objects as alternates, which writes nothing; only the paths left uncommitted need an index and the ignore rules, which are written as a temporary repository in `$TMPDIR` and removed after (a Ctrl-C, a termination or a kill of `wb status` can leave it). All the git runs on one view end within 20 seconds in all, besides the 3 seconds of each, and a time-out is said as one, naming the git command. The tests of `wb` set the 3 seconds and the 20 seconds by two variables, `WB_TEST_RUN_BOUND` and `WB_TEST_VIEW_DEADLINE`; only a build with debug assertions reads them, which the RPM's build does not, so no such value changes `wb` as installed. No config of the person is read by those runs, so no filter driver runs on the view's files. A session whose `outcome` says `running` with its `wb` alive is skipped, and its view is not read. `--ignore-kept` runs no git on any view: it counts the views the lines name. A review or a fix started with `--change` and no issue writes no `folder`, so its kept view is not seen: a limit of this piece.
- **A view of the whole tree** (#202). The planner, which reads the whole code by its role, and a role started with no issue's file (#116), also a review or a fix of a change with no issue, run in a view of every file of `HEAD`, made as any view is: a new repository with one commit, no history, no remote and no other object of the repository; a submodule is left out. The files are copied by git's own checkout from the view's index (one pack of the blobs of `HEAD`, streamed from the checkout into the view), never one `git show` for each. Before the session `wb` says `the view holds the whole tree of HEAD: <n> files, <bytes> bytes` on standard error. There is no limit: a large tree is a slow start. The checkout's `.git` is in no sandbox: a hook, a ref, a replace ref or a setting that a session writes in its `.git` is the view's, and `wb` never runs git with it. The stamp's `view:` is `none`, as there is no list to hash.
  - **The planner's view** is writable, but for `.watchbill/` other than `.watchbill/issues/`, which is read-only (#295, which reversed #202's read-only tree after a trial in which the planner could not list the tests it guards). A build, `needs` and the tests may write in it, a `Cargo.lock` or a `target/` among them; what they write stays in the view and goes with it, and a path left uncommitted beside its issue's file is neither said nor keeps the view. A write under `.watchbill/` but `.watchbill/issues/`, `.watchbill/watchbill.yml` among them, fails inside the sandbox. Only its issue's file, `.watchbill/issues/<issue>.yml`, comes back, as a plain file and by the same take-back as any view: a commit that changes any other path, another issue's file or a `Cargo.lock` too, is refused with 1, naming the path, and nothing of it or after it is taken. Its prompt says to commit and push nothing, as those of the tester, the build and the fix do, and `wb` says `git push origin <branch>` after it takes the commit back. Any local change in the checkout stops it before anything is posted: a file changed, staged or deleted, a file marked unchanged that is changed, and an untracked file that git does not ignore, each named. `--ignore-local` passes them all but a change to its issue's file, which is never passed, untracked and ignored too; the stamp then says `local: ignored <n>`. An ignored file elsewhere, such as a build folder, does not stop it. Its view holds the classed paths with the rest: the planner is not checked on the class, as it changes only its file, and classed code is still not worked on by any other role until #65 ([Classed code](#classed-code)).
  - **The view of a role with no issue's file** is writable, but for `.watchbill/`, which is read-only. What comes back is any commit that keeps out of `.watchbill/` and `build_files` and that git takes, by the same take-back as any view: a commit that does not is refused with 1, naming the path. It is checked for local changes over the whole tree, as the planner is, and `--ignore-local` passes every one: the stamp then says `local: ignored <n>`, and the session sees `HEAD`'s files. `wb` says before it starts that the view is `HEAD`'s.
  - **Nothing untracked is written over.** At every take-back, of any view, a commit that adds a path the checkout holds and does not track, ignored or not, is refused with 1, naming the path: the fast-forward would write over it without a word. Such a path cannot be known before the start.
  - **Taking it back later** works as for any kept view, by the rule its session ran under: the stamp's `view:` says it, and where the issue's file in `HEAD` now gives another `view:`, nothing is taken, and `wb` says both. Then `wb start planner <issue> --take-back` takes back the planner's issue's file alone, and a role whose issue has no file takes back by the rule above.
- **The stamp** has the line `view:`, after `time:`: the SHA-256 of the list of paths, one to a line, as the file `view` of the lines holds it, or `none` for a view of the whole tree. `wb prompt --stamp` gives the same line: it says what was handed out, as the model and the effort do. For a session in a view the line `level:` follows it (#209).

### Seeing a view first

`wb view` shows what a session of a role will see, before it starts, and says for one file whether it is in the view and why (#173). A list that falls short is found before a session is spent on it.

```sh
wb view 54 build              # the paths of the build's view, and nothing else
wb view 54 build --in         # what of the root the view holds, one level deep
wb view 54 build --out src    # what of src/ the view leaves out, one level deep
wb view 54 src/brake.c        # in or out, for each role, and why
```

- **`wb view <issue> <role>`** prints the paths of the view of that role, one to a line: the list the start makes the view from, `changes` and then the role's `reads`. It prints nothing else: no level, no line `view:` and no local changes. It ends with 0.
- **`--in [<folder>]`** prints what of `HEAD` the view holds in the folder, one level deep: each file directly in it, written as its path from the root and not the name alone, and then `<count><tab><folder>/` for each folder below it, with the count of the files of the view under it. **`--out [<folder>]`** prints the same for what the view leaves out. With no folder it is the root. A folder is a path from the root, with or without a closing `/`. Both print names and counts only: why a file is in or out is `wb view <issue> <path>`. A folder that is not in `HEAD` ends with 1; a path that is a file, or a folder that starts with `/` or holds `..`, ends with 2, the file with the words to use `wb view <issue> <path>`. A file of the view that is not yet in `HEAD` is not counted. `--in` and `--out` together end with 2.
- **The level, `view:`, the model, the effort and the runner** are in the stamp: `wb prompt <role> <issue> --stamp` prints `level:` after `view:` (#209), for every role, as every session that `wb start` starts runs in a view (#202). The level is that of the repository as `--parts` of the planner gives it (the organisation's file, else the line `confidentiality:` of `watchbill.yml`); where `watchbill.yml` gives no level that holds (a level that is not on the organisation's list), it is the most restrictive level of the organisation's file, and `wb prompt` says why on standard error; it is written `not known` only where there is no organisation's file and no line `confidentiality:`, and then nothing is said. `--stamp` takes `--runner` as `wb start` does. The local changes are named by the start, which refuses on them (#172).
- **`wb view <issue> <path>`** takes a path from the root of the repository, as the issue's file writes it. A word that is a role is the role: a folder named `build` is written `./build`. It prints one line for each role with a view, the tester, the build, the review and the fix: `build: in: reads.build, line 9 of .watchbill/issues/54.yml: include`, with the list and the line of the issue's file that hold the path and the entry as written; or `review: out: <why>`. The reasons are, in this order: under `.watchbill/`; a path git does not take (#192); listed, and not in `HEAD`, with the list and the line (a session may make a path of `changes`); a submodule; ignored by the checkout, with the rule, and not in `HEAD`; not in `HEAD`; under `build_files` and not listed; not listed. A folder that is not listed, and holds paths that are, is out, and they are named. It ends with 0 when the path is in the view of at least one role, and with 1 when in none.
- **Ignored** is asked of git in the checkout: the `.gitignore` files of the working tree and `.git/info/exclude`. The person's global file is not read.
- **No list of paths.** An issue with no file, or one not accepted or not committed, ends with 1: with no file, `wb view` says that the role would run in a view of every file of `HEAD`, with `.watchbill/` read-only; otherwise it gives the reason `wb prompt` gives. `wb view <issue> planner` ends with 1, saying that the planner runs in a view of every file of `HEAD`, writable, with `.watchbill/` read-only but for `.watchbill/issues/`.
- **It only reads.** It makes no view, runs no pattern of `watchbill.yml`, posts nothing, writes nothing, not in the state folder either, and asks nothing of the network. `watchbill.yml` is read as `HEAD` has it (#186): where the working tree's file differs, `wb view` says so in one line on standard error and goes on with the file of `HEAD` (#254); a file missing from the working tree but in `HEAD` counts as differing; where it is not in `HEAD`, it ends with 2. A word that is not known, or a path that starts with `/`, holds `..` or names the root, ends with 2.

- **`wb view --kept`** lists the views kept on this machine for this repository (#176), one to a line, and takes no other word but `-v`. A kept view is the folder a session left, with work in it that was not taken back, or with nothing left. For each view whose session does not run it prints `<folder> <role> round <n> issue <issue> <k> commits not taken <m> paths uncommitted`, or `<folder> <role> round <n> issue <issue> not read` where git could not read the view within its bound, and nothing else, so that a script can read it. The words are fixed. **`-v`** (`--verbose`, as `git branch -v`) adds below each line, indented: `why:` it was kept (read from the `back` of its session: a commit refused, commits not taken, paths left uncommitted, a take-back that failed, a session cut off, or that nothing is left in it), `commit <short> <subject>` for each commit not taken back (the first 200, then `and <k> more`), `uncommitted <path>` for each path left, and `not read: <why>`. It reads only the state folder of this machine and the objects of a view, and writes nothing. The files that a commit changes are the commit's own: they are not listed. A view of another repository on the same machine is not listed.

### Taking away what is done

`wb clean` is a new command (#176): none of the others takes away what `wb` left on the machine. `wb status` and `wb view --kept` show it; `wb start <role> <issue> --take-back` takes one role's work back and removes only that view; `wb revert` is about the comments of an issue. In the trial of 0.0.9 every view was kept, and a person removed them by hand with `rm -rf`.

```
wb clean -n               # Would remove <folder> ... / Keeping <folder>: <what is left>
wb clean                  # Removing <folder> ..., then removes
wb clean <view>           # removes that view with its work
```

- **What it removes without being told:** a view of this repository whose session ended, that git could read, and that holds no commit not taken back and no path left uncommitted. A commit that a take-back took (`took:` in the `back` of its session, #277) is taken. The choice is one function (`kept::removable`), so that a later piece can widen it: a job whose change is merged is done (#282).
- **What "work left" is:** the first-parent commits from the commit the view was made with to its `HEAD`, and the paths that are not committed, as `wb status` reads them (#222, #277). A commit on another branch of the view, a stash, or a commit that the session reset away is not seen, and the view is removed with it. The session's `wb` is known to be alive by its process ID and its start time, which is read on this machine: a view of a `wb` that runs on another host that shares the state folder, or under another name than `wb`, reads as cut off. The state folder is per machine and per person (`XDG_STATE_HOME`).
- **What it says first:** on standard output, `Removing <folder>` for each view it will remove and `Keeping <folder>: <what is left>` for each view it keeps, all of them before the first removal. The words are those of `git clean`. `--dry-run` (`-n`) says `Would remove` and removes nothing.
- **A view with work left** is kept unless it is named: `wb clean <view>`, with the name of its folder or its whole path as `wb view --kept` prints it, removes it with its work, and says first each commit (its short name and subject) and each path it loses, 20 of each at most and then `and <k> more commits` or `and <k> more paths`. Naming it is the waiver; there is no `--force`. Only a view of the listing is taken: a path elsewhere, `..` or a link is refused.
- **Never removed:** a view that git could not read (a FIFO for an object, a view that is not as `wb` made it), and a view whose session runs, named or not. A view whose session was cut off is kept unless it is named. `wb clean` never removes the checkout, a branch or the lines of a session, and it removes only a folder directly below the folder of the views.
- **A job that is done** (#282): a view that was read, whose session ended and whose commits are all in the default branch, with no path uncommitted, is removed where its job is done ([A job is done](#a-job-is-done)). A commit of a view is in the default branch only where that very commit was merged: a take-back makes new commits, and those it took are not counted as left. A done job's view with a commit that the default branch lacks is kept and named.
- **Not built here:** the worktrees of sessions out of sight (#140) moved to 0.13.0, so `wb clean` handles kept views only.
- **A session that is starting:** `wb start` writes `outcome: running`, with its process ID and start time, before it writes the file `folder` and before it posts the stamp, so a view is never without the mark of a `wb` that is alive, and a view named to `wb clean` while its session starts is not removed.
- **The exit status:** 0 it is done or there is nothing to remove; 1 a named view is not removed (not a view of this repository, not read, its session runs, or it could not be removed), written as `wb clean: limit: …`; 2 it was used wrongly or this folder is not in a git repository.

### What still leaks

A view keeps the rest of the code and the history away from a session. It does not keep everything:

- **The network is the machine's.** A session in a view shares the network of the machine, so it can reach every service that listens on this machine: `localhost`, a database, a local model server, and anything else with no login. It can reach the rest of the network too. It is not held to its runner's model. Until the sandbox has a network of its own (#195, planned for 0.12.0), a team runs no service without a login on a machine while sessions run on it.
- **Every file of the lists goes to the model**, as the session reads it. The level of the repository still decides which runner may have them. A view decides how much is seen, not where it is sent.
- **A test that passes or fails tells one bit** about code the session cannot read, at each try. A session that runs the tests many times learns more.
- **The runner's own settings and login are inside the view**, as its `binds`. What the runner keeps there, the session can read.
- **Nothing stops a person** from pasting code into another tool.
- **A stamp is a comment.** Anyone who can comment on the issue can write one, until each role has an account of its own (#83, planned for 0.12.0).
- **A session started by hand**, outside `wb`, runs as the person, with a shell, and writes the checkout itself: it can change and commit `.watchbill/watchbill.yml`, move a ref or write a hook (`.git/hooks/` runs when `wb accept` commits). Every session that `wb start` starts runs in a box (#202): none has the checkout's `.git`, the planner can write nothing under `.watchbill/` but `.watchbill/issues/` and gets back its issue's file alone, and a role with no issue's file cannot write `.watchbill/`, and gets back nothing under it or under `build_files`. `wb` runs the patterns of the file as the person, so it runs them only while the file is as `HEAD` has it ([The patterns](#the-patterns)). That stops an uncommitted change only: a commit in the checkout lifts it, so a person reads `git log -p -- .watchbill/watchbill.yml` before the next command. What a person commits there is run.
- **The project's text** is held as `HEAD` has it, as `watchbill.yml` is. That stops an edit nobody committed, so that a stamp's `part:` hashes are files of a commit. A commit lifts it, and what a person commits there goes into the prompts. A session that `wb start` starts cannot commit under `.watchbill/` (#202); a session started by hand can.

## Starting a session

`wb start <role>` starts a session of a role in the person's terminal (#84). One command checks what `wb prompt` checks, posts the stamp, and starts the session with the model and the effort of the stamp. A person writes no stamp, no shell and no model name.

```sh
wb start planner 54
wb start tester 54
wb start build 54
wb start review 54
wb start fix 54
wb start review --change 174        # a change with no issue
```

Each takes the words `wb prompt` takes for its role, `--runner <name>`, `--prompt`, `--ignore-local`, `--ignore-kept`, `--ignore-branch` and `--no-push` ([The view](#the-view)); `wb start <role> <issue> --take-back` takes a kept view back and takes no other option. Run it anywhere inside the repository.

The role is a word after `start`, never a command of its own. The old form, `wb build 54`, ends with 2 and says the new one with the same words: `use: wb start build 54`. A word after `start` that is no role of the revision ends with 2 and names the roles.

### What it checks

In this order. The first fault ends it, before anything is posted or started:

1. Everything `wb prompt` checks for the same words. The message is the same, after the command's name (`wb start build:` where `wb prompt` says `wb prompt:`), and so is the status.
2. The rule table: a fix follows a review. A fix with no review stamp before it is refused with 1, also when `--round` is given, and also when the stamps cannot be read. The number of review rounds is not checked: a person who starts one more review has decided to. That limit is held where roles are handed on without a person (#88).
3. The class: no classed code is worked on by a session started here until a session sees only its files (#65). See [Classed code](#classed-code).
4. The runner has `start`, and no word of it is 128 KiB or more once filled: Linux hands a program no longer word. Otherwise 1, saying to use `--prompt`. A program that `start` names by its path is there, as a session in a sandbox that cannot find it is said only on the terminal (#202); otherwise 1, naming it.
5. Standard input and standard output are a terminal. Otherwise 1, naming #65: no session is started out of a person's sight before the sandbox.
6. No local change to a file of the view, for a session of an issue with an accepted file, and none in the whole checkout for the planner and a role with no issue's file ([The view](#the-view)). Otherwise 1, naming the files.
7. The view, for every session (#202): no bind of the runner holds or lies in the checkout, the folder of the organisation's file or the state folder; there is a state folder; bubblewrap is at `/usr/bin/bwrap` and can make a sandbox; the checkout's `HEAD` names a branch; the view is made; and a program that `start` names by its path lies, as written and as resolved, under what the sandbox holds: `/usr`, `/bin`, `/sbin`, `/lib*`, `/etc` and the runner's `binds` (#202). Otherwise 1, saying why. Where unprivileged user namespaces are off, bubblewrap cannot make a sandbox, and its own words are said.

Then `watchbill.yml` must have `issue_comment`, or `change_comment` for a change with no issue. Without it the command is refused with 1, and nothing is posted.

### The runner

The runner is `--runner <name>`, or else the `default` of the organisation's file ([Confidentiality](#confidentiality)). With neither, the command is refused with 2 and the usage. The runner must be one of the file, and have `start`.

### The round and the change

They are worked out, never asked. `--round` and `--change` are still taken, and win.

- **The stamps** are the lines `[stamp] <role> round <n>` that `issue_comments` prints: the first line of each comment on the issue, one to a line. Every stamp is posted by `wb`, so their form is known. For a review or a fix with no issue, they are those `change_comments` prints, on the change.
- **The round** of the planner, the tester, the build and the review is one more than the stamps of that role. The fix takes the number of review stamps: it answers the last review.
- **The change** of a review or a fix of an issue is the one open change from its branch, as `change_of_branch` prints it. None, or more than one, is refused with 1, naming what was found and `--change`.
- **What was worked out is said** on standard error, before the checks: `wb start review: round 2 (1 review stamp on issue 41)`, and for a review or a fix of an issue the change, once, on a line of its own: `wb start review: issue 41, change 174 (from the branch issue-41)`, or `(given)` where `--change` gave it. A review or a fix with no issue says `wb start review: change 174`. `wb prompt` says the line only for a change it worked out, and prints the prompt alone on standard output.
- `wb start <role>` always needs the round, for the stamp's first line. With no `--round`, and no `issue_comments` (or `change_comments`), it is refused with 1. A `--round` given is a whole number from 1, or the command was used wrongly (2).

`wb prompt` works out the same, by the same function, where the role's text holds `{round}` or `{change}` and the patterns are there.

### The stamp and the session

- **The stamp.** `wb` says on standard error what it will post. It then runs `issue_comment` with `{issue}` filled and the stamp on standard input: the line `[stamp] <role> round <n>`, then the lines `wb prompt --stamp` prints, with `runner:` the runner it starts. A review or a fix with no issue posts it by `change_comment`, with `{change}` filled. What the command prints goes to standard error. When it ends with anything but 0, nothing is started, and `wb` ends with 1.
- **The session.** `wb` runs the runner's `start`, its words filled: `{model}` and `{effort}` are those of the stamp, and `{prompt}` is the whole prompt as one word. Nothing goes through a shell. It runs at the root of the checkout, in the foreground, in the person's terminal, in its view, in bubblewrap (#202): of the paths of its issue's file, or of every file of `HEAD`, which `wb` says first. Ctrl-C, Ctrl-\ and a hangup (a closed terminal) go to the session, and `wb` waits for it. In a view Ctrl-C and Ctrl-\ reach the session inside bubblewrap, as they would outside it, also where the session has no controlling terminal (bubblewrap's `--new-session`, used where `dev.tty.legacy_tiocsti` is not 0, as on the kernel of Enterprise Linux 9) (#271). A termination (`kill` of `wb`) is sent on to the session, and `wb` waits for it too: what the session committed is taken back after any end of it, and a further termination or hangup while `wb` takes it back does not end `wb` before `back` and `outcome` are written. In a view a termination reaches the session inside bubblewrap, so it can end as it would with no view: bubblewrap is started with the interrupt, the quit, the hangup and the termination ignored, the session with them as `wb` found them, and `wb` sends the termination on to the session that bubblewrap names (#257). Where the session there has no controlling terminal, `wb` sends an interrupt, a quit and a hangup on to it too, as the terminal cannot (#271); where it has one, it gets them from the terminal, once. What `wb` sends on reaches the session's own process, the runner that `start` names: a runner that is a wrapper gets the signal in its own process, and its children only as it passes them on. bubblewrap gives a session ended by a signal as the status 128 plus the signal's number, and `outcome` names the signal for it, also where a session in a view ends with such a status by itself. Should `wb` not find the session within 2 seconds, a termination goes to bubblewrap, which ignores it, so it does not reach the session. Where the session there has no controlling terminal, a hangup does not reach it either, and an interrupt or a quit (Ctrl-C, Ctrl-\) ends bubblewrap and the session with it, as nothing else would reach the session; where it has one, it gets those from the terminal, and bubblewrap is not killed under it. `wb` says so in one line on standard error. In a view the runner's words are run through `env`: a first word that holds `=` is read by `env` as a variable, not as the program. Where `/usr/bin/env` is not there, bubblewrap is started as before #257, and an interrupt, a quit, a hangup or a termination that reaches it ends it and the session. A termination sent to the whole group reaches the session twice, once direct and once from `wb`. A hangup that does not come from the terminal does nothing while the session runs, except in a view whose session has no controlling terminal. After the session, a write to a terminal that is gone is no error; what `wb` says goes to the session's folder (`back`, `outcome`). A `kill -9` cannot be caught; a `kill -9` of `wb` always ends bubblewrap and the session in it (`--die-with-parent`). `wb` ends with 0 when the session ends with 0, and with 1 otherwise.
- **The planner's issue** (#295). The planner's session has no login to the tracker, and needs none: before the stamp is posted, `wb` runs `issue_text`, with `{issue}` filled, and `issue_comment_bodies`, as the person, outside the sandbox, each read through the cap of a text file (1 MiB). What `issue_text` printed, the issue's text alone, goes into the view whole and byte for byte as `.watchbill/issue/<issue>.md`: nothing is ever cut from it; the comments whose first line is `[questions] <issue>` or `[answers] <issue>`, or a form of 0.0.8, go whole, in order, into `.watchbill/issue/<issue>.questions`. No stamp and no other comment is brought in. Both are read-only in the sandbox and never taken back. The stamp holds the SHA-256 of the first, `issue_text: <sha256>`, before its parts, and the planner writes the same under `from`. Where a line is not in `watchbill.yml`, a pattern fails, prints more than 1 MiB, or `HEAD` holds `.watchbill/issue`, the start ends with 1, as a limit naming it, and nothing is posted or started. Without `issue_text` the start is refused, naming the line, which `wb upgrade` adds. `issue_text` is a pattern of its own because no rule can tell the end of an issue's text from the lines a tracker prints around a comment (the second look at #297): `issue_address`, which prints the comments too, is not read for it.
- **The planner's questions** (#295). The planner posts nothing: it writes each question in full in the issue's file, under `questions`. After the session, where a commit of its view was taken back, `wb` reads the questions on the issue by `issue_comments`, and posts the questions of the file that are not there yet in one comment by `issue_comment`, as the person, outside the sandbox, after it says the comment on standard error. It reads the issue's file from the last commit it took back, not from the working tree. A question that holds a control character (but a line break or a tab), a line separator or a bidirectional control is posted by no one: `wb` names it escaped, as a limit, and ends with 1. The comment: `[questions] <issue> q1 q2`, then for each question its line `q<n>: <what is unclear>` and the rest under `<details>`, as the file has it. A file with no new question posts nothing. Where they cannot be posted, `wb` says so as a limit and ends with 1; the commit taken stays on the branch.
- **What every other role reads and writes** (#153). The tester, the build, the review and the fix have no login either, and need none: their prompts name no program of the tracker. Before the stamp is posted, `wb` runs, as the person and outside the sandbox, each through the cap of a text file (1 MiB): `issue_text` and `issue_comment_bodies` of the issue, and for the review and the fix `change_text` and `change_comment_bodies` of the change. It writes what they print into a new folder of its own, which the sandbox holds read-only at `/run/watchbill/in`, `{brought}`, outside the view's tree: `issue.text`, the issue's text, whole; `questions`, the comments whose first line is a question or an answer of that role (`[questions] <issue>.<role>`, `[answers] <issue>.<role>`, or a form of 0.0.8), whole and in order, and no stamp or other comment; and for the review and the fix `change.text`, the change's description, whole; `change.comments`, the comments of the change whose first line is `[review] <on> round` or `[fix] <on> round`; and `change.diff`, the diff of the branch against its merge base with the default branch, of the paths of `changes` and the role's `reads` only (of every path where there is no issue's file, and of none where that list is empty), with `--submodule=short`, `--no-ext-diff` and `--no-textconv` whatever the person's git config. Its first line is `wb`'s own: the ref and the commit the diff ends at, and its base, as the checkout holds them; nothing is fetched, so a branch behind the server's shows there. Where the issue is none, there is no `issue.text`, and `questions` comes from the change. A line missing, a pattern that fails, more than 1 MiB, or a diff that cannot be had ends the start with 1, as a limit naming it, before anything is posted. Without `change_text`, the review and the fix are refused, naming the line, which `wb upgrade` adds.
- **What `wb` posts for a session** (#153). A session posts nothing. It writes in `/run/watchbill/`, beside `{description}`: `{questions}`, each question in full, from a line `q<n>: <what is unclear>`; `{change_comment}`, the review's findings or the fix's answers; and `{needs_person}`, what only a person can settle. After the commits are taken back and after the push, `wb` reads each as it reads the description (a plain file of 64 KiB at most, never through a link, UTF-8, with no control character but a line break or a tab, no line separator and no bidirectional control) and says on standard error what it will post, then posts it, as the person, outside the sandbox. The questions of the tester, the build, the review and the fix that `issue_comments` does not show yet go in one comment `[questions] <issue>.<role> q1 q2`, each question's line and the rest under `<details>`, by `issue_comment` (by `change_comment` where the issue is none). The review's findings and the fix's answers go on the change by `change_comment`, under the line `[review] <on> round <n>` or `[fix] <on> round <n>`; only where the session ended with 0, and for the fix, where a commit of the session came back, only where its branch was pushed (a fix that commits nothing answers with reasons, and they are posted): otherwise `wb` says why, and the text, for a person to post. What only a person can settle goes on the issue (the change where the issue is none) under the line `[needs_person] <issue>.<role>` (`[needs_person] <issue>` for the planner), whatever the session ended with, unless the file holds only `none`, in any case, with or without backticks and a final full stop. The first line of each comment is `wb`'s own, never the session's. A file that cannot be read, or that holds such a character, is posted by no one: `wb` says why as a limit, never what it holds, and ends with 1.
- **`--prompt`** prints the prompt as `wb prompt` does, and posts and starts nothing. `--stamp` and `--parts` do as with `wb prompt`, with `--prompt` or without. No `default` runner is taken then: the output is that of `wb prompt` for the same words.
- **`--list`** (#291) prints the roles of the pinned revision that `wb start` starts, one to a line and nothing else. With `-v` (`--verbose`) each line adds `view` or `tree` and `cleared` or `not-cleared`: whether the organisation's file clears a runner for the role, for the planner one runner with both `start` and `planner` up to the level of the repository, as `wb start planner` needs. It is given alone, reads, writes nothing and asks nothing of the network. `--show-origin` is for a person to read, as `git config --show-origin` is; `--parts` and `--stamp` are the plumbing. Its origin lines start with `# `, as the headings of a role's text do, and are told apart by their kinds.

### The lines it leaves

Each start of `wb start <role>`, started or refused, leaves a few plain lines on the machine where it ran (#124). When a run does not work, a person reads them and sees why, without the session's transcript.

- **Where:** a new folder `<state>/watchbill/sessions/<time>-<role>-<issue>/`. `<state>` is `$XDG_STATE_HOME` when it is an absolute path, else `~/.local/state`. `<time>` is the stamp's, in UTC, with `:` left out, as `2026-11-19T094959Z`. `<issue>` is `none` for a role started with no issue. A folder that is there is never written over: a second start in the same second gets `-2`, a third `-3`.
- **Said first:** `wb start build: will write the lines of this start in <folder>`, on standard error, before it is written.
- **`stamp`:** byte for byte the stamp posted, or the one that would have been posted. A start refused by the checks of `wb prompt` has none.
- **`outcome`:** lines of `name: value`, in this order:

  ```text
  outcome: refused        outcome: not posted     outcome: ended
  check: class            status: 1               status: 3
  status: 1               message: …              seconds: 412
  message: …
  ```

  `check` is the one that refused: `prompt`, `rule`, `class`, `start`, `terminal`, `ci`, `kept`, `branch`, `local` or `view`. `message` is the first line of what was said. `not posted` is a stamp that was not posted, and no session was started. `ended` gives the session's own status (or `signal:`, with `SIGHUP`, `SIGINT`, `SIGQUIT`, `SIGKILL` or `SIGTERM` by name and any other by its number, when a signal ended it) and the seconds it ran. `outcome` is written first, before the session starts, as `outcome: running`, `pid: <process ID of wb>` and `started: <its start time in clock ticks>`, and the lines above replace it when the start ends. A `wb` that is ended before its session leaves `running`: a folder whose `outcome` says `running`, and whose `pid` is gone, is not `wb`, or started at another time, was `cut off`, and `wb status <issue>` says so.
- **`ci`,** for a review: `ci: green`, `ci: red` or `ci: pending`, the word `change_checks` printed; `ci: not read` where there is no `change_checks`, and `ci: unread` where it printed something else or failed. It is kept whether or not the review started, so that a count of views that were green and CIs that were not can be made.
- **`local`,** for a start that `--ignore-local` let go on: the paths of the local changes it ignored, one to a line, escaped. A start with none has no such file.
- **`folder`,** for a session in a view, written before it starts: `folder:` the view's folder, `base:` the commit it was made with, `head:` the commit of the checkout it was made from, `branch:` and `issue:`. A `wb` that was ended leaves no `back`, and the view is found by this file.
- **`view` and `back`,** for a session in a view: the list of its paths, one to a line, whose SHA-256 is the stamp's `view:`; and what came back, a line each: `taken: <commit on the branch> <subject>`, `took: <commit of the view> <commit on the branch>` after each (#277), so that a commit taken is not named again as not taken; `took:` is not checked against the branch: where the branch is reset after a take-back, the commits it took are still not named and not taken again, so take them from the view by hand (`git -C <view> log` names them, and `git cherry-pick` takes one), `refused: <commit of the view> <subject>: <why>`, `not taken: …`, `uncommitted: <path>`, `failed: <why>`, and last `view: removed` or `view: kept <folder>`. A path is written escaped, so each is one line.
- **`push`,** after a session of the tester, the build or the fix of an issue (#151): `pushed: <commit>`, or `not pushed: <why>`; after a build that was pushed, `change: <number>`, or `change: not opened: <why>`.
- **Not in the way:** a folder that cannot be written is said on standard error. It changes neither the status nor whether the session starts.
- **What writes none:** `--prompt`, `--stamp` and `--parts`, and `wb prompt` and `wb check`. Nor does a command used wrongly (status 2 before the checks begin): no start was tried.
- **What they are:** diagnostics on one machine, not a record ([Not goals](#not-goals)). Only `wb start` and `wb status` read them back, to name a view kept with work not taken back and a session cut off; nothing else is decided by them, and nothing in them leaves the machine. They are plain files, so the team's own tools or a recorder can collect them, and a person removes them when they are no longer wanted.

### The patterns

`wb prompt` and `wb start <role>` ask the tracker and the git server only through lines of `watchbill.yml`, the team's own commands. For a GitHub `origin`, `wb init` writes them, here for `example/brake-controller`:

```yaml
issue_comment: 'gh issue comment {issue} --repo example/brake-controller --body-file -'
issue_comments: 'gh api repos/example/brake-controller/issues/{issue}/comments --paginate --jq ''.[].body | split("\n")[0]'''
change_comment: 'gh pr comment {change} --repo example/brake-controller --body-file -'
change_comments: 'gh api repos/example/brake-controller/issues/{change}/comments --paginate --jq ''.[].body | split("\n")[0]'''
change_of_branch: 'gh pr list --repo example/brake-controller --head {branch} --state open --json number --jq ''.[].number'''
change_checks: 'gh pr checks {change} --repo example/brake-controller --json bucket --jq ''[.[].bucket] | if length == 0 then "pending" elif any(. == "fail" or . == "cancel") then "red" elif all(. == "pass" or . == "skipping") then "green" else "pending" end'''
issue_comment_bodies: 'gh api repos/example/brake-controller/issues/{issue}/comments --paginate --jq ''.[] | .body + "\u0000"'''
change_comment_bodies: 'gh api repos/example/brake-controller/issues/{change}/comments --paginate --jq ''.[] | .body + "\u0000"'''
change_text: 'gh pr view {change} --repo example/brake-controller --json body --jq .body'
draft_change: 'gh pr create --repo example/brake-controller --head {branch} --draft --fill --body-file -'
issue_text: 'gh issue view {issue} --repo example/brake-controller --json body --jq .body'
```

- `issue_comment` and `change_comment` post a comment from standard input. `issue_comments` and `change_comments` print the first line of each comment, one to a line. `change_of_branch` prints the numbers of the open changes from `{branch}`, one to a line. At GitHub the comments of a pull request are those of an issue, so `change_comments` reads them the same way.
- `issue_text` prints the issue's text alone, with no comment: `wb start planner` brings it into the view (#295). For GitHub `wb init` writes `gh issue view {issue} --repo <repository> --json body --jq .body`, for GitLab `glab issue view {issue} --repo <address> -F json | jq -r .description`, and `wb upgrade` adds it where it is missing. `wb start` brings it in for the tester, the build, the review and the fix too (#153).
- `change_text` prints the change's description alone, with no comment: `wb start review` and `wb start fix` bring it in (#153). For GitHub `wb init` writes `gh pr view {change} --repo <repository> --json body --jq .body`, for GitLab `glab mr view {change} --repo <address> -F json | jq -r .description`, and `wb upgrade` adds it where it is missing.
- `issue_comment_bodies` and `change_comment_bodies` print each comment whole, followed by a NUL byte. One line break straight after a NUL is not read as part of the next comment: `gh --jq` and `jq -r` print one. `wb answer` reads them ([Answering the questions](#answering-the-questions)), and `wb start` reads them to bring a role's questions and answers, and the review's and the fix's comments, in ([The stamp and the session](#the-stamp-and-the-session)); no session reads them itself.
- `draft_change` is the command that opens the change of `{branch}` as a draft, its title filled from the commits and its description read from standard input. `wb start build` runs it after a session that it pushed, where no change is open, with the description the session wrote ([The view](#the-view), #151); where `wb` does not push, it says it, filled. `wb upgrade` does not change a `draft_change` that is there: an older one, which reads nothing from standard input, opens the change with the commits' text.
- `open_issues` is no longer read (#155): `wb init` does not write it, and a line that is there is passed over. Listing the tracker's issues is the tracker's own program's job.
- Each is one line for `/bin/sh -c`, run in the root of the checkout, as an agent runs the addresses. The values filled in (`{issue}`, `{change}`, `{branch}`) are plain names, as for the addresses. A value given that is not is refused with 2, by its option. A branch worked out that is not is refused with 1, saying to give `--change`.
- A repository set up before these lines gets them from `wb upgrade` ([Upgrading it](#upgrading-it)); one with another server adds them by hand. Without them, `wb prompt` asks for `--change` and `--round` as before, and `wb start <role>` is refused, naming the line it needs.
- `wb start <role>` asks nothing of the network but through these lines, the runner's `start`, and the push of the branch after a session (#151).
- **Only as `HEAD` has it** (#186). `wb` runs a pattern of `watchbill.yml`, reads `needs` and `build_files` from it, and fills a prompt from it, only while `.watchbill/watchbill.yml` in the working tree is the file that `HEAD` has. It reads the file of `HEAD` once, with no replace refs, and takes every pattern from those bytes. `wb start`, `wb prompt`, `wb answer`, `wb revert` and `wb accept`, which hand something out or write, are refused with 1, naming the file, before anything runs or is posted, where the file differs from `HEAD`, is staged and not committed, or is not in `HEAD` at all. **`wb status` and `wb view` only read, so they say in one line on standard error that the file differs from `HEAD` and go on (#254): every pattern they run and every key they read comes from the file as `HEAD` has it, never the working tree's, so a pattern changed only in the working tree is not run. A file that is missing from the working tree but is in `HEAD` counts as differing: they say so and go on with the file of `HEAD`. Where the file is not in `HEAD` at all they end with 2, as for a repository that is not set up. Something in its place that is not a plain file (a FIFO, a device, a folder, a symlink), or is over 1 MiB, counts as differing and is never read. The mode ends with the command: `starter::read_only` gives a guard, and the refusal above holds again once it is dropped.** The command `draft_change` that `wb start build` says after a session is held the same way: where the file differs, `wb` says the refusal and no line. A person reads the change and commits it, or takes it back with git, before the next command. It is a limit: no option lifts it. **It stops an uncommitted change only.** Every session that `wb start` starts runs in a sandbox and cannot commit the file (#202). A session started by hand runs as the person, with a shell, and can commit a changed `watchbill.yml`, move a ref or write a hook. A commit in the checkout by anyone lifts the limit, so a person reads `git log -p -- .watchbill/watchbill.yml` before the next command. A `watchbill.yml` over 1 MiB in `HEAD` is refused as too big, with 1 in every command, `wb status` and `wb view` too (#261). Every file of the working tree that a session can write is read by one function (#261): a file that is not a plain file is never opened, one that is is opened without waiting and looked at again once open, and what is read is read through a cap (1 MiB for the text files), so a FIFO or a link to `/dev/zero` in the place of `watchbill.yml` ends `wb prompt` and `wb start` with 1 within seconds, naming the file. `wb check` runs no pattern and is not held. `wb init` and `wb upgrade` write the file and are not held; `wb upgrade` says that the file is to be committed (#174).

For a GitLab `origin`, `wb init` writes the same patterns with `glab`, here for `brakes/controller` on `gitlab.example.com`, and `issue_address` and `change_address` besides:

```yaml
issue_comment: 'glab api --hostname gitlab.example.com --method POST projects/brakes%2Fcontroller/issues/{issue}/notes -F body=@-'
issue_comments: 'glab api --hostname gitlab.example.com projects/brakes%2Fcontroller/issues/{issue}/notes --paginate --output ndjson | jq -r ''select(.system == false) | .body | split("\n")[0]'''
change_comment: 'glab api --hostname gitlab.example.com --method POST projects/brakes%2Fcontroller/merge_requests/{change}/notes -F body=@-'
change_comments: 'glab api --hostname gitlab.example.com projects/brakes%2Fcontroller/merge_requests/{change}/notes --paginate --output ndjson | jq -r ''select(.system == false) | .body | split("\n")[0]'''
change_of_branch: 'glab mr list --repo https://gitlab.example.com/brakes/controller --source-branch {branch} -F json --jq ''.[].iid'''
issue_address: 'glab issue view {issue} --repo https://gitlab.example.com/brakes/controller --comments'
issue_text: 'glab issue view {issue} --repo https://gitlab.example.com/brakes/controller -F json | jq -r .description'
change_address: 'glab mr view {change} --repo https://gitlab.example.com/brakes/controller --comments'
change_checks: 'glab api --hostname gitlab.example.com projects/brakes%2Fcontroller/merge_requests/{change} | jq -r ''if (.iid | type) != "number" then error("not a merge request") else (.head_pipeline.status // "none") as $s | if $s == "success" then "green" elif $s == "failed" or $s == "canceled" then "red" else "pending" end end'''
issue_comment_bodies: 'glab api --hostname gitlab.example.com ''projects/brakes%2Fcontroller/issues/{issue}/notes?sort=asc&order_by=created_at'' --paginate --output ndjson | jq -j ''select(.system == false) | .body + "\u0000"'''
change_comment_bodies: 'glab api --hostname gitlab.example.com ''projects/brakes%2Fcontroller/merge_requests/{change}/notes?sort=asc&order_by=created_at'' --paginate --output ndjson | jq -j ''select(.system == false) | .body + "\u0000"'''
change_text: 'glab mr view {change} --repo https://gitlab.example.com/brakes/controller -F json | jq -r .description'
draft_change: 'glab mr create --repo https://gitlab.example.com/brakes/controller --source-branch {branch} --draft --fill --push=false --yes --description "$(cat)"'
```

- The host is that of `origin`, without a user name or a token; without it `glab` asks gitlab.com. An `origin` with a port gets none of these lines.
- The comments are posted through `glab api`, which reads the whole text from standard input; `glab issue note` would ask for it at a prompt. `glab api` has no `--jq`, so `issue_comments` and `change_comments` need `jq` on the machine of the team. `wb` does not. They keep no system note, and print the first line of each other note.
- The project is written in the path of the API with each `/` as `%2F`.
- `draft_change` takes the title from the commits (`--fill`) and the description from standard input; a given `--description` is kept over the one `--fill` would make. `--fill` alone has `glab` push the branch itself: `--push=false` stops it, so `wb`'s own push is the only push (#151).
- `issue_comment_bodies` and `change_comment_bodies` ask the notes oldest first (`sort=asc`), and keep no system note. They, and `draft_change`, were written from the documentation of the API and of `glab`, and have not been run against a GitLab server.

`change_checks` is read by `wb start review` before it posts the stamp (#127). It prints one word for the change, `green`, `red` or `pending`, and ends with 0.

- `wb start review` starts on `green`. On `pending` it is refused with 1, saying to wait. On `red` it is refused with 1, saying that the change goes to a person, and that the lists or the stubs of the planner may be wrong: the CI builds the whole tree and runs every test, and the session saw only its view. A word that is not one of the three, or a status but 0, is refused with 1, naming `change_checks`.
- The review's prompt gets nothing of it: not the word, and never the CI's log. Only `wb` reads the word.
- For GitHub the word comes from the `bucket` of `gh pr checks`: `red` where a check failed or was cancelled, `pending` where one has not ended or has a bucket that is not known (the most restrictive applies), else `green`: only `pass` and `skipping` are green, so a change whose checks were all skipped is green. A pull request with no check reported, as a change just pushed has before its first check, ends `gh` with 1 and `wb start review` is refused, naming `change_checks`: wait, and start it again. A repository with no CI has no `change_checks` line.
- For GitLab the word comes from the status of the merge request's head pipeline, through `glab api`: only `success` is `green`, `failed` and `canceled` are `red`, and anything else, a merge request with no pipeline too, is `pending`. A `skipped` pipeline or one that waits for a manual job is `pending` and does not end by itself: a person runs the job, or sets the line another way. An answer that is not a merge request (a wrong number, a wrong project, no token) makes `jq` fail, and the review is refused, naming `change_checks`. This line was written from the documentation of `glab` and has not been run against a GitLab server.
- With no `change_checks`, the review starts, and `wb` says on standard error that the CI was not read.

### Classed code

The project says which code is classed, as the list `classed` of `watchbill.yml` ([docs/revision.md](docs/revision.md), point 4). Each is a file or a folder, from the root of the repository:

```yaml
classed: [src/brake, firmware/safety.c]
```

- A path of the issue's `changes` is classed when it is a path of the list, lies below one, or is a folder above one. Such an issue is refused with 1, naming the paths and #65.
- **With no `classed`, every path is classed.** Where a class is not given, the most restrictive applies. `wb init` asks for the list and proposes `classed: []`, and `wb init --yes` writes that: what is classed is for a person to say. A repository set up before this adds the line by hand.
- **With no issue's file** (or no issue), the paths the session may change are not known. The role is then refused unless the list is empty: `classed: []`.
- The planner is not checked on the class. It changes only the issue's file. Its view holds every file of `HEAD`, classed ones too (#202); classed code is still not worked on by a session started here until #65.
- **The review is told by the program** which paths are classed, and which paths of the change lie under them, so that it never guesses. `{classed}` is the paths of `classed`, one to a line, `none` for `classed: []`, and for a file with no `classed` the words that every path is classed. `{classed_touched}` is the paths of the change, from the diff of its branch against the merge base with the default branch, that meet a classed path, or `none`; where the branch or the default branch is not there it says `cannot be told` and why. The review reports it and may add what the paths alone do not show. The review's text also names the issue's file and its acceptance as the process's own files on the branch, which are no finding.
- `--prompt` is not refused on the class.

### Two runners, worked

**Claude Code**, a hosted model:

```yaml
    start: [claude, --model, '{model}', --effort, '{effort}', '{prompt}']
```

`claude` with a prompt as its last word starts an interactive session with the prompt as the first message. `--model` and `--effort` set what the stamp says.

**goose with a model of Ollama**, on the machine. Checked with goose 1.45 from Fedora 44, on 2026-11-19:

```yaml
  goose-local:
    tools: this machine
    model: qwen3-coder:30b, by Ollama on this machine
    agreement: nothing leaves the machine
    up_to: confidential
    start: [goose, run, --interactive, --provider, ollama, --model, 'qwen3-coder:30b', --text, '{prompt}']
```

- `goose run --text` takes the first message, and `--interactive` goes on as a session after it. `goose session` takes no first message. The address of Ollama is `OLLAMA_HOST`, or what `goose configure` wrote.
- **The stamp's effort is not what ran.** goose has no option for the effort, so this `start` leaves `{effort}` out.
- **The stamp's model is not what ran either.** The stamp has the model table's model, a vendor's, which Ollama does not have, so this `start` names its own model and leaves `{model}` out. The runner's `model` line says what runs.

Where a `start` leaves out `{model}` or `{effort}`, the stamp's model or effort is what was asked for, not what ran.

### A whole issue

With `classed` written, and the patterns of `watchbill.yml` in place:

```sh
wb start planner 54     # writes .watchbill/issues/54.yml on the branch issue-54
wb accept 54        # the person accepts it, and wb commits the acceptance (wb plan accept 54 for a plan)
wb start tester 54        # the tests, red
wb answer 54        # the person answers what a session asked, any time it asked
wb start build 54         # the code, committed; wb pushes it and opens the change as a draft
wb start review 54        # round 1
wb start fix 54           # answers round 1
wb start review 54        # round 2
```

Each command posts its stamp and starts its session. After the tester, the build and the fix, `wb` pushes the branch (#151). The person answers the questions with `wb answer`, and merges.

### The exit status

- **0**: the session ended with 0; with `--prompt`, `--stamp` or `--parts`, it was printed.
- **1**: it was refused, by `wb prompt`'s checks or by one of the five after them; `watchbill.yml` has no pattern it needs; a pattern ended with anything but 0; the session could not be started, or did not end with 0; or a commit of its view was refused, or the view's work could not be taken back; or git refused the push, or the change was not opened (#151).
- **2**: it was used wrongly, as for `wb prompt`, or with no runner and no `default`, `--description` without `--prompt`, `--stamp` or `--parts`, a `--round` that is not a whole number from 1, or an issue or a change to post on that is not plain; the folder is not in a git repository; or Watchbill is not set up.

### What it does not do

- Start a session without a terminal, for the listener (#85, planned for 0.12.0).
- Keep the session's last message and hand it on (#88, planned for 0.12.0).
- Know whether a session of the same role already runs on the issue (#88, planned for 0.12.0).
- Say what the session does with its prompt. The stamp says what was handed out and started.

## A job is done

`wb accept <issue>` accepts the work (#282). Where the issue's file is accepted as it stands, and a role has committed work on the branch of the issue, a person says that the work is as wanted, at any stage: after the build alone, or after the review and the fix. It says first `accepting the work <commit> (not run: <roles>)`, the commit being the head of the branch, and the roles whose stamp does not count. It then writes `.watchbill/issues/<issue>.finished`, beside `.accepted`, and commits it alone on the branch of the issue: the lines `commit:`, `by:`, `time:` and, where a role did not run, `not run:`. A record is never written over: after a later commit voided it, it is removed in a commit of its own, and `wb accept` is run again. It holds while the tip of the branch is the commit that added it, whose parent is the commit it names: any later commit voids it. The signed tag of the acceptances is #235's, and is made beside the record.

It ends with 1, writing nothing, where a limit is not passed: no tester, build or fix stamp counts (no role has committed work); a question is open; a view is kept with work not taken back; the change of the branch touches classed code, which needs a person as verifier (`classed_code.verifier` of the rule table); the checkout is not on the branch of the issue or differs from `HEAD`; there is no `issue_comments` to read the roles from. Skipping a review is a person's choice; a limit is not. `--no-commit` writes the record and says the commit, which then is not an acceptance until it is committed.

A job is done by this acceptance or by a merge ([Where each issue stands](#where-each-issue-stands)). Then `wb status` does not list it, `wb revert` of it ends with 1, and `wb clean` removes the views of it whose commits are all in the default branch and that hold nothing uncommitted (#176).

## Where each issue stands

`wb status` lists the issues in the flow under headings, as `git status` lists files under theirs: each issue under the last step it has done, with its branch. It only reads, writing nothing but the temporary repository in `$TMPDIR` that reading a kept view needs (#222), removed after, and it says what is, never what to do. It runs anywhere inside the repository.

```
Not planned:
  178  issue-178

Planned:
  176  issue-176  #181  2 not pushed

Accepted:
  175  issue-175-event-source  #180

Reviewed:
  177  issue-177

Questions open:
  176  176.q1, 176.q2
```

- **The headings**, in the order of the flow. An issue goes under the first that holds:
  - `Fixed`: the last stamp on the issue is a `[stamp] fix`;
  - `Reviewed`: the last stamp is a `[stamp] review`;
  - `Built`: there is a `[stamp] build`;
  - `Tested`: there is a `[stamp] tester`;
  - `Accepted`: the issue's file and its acceptance are on the issue's branch, and the acceptance holds for the file;
  - `Planned`: the issue's file is on the issue's branch, and not accepted: also where its acceptance is void by a source of its plan or a planner round after it ([Managing a plan](#managing-a-plan));
  - `Not planned`: none of these.
  A new planner takes the issue back: only the stamps after the last `[stamp] planner` count, and with none after it the issue is under `Accepted` or `Planned` by its file.
  A stamp of the tester, the build or the fix moves the issue to its heading only where a commit of that round came back, as the starts on this machine record it (#300); a round with no such record counts by its stamp.
  A heading with no issue is not printed. In the list, no round, no command and no word for a step going on is printed; `wb status <issue>` names a round that brought nothing back. Bring a kept view back with `--take-back`: work taken by hand leaves no `taken:` line, and its round still counts as bringing nothing.
- **The questions, last.** `Questions open` lists each issue that has a question no one has answered, with the names of those questions. The issue stays under its step too. A question is asked by a comment whose first line is `[questions] <issue>.<role> q1 q2` (`[questions] <issue> q1` for the planner), which names `<issue>.<role>.q1` and `<issue>.<role>.q2`, and closed by `[answers] <issue>.<role> q1 …`, which names the questions it answers ([Answering the questions](#answering-the-questions)). The forms of revision 0.0.8 are still read: a question `[question] <name>`, closed by an `[answer] <name> …` line, and `[answer] <issue> suggested`, which closes every question asked before it. Other words after `[answers] <issue>.<role>` close nothing.
- **Who answered.** `issue_comments` prints the first line of each comment and no author, so `wb` cannot yet tell an answer of a person from another. Until it reads who wrote a comment (#83), each `[answers]` and `[answer]` line counts as a person's; a line that starts with `[question]` or `[questions]` is never an answer. It reads first lines only, so an answer below the first line of a comment is not seen, which errs towards "open"; and a choice number is not held against the choices of the question (`wb answer` holds it when it posts): `[answer] 5.q1 7` closes `5.q1` whatever its choices are.
- **Without `issue_comments`** in `watchbill.yml` no stamp, question or answer is read, so each issue is at `Accepted` or below; `wb status` says so once on standard error.
- **The planner's stamps are read whole** (#290). For an issue whose acceptance holds and whose first lines hold a `[stamp] planner` that counts, `wb status` reads the whole comments once by `issue_comment_bodies`, for the stamps' `time:`. A stamp of the acceptance's second or later voids it, as a source of its plan does, and the issue is under `Planned`. Without `issue_comment_bodies` the times are not read and the acceptance stands; `wb status <issue>` says so.
- **Which issues:** those in the flow: an issue whose branch, by the pattern `branch` (local, or at `origin`), holds a commit that the default branch of `origin` does not. Git alone says which, from the refs as they are: it writes nothing and fetches nothing. The default branch is `refs/remotes/origin/HEAD`, else `main`, else `master`; where none is there, or git cannot list the branches, `wb status` says so and ends with 2. The tracker's own program lists the rest. A title, which `wb status <issue>` prints, is printed escaped: control characters, line and paragraph separators, bidi controls and the backslash are written as escapes (#167).
- **An issue that is done** is not listed, also when its file is on the default branch. Its branch is done when it is merged into the default branch; or when its whole change is one commit on it, found by `git patch-id --stable` of the diff from the merge base to the branch against each commit since the merge base (a squash merge, as a pull request is merged); or when it is a local branch whose upstream at `origin` is gone, as after `git fetch --prune`. The limits: a squash whose diff differs from the branch's (a branch not brought up to date with the default branch, or a squash edited at the server) is not found, and the issue stays listed; an issue merged since the last fetch stays listed until a fetch, and a branch that is only on the server shows once it is fetched.
- **The line of an issue:** its number, its branch and, once there is one, its open change from `change_of_branch` as `#<n>`; where the local branch holds commits that the branch at `origin` does not (or `origin` has none of that name), how many: `129  issue-129  #178  1 not pushed`. It says what is, never what to run. No title is printed in the list. A branch with no issue's file on it yet is under `Not planned`.
- **`wb status <issue>`** prints that issue whatever its state, with its title (the first line of `issue_address`, escaped) last on its line, then, where an `[answers]` first line names a role (#226), `waits for: <role> (<question>)`, for the newest such answer, as `waits for: tester (54.build.q3)`, unless that role has a stamp on the issue newer than the answer (it has acted since; `person` has no stamp), and `waits for: person (…)` where a person is to act. It is a fact about the issue, not a command, and it is read from the comments `wb status` reads already: nothing more of the tracker. An answer of 0.10.0, `[answers] 54.build q3`, closes its question and names no role. Under it `Worked out from:` and the lines it was worked out from: its stamps, questions and answers, and the branch and files it found. An issue that is not in the flow has the line `not in the flow: ...` first.
- **A job that is done** (#282) is not in the list, not even as a stage. `wb status <issue>` prints `Done:` in place of its step, and what closed it: `merged: into <default branch>`, or the acceptance of the work. It also prints one line for each acceptance that holds, by its object: `accepted: the issue's file <12 hex digits> (<who>, <time>)` and `accepted: the work <12 hex digits> (<who>, <time>; not run: <roles>)`. Done by a merge is read from git alone: a branch of the issue is there, none holds a commit that the default branch lacks (a merge, a squash, or an upstream gone), one holds a commit after the acceptance that changes a path other than the issue's own files, and the default branch holds the issue's file with an acceptance that holds for it. With no branch of the issue left, or with none that holds work, git does not say the change was its, and the job is not done by a merge. A record of the acceptance of the work that is in the default branch beside the file and its acceptance makes the job done, with no branch needed. There is no stage word `Done`: `--stage` knows none.
- **`wb status --stage <heading>`** prints only that heading, for example `wb status --stage "not planned"`. The names are matched without regard to case; `Questions open` and `Not read` too. A name that is not known is used wrongly.
- **What it reads:** the branches with git, the issue's file and its acceptance from the branch with `git show`, never by switching branches, and the comments through `issue_comments` and the change through `change_of_branch`, each once for each issue listed (`issue_address` for one issue). It writes nothing, and asks the network only through those patterns.
- **The exit status:** 0 when every issue could be read; 1 when a pattern failed for an issue (it is listed last, under `Not read`, with why; the others are printed); 2 when it was used wrongly, Watchbill is not set up, or no default branch is found.
- **It is not a dashboard** ([Not goals](#not-goals)): a list in the terminal, of now, for one repository. The sessions that run on this machine are for #140 (planned for 0.12.0).

## Answering the questions

A session asks what only a person can settle on the issue, in one comment for the session (#145). `wb answer <issue>` shows each open question in full in the terminal and posts the answers in the form the next session and `wb status` read. The questions and the answers stay on the issue, where the team, the next session and an assessor read them.

A question takes one line on the issue, and the rest is folded under `<details>`, which GitHub and GitLab show closed:

```text
[questions] 54.tester q1 q2
q1: Is a clause that no test may reach left without a test?
<details>

Why it matters: nothing fails if the clause breaks.

1. (tester) Let the tester read the two files of tests.
2. (person) Change the issue's file and accept it again.

Suggested: 1, because the clause needs a check.

</details>

q2: …
```

```sh
wb answer 54                    # each open question, one at a time
wb answer 54 --show             # read every open question in full, answer none
wb answer 54 --suggested        # every suggestion
wb answer 54 tester.q1 2        # one answer, without asking
wb answer 54 tester.q1 2 q3 1   # several answers, in one call
wb answer 54 --suggested q3 2   # every suggestion but the question named
wb answer 54 q3 "only the first command"   # a question of the planner
wb answer --change 174          # a change with no issue
```

A choice starts with the role that acts on it, in brackets (#226): `tester`, `build`, `review`, `fix`, `planner`, or `person` for a person's own step (any other word in brackets is no role, and the person is asked), such as changing the issue's file and accepting it again. A choice that changes the issue's file is `(person)`. So the answer says whose move is next, and `wb status <issue>` says it too ([Where each issue stands](#where-each-issue-stands)).

- **What it shows:** each open question of the issue, in the order asked, with its name, what is unclear, why it matters, the choices and the suggestion, laid out plainly. A question is open until an answer names it, as `wb status` reads it, and a question a `[revert]` marker took back is not shown. The questions of revision 0.0.8, one `[question] <name>` comment each, are shown and answered the same way.
- **What it asks:** Enter takes the suggestion. A number takes that choice; a number that is not one of the choices is asked again. Any other text is the answer in words. A choice takes the role it names. For words, or a choice with no role, it asks which role acts on the question, and Enter takes the role that asked; a word that is no role is asked again.
- **What it posts:** first it says on standard error what it will post, then one comment per role through `issue_comment`: `[answers] 54.tester q1>tester q2>person`, the role that acts after each question, then `q1: 2` and `q2: <words>`, a line each. A question of 0.10.0 or before, whose choices name no role, is answered as before: `[answers] 54.tester q1 q2`, no role asked and none written. Nothing is posted until every question shown has its answer: an input that ends first, or Ctrl-C, posts nothing.
- **`--show`** (#225) prints each open question in full, as it is laid out when `wb answer` asks, the questions apart by one empty line, to standard output, so it can be piped to a pager. It asks nothing, posts nothing and writes nothing, and ends with 0; with no question open it says so, as `wb answer` does. `wb answer --change <n> --show` does the same for a change. With `--suggested`, or with a question and its answer, it ends with 2.
- **`--suggested`** takes every suggestion and names the questions that have none, which stay open. A suggestion is posted written out, as `q1: 1`.
- **`<question> <answer>`** answers one question without asking: `tester.q1`, or `q1` for the planner, whose questions are `<issue>.q<n>`. The answer is read as at the terminal, and a number that is not a choice is refused with 1.
- **Several answers** (#181): the pairs `<question> <answer>` repeat, and `--suggested` goes with them: the suggestion is taken for every open question the pairs do not answer. One `[answers]` comment is posted per role, in the order the questions are named (with `--suggested`, in the order asked). It is all or nothing: if any pair names a question that is not open, or a number that is not a choice, nothing is posted and it ends with 1. A question named twice, by one word or by two (`q1` and `planner.q1`), is refused with 2.
- **The role in a pair or with `--suggested`:** `<question> <answer>` and `--suggested` ask nothing: the answer takes the role of its choice, and the role that asked where the choice has none or the answer is in words.
- **The issue's file.** Where the answer's role is `person`, `wb answer` says after posting that the file is to be changed and accepted again (`wb plan accept` for a plan, else `wb accept`) before the role is started again. An answer to a role is not said so. For a question of the planner the message is that of the planner: start `wb start planner <issue>` again to fold the answer into the file, and accept the file with `wb plan accept <issue>` (`wb accept <issue>` where the issue is no plan); for a change with no issue, that its description is changed. It never changes the file. A question of 0.10.0 or before, whose choices name no role, says it with the line `changes the issue's file: yes`, as it did.
- **A change with no issue:** `wb answer --change <n>` reads `change_comment_bodies` and posts through `change_comment`.
- **What it reads:** the whole comments, through `issue_comment_bodies` of `watchbill.yml` ([The patterns](#the-patterns)). Without it, or without `issue_comment`, it is refused with 1, naming the line, and posts nothing.
- **The exit status:** 0 when the answers are posted, or no question is open, which it says; 1 when it was refused, the input ended before every question was answered, or a pattern failed; 2 when it was used wrongly or Watchbill is not set up.

## Taking a step back

`wb revert <issue> <role>` takes an issue back to before a step, for example to run review round 1 again. It deletes nothing at the tracker: it posts one marker, and from then on `wb` reads the issue as if that step and every step after it had not been done. Deleting comments would rewrite the history that an assessor reads, and the questions and answers in it are what a person decided.

- **The marker** is one comment whose first line is `[revert] <role> round <n>`. `wb revert` finds the last `[stamp] <role> round <n>` on the issue, says on standard error what it will post, and posts it through `issue_comment` of `watchbill.yml`. `--round <n>` names an earlier round of that role; it is a whole number from 1.
- **What it means:** the stamp it names, and every `[stamp]`, `[questions]`, `[answers]`, `[question]` and `[answer]` after it up to the marker, no longer count, whatever their role. Reverting review round 1 after fix round 1 takes the fix back too, since the fix answered that review. An `[answer]` after that stamp is taken back too, also one that answers a question asked before it: that question is then open again, as the owner decided (#142). A stamp before it counts as before, so the next review after `wb revert 174 review` is round 1 again if it was the first.
- **Who reads it:** the round that `wb start <role>` and `wb prompt` work out ([the round and the change](#the-round-and-the-change)), the check that a fix follows a review, the step and the open questions of `wb status`, and the questions `wb answer` shows. A marker is read only from the first line of a comment, as a stamp is, so a marker quoted in a comment's text counts for nothing, and so does one with no such stamp before it, and so does a `[revert] planner` posted by hand. `wb status <issue>` marks the lines a marker took back in what it was worked out from.
- **A change with no issue:** `wb revert --change <n> <role>` posts the marker on the change, through `change_comment`, and reads the stamps from `change_comments`.
- **What it does not touch:** the branch, the issue's file and its acceptance, and the change. The commits of the steps taken back stay on the branch, and taking them back is git's; `wb revert` says so on standard error.
- **A later planner:** when a `[stamp] planner` stands after the stamp to be reverted, that stamp no longer counts, since the new planner took the issue back ([where each issue stands](#where-each-issue-stands)). `wb revert` is then refused with 1, saying so, and posts nothing.
- **The planner** is not taken back by a marker. The issue's file and its acceptance are files of the repository, and git takes them back. `wb revert <issue> planner` is refused with 1, and says so.
- **Refused with 1:** a role with no stamp on the issue that still counts (naming the role), a missing `issue_comments` or `issue_comment` (or the change's), a pattern that ends with anything but 0.
- **The exit status:** 0 when the marker is posted; 1 when it was refused or the posting failed; 2 when it was used wrongly (no issue or role, a role the revision does not have, an option that is not known, a `--change` that is not a plain name, `--round` that is not a whole number from 1, an issue and `--change` together) or Watchbill is not set up.
- **Not yet:** who may revert once the roles have accounts (#83, planned for 0.12.0). Until then any person who can post a comment can.

## Taking it on one role at a time

A team takes Watchbill on one role at a time (#116). A person can start any role alone, with no step before it: no planner, no issue's file, and for a review or a fix, no issue.

The first step is reviews only. A person writes a change as before, checks out its branch, and has it reviewed by a fresh session:

```sh
git switch my-change            # the change's own branch
wb start review --change 174          # posts the stamp on the change and starts the review
```

- The change's description is the spec. The review reads it at `change_address`, and says so in a finding when it is empty.
- `requirements` is "what the description of change 174 says the change does", unless `--requirements` is given.
- The stamp's `issue:` is empty. `wb` posts it on the change by `change_comment`, with the first line `[stamp] review round 1`, and the round after it from the stamps on the change.
- The branch is the one checked out, unless it is the one `origin/HEAD` names. Otherwise give `--branch`.
- With no issue's file, the session may change paths that are not known, so the review starts only with `classed: []` in `watchbill.yml` ([Classed code](#classed-code)).
- For a runner `wb` cannot start, `wb prompt review --change 174 --stamp` prints the stamp to post by hand, and `wb prompt review --change 174` the prompt.

Then the team adds the rest when it is ready:

- **The fix,** in the same way: `wb start fix --change 174`.
- **Issues with no file:** the tester, the build, the review and the fix are handed out for an issue, and work from its own numbered items.
- **The planner and the acceptance:** once an issue has a file, the chain's gates bind. The file must be accepted, and the file and its acceptance committed, before any role of the issue is handed out.
- **The hand-on** from one role to the next always needs the whole chain: an accepted file for every issue. It is not built yet: `wb handover` (#88, planned for 0.12.0), the listener (#85, 0.12.0) and the officer (#86, 0.12.0).

## The issue's file and its acceptance

An issue has one file, `.watchbill/issues/<issue>.yml`, which the planner writes or a person does ([docs/revision.md](docs/revision.md), point 13). It is the project's file, not the revision's: `wb check` does not fail on it. `wb` reads it through the one loader, so values are plain text, and holds it to this form:

```yaml
issue: '54'
from: <SHA-256 of the issue's text the planner read>
requirements:
  - id: '54.1'
    text: <one requirement>
    source: <where in the issue it comes from>
    guard: <name of an existing test, run as it is>   # only for a clause that already holds
interface: |
  <the names, types and signatures the tests call>
changes: [<paths the change may change>]
reads:
  tester: [<paths>]
  build: [<paths>]
  review: [<paths>]
  fix: [<paths>]
existing_tests_changed: [<test names>]
questions: [<what the issue leaves open>]
```

- Every key is there, in this order. A list may be empty. A key that is not on the list is refused. A requirement holds `id`, `text`, `source` and, if it has one, `guard`, in that order.
- `issue` is the name given to `wb accept`. An ID is unique in the file. A path is relative to the root of the repository, has no `..`, and names nothing under `.watchbill/`.

`wb accept <issue>` accepts the file. It reads it, names the first fault of form, and refuses while `questions` holds anything, naming them. It checks the lists ([below](#the-lists-checked-at-acceptance)). Then it says what it will write, writes `.watchbill/issues/<issue>.accepted`, and commits it:

```
$ wb accept 54
wb accept: will write .watchbill/issues/54.accepted:
sha256: 5d41402abc4b2a76b9719d911017c592ae0a1b3c1e2f8d3b0a7c9e4f6d2b8a11
by: Ada Lovelace <ada@example.org>
time: 2026-11-18T09:30:00Z

wb accept: .watchbill/issues/54.yml is accepted.
wb accept: will commit .watchbill/issues/54.accepted alone, with the message "Accept the issue's file (54)", as Ada Lovelace <ada@example.org>.
wb accept: committed .watchbill/issues/54.accepted alone (3f9a1c2). What was staged before stays staged.
wb accept: wb pushes nothing. Push the branch issue-54 so that the history elsewhere holds the acceptance.
```

The commit is made by `wb accept`, alone, so the history holds the acceptance before any work it allows. Only that one file is committed: whatever else is staged stays staged. It is made as the `user.name` and `user.email` of git, on the issue's branch. Before it writes anything, `wb accept` refuses with 1 when the checkout is not on that branch (the pattern `branch` of `watchbill.yml`, naming both), or when the issue's file is not committed in `HEAD` or differs from it: the acceptance holds for the file as the history has it. When the commit fails (a hook, a signing key), the acceptance stays written and is unstaged again, `wb accept` says so and ends with 1, and the person commits it with the command it prints. A commit that holds more than the acceptance (a hook of git added a file) is refused with 1 and named. `wb` pushes nothing. `wb accept --no-commit` keeps the form of 0.0.9: it writes the acceptance and prints the command, and makes none of those refusals.

`by` is the `user.name` and `user.email` of git, and `time` is UTC. Without both, `wb accept` refuses. An acceptance that is there is never written over: remove it first. `wb accept` writes nothing but the acceptance, commits nothing but that file, and asks nothing of the network; `needs` is the team's own command.

An issue's file counts as accepted only while the SHA-256 in its acceptance is that of the file as it is. The function `watchbill::issue::is_accepted` answers that. One byte changed, and it is not accepted: it is accepted again by removing the `.accepted` and running `wb accept`.

What acceptance cannot stop: the hash stops a change after the acceptance, not a false acceptance. Where a person and the agents share one account, an agent can run `wb accept` itself, or write the `sha256:` line by hand to match a file it changed ([docs/revision.md](docs/revision.md), point 13). Only accounts of their own for the agents stop that, with the person's `[accepted]` comment on the issue (point 13 and point 14).

`wb accept` never follows a link: `.watchbill` or `.watchbill/issues` that is a link, or is not a folder, is refused by name, and nothing is read or written through it. The acceptance is written whole or not at all, to a file of its own in the same folder and then linked to its name. An issue is named by at most 64 letters, digits, `.`, `_` and `-`. A path of the file that names the root (`.`, `./`) is refused. What `wb accept` prints of the file is escaped in the same way: control characters, line and paragraph separators, bidi controls and the backslash (#167). A git name or email that holds `<` or `>` is refused.

### The lists checked at acceptance

A session builds and runs the tests from the files of its lists. So the lists must hold every file that their files need to build: every header, direct or through another header, and the helpers of the tests. And no build script is among the files a change may change. `wb accept` checks both before it writes, from two lines of `watchbill.yml` (#125):

```yaml
build_files: [Makefile, .gitlab-ci.yml, cmake]
needs: 'sh tools/needs {file}'
```

- **`build_files`** is a list of the files and folders the project counts as its build. A path of `changes` that is one of them, lies under one, or is a folder that holds one is named, and the file is refused with 1. A build script is changed by a person, not by a role.
- **`needs`** is the team's own command, for its own language. Filled with `{file}`, it prints, one to a line, the paths from the root of the repository that the file needs to build. `wb accept` runs it with `/bin/sh -c`, at the root of the checkout, once for each path of `changes` and of the tester's and the build's `reads`. `{file}` is filled with the path in single quotes, as one word of the shell, so it is written in the pattern without quotes. A path it prints must be in `changes` or in the `reads` of each of the two roles that sees the file, or lie under a folder that is. One that is not is named, with the role and the file, and the file is refused with 1. `./` and `..` in a printed path are worked out from the root, as `cc -MM` prints `src/../include/regs.h`. A command that ends with anything but 0 refuses the file, naming the file it was run for. Its standard error is the person's.
- **Without `needs`**, the lists are not checked, and `wb accept` says so. `build_files` is checked all the same. The lists of the review and the fix are not checked: they read the diff, which the view holds.

For C, `tools/needs` can be built on `cc -MM`, which prints what a file includes, but not the system's headers:

```sh
#!/bin/sh
# Prints, one to a line, the files of the repository that the file $1 needs to build.
# A file that is not C, or is not there yet, needs nothing that can be read.
case "$1" in
  *.c|*.h) ;;
  *) exit 0 ;;
esac
[ -f "$1" ] || exit 0
found=$(cc -MM -Iinclude "$1") || exit 1
printf '%s\n' "$found" | sed -e 's/^[^:]*://' -e 's/\\$//' | tr -s ' ' '\n' | sed '/^$/d'
```

A test is linked from what the view holds. A function that the code hides, and that the tests need, is a stub: a file of its own, listed in the tester's and the build's `reads`, and the planner lists it there. The planner runs `needs` itself before it commits the issue's file, so a list that falls short is found before a person reads it.

The exit status:

- **0**: the file is accepted.
- **1**: it is refused: no file, a fault of form, a question, an acceptance that is there, no git identity, a path of `changes` under `build_files`, a list that leaves out what a file needs to build, `needs` that ends with anything but 0, or a `watchbill.yml` the loader refuses.
- **2**: `wb accept` was used wrongly (no issue, a word that is not known, a name that is not an issue), the folder is not in a git repository, or Watchbill is not set up.

## Installing it

`wb` is one program, built from Rust and a few crates that the distribution packages ([docs/revision.md](docs/revision.md), point 6). The owner decided this on 2026-11-17 (#20, #34). Building it on Enterprise Linux 9 needs EPEL, where the crates are. Installed, it needs `git` and no library of its own.

On Fedora and Enterprise Linux it is an RPM package, installed and removed with `dnf` like any other program:

```sh
packaging/rpm.sh                                                  # builds the package from this checkout
sudo dnf install build/rpm/RPMS/$(uname -m)/watchbill-[0-9]*.rpm      # puts wb in /usr/bin and the revision in /usr/share/watchbill
```

The package also installs a bash completion of `wb` and `watchbill` (`/usr/share/bash-completion/completions/wb`): with `bash-completion` installed, Tab completes the command, the role, the issue and the flags, and it runs no `wb`. zsh can load the file with `autoload -U +X bashcompinit && bashcompinit`, then `source /usr/share/bash-completion/completions/wb`.

It also installs a man page of section 1 for `wb` and one for each command: `man wb` lists them, and `man wb-start` says what `wb help start` says, in the form of a page. The pages are written in roff in [packaging/man/](packaging/man/); nothing is fetched to make them.

The package is built here from a checkout, and dated by the last commit of it: two builds of one commit into the same result folder (the argument of `rpm.sh`, `build/rpm` by default) give the same package. Another folder gives another `wb`, as the path of the build is in it. `rpm.sh` needs a git checkout and stops with 2 outside one. There is no repository to `dnf install watchbill` from yet; the RPMs of each version are on its release (below). Building needs `rpm-build`, `git-core`, and the Rust of the distribution: `cargo`, `rust` (1.97 or later), `cargo-rpm-macros` and the crates `yaml-rust2`, `sha2` and `lexopt` (`rust-yaml-rust2-devel`, `rust-sha2-devel`, `rust-lexopt-devel`). In a container image of Fedora, which leaves out the documents of a package, install the packages with `dnf install --setopt=tsflags= …`: a crate includes its `README.md`. On Enterprise Linux 9 it also needs **EPEL**: `cargo-rpm-macros` and the crates are only there.

```sh
sudo dnf install epel-release                    # Enterprise Linux 9 only
```

With only BaseOS and AppStream the build stops before it compiles anything, and says what is missing:

```
error: Failed build dependencies:
	cargo-rpm-macros is needed by watchbill-0.0.1-1.el9.x86_64
	crate(lexopt) >= 0.3 is needed by watchbill-0.0.1-1.el9.x86_64
	crate(sha2) >= 0.10.9 is needed by watchbill-0.0.1-1.el9.x86_64
	crate(yaml-rust2) >= 0.10.4 is needed by watchbill-0.0.1-1.el9.x86_64
```

The build runs the tests of `wb`. The package is built for the architecture of the machine, and it installs the program as `wb` and `watchbill`, and the files of the revision under `/usr/share/watchbill/`. That folder is fixed in the program when it is built. A build asks nothing of the network: the macros of the distribution point cargo at `/usr/share/cargo/registry`, where the `rust-<name>-devel` packages put the crates, and never at crates.io. The `.cargo/config.toml` in the tree does the same for `cargo test` run by hand in a checkout.

A pushed tag of a version, such as `0.0.7`, builds the RPMs on Fedora and on Enterprise Linux 9 (`.github/workflows/release.yml`). When both builds pass, the release of that tag holds the two binary RPMs, the source RPM and `SHA256SUMS`. Download them from the release of the tag, `https://github.com/<owner>/watchbill/releases/tag/<version>`, and check the ones you downloaded with:

```sh
sha256sum -c --ignore-missing SHA256SUMS
```

To try it without installing anything, from a checkout:

```sh
cargo test                                       # every test of `wb`: needs cargo, rust 1.97 or later
cargo run -- init                                # runs wb from the checkout; the revision is read from share/watchbill/
```

The folder of the revision is fixed in the program when it is built, as the path of the checkout. After a checkout is moved, run `cargo clean` before building again.

## The revision it carries

`wb revision` prints the version of the package, the hash of its revision and the manifest. It writes nothing.

```
$ wb revision
watchbill 0.0.1
revision <the SHA-256 of the manifest>

<the SHA-256 of each file>  revision/models.yml
...
```

The manifest has one line per file of the revision, in the form `sha256sum` prints, sorted by path. The package holds it as `revision.sha256`, beside the folder `revision/`. Anyone can check the files without `wb`. In a checkout:

```sh
cd share/watchbill && sha256sum -c revision.sha256
```

In an installed package, in the folder that holds `revision/`:

```sh
cd /usr/share/watchbill && sha256sum -c revision.sha256
```

Every line ends in `OK` when the files are the revision. `revisions.txt` beside it lists each released version with the hash of its revision. What is in a revision: [docs/revision.md](docs/revision.md).

## Comparing two revisions

`wb compare` says what changed in what the agents are told between two revisions: in the role files, the rule table and the model table. A revision is named by its version or by its whole hash. `wb` holds two: the installed one, and with `--from <path>` the one the repository at `<path>` is pinned to, read from its `.watchbill/revision/` (a link is never followed, and the files must give the hash of the line `revision:` of `watchbill.yml`). Reading an older revision from the history of a checkout of Watchbill is not built.

```
$ wb compare --from ../brake-controller 0.10.0 0.11.1
changed: revision/roles/build.md

--- a/revision/roles/build.md
+++ b/revision/roles/build.md
@@ -95,3 +95,4 @@
 ...
+A new last line.
```

The output is fixed. Nothing when the revisions are the same; else one line for each file, sorted by path, `added:`, `removed:` or `changed:`, then an empty line and the unified diff of each changed file, `a` being the first revision. The diff is made by `git diff --no-index` with fixed options and none of the person's git configuration. It writes nothing and asks nothing of the network. The exit status: 0 the same, 1 they differ or the files of a revision are refused (written as `wb compare: limit: …` on standard error); 2 a revision is not found, a repository is not set up, or it was used wrongly.

## The name

A watch bill is a ship's posted list of who stands which watch at which station.

## The revision

What a revision is, and each decision behind it, is in [docs/revision.md](docs/revision.md).

## Licence

Watchbill is licensed under the Apache License, Version 2.0. The text is in [LICENSE](LICENSE) and the holder of the copyright is named in [NOTICE](NOTICE).

## Not decided

- Which standard comes first, and which tools the first adapters are for.
