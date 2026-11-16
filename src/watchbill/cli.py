"""The command `wb`."""

from __future__ import annotations

import subprocess
import sys
from typing import List, Optional

from watchbill import __version__
from watchbill.init import init

USAGE = """usage:
  wb init [--yes]
  wb --version

  init      set Watchbill up in the git repository of this folder: propose what
            the repository says about itself (its name, the git server of
            origin, a folder of requirements, the CI driver), ask one line per
            answer, show the file and write .watchbill/watchbill.yml at the
            root of the repository. --yes (-y) takes what it found and asks
            nothing. A file that is already there is never changed
"""


def _repository_root() -> Optional[str]:
    """The root of the git repository the command runs in, or None outside one."""
    try:
        git = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            universal_newlines=True,
        )
    except OSError:
        return None
    root = git.stdout.strip() if git.returncode == 0 else ""
    return root or None


def _ask(prompt: str) -> Optional[str]:
    """A question on standard output and its answer, one line from standard input."""
    try:
        return input(prompt)
    except EOFError:
        # No more input: end the line of the question, so what follows starts on its own.
        print()
        return None


def _say(text: str) -> None:
    print(text)


def main(argv: List[str]) -> int:
    command = argv[0] if argv else None
    if command in (None, "--help", "-h", "help"):
        sys.stdout.write(USAGE)
        return 2 if command is None else 0
    if command in ("--version", "-v"):
        print(__version__)
        return 0
    if command != "init":
        sys.stderr.write("wb: no command '{}'\n\n{}".format(command, USAGE))
        return 2

    yes = False
    for arg in argv[1:]:
        if arg in ("--yes", "-y"):
            yes = True
        else:
            sys.stderr.write("wb init: '{}' is not known\n\n{}".format(arg, USAGE))
            return 2

    try:
        root = _repository_root()
        if root is None:
            sys.stderr.write("wb init: this folder is not in a git repository. Watchbill is set up in one.\n")
            return 1
        return init(root, __version__, yes, _ask, _say)
    except KeyboardInterrupt:
        # 1: it was not done. The file is only written at the very end.
        print("\nNothing written.")
        return 1


def run() -> None:
    sys.exit(main(sys.argv[1:]))
