# The revision in the package

The files of a revision as the package carries them, their manifest, and `wb revision`. What a revision is: [docs/revision.md](../docs/revision.md).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-REV-1 | The installed package shall hold every file under `revision/`, also when it is installed from the RPM. Issue: #6. | D | Implemented |
| WB-REV-2 | The manifest shall have one line per file of the revision, in the form `sha256sum` prints, sorted by path. Issue: #6. | T | Implemented |
| WB-REV-3 | The hash of the revision shall be the SHA-256 of the manifest. Issue: #6. | T | Implemented |
| WB-REV-4 | A file of the revision that has CRLF line endings or is not UTF-8 shall be refused by name. Issue: #6. | T | Implemented |
| WB-REV-5 | The committed manifest `revision.sha256` shall be the one computed from the files. Issue: #6. | T | Implemented |
| WB-REV-6 | `sha256sum -c` on the manifest, run in the folder that holds `revision/`, shall pass. Issue: #6. | T | Implemented |
| WB-REV-7 | Every prompt file named by a role of the rule table shall exist, and every prompt file shall be named by a role. Issue: #6. | T | Implemented |
| WB-REV-8 | `revisions.txt` shall hold each released version of the package with the hash of its revision, and the current version shall not stand there with another hash than the computed one. Issue: #6. | T | Implemented |
| WB-REV-9 | `wb revision` shall print the version of the package, the hash of the revision and the manifest, and end with 0. Issue: #6. | T | Implemented |
| WB-REV-10 | `wb revision` shall write nothing. Issue: #6. | T | Implemented |
| WB-REV-11 | A file of the revision that is empty or does not end with a line break shall be refused by name. Issue: #6. | T | Implemented |
| WB-REV-12 | `wb revision` shall end with 2 and print no revision when it is given an argument. Issue: #6. | T | Implemented |
| WB-REV-13 | `wb revision` shall end with 1, name the file and print no revision when a file of the revision is refused. Issue: #6. | T | Implemented |
| WB-REV-14 | `wb revision` shall end with 1, name `revision/` and print no revision when the folder holds no file. Issue: #6. | T | Implemented |
| WB-REV-15 | Every path of the committed manifest, and the manifest and `revisions.txt`, shall match a pattern of `package_data` in `setup.cfg`. Replaced by WB-REV-22. Issue: #6. | T | Deprecated |
| WB-REV-16 | A file of the revision that cannot be read, or whose name is not UTF-8, shall be refused by name, with no stack trace. Issue: #6. | T | Implemented |
| WB-REV-17 | A file of the revision whose name has a line break, a carriage return or a backslash shall be refused by name. Issue: #6. | T | Implemented |
| WB-REV-18 | A symbolic link, a hidden file or folder and a file whose name ends with `~` under `revision/` shall be refused by name. Issue: #6. | T | Implemented |
| WB-REV-19 | `revisions.txt` that is not UTF-8, has CR, or has no line break at the end shall be refused by name. Issue: #6. | T | Implemented |
| WB-REV-20 | A folder under `revision/` that cannot be listed shall be refused by name, and its files shall not be left out in silence. Issue: #6. | T | Implemented |
| WB-REV-21 | Anything under `revision/` that is not a plain file or a folder, such as a named pipe, shall be refused by name and not opened. Issue: #6. | T | Implemented |
| WB-REV-22 | Every path of the committed manifest, and the manifest and `revisions.txt`, shall be among the files the RPM installs. Issue: #25. | T | Implemented |
| WB-REV-23 | A `revision/` that is itself a symbolic link shall be refused by name. Issue: #24. | T | Implemented |
| WB-REV-24 | A file of the revision that has become a named pipe after its kind was looked at shall be refused by name, without `wb` waiting on it. Issue: #24. | T | Implemented |
| WB-REV-25 | No option and no environment variable shall change the folder `wb` reads the revision from. Issue: #24. | T | Implemented |
| WB-REV-26 | No file of the revision shall name a file that is not in the repository, such as `rules.py`. Issue: #38. | T | Implemented |
| WB-REV-27 | `wb compare [--from <path>] <revision> <revision>` shall name a revision by its version or by its full hash, read the installed revision and, with `--from`, the revision the repository at `<path>` is pinned to (its `.watchbill/revision/`), and end with 0 and print nothing when the two are the same. Issue: #179. | T | Implemented |
| WB-REV-28 | `wb compare` shall, when the revisions differ, print one line for each file `added:`, `removed:` or `changed:` with its path, sorted by path, then one empty line and the unified diff of each changed file in the same order, headed `--- a/` and `+++ b/` with the path, a being the first revision; and end with 1. Issue: #179. | T | Implemented |
| WB-REV-29 | `wb compare` shall end with 2 and print nothing on standard output when a revision is not found: a version or hash that is not the installed one or the pinned one, a repository with no `.watchbill/` or no line `revision:`, files that are not the revision its line names, or a version that names two different revisions. Issue: #179. | T | Implemented |
| WB-REV-30 | `wb compare` shall print the same for a revision named by its version and by its hash. Issue: #179. | T | Implemented |
| WB-REV-31 | `wb compare` shall read the files of a revision as `wb revision` does, refuse by name, with 2, a link, a hidden file or a file not in the fixed form, and never follow a link. Issue: #179. | T | Implemented |
| WB-REV-32 | The output of `wb compare` shall not change with the git configuration, the attributes or the external diff of the person: the diff is made by `git diff --no-index` with fixed options and no configuration. Issue: #179. | T | Implemented |
| WB-REV-33 | `wb compare` shall write nothing and ask nothing of the network. Issue: #179. | T | Implemented |
| WB-REV-34 | `wb compare` shall end with 2 and print the usage on standard error when it is not given two revisions, or `--from` with no path or twice, or a word it does not know. Issue: #179. | T | Implemented |
