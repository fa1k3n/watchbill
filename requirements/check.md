# `wb check`

Whether a repository still is at the revision it is pinned to, and whether the installed `wb` is that revision. What a revision is: [docs/revision.md](../docs/revision.md).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-CHECK-1 | `wb check` shall end with 0 when the files under `.watchbill/revision/`, the manifest, the `revision:` line and the installed `wb` agree. Issue: #8. | T | Implemented |
| WB-CHECK-2 | `wb check` shall end with 1 and name the file when a file under `.watchbill/revision/` does not have the hash its line of the manifest says. Issue: #8. | T | Implemented |
| WB-CHECK-3 | `wb check` shall end with 1 and name the file when a file of the manifest is missing. Issue: #8. | T | Implemented |
| WB-CHECK-4 | `wb check` shall end with 1 and name the file when a file under `.watchbill/revision/` has no line in the manifest. Issue: #8. | T | Implemented |
| WB-CHECK-5 | `wb check` shall end with 1 when the SHA-256 of the manifest is not the `revision:` line of `watchbill.yml`. Issue: #8. | T | Implemented |
| WB-CHECK-6 | `wb check` shall end with 1 and name both revisions when the repository is pinned to another version or hash than the installed `wb`. Issue: #8. | T | Implemented |
| WB-CHECK-7 | `wb check` shall end with 2 in a repository that is not set up and outside a git repository. Issue: #8. | T | Implemented |
| WB-CHECK-8 | `wb check` shall give the same result from a folder inside the repository as from its root. Issue: #8. | T | Implemented |
| WB-CHECK-9 | `wb check` shall write nothing and ask nothing of the network. Issue: #8. | T | Implemented |
| WB-CHECK-10 | Every value `wb init` can write into `watchbill.yml` shall be read back unchanged. Issue: #8. | T | Implemented |
