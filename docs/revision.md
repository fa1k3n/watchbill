# What a revision contains

A revision is what a project is set up for and what its agents run under: the same on every machine, and checkable by anyone. This page says exactly what it is. Each point was settled with the owner on 2026-11-16 (#3).

## In short

- A revision is a version of the package and the hash of its files, both written.
- The files are the role prompts, the rule table and the model table.
- `wb init` copies them into the repository, and `wb check` fails on any difference.
- The levels of confidentiality and the runners are the organisation's and are not part of the revision.

## 1. The files of a revision

The role prompts, the rule table and the model table. The code of `wb` is named by the version of the package, the files by their hash, and a revision is both.

In the package:

| Path | Holds |
|---|---|
| `watchbill/revision/roles/<role>.md` | The prompt of a role |
| `watchbill/revision/roles/placeholders.md` | The words a prompt is filled in with |
| `watchbill/revision/rules.yml` | The rule table |
| `watchbill/revision/models.yml` | The model table |
| `watchbill/revision.sha256` | The manifest: the hash of every file above |
| `watchbill/revisions.txt` | Each version of the package that was released, with the hash of its revision |

The code that reads them is `watchbill/manifest.py` (files, hashes, manifest) and `watchbill/rules.py` (the two tables).

## 2. The roles of the first revision

Tester, build, review and fix. The watch officer has rows in the rule table and no prompt yet (point 10). Architect and breakdown come in a later revision.

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
- what always goes to a person;
- the rule for classed code (point 4).

The program enforces the table. A prompt repeats a limit so the agent knows it, but no limit rests on the prompt alone.

## 4. Code classes

One rule in the first revision: a change that touches classed code needs a person as its verifier. Which paths are classed is the project's to say. Levels per class come later.

## 5. The model table

One exact model ID and one effort for each role that has a prompt. Never "latest", and never a name without a version.

| Role | Model | Effort |
|---|---|---|
| Tester | `claude-sonnet-5-5` | medium |
| Build | `claude-sonnet-5-5` | medium |
| Fix | `claude-sonnet-5-5` | medium |
| Review | `claude-opus-5-5` | high |

Figures on the findings of the first runs correct it. A changed model is a new revision. The table is for a vendor's runner; a runner with a local model gets a table of its own when the first one exists (point 11).

## 6. The format of the tables

YAML, read through a library: the YAML library that the distribution packages (PyYAML; `python3-pyyaml` in Fedora and in the base repository of Enterprise Linux 9). The owner chose this over JSON and over a reader of Watchbill's own.

What follows from it:

- `wb` needs one library beside Python's standard library. The RPM names it, so `dnf` brings it. Nothing is installed from outside the distribution's own repositories.
- One loader in `wb` reads every YAML file, and nothing else calls the library:
  - it uses the safe loader, which builds only plain values;
  - a key written twice is refused. The library alone would keep the last one silently;
  - a value is held to the type its field asks for. Where a text is expected and YAML reads something else (`no` as false, `1.10` as a number), the file is refused and the message says to put the value in quotes.
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

The role that is given tasks and hands them on. The captain is the person, who decides what is built.

- **The program** decides all that the rule table can decide (who is free, who may hold which role on this change, the order, the rounds, what goes to a person) and performs every handover.
- **The officer** is a model's role for what takes judgement: whether a task is clear enough to start, how it is broken into pieces, which paths and which class it touches, the brief for the one who gets it, and what to do with a piece that is stuck.
- The officer proposes and the program checks. A handover that breaks a rule is refused, whoever proposed it.
- The officer holds no other role on a task it has handed out, and never merges. It has an account of its own, so every handover shows at the tracker as an assignment from it.
- Taking a task that nobody assigned is off, unless a rule the captain approved names the queue it may take from, with a cap on the work in progress, and never for classed code. The first table names no queue.
- In the first revision the officer has its rows in the rule table. Its prompt is written after the first run of the other four roles.

## 11. Confidentiality

A level that decides where an assignment may run. It is a second axis beside the code class: the class decides who must verify, the level decides where the work may run.

- **The levels are the organisation's:** one ordered list of names for every project (for example internal, confidential, strictly confidential). Watchbill ships no list of its own and gives the names no meaning beyond their order.
- **A place to run is a runner**, and a runner is three things: where the agent's tools run, which model answers, and under which agreement (region, retention, use for training). Code leaves the premises through the model call also when the tools run on this machine, so the model and its agreement are part of the place.
- **The organisation names the runners** and, for each, the highest level it may be given. Levels and runners are in one file outside every repository: `/etc/watchbill/organisation.yml`. It is laid out by the organisation's own means and is not part of the revision.
- **A project states only its own level.** It can be stricter than the organisation, never freer, and a team cannot clear a runner for itself.
- **With no organisation's file, no AI is allowed.** `wb` hands out no prompt on that machine and names the file it looked for. Nothing is assumed about where work may run, not even this machine with a local model.
- **The level of an assignment** is the higher of the level of its task and the level of everything the session can read. A session reads the whole checkout, so until a runner can limit that, the second is the level of the repository.
- **The program refuses, never a model.** No level given means the most restrictive. A person sets or lowers a level; an agent may only raise one.
- **What it does not do:** stop a person from pasting code into another tool. The network rules of a runner do that. Watchbill refuses the handover and leaves a record.

## The requirements

The pieces that build this have their requirements reserved as Draft rows in `requirements/`:

| Area | File | Issue |
|---|---|---|
| `WB-ROLE` | `requirements/role.md` | #4 |
| `WB-RULE` | `requirements/rule.md` | #5 |
| `WB-REV` | `requirements/rev.md` | #6 |
| `WB-PIN` | `requirements/pin.md` | #7 |
| `WB-CHECK` | `requirements/check.md` | #8 |
| `WB-PROMPT` | `requirements/prompt.md` | #9, #10 |
| `WB-CONF` | `requirements/conf.md` | #11 |
