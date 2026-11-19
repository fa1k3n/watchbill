# Taking a step back

What `wb revert` posts, and how `wb` reads the marker. Nothing at the tracker is deleted.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-REVERT-1 | `wb revert <issue> <role>` shall post, through `issue_comment`, one comment whose first line is `[revert] <role> round <n>`, for the last round of that role, or the round of `--round`, after saying what it will post. Issue: #142. | T | Implemented |
| WB-REVERT-2 | A `[stamp]`, `[question]` or `[answer]` from the reverted stamp up to the marker shall not count for the round that `wb <role>` and `wb prompt` work out, nor for `wb status`. Issue: #142. | T | Implemented |
| WB-REVERT-3 | `wb revert` shall leave the branch, the issue's file, its acceptance and the change as they are, and say on standard error that taking commits back is git's. Issue: #142. | T | Implemented |
| WB-REVERT-4 | `wb revert <issue> breakdown` shall be refused with 1, saying that the issue's file and its acceptance are taken back with git. Issue: #142. | T | Implemented |
| WB-REVERT-5 | `wb revert` shall refuse with 1 a role that has no stamp on the issue, and end with 2 for a role the revision does not have. Issue: #142. | T | Implemented |
| WB-REVERT-6 | A `[revert]` line shall count only as the first line of a comment. Issue: #142. | T | Implemented |
| WB-REVERT-7 | `wb revert --change <n> <role>` shall post the marker on the change through `change_comment`. Issue: #142. | T | Implemented |
