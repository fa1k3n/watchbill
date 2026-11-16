"""`wb init`: sets Watchbill up in a repository, the way `git init` starts one.

It proposes what the repository says about itself, asks one line per answer,
shows the file and writes `.watchbill/watchbill.yml`. A file that is already
there is never changed.
"""

from __future__ import annotations

import os
import re
from dataclasses import replace
from typing import Callable, Mapping, Optional

from watchbill.detect import NONE, Setup, detect

CONFIG_FOLDER = ".watchbill"
CONFIG_FILE = "watchbill.yml"

# `ask` gives the next line that was typed, or None when there are no more.
Ask = Callable[[str], Optional[str]]
Say = Callable[[str], None]

_QUESTIONS = [
    ("project", "project"),
    ("git_server", "git server (github, gitlab, gerrit, bitbucket or none)"),
    ("repository", "repository at the git server"),
    ("tracker", "tracker (github, gitlab, jira or none)"),
    ("requirements", "requirements (a path to files, the name of a tool, or none)"),
    ("ci", "ci driver (github-actions, gitlab-ci, jenkins or none)"),
]

_PROJECT_NAME = re.compile(r"[a-z0-9][a-z0-9._-]*")
_CONTROL = re.compile(r"[\x00-\x1f\x7f]")


def _fault(key: str, answer: str) -> Optional[str]:
    """Why an answer cannot be taken, or None when it can."""
    if len(answer) > 200:
        return "that is over 200 characters"
    # A control character would break the line of the file.
    if _CONTROL.search(answer):
        return "that holds a character that cannot be written"
    if key == "project" and not _PROJECT_NAME.fullmatch(answer):
        return (
            "a project name is lower-case letters, digits, '.', '_' and '-', "
            "and starts with a letter or a digit"
        )
    return None


# Plain only when YAML is sure to read it back as the same text: a word that starts with a letter,
# and is not one of the words YAML reads as a truth value or as nothing; or a version of
# three numbers or more, which no YAML number or date has the form of.
_PLAIN = re.compile(r"[A-Za-z][A-Za-z0-9_./*-]*")
_WORDS = {"y", "n", "yes", "no", "true", "false", "on", "off", "null"}
_VERSION = re.compile(r"\d+\.\d+\.\d+[A-Za-z0-9._+-]*")


def _yaml_value(value: str) -> str:
    """A value as YAML reads it back unchanged: plain when that is certain, else in single quotes."""
    if _VERSION.fullmatch(value) or (_PLAIN.fullmatch(value) and value.lower() not in _WORDS):
        return value
    return "'" + value.replace("'", "''") + "'"


def render(revision: str, setup: Setup) -> str:
    """The file `wb init` writes."""
    rows = [
        ("watchbill", revision),
        ("project", setup.project),
        ("git_server", setup.git_server),
        ("repository", setup.repository),
        ("tracker", setup.tracker),
        ("requirements", setup.requirements),
        ("ci", setup.ci),
    ]
    lines = [
        "# How Watchbill is set up in this repository. Written by `wb init`.",
        "# `watchbill` is the version of the package the repository is set up for.",
    ]
    lines += ["{}: {}".format(key, _yaml_value(value)) for key, value in rows]
    return "\n".join(lines) + "\n"


def _answers(found: Setup, ask: Ask, say: Say) -> Setup:
    setup = replace(found)
    for key, text in _QUESTIONS:
        if key == "repository" and setup.git_server == NONE:
            setup.repository = NONE
            continue
        while True:
            line = ask("{}: ({}) ".format(text, getattr(setup, key)))
            answer = (line or "").strip()
            if not answer:
                break
            why = _fault(key, answer)
            if why is None:
                setattr(setup, key, answer)
                break
            say("Not taken: {}.".format(why))
    return setup


def init(
    root: str,
    revision: str,
    yes: bool,
    ask: Ask,
    say: Say,
    env: Optional[Mapping[str, str]] = None,
) -> int:
    """Runs `wb init` in the repository at `root`. Returns the exit status."""
    folder = os.path.join(root, CONFIG_FOLDER)
    file = os.path.join(folder, CONFIG_FILE)
    if os.path.lexists(file):
        say("{} is already there. wb init does not change it.".format(file))
        return 1

    setup = detect(root, env)
    if not yes:
        say("This sets Watchbill {} up in {}.".format(revision, root))
        say("It asks a few things and writes {}/{}. What it found in the".format(CONFIG_FOLDER, CONFIG_FILE))
        say("repository stands in brackets: Enter keeps it. Ctrl-C leaves at any time.")
        say("")
        setup = _answers(setup, ask, say)

    text = render(revision, setup)
    say("\nAbout to write {}:\n\n{}".format(file, text))
    if not yes:
        answer = ask("Is this OK? (yes) ")
        # No answer at all, because the input ended, is no yes.
        if answer is None or answer.strip().lower() not in ("", "y", "yes"):
            say("Nothing written.")
            return 1

    os.makedirs(folder, exist_ok=True)
    # `x`: never over a file that came to be there meanwhile.
    with open(file, "x", encoding="utf-8", newline="\n") as out:
        out.write(text)
    say("Wrote {}.".format(file))
    say("Keep it under version control with the code.")
    return 0
