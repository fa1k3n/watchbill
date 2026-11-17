# Working rules

Watchbill: a harness for AI agents in embedded software development, deployed at a fixed revision onto a project that already exists. The command is `wb`. What it is to be is in [README.md](README.md). What a revision is, with each decision behind it, is in [docs/revision.md](docs/revision.md). How a requirement is written is in [requirements/README.md](requirements/README.md).

## The repository

- Only general material. Never a client's rules, flows, names or data, in code, tests or documents.
- It stands alone: no code, field names or plans from any other project. Another tool may watch a team that runs Watchbill; Watchbill never needs it.
- Say "supports certification" or "can be assessed", never "certified".
- Documents are short, plain sentences, British spelling (organisation, licence).
- The captain is the person who decides what is built. The agent role that is given tasks and hands them on is the watch officer, "officer" for short. An agent is never called the captain.

## The work

- Each piece of work is an issue, and the issue is the spec: build what it says, and write any deviation into the issue before or with the code.
- An issue starts with its Run line: the model, the effort, what it needs that a session cannot do alone, and the issues it depends on. Read it first. If the session's model differs, say so before starting. An issue is started only when the issues it depends on are closed.
- Tests first, from the issue's list, with Rust's own test harness (`cargo test`). No test crate. Every new test is red before the code. A test that does not compile because the code is not there yet is red.
- A change reaches `main` only through a pull request, from a branch `issue-<number>-<words>`. Documents too.
- The review is a fresh session, one model level above the one that built the change (haiku, sonnet, opus, fable). It starts from the issue and the diff, never from the author's summary. It writes its findings as one comment on the pull request whose first line starts with `[review]`. A piece built at the top level is reviewed by the owner.
- A pull request is opened as a draft and stays one while the agents' review goes on: the review, the answers to it, and the second look. A piece that the owner alone reviews is opened ready.
- The author answers every finding of the review with a fix or a reason, in one comment. The reviewer then takes a second look, at the new commit. There are at most two looks.
- When the second look finds nothing that should block, the session that runs the piece marks the pull request ready for review and writes one comment whose first line starts with `[ready]`: the commit it holds for, what was run, and what is left open. A push after that comment puts the pull request back to draft until it has been looked at again.
- The session that wrote a change never merges it. The owner merges, with a squash, and only a pull request that is ready, never a draft.
- An existing test is changed only when the issue says so, and the pull request names each one.
- Stage files by name, never all at once.

## The code

- Rust, edition 2021, for the Rust that Enterprise Linux 9 packages (1.97) or later. The owner decided this on 2026-11-17 (#20); before that `wb` was written in Python.
- `wb` is built from the standard library and crates the distribution packages. A crate is taken where it does most of a job `wb` needs; where `wb` would use only a minor part of it, the lines are written instead. A crate is added to the list below by the issue that first needs it, and the owner agrees. The owner decided this on 2026-11-17 (#34).
- The crates of `wb`, each with what it is for:
  - `yaml-rust2`: reading YAML
  - `sha2`: SHA-256
  - `lexopt`: reading the command line; `wb` writes every word itself
- The crates come from the distribution's packages: Fedora's own repositories, and EPEL on Enterprise Linux 9. Nothing is fetched from crates.io, and a build asks nothing of the network. No `Cargo.lock` is committed: the versions are the distribution's.
- Installed, `wb` needs `git` and no library of its own.
- Every YAML file is read through the one loader in `wb`: only plain values are built; a key written twice, a tag, a merge key and an alias are refused; a value is held to the type its field asks for ([docs/revision.md](docs/revision.md), point 6).
- The folder that holds the files of the revision is fixed when `wb` is built. No option and no environment variable changes it when `wb` runs.
- `wb` is installed with `dnf` from an RPM ([packaging/](packaging/)). Nothing in the code or the texts assumes another way.
- Every exit status means the same in every command: 0 it holds or it is done, 1 it does not hold or it was refused, 2 the command was used wrongly or the repository is not set up.
- A command that only reads writes nothing. A command that writes says what it will write first, and never writes over a file that is there.
- `wb` asks nothing of the network unless the command is about a system on the network.
- What a team programs itself meets `wb` as a process: text on standard input and output, and the exit status. There is no API inside `wb` for it.

## Requirements

- Every behaviour of `wb` has a row in `requirements/`, and its ID is `WB-<AREA>-<number>`.
- The name of a test carries the ID of the requirement it covers: `test_WB_INIT_2_…`.
- A session sets a row to Draft when the issue is written and to Implemented when the piece is built. Only a person sets Verified.

## Safe defaults

- The program refuses, never a model: what a rule can decide is decided by code.
- Agents never merge.
- Where a level, a class or a runner is not given, the most restrictive applies.
