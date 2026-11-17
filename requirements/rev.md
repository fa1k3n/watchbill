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
| WB-REV-15 | Every path of the committed manifest, and the manifest and `revisions.txt`, shall match a pattern of `package_data` in `setup.cfg`. Issue: #6. | T | Implemented |
| WB-REV-16 | A file of the revision that cannot be read, or whose name is not UTF-8, shall be refused by name, with no stack trace. Issue: #6. | T | Implemented |
| WB-REV-17 | A file of the revision whose name has a line break, a carriage return or a backslash shall be refused by name. Issue: #6. | T | Implemented |
| WB-REV-18 | A symbolic link, a hidden file or folder and a file whose name ends with `~` under `revision/` shall be refused by name. Issue: #6. | T | Implemented |
| WB-REV-19 | `revisions.txt` that is not UTF-8, has CR, or has no line break at the end shall be refused by name. Issue: #6. | T | Implemented |
| WB-REV-20 | A folder under `revision/` that cannot be listed shall be refused by name, and its files shall not be left out in silence. Issue: #6. | T | Implemented |
| WB-REV-21 | Anything under `revision/` that is not a plain file or a folder, such as a named pipe, shall be refused by name and not opened. Issue: #6. | T | Implemented |
| WB-REV-22 | Every path of the committed manifest, and the manifest and `revisions.txt`, shall be among the files the RPM installs. Issue: #25. | T | Draft |
| WB-REV-23 | A `revision/` that is itself a symbolic link shall be refused by name. Issue: #24. | T | Implemented |
| WB-REV-24 | A file of the revision that has become a named pipe after its kind was looked at shall be refused by name, without `wb` waiting on it. Issue: #24. | T | Implemented |
| WB-REV-25 | No option and no environment variable shall change the folder `wb` reads the revision from. Issue: #24. | T | Implemented |
