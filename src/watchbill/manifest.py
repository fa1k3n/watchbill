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


def _shown(path: str) -> str:
    """A path as text that can be printed, also when its name is not UTF-8."""
    return path.encode("utf-8", "backslashreplace").decode("utf-8")


def _refuse_name(base: str, path: str, name: str, is_folder: bool) -> None:
    """Refuse a path the manifest cannot carry safely, by name. `path` is relative to `base`."""
    try:
        path.encode("utf-8")
    except UnicodeEncodeError:
        raise ManifestError("{}: the name is not UTF-8".format(_shown(path)))
    if os.path.islink(os.path.join(base, *path.split("/"))):
        raise ManifestError("{}: is a symbolic link; the revision holds only plain files".format(path))
    if name.startswith("."):
        raise ManifestError("{}: is hidden; the revision holds no hidden {}".format(path, "folder" if is_folder else "file"))
    if not is_folder and name.endswith("~"):
        raise ManifestError("{}: is a backup file".format(path))
    if "\n" in name or "\r" in name or "\\" in name:
        # `sha256sum` writes such a name escaped, and the manifest is in its form.
        raise ManifestError("{}: the name has a line break or a backslash in it".format(_shown(path).replace("\n", "\\n").replace("\r", "\\r")))


def files(base: Optional[str] = None) -> List[str]:
    """The paths of the files under `revision/`, relative to `base`, with `/`, sorted by path.

    The order is that of the code points of the path: no locale, and not the order of the file system.
    A path the manifest cannot carry safely is refused by name, and so is a revision that holds no file.
    """
    base = PACKAGE_FOLDER if base is None else base
    found = []
    for folder, folders, names in os.walk(os.path.join(base, "revision")):
        folders.sort()
        names.sort()
        for name in folders + names:
            path = "/".join(os.path.relpath(os.path.join(folder, name), base).split(os.sep))
            _refuse_name(base, path, name, name in folders)
        found.extend("/".join(os.path.relpath(os.path.join(folder, name), base).split(os.sep)) for name in names)
    if not found:
        raise ManifestError("revision/: holds no file, or is not there")
    return sorted(found)


def _hash(base: str, path: str) -> str:
    try:
        with open(os.path.join(base, *path.split("/")), "rb") as file:
            data = file.read()
    except OSError as error:
        raise ManifestError("{}: cannot be read ({})".format(path, error.strerror or error))
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
    try:
        with open(path, "rb") as file:
            data = file.read()
    except OSError as error:
        raise ManifestError("{}: cannot be read ({})".format(name, error.strerror or error))
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError:
        raise ManifestError("{}: is not UTF-8".format(name))
    if "\r" in text:
        raise ManifestError("{}: has CR in it; the line endings are LF".format(name))
    if text and not text.endswith("\n"):
        raise ManifestError("{}: does not end with a line break".format(name))
    lines = text.split("\n")[:-1]
    found: List[Tuple[str, str]] = []
    for number, line in enumerate(lines, 1):
        parts = line.split("  ")
        if len(parts) != 2 or not parts[0] or " " in parts[0] or not HASH.match(parts[1]):
            raise ManifestError("{}: line {} is not `<version>  <hash>`".format(name, number))
        if parts[0] in [version for version, _ in found]:
            raise ManifestError("{}: line {}: version {} stands twice".format(name, number, parts[0]))
        found.append((parts[0], parts[1]))
    return found
