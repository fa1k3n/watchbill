<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/logo-dark.svg">
    <img src="docs/logo-light.svg" width="96" height="96" alt="The mark of Watchbill: a grid of three by three rounded squares, the diagonal filled in another colour">
  </picture>
</p>

# Watchbill

A harness for AI agents in embedded software development, deployed at a fixed revision onto a project that already exists.

**Status: an idea, written down 2026-11-16. Built: `wb init`, `wb check`, `wb prompt`, `wb accept` and `wb revision`. Nothing hands work from one role to the next yet, and `wb` starts no session.**

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

At 0.0.8. An issue that changes a verdict changes its row.

| Goal | Today | What will hold it |
|---|---|---|
| 1. Fits the workflow | Partly: any tracker and git server with a command line, by a pattern the team writes. `wb init` writes the patterns for GitHub only. The CI and the requirements tool are names and nothing more. `wb` installs on Fedora and Enterprise Linux only | No issue yet for GitLab's patterns |
| 2. What the agents are told | Held for what `wb` hands out: the pinned revision, `wb check`. That a session was given that prompt is the word of the one who started it | The starter, #84 |
| 3. Rules enforced | Partly: the pinned revision, the acceptance, the breakdown's runner and the placeholders are checked in code. No limit of the rule table is | #28 names each limit and what holds it; then #65, #88 and the pieces #28 writes |
| 4. People keep the decisions | Partly: `wb accept` asks for a person's word, but cannot tell a person from an agent under one account. That an agent cannot merge rests on the git server's settings | The roles' accounts, #83, and the acceptance read at the tracker, #88. No issue yet checks that a role's account cannot merge |
| 5. Duties separated | Partly: the rule table says it, and nothing knows who holds which role | #83 and #88, for work that is handed on. No issue yet for a session a person starts |
| 6. Sees only the code it needs | Not yet: the lists are in the prompt only | The sandbox, #65 |
| 7. Code goes only where allowed | Partly: checked for the breakdown | The other roles, #11. No issue yet for a local model for them |
| 8. Starts small | Partly: `wb prompt` hands out any role alone, for an issue with no file, and a review or a fix of a change with no issue ([one role at a time](#taking-it-on-one-role-at-a-time)). The person starts the session and posts the stamp by hand | The starter, #84, posts the stamp on the issue or the change and starts the session |
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
```

- **`watchbill` is the version of the package** the repository is set up for: the version of the `wb` that wrote the file. A revision is the version and the hash of its files together ([docs/revision.md](docs/revision.md)); the hash is the line `revision`, and is the one `wb revision` prints.
- **What it finds, it finds in the repository:** the name of the folder, the address of `origin` (only its host and path; a user name or a token in it is dropped), a folder of requirements, and the file a CI driver is known by. It asks nothing of the network.
- **The revision is copied, never edited.** The files under `.watchbill/revision/` are written by `wb` and never by hand. `cd .watchbill && sha256sum -c revision.sha256` checks them without `wb`.
- **A file that is already there is never changed.** When any file `wb init` would write is there, or a folder on the way to one is a link or not a folder, or `.watchbill/revision` is there, it writes nothing at all and names the path. Bringing a repository to a newer revision is a command of its own, later. A write that fails part way removes what it had written.
- **The project's own text goes in `.watchbill/project/`.** `all.md` is added to the prompt of every role and `<role>.md` (`build.md`, `review.md`, `fix.md`, `tester.md`) to that of one role. Both are optional. They are the project's files: not part of the revision, not in the manifest, and `wb check` does not fail on them, except for a placeholder that the revision does not know (see `wb check`). A prompt is the role's text of the revision, then one fixed line, then `all.md`, then `<role>.md`. This text says how things are done here: the commands, the board, the words. It never lifts a limit of a role. `wb init` leaves `project/README.md`, which says so. The placeholders of `revision/roles/placeholders.md` (`{issue}`, `{branch}` and the others) are filled in every part, and any other placeholder is refused. A placeholder is a lower-case word in braces: `{`, a letter a to z, letters a to z or `_`, then `}`, with no `$` just before it. Anything else in braces is text and stays as it is, such as `${BUILD_DIR}`, `{Issue}`, `{issue-id}` and `{ issue }`. A value that is filled in is never itself read for placeholders. The command that prints a prompt comes later.
- **The answers are only written down.** No adapter exists yet, so a git server or a tracker is a name and nothing more.

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

`wb prompt <role>` prints the prompt of a role on standard output, and nothing else. Whatever starts an agent's session, in any language, calls it as a process. Run it anywhere inside the repository. It writes nothing and asks nothing of the network.

The prompt is the role's text of the pinned revision, then the project's own text (`.watchbill/project/all.md` and `.watchbill/project/<role>.md`, where they are). A call asks only for what `wb` cannot know on the machine: the issue, and for the review and the fix the change and the round. The change and the round live at the git server, and `wb` asks nothing of the network.

```sh
wb prompt tester 54
wb prompt build 54
wb prompt review 54 --change 168 --round 1
wb prompt fix 54 --change 168 --round 1
wb prompt review --change 174 --round 1             # a change with no issue
```

The issue is the word after the role, or `--issue`; both together end with 2. The tester and the build with no issue end with 2 and the usage. Only the review and the fix take a change alone, with no issue (see [Taking it on one role at a time](#taking-it-on-one-role-at-a-time)). The placeholders are filled from the options, one for each, and what no option gives is derived. An option given always wins over the derived value, and the stamp holds each value as it was handed out:

- `--issue <text>`, `--change <text>`, `--requirements <text>`, `--round <text>`, `--checkout <text>`, `--branch <text>`, `--issue_address <text>`, `--change_address <text>`. What each is, is in `.watchbill/revision/roles/placeholders.md`.
- `checkout` is the root of the repository `wb` runs in.
- `issue_address` and `change_address` are the patterns of those names in `.watchbill/watchbill.yml`, with `{issue}` and `{change}` filled in. `wb init` proposes them for a GitHub `origin` only (`gh issue view {issue} --repo <repository>; gh issue view {issue} --repo <repository> --comments`, `gh pr view {change} --repo <repository>; gh pr view {change} --repo <repository> --comments`: each shows the text or the description first, then the comments, as `--comments` alone hides the first). For another server, or none, there is no pattern and the option must be given.
- `branch` is the pattern of that name in `watchbill.yml`, proposed as `issue-{issue}`, with `{issue}` filled in. The breakdown makes the branch, so it gets the pattern as it is. For the tester, the build, the review and the fix it is the one local branch (from the refs of the repository) whose name is the pattern, or starts with it and a `-`. None, or more than one, is refused with 1, the names found are said on standard error, and `--branch` must be given.
- For a review or a fix of a change with no issue, `branch` is the branch checked out. `wb` asks nothing of the network, so it cannot see which branch the change is from: it refuses, with 2 and the words to give `--branch`, a HEAD on no branch, the branch that `origin/HEAD` names (where changes go), and any branch where `origin/HEAD` is not there.
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
- `--stamp` prints, in place of the text, the stamp of the assignment: what was handed out ([docs/revision.md](docs/revision.md), point 12). It is lines of `name: value` in this order: `role`, `model`, `effort`, `runner`, `version`, `revision`, `issue`, `change`, `round`, `branch`, `checkout`, `time`, then one line `part: <sha256>  <path>` for each part. `model` and `effort` are those of the model table for the role, and for the breakdown those its runner is cleared with. `runner` is the value of `--runner`, and empty when none is given. `version` and `revision` are those of the pinned revision. `time` is UTC, as `2026-11-17T18:04:05Z`. A value that is empty, or that the prompt of the role does not hold (`--change` for `build`), is written as its name and the colon alone: it was not handed out. A value of `--issue`, `--change`, `--round`, `--branch` or `--checkout` that holds a control character, a line or paragraph separator, or white space at its start or end is refused by name, as one field must stay one line that a comment can trim. `--stamp` with `--parts`, and either given twice, are used wrongly.

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
- **1**: it was refused and nothing is printed: the breakdown's runner is not cleared for it (see [Confidentiality](#confidentiality)), the check does not hold, the clock reads before 1970 (`--stamp`), the role is not one of the revision (the roles it has are named), a project file is not UTF-8, a placeholder is not known or got no value, the branch is not found or not one, the issue's file is there and is not accepted (or changed since), `watchbill.yml` holds `task_address`, `{task}` in a pattern or the key `breakdown`, or a value derived for the stamp (the checkout, the branch) is not one plain line.
- **2**: `wb prompt` was used wrongly (no role, a third word or an option that is not known, the issue given as a word and as `--issue`, an option given twice, with no value or with an empty one, `--stamp` with `--parts`, an option given twice, a stamped value that is not one plain line, an issue or change that fills a pattern and is not plain, the breakdown with no `--runner`, the tester or the build with no issue, a review or a fix with no issue whose branch is not given and is not the branch checked out), the folder is not in a git repository, or Watchbill is not set up. The usage or the reason is on standard error.

The one who starts a session posts its stamp on the issue, one comment for each session, the first line `[stamp] <role> round <n>`, and the lines of the stamp after it. A review or a fix with no issue has its stamp posted on the change. Where a session is started by hand, the stamp is posted as it was printed. A session with no stamp on its issue, or on its change where it has no issue, is not taken as the work of a role.

```
[stamp] build round 1
role: build
model: claude-sonnet-5-5
effort: medium
runner:
version: 0.0.8
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

Four lines of shell that start a session with it, and record what the session was given. The files are written to a folder of their own and moved only when both calls succeed, so a failed call leaves none:

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
```

- `levels` is at least one name, each once, from the least to the most restrictive.
- A runner has `tools`, `model`, `agreement` and `up_to`, which is one of the levels. `breakdown` is there only when the runner is cleared for the breakdown: its `up_to` is no higher than the runner's own, its `model` is an exact model ID and its `effort` is one of the model table's.
- A key that is not known, at any depth, a level written twice, a field missing or a value of the wrong type is refused, with the file and the place named.

The repository states its own level, as the line `confidentiality:` of `.watchbill/watchbill.yml`. Where the line is missing, the level is the most restrictive of the list. A level that is not on the list is refused by name, and so are `levels` and `runners` in `watchbill.yml`: they are the organisation's.

`wb prompt breakdown <issue> --runner <name>` hands out the prompt only when the organisation's file is there, names the runner, the runner has `breakdown`, and the repository's level is no higher than the breakdown's `up_to`. Otherwise it prints nothing on standard output, names on standard error the file it looked for, the runner, or the level and the `up_to`, and ends with 1. With no `--runner` the breakdown ends with 2 and the usage. Its stamp has `runner: <name>` and the `model` and `effort` of the runner's `breakdown`, and `--parts` adds two lines after the parts: `level: <the repository's level>` and `runner: <name>`.

What it does not do yet (#11): the tester, the build, the review and the fix are not refused on a runner or a missing file, and the level of an issue is not read. It cannot stop a person from pasting code into another tool, or running a session with a prompt that `wb` did not hand out. The stamp on the issue is what shows where the work was handed out.

## Taking it on one role at a time

A team takes Watchbill on one role at a time (#116). A person can start any role alone, with no step before it: no breakdown, no issue's file, and for a review or a fix, no issue.

The first step is reviews only. A person writes a change as before, checks out its branch, and has it reviewed by a fresh session:

```sh
git switch my-change                              # the change's own branch
wb prompt review --change 174 --round 1 --stamp   # what is handed out: post it on the change
wb prompt review --change 174 --round 1           # the prompt, for the session the person starts
```

- The change's description is the spec. The review reads it at `change_address`, and says so in a finding when it is empty.
- `requirements` is "what the description of change 174 says the change does", unless `--requirements` is given.
- The stamp's `issue:` is empty, and the person posts it on the change, with the first line `[stamp] review round 1`.
- The branch is the one checked out, unless it is the one `origin/HEAD` names. Otherwise give `--branch`.

Then the team adds the rest when it is ready:

- **The fix,** in the same way: `wb prompt fix --change 174 --round 1`.
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

`wb accept <issue>` accepts the file. It reads it, names the first fault of form, and refuses while `questions` holds anything, naming them. Then it says what it will write, and writes `.watchbill/issues/<issue>.accepted`:

```
$ wb accept 54
wb accept: will write .watchbill/issues/54.accepted:
sha256: 5d41402abc4b2a76b9719d911017c592ae0a1b3c1e2f8d3b0a7c9e4f6d2b8a11
by: Ada Lovelace <ada@example.org>
time: 2026-11-18T09:30:00Z

wb accept: .watchbill/issues/54.yml is accepted.
wb accept: commit the acceptance alone, so the history holds it before any work it allows:
  git add .watchbill/issues/54.accepted && git commit -m "Accept the issue's file (54)" -- .watchbill/issues/54.accepted
```

`by` is the `user.name` and `user.email` of git, and `time` is UTC. Without both, `wb accept` refuses. An acceptance that is there is never written over: remove it first. Nothing else is written, and the network is not asked.

An issue's file counts as accepted only while the SHA-256 in its acceptance is that of the file as it is. The function `watchbill::issue::is_accepted` answers that. One byte changed, and it is not accepted: it is accepted again by removing the `.accepted` and running `wb accept`.

What acceptance cannot stop: the hash stops a change after the acceptance, not a false acceptance. Where a person and the agents share one account, an agent can run `wb accept` itself, or write the `sha256:` line by hand to match a file it changed ([docs/revision.md](docs/revision.md), point 13). Only accounts of their own for the agents stop that, with the person's `[accepted]` comment on the issue (point 13 and point 14).

`wb accept` never follows a link: `.watchbill` or `.watchbill/issues` that is a link, or is not a folder, is refused by name, and nothing is read or written through it. The acceptance is written whole or not at all, to a file of its own in the same folder and then linked to its name. An issue is named by at most 64 letters, digits, `.`, `_` and `-`. A path of the file that names the root (`.`, `./`) is refused. What `wb accept` prints of the file has its control characters escaped. A git name or email that holds `<` or `>` is refused.

The exit status:

- **0**: the file is accepted.
- **1**: it is refused: no file, a fault of form, a question, an acceptance that is there, no git identity.
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
