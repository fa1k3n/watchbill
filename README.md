<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/logo-dark.svg">
    <img src="docs/logo-light.svg" width="96" height="96" alt="The mark of Watchbill: a grid of three by three rounded squares, the diagonal filled in another colour">
  </picture>
</p>

# Watchbill

A harness for AI agents in embedded software development, deployed at a fixed revision onto a project that already exists.

**Status: an idea, written down 2026-11-16. Built: `wb init`, `wb check`, `wb prompt`, `wb accept`, `wb revision`, and `wb <role>`, which starts a session of a role at a person's terminal. Nothing hands work from one role to the next yet.**

## What Watchbill is for

Watchbill is for teams that build software under a standard, such as IEC 62304 for medical devices or ISO 26262 in automotive, or under rules about which source code may leave the premises.

AI agents give such a team three problems:

- **Roles.** The standard, or the team's process under it, says who may specify, implement, verify and release. One agent session does all four unless something stops it.
- **Confidentiality.** A hosted model sees every file its agent opens. Some code may not leave the premises.
- **Assessment.** An assessor asks what the agents are told to do, and what stops them from doing anything else. A prompt typed into a terminal answers neither.

Watchbill sits between the team's tools and its agents, and is built to answer each:

- **Roles.** The work is split into roles: breakdown, tests, build, review and fix. Each role has a fixed, versioned prompt. Where a limit of a role can be checked, the program checks it and refuses. No agent checks its own work, and no agent merges.
- **Confidentiality.** A session is given only the files its work needs, and is started only with tools and a model the organisation has cleared for that code. Code that must stay on the premises is worked on there, by a model that runs there. One role reads the whole code to make the lists of files, and runs only where the organisation has cleared it.
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

### When it works

A team takes an issue from its text to a merged change, and a person only accepts what is to be built, starts each role, answers questions and merges.

### Where it stands

At 0.0.9. An issue that changes a verdict changes its row.

| Goal | Today | What will hold it |
|---|---|---|
| 1. Fits the workflow | Partly: any tracker and git server with a command line, by a pattern the team writes. `wb init` writes the patterns for GitHub and GitLab only. The CI and the requirements tool are names and nothing more. `wb` installs on Fedora and Enterprise Linux only | |
| 2. What the agents are told | Held for what `wb` hands out: the pinned revision, `wb check`. `wb <role>` posts the stamp and starts the session with that prompt itself. A session started by hand is the word of the one who started it, and so is the effort of a runner whose `start` cannot set it | |
| 3. Rules enforced | Partly: the pinned revision, the acceptance, the breakdown's runner and the placeholders are checked in code. What a session of an issue with an accepted file reads and changes is held by its view. No other limit of the rule table is | #28 names each limit and what holds it; then #88 and the pieces #28 writes |
| 4. People keep the decisions | Partly: `wb accept` asks for a person's word, but cannot tell a person from an agent under one account. That an agent cannot merge rests on the git server's settings | The roles' accounts, #83, and the acceptance read at the tracker, #88. No issue yet checks that a role's account cannot merge |
| 5. Duties separated | Partly: the rule table says it, and nothing knows who holds which role | #83 and #88, for work that is handed on. No issue yet for a session a person starts |
| 6. Sees only the code it needs | Partly: a session that `wb <role>` starts for an issue with an accepted file runs in a view, in bubblewrap. It sees the paths of its lists and no history, and its commits come back only where `changes` allows ([The view](#the-view)). `wb accept` checks that the lists hold what each file needs to build, by the team's own command `needs` ([below](#the-lists-checked-at-acceptance)). A session started by hand, for an issue with no file, or as the breakdown sees the whole checkout. What still leaks is [said below](#what-still-leaks) | No issue yet for a session started by hand |
| 7. Code goes only where allowed | Partly: checked for the breakdown | The other roles, #11. No issue yet for a local model for them |
| 8. Starts small | Held: `wb <role>` starts any role alone, for an issue with no file, and a review or a fix of a change with no issue, and posts the stamp on the issue or the change ([one role at a time](#taking-it-on-one-role-at-a-time)) | |
| 9. Starts strict, opens up | Not yet: one level for the whole repository, and no level given means the strictest | #11, #65, and a level for each path, which has no issue yet |
| 10. Shown to an assessor | Partly: what the agents are told can be read. The rule table says more than the program enforces | #28 |
| 11. Needs no other product | Held: one RPM, `git`, and a build with the network off | |

### Not goals

- A record of what happened. That is for the team's own tools or a recorder, such as [Handover Ledger](https://github.com/fa1k3n/handover-ledger). Watchbill needs none. The stamp `wb` asks for on the issue is working state, not an archive.
- A new place to work. There is no dashboard and no tracker of its own.
- A model or an agent of its own. Watchbill ships neither.
- Deciding what is built. That is a person's.

### Later

- Agents handing work to each other without a person starting each step. A convenience on top of the goals, not a goal.
- Hardware in the loop: tests that run on the real device.

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
- **A file that is already there is never changed.** When any file `wb init` would write is there, or a folder on the way to one is a link or not a folder, or `.watchbill/revision` is there, it writes nothing at all and names the path. Bringing a repository to a newer revision is a command of its own, later. A write that fails part way removes what it had written.
- **The project's own text goes in `.watchbill/project/`.** `all.md` is added to the prompt of every role and `<role>.md` (`build.md`, `review.md`, `fix.md`, `tester.md`) to that of one role. Both are optional. They are the project's files: not part of the revision, not in the manifest, and `wb check` does not fail on them, except for a placeholder that the revision does not know (see `wb check`). A prompt is the role's text of the revision, then one fixed line, then `all.md`, then `<role>.md`. This text says how things are done here: the commands, the board, the words. It never lifts a limit of a role. `wb init` leaves `project/README.md`, which says so. The placeholders of `revision/roles/placeholders.md` (`{issue}`, `{branch}` and the others) are filled in every part, and any other placeholder is refused. A placeholder is a lower-case word in braces: `{`, a letter a to z, letters a to z or `_`, then `}`, with no `$` just before it. Anything else in braces is text and stays as it is, such as `${BUILD_DIR}`, `{Issue}`, `{issue-id}` and `{ issue }`. A value that is filled in is never itself read for placeholders. The command that prints a prompt comes later.
- **`build_files` is the build** ([The lists checked at acceptance](#the-lists-checked-at-acceptance)). `wb init` writes the file or folder of each CI driver it found in the repository, whatever the answer for the driver: `.github/workflows`, `.gitlab-ci.yml`, `Jenkinsfile`, `azure-pipelines.yml`, `bitbucket-pipelines.yml`, `.circleci`, `.zuul.yaml` and `zuul.d`. Where it found none, it writes no line. The rest of the build, such as `Makefile` or `CMakeLists.txt`, a person adds.
- **`classed` is the code that is classed** ([Classed code](#classed-code)). `wb init` proposes `[]`, none, and `--yes` takes that. A person who answers gives paths from the root of the repository, separated by commas; a path that starts with `/`, holds `..` or is the root is not taken, and the question is asked again. A repository set up before this has no line `classed`, so every path counts as classed: add the line by hand.
- **The answers are only written down.** No adapter exists yet, so a git server or a tracker is a name and nothing more. For a GitHub or a GitLab `origin`, `wb init` also writes the patterns that `wb prompt` and `wb <role>` run ([below](#starting-a-session)).

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

## The prompt of an assignment

`wb prompt <role>` prints the prompt of a role on standard output, and nothing else. Whatever starts an agent's session, in any language, calls it as a process. Run it anywhere inside the repository. It writes nothing. `wb <role>` ([below](#starting-a-session)) does all that `wb prompt` does, then posts the stamp and starts the session.

The prompt is the role's text of the pinned revision, then the project's own text (`.watchbill/project/all.md` and `.watchbill/project/<role>.md`, where they are). A call asks only for what `wb` cannot know on the machine: the issue, and for the review and the fix the change and the round. The change and the round live at the tracker and the git server. Where `watchbill.yml` has the patterns for them, `wb prompt` works them out by running those (#84); otherwise they are given.

```sh
wb prompt tester 54
wb prompt build 54
wb prompt review 54                                 # the change and the round worked out
wb prompt fix 54 --change 168 --round 1             # given, as a repository with no patterns needs
wb prompt review --change 174                       # a change with no issue
```

The issue is the word after the role, or `--issue`; both together end with 2. The tester and the build with no issue end with 2 and the usage. Only the review and the fix take a change alone, with no issue (see [Taking it on one role at a time](#taking-it-on-one-role-at-a-time)). The placeholders are filled from the options, one for each, and what no option gives is derived. An option given always wins over the derived value, and the stamp holds each value as it was handed out:

- `--issue <text>`, `--change <text>`, `--requirements <text>`, `--round <text>`, `--checkout <text>`, `--branch <text>`, `--issue_address <text>`, `--change_address <text>`. What each is, is in `.watchbill/revision/roles/placeholders.md`.
- `checkout` is the root of the repository `wb` runs in.
- `issue_address` and `change_address` are the patterns of those names in `.watchbill/watchbill.yml`, with `{issue}` and `{change}` filled in. `wb init` proposes them for a GitHub `origin` (`gh issue view {issue} --repo <repository>; gh issue view {issue} --repo <repository> --comments`, `gh pr view {change} --repo <repository>; gh pr view {change} --repo <repository> --comments`: each shows the text or the description first, then the comments, as `--comments` alone hides the first). For a GitLab `origin` it writes one command each, with `--comments`. For another server, or none, there is no pattern and the option must be given.
- `branch` is the pattern of that name in `watchbill.yml`, proposed as `issue-{issue}`, with `{issue}` filled in. The breakdown makes the branch, so it gets the pattern as it is. For the tester, the build, the review and the fix it is the one local branch (from the refs of the repository) whose name is the pattern, or starts with it and a `-`. None, or more than one, is refused with 1, the names found are said on standard error, and `--branch` must be given.
- For a review or a fix of a change with no issue, `branch` is the branch checked out. `wb` asks nothing of the network, so it cannot see which branch the change is from: it refuses, with 2 and the words to give `--branch`, a HEAD on no branch, the branch that `origin/HEAD` names (where changes go), and any branch where `origin/HEAD` is not there.
- `round` and `change`, where the role's text holds them (the review and the fix) and they are not given, are worked out as `wb <role>` works them out ([the round and the change](#the-round-and-the-change)), when `watchbill.yml` has the patterns. Without them the options are needed, as before. What was worked out is said on standard error, for example `wb prompt: round 2 (1 review stamp on issue 41), change 174 (the one open change from issue-41)`. This, and nothing else of `wb prompt`, asks the network, through the team's own patterns. The tester and the build ask nothing.
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
- `--stamp` prints, in place of the text, the stamp of the assignment: what was handed out ([docs/revision.md](docs/revision.md), point 12). It is lines of `name: value` in this order: `role`, `model`, `effort`, `runner`, `version`, `revision`, `issue`, `change`, `round`, `branch`, `checkout`, `time`, then one line `part: <sha256>  <path>` for each part. `model` and `effort` are those of the model table for the role, and for the breakdown those its runner is cleared with. `runner` is the value of `--runner`, and empty when none is given. `version` and `revision` are those of the pinned revision. `time` is UTC, as `2026-11-17T18:04:05Z`. A value that is empty, or that the prompt of the role does not hold (`--change` for `build`), is written as its name and the colon alone: it was not handed out. The round is the exception: `round:` holds the round that `--round` gives, for every role. Where it is not given, it is worked out only for a role whose text holds `{round}`, as above; for the tester and the build `round:` is then empty, because `wb prompt` asks nothing of the network for them. `wb <role>` always fills it, and it agrees with the first line it posts (#136). A value of `--issue`, `--change`, `--round`, `--branch` or `--checkout` that holds a control character, a line or paragraph separator, or white space at its start or end is refused by name, as one field must stay one line that a comment can trim. `--stamp` with `--parts`, and either given twice, are used wrongly.

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

Each session has its stamp on the issue, one comment for each session, the first line `[stamp] <role> round <n>`, and the lines of the stamp after it. A review or a fix with no issue has its stamp on the change. `wb <role>` posts it ([below](#starting-a-session)). Where a session is started by hand, the person posts the stamp as it was printed. A session with no stamp on its issue, or on its change where it has no issue, is not taken as the work of a role.

```
[stamp] build round 1
role: build
model: claude-sonnet-5-5
effort: medium
runner:
version: 0.0.9
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

Where an assignment may run is the organisation's to say, not the project's ([docs/revision.md](docs/revision.md), point 11). In this revision it is checked for the breakdown, which reads more of the code than any other role.

The organisation lays out one file on each machine, by its own means: `/etc/watchbill/organisation.yml`. `wb` reads it from there and from nowhere else: no option and no environment variable changes the path, so a team cannot clear a runner for itself. It is read through the same loader as every YAML file of `wb`.

```yaml
levels: [public, internal, confidential]   # from the least to the most restrictive
default: claude-code                       # the runner `wb <role>` takes when none is given
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
    start: [claude, --model, '{model}', --effort, '{effort}', '{prompt}']   # how `wb <role>` starts a session
    binds: ['~/.local/bin/claude', '~/.local/share/claude', {writable: '~/.claude'}, {writable: '~/.claude.json'}]
```

- `levels` is at least one name, each once, from the least to the most restrictive.
- A runner has `tools`, `model`, `agreement` and `up_to`, which is one of the levels. `breakdown` is there only when the runner is cleared for the breakdown: its `up_to` is no higher than the runner's own, its `model` is an exact model ID and its `effort` is one of the model table's.
- `start` is there only when `wb <role>` may start sessions on the runner ([Starting a session](#starting-a-session)). It is a list of one word or more, each a text of one line. `{model}`, `{effort}` and `{prompt}` are the only placeholders, and the first word, the program, holds none.
- `binds` is there only when the runner needs paths of its own in a view ([The view](#the-view)): its program, its settings, its login. Each is a path from the root, `/…`, or from the home folder of the person who runs `wb`, `~/…`, with no `..`, and not the root or the home folder itself. It is read-only, unless it is written `{writable: <path>}`. A path that is not there is left out. The paths above are where Claude Code keeps itself on one machine; they are an example, not a promise.
- `default` is the name of one of the runners.
- A key that is not known, at any depth, a level written twice, a field missing or a value of the wrong type is refused, with the file and the place named.

The repository states its own level, as the line `confidentiality:` of `.watchbill/watchbill.yml`. Where the line is missing, the level is the most restrictive of the list. A level that is not on the list is refused by name, and so are `levels` and `runners` in `watchbill.yml`: they are the organisation's.

`wb prompt breakdown <issue> --runner <name>` hands out the prompt only when the organisation's file is there, names the runner, the runner has `breakdown`, and the repository's level is no higher than the breakdown's `up_to`. Otherwise it prints nothing on standard output, names on standard error the file it looked for, the runner, or the level and the `up_to`, and ends with 1. With no `--runner` the breakdown ends with 2 and the usage. Its stamp has `runner: <name>` and the `model` and `effort` of the runner's `breakdown`, and `--parts` adds two lines after the parts: `level: <the repository's level>` and `runner: <name>`.

What it does not do yet (#11): the tester, the build, the review and the fix are not refused on a runner or a missing file, and the level of an issue is not read. It cannot stop a person from pasting code into another tool, or running a session with a prompt that `wb` did not hand out. The stamp on the issue is what shows where the work was handed out.

#### What still leaks

A view ([below](#the-view)) keeps the rest of the code and the history away from a session. It does not keep everything:

- Every file of the lists goes to the model, as the session reads it. The level of the repository still decides which runner may have them.
- A test that passes or fails tells one bit about code the session cannot read, at each try. A session that runs the tests many times learns more.
- The runner's own settings and login are inside the view, as its `binds`. What the runner keeps there, the session can read.
- Nothing stops a person from pasting code into another tool.

## Starting a session

`wb <role>` starts a session of a role in the person's terminal (#84). One command checks what `wb prompt` checks, posts the stamp, and starts the session with the model and the effort of the stamp. A person writes no stamp, no shell and no model name.

```sh
wb breakdown 54
wb tester 54
wb build 54
wb review 54
wb fix 54
wb review --change 174        # a change with no issue
```

Each takes the words `wb prompt` takes for its role, `--runner <name>`, and `--prompt`. Run it anywhere inside the repository.

### What it checks

In this order. The first fault ends it, before anything is posted or started:

1. Everything `wb prompt` checks for the same words. The message is the same, after the command's name (`wb build:` where `wb prompt` says `wb prompt:`), and so is the status.
2. The rule table: a fix follows a review. A fix with no review stamp before it is refused with 1, also when `--round` is given, and also when the stamps cannot be read. The number of review rounds is not checked: a person who starts one more review has decided to. That limit is held where roles are handed on without a person (#88).
3. The class: no classed code is worked on by a session started here until a session sees only its files (#65). See [Classed code](#classed-code).
4. The runner has `start`, and no word of it is 128 KiB or more once filled: Linux hands a program no longer word. Otherwise 1, saying to use `--prompt`.
5. Standard input and standard output are a terminal. Otherwise 1, naming #65: no session is started out of a person's sight before the sandbox.
6. The view, for a session of an issue with an accepted file ([The view](#the-view)): no bind of the runner holds or lies in the checkout, the folder of the organisation's file or the state folder; there is a state folder; bubblewrap is at `/usr/bin/bwrap` and can make a sandbox; the checkout's `HEAD` names a branch; and the view is made. Otherwise 1, saying why. Where unprivileged user namespaces are off, bubblewrap cannot make a sandbox, and its own words are said.

Then `watchbill.yml` must have `issue_comment`, or `change_comment` for a change with no issue. Without it the command is refused with 1, and nothing is posted.

### The runner

The runner is `--runner <name>`, or else the `default` of the organisation's file ([Confidentiality](#confidentiality)). With neither, the command is refused with 2 and the usage. The runner must be one of the file, and have `start`.

### The round and the change

They are worked out, never asked. `--round` and `--change` are still taken, and win.

- **The stamps** are the lines `[stamp] <role> round <n>` that `issue_comments` prints: the first line of each comment on the issue, one to a line. Every stamp is posted by `wb`, so their form is known. For a review or a fix with no issue, they are those `change_comments` prints, on the change.
- **The round** of the breakdown, the tester, the build and the review is one more than the stamps of that role. The fix takes the number of review stamps: it answers the last review.
- **The change** of a review or a fix of an issue is the one open change from its branch, as `change_of_branch` prints it. None, or more than one, is refused with 1, naming what was found and `--change`.
- **What was worked out is said** on standard error, before the stamp: `wb review: round 2 (1 review stamp on issue 41), change 174 (the one open change from issue-41)`.
- `wb <role>` always needs the round, for the stamp's first line. With no `--round`, and no `issue_comments` (or `change_comments`), it is refused with 1. A `--round` given is a whole number from 1, or the command was used wrongly (2).

`wb prompt` works out the same, by the same function, where the role's text holds `{round}` or `{change}` and the patterns are there.

### The stamp and the session

- **The stamp.** `wb` says on standard error what it will post. It then runs `issue_comment` with `{issue}` filled and the stamp on standard input: the line `[stamp] <role> round <n>`, then the lines `wb prompt --stamp` prints, with `runner:` the runner it starts. A review or a fix with no issue posts it by `change_comment`, with `{change}` filled. What the command prints goes to standard error. When it ends with anything but 0, nothing is started, and `wb` ends with 1.
- **The session.** `wb` runs the runner's `start`, its words filled: `{model}` and `{effort}` are those of the stamp, and `{prompt}` is the whole prompt as one word. Nothing goes through a shell. It runs in the root of the checkout, in the foreground, in the person's terminal: for an issue with an accepted file in its view, and otherwise as it is, which `wb` says first. Ctrl-C and Ctrl-\ go to the session, and `wb` waits for it. `wb` ends with 0 when the session ends with 0, and with 1 otherwise.
- **`--prompt`** prints the prompt as `wb prompt` does, and posts and starts nothing. `--stamp` and `--parts` do as with `wb prompt`, with `--prompt` or without. No `default` runner is taken then: the output is that of `wb prompt` for the same words.

### The view

A session of the tester, the build, the review or the fix of an issue with an accepted, committed file sees only the files its issue's file lists for it (#126). The program keeps this limit, not the prompt.

- **What it holds.** A new folder, `<state>/watchbill/views/<time>-<role>-<issue>/`, made by `wb` from the paths of `changes` and of the role's `reads`, as they are in `HEAD` of the checkout. It is a new repository with one commit of those files, on a branch of the checkout's name, with no remote and no history of the repository. A path of `changes` that is not there yet is not made: the session may make it. A worktree or a sparse checkout would not do: `git show` reads every file from the history.
- **Where it runs.** In bubblewrap (`/usr/bin/bwrap`, from the package `bubblewrap`): the view at the path of the checkout, writable, so that `{checkout}` of the prompt is right. `/usr`, `/etc` and `/sys` are there read-only, `/tmp` and `/run` are its own, and the home folder is an empty one of its own, with the runner's `binds` in it. The checkout's `.git` and `.watchbill/`, the folder of the organisation's file and the person's other folders are not there. The variables `GIT_*` are not handed in. The network stays as the runner needs it: the view is what limits the code that can be sent.
- **A terminal.** Where the kernel lets a program type into its terminal (`TIOCSTI`), a session could type into the person's shell after it ends. `wb` gives a session in a view no controlling terminal then. Where `/proc/sys/dev/tty/legacy_tiocsti` is `0`, as on Fedora, the kernel refuses it, and the session keeps its terminal.
- **What comes back.** When the session ends, `wb` takes each commit of the view, in order. A commit that touches a path outside `changes`, under `.watchbill/` or under `build_files` of `watchbill.yml`, a merge or a submodule is refused, with the paths named: nothing of it or after it is taken, and `wb` ends with 1. The commits before it are put on the checkout's branch, with their messages and authors, by a fast-forward. Work left uncommitted in the view is named and not taken.
- **The push is the person's** (#145). A view has no remote, and the prompts of the tester, the build and the fix say to commit and not to push or open a change. When commits were taken onto the branch, `wb` says on standard error the command that pushes it, `git push origin <branch>`, and after a build the command that opens the change as a draft: `draft_change` of `watchbill.yml`, with `{branch}` filled ([The patterns](#the-patterns)). Without that line it says so. `wb` runs neither. A role started without a view is told the same in its prompt, and `wb` says the same commands after it when the branch checked out has new commits. That `wb` pushes itself is #151.
- **What is left.** The view is removed when all of it was taken back. It is kept, and named, when anything was refused or left uncommitted.
- **How it is read.** Once the session has run, the `.git` of the view is the session's. `wb` never runs git with its settings, which would run outside the sandbox a command the session wrote there: it reads the view's objects through a repository of its own, and its `HEAD` from the files.
- **Without a view.** A role started with no issue's file (#116), and the breakdown, which reads the whole code by its role, run as before, in the checkout. For a role with no file, `wb` says so on standard error before it starts. A person who starts it has decided to.
- **The stamp** has the line `view:`, after `time:`: the SHA-256 of the list of paths, one to a line, as the file `view` of the lines holds it, or `none`. `wb prompt --stamp` gives the same line: it says what was handed out, as the model and the effort do.

### The lines it leaves

Each start of `wb <role>`, started or refused, leaves a few plain lines on the machine where it ran (#124). When a run does not work, a person reads them and sees why, without the session's transcript.

- **Where:** a new folder `<state>/watchbill/sessions/<time>-<role>-<issue>/`. `<state>` is `$XDG_STATE_HOME` when it is an absolute path, else `~/.local/state`. `<time>` is the stamp's, in UTC, with `:` left out, as `2026-11-19T094959Z`. `<issue>` is `none` for a role started with no issue. A folder that is there is never written over: a second start in the same second gets `-2`, a third `-3`.
- **Said first:** `wb build: will write the lines of this start in <folder>`, on standard error, before it is written.
- **`stamp`:** byte for byte the stamp posted, or the one that would have been posted. A start refused by the checks of `wb prompt` has none.
- **`outcome`:** lines of `name: value`, in this order:

  ```text
  outcome: refused        outcome: not posted     outcome: ended
  check: class            status: 1               status: 3
  status: 1               message: …              seconds: 412
  message: …
  ```

  `check` is the one that refused: `prompt`, `rule`, `class`, `start`, `terminal`, `ci` or `view`. `message` is the first line of what was said. `not posted` is a stamp that was not posted, and no session was started. `ended` gives the session's own status (or `signal:` when a signal ended it) and the seconds it ran. A `wb` killed while the session runs leaves no `outcome`.
- **`ci`,** for a review: `ci: green`, `ci: red` or `ci: pending`, the word `change_checks` printed; `ci: not read` where there is no `change_checks`, and `ci: unread` where it printed something else or failed. It is kept whether or not the review started, so that a count of views that were green and CIs that were not can be made.
- **`view` and `back`,** for a session in a view: the list of its paths, one to a line, whose SHA-256 is the stamp's `view:`; and what came back, a line each: `taken: <commit on the branch> <subject>`, `refused: <commit of the view> <subject>: <why>`, `not taken: …`, `uncommitted: <path>`, `failed: <why>`, and last `view: removed` or `view: kept <folder>`.
- **Not in the way:** a folder that cannot be written is said on standard error. It changes neither the status nor whether the session starts.
- **What writes none:** `--prompt`, `--stamp` and `--parts`, and `wb prompt` and `wb check`. Nor does a command used wrongly (status 2 before the checks begin): no start was tried.
- **What they are:** diagnostics on one machine, not a record ([Not goals](#not-goals)). `wb` never reads them back to decide anything, and nothing in them leaves the machine. They are plain files, so the team's own tools or a recorder can collect them, and a person removes them when they are no longer wanted.

### The patterns

`wb prompt` and `wb <role>` ask the tracker and the git server only through lines of `watchbill.yml`, the team's own commands. For a GitHub `origin`, `wb init` writes them, here for `example/brake-controller`:

```yaml
issue_comment: 'gh issue comment {issue} --repo example/brake-controller --body-file -'
issue_comments: 'gh api repos/example/brake-controller/issues/{issue}/comments --paginate --jq ''.[].body | split("\n")[0]'''
change_comment: 'gh pr comment {change} --repo example/brake-controller --body-file -'
change_comments: 'gh api repos/example/brake-controller/issues/{change}/comments --paginate --jq ''.[].body | split("\n")[0]'''
change_of_branch: 'gh pr list --repo example/brake-controller --head {branch} --state open --json number --jq ''.[].number'''
open_issues: 'gh issue list --repo example/brake-controller --state open --limit 1000 --json number,title --jq ''.[] | "\(.number)\t\(.title | gsub("[\n\r]"; " "))"'''
change_checks: 'gh pr checks {change} --repo example/brake-controller --json bucket --jq ''[.[].bucket] | if length == 0 then "pending" elif any(. == "fail" or . == "cancel") then "red" elif all(. == "pass" or . == "skipping") then "green" else "pending" end'''
issue_comment_bodies: 'gh api repos/example/brake-controller/issues/{issue}/comments --paginate --jq ''.[] | .body + "\u0000"'''
change_comment_bodies: 'gh api repos/example/brake-controller/issues/{change}/comments --paginate --jq ''.[] | .body + "\u0000"'''
draft_change: 'gh pr create --repo example/brake-controller --head {branch} --draft --fill-verbose'
```

- `issue_comment` and `change_comment` post a comment from standard input. `issue_comments` and `change_comments` print the first line of each comment, one to a line. `change_of_branch` prints the numbers of the open changes from `{branch}`, one to a line. At GitHub the comments of a pull request are those of an issue, so `change_comments` reads them the same way.
- `issue_comment_bodies` and `change_comment_bodies` print each comment whole, followed by a NUL byte. One line break straight after a NUL is not read as part of the next comment: `gh --jq` and `jq -r` print one. `wb answer` reads them ([Answering the questions](#answering-the-questions)); no role needs them.
- `draft_change` is the command that opens the change of `{branch}` as a draft, its title and description filled from the commits. `wb build` says it, filled, after a session; it never runs it.
- `open_issues` prints the open issues, one to a line: the number, a tab and the title, with a line break in a title written as a space. For GitHub it stops at 1000 open issues. `wb status` reads it ([Where each issue stands](#where-each-issue-stands)); no role needs it.
- Each is one line for `/bin/sh -c`, run in the root of the checkout, as an agent runs the addresses. The values filled in (`{issue}`, `{change}`, `{branch}`) are plain names, as for the addresses. A value given that is not is refused with 2, by its option. A branch worked out that is not is refused with 1, saying to give `--change`.
- A repository set up before these lines, or with another server, adds them by hand. Without them, `wb prompt` asks for `--change` and `--round` as before, and `wb <role>` is refused, naming the line it needs.
- `wb <role>` asks nothing of the network but through these lines and the runner's `start`.

For a GitLab `origin`, `wb init` writes the same patterns with `glab`, here for `brakes/controller` on `gitlab.example.com`, and `issue_address` and `change_address` besides:

```yaml
issue_comment: 'glab api --hostname gitlab.example.com --method POST projects/brakes%2Fcontroller/issues/{issue}/notes -F body=@-'
issue_comments: 'glab api --hostname gitlab.example.com projects/brakes%2Fcontroller/issues/{issue}/notes --paginate --output ndjson | jq -r ''select(.system == false) | .body | split("\n")[0]'''
change_comment: 'glab api --hostname gitlab.example.com --method POST projects/brakes%2Fcontroller/merge_requests/{change}/notes -F body=@-'
change_comments: 'glab api --hostname gitlab.example.com projects/brakes%2Fcontroller/merge_requests/{change}/notes --paginate --output ndjson | jq -r ''select(.system == false) | .body | split("\n")[0]'''
change_of_branch: 'glab mr list --repo https://gitlab.example.com/brakes/controller --source-branch {branch} -F json --jq ''.[].iid'''
open_issues: 'glab api --hostname gitlab.example.com ''projects/brakes%2Fcontroller/issues?state=opened&per_page=100'' --paginate --output ndjson | jq -r ''"\(.iid)\t\(.title | gsub("[\n\r]"; " "))"'''
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

`change_checks` is read by `wb review` before it posts the stamp (#127). It prints one word for the change, `green`, `red` or `pending`, and ends with 0.

- `wb review` starts on `green`. On `pending` it is refused with 1, saying to wait. On `red` it is refused with 1, saying that the change goes to a person, and that the lists or the stubs of the breakdown may be wrong: the CI builds the whole tree and runs every test, and the session saw only its view. A word that is not one of the three, or a status but 0, is refused with 1, naming `change_checks`.
- The review's prompt gets nothing of it: not the word, and never the CI's log. Only `wb` reads the word.
- For GitHub the word comes from the `bucket` of `gh pr checks`: `red` where a check failed or was cancelled, `pending` where one has not ended or has a bucket that is not known (the most restrictive applies), else `green`: only `pass` and `skipping` are green, so a change whose checks were all skipped is green. A pull request with no check reported, as a change just pushed has before its first check, ends `gh` with 1 and `wb review` is refused, naming `change_checks`: wait, and start it again. A repository with no CI has no `change_checks` line.
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
wb breakdown 54     # writes .watchbill/issues/54.yml on the branch issue-54
wb accept 54        # the person accepts it, and commits the acceptance
wb tester 54        # the tests, red
wb answer 54        # the person answers what a session asked, any time it asked
wb build 54         # the code, committed; wb says how to push it and open the change
wb review 54        # round 1
wb fix 54           # answers round 1
wb review 54        # round 2
```

Each command posts its stamp and starts its session. The person answers the questions with `wb answer`, pushes the branch, opens the change and merges.

### The exit status

- **0**: the session ended with 0; with `--prompt`, `--stamp` or `--parts`, it was printed.
- **1**: it was refused, by `wb prompt`'s checks or by one of the five after them; `watchbill.yml` has no pattern it needs; a pattern ended with anything but 0; the session could not be started, or did not end with 0; or a commit of its view was refused, or the view's work could not be taken back.
- **2**: it was used wrongly, as for `wb prompt`, or with no runner and no `default`, a `--round` that is not a whole number from 1, or an issue or a change to post on that is not plain; the folder is not in a git repository; or Watchbill is not set up.

### What it does not do

- Start a session without a terminal, for the listener (#85).
- Run the breakdown, or a role with no issue's file, in a view.
- Keep the session's last message and hand it on (#88).
- Know whether a session of the same role already runs on the issue (#88).
- Say what the session does with its prompt. The stamp says what was handed out and started.

## Where each issue stands

`wb status` lists the issues in the flow under headings, as `git status` lists files under theirs: each issue under the last step it has done, with its title. It only reads, and it says what is, never what to do. It runs anywhere inside the repository.

```
Not broken down:
  178  howl export takes a range of dates

Broken down:
  176  The page shows how old the ledger is

Accepted:
  175  An event says where its task came from

Reviewed:
  177  A review round is counted the same everywhere

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
- **Which issues, and their titles:** the open issues from the pattern `open_issues` of `watchbill.yml` ([The patterns](#the-patterns)), one to a line: number, a tab, title. `wb init` writes it for a GitHub and a GitLab `origin`. Without it, the issues are those with a branch of the pattern `branch` (local or at `origin`) or a file under `.watchbill/issues/`, with no title. A title is printed with its control characters escaped. A line of `open_issues` whose first field is not a plain issue name is listed under `Not read`.
- **`wb status <issue>`** prints that issue's heading and line and, under them, `Worked out from:` and the lines it was worked out from: its stamps, questions and answers, and the branch and files it found.
- **`wb status --stage <heading>`** prints only that heading, for example `wb status --stage "not broken down"`. The names are matched without regard to case; `Questions open` and `Not read` too. A name that is not known is used wrongly.
- **What it reads:** the issue's file and its acceptance from the branch with `git show`, never by switching branches, and the comments through `issue_comments`. It writes nothing, and asks the network only through `open_issues` and `issue_comments`.
- **The exit status:** 0 when every issue could be read; 1 when a pattern failed for an issue (it is listed last, under `Not read`, with why; the others are printed), or `open_issues` itself failed; 2 when it was used wrongly or Watchbill is not set up.
- **It is not a dashboard** ([Not goals](#not-goals)): a list in the terminal, of now, for one repository. The sessions that run on this machine are for #140.

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
- **Who reads it:** the round that `wb <role>` and `wb prompt` work out ([the round and the change](#the-round-and-the-change)), the check that a fix follows a review, the step and the open questions of `wb status`, and the questions `wb answer` shows. A marker is read only from the first line of a comment, as a stamp is, so a marker quoted in a comment's text counts for nothing, and so does one with no such stamp before it, and so does a `[revert] breakdown` posted by hand. `wb status <issue>` marks the lines a marker took back in what it was worked out from.
- **A change with no issue:** `wb revert --change <n> <role>` posts the marker on the change, through `change_comment`, and reads the stamps from `change_comments`.
- **What it does not touch:** the branch, the issue's file and its acceptance, and the change. The commits of the steps taken back stay on the branch, and taking them back is git's; `wb revert` says so on standard error.
- **A later breakdown:** when a `[stamp] breakdown` stands after the stamp to be reverted, that stamp no longer counts, since the new breakdown took the issue back ([where each issue stands](#where-each-issue-stands)). `wb revert` is then refused with 1, saying so, and posts nothing.
- **The breakdown** is not taken back by a marker. The issue's file and its acceptance are files of the repository, and git takes them back. `wb revert <issue> breakdown` is refused with 1, and says so.
- **Refused with 1:** a role with no stamp on the issue that still counts (naming the role), a missing `issue_comments` or `issue_comment` (or the change's), a pattern that ends with anything but 0.
- **The exit status:** 0 when the marker is posted; 1 when it was refused or the posting failed; 2 when it was used wrongly (no issue or role, a role the revision does not have, an option that is not known, a `--change` that is not a plain name, `--round` that is not a whole number from 1, an issue and `--change` together) or Watchbill is not set up.
- **Not yet:** who may revert once the roles have accounts (#83). Until then any person who can post a comment can.

## Taking it on one role at a time

A team takes Watchbill on one role at a time (#116). A person can start any role alone, with no step before it: no breakdown, no issue's file, and for a review or a fix, no issue.

The first step is reviews only. A person writes a change as before, checks out its branch, and has it reviewed by a fresh session:

```sh
git switch my-change            # the change's own branch
wb review --change 174          # posts the stamp on the change and starts the review
```

- The change's description is the spec. The review reads it at `change_address`, and says so in a finding when it is empty.
- `requirements` is "what the description of change 174 says the change does", unless `--requirements` is given.
- The stamp's `issue:` is empty. `wb` posts it on the change by `change_comment`, with the first line `[stamp] review round 1`, and the round after it from the stamps on the change.
- The branch is the one checked out, unless it is the one `origin/HEAD` names. Otherwise give `--branch`.
- With no issue's file, the session may change paths that are not known, so the review starts only with `classed: []` in `watchbill.yml` ([Classed code](#classed-code)).
- For a runner `wb` cannot start, `wb prompt review --change 174 --stamp` prints the stamp to post by hand, and `wb prompt review --change 174` the prompt.

Then the team adds the rest when it is ready:

- **The fix,** in the same way: `wb fix --change 174`.
- **Issues with no file:** the tester, the build, the review and the fix are handed out for an issue, and work from its own numbered items.
- **The breakdown and the acceptance:** once an issue has a file, the chain's gates bind. The file must be accepted, and the file and its acceptance committed, before any role of the issue is handed out.
- **The hand-on** from one role to the next always needs the whole chain: an accepted file for every issue. It is not built yet (#88, and the listener and the officer, #85 and #86).

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

The commit is made by `wb accept`, alone, so the history holds the acceptance before any work it allows. Only that one file is committed: whatever else is staged stays staged. It is made as the `user.name` and `user.email` of git, on the issue's branch. Before it writes anything, `wb accept` refuses with 1 when the checkout is not on that branch (the pattern `branch` of `watchbill.yml`, naming both), or when the issue's file is not committed in `HEAD` or differs from it: the acceptance holds for the file as the history has it. When the commit fails (a hook, a signing key), the acceptance stays written, `wb accept` says so and ends with 1, and the person commits it with the command it prints. `wb` pushes nothing. `wb accept --no-commit` keeps the form of 0.0.9: it writes the acceptance and prints the command, and makes none of those refusals.

`by` is the `user.name` and `user.email` of git, and `time` is UTC. Without both, `wb accept` refuses. An acceptance that is there is never written over: remove it first. `wb` writes nothing else, and asks nothing of the network; `needs` is the team's own command.

An issue's file counts as accepted only while the SHA-256 in its acceptance is that of the file as it is. The function `watchbill::issue::is_accepted` answers that. One byte changed, and it is not accepted: it is accepted again by removing the `.accepted` and running `wb accept`.

What acceptance cannot stop: the hash stops a change after the acceptance, not a false acceptance. Where a person and the agents share one account, an agent can run `wb accept` itself, or write the `sha256:` line by hand to match a file it changed ([docs/revision.md](docs/revision.md), point 13). Only accounts of their own for the agents stop that, with the person's `[accepted]` comment on the issue (point 13 and point 14).

`wb accept` never follows a link: `.watchbill` or `.watchbill/issues` that is a link, or is not a folder, is refused by name, and nothing is read or written through it. The acceptance is written whole or not at all, to a file of its own in the same folder and then linked to its name. An issue is named by at most 64 letters, digits, `.`, `_` and `-`. A path of the file that names the root (`.`, `./`) is refused. What `wb accept` prints of the file has its control characters escaped. A git name or email that holds `<` or `>` is refused.

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

The package is built here from a checkout, and dated by the last commit of it: two builds of one commit into the same result folder (the argument of `rpm.sh`, `build/rpm` by default) give the same package. Another folder gives another `wb`, as the path of the build is in it. `rpm.sh` needs a git checkout and stops with 2 outside one. There is no repository to `dnf install watchbill` from yet; the RPMs of each version are on its release (below). Building needs `rpm-build`, `git-core`, and the Rust of the distribution: `cargo`, `rust` (1.97 or later), `cargo-rpm-macros` and the crates `yaml-rust2`, `sha2` and `lexopt` (`rust-yaml-rust2-devel`, `rust-sha2-devel`, `rust-lexopt-devel`). On Enterprise Linux 9 it also needs **EPEL**: `cargo-rpm-macros` and the crates are only there.

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
