# The revision pinned in a repository

What `wb init` writes of the revision into the repository it sets up. What a revision is: [docs/revision.md](../docs/revision.md).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-PIN-1 | `wb init` shall write the files of the revision under `.watchbill/revision/`, byte for byte as the package has them. Issue: #7. | T | Draft |
| WB-PIN-2 | `wb init` shall write the manifest of the revision as `.watchbill/revision.sha256`. Issue: #7. | T | Draft |
| WB-PIN-3 | `wb init` shall write the hash of the revision as the line `revision:` of `.watchbill/watchbill.yml`. Issue: #7. | T | Draft |
| WB-PIN-4 | When any file `wb init` would write is already there, it shall write nothing at all and name the file. Issue: #7. | T | Draft |
| WB-PIN-5 | Before it writes, `wb init` shall name every file it is about to write. Issue: #7. | T | Draft |
| WB-PIN-6 | `sha256sum -c` on the written manifest, run in `.watchbill/`, shall pass. Issue: #7. | T | Draft |
