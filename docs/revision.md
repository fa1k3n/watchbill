# What a revision contains

A revision is what a project is set up for and what its agents run under: the same on every machine, and checkable by anyone. This page says exactly what it is. Each point was settled with the owner on 2026-11-16 (#3). Points 1 and 6 were changed on 2026-11-17, when the owner decided that `wb` is written in Rust and not in Python (#20). Nothing else on the page changed: the files of a revision and their hash are the same. Point 14 and the changes to points 10, 12 and 13 on accounts were settled with the owner on 2026-11-18 (#30), and are in revision 0.0.7 (#82). That a person can start any role alone was settled with the owner on 2026-11-19, and changed points 12, 13 and 14 (#116, revision 0.0.8). That every role asks on the issue, plainly and with a suggested answer, changed points 12 and 13 (#113, revision 0.0.8). That `wb accept` checks the lists by the project's `needs` and keeps the build out of `changes` changed point 13 (#125, revision 0.0.9). That a session sees only its files, in a view, changed points 3 and 12 (#126, revision 0.0.9). That a session asks in one comment, a person answers with `wb answer`, and a session in a view commits and a person pushes, changed points 3 and 13 (#145, revision 0.0.9).

## In short

- A revision is a version of the package and the hash of its files, both written.
- The files are the role prompts, the rule table and the model table.
- `wb init` copies them into the repository, and `wb check` fails on any difference.
- The levels of confidentiality and the runners are the organisation's and are not part of the revision.
- Each role works under an account of its own, and a handover is an assignment at the tracker.

## 1. The files of a revision

The role prompts, the rule table and the model table. The code of `wb` is named by the version of the package, the files by their hash, and a revision is both.

In the installed package, under `/usr/share/watchbill/`:

| Path | Holds |
|---|---|
| `revision/roles/<role>.md` | The prompt of a role |
| `revision/roles/placeholders.md` | The words a prompt is filled in with |
| `revision/rules.yml` | The rule table |
| `revision/models.yml` | The model table |
| `revision.sha256` | The manifest: the hash of every file above |
| `revisions.txt` | Each version of the package that was released, with the hash of its revision |

In a checkout the same files are under `share/watchbill/`.

- The folder is fixed when `wb` is built. No option and no environment variable changes it when `wb` runs, so nobody points `wb` at another revision.
- The code that reads them is `src/manifest.rs` (files, hashes, manifest) and `src/rules.rs` (the two tables).

## 2. The roles

Breakdown, tester, build, review and fix. The first revision had the last four. The breakdown has its prompt and its rows from revision 0.0.5 (#75). It comes before the tester, and once an issue has a file, no other role of it is started until a person has accepted the file (point 13). A person may start any role alone, with no file (#116). The watch officer has rows in the rule table and no prompt yet (point 10). The architect comes later.

- **Breakdown:** reads a person's issue and the code, and writes the issue's file (point 13). It never changes the issue, the code or a test, and never answers what the issue leaves open. It is the one role that reads the code without a list. It holds no other role on the issue.
- **Tester:** writes the tests from the requirement and never reads the implementation.
- **Build:** makes the tests pass and never changes them.
- **Review:** a fresh session that reports findings, clause by clause, and never fixes.
- **Fix:** answers each finding with a fix or a reason. It is neither the builder nor the reviewer.
- An agent is a name without a personality. Any agent can hold any role; the role, the model and the effort belong to the assignment.
- Review starts with one lens. More lenses are versioned checklists under the review role, added when figures show what one lens misses.

## 3. What the rule table says

For each role:

- the step it performs and what it produces;
- what it may read;
- what it may do;
- the roles the same team member may not also hold on the same change;
- its prompt file, or `none` when no agent holds the role yet.

For the whole table:

- at most two review rounds;
- how many times a session is asked for its last message, with the fields its prompt names, before it goes to a person (#45);
- what always goes to a person;
- the roles a requirement that a person adds to a change under review goes to, before the next round (#49);
- the rule for classed code (point 4).

The program enforces the table. A prompt repeats a limit so the agent knows it, but no limit rests on the prompt alone.

What a session may read and change is the program's to hold for a session that `wb start <role>` starts for an issue with an accepted file: it runs in a view, in bubblewrap, with only the paths of its lists and no history, and its commits come back only where `changes` allows (#126). For a session a person starts by hand, or one with no issue's file, that limit rests on the prompt alone. Decided by the owner on 2026-11-19 (#65), in revision 0.0.9. A view has no remote, so the tester, the build and the fix commit and push nothing, and open no change: `wb` says the command that pushes the branch and, after a build, the one that opens the change as a draft, and a person runs them. The breakdown, which runs without a view, still pushes. Decided by the owner on 2026-11-20 (#145); that `wb` pushes itself is #151.

## 4. Code classes

One rule in the first revision: a change that touches classed code needs a person as its verifier. Which paths are classed is the project's to say: the list `classed` of `watchbill.yml`, and every path where it says none (#84). `wb init` asks for the list and proposes none: `classed: []`. Levels per class come later.

## 5. The model table

One exact model ID and one effort for each role that has a prompt, but the breakdown. Never "latest", and never a name without a version.

| Role | Model | Effort |
|---|---|---|
| Tester | `claude-sonnet-5-5` | medium |
| Build | `claude-sonnet-5-5` | medium |
| Fix | `claude-sonnet-5-5` | medium |
| Review | `claude-opus-5-5` | high |

Where an issue or the project names another model or effort than the table, the table wins. An issue may ask for a stronger model or a higher effort, never for less, and the stamp of #31 records what ran (#51).

Figures on the findings of the first runs correct it. A changed model is a new revision. The table is for a vendor's runner. The models of local runners have a place of their own in the table, `local_models`, empty until the first one exists (point 11). A table with anything there is refused until then.

The breakdown has no line for a vendor's runner, and a table that gives it one is refused. It reads the code without a list, so it sends a model more of the code than any other role. It runs on a local runner, or on another runner only where a person has cleared that runner for the breakdown in the organisation's file (point 11). There is no word per issue for it. The organisation clears a runner once, in its file, up to a level, with the model and the effort it runs with; `wb prompt breakdown --runner <name>` refuses a runner that is not cleared for the level of the repository, and its stamp names the runner, the model and the effort (#93).

## 6. The format of the tables

YAML, read through a library: the crate `yaml-rust2`, as the distribution packages it. The owner chose a library over JSON and over a reader of Watchbill's own.

What follows from it:

- `wb` is built from Rust's standard library and the crates that `CLAUDE.md` lists: `yaml-rust2`, `sha2` for the hashes of point 8, and `lexopt` for the command line. A crate is taken where it does most of a job `wb` needs (#34). Fedora packages each in its own repositories. Enterprise Linux 9 packages Rust but no crates, so there they come from EPEL. Nothing is fetched from crates.io, and a build asks nothing of the network.
- The crates are built into `wb`. Installed, it needs `git` and no library of its own.
- One loader in `wb` reads every YAML file, and nothing else uses the library:
  - it reads the events the library gives and builds only plain values: texts, whole numbers, lists and tables;
  - a key written twice is refused;
  - a tag, a merge key and an alias are refused;
  - it decides itself what a plain value is, so the answer does not rest on which version of YAML the library follows;
  - a value is held to the type its field asks for. Where a text is expected and the value is written plain and could be read as something else (`no`, `1.10`, a date), the file is refused and the message says to put the value in quotes.
- `.watchbill/watchbill.yml` and the organisation's file (point 11) are read with the same loader.
- A file is hashed as its bytes. Nothing is rewritten before hashing, so the format does not touch the hash.

## 7. Where the files live in a repository

`wb init` copies them into the repository:

| Path | Holds |
|---|---|
| `.watchbill/watchbill.yml` | The project's answers, the version (`watchbill:`) and the hash of the revision (`revision:`) |
| `.watchbill/revision/` | The files of the revision, byte for byte as the package has them |
| `.watchbill/revision.sha256` | The manifest |
| `.watchbill/project/` | The project's own text for its agents. Not part of the revision |
| `.watchbill/issues/` | The file of each issue and its acceptance (point 13). Not part of the revision |

The team and an assessor then read exactly what the agents are told, and a clone carries its revision. The files under `.watchbill/revision/` are written by `wb` and never by hand.

## 8. The fixed form that is hashed

- Each file is hashed as the bytes that are shipped: UTF-8, LF line endings, one line break at the end. A file in another form is refused by name.
- The hash is SHA-256.
- The manifest has one line per file, in the form `sha256sum` prints, sorted by path. Paths are relative to the folder that holds `revision/`.
- The hash of the revision is the SHA-256 of the manifest.

So the manifest is checked without `wb`:

```sh
cd .watchbill && sha256sum -c revision.sha256
```

## 9. What a change means

Any change to a file of the revision is a new version of the package.

- The manifest is committed, and a test fails when it is not the one computed from the files.
- `revisions.txt` holds each released version with its hash. A test fails when the current version stands there with another hash, so a prompt cannot change under a version that is already out.

## 10. The watch officer

The role that is given issues and hands them on. The captain is the person, who decides what is built.

- **The program** decides all that the rule table can decide (who is free, who may hold which role on this change, the order, the rounds, what goes to a person) and performs every handover.
- **The officer** is a model's role for what takes judgement: whether an issue is clear enough to start, how it is broken into pieces, the brief for the one who gets it, and what to do with a piece that is stuck. Which paths and which class a piece touches are not the officer's: the breakdown says them (#64).
- The officer reads only the issue and the tracker, as the rule table says.
- The officer proposes and the program checks. A handover that breaks a rule is refused, whoever proposed it.
- The officer holds no other role on an issue it has handed out, and never merges. It has an account of its own, so every handover shows at the tracker as an assignment from it (point 14).
- Taking an issue that nobody assigned is off, unless a rule the captain approved names the queue it may take from, with a cap on the work in progress, and never for classed code. The first table names no queue.
- In the first revision the officer has its rows in the rule table. Its prompt is written after the first run of the other four roles.

## 11. Confidentiality

A level that decides where an assignment may run. It is a second axis beside the code class: the class decides who must verify, the level decides where the work may run.

- **The levels are the organisation's:** one ordered list of names for every project, from the least to the most restrictive (for example internal, confidential, strictly confidential). Watchbill ships no list of its own and gives the names no meaning beyond their order.
- **A place to run is a runner**, and a runner is three things: where the agent's tools run, which model answers, and under which agreement (region, retention, use for training). Code leaves the premises through the model call also when the tools run on this machine, so the model and its agreement are part of the place.
- **The organisation names the runners** and, for each, the highest level it may be given. Levels and runners are in one file outside every repository: `/etc/watchbill/organisation.yml`. It is laid out by the organisation's own means and is not part of the revision. The path is fixed in `wb`: no option and no environment variable changes it, so that a team cannot point `wb` at a file of its own (#93).
- **The breakdown is cleared once,** in the organisation's file, up to a level: a runner may have `breakdown`, with the highest level of an assignment it may break down, no higher than the runner's own, and the exact model and the effort it runs with. Nobody clears the breakdown by hand on each run. The other roles are checked with #11.
- **A project states only its own level.** It can be stricter than the organisation, never freer, and a team cannot clear a runner for itself.
- **With no organisation's file, no AI is allowed.** `wb` hands out no prompt on that machine and names the file it looked for. Nothing is assumed about where work may run, not even this machine with a local model.
- **The level of an assignment** is the higher of the level of its issue and the level of everything the session can read. A session reads the whole checkout, so until a runner can limit that, the second is the level of the repository.
- **The program refuses, never a model.** No level given means the most restrictive. A person sets or lowers a level; an agent may only raise one.
- **What it does not do:** stop a person from pasting code into another tool. The network rules of a runner do that. Watchbill refuses the handover.

## 12. The stamp

What a session ran as is said by the one who started it, not by the session. Settled with the owner on 2026-11-17 (#31), after two runs in which every session gave its effort as `unknown`.

- **What is stamped:** the role; the model ID and the effort of the model table; the runner; the version of the package and the hash of the revision; the issue, the change and the round; the branch and the checkout; the time; the view, as the hash of its list of paths, or `none` (#126); and the hash of each part of the prompt.
- **What it means:** what was handed out. `wb prompt` knows what the table asks for and what the prompt holds. It cannot see what the session truly runs as. The one who starts the session starts it so. `wb start <role>` does both in one step (#84), and closes the gap for a runner whose `start` sets the model and the effort.
- **Who stamps:** the program that hands out the prompt: `wb prompt --stamp` (#68), and `wb start <role>`, which posts it by `issue_comment` before it starts the session (#84). A person who starts a session by hand posts the stamp as it was printed. The stamp is posted by the identity of the starter, never by the session's (point 14).
- **Its form:** lines of `name: value` in a fixed order, the same on every machine, with one line for each part of the prompt in the form `sha256sum` prints. A program can read it and `sha256sum` can check its parts.
- **Where it is kept:** on the issue, one comment per session, whose first line starts with `[stamp] <role> round <n>`. The tester and the build run before there is a change, so the issue is the one place that holds every session. #30 may move it later. A review or a fix of a change with no issue has its stamp on the change, and its line `issue:` is empty (#116).
- **The agent's own word:** `model` and `effort` stay in the last message, as what the session says of itself (#69). Where they differ from the stamp, that is a finding for a person, and the stamp is what counts. `unknown` is written down, not a finding: a session often cannot see its own effort. So every role prompt says that its model and effort are in its stamp, the latest `[stamp] <role>` comment, and that the last message gives them from there (#113).
- **What it is not:** a record kept for an assessment. The stamp is working state: the program reads it to know what was handed out. Keeping a record is for the team's own tools or a recorder (the README, "Not goals").
- **A copy on the machine:** `wb start <role>` also writes the stamp, posted or as it would have been, with how the start ended, in a folder of its own on the machine where it ran (#124). They are diagnostics: `wb` never reads them back (the README, "The lines it leaves").
- **A session with no stamp:** its work is not taken as the work of a role, and nothing is handed on from it. Until a program hands work on (#66, the officer), a person keeps this rule: no stamp on the issue, no next role.

## 13. The issue's file

A person's issue, read into a form `wb` reads. It was called the task's file until revision 0.0.6 (#78). Settled with the owner on 2026-11-18 (#64), after two runs in which every issue had to be made ready by hand before the tester could start.

- **Why it is apart from the issue:** the issue is what the person asked for. The issue's file is a reading of it, and a reading is a claim until a person accepts it. The breakdown never changes the issue.
- **Where it lives:** `.watchbill/issues/<issue>.yml`, in the repository. The breakdown commits it first on the issue's branch, alone. It is reviewed and merged with the change, so the history keeps what each issue was read as.
- **What it holds:** the name of the issue; the SHA-256 of the issue's text it was read from; the requirements, each with an ID, its text and where in the issue it comes from, and for a clause that already holds the existing test that guards it; the interface the tests call; the paths the change may change; what each role may read beside them; the existing tests the change changes; and the questions the issue leaves open. The form is in #74.
- **What a requirement is without a table:** a point of the issue's file. Its ID is the issue's number and the point, `54.1`. A test is named after it, a finding points at it, and it can be followed both ways. A project with a table of requirements gives the row's ID instead.
- **The questions are asked on the issue:** the breakdown posts each question as its own comment, whose first line is `[question] <issue>.q<n>`. A person answers there, in a comment whose first line is `[answer] <issue>.q<n>`, or in the question's thread. A second breakdown session, a fresh one, reads the answers, folds them into the file, and removes the questions answered: the file follows the answers. The comments are not part of the hash under `from`, so a change of the issue's text still means a new breakdown. Settled with the owner on 2026-11-18 (#95).
- **Every role asks on the issue:** the tester, the build, the review and the fix post each point of `needs_person` as its own comment, whose first line is `[question] <issue>.<role>.q<n>`, and a person answers with `[answer] <issue>.<role>.q<n>`. A question says, in plain words, what is unclear, why it matters, the choices numbered from 1, and a suggested answer, marked as one, that names its choice by number. It is never edited once posted. An answer is a choice by its number, or words; `[answer] <issue> suggested` takes the suggestion of every open question. An answer that changes the issue's file is taken by a person, who changes the file and accepts it again; any other answer is taken by the next session of the role. A review or a fix of a change with no issue asks on the change. Since revision 0.0.8 (#113), after a run in which the tester's questions were found only in its last message.
- **The questions folded, the answers in the terminal:** from revision 0.0.9 every role, the breakdown too, posts its questions in one comment for the session, whose first line is `[questions] <issue>.<role> q1 q2` (`[questions] <issue> q1` for the breakdown, whose questions keep their names `<issue>.q<n>`). Each question is one line, `q<n>: <what is unclear>`, and the rest is folded under `<details>`, with a line `changes the issue's file: yes` or `no`. The numbers count within the role on the issue. A person answers with `wb answer`, which shows each question in full and posts one comment per role, `[answers] <issue>.<role> q1 q2`, with a line `q<n>: <answer>` each. The forms of 0.0.8 are still read, so an issue begun under it reads right. Asked for by the owner after the trial of 0.0.8 (#145), in which seven comments of questions cluttered the issue and the owner's answer was in a form `wb status` did not read.
- **How a person accepts it:** `wb accept <issue>` writes `.watchbill/issues/<issue>.accepted`, with the SHA-256 of the issue's file, the person's git identity and the time. It refuses while the file holds a question. It never writes over an acceptance that is there: to accept again, the person removes the old one. Then it commits the acceptance alone, on the issue's branch, as that identity; it refuses before it writes when the checkout is not on that branch or the issue's file is not committed as it is (`--no-commit` only writes it).
- **The lists are checked at acceptance:** the tester and the build build from the files of their lists, so each list holds every file its files need to build, every header and every helper of the tests. Where `watchbill.yml` has `needs`, the project's own command, `wb accept` runs it for each path of `changes` and of the tester's and the build's `reads`, and refuses the file when a path it prints is in neither that role's `reads` nor `changes`. Without `needs` nothing is checked, and `wb accept` says so. A function the code hides and the tests need is a stub, in the tester's and the build's `reads`. The breakdown runs the command itself before it commits the file. The build is not the change's: `wb accept` refuses a path of `changes` under `build_files` of `watchbill.yml`. Decided by the owner on 2026-11-19 (#125), so that a session builds from its own view with no filter of the compiler's messages.
- **An edit after acceptance** voids it: the hash no longer holds, and no prompt of the issue is handed out until a person accepts again (#76).
- **The acceptance at the tracker:** the person also posts one comment on the issue, from their own account, whose first line starts with `[accepted]`, with the SHA-256 of the issue's file. The tracker keeps who wrote it. What a listener trusts is a comment with `system: false` whose author's role at the project, read from the server, is at least the one the rule table names. The `.accepted` file stays.
- **What acceptance cannot stop:** under one account shared by a person and the agents, an agent can run `wb accept` itself, or post the comment. The hash stops a change after acceptance, not a false acceptance. Accounts of their own for the agents (point 14) stop that: an agent's account is never a person's, so its comment counts for nothing. A signed commit may later do the same for the `.accepted` file.
- **A role started alone:** a person may start any role with no step before it (decided by the owner, 2026-11-19, #116). With no issue's file, the role works from the issue's own numbered items. A review or a fix may be of a change with no issue: the change's description is the spec, and the stamp is posted on the change. A team takes Watchbill on one role at a time this way, starting with reviews. This replaces the rule of 2026-11-18 (#97) that no role after the breakdown is handed out without the file, and `breakdown: none`, which let a project out of it: since revision 0.0.8 the key `breakdown` is refused by name.
- **The chain's gates bind once a file is there:** the tester, the build, the review and the fix of an issue with a file are handed out only when the file is accepted, and the file and its acceptance are both committed in `HEAD` and are as git has them in the working tree, so the history holds what the issue was read as before the work it allows (#97).
- **The hand-on always needs the whole chain:** `wb handover`, the listener and the officer hand nothing on for an issue without an accepted file (#88, #85, #86).

## 14. Accounts

Each role works under an account of its own at the git server and the tracker. Settled with the owner on 2026-11-18 (#30), after two runs in which every session ran under the owner's account, so that nothing told a person's decision from an agent's claim.

- **One account per role:** tester, build, review, fix, officer and breakdown. Each is bound to one project, and none is a person's. It is not a pool of named agents: work in parallel is a matter of free places on a runner, and the stamp tells the sessions apart. An account bound to one project keeps the level of an assignment (point 11) from being raised by other projects.
- **The first tools:** GitLab, as the git server and as the tracker.
- **Setting them up:** a person creates the accounts from a written recipe. `wb` creates none, and no model takes part. `wb accounts check` only reads: each role has an account, they are distinct, none is a person's, each has the rights of its role and no more, and each is bound to this project (#83).
- **The handover is an assignment.** The program reads the last assignment note of the issue: a note with `system: true`, its author and its time. It counts only when its author is the officer's account, or a person whose role at the project, read from the server, is at least the one the rule table names. The organisation's file may ask for a higher role, never a lower one. A comment whose words read like an assignment has `system: false` and counts for nothing.
- **The server does not refuse a low role.** On GitLab a Reporter can assign an issue (trial of 2026-11-18, #30). `wb` reads the role of the author and decides.
- **The assignee is whose move it is.** A session that needs a person assigns the issue back to that person, with a comment. `wb` does not rely on the server to keep one assignee at a time.
- **The entry, in two steps.** First a listener picks up what a person assigns straight to a role's account. Then the officer is the normal entry: a person assigns the issue to the officer, it judges, and the program hands the issue on. A person may still assign straight to a role.
- **The listener** runs on a machine that is always on and asks the tracker at a fixed interval for the issues assigned to the roles' accounts. No webhook: no port is opened, and nothing is lost while the machine is down (#85).
- **The starter** is a command of `wb`, one path for a person's `wb start build <n>` and, after the sandbox, for the listener. The checks of the rule table, the acceptance and the stamp sit in that path (#84, #66).
- **The issue's file is never left out of work that is handed on.** Every piece the program hands on has an issue's file and a person's acceptance before the tester starts. A role a person starts alone needs neither (point 13, #116).
- **An issue that is several pieces:** the officer posts the pieces as one comment in a fixed form, and the issue is assigned back to the person. After a yes, the program writes the issues from the officer's account. The person's issue is never changed (#86).
- **The stamp** is posted by the identity of the starter, never by the session's (point 12).
- **Local work:** `wb start build <n>` on a person's own machine runs under that person's account and counts as the person's. The stamp says an agent wrote it. That person's account holds build on the change, so the review is never that person. Classed code is refused in local work until a session sees only its files (#65).
- **The tokens:** the program holds them and gives a session only its own.
- **The rule table** keeps `may_not_also_hold` as it is.

## The requirements

The pieces that build this have their requirements reserved as Draft rows in `requirements/`:

| Area | File | Issue |
|---|---|---|
| `WB-ROLE` | `requirements/role.md` | #4, #69, #75, #78, #82, #95, #98, #113, #116, #125, #145 |
| `WB-RULE` | `requirements/rule.md` | #5, #75, #95, #98, #113, #145 |
| `WB-REV` | `requirements/rev.md` | #6 |
| `WB-PIN` | `requirements/pin.md` | #7 |
| `WB-CHECK` | `requirements/check.md` | #8 |
| `WB-PROMPT` | `requirements/prompt.md` | #9, #10, #68, #76, #78, #116 |
| `WB-CONF` | `requirements/conf.md` | #11 |
| `WB-TASK` | `requirements/task.md` | #74, #78, #116, #157: the area keeps its name, and its rows say issue |
| `WB-ACCT` | `requirements/acct.md` | #83 |
| `WB-BUILD` | `requirements/build.md` | #84, #162, #172, #175, #186 |
| `WB-LISTEN` | `requirements/listen.md` | #85 |
| `WB-OFFICER` | `requirements/officer.md` | #86 |
| `WB-HAND` | `requirements/hand.md` | #88 |
| `WB-MON` | `requirements/mon.md` | #124, #126, #127 |
| `WB-VIEW` | `requirements/view.md` | #126, #145, #159, #167, #173, #192 |
| `WB-ANSWER` | `requirements/answer.md` | #145 |
| `WB-STATUS` | `requirements/status.md` | #139, #155 |
| `WB-REVERT` | `requirements/revert.md` | #142 |
| `WB-PUSH` | `requirements/push.md` | #151 |
| `WB-CMD` | `requirements/cmd.md` | #171 |
| `WB-UPGRADE` | `requirements/upgrade.md` | #174 |

The move to Rust builds no new area. #21 to #25 port what #2 and #4 to #6 built, row by row, and their new rows are reserved in `requirements/pkg.md`, `init.md`, `rule.md` and `rev.md`.
