<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/logo-dark.svg">
    <img src="docs/logo-light.svg" width="96" height="96" alt="The mark of Watchbill: a grid of three by three rounded squares, the diagonal filled in another colour">
  </picture>
</p>

# Watchbill

A harness for AI agents in embedded software development, deployed at a fixed revision onto a project that already exists.

**Status: an idea, written down 2026-11-16. Only `wb init` is built, and nothing reads the file it writes yet.**

## What it is to be

- **Deployed onto what the project already has.** The requirements tool, the tracker, the git server and the CI driver stay as they are.
- **The agents show up where the team works.** When Watchbill is deployed, the agents appear in the team's own tools, not in a new place to look.
- **One fixed revision.** A deployment is one revision of the harness, and the prompts are under version control.
- **Traceable.** What an agent did, for which requirement, under which revision, can be followed afterwards.
- **A method that can be certified.** The aim is that a team's process with Watchbill in it can pass an assessment against an ISO standard. Watchbill is not certified, and nothing here says it is.

## Setting it up in a repository

The command is `wb`. Watchbill is set up in a repository the way `git init` starts one: a single command, run in the folder.

```sh
cd brake-controller
wb init
```

`wb init` proposes what the repository says about itself, asks one line per answer, shows the file and writes it:

```
project: (brake-controller)
git server (github, gitlab, gerrit, bitbucket or none): (gitlab)
repository at the git server: (brakes/controller)
tracker (github, gitlab, jira or none): (gitlab) jira
requirements (a path to files, the name of a tool, or none): (requirements/*.md)
ci driver (github-actions, gitlab-ci, jenkins or none): (gitlab-ci)
```

Enter keeps what stands in brackets. `wb init --yes` takes all of it and asks nothing. The result is `.watchbill/watchbill.yml` at the root of the repository, to be kept under version control with the code:

```yaml
watchbill: 0.0.1
project: brake-controller
git_server: gitlab
repository: brakes/controller
tracker: jira
requirements: requirements/*.md
ci: gitlab-ci
```

- **`watchbill` is the revision of the harness** the repository is set up for: the version of the `wb` that wrote the file.
- **What it finds, it finds in the repository:** the name of the folder, the address of `origin` (only its host and path; a user name or a token in it is dropped), a folder of requirements, and the file a CI driver is known by. It asks nothing of the network.
- **A file that is already there is never changed.**
- **The answers are only written down.** No adapter exists yet, so a git server or a tracker is a name and nothing more.

## Installing it

Watchbill is written in Python and uses only its standard library, so the Python of the distribution runs it: 3.9 or later, which is what Enterprise Linux 9 has. It needs `git`, and nothing else.

On Fedora and Enterprise Linux it is an RPM package, installed and removed with `dnf` like any other program:

```sh
packaging/rpm.sh                                                  # builds the package from this checkout
sudo dnf install build/rpm/RPMS/noarch/watchbill-*.noarch.rpm     # puts wb in /usr/bin
```

The package is built here from a checkout. There is no repository to `dnf install watchbill` from yet. Building needs `rpm-build`, `python3-devel`, `pyproject-rpm-macros`, `python3-setuptools`, `python3-pip` and `python3-wheel`, and on Enterprise Linux 9 also `python3-tomli` and the CRB repository.

To try it without installing anything, from a checkout:

```sh
python3 -m unittest discover -s tests            # every test
PYTHONPATH=/path/to/watchbill/src python3 -m watchbill init
```

## The name

A watch bill is a ship's posted list of who stands which watch at which station.

## Beside Handover Ledger

Watchbill is a product of its own. [Handover Ledger](https://github.com/fa1k3n/handover-ledger) records the handovers in a team and can be used to monitor a team that runs Watchbill. Watchbill does not need it.

## The revision

What a revision is, and each decision behind it, is in [docs/revision.md](docs/revision.md).

## Not decided

- The licence.
- Which standard comes first, and which tools the first adapters are for.
