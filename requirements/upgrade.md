# Upgrading the pin

What `wb upgrade` writes to pin a repository to the installed revision.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-UPGRADE-1 | `wb upgrade` shall make `.watchbill/revision/` hold exactly the files of the installed revision, and `.watchbill/revision.sha256` its manifest. Issue: #174. | T | Implemented |
| WB-UPGRADE-2 | `wb upgrade` shall set the lines `watchbill:` and `revision:` of `watchbill.yml` to the installed version and hash. Issue: #174. | T | Implemented |
| WB-UPGRADE-3 | `wb upgrade` shall add each pattern that the installed `wb init` would write for the repository and that `watchbill.yml` does not have. Issue: #174. | T | Implemented |
| WB-UPGRADE-4 | `wb upgrade` shall leave every other line of `watchbill.yml` as it is. Issue: #174. | T | Implemented |
| WB-UPGRADE-5 | `wb upgrade` shall say each file it adds, changes or removes, and each line it sets or adds, before it writes. Issue: #174. | T | Implemented |
| WB-UPGRADE-6 | `wb upgrade --dry-run` shall say what it would write, and write nothing. Issue: #174. | T | Implemented |
| WB-UPGRADE-7 | `wb upgrade` shall end with 0 and write nothing when the repository is at the installed revision. Issue: #174. | T | Implemented |
| WB-UPGRADE-8 | `wb upgrade` shall refuse with 1, writing nothing, when a file under `.watchbill/` has a change that is not committed. Issue: #174. | T | Implemented |
| WB-UPGRADE-9 | `wb upgrade` shall refuse with 1, writing nothing, when the installed version is older than the pinned one. Issue: #174. | T | Implemented |
| WB-UPGRADE-10 | `wb upgrade` shall run the check of `wb check` after it writes, and say its result. Issue: #174. | T | Implemented |
| WB-UPGRADE-11 | `wb upgrade` shall commit nothing, and say the command that commits its change. Issue: #174. | T | Implemented |
| WB-UPGRADE-12 | `wb upgrade` in a repository with no `watchbill.yml` shall end with 2, naming `wb init`. Issue: #174. | T | Implemented |
