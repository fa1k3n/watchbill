"""The manifest of the revision: its files, the SHA-256 of each, and the hash of the revision.

A file is hashed as the bytes that are shipped, and nothing is rewritten first. A file that is not
in the fixed form (UTF-8, LF line endings, one line break at the end) is refused by name
(docs/revision.md, point 8). The manifest has one line per file in the form `sha256sum` prints,
sorted by path, with the paths relative to the folder that holds `revision/`.
"""

import hashlib
import os
import re
from typing import List, Optional, Tuple

__all__ = [
    "PACKAGE_FOLDER", "MANIFEST_FILE", "RELEASED_FILE", "ManifestError",
    "files", "manifest", "hash_of", "revision_hash", "released",
]

# The folder that holds `revision/`, the manifest and `revisions.txt`: this package.
PACKAGE_FOLDER = os.path.dirname(os.path.abspath(__file__))
MANIFEST_FILE = "revision.sha256"
RELEASED_FILE = "revisions.txt"

HASH = re.compile(r"^[0-9a-f]{64}$")


class ManifestError(Exception):
    """A file of the revision is not in the fixed form, or `revisions.txt` is wrong. The message names it."""


def files(base: Optional[str] = None) -> List[str]:
    """The paths of the files under `revision/`, relative to `base`, with `/`, sorted by path.

    The order is that of the code points of the path: no locale, and not the order of the file system.
    """
    base = PACKAGE_FOLDER if base is None else base
    found = []
    for folder, _, names in os.walk(os.path.join(base, "revision")):
        for name in names:
            path = os.path.relpath(os.path.join(folder, name), base)
            found.append("/".join(path.split(os.sep)))
    return sorted(found)


def _hash(base: str, path: str) -> str:
    with open(os.path.join(base, *path.split("/")), "rb") as file:
        data = file.read()
    try:
        data.decode("utf-8")
    except UnicodeDecodeError:
        raise ManifestError("{}: is not UTF-8".format(path))
    if b"\r" in data:
        raise ManifestError("{}: has CR in it; the line endings are LF".format(path))
    if not data.endswith(b"\n"):
        raise ManifestError("{}: is empty or does not end with a line break".format(path))
    return hashlib.sha256(data).hexdigest()


def manifest(base: Optional[str] = None) -> str:
    """The manifest of the files under `revision/` in `base`: `<hash>  <path>`, one line each."""
    base = PACKAGE_FOLDER if base is None else base
    return "".join("{}  {}\n".format(_hash(base, path), path) for path in files(base))


def hash_of(text: str) -> str:
    """The hash of the revision, given its manifest: the SHA-256 of the manifest."""
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def revision_hash(base: Optional[str] = None) -> str:
    """The hash of the revision: the SHA-256 of the manifest."""
    return hash_of(manifest(base))


def released(path: Optional[str] = None) -> List[Tuple[str, str]]:
    """The versions of `revisions.txt` with the hash of each: lines of `<version>  <hash>`."""
    path = os.path.join(PACKAGE_FOLDER, RELEASED_FILE) if path is None else path
    name = os.path.basename(path)
    with open(path, encoding="utf-8") as file:
        lines = file.read().splitlines()
    found: List[Tuple[str, str]] = []
    for number, line in enumerate(lines, 1):
        parts = line.split("  ")
        if len(parts) != 2 or not parts[0] or " " in parts[0] or not HASH.match(parts[1]):
            raise ManifestError("{}: line {} is not `<version>  <hash>`".format(name, number))
        if parts[0] in [version for version, _ in found]:
            raise ManifestError("{}: line {}: version {} stands twice".format(name, number, parts[0]))
        found.append((parts[0], parts[1]))
    return found
