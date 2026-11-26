# What a revision contains

A revision is what a project is set up for and what its agents run under: the same on every machine, and checkable by anyone. This page says exactly what it is. Each point was settled with the owner on 2026-11-16 (#3). Points 1 and 6 were changed on 2026-11-17, when the owner decided that `wb` is written in Rust and not in Python (#20). Nothing else on the page changed: the files of a revision and their hash are the same. Point 14 and the changes to points 10, 12 and 13 on accounts were settled with the owner on 2026-11-18 (#30), and are in revision 0.0.7 (#82). That a person can start any role alone was settled with the owner on 2026-11-19, and changed points 12, 13 and 14 (#116, revision 0.0.8). That every role asks on the issue, plainly and with a suggested answer, changed points 12 and 13 (#113, revision 0.0.8). That `wb accept` checks the lists by the project's `needs` and keeps the build out of `changes` changed point 13 (#125, revision 0.0.9). That a session sees only its files, in a view, changed points 3 and 12 (#126, revision 0.0.9). That a session asks in one comment, a person answers with `wb answer`, and a session in a view commits and a person pushes, changed points 3 and 13 (#145, revision 0.0.9). Point 15, on identity, history and trust, was settled with the owner on 2026-11-22 (#219), and changes points 12, 13 and 14 as its pieces are built. That the planner reads its issue as `wb` brings it in, works in a writable view, and writes its questions in full in the issue's file for `wb` to post, changed points 3, 12 and 13 (#295, revision 0.11.1). The role `planner` was called `breakdown` until revision 0.12.0, in its file `roles/breakdown.md`, in the rule table, in the organisation's file and in the stamps; the owner renamed it on 2026-11-24 (#290). That each role has the limits of a session in the rule table, which `watchbill.yml` may set lower and never higher, changed point 3 (#267, revision 0.12.0).

## In short

- A revision is a version of the package and the hash of its files, both written.
- The files are the role prompts, the rule table and the model table.
- `wb init` copies them into the repository, and `wb check` fails on any difference.
- The levels of confidentiality and the runners are the organisation's and are not part of the revision.
- Each role works under an account of its own, and a handover is an assignment at the tracker.
- Decided, not built yet: `wb` keeps jobs, stamps, acceptances and handovers in git, signed, and a tracker is optional (point 15).

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

Planner, tester, build, review and fix. The first revision had the last four. The planner has its prompt and its rows from revision 0.0.5 (#75). It comes before the tester, and once an issue has a file, no other role of it is started until a person has accepted the file (point 13). A person may start any role alone, with no file (#116). The watch officer has rows in the rule table and no prompt yet (point 10). The architect comes later.

- **Planner:** (called the breakdown until revision 0.12.0, #290) reads a person's issue and the code, and writes the issue's file (point 13). It never changes the issue, the code or a test, and never answers what the issue leaves open. It is the one role that reads the code without a list. It holds no other role on the issue.
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
- its prompt file, or `none` when no agent holds the role yet;
- the limits of a session, from revision 0.12.0 (#267): `quiet_minutes`, past which with nothing changed in its view `wb start` says so, and `max_minutes`, past which it ends the session as a termination of `wb` does. Every role has 30 and 240, the planner too. `watchbill.yml` may set them lower for a role (`session_limits`), never higher: a higher one is refused by `wb check` and by `wb start`. Decided by the owner on 2026-11-23.

For the whole table:

- at most two review rounds;
- how many times a session is asked for its last message, with the fields its prompt names, before it goes to a person (#45);
- what always goes to a person;
- the roles a requirement that a person adds to a change under review goes to, before the next round (#49);
- the rule for classed code (point 4).

The program enforces the table. A prompt repeats a limit so the agent knows it, but no limit rests on the prompt alone.

What a session may read and change is the program's to hold for a session that `wb start <role>` starts for an issue with an accepted file: it runs in a view, in bubblewrap, with only the paths of its lists and no history, and its commits come back only where `changes` allows (#126). For a session a person starts by hand, that limit rests on the prompt alone. Decided by the owner on 2026-11-19 (#65), in revision 0.0.9. From revision 0.11.0 (#202) every other session that `wb start` starts runs in a view too: the planner, and a role with no issue's file, in a view of every file of `HEAD`, with no history. The planner's view is writable but for `.watchbill/` other than `.watchbill/issues/`, and only its issue's file comes back: from revision 0.11.1 (#295) the owner reversed #202's read-only tree, after a trial in which the planner could not run the tests it guards; a role with no issue's file cannot write `.watchbill/`, and nothing under it or under `build_files` comes back. A view has no remote, so the planner, the tester, the build and the fix commit and push nothing, and open no change: `wb` says the command that pushes the branch and, after a build, the one that opens the change as a draft, and a person runs them. Decided by the owner on 2026-11-20 (#145), and for the planner on 2026-11-23 (#202), when its row of the rule table lost its push: no row allows one. From revision 0.12.0 (#151) `wb` pushes the branch itself after the tester, the build and the fix, as the person and outside the sandbox, never with force, and after a build opens the change as a draft with the description the session wrote to `{description}`, a file in the sandbox outside the view. The owner agreed on 2026-11-20 that this is a step of the program, not a role: it needs no model, and a session that held the login of the code host could read every file through its API. After the planner, the review, or with `--no-push`, `wb` says the commands, as before.

## 4. Code classes

One rule in the first revision: a change that touches classed code needs a person as its verifier. Which paths are classed is the project's to say: the list `classed` of `watchbill.yml`, and every path where it says none (#84). `wb init` asks for the list and proposes none: `classed: []`. Levels per class come later.

## 5. The model table

One exact model ID and one effort for each role that has a prompt, but the planner. Never "latest", and never a name without a version.

| Role | Model | Effort |
|---|---|---|
| Tester | `claude-sonnet-5-5` | medium |
| Build | `claude-sonnet-5-5` | medium |
| Fix | `claude-sonnet-5-5` | medium |
| Review | `claude-opus-5-5` | high |

Where an issue or the project names another model or effort than the table, the table wins. An issue may ask for a stronger model or a higher effort, never for less, and the stamp of #31 records what ran (#51).

Figures on the findings of the first runs correct it. A changed model is a new revision. The table is for a vendor's runner. The models of local runners have a place of their own in the table, `local_models`, empty until the first one exists (point 11). A table with anything there is refused until then.

The planner has no line for a vendor's runner, and a table that gives it one is refused. It reads the code without a list, so it sends a model more of the code than any other role. It runs on a local runner, or on another runner only where a person has cleared that runner for the planner in the organisation's file (point 11). There is no word per issue for it. The organisation clears a runner once, in its file, up to a level, with the model and the effort it runs with; `wb prompt planner --runner <name>` refuses a runner that is not cleared for the level of the repository, and its stamp names the runner, the model and the effort (#93).

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
- **The officer** is a model's role for what takes judgement: whether an issue is clear enough to start, how it is broken into pieces, the brief for the one who gets it, and what to do with a piece that is stuck. Which paths and which class a piece touches are not the officer's: the planner says them (#64).
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
- **The planner is cleared once,** in the organisation's file, up to a level: a runner may have `planner`, with the highest level of an assignment it may break down, no higher than the runner's own, and the exact model and the effort it runs with. Nobody clears the planner by hand on each run. The other roles are checked with #11.
- **A project states only its own level.** It can be stricter than the organisation, never freer, and a team cannot clear a runner for itself.
- **With no organisation's file, no AI is allowed.** `wb` hands out no prompt on that machine and names the file it looked for. Nothing is assumed about where work may run, not even this machine with a local model.
- **The level of an assignment** is the higher of the level of its issue and the level of everything the session can read. A session reads the whole checkout, so until a runner can limit that, the second is the level of the repository.
- **The program refuses, never a model.** No level given means the most restrictive. A person sets or lowers a level; an agent may only raise one.
- **What it does not do:** stop a person from pasting code into another tool. The network rules of a runner do that. Watchbill refuses the handover.

## 12. The stamp

Point 15 moves the stamp to a commit on the job's ref, signed by the starter, and #236 built it (revision 0.12.0): for a session of an issue the stamp below is the message of a commit on `refs/watchbill/<issue>`, with two lines more, `job:` and `base:`; the round is counted from those commits, and the comment is a copy that `wb` never reads back. The rest of this point holds.

What a session ran as is said by the one who started it, not by the session. Settled with the owner on 2026-11-17 (#31), after two runs in which every session gave its effort as `unknown`.

- **What is stamped:** the role; the model ID and the effort of the model table; the runner; the version of the package and the hash of the revision; the issue, the change and the round; the branch and the checkout; the time; the view, as the hash of its list of paths, or `none` for a view of the whole tree (#126, #202); the level of the repository, on the line `level:` after the view (#209), for every session, as every session that `wb start` starts runs in a view (#202); the line `local: ignored <n>`, after the view, only where a start went on with local changes under `reads` that `--ignore-local` passed (#172); the line `requirements: given`, where the option was given; for the planner that `wb start` starts, the SHA-256 of the issue's text it brought into the view, on the line `issue_text:` (#295); for a session that `wb start` starts, the endpoints its view reaches, on the line `reach:`, or `reach: none` (#195); and the hash of each part of the prompt.
- **What it means:** what was handed out. `wb prompt` knows what the table asks for and what the prompt holds. It cannot see what the session truly runs as. The one who starts the session starts it so. `wb start <role>` does both in one step (#84), and closes the gap for a runner whose `start` sets the model and the effort.
- **Who stamps:** the program that hands out the prompt: `wb prompt --stamp` (#68), and `wb start <role>`, which posts it by `issue_comment` before it starts the session (#84). A person who starts a session by hand posts the stamp as it was printed. The stamp is posted by the identity of the starter, never by the session's (point 14).
- **Its form:** lines of `name: value` in a fixed order, the same on every machine, with one line for each part of the prompt in the form `sha256sum` prints. A program can read it and `sha256sum` can check its parts.
- **Where it is kept:** on the issue, one comment per session, whose first line starts with `[stamp] <role> round <n>`. The tester and the build run before there is a change, so the issue is the one place that holds every session. #30 may move it later. A review or a fix of a change with no issue has its stamp on the change, and its line `issue:` is empty (#116). Since #236, for an issue, it is kept as a commit on the job's ref, signed by the starter with a key of `.watchbill/signers` of the default branch (with no list, no stamp is written and none counts, #323), and the comment is a copy posted where a tracker is set up; a review or a fix of a change with no issue keeps its stamp on the change, as a comment.
- **The agent's own word:** `model` and `effort` stay in the last message, as what the session says of itself (#69). Where they differ from the stamp, that is a finding for a person, and the stamp is what counts. `unknown` is written down, not a finding: a session often cannot see its own effort. So every role prompt says that its model and effort are in its stamp, the latest `[stamp] <role>` comment, and that the last message gives them from there (#113).
- **What it is not:** a record kept for an assessment. The stamp is working state: the program reads it to know what was handed out. Keeping a record is for the team's own tools or a recorder (the README, "Not goals").
- **A copy on the machine:** `wb start <role>` also writes the stamp, posted or as it would have been, with how the start ended, in a folder of its own on the machine where it ran (#124). They are diagnostics: only `wb start` and `wb status` read them back, to name a view kept with work not taken back and a session cut off (#222; the README, "The lines it leaves").
- **A session with no stamp:** its work is not taken as the work of a role, and nothing is handed on from it. Until a program hands work on (#66, the officer), a person keeps this rule: no stamp on the issue, no next role.

## 13. The issue's file

Point 15 makes the acceptance a signed tag, and the questions and answers commits on the job's ref. The signed tag is built (#235): each acceptance below is also a tag signed by the person, and counts only where its signature checks against `.watchbill/signers` of the default branch; without that file nothing counts (#323). The questions and answers are built (#238, revision 0.12.0): for a job whose ref holds a stamp, a commit of `wb plan` or a questions commit (#333), the comments named below are copies posted out, and `wb` reads the questions and the answers from the ref alone, as the bullet of point 15 says; for a job begun before, whose ref holds none of them, the comments are read as below. Until the other pieces are built, this point holds.

A person's issue, read into a form `wb` reads. It was called the task's file until revision 0.0.6 (#78). Settled with the owner on 2026-11-18 (#64), after two runs in which every issue had to be made ready by hand before the tester could start.

- **Why it is apart from the issue:** the issue is what the person asked for. The issue's file is a reading of it, and a reading is a claim until a person accepts it. The planner never changes the issue.
- **Where it lives:** `.watchbill/issues/<issue>.yml`, in the repository. The planner commits it first on the issue's branch, alone. It is reviewed and merged with the change, so the history keeps what each issue was read as.
- **What it holds:** the name of the issue; the SHA-256 of the issue's text it was read from; the requirements, each with an ID, its text and where in the issue it comes from, and for a clause that already holds the existing test that guards it; the interface the tests call; the paths the change may change; what each role may read beside them; the existing tests the change changes; and the questions the issue leaves open. The form is in #74.
- **What a requirement is without a table:** a point of the issue's file. Its ID is the issue's number and the point, `54.1`. A test is named after it, a finding points at it, and it can be followed both ways. A project with a table of requirements gives the row's ID instead.
- **The questions are asked on the issue:** the planner posts each question as its own comment, whose first line is `[question] <issue>.q<n>`. A person answers there, in a comment whose first line is `[answer] <issue>.q<n>`, or in the question's thread. A second planner session, a fresh one, reads the answers, folds them into the file, and removes the questions answered: the file follows the answers. The comments are not part of the hash under `from`, so a change of the issue's text still means a new planner. Settled with the owner on 2026-11-18 (#95).
- **The planner reads the issue as `wb` brings it in** (#295, revision 0.11.1). Its session has no login to the tracker (#153, #202). Before it starts, `wb` runs `issue_text`, a pattern that prints the issue's text alone, and `issue_comment_bodies` as the person, outside the sandbox, through the cap of a text file, and writes into the view `.watchbill/issue/<issue>.md`, the issue's text as `issue_text` printed it, whole: nothing is cut from it (the second look at #297), and `.watchbill/issue/<issue>.questions`, the planner's questions and the answers to them, whole; no stamp and no other comment. Both are read-only there. `from` is the SHA-256 of the first, as the stamp's `issue_text:` is, so the comments are still not part of it. The planner posts nothing: each question goes in full under `questions` of the issue's file (what is unclear, why it matters, the choices with their roles, the suggestion), and after the session `wb` posts the new ones in the comment below, as the person, from the file in the last commit taken back; a question with a control character or a bidirectional control is refused, never posted. The planner's stamp is not in its view, so it gives its model and effort as its own word. Decided by the owner on 2026-11-24, after the trial of 0.11.0, in which the planner found no login and its questions were lost in its sandbox.
- **No session holds a login; `wb` reads and posts for each** (#153, revision 0.12.0). The tester, the build, the review and the fix work as the planner does since #295. Before the session `wb` reads, as the person, the issue's text (`issue_text`), the role's own questions and answers, and for the review and the fix the change's description (`change_text`, a new pattern), the comments of the review and the fix on it, and its diff against the base, of the paths of the role's lists only. It brings them in read-only at `/run/watchbill/in`, `{brought}`, outside the view's tree. The session writes its questions in full, the review's findings or the fix's answers, and what only a person can settle, in files in `/run/watchbill/`; after the session `wb` posts each, under a first line of its own: `[questions] <issue>.<role> q1 …`, `[review] <issue> round <n>` and `[fix] <issue> round <n>` on the change, and `[needs_person] <issue>.<role>`. The findings and the answers are posted only after a session that ended with 0, the answers only once the fix's branch is pushed. No prompt names a program of the tracker, and no stamp is in a view: every role gives its model and effort as its own word. Decided by the owner on 2026-11-24, after the trial of 0.11.1, in which the tester found no login and the build's `needs_person` stayed in the terminal.
- **Every role asks on the issue:** the tester, the build, the review and the fix post each point of `needs_person` as its own comment, whose first line is `[question] <issue>.<role>.q<n>`, and a person answers with `[answer] <issue>.<role>.q<n>`. A question says, in plain words, what is unclear, why it matters, the choices numbered from 1, and a suggested answer, marked as one, that names its choice by number. It is never edited once posted. An answer is a choice by its number, or words; `[answer] <issue> suggested` takes the suggestion of every open question. An answer that changes the issue's file is taken by a person, who changes the file and accepts it again; any other answer is taken by the next session of the role. A review or a fix of a change with no issue asks on the change. Since revision 0.0.8 (#113), after a run in which the tester's questions were found only in its last message.
- **The questions folded, the answers in the terminal:** from revision 0.0.9 every role, the planner too, posts its questions in one comment for the session, whose first line is `[questions] <issue>.<role> q1 q2` (`[questions] <issue> q1` for the planner, whose questions keep their names `<issue>.q<n>`). Each question is one line, `q<n>: <what is unclear>`, and the rest is folded under `<details>`. From revision 0.11.0 (#226) each choice starts with the role that acts on it, in brackets, `1. (tester) …`, and `person` is a person's own step, such as changing the issue's file and accepting it again; the line `changes the issue's file` of 0.0.9 and 0.10.0 is gone, and a choice that changes the file is `(person)`. The numbers count within the role on the issue. A person answers with `wb answer`, which shows each question in full and posts one comment per role, `[answers] <issue>.<role> q1 q2`, with a line `q<n>: <answer>` each; from 0.11.0 the first line names the role that acts after each question, `q1>tester q2>person`, and `wb status <issue>` says `waits for: <role> (<question>)` for the newest. The first lines of 0.10.0 still close their question and name no role. The forms of 0.0.8 are still read, so an issue begun under it reads right. Asked for by the owner after the trial of 0.0.8 (#145), in which seven comments of questions cluttered the issue and the owner's answer was in a form `wb status` did not read.
- **How a person accepts it:** `wb accept <issue>` writes `.watchbill/issues/<issue>.accepted`, with the SHA-256 of the issue's file, the person's git identity and the time. It refuses while the file holds a question. It never writes over an acceptance that is there: to accept again, the person removes the old one. Then it commits the acceptance alone, on the issue's branch, as that identity; it refuses before it writes when the checkout is not on that branch or the issue's file is not committed as it is (`--no-commit` only writes it).
- **The lists are checked at acceptance:** the tester and the build build from the files of their lists, so each list holds every file its files need to build, every header and every helper of the tests. Where `watchbill.yml` has `needs`, the project's own command, `wb accept` runs it for each path of `changes` and of the tester's and the build's `reads`, and refuses the file when a path it prints is in neither that role's `reads` nor `changes`. Without `needs` nothing is checked, and `wb accept` says so. A function the code hides and the tests need is a stub, in the tester's and the build's `reads`. The planner runs the command itself before it commits the file. The build is not the change's: `wb accept` refuses a path of `changes` under `build_files` of `watchbill.yml`. Decided by the owner on 2026-11-19 (#125), so that a session builds from its own view with no filter of the compiler's messages.
- **An edit after acceptance** voids it: the hash no longer holds, and no prompt of the issue is handed out until a person accepts again (#76).
- **A plan's acceptance** (#290, revision 0.12.0): for a plan of `wb plan add`, the person accepts the issue's file with `wb plan accept <name>`, which does what `wb accept` does above; `wb accept <name>` of a plan not accepted ends with 2 and names it, as `wb accept` is the acceptance of the work (#282): where the issue's file is accepted, it writes and commits alone `<issue>.finished`, and a job with that record, or whose change is merged, is done. A source added to the plan after the acceptance voids it, as an edit does. A planner round after it voids it too, even where the file comes out the same: a new round is a person asking for the plan to be looked at again, so it needs the person's word again (decided by the owner on 2026-11-24, after the trial of 0.11.0). Until the stamps are on the job's ref (point 15), a source or a `[stamp] planner` whose time is not before the `time:` of `.accepted` voids it, as `wb plan show` and `wb status` read it, and no prompt of the issue is handed out after it until `wb plan accept` again.
- **The acceptance at the tracker:** the person also posts one comment on the issue, from their own account, whose first line starts with `[accepted]`, with the SHA-256 of the issue's file. The tracker keeps who wrote it. What a listener trusts is a comment with `system: false` whose author's role at the project, read from the server, is at least the one the rule table names. The `.accepted` file stays. Since #235 the comment is a notice: where the acceptance is signed, `wb plan accept` posts it, and `wb` never reads it back.
- **What acceptance cannot stop:** under one account shared by a person and the agents, an agent can run `wb accept` itself, or post the comment. The hash stops a change after acceptance, not a false acceptance. Accounts of their own for the agents (point 14) stop that: an agent's account is never a person's, so its comment counts for nothing. A signed commit may later do the same for the `.accepted` file. Until point 15, whether a source or a planner round came before the acceptance or after it rests on the clocks of the machines that wrote their times, and on times that anyone who can write the job's ref or edit a comment can set.
- **A role started alone:** a person may start any role with no step before it (decided by the owner, 2026-11-19, #116). With no issue's file, the role works from the issue's own numbered items. A review or a fix may be of a change with no issue: the change's description is the spec, and the stamp is posted on the change. A team takes Watchbill on one role at a time this way, starting with reviews. This replaces the rule of 2026-11-18 (#97) that no role after the planner is handed out without the file, and `breakdown: none`, which let a project out of it: since revision 0.0.8 the key `breakdown` is refused by name.
- **The chain's gates bind once a file is there:** the tester, the build, the review and the fix of an issue with a file are handed out only when the file is accepted, and the file and its acceptance are both committed in `HEAD` and are as git has them in the working tree, so the history holds what the issue was read as before the work it allows (#97).
- **The hand-on always needs the whole chain:** `wb handover`, the listener and the officer hand nothing on for an issue without an accepted file (#88, #85, #86).

## 14. Accounts

Point 15 makes the handover a signed commit on the job's ref, with the assignment at the tracker a notice. Until that piece is built, this point holds.

Each role works under an account of its own at the git server and the tracker. Settled with the owner on 2026-11-18 (#30), after two runs in which every session ran under the owner's account, so that nothing told a person's decision from an agent's claim.

- **One account per role:** tester, build, review, fix, officer and planner. Each is bound to one project, and none is a person's. It is not a pool of named agents: work in parallel is a matter of free places on a runner, and the stamp tells the sessions apart. An account bound to one project keeps the level of an assignment (point 11) from being raised by other projects.
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

## 15. Identity, history and trust

What identifies a job and a session, where their history lives, and who can be trusted to have written it. Settled with the owner on 2026-11-22 (#219), from #216 and #217. It is built in pieces after this point. Until a piece is built, points 12, 13 and 14 hold as they are written.

- **Git is the store.** `wb` keeps its state in git, as git keeps its own: written with `hash-object`, `commit-tree` and `update-ref`. There is no store of `wb`'s own. A tracker is not needed.
- **A job** is the text of what is to be built. It is stored as a blob, under the hash of its content, so changed text is a new job. Its name is a ref, `refs/watchbill/<name>`, that points at it. A job with no name is named by the first 12 hex digits of its hash. A tracker's number is a name, never the identity.
- **Taking a job in:** `wb plan add` reads the text from standard input, from a file or from the command line, as `gh issue view 97 | wb plan add github/97 -F -`. The pipe is the way in: no pattern is needed to read a tracker. `--source` keeps the link back. `wb plan add` only queues: a job still goes through the planner, the acceptance and a view. Until 2026-11-24 it was called `wb add` (#290). A plan may have several sources, each under its own hash: other text under a name that is there is added as a new commit on its ref.
- **The job's text and the level:** the text is in every clone and cannot be deleted from them. `wb plan add` refuses a job whose level is above the repository's (point 11). Where the organisation's file allows it, a flag keeps only the hash and the source, not the text.
- **A stamp is a commit** on the job's ref, each naming the one before. Its message is the stamp's lines as they are (point 12). Its tree holds what the view held: the view's list and the issue's file. A ref of its own, not the issue's branch: a commit on the branch restarts its checks, and a squash merge erases it. Built by #236: its message adds `job:` and `base:` (the commit of `HEAD` the view is made from), and its tree holds the job's tree too. Where there is no list, none is written and none counts (#323). A stamp counts only with exactly one line `issue: <issue>`, a `base:` that is a commit object this repository has, and `job:` lines that are `job: none` where the tip of the ref holds no plan, or a removed one, or a non-empty start, in order, of the sources of the plan it holds (#324): so a stamp signed in another repository and put on this ref counts for nothing. A fork or a clone of the same project cannot be told apart this way, as it holds the same commits and sources; the list of keys is what is left. A marker of `wb revert` is a commit of the same kind, with a `base:` too. Every commit the ref reaches is read, and the ref counts only as `wb` writes it (#327): one line of commits, with no merge, each signed by a key of the list with the role it needs, `start` for a stamp or a marker and `accept` for a commit of `wb plan add` or `wb plan remove`. A stamp's tree is its parent's but for the view's list and the issue's file, and a marker's is its parent's, so a key with `start` cannot change the plan. A merge or an unsigned commit could otherwise hide the stamps that count, or choose the plan they are checked against. A bad ref also voids the acceptance, as a planner round after it cannot be told (the review of #328). Where the ref is not so, nothing on it counts and nothing more is written on it until a person repairs it (decided on the owner's word given in advance, 2026-11-25).
- **The lock is git's.** `wb start` pushes the stamp before the session starts, and a push that is not a fast-forward fails, so two starts of one role cannot both go on. The lock is for two honest starts. A forced push passes it, and a ref under `refs/watchbill/` cannot be protected at the server. The guard is that no session holds a login that can push (#153), and that each stamp is signed. Built by #236: a start whose push is refused as not a fast-forward ends with 1 and starts nothing; with no remote `origin`, the stamp stays on the local ref, and the start says so.
- **Trust is a signature.** In git, an author is free text and a push records no pusher. So the acceptance is a signed tag on one exact hash, and the starter signs each stamp. `wb` checks every signature against a list of keys the project keeps. A signature that does not check counts for nothing. Built for the acceptances by #235: the list is `.watchbill/signers`, read as the default branch has it, with the roles `accept`, `start`, `handover` and `answer`.
- **A missing list is refused, never skipped** (#323, the owner on 2026-11-25). Where the default branch holds no `.watchbill/signers`, or there is no default branch, nothing that needs a signature counts: no acceptance, no stamp, no marker, signed or not. `wb plan accept`, `wb accept`, `wb start` and `wb revert` of an issue end with 1 before they write anything, naming the file and how to add a key; `wb check` ends with 1 where the file is not in the working tree. The most restrictive applies where something is not given. `wb init` cannot know a key, so it writes no list and says that one is needed. The package requires `/usr/bin/ssh-keygen`, which checks every signature.
- **The handover is a commit** on the job's ref, signed by the officer or by a person. The listener reads the ref. An assignment at the tracker is a notice posted out.
- **Questions and answers** are commits on the job's ref: the session's start writes the questions, and `wb answer` writes the answers. Built by #238: a questions commit is written by `wb start` after a session, after its stamp, signed with a key of `.watchbill/signers` with the role `start`; an answers commit is written by `wb answer`, one for each role, signed by the person with a key with the role `answer`, and pushed with no force. Each holds the tree of the commit before it, and a message that is the first line of the comment (`[questions] <issue>.<role> q1 q2`, `[answers] <issue>.<role> q1>role`), the header `issue:`, `job:` and `base:` of a stamp, one blank line, and the form the comment has; only the header is read for the three lines, and a message over 48 KiB is never written, so that the commit stays under the 64 KiB the walk reads. The walk of #327 holds each to the role it needs and to its tree: where it does not, the ref is not as `wb` writes it and nothing on it counts. A copy is posted by `issue_comment` where it is set up, and `wb` never reads it back: an `[answers]` comment at the tracker answers nothing. Where the ref of a job holds a stamp (#333), a commit of `wb plan` (which is in no release before this one, so a plan's ref is always this revision) or a questions commit, whether it counts or not, the tracker's `[questions]` and `[answers]` are not read for it, from that stamp on: no questions commit is needed, none is open before the first, and no tracker pattern is run for its questions or answers, so a project with no `issue_comments` or `issue_comment_bodies` runs a job from the first planner round of a plan to `wb accept`; since #236 every stamp is a commit on the ref and 0.12.0 is the first release with them, so a stamp on the ref always means this revision. Where it holds none, or the job has no ref, they are read as before (an issue begun before), with the comments' `[stamp]` and `[revert]` lines placing them, until the first stamp is on the ref. A marker on the ref takes back the question and answer commits after the stamp it names, copy posted or not. The session is brought its own questions and answers from the ref too. For a job begun before, the start that writes its first stamp reads the tracker first, as it brings its files in (for a job taken into a plan, the planner's first round runs `issue_comment_bodies` once, where it is set, for the notice alone; nothing of it is brought in or counted); it says on standard error each question still open there, in any role, and each answer there that waits for a role, that the tracker is not read for the job from this start on, and that the questions are asked again by the next round of their role and answered with `wb answer`. Only the files of the role started hold what the tracker held of that role, so for the others the notice is all there is. It is a notice and no refusal, which would leave no way on.
- **The commits of a session** carry a note that names the session's stamp. The lines of a start stay on the machine, as a reflog does (#124). Built by #237: each commit `wb` takes back, at the end of a session or by `--take-back`, gets a note in `refs/notes/watchbill`, `stamp: <the commit of the stamp on the job's ref>` and `role: <role>`, written with `git notes add` and no `-f`; `wb start` pushes the notes ref to `origin` with no force, only where it pushed the branch in the same start (never with `--no-push`, for `--take-back`, or where it holds the branch back). A session of a change with no issue has a stamp that is a comment, and so no commit to name: it gets no note. **A note is not signed, and `wb` never reads one:** anyone who can push to `origin` can write any note, so no round, stamp, plan, acceptance or session is counted from them. They are an index for a person or a script, never evidence; the evidence is the signed stamp on the job's ref. The notes stay on the commits of the issue's branch, which a squash merge leaves, and keep those commits findable only while that branch is kept: a notes tree does not keep the commits it names reachable, so where the branch is deleted and git collects garbage, the commits and their notes name nothing. The stamp is written before the session, so it cannot name the commits that come after it.
- **A revert** is a new commit, a marker added. Nothing is rewritten.
- **The tracker is kept in step one way each.** In: `wb plan add` again, a new source under a new hash. A source that changed since it was taken in is said, and a person takes it in again. Out: notices, such as a stamp made, findings, a handover, a question, done. `wb` never reads a notice back. Never a merge of the two.
- **Left outside git:** the change, its checks and its merge, at the git server; discussion between people, at the tracker.
- **`wb init` and `wb upgrade`** write the fetch line for `refs/watchbill/*`, and since #237 the line `+refs/notes/watchbill*:refs/notes/watchbill*`, each that is not there, so a clone and the build server see the history and the notes. The notes line is a pattern, so a fetch ends with 0 where `origin` holds no notes yet; `wb upgrade` replaces the line that named the one ref, which #237's first build wrote. The line forces the ref, so a fetch replaces notes that were never pushed: a refused push says how to keep them.
- **The names kept:** "job" is `wb`'s word, and an issue is the tracker's thing a job may come from. `.watchbill/issues/<issue>.yml` and the branch `issue-<n>` keep their names.
- **What changes when it is built:** point 12, the stamp is kept on the job's ref, not on the issue; point 13, the acceptance is a signed tag, and the questions and answers are in git; point 14, the handover is a commit, not an assignment. #83, #88, #85 and #86 are rewritten by this point.

## The requirements

The pieces that build this have their requirements reserved as Draft rows in `requirements/`:

| Area | File | Issue |
|---|---|---|
| `WB-ROLE` | `requirements/role.md` | #4, #69, #75, #78, #82, #95, #98, #113, #116, #125, #145, #197, #224, #227, #295 |
| `WB-RULE` | `requirements/rule.md` | #5, #75, #95, #98, #113, #145 |
| `WB-REV` | `requirements/rev.md` | #6 |
| `WB-PIN` | `requirements/pin.md` | #7 |
| `WB-CHECK` | `requirements/check.md` | #8, #240 |
| `WB-PROMPT` | `requirements/prompt.md` | #9, #10, #68, #76, #78, #116, #211, #227, #236, #295 |
| `WB-CONF` | `requirements/conf.md` | #11 |
| `WB-TASK` | `requirements/task.md` | #74, #78, #116, #157, #235: the area keeps its name, and its rows say issue |
| `WB-ACCT` | `requirements/acct.md` | #83 |
| `WB-BUILD` | `requirements/build.md` | #84, #162, #172, #175, #186, #236, #295 |
| `WB-LISTEN` | `requirements/listen.md` | #85 |
| `WB-OFFICER` | `requirements/officer.md` | #86 |
| `WB-HAND` | `requirements/hand.md` | #88 |
| `WB-MON` | `requirements/mon.md` | #124, #126, #127 |
| `WB-VIEW` | `requirements/view.md` | #126, #145, #159, #167, #173, #192, #237, #295 |
| `WB-ANSWER` | `requirements/answer.md` | #145, #238, #295 |
| `WB-STATUS` | `requirements/status.md` | #139, #155, #238, #239, #240 |
| `WB-REVERT` | `requirements/revert.md` | #142 |
| `WB-PUSH` | `requirements/push.md` | #151, #240 |
| `WB-CMD` | `requirements/cmd.md` | #171 |
| `WB-UPGRADE` | `requirements/upgrade.md` | #174 |
| `WB-JOB` | `requirements/job.md` | #234 |

The move to Rust builds no new area. #21 to #25 port what #2 and #4 to #6 built, row by row, and their new rows are reserved in `requirements/pkg.md`, `init.md`, `rule.md` and `rev.md`.
