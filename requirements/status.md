# Where each issue stands

What `wb status` says of the issues in the flow. It only reads.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-STATUS-1 | `wb status` shall print one line per issue: the issue, its stage, and the command that comes next. Issue: #139. | T | Draft |
| WB-STATUS-2 | `wb status` shall give an issue the first stage that holds, in the order `needs a person`, `in review round <n>` or `fixed round <n>`, `built`, `tested`, `accepted`, `broken down`, `new`, each worked out as #139 says. Issue: #139. | T | Draft |
| WB-STATUS-3 | `wb status` shall count a `[question]` as open until a person has answered it, and an `[answer]` that a person did not write shall not close it. Issue: #139. | T | Draft |
| WB-STATUS-4 | `wb status` shall take the issues from the pattern `open_issues` of `watchbill.yml`, and without it from the branches of the pattern `branch` and the files under `.watchbill/issues/`. Issue: #139. | T | Draft |
| WB-STATUS-5 | `wb status <issue>` shall print the line of that issue and the lines it was worked out from. Issue: #139. | T | Draft |
| WB-STATUS-6 | `wb status --stage <stage>` shall print only the issues at that stage, and a stage that is not known shall end with 2. Issue: #139. | T | Draft |
| WB-STATUS-7 | When a pattern fails for one issue, `wb status` shall say so on that issue's line, print the others, and end with 1. Issue: #139. | T | Draft |
| WB-STATUS-8 | `wb status` shall write nothing, switch no branch, and ask the network only through `open_issues`, `issue_comments` and `change_of_branch`. Issue: #139. | T | Draft |
