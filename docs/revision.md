# What a revision contains

A revision is what a project is set up for and what its agents run under: the same on every machine, and checkable by anyone. This page says exactly what it is. Each point was settled with the owner on 2026-11-16 (#3). Points 1 and 6 were changed on 2026-11-17, when the owner decided that `wb` is written in Rust and not in Python (#20). Nothing else on the page changed: the files of a revision and their hash are the same.

## In short

- A revision is a version of the package and the hash of its files, both written.
- The files are the role prompts, the rule table and the model table.
- `wb init` copies them into the repository, and `wb check` fails on any difference.
- The levels of confidentiality and the runners are the organisation's and are not part of the revision.

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

Breakdown, tester, build, review and fix. The first revision had the last four. The breakdown has its prompt and its rows from revision 0.0.5 (#75). It comes before the tester, and no other role of an issue is started until a person has accepted its file (point 13). The watch officer has rows in the rule table and no prompt yet (point 10). The architect comes later.

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

## 4. Code classes

One rule in the first revision: a change that touches classed code needs a person as its verifier. Which paths are classed is the project's to say. Levels per class come later.

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

The breakdown has no line for a vendor's runner, and a table that gives it one is refused. It reads the code without a list, so it sends a model more of the code than any other role. It runs on a local runner, or on another runner only where a person has cleared that runner for the breakdown in the organisation's file (point 11). There is no word per issue for it. Until the check of #11 is built, nothing refuses another runner: the breakdown's prompt says where it should run, and the stamp says where it was handed out (#64).

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
- **The officer** is a model's role for what takes judgement: whether an issue is clear enough to start, how it is broken into pieces, which paths and which class it touches, the brief for the one who gets it, and what to do with a piece that is stuck.
- The officer proposes and the program checks. A handover that breaks a rule is refused, whoever proposed it.
- The officer holds no other role on an issue it has handed out, and never merges. It has an account of its own, so every handover shows at the tracker as an assignment from it.
- Taking an issue that nobody assigned is off, unless a rule the captain approved names the queue it may take from, with a cap on the work in progress, and never for classed code. The first table names no queue.
- In the first revision the officer has its rows in the rule table. Its prompt is written after the first run of the other four roles.

## 11. Confidentiality

A level that decides where an assignment may run. It is a second axis beside the code class: the class decides who must verify, the level decides where the work may run.

- **The levels are the organisation's:** one ordered list of names for every project (for example internal, confidential, strictly confidential). Watchbill ships no list of its own and gives the names no meaning beyond their order.
- **A place to run is a runner**, and a runner is three things: where the agent's tools run, which model answers, and under which agreement (region, retention, use for training). Code leaves the premises through the model call also when the tools run on this machine, so the model and its agreement are part of the place.
- **The organisation names the runners** and, for each, the highest level it may be given. Levels and runners are in one file outside every repository: `/etc/watchbill/organisation.yml`. It is laid out by the organisation's own means and is not part of the revision.
- **A project states only its own level.** It can be stricter than the organisation, never freer, and a team cannot clear a runner for itself.
- **With no organisation's file, no AI is allowed.** `wb` hands out no prompt on that machine and names the file it looked for. Nothing is assumed about where work may run, not even this machine with a local model.
- **The level of an assignment** is the higher of the level of its issue and the level of everything the session can read. A session reads the whole checkout, so until a runner can limit that, the second is the level of the repository.
- **The program refuses, never a model.** No level given means the most restrictive. A person sets or lowers a level; an agent may only raise one.
- **What it does not do:** stop a person from pasting code into another tool. The network rules of a runner do that. Watchbill refuses the handover and leaves a record.

## 12. The stamp

What a session ran as is said by the one who started it, not by the session. Settled with the owner on 2026-11-17 (#31), after two runs in which every session gave its effort as `unknown`.

- **What is stamped:** the role; the model ID and the effort of the model table; the runner; the version of the package and the hash of the revision; the issue, the change and the round; the branch and the checkout; the time; and the hash of each part of the prompt.
- **What it means:** what was handed out. `wb prompt` knows what the table asks for and what the prompt holds. It cannot see what the session truly runs as. The one who starts the session starts it so; a starter that does both in one step closes the gap (#66).
- **Who stamps:** the program that hands out the prompt: `wb prompt --stamp` (#68). A person who starts a session by hand posts the stamp as it was printed.
- **Its form:** lines of `name: value` in a fixed order, the same on every machine, with one line for each part of the prompt in the form `sha256sum` prints. A program can read it and `sha256sum` can check its parts.
- **Where it is kept:** on the issue, one comment per session, whose first line starts with `[stamp] <role> round <n>`. The tester and the build run before there is a change, so the issue is the one place that holds every session. #30 may move it later.
- **The agent's own word:** `model` and `effort` stay in the last message, as what the session says of itself (#69). Where they differ from the stamp, that is a finding for a person, and the stamp is what counts. `unknown` is written down, not a finding: a session often cannot see its own effort.
- **A session with no stamp:** its work is not taken as the work of a role, and nothing is handed on from it. Until a program hands work on (#66, the officer), a person keeps this rule: no stamp on the issue, no next role.

## 13. The issue's file

A person's issue, read into a form `wb` reads. It was called the task's file until revision 0.0.6 (#78). Settled with the owner on 2026-11-18 (#64), after two runs in which every issue had to be made ready by hand before the tester could start.

- **Why it is apart from the issue:** the issue is what the person asked for. The issue's file is a reading of it, and a reading is a claim until a person accepts it. The breakdown never changes the issue.
- **Where it lives:** `.watchbill/issues/<issue>.yml`, in the repository. The breakdown commits it first on the issue's branch, alone. It is reviewed and merged with the change, so the history keeps what each issue was read as.
- **What it holds:** the name of the issue; the SHA-256 of the issue's text it was read from; the requirements, each with an ID, its text and where in the issue it comes from, and for a clause that already holds the existing test that guards it; the interface the tests call; the paths the change may change; what each role may read beside them; the existing tests the change changes; and the questions the issue leaves open. The form is in #74.
- **What a requirement is without a table:** a point of the issue's file. Its ID is the issue's number and the point, `54.1`. A test is named after it, a finding points at it, and it can be followed both ways. A project with a table of requirements gives the row's ID instead.
- **How a person accepts it:** `wb accept <issue>` writes `.watchbill/issues/<issue>.accepted`, with the SHA-256 of the issue's file, the person's git identity and the time. It refuses while the file holds a question. It never writes over an acceptance that is there: to accept again, the person removes the old one.
- **An edit after acceptance** voids it: the hash no longer holds, and no prompt of the issue is handed out until a person accepts again (#76).
- **What acceptance cannot stop:** under one account shared by a person and the agents, an agent can run `wb accept` itself. The hash stops a change after acceptance, not a false acceptance. Only accounts of their own (#30), or later a signed commit, stop that.
- **Without an issue's file** no role after the breakdown is handed out (decided by the owner, 2026-11-18, #97). A project that has no breakdown yet writes `breakdown: none` in `watchbill.yml`; then it works as before: the issue holds its numbered requirements, and `wb prompt` is given what it cannot know. `breakdown: none` is refused when the revision's rule table holds the breakdown, by `wb prompt` and by `wb check`. Where `breakdown:` is not written, the file is needed.
- **Committed first:** the tester, the build, the review and the fix are handed out only when the issue's file and its acceptance are both committed in `HEAD` and are as git has them in the working tree, so the history holds what the issue was read as before the work it allows (#97).

## The requirements

The pieces that build this have their requirements reserved as Draft rows in `requirements/`:

| Area | File | Issue |
|---|---|---|
| `WB-ROLE` | `requirements/role.md` | #4, #69, #75, #78 |
| `WB-RULE` | `requirements/rule.md` | #5, #75 |
| `WB-REV` | `requirements/rev.md` | #6 |
| `WB-PIN` | `requirements/pin.md` | #7 |
| `WB-CHECK` | `requirements/check.md` | #8 |
| `WB-PROMPT` | `requirements/prompt.md` | #9, #10, #68, #76, #78 |
| `WB-CONF` | `requirements/conf.md` | #11 |
| `WB-TASK` | `requirements/task.md` | #74, #78: the area keeps its name, and its rows say issue |
| `WB-ACCT` | `requirements/acct.md` | #83 |
| `WB-BUILD` | `requirements/build.md` | #84 |
| `WB-LISTEN` | `requirements/listen.md` | #85 |
| `WB-OFFICER` | `requirements/officer.md` | #86 |
| `WB-HAND` | `requirements/hand.md` | #88 |

The move to Rust builds no new area. #21 to #25 port what #2 and #4 to #6 built, row by row, and their new rows are reserved in `requirements/pkg.md`, `init.md`, `rule.md` and `rev.md`.
