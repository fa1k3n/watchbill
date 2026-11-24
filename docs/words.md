# The words of `wb`

The words `wb` and its documents use, one line each, and how they are like git. This is for 0.10.0. A command that is not built yet is marked with the release it is planned for, as `wb help` marks it. The README says what each does: [README.md](../README.md).

## The words

- **Revision:** the version of the package and the hash of its files (role prompts, rule table, model table). What a repository is pinned to ([revision.md](revision.md)).
- **Manifest:** `.watchbill/revision.sha256`, the hash of each file of the revision.
- **Role:** planner, tester, build, review, fix. A fixed prompt and a row of limits in the rule table.
- **Session:** one run of an agent in one role.
- **Runner:** the tool and model a session runs with, named and cleared in the organisation's file. Its `start` is the command.
- **Level:** how confidential code is. The organisation's to set.
- **Classed code:** paths that need a person as verifier.
- **Job:** the text of what is to be built, stored in git under the hash of its content; its name is a ref. An issue is the tracker's thing a job may come from. Decided, built from 0.12.0 ([revision.md](revision.md), point 15). Until then `wb` takes an issue.
- **Issue's file:** `.watchbill/issues/<issue>.yml`: the requirements, the interface, `changes` and each role's `reads`.
- **Acceptance:** a person's word that the issue's file is right.
- **`changes` and `reads`:** the paths a role may change, and the paths it may read.
- **View:** the folder a session works in: only its paths, no history.
- **Kept view:** a view left in place because work was refused or left uncommitted.
- **Stamp:** what a session was handed out as, posted on the issue.
- **Round:** the count of a role's sessions on an issue, from the stamps.
- **Change:** a pull request or a merge request.
- **Pattern:** a shell line in `watchbill.yml` that asks the tracker or the git server.
- **Adapter:** a file for each tool that gives every pattern. Proposed, not built.
- **The lines:** the local folder each start leaves: the stamp and the outcome.
- **Limit:** a refusal that protects the process, or a fault `wb` cannot go past, such as a pattern that failed or a file that cannot be written. Never passed by a flag. Said as `<command>: limit: <why>` (#180).
- **Guard:** a refusal that protects a person's time, passed by the flag named for it, and by that flag alone. Said as `<command>: guard: <why>`, naming the flag (#180). The flags are `--ignore-local` (a local change under `reads`), `--ignore-kept` (a kept view) and `--ignore-branch` (a checkout on a branch that is neither the stamp's nor the default branch) of `wb start`, and `--no-commit` of `wb accept` (the checks of its commit). After `--no-commit` the acceptance is written and not committed, and the limit of #97 still holds: no prompt is handed out and no session started until the issue's file and its acceptance are committed.
- **Plumbing and porcelain:** output that is fixed, for scripts, and wording, for people.
- **Captain:** the person who decides what is built.
- **Officer:** the watch officer, the agent role that is given issues and hands them on. Not built (#86, planned for 0.12.0).

## Likeness with git

`wb` is run in the folder, one command per job, with an exit status that scripts read, as git is. Each row says the nearest thing in git and where `wb` differs.

| `wb` | Nearest in git | Differs in |
|---|---|---|
| `init` | `git init` | never writes over what is there |
| `check` | `git fsck` | also checks the installed `wb` |
| `status` | `git status` | reads the tracker by patterns |
| `log <issue>` (planned for 0.12.0, #177) | `git log` | stores nothing itself |
| `compare a b` | `git diff a b` | revisions only |
| `revision` | `git rev-parse HEAD` | |
| `prompt` | `git cat-file -p` | fills the placeholders |
| `accept` | `git tag -s` | a person's word, committed |
| `start` | `git worktree add`, then run | makes a view, no history |
| `run` (planned for 0.12.0, #183) | `git rebase` | stops at gates, no `--continue` |
| `answer` | resolving, then continuing | posts on the issue |
| `revert` | `git revert` | takes back a step and every step after it, by a marker on the issue; nothing is deleted |
| `clean` | `git clean` | removes the kept views whose work is all taken back; `-n` and `--dry-run` as git; a view named is removed with its work, no `--force` |
| `view --kept` | `git status` | the kept views, what each holds |
| `view --in` | `git ls-files` | |
| `view --out` | `git ls-files --others` | by folder, with counts |
| `view <path>` | `git check-ignore -v` | says the list and the line |
| `clean` (planned for 0.12.0, #176) | `git clean`, `git worktree prune` | lists and asks first |
| `upgrade` | `git submodule update` | shows what changes first |
| `doctor` (planned for 0.12.0, #178) | none in git | checks the machine |
| `help` | `git help` | |

| Thing | Nearest in git | Differs in |
|---|---|---|
| `.watchbill/` | `.git/` | is committed with the code |
| `watchbill.yml` | `.git/config` | shared by the team |
| the organisation's file | `/etc/gitconfig` | a team cannot override it |
| view | worktree | no object store, no remote |
| stamp | commit trailer | lives at the tracker |
| job (decided, point 15) | blob, its name a branch | its ref is under `refs/watchbill/` |
| acceptance (decided, point 15) | signed tag | checked against the project's keys |
| adapter (not built) | remote | declarative, for each tool |
| `--ignore-local` | `--force` | waives one named guard only |

A man page for each command is planned for 0.11.0 (#182).
