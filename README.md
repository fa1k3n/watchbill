<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/logo-dark.svg">
    <img src="docs/logo-light.svg" width="96" height="96" alt="The mark of Watchbill: a grid of three by three rounded squares, the diagonal filled in another colour">
  </picture>
</p>

# Watchbill

A harness for AI agents in embedded software development, deployed at a fixed revision onto a project that already exists.

**Status: 0.10.0. Built: `wb init`, `wb check`, `wb upgrade`, `wb prompt`, `wb accept`, `wb status`, `wb view`, `wb answer`, `wb revert`, `wb revision` and `wb start`, which starts a session of a role at a person's terminal. For an issue with an accepted file the session runs in a view: only the files its work needs, and none of the history. A person starts each role. Nothing hands work from one role to the next yet. A command that is planned is marked with its release.**

## What Watchbill is for

Watchbill is for teams that build software under a standard, such as IEC 62304 for medical devices or ISO 26262 in automotive, or under rules about which source code may leave the premises.

AI agents give such a team three problems:

- **Roles.** The standard, or the team's process under it, says who may specify, implement, verify and release. One agent session does all four unless something stops it.
- **Confidentiality.** A hosted model sees every file its agent opens. Some code may not leave the premises.
- **Assessment.** An assessor asks what the agents are told to do, and what stops them from doing anything else. A prompt typed into a terminal answers neither.

Watchbill sits between the team's tools and its agents, and is built to answer each:

- **Roles.** The work is split into roles: breakdown, tests, build, review and fix. Each role has a fixed, versioned prompt. Where a limit of a role can be checked, the program checks it and refuses. No agent checks its own work, and no agent merges.
- **Confidentiality.** A session works in a view: a folder that holds only the files its work needs, and none of the history. The organisation says which tools and models may have a view of which code. Code that must stay on the premises is worked on there, by a model that runs there. One role, the breakdown, reads the whole code to make the lists of files, and runs only where the organisation has cleared it. A view is not a barrier against sending: the files in it go to the model of its runner ([What still leaks](#what-still-leaks)).
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

At 0.11.0. An issue that changes a verdict changes its row.

| Goal | Today | What will hold it |
|---|---|---|
| 1. Fits the workflow | Partly: any tracker and git server with a command line, by a pattern the team writes. `wb init` writes the patterns for GitHub and GitLab only. The CI and the requirements tool are names and nothing more. `wb` installs on Fedora and Enterprise Linux only | |
| 2. What the agents are told | Held for what `wb` hands out: the pinned revision, `wb check`. `wb start <role>` posts the stamp and starts the session with that prompt itself. A session started by hand is the word of the one who started it, and so is the effort of a runner whose `start` cannot set it | |
| 3. Rules enforced | Partly: the pinned revision, the acceptance, the breakdown's runner and the placeholders are checked in code. What a session of an issue with an accepted file reads and changes is held by its view. No other limit of the rule table is | #28 names each limit and what holds it; then #88 and the pieces #28 writes |
| 4. People keep the decisions | Partly: `wb accept` asks for a person's word, but cannot tell a person from an agent under one account. That an agent cannot merge rests on the git server's settings | The roles' accounts, #83, and the acceptance read at the tracker, #88 (both planned for 0.12.0). No issue yet checks that a role's account cannot merge |
| 5. Duties separated | Partly: the rule table says it, and nothing knows who holds which role | #83 and #88, for work that is handed on (both planned for 0.12.0). No issue yet for a session a person starts |
| 6. Sees only the code it needs | Partly: a session that `wb start <role>` starts for an issue with an accepted file runs in a view, in bubblewrap. It sees the paths of its lists and no history, and its commits come back only where `changes` allows ([The view](#the-view)). `wb accept` checks that the lists hold what each file needs to build, by the team's own command `needs` ([below](#the-lists-checked-at-acceptance)). A session started by hand, for an issue with no file, or as the breakdown sees the whole checkout. What still leaks is [said below](#what-still-leaks) | No issue yet for a session started by hand |
| 7. Code goes only where allowed | Partly: checked for the breakdown | The other roles, #11. No issue yet for a local model for them |
| 8. Starts small | Held: `wb start <role>` starts any role alone, for an issue with no file, and a review or a fix of a change with no issue, and posts the stamp on the issue or the change ([one role at a time](#taking-it-on-one-role-at-a-time)) | |
| 9. Starts strict, opens up | Not yet: one level for the whole repository, and no level given means the strictest | #11, #65, and a level for each path, which has no issue yet |
| 10. Shown to an assessor | Partly: what the agents are told can be read. The rule table says more than the program enforces | #28 |
| 11. Needs no other product | Held: one RPM, `git`, and a build with the network off | |
| 12. Work is handed on by the program | Not yet: a person starts each role. `wb status` says where each issue stands, and nothing more | `wb handover` (#88), the roles' accounts (#83), `wb listen` (#85), the officer (#86) and `wb run` (#183) for 0.12.0 |

### Not goals

- Reports. `wb` keeps its state in git (decided, not built: [point 15](docs/revision.md#15-identity-history-and-trust)), and a report of it is for the team's own tools or a recorder, such as [Handover Ledger](https://github.com/fa1k3n/handover-ledger).
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
| work | `accept`, `start`, `run` (planned for 0.12.0, #183) |
| see | `status`, `view` ([Seeing a view first](#seeing-a-view-first)), `log` (planned for 0.12.0, #177), `answer` |
| take back | `revert`, `clean` (planned for 0.12.0, #176) |
| plumbing | `prompt`, `revision`, `compare` (planned for 0.12.0, #179) |

After 0.0.9 the releases are 0.10.0, 0.11.0, 0.12.0 and 1.0.0. A man page for each command is planned for 0.11.0.

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

Enter keeps what stands in brackets. `wb init --yes` takes all of it and asks nothing; it shows the files before it writes them. Input that ends before a yes, and Ctrl-C, write nothing and end with 1. The result is the folder `.watchbill/` at the root of the repository, to be kept under version control with the code:

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
  change  .watchbill/revision/roles/breakdown.md
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
- **Nothing to do:** a repository at the installed revision, its files, its manifest, its two lines and its patterns, ends with 0 and writes nothing. That is so right after an upgrade too, before the commit. The same version with another hash is upgraded.

The exit status:

- **0**: it wrote and the check holds, `--dry-run` said what it would write, or there was nothing to do.
- **1**: it was refused and wrote nothing, or it wrote and the check does not hold. The differences are said, and what was written stays.
- **2**: `wb upgrade` was used wrongly, the folder is not in a git repository, or `.watchbill/watchbill.yml` is not there; `wb init` sets Watchbill up.

## The prompt of an assignment

`wb prompt <role>` prints the prompt of a role on standard output, and nothing else. Whatever starts an agent's session, in any language, calls it as a process. Run it anywhere inside the repository. It writes nothing. `wb start <role>` ([below](#starting-a-session)) does all that `wb prompt` does, then posts the stamp and starts the session.

The prompt is the role's text of the pinned revision, then the project's own text (`.watchbill/project/all.md` and `.watchbill/project/<role>.md`, where they are). A call asks only for what `wb` cannot know on the machine: the issue, and for the review and the fix the change and the round. The change and the round live at the tracker and the git server. Where `watchbill.yml` has the patterns for them, `wb prompt` works them out by running those (#84); otherwise they are given.

```sh
wb prompt tester 54
wb prompt build 54
wb prompt review 54                                 # the change and the round worked out
wb prompt fix 54 --change 168 --round 1             # given, as a repository with no patterns needs
wb prompt review --change 174                       # a change with no issue
```

The issue is the word after the role, or `--issue`; both together end with 2. The tester and the build with no issue end with 2 and the usage. Only the review and the fix take a change alone, with no issue (see [Taking it on one role at a time](#taking-it-on-one-role-at-a-time)). The placeholders are filled from the options, one for each, and what no option gives is derived. An option given always wins over the derived value, and the stamp holds each value as it was handed out:

- `--issue <text>`, `--change <text>`, `--requirements <text>`, `--round <text>`, `--checkout <text>`, `--branch <text>`, `--issue_address <text>`, `--change_address <text>`, `--classed <text>`, `--classed_touched <text>`. What each is, is in `.watchbill/revision/roles/placeholders.md`.
- `checkout` is the root of the repository `wb` runs in.
- `classed` and `classed_touched` are worked out for a prompt that holds them: see [Classed code](#classed-code).
- `issue_address` and `change_address` are the patterns of those names in `.watchbill/watchbill.yml`, with `{issue}` and `{change}` filled in. `wb init` proposes them for a GitHub `origin` (`gh issue view {issue} --repo <repository>; gh issue view {issue} --repo <repository> --comments`, `gh pr view {change} --repo <repository>; gh pr view {change} --repo <repository> --comments`: each shows the text or the description first, then the comments, as `--comments` alone hides the first). For a GitLab `origin` it writes one command each, with `--comments`. For another server, or none, there is no pattern and the option must be given.
- `branch` is the pattern of that name in `watchbill.yml`, proposed as `issue-{issue}`, with `{issue}` filled in. The breakdown makes the branch, so it gets the pattern as it is. For the tester, the build, the review and the fix it is the one local branch (from the refs of the repository) whose name is the pattern, or starts with it and a `-`. None, or more than one, is refused with 1, the names found are said on standard error, and `--branch` must be given.
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
- `--runner <name>` names the runner the prompt is handed to. The breakdown needs it: see [Confidentiality](#confidentiality). For the tester, the build, the review and the fix it is taken without a check in this revision, and the stamp names it. A value that is empty, given twice, or not one plain line is used wrongly.
- `--stamp` prints, in place of the text, the stamp of the assignment: what was handed out ([docs/revision.md](docs/revision.md), point 12). It is lines of `name: value` in this order: `role`, `model`, `effort`, `runner`, `version`, `revision`, `issue`, `change`, `round`, `branch`, `checkout`, `time`, then one line `part: <sha256>  <path>` for each part. `model` and `effort` are those of the model table for the role, and for the breakdown those its runner is cleared with. `runner` is the value of `--runner`, and empty when none is given. `version` and `revision` are those of the pinned revision. `time` is UTC, as `2026-11-17T18:04:05Z`. A value that is empty, or that the prompt of the role does not hold (`--change` for `build`), is written as its name and the colon alone: it was not handed out. The round is the exception: `round:` holds the round that `--round` gives, for every role. Where it is not given, it is worked out only for a role whose text holds `{round}`, as above; for the tester and the build `round:` is then empty, because `wb prompt` asks nothing of the network for them. `wb start <role>` always fills it, and it agrees with the first line it posts (#136). A value of `--issue`, `--change`, `--round`, `--branch` or `--checkout` that holds a control character, a line or paragraph separator, or white space at its start or end is refused by name, as one field must stay one line that a comment can trim. `--stamp` with `--parts`, and either given twice, are used wrongly.

For a GitHub `origin`, `wb init` writes the addresses so that each shows the issue's text or the change's description first, then the comments. Here for `example/brake-controller`:

```yaml
issue_address: 'gh issue view {issue} --repo example/brake-controller; gh issue view {issue} --repo example/brake-controller --comments'
change_address: 'gh pr view {change} --repo example/brake-controller; gh pr view {change} --repo example/brake-controller --comments'
branch: 'issue-{issue}'
```

An address that ends in `--comments` alone shows the comments and hides both the issue's text and the change's description. A repository set up with such addresses keeps its `watchbill.yml`, as `wb` never writes over it: change the two lines by hand.

### An issue with a file

The flow of an issue is: the breakdown reads the issue and writes `.watchbill/issues/<issue>.yml`; a person accepts it with `wb accept <issue>`; then the four roles are handed out: `wb prompt tester <issue>`, `build`, `review` and `fix`. For these four, `wb prompt` looks for the issue's file ([below](#the-issues-file-and-its-acceptance)):

- **No file:** the role is handed out, and works from the issue's own numbered items (#116). A person may start any role alone. The chain's gates bind once a file is there.
- **A file that is accepted and committed:** the file and its acceptance must both be committed in `HEAD` and be the same in the working tree, or `wb prompt` prints nothing on standard output, names the file, says to commit it, and ends with 1. The history then holds what the issue was read as before the work it allows. `{requirements}` is its IDs and texts, one to a line (`- 54.1: <text>`). Two parts follow the role's text, before the fixed line and the project's own text: the interface, and what the role may change (`changes`) and read (its own list of `reads`). The file is a part of the prompt: `--parts` and `--stamp` give its SHA-256 and path (`.watchbill/issues/<issue>.yml`) as the second part, after the role. Text of the file is put in as it is and is never read for placeholders.
- **A file that is there and not accepted, or changed since its acceptance:** nothing is printed on standard output, the reason is on standard error, and the status is 1. No prompt of the issue is handed out until a person accepts the file again. The same holds for a file that does not hold to its form, or holds a question.
- A value of the file that reaches a prompt is refused, by the name of its field and with status 1, before a prompt is handed out, when it holds a control character, a line or paragraph separator (U+2028, U+2029) or a bidirectional control (U+202A to U+202E, among them the right-to-left override, and U+2066 to U+2069). The values are the interface, each requirement ID and text, and the paths. A line break and a tab are allowed in the interface only. The file is read once and is refused when it is over 1 MiB.
- An option `--requirements` given still wins over the file. The interface and the lists are still handed out, and the stamp says so with the line `requirements: given`, before its part lines.
- The text of the fix does not hold `{requirements}`, because the fix answers findings, not requirements. It gets the two parts and no requirement.

`wb prompt` runs the check of `wb check` first. When the repository is not at its revision, it prints nothing on standard output and says why on standard error.

The exit status:

- **0**: the prompt, or with `--stamp` the stamp, is printed.
- **1**: it was refused and nothing is printed: the breakdown's runner is not cleared for it (see [Confidentiality](#confidentiality)), the check does not hold, the clock reads before 1970 (`--stamp`), the role is not one of the revision (the roles it has are named), a project file is not UTF-8, a placeholder is not known or got no value, the branch is not found or not one, the issue's file is there and is not accepted (or changed since), `watchbill.yml` holds `task_address`, `{task}` in a pattern or the key `breakdown`, a value derived for the stamp (the checkout, the branch) is not one plain line, a pattern run to work out the round or the change ends with anything but 0, a fix has no review stamp to answer, or the branch has no open change or more than one.
- **2**: `wb prompt` was used wrongly (no role, a third word or an option that is not known, the issue given as a word and as `--issue`, an option given twice, with no value or with an empty one, `--stamp` with `--parts`, an option given twice, a stamped value that is not one plain line, an issue or change that fills a pattern and is not plain, the breakdown with no `--runner`, the tester or the build with no issue, a review or a fix with no issue whose branch is not given and is not the branch checked out), the folder is not in a git repository, or Watchbill is not set up. The usage or the reason is on standard error.

Each session has its stamp on the issue, one comment for each session, the first line `[stamp] <role> round <n>`, and the lines of the stamp after it. A review or a fix with no issue has its stamp on the change. `wb start <role>` posts it ([below](#starting-a-session)). Where a session is started by hand, the person posts the stamp as it was printed. A session with no stamp on its issue, or on its change where it has no issue, is not taken as the work of a role.

```
[stamp] build round 1
role: build
model: claude-sonnet-5-5
effort: medium
runner:
version: 0.11.0
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

1. **What is in the view?** The paths the issue's file lists for the role. The breakdown writes the lists, `wb accept` checks them, and a person accepts them.
2. **Who may have the view?** The organisation's file says which runner is cleared for which level. In this revision code checks it for the breakdown only; the other roles are #11.
3. **What comes back?** The commits that keep to `changes`. Nothing else is taken.

A view decides how much is seen. The level decides by whom. The network stays open, so every file in a view goes to the model of its runner: a view is not a barrier against sending.

Three sessions run with no view and see the whole checkout: the breakdown, a role started for an issue with no file, and a session started by hand.

Where an assignment may run is the organisation's to say, not the project's ([docs/revision.md](docs/revision.md), point 11). In this revision it is checked for the breakdown, which reads more of the code than any other role.

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
    breakdown:                             # only when the runner is cleared for the breakdown
      up_to: public                        # the highest level of an assignment it may break down
      model: claude-opus-5-5
      effort: high
    start: [claude, --model, '{model}', --effort, '{effort}', '{prompt}']   # how `wb start <role>` starts a session
    binds: ['~/.local/bin/claude', '~/.local/share/claude', {writable: '~/.claude'}, {writable: '~/.claude.json'}]
```

- `levels` is at least one name, each once, from the least to the most restrictive.
- A runner has `tools`, `model`, `agreement` and `up_to`, which is one of the levels. `breakdown` is there only when the runner is cleared for the breakdown: its `up_to` is no higher than the runner's own, its `model` is an exact model ID and its `effort` is one of the model table's.
- `start` is there only when `wb start <role>` may start sessions on the runner ([Starting a session](#starting-a-session)). It is a list of one word or more, each a text of one line. `{model}`, `{effort}` and `{prompt}` are the only placeholders, and the first word, the program, holds none.
- `binds` is there only when the runner needs paths of its own in a view ([The view](#the-view)): its program, its settings, its login. Each is a path from the root, `/…`, or from the home folder of the person who runs `wb`, `~/…`, with no `..`, and not the root or the home folder itself. It is read-only, unless it is written `{writable: <path>}`. A path that is not there is left out. The paths above are where Claude Code keeps itself on one machine; they are an example, not a promise.
- `default` is the name of one of the runners.
- A key that is not known, at any depth, a level written twice, a field missing or a value of the wrong type is refused, with the file and the place named.

The repository states its own level, as the line `confidentiality:` of `.watchbill/watchbill.yml`. Where the line is missing, the level is the most restrictive of the list. A level that is not on the list is refused by name, and so are `levels` and `runners` in `watchbill.yml`: they are the organisation's.

`wb prompt breakdown <issue> --runner <name>` hands out the prompt only when the organisation's file is there, names the runner, the runner has `breakdown`, and the repository's level is no higher than the breakdown's `up_to`. Otherwise it prints nothing on standard output, names on standard error the file it looked for, the runner, or the level and the `up_to`, and ends with 1. With no `--runner` the breakdown ends with 2 and the usage. Its stamp has `runner: <name>` and the `model` and `effort` of the runner's `breakdown`, and `--parts` adds two lines after the parts: `level: <the repository's level>` and `runner: <name>`.

What it does not do yet (#11): the tester, the build, the review and the fix are not refused on a runner or a missing file, and the level of an issue is not read. It cannot stop a person from pasting code into another tool, or running a session with a prompt that `wb` did not hand out. The stamp on the issue is what shows where the work was handed out.

## The view

Three questions settle confidentiality ([above](#confidentiality)). The view answers the first and the third.

A session of the tester, the build, the review or the fix of an issue with an accepted, committed file sees only the files its issue's file lists for it (#126). The program keeps this limit, not the prompt.

- **What it holds.** A new folder, `<state>/watchbill/views/<time>-<role>-<issue>/`, made by `wb` from the paths of `changes` and of the role's `reads`, as they are in `HEAD` of the checkout. It is a new repository with one commit of those files, on a branch of the checkout's name, with no remote and no history of the repository. A path of `changes` that is not there yet is not made: the session may make it. A worktree or a sparse checkout would not do: `git show` reads every file from the history.
- **A local change stops the start** (#172). The view is made from `HEAD`, so a path of it that differs from `HEAD` in the index or in the working tree (changed, staged, deleted; a rename is named by both its paths), or a file marked `assume-unchanged` or `skip-worktree` that is changed, or any untracked file under a path of `changes`, would not be seen by the session, and its commits might not come back over it. `wb` names each such path under its list, `changes` or `reads`, says that this is a guard, and ends with 1, before the stamp is posted and before the view is made. `--ignore-local` passes it when every such path is under `reads` only: the session sees `HEAD`'s file. A change under `changes` is never passed, and the message says so. A start that went on with it has the line `local: ignored <n>` in its stamp, after `view:`, and the file `local` in its session's folder; without it the stamp has no such line. An untracked file under `changes` is named also when it is ignored, by the checkout's rules or by the person's own: the fast-forward at the end writes over an ignored file without a word, so the most restrictive applies. An untracked file under `reads` is not in the view and is not named. More than 20 paths of one list are said as the first 20 and a count. The breakdown and a role with no issue's file run without a view, and are not checked.
- **Where it runs.** In bubblewrap (`/usr/bin/bwrap`, from the package `bubblewrap`): the view at the path of the checkout, writable, so that `{checkout}` of the prompt is right. `/usr`, `/etc` and `/sys` are there read-only, `/tmp` and `/run` are its own, and the home folder is an empty one of its own, with the runner's `binds` in it. The checkout's `.git` and `.watchbill/`, the folder of the organisation's file and the person's other folders are not there. The variables `GIT_*` are not handed in. The network is the machine's, as the runner calls its model over it: the view limits the code that is there to send, not where it can be sent ([What still leaks](#what-still-leaks)). A network of its own is #195, planned for 0.11.0.
- **A terminal.** Where the kernel lets a program type into its terminal (`TIOCSTI`), a session could type into the person's shell after it ends. `wb` gives a session in a view no controlling terminal then. Where `/proc/sys/dev/tty/legacy_tiocsti` is `0`, as on Fedora, the kernel refuses it, and the session keeps its terminal.
- **What comes back.** When the session ends, `wb` takes each commit of the view, in order. A commit that touches a path outside `changes`, under `.watchbill/` or under `build_files` of `watchbill.yml`, a merge or a submodule is refused, with the paths named: nothing of it or after it is taken, and `wb` ends with 1. The commits before it are put on the checkout's branch, with their messages and authors, by a fast-forward. A file entry of `changes` covers that path only, never a path under it; a folder covers the paths under it (#192). `wb` decides it from `HEAD` of the checkout, never from the view, and asks `HEAD` first: an entry that is a folder there is a folder, and one that is a file there is a file, also when it is written with a `/`. For an entry that is not there, the `/` at its end decides: with it a folder, without it a file, so a new folder is written `docs/`. A path with a part `.git` in any case, or one git refuses to check out (the forms NTFS and HFS+ read as `.git`, `.gitmodules` as a link), is refused like a path outside `changes`. Before a commit is put on the branch, `wb` refuses a commit with a tree that is not in the form git writes (a name twice, a mode git rewrites such as `100664`, entries not in git's order), and checks that what the commit it made changes is what the view's commit changed, path for path, mode and file; a commit that differs is refused, and none after it is taken. Work left uncommitted in the view is named and not taken. What the project's git ignores is not work left (#159): `wb` judges it by every `.gitignore` of the checkout's `HEAD` and the checkout's `.git/info/exclude`, read from the checkout and never from the view, so a session cannot widen them, and a `node_modules/` that the project ignores neither keeps the view nor is listed. The person's global ignore file is not read. Where more than 20 paths are left, `wb` says how many, the first 20, and that the whole list is in the file `back` of the session's folder ([The lines it leaves](#the-lines-it-leaves)). A path from the view, and a commit's subject, is said and written escaped (#167: control characters, line and paragraph separators, bidi controls and the backslash), so a file name cannot add a line or send escape codes to the terminal.
- **The push is the person's** (#145). A view has no remote, and the prompts of the tester, the build and the fix say to commit and not to push or open a change. When commits were taken onto the branch, `wb` says on standard error how to look at each commit taken, `git show --stat <commit>`, the command that pushes it, `git push origin <branch>`, and after a build the command that opens the change as a draft: `draft_change` of `watchbill.yml`, with `{branch}` filled ([The patterns](#the-patterns)). Without that line it says so. `wb` runs none of them. A role started without a view is told the push and the draft in its prompt, and `wb` says those two commands after it when the branch checked out has new commits; `git show --stat` is said after a view only. That `wb` pushes itself is #151.
- **What is left.** The view is removed when all of it was taken back. It is kept, and named, when anything was refused or left uncommitted.
- **How it is read.** Once the session has run, the `.git` of the view is the session's. `wb` never runs git with its settings, which would run outside the sandbox a command the session wrote there: it reads the view's objects through a repository of its own, and its `HEAD` from the files.
- **Without a view.** A role started with no issue's file (#116), and the breakdown, which reads the whole code by its role, run as before, in the checkout. For a role with no file, `wb` says so on standard error before it starts. A person who starts it has decided to.
- **The stamp** has the line `view:`, after `time:`: the SHA-256 of the list of paths, one to a line, as the file `view` of the lines holds it, or `none`. `wb prompt --stamp` gives the same line: it says what was handed out, as the model and the effort do.

### Seeing a view first

`wb view` shows what a session of a role will see, before it starts, and says for one file whether it is in the view and why (#173). A list that falls short is found before a session is spent on it.

```sh
wb view 54 build              # the paths of the build's view, its level and its view: line
wb view 54 build --out        # what the view leaves out of HEAD, by folder
wb view 54 src/brake.c        # in or out, for each role, and why
```

- **`wb view <issue> <role>`** prints the paths of the view of that role, one to a line: the list the start makes the view from, `changes` and then the role's `reads`. Then `level:`, the level of the repository as `--parts` of the breakdown gives it (the line `confidentiality:` of `watchbill.yml`, or the most restrictive of the organisation's file), and `view:`, the line the role's stamp would hold, made by the same function. Where a file of the view differs from `HEAD`, or an untracked file is under `changes`, an empty line follows and what the start would say of the local changes (#172). It ends with 0.
- **`--in`** is that, and the default. **`--out`** prints in its place what the view leaves out of `HEAD`, one folder to a line, `<count><tab><folder>/`: a folder that holds no file of the view, at its top, with the count of its files; for a folder that holds some, the files left out directly in it. The files at the root are under `./`. `--in` and `--out` together end with 2.
- **`wb view <issue> <path>`** takes a path from the root of the repository, as the issue's file writes it. A word that is a role is the role: a folder named `build` is written `./build`. It prints one line for each role with a view, the tester, the build, the review and the fix: `build: in: reads.build, line 9 of .watchbill/issues/54.yml: include`, with the list and the line of the issue's file that hold the path and the entry as written; or `review: out: <why>`. The reasons are, in this order: under `.watchbill/`; a path git does not take (#192); listed, and not in `HEAD`, with the list and the line (a session may make a path of `changes`); a submodule; ignored by the checkout, with the rule, and not in `HEAD`; not in `HEAD`; under `build_files` and not listed; not listed. A folder that is not listed, and holds paths that are, is out, and they are named. It ends with 0 when the path is in the view of at least one role, and with 1 when in none.
- **Ignored** is asked of git in the checkout: the `.gitignore` files of the working tree and `.git/info/exclude`. The person's global file is not read.
- **Without a view.** An issue with no file, or one not accepted or not committed, ends with 1: with no file, `wb view` says that the role would run without a view; otherwise it gives the reason `wb prompt` gives. `wb view <issue> breakdown` ends with 1: the breakdown runs without a view.
- **It only reads.** It makes no view, runs no pattern of `watchbill.yml`, posts nothing, writes nothing, not in the state folder either, and asks nothing of the network. `watchbill.yml` is read as `HEAD` has it (#186): where it differs, `wb view` ends with 1. A word that is not known, or a path that starts with `/`, holds `..` or names the root, ends with 2.

### What still leaks

A view keeps the rest of the code and the history away from a session. It does not keep everything:

- **The network is the machine's.** A session in a view shares the network of the machine, so it can reach every service that listens on this machine: `localhost`, a database, a local model server, `howl serve`, and anything else with no login. It can reach the rest of the network too. It is not held to its runner's model. Until the sandbox has a network of its own (#195, planned for 0.11.0), a team runs no service without a login on a machine while sessions run on it.
- **Every file of the lists goes to the model**, as the session reads it. The level of the repository still decides which runner may have them. A view decides how much is seen, not where it is sent.
- **A test that passes or fails tells one bit** about code the session cannot read, at each try. A session that runs the tests many times learns more.
- **The runner's own settings and login are inside the view**, as its `binds`. What the runner keeps there, the session can read.
- **Nothing stops a person** from pasting code into another tool.
- **A stamp is a comment.** Anyone who can comment on the issue can write one, until each role has an account of its own (#83, planned for 0.12.0).
- **A session with no view** writes the checkout itself, so it can change `.watchbill/watchbill.yml`. `wb` runs the patterns of the file as the person, so it runs them only while the file is as `HEAD` has it ([The patterns](#the-patterns)). That stops an uncommitted change only. The same session runs as the person, with a shell: it can commit the changed file, move a ref or write a hook (`.git/hooks/` runs when `wb accept` commits), and a commit in the checkout lifts the limit. A person reads `git log -p -- .watchbill/watchbill.yml` before the next command. What a person commits there is run. A sandbox for every session is #202, planned for 0.11.0.

## Starting a session

`wb start <role>` starts a session of a role in the person's terminal (#84). One command checks what `wb prompt` checks, posts the stamp, and starts the session with the model and the effort of the stamp. A person writes no stamp, no shell and no model name.

```sh
wb start breakdown 54
wb start tester 54
wb start build 54
wb start review 54
wb start fix 54
wb start review --change 174        # a change with no issue
```

Each takes the words `wb prompt` takes for its role, `--runner <name>`, `--prompt`, and `--ignore-local` ([The view](#the-view)). Run it anywhere inside the repository.

The role is a word after `start`, never a command of its own. The old form, `wb build 54`, ends with 2 and says the new one with the same words: `use: wb start build 54`. A word after `start` that is no role of the revision ends with 2 and names the roles.

### What it checks

In this order. The first fault ends it, before anything is posted or started:

1. Everything `wb prompt` checks for the same words. The message is the same, after the command's name (`wb start build:` where `wb prompt` says `wb prompt:`), and so is the status.
2. The rule table: a fix follows a review. A fix with no review stamp before it is refused with 1, also when `--round` is given, and also when the stamps cannot be read. The number of review rounds is not checked: a person who starts one more review has decided to. That limit is held where roles are handed on without a person (#88).
3. The class: no classed code is worked on by a session started here until a session sees only its files (#65). See [Classed code](#classed-code).
4. The runner has `start`, and no word of it is 128 KiB or more once filled: Linux hands a program no longer word. Otherwise 1, saying to use `--prompt`.
5. Standard input and standard output are a terminal. Otherwise 1, naming #65: no session is started out of a person's sight before the sandbox.
6. No local change to a file of the view, for a session of an issue with an accepted file ([The view](#the-view)). Otherwise 1, naming the files.
7. The view, for a session of an issue with an accepted file: no bind of the runner holds or lies in the checkout, the folder of the organisation's file or the state folder; there is a state folder; bubblewrap is at `/usr/bin/bwrap` and can make a sandbox; the checkout's `HEAD` names a branch; and the view is made. Otherwise 1, saying why. Where unprivileged user namespaces are off, bubblewrap cannot make a sandbox, and its own words are said.

Then `watchbill.yml` must have `issue_comment`, or `change_comment` for a change with no issue. Without it the command is refused with 1, and nothing is posted.

### The runner

The runner is `--runner <name>`, or else the `default` of the organisation's file ([Confidentiality](#confidentiality)). With neither, the command is refused with 2 and the usage. The runner must be one of the file, and have `start`.

### The round and the change

They are worked out, never asked. `--round` and `--change` are still taken, and win.

- **The stamps** are the lines `[stamp] <role> round <n>` that `issue_comments` prints: the first line of each comment on the issue, one to a line. Every stamp is posted by `wb`, so their form is known. For a review or a fix with no issue, they are those `change_comments` prints, on the change.
- **The round** of the breakdown, the tester, the build and the review is one more than the stamps of that role. The fix takes the number of review stamps: it answers the last review.
- **The change** of a review or a fix of an issue is the one open change from its branch, as `change_of_branch` prints it. None, or more than one, is refused with 1, naming what was found and `--change`.
- **What was worked out is said** on standard error, before the checks: `wb start review: round 2 (1 review stamp on issue 41)`, and for a review or a fix of an issue the change, once, on a line of its own: `wb start review: issue 41, change 174 (from the branch issue-41)`, or `(given)` where `--change` gave it. A review or a fix with no issue says `wb start review: change 174`. `wb prompt` says the line only for a change it worked out, and prints the prompt alone on standard output.
- `wb start <role>` always needs the round, for the stamp's first line. With no `--round`, and no `issue_comments` (or `change_comments`), it is refused with 1. A `--round` given is a whole number from 1, or the command was used wrongly (2).

`wb prompt` works out the same, by the same function, where the role's text holds `{round}` or `{change}` and the patterns are there.

### The stamp and the session

- **The stamp.** `wb` says on standard error what it will post. It then runs `issue_comment` with `{issue}` filled and the stamp on standard input: the line `[stamp] <role> round <n>`, then the lines `wb prompt --stamp` prints, with `runner:` the runner it starts. A review or a fix with no issue posts it by `change_comment`, with `{change}` filled. What the command prints goes to standard error. When it ends with anything but 0, nothing is started, and `wb` ends with 1.
- **The session.** `wb` runs the runner's `start`, its words filled: `{model}` and `{effort}` are those of the stamp, and `{prompt}` is the whole prompt as one word. Nothing goes through a shell. It runs in the root of the checkout, in the foreground, in the person's terminal: for an issue with an accepted file in its view, and otherwise as it is, which `wb` says first. Ctrl-C and Ctrl-\ go to the session, and `wb` waits for it. `wb` ends with 0 when the session ends with 0, and with 1 otherwise.
- **`--prompt`** prints the prompt as `wb prompt` does, and posts and starts nothing. `--stamp` and `--parts` do as with `wb prompt`, with `--prompt` or without. No `default` runner is taken then: the output is that of `wb prompt` for the same words.

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

  `check` is the one that refused: `prompt`, `rule`, `class`, `start`, `terminal`, `ci`, `local` or `view`. `message` is the first line of what was said. `not posted` is a stamp that was not posted, and no session was started. `ended` gives the session's own status (or `signal:` when a signal ended it) and the seconds it ran. A `wb` killed while the session runs leaves no `outcome`.
- **`ci`,** for a review: `ci: green`, `ci: red` or `ci: pending`, the word `change_checks` printed; `ci: not read` where there is no `change_checks`, and `ci: unread` where it printed something else or failed. It is kept whether or not the review started, so that a count of views that were green and CIs that were not can be made.
- **`local`,** for a start that `--ignore-local` let go on: the paths of the local changes it ignored, one to a line, escaped. A start with none has no such file.
- **`view` and `back`,** for a session in a view: the list of its paths, one to a line, whose SHA-256 is the stamp's `view:`; and what came back, a line each: `taken: <commit on the branch> <subject>`, `refused: <commit of the view> <subject>: <why>`, `not taken: …`, `uncommitted: <path>`, `failed: <why>`, and last `view: removed` or `view: kept <folder>`. A path is written escaped, so each is one line.
- **Not in the way:** a folder that cannot be written is said on standard error. It changes neither the status nor whether the session starts.
- **What writes none:** `--prompt`, `--stamp` and `--parts`, and `wb prompt` and `wb check`. Nor does a command used wrongly (status 2 before the checks begin): no start was tried.
- **What they are:** diagnostics on one machine, not a record ([Not goals](#not-goals)). `wb` never reads them back to decide anything, and nothing in them leaves the machine. They are plain files, so the team's own tools or a recorder can collect them, and a person removes them when they are no longer wanted.

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
draft_change: 'gh pr create --repo example/brake-controller --head {branch} --draft --fill-verbose'
```

- `issue_comment` and `change_comment` post a comment from standard input. `issue_comments` and `change_comments` print the first line of each comment, one to a line. `change_of_branch` prints the numbers of the open changes from `{branch}`, one to a line. At GitHub the comments of a pull request are those of an issue, so `change_comments` reads them the same way.
- `issue_comment_bodies` and `change_comment_bodies` print each comment whole, followed by a NUL byte. One line break straight after a NUL is not read as part of the next comment: `gh --jq` and `jq -r` print one. `wb answer` reads them ([Answering the questions](#answering-the-questions)); no role needs them.
- `draft_change` is the command that opens the change of `{branch}` as a draft, its title and description filled from the commits. `wb build` says it, filled, after a session; it never runs it.
- `open_issues` is no longer read (#155): `wb init` does not write it, and a line that is there is passed over. Listing the tracker's issues is the tracker's own program's job.
- Each is one line for `/bin/sh -c`, run in the root of the checkout, as an agent runs the addresses. The values filled in (`{issue}`, `{change}`, `{branch}`) are plain names, as for the addresses. A value given that is not is refused with 2, by its option. A branch worked out that is not is refused with 1, saying to give `--change`.
- A repository set up before these lines gets them from `wb upgrade` ([Upgrading it](#upgrading-it)); one with another server adds them by hand. Without them, `wb prompt` asks for `--change` and `--round` as before, and `wb start <role>` is refused, naming the line it needs.
- `wb start <role>` asks nothing of the network but through these lines and the runner's `start`.
- **Only as `HEAD` has it** (#186). `wb` runs a pattern of `watchbill.yml`, reads `needs` and `build_files` from it, and fills a prompt from it, only while `.watchbill/watchbill.yml` in the working tree is the file that `HEAD` has. It reads the file of `HEAD` once, with no replace refs, and takes every pattern from those bytes. `wb start`, `wb prompt`, `wb status`, `wb answer`, `wb revert` and `wb accept` are refused with 1, naming the file, before anything runs or is posted, where the file differs from `HEAD`, is staged and not committed, or is not in `HEAD` at all. The command `draft_change` that `wb start build` says after a session is held the same way: where the file differs, `wb` says the refusal and no line. A person reads the change and commits it, or takes it back with git, before the next command. It is a limit: no option lifts it. **It stops an uncommitted change only.** A session with no view runs as the person, with a shell, and can commit a changed `watchbill.yml`, move a ref or write a hook. A commit in the checkout by anyone, the session included, lifts the limit, so a person reads `git log -p -- .watchbill/watchbill.yml` before the next command. Running every session in a sandbox is #202. `wb check` runs no pattern and is not held. `wb init` and `wb upgrade` write the file and are not held; `wb upgrade` says that the file is to be committed (#174).

For a GitLab `origin`, `wb init` writes the same patterns with `glab`, here for `brakes/controller` on `gitlab.example.com`, and `issue_address` and `change_address` besides:

```yaml
issue_comment: 'glab api --hostname gitlab.example.com --method POST projects/brakes%2Fcontroller/issues/{issue}/notes -F body=@-'
issue_comments: 'glab api --hostname gitlab.example.com projects/brakes%2Fcontroller/issues/{issue}/notes --paginate --output ndjson | jq -r ''select(.system == false) | .body | split("\n")[0]'''
change_comment: 'glab api --hostname gitlab.example.com --method POST projects/brakes%2Fcontroller/merge_requests/{change}/notes -F body=@-'
change_comments: 'glab api --hostname gitlab.example.com projects/brakes%2Fcontroller/merge_requests/{change}/notes --paginate --output ndjson | jq -r ''select(.system == false) | .body | split("\n")[0]'''
change_of_branch: 'glab mr list --repo https://gitlab.example.com/brakes/controller --source-branch {branch} -F json --jq ''.[].iid'''
issue_address: 'glab issue view {issue} --repo https://gitlab.example.com/brakes/controller --comments'
change_address: 'glab mr view {change} --repo https://gitlab.example.com/brakes/controller --comments'
change_checks: 'glab api --hostname gitlab.example.com projects/brakes%2Fcontroller/merge_requests/{change} | jq -r ''if (.iid | type) != "number" then error("not a merge request") else (.head_pipeline.status // "none") as $s | if $s == "success" then "green" elif $s == "failed" or $s == "canceled" then "red" else "pending" end end'''
issue_comment_bodies: 'glab api --hostname gitlab.example.com ''projects/brakes%2Fcontroller/issues/{issue}/notes?sort=asc&order_by=created_at'' --paginate --output ndjson | jq -j ''select(.system == false) | .body + "\u0000"'''
change_comment_bodies: 'glab api --hostname gitlab.example.com ''projects/brakes%2Fcontroller/merge_requests/{change}/notes?sort=asc&order_by=created_at'' --paginate --output ndjson | jq -j ''select(.system == false) | .body + "\u0000"'''
draft_change: 'glab mr create --repo https://gitlab.example.com/brakes/controller --source-branch {branch} --draft --fill --fill-commit-body --yes'
```

- The host is that of `origin`, without a user name or a token; without it `glab` asks gitlab.com. An `origin` with a port gets none of these lines.
- The comments are posted through `glab api`, which reads the whole text from standard input; `glab issue note` would ask for it at a prompt. `glab api` has no `--jq`, so `issue_comments` and `change_comments` need `jq` on the machine of the team. `wb` does not. They keep no system note, and print the first line of each other note.
- The project is written in the path of the API with each `/` as `%2F`.
- `issue_comment_bodies` and `change_comment_bodies` ask the notes oldest first (`sort=asc`), and keep no system note. They, and `draft_change`, were written from the documentation of the API and of `glab`, and have not been run against a GitLab server.

`change_checks` is read by `wb start review` before it posts the stamp (#127). It prints one word for the change, `green`, `red` or `pending`, and ends with 0.

- `wb start review` starts on `green`. On `pending` it is refused with 1, saying to wait. On `red` it is refused with 1, saying that the change goes to a person, and that the lists or the stubs of the breakdown may be wrong: the CI builds the whole tree and runs every test, and the session saw only its view. A word that is not one of the three, or a status but 0, is refused with 1, naming `change_checks`.
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
- The breakdown is not checked on the class. It changes only the issue's file.
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
wb start breakdown 54     # writes .watchbill/issues/54.yml on the branch issue-54
wb accept 54        # the person accepts it, and wb commits the acceptance
wb start tester 54        # the tests, red
wb answer 54        # the person answers what a session asked, any time it asked
wb start build 54         # the code, committed; wb says how to push it and open the change
wb start review 54        # round 1
wb start fix 54           # answers round 1
wb start review 54        # round 2
```

Each command posts its stamp and starts its session. The person answers the questions with `wb answer`, pushes the branch, opens the change and merges.

### The exit status

- **0**: the session ended with 0; with `--prompt`, `--stamp` or `--parts`, it was printed.
- **1**: it was refused, by `wb prompt`'s checks or by one of the five after them; `watchbill.yml` has no pattern it needs; a pattern ended with anything but 0; the session could not be started, or did not end with 0; or a commit of its view was refused, or the view's work could not be taken back.
- **2**: it was used wrongly, as for `wb prompt`, or with no runner and no `default`, a `--round` that is not a whole number from 1, or an issue or a change to post on that is not plain; the folder is not in a git repository; or Watchbill is not set up.

### What it does not do

- Start a session without a terminal, for the listener (#85, planned for 0.12.0).
- Run the breakdown, or a role with no issue's file, in a view. Every session in a sandbox is #202, planned for 0.11.0.
- Keep the session's last message and hand it on (#88, planned for 0.12.0).
- Know whether a session of the same role already runs on the issue (#88, planned for 0.12.0).
- Say what the session does with its prompt. The stamp says what was handed out and started.

## Where each issue stands

`wb status` lists the issues in the flow under headings, as `git status` lists files under theirs: each issue under the last step it has done, with its branch. It only reads, and it says what is, never what to do. It runs anywhere inside the repository.

```
Not broken down:
  178  issue-178

Broken down:
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
  - `Broken down`: the issue's file is on the issue's branch;
  - `Not broken down`: none of these.
  A new breakdown takes the issue back: only the stamps after the last `[stamp] breakdown` count, and with none after it the issue is under `Accepted` or `Broken down` by its file.
  A heading with no issue is not printed. No round, no command and no word for a step going on is printed.
- **The questions, last.** `Questions open` lists each issue that has a question no one has answered, with the names of those questions. The issue stays under its step too. A question is asked by a comment whose first line is `[questions] <issue>.<role> q1 q2` (`[questions] <issue> q1` for the breakdown), which names `<issue>.<role>.q1` and `<issue>.<role>.q2`, and closed by `[answers] <issue>.<role> q1 …`, which names the questions it answers ([Answering the questions](#answering-the-questions)). The forms of revision 0.0.8 are still read: a question `[question] <name>`, closed by an `[answer] <name> …` line, and `[answer] <issue> suggested`, which closes every question asked before it. Other words after `[answers] <issue>.<role>` close nothing.
- **Who answered.** `issue_comments` prints the first line of each comment and no author, so `wb` cannot yet tell an answer of a person from another. Until it reads who wrote a comment (#83), each `[answers]` and `[answer]` line counts as a person's; a line that starts with `[question]` or `[questions]` is never an answer. It reads first lines only, so an answer below the first line of a comment is not seen, which errs towards "open"; and a choice number is not held against the choices of the question (`wb answer` holds it when it posts): `[answer] 5.q1 7` closes `5.q1` whatever its choices are.
- **Without `issue_comments`** in `watchbill.yml` no stamp, question or answer is read, so each issue is at `Accepted` or below; `wb status` says so once on standard error.
- **Which issues:** those in the flow: an issue whose branch, by the pattern `branch` (local, or at `origin`), holds a commit that the default branch of `origin` does not. Git alone says which, from the refs as they are: it writes nothing and fetches nothing. The default branch is `refs/remotes/origin/HEAD`, else `main`, else `master`; where none is there, or git cannot list the branches, `wb status` says so and ends with 2. The tracker's own program lists the rest. A title, which `wb status <issue>` prints, is printed escaped: control characters, line and paragraph separators, bidi controls and the backslash are written as escapes (#167).
- **An issue that is done** is not listed, also when its file is on the default branch. Its branch is done when it is merged into the default branch; or when its whole change is one commit on it, found by `git patch-id --stable` of the diff from the merge base to the branch against each commit since the merge base (a squash merge, as a pull request is merged); or when it is a local branch whose upstream at `origin` is gone, as after `git fetch --prune`. The limits: a squash whose diff differs from the branch's (a branch not brought up to date with the default branch, or a squash edited at the server) is not found, and the issue stays listed; an issue merged since the last fetch stays listed until a fetch, and a branch that is only on the server shows once it is fetched.
- **The line of an issue:** its number, its branch and, once there is one, its open change from `change_of_branch` as `#<n>`; where the local branch holds commits that the branch at `origin` does not (or `origin` has none of that name), how many: `129  issue-129  #178  1 not pushed`. It says what is, never what to run. No title is printed in the list. A branch with no issue's file on it yet is under `Not broken down`.
- **`wb status <issue>`** prints that issue whatever its state, with its title (the first line of `issue_address`, escaped) last on its line, and under it `Worked out from:` and the lines it was worked out from: its stamps, questions and answers, and the branch and files it found. An issue that is not in the flow has the line `not in the flow: ...` first.
- **`wb status --stage <heading>`** prints only that heading, for example `wb status --stage "not broken down"`. The names are matched without regard to case; `Questions open` and `Not read` too. A name that is not known is used wrongly.
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

1. Let the tester read the two files of tests.
2. Leave the clause without a test.

Suggested: 1, because the clause needs a check.

changes the issue's file: yes

</details>

q2: …
```

```sh
wb answer 54                    # each open question, one at a time
wb answer 54 --suggested        # every suggestion
wb answer 54 tester.q1 2        # one answer, without asking
wb answer 54 q3 "only the first command"   # a question of the breakdown
wb answer --change 174          # a change with no issue
```

- **What it shows:** each open question of the issue, in the order asked, with its name, what is unclear, why it matters, the choices and the suggestion, laid out plainly. A question is open until an answer names it, as `wb status` reads it, and a question a `[revert]` marker took back is not shown. The questions of revision 0.0.8, one `[question] <name>` comment each, are shown and answered the same way.
- **What it asks:** Enter takes the suggestion. A number takes that choice; a number that is not one of the choices is asked again. Any other text is the answer in words.
- **What it posts:** first it says on standard error what it will post, then one comment per role through `issue_comment`: `[answers] 54.tester q1 q2`, then `q1: 2` and `q2: <words>`, a line each. Nothing is posted until every question shown has its answer: an input that ends first, or Ctrl-C, posts nothing.
- **`--suggested`** takes every suggestion and names the questions that have none, which stay open. A suggestion is posted written out, as `q1: 1`.
- **`<question> <answer>`** answers one question without asking: `tester.q1`, or `q1` for the breakdown, whose questions are `<issue>.q<n>`. The answer is read as at the terminal, and a number that is not a choice is refused with 1.
- **The issue's file.** Where an answered question says `changes the issue's file: yes`, `wb answer` says after posting that the file is to be changed and accepted again (`wb accept`) before the role is started again; for the breakdown, that the breakdown is started again to fold the answers in. It never changes the file.
- **A change with no issue:** `wb answer --change <n>` reads `change_comment_bodies` and posts through `change_comment`.
- **What it reads:** the whole comments, through `issue_comment_bodies` of `watchbill.yml` ([The patterns](#the-patterns)). Without it, or without `issue_comment`, it is refused with 1, naming the line, and posts nothing.
- **The exit status:** 0 when the answers are posted, or no question is open, which it says; 1 when it was refused, the input ended before every question was answered, or a pattern failed; 2 when it was used wrongly or Watchbill is not set up.

## Taking a step back

`wb revert <issue> <role>` takes an issue back to before a step, for example to run review round 1 again. It deletes nothing at the tracker: it posts one marker, and from then on `wb` reads the issue as if that step and every step after it had not been done. Deleting comments would rewrite the history that an assessor reads, and the questions and answers in it are what a person decided.

- **The marker** is one comment whose first line is `[revert] <role> round <n>`. `wb revert` finds the last `[stamp] <role> round <n>` on the issue, says on standard error what it will post, and posts it through `issue_comment` of `watchbill.yml`. `--round <n>` names an earlier round of that role; it is a whole number from 1.
- **What it means:** the stamp it names, and every `[stamp]`, `[questions]`, `[answers]`, `[question]` and `[answer]` after it up to the marker, no longer count, whatever their role. Reverting review round 1 after fix round 1 takes the fix back too, since the fix answered that review. An `[answer]` after that stamp is taken back too, also one that answers a question asked before it: that question is then open again, as the owner decided (#142). A stamp before it counts as before, so the next review after `wb revert 174 review` is round 1 again if it was the first.
- **Who reads it:** the round that `wb start <role>` and `wb prompt` work out ([the round and the change](#the-round-and-the-change)), the check that a fix follows a review, the step and the open questions of `wb status`, and the questions `wb answer` shows. A marker is read only from the first line of a comment, as a stamp is, so a marker quoted in a comment's text counts for nothing, and so does one with no such stamp before it, and so does a `[revert] breakdown` posted by hand. `wb status <issue>` marks the lines a marker took back in what it was worked out from.
- **A change with no issue:** `wb revert --change <n> <role>` posts the marker on the change, through `change_comment`, and reads the stamps from `change_comments`.
- **What it does not touch:** the branch, the issue's file and its acceptance, and the change. The commits of the steps taken back stay on the branch, and taking them back is git's; `wb revert` says so on standard error.
- **A later breakdown:** when a `[stamp] breakdown` stands after the stamp to be reverted, that stamp no longer counts, since the new breakdown took the issue back ([where each issue stands](#where-each-issue-stands)). `wb revert` is then refused with 1, saying so, and posts nothing.
- **The breakdown** is not taken back by a marker. The issue's file and its acceptance are files of the repository, and git takes them back. `wb revert <issue> breakdown` is refused with 1, and says so.
- **Refused with 1:** a role with no stamp on the issue that still counts (naming the role), a missing `issue_comments` or `issue_comment` (or the change's), a pattern that ends with anything but 0.
- **The exit status:** 0 when the marker is posted; 1 when it was refused or the posting failed; 2 when it was used wrongly (no issue or role, a role the revision does not have, an option that is not known, a `--change` that is not a plain name, `--round` that is not a whole number from 1, an issue and `--change` together) or Watchbill is not set up.
- **Not yet:** who may revert once the roles have accounts (#83, planned for 0.12.0). Until then any person who can post a comment can.

## Taking it on one role at a time

A team takes Watchbill on one role at a time (#116). A person can start any role alone, with no step before it: no breakdown, no issue's file, and for a review or a fix, no issue.

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
- **The breakdown and the acceptance:** once an issue has a file, the chain's gates bind. The file must be accepted, and the file and its acceptance committed, before any role of the issue is handed out.
- **The hand-on** from one role to the next always needs the whole chain: an accepted file for every issue. It is not built yet: `wb handover` (#88, planned for 0.12.0), the listener (#85, 0.12.0) and the officer (#86, 0.12.0).

## The issue's file and its acceptance

An issue has one file, `.watchbill/issues/<issue>.yml`, which the breakdown writes or a person does ([docs/revision.md](docs/revision.md), point 13). It is the project's file, not the revision's: `wb check` does not fail on it. `wb` reads it through the one loader, so values are plain text, and holds it to this form:

```yaml
issue: '54'
from: <SHA-256 of the issue's text the breakdown read>
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

A test is linked from what the view holds. A function that the code hides, and that the tests need, is a stub: a file of its own, listed in the tester's and the build's `reads`, and the breakdown lists it there. The breakdown runs `needs` itself before it commits the issue's file, so a list that falls short is found before a person reads it.

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

## The name

A watch bill is a ship's posted list of who stands which watch at which station.

## Beside Handover Ledger

Watchbill is a product of its own. [Handover Ledger](https://github.com/fa1k3n/handover-ledger) records the handovers in a team and can be used to monitor a team that runs Watchbill. Watchbill does not need it.

## The revision

What a revision is, and each decision behind it, is in [docs/revision.md](docs/revision.md).

## Licence

Watchbill is licensed under the Apache License, Version 2.0. The text is in [LICENSE](LICENSE) and the holder of the copyright is named in [NOTICE](NOTICE).

## Not decided

- Which standard comes first, and which tools the first adapters are for.
