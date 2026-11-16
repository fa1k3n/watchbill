"""What `wb init` proposes for a repository, read from the repository itself.

The name of its folder, the address of `origin`, and the files that a CI driver
or a folder of requirements leaves behind. Nothing here asks the network, and
nothing but the host and the path is kept of an address.
"""

from __future__ import annotations

import os
import re
import subprocess
from dataclasses import dataclass
from typing import Callable, List, Mapping, Optional, Tuple

NONE = "none"


@dataclass
class Setup:
    """The answers of `wb init`, one per line of the file it writes."""

    project: str
    git_server: str
    repository: str
    tracker: str
    requirements: str
    ci: str


@dataclass(frozen=True)
class Remote:
    host: str
    port: str
    path: str


# scheme://[user[:password]@]host[:port]/path
_FULL = re.compile(r"[a-z][a-z0-9+.-]*://(?:[^@/]*@)?([^/:@]+)(?::(\d+))?/(.+)", re.IGNORECASE)
# [user@]host:path
_SHORT = re.compile(r"(?:[^@/:]+@)?([^/:@]+):(?!/)(.+)")


def _clean(path: str) -> str:
    return re.sub(r"\.git$", "", path.strip("/"))


def parse_remote(url: str) -> Optional[Remote]:
    """The host, port and path of a git address; a user name, password or token in it is dropped."""
    text = url.strip()
    full = _FULL.fullmatch(text)
    if full:
        return Remote(full.group(1).lower(), full.group(2) or "", _clean(full.group(3)))
    short = _SHORT.fullmatch(text)
    if short:
        return Remote(short.group(1).lower(), "", _clean(short.group(2)))
    return None


def git_server_of(remote: Optional[Remote]) -> str:
    """The kind of git server an address points at, as far as the address can tell."""
    if remote is None:
        return NONE
    host = remote.host
    if host == "github.com":
        return "github"
    if "gitlab" in host:
        return "gitlab"
    if "gerrit" in host or remote.port == "29418":
        return "gerrit"
    if "bitbucket" in host:
        return "bitbucket"
    if host == "dev.azure.com" or host.endswith(".visualstudio.com"):
        return "azure-devops"
    return "other"


def project_name(folder: str) -> str:
    """`My Project` -> `my-project`: a name that is safe as a folder and in an address."""
    name = os.path.basename(os.path.normpath(folder)).lower()
    name = re.sub(r"[^a-z0-9._-]+", "-", name)
    name = re.sub(r"^[^a-z0-9]+|[^a-z0-9]+$", "", name)
    return name or "project"


def _files_in(folder: str) -> List[str]:
    try:
        return [name for name in os.listdir(folder) if os.path.isfile(os.path.join(folder, name))]
    except OSError:
        return []


_REQUIREMENT_FOLDERS = ["requirements", "docs/requirements", "doc/requirements"]


def _requirements_of(root: str) -> str:
    """The first folder of requirements that holds a file, with the file ending most of its files have."""
    for folder in _REQUIREMENT_FOLDERS:
        count: dict = {}
        for name in _files_in(os.path.join(root, folder)):
            ending = os.path.splitext(name)[1]
            if ending:
                count[ending] = count.get(ending, 0) + 1
        if count:
            ending = sorted(count, key=lambda e: (-count[e], e))[0]
            return "{}/*{}".format(folder, ending)
    return NONE


def _has(*path: str) -> Callable[[str], bool]:
    return lambda root: os.path.exists(os.path.join(root, *path))


# The CI drivers, each with the file or folder it is known by. The first that is found is proposed.
_CI_DRIVERS: List[Tuple[str, Callable[[str], bool]]] = [
    (
        "github-actions",
        lambda root: any(
            name.endswith((".yml", ".yaml")) for name in _files_in(os.path.join(root, ".github", "workflows"))
        ),
    ),
    ("gitlab-ci", _has(".gitlab-ci.yml")),
    ("jenkins", _has("Jenkinsfile")),
    ("azure-pipelines", _has("azure-pipelines.yml")),
    ("bitbucket-pipelines", _has("bitbucket-pipelines.yml")),
    ("circleci", _has(".circleci", "config.yml")),
    ("zuul", lambda root: _has(".zuul.yaml")(root) or _has("zuul.d")(root)),
]


def _origin(root: str, env: Optional[Mapping[str, str]]) -> str:
    """The address of `origin`, or an empty text when there is none."""
    try:
        git = subprocess.run(
            ["git", "-C", root, "remote", "get-url", "origin"],
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            universal_newlines=True,
            env=env,
        )
    except OSError:
        return ""
    return git.stdout.strip() if git.returncode == 0 else ""


def detect(root: str, env: Optional[Mapping[str, str]] = None) -> Setup:
    """What the repository in `root` says about itself."""
    remote = parse_remote(_origin(root, env))
    git_server = git_server_of(remote)
    return Setup(
        project=project_name(root),
        git_server=git_server,
        repository=(remote.path if remote and remote.path else NONE),
        # The tracker of a code host is the likely one; any other is the team's to name.
        tracker=git_server if git_server in ("github", "gitlab") else NONE,
        requirements=_requirements_of(root),
        ci=next((name for name, found in _CI_DRIVERS if found(root)), NONE),
    )
