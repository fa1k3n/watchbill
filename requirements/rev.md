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
