# The revision pinned in a repository

What `wb init` writes of the revision into the repository it sets up. What a revision is: [docs/revision.md](../docs/revision.md).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-PIN-1 | `wb init` shall write the files of the revision under `.watchbill/revision/`, byte for byte as the package has them. Issue: #7. | T | Implemented |
| WB-PIN-2 | `wb init` shall write the manifest of the revision as `.watchbill/revision.sha256`. Issue: #7. | T | Implemented |
| WB-PIN-3 | `wb init` shall write the hash of the revision as the line `revision:` of `.watchbill/watchbill.yml`. Issue: #7. | T | Implemented |
| WB-PIN-4 | When any file `wb init` would write is already there, it shall write nothing at all and name the file. Issue: #7. | T | Implemented |
| WB-PIN-5 | Before it writes, `wb init` shall name every file it is about to write. Issue: #7. | T | Implemented |
| WB-PIN-6 | `sha256sum -c` on the written manifest, run in `.watchbill/`, shall pass. Issue: #7. | T | Implemented |
| WB-PIN-7 | When a folder on the way to a file `wb init` would write, from `.watchbill` down, is a link or is not a folder, `wb init` shall write nothing, name that path and say why: it is a link, or it is not a folder. It shall stop before it asks. Issue: #7. Changed by #53. | T | Implemented |
| WB-PIN-8 | When `.watchbill/revision` is already there, whatever it is and whatever it holds, `wb init` shall write nothing, name it, and stop before it asks. Issue: #7. | T | Implemented |
| WB-PIN-9 | When writing a file fails part way, `wb init` shall remove what it made, leave what was there before, and end with 1. Issue: #7. | T | Implemented |
| WB-PIN-10 | When a file `wb init` would write is not below `.watchbill`, `wb init` shall take it as in the way, write nothing and name it. Issue: #53. | T | Implemented |
