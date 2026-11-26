# The words of `wb`

The words `wb` and its documents use, one line each, and how they are like git. This is for 0.12.0. A command that is not built yet is marked with the release it is planned for, as `wb help` marks it. The README says what each does: [README.md](../README.md).

## The words

- **Revision:** the version of the package and the hash of its files (role prompts, rule table, model table). What a repository is pinned to ([revision.md](revision.md)).
- **Manifest:** `.watchbill/revision.sha256`, the hash of each file of the revision.
- **Role:** planner, tester, build, review, fix. A fixed prompt and a row of limits in the rule table.
- **Session:** one run of an agent in one role.
- **Runner:** the tool and model a session runs with, named and cleared in the organisation's file. Its `start` is the command.
- **Level:** how confidential code is. The organisation's to set.
- **Classed code:** paths that need a person as verifier.
- **Job:** the text of what is to be built, stored in git under the hash of its content; its name is a ref under `refs/watchbill/`. An issue is the tracker's thing a job may come from. `wb plan add` takes one in (#234); the rest of point 15 is built in 0.12.0 ([revision.md](revision.md)). Until then the other commands take an issue.
- **Plan:** a job of `wb plan add`, with the issue's file the planner writes for it. `wb plan` lists, shows, edits, accepts and removes it (#290). Its state is `planned` until a person accepts it with `wb plan accept`.
- **Source:** one text of a job, each under its own hash. A job may have several.
- **Issue's file:** `.watchbill/issues/<issue>.yml`: the requirements, the interface, `changes` and each role's `reads`.
- **Acceptance:** a person's word that the issue's file is right.
- **`changes` and `reads`:** the paths a role may change, and the paths it may read.
- **View:** the folder a session works in: only its paths, no history.
- **Kept view:** a view left in place because work was refused or left uncommitted.
- **Stamp:** what a session was handed out as: a signed commit on the job's ref, with a copy posted on the issue or the change where `issue_comment` is set up.
- **Round:** the count of a role's sessions on an issue, from the stamps.
- **Change:** a pull request or a merge request.
- **Pattern:** a shell line in `watchbill.yml` that asks the tracker or the git server. A job read from its ref needs none for its questions and stamps.
- **Adapter:** a file for each tool that gives every pattern. Proposed, not built.
- **The lines:** the local folder each start leaves: the stamp and the outcome.
- **Limit:** a refusal that protects the process, or a fault `wb` cannot go past, such as a pattern that failed or a file that cannot be written. Never passed by a flag. Said as `<command>: limit: <why>` (#180).
- **Guard:** a refusal that protects a person's time, passed by the flag named for it, and by that flag alone. Said as `<command>: guard: <why>`, naming the flag (#180). The flags are `--ignore-local` (a local change under `reads`), `--ignore-kept` (a kept view) and `--ignore-branch` (a checkout on a branch that is neither the stamp's nor the default branch) of `wb start`, and `--no-commit` of `wb accept` and `wb plan accept` (the checks of its commit). After `--no-commit` the acceptance is written and not committed, and the limit of #97 still holds: no prompt is handed out and no session started until the issue's file and its acceptance are committed.
- **Plumbing and porcelain:** output that is fixed, for scripts, and wording, for people.
- **Captain:** the person who decides what is built.
- **Officer:** the watch officer, the agent role that is given issues and hands them on. Not built (#86, planned for 0.12.0).

## Likeness with git

`wb` is run in the folder, one command per job, with an exit status that scripts read, as git is. Each row says the nearest thing in git and where `wb` differs.

| `wb` | Nearest in git | Differs in |
|---|---|---|
| `init` | `git init` | never writes over what is there; `--survey` drafts `style.md` on a local runner and commits nothing |
| `check` | `git fsck` | also checks the installed `wb` |
| `status` | `git status` | lists jobs, not files, under two headings by whose move it is; see [`wb status` and `git status`](#wb-status-and-git-status) |
| `log <issue>` | `git log` | reads the job's ref and git alone and stores nothing; `--oneline` and `-q` as git; oldest first, where git's newest is first |
| `compare a b` | `git diff a b` | revisions only |
| `revision` | `git rev-parse HEAD` | |
| `prompt` | `git cat-file -p` | fills the placeholders |
| `plan add` | `git hash-object -w`, `git update-ref` | the text is a blob, the name a ref under `refs/watchbill/` |
| `plan list` | `git worktree list`, `git stash list` | one plan to a line, by its first name |
| `plan show` | `git notes show`, `git stash show` | the sources, the issue's file and its state |
| `plan edit` | `git notes edit` | commits the edit alone; an acceptance before is void |
| `plan accept` | `git tag -s` | the acceptance of the issue's file, committed (#235 signs it) |
| `plan remove` | `git remote remove`, `git worktree remove` | a marker added, nothing rewritten |
| `accept` | `git tag -s` | a person's word, committed |
| `start` | `git worktree add`, then run | makes a view, no history |
| `run` (planned for 0.12.0, #183) | `git rebase` | stops at gates, no `--continue` |
| `answer` | resolving, then continuing | writes a signed commit on the job's ref, and posts a copy where `issue_comment` is set up; a job begun before the refs is answered at the tracker |
| `revert` | `git revert` | takes back a step and every step after it, by a marker on the issue; nothing is deleted |
| `clean` | `git clean`, `git worktree prune` | removes the kept views whose work is all taken back, and says each first; `-n` and `--dry-run` as git; a view named is removed with its work, no `--force` |
| `view --kept` | `git status` | the kept views, what each holds |
| `view --in` | `git ls-files` | |
| `view --out` | `git ls-files --others` | by folder, with counts |
| `view <path>` | `git check-ignore -v` | says the list and the line |
| `upgrade` | `git submodule update` | shows what changes first; `-n` and `--dry-run` as git, and it commits nothing |
| `doctor` | none in git | checks the machine: what Watchbill needs on it, one line to a check, and the ports that listen |
| `help` | `git help` | `wb help <command>` says more of one; a command not built is marked with its release |

| Thing | Nearest in git | Differs in |
|---|---|---|
| `.watchbill/` | `.git/` | is committed with the code |
| `watchbill.yml` | `.git/config` | shared by the team |
| the organisation's file | `/etc/gitconfig` | a team cannot override it |
| view | worktree | no object store, no remote |
| stamp | commit trailer | lives at the tracker |
| job (point 15) | blob, its name a branch | its ref is under `refs/watchbill/` |
| acceptance (decided, point 15) | signed tag | checked against the project's keys |
| adapter (not built) | remote | declarative, for each tool |
| `--ignore-local` | `--force` | waives one named guard only |

Only `run` is not built: it is planned for 0.12.0 (#183).

## `wb status` and `git status`

`wb status` is made to look like `git status` (#284). It lists jobs where git lists files.

- **Headings:** by whose move it is next: "Jobs waiting for you" and "Jobs in the flow", as git has "Changes not staged" and "Changes to be committed". A heading with no job is not printed.
- **Colour:** the first heading's lines red and the second's green, as git colours what is not staged and what is. Only on a terminal. `--color[=<when>]` is the flag of `git diff` and `git log` (`always`, `never` or `auto`, the default); alone it is `always`. `git status` has no `--color`.
- **Short and porcelain:** `-s` and `--short` print one line to a job, as `git status -s`. `--porcelain` prints the same with no colour and no hint, for a script. Of the two, the one given last wins, as in git.
- **Hints:** lines in git's form, `(use "wb answer <issue>..." to answer its questions)`, as `advice.statusHints` gives in git. They are on unless `status_hints: false` is written in `watchbill.yml`. Only `true` and `false` are values.

Where it differs, on purpose:

- A job that could not be read is `**` and its number in the short form, never `??`, which is git's word for untracked. Its reason goes to standard error.
- The colour settings of git (`color.status`, `color.ui`) are not read.
- Jobs are in the order of their stage, then of their number, not of their path. The stage is a word on each line.
- The hints and the headings are not coloured.
- A job is in the flow when its branch holds a commit the default branch of origin does not. A job that is done is not listed.

## The words of the flags

The rule is in CLAUDE.md (#280). A flag takes the word git uses for the same meaning, and its short form where git has one: `--quiet` and `-q`, `--dry-run` and `-n`. Where git has none, it takes docker's. Where they differ, git's. A flag with no likeness in either is named for what it does. There is no general `--force`.

These are kept as they are, on purpose (#285):

| Flag | Why it stays |
|---|---|
| `wb init --yes`, `-y` | git has no likeness; apt and dnf use `-y` |
| `wb answer --show` | git's word for printing in full (`git notes show`, `git stash show`) |
| `wb view --in`, `--out` | git's `--cached` and `--others` mean in the index and untracked; a view's in and out are neither, and git's words would mislead |
| `--ignore-local`, `--ignore-kept`, `--ignore-branch`, `--take-back` | named waivers: no general `--force`; no likeness in git |
| `wb prompt --parts`, `--stamp` | plumbing; no likeness in git |

`-n` is `--dry-run` for `wb upgrade` and `wb clean`, and `--no-commit` for `wb accept` and `wb plan accept`, as `git cherry-pick -n` and `git revert -n` have it. `-q` is taken by every command. It leaves out only a line that confirms what was done, says there was nothing to do, or only informs. A refusal, a warning, what is said before a write and the output a script reads stay.

## The exit statuses

git ends with 128 for a fatal error and 129 for wrong use, and its commands each say what 1 is. `wb` ends with 0, 1 and 2, and they mean the same in every command (decided 2026-11-16):

- 0: it holds, or it is done.
- 1: it does not hold, or it was refused.
- 2: the command was used wrongly, or the repository is not set up.

The reason is the scripts. A script that runs any `wb` command reads one status the one way, and needs no table for each command.
