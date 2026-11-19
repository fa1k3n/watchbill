# Where each issue stands

What `wb status` says of the issues in the flow. It only reads.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-STATUS-1 | `wb status` shall print a heading for each step that has issues, and under it one line per issue with its number and its title, and no round, no command to run and no word that a step is going on. Issue: #139. | T | Draft |
| WB-STATUS-2 | `wb status` shall put an issue under the first step that holds, in the order `Fixed`, `Reviewed`, `Built`, `Tested`, `Accepted`, `Broken down`, `Not broken down`, each worked out as #139 says, and print the steps in the order of the flow, `Not broken down` first. Issue: #139. | T | Draft |
| WB-STATUS-3 | `wb status` shall count a `[question]` as open until a person has answered it, and an `[answer]` that a person did not write shall not close it. Issue: #139. | T | Draft |
| WB-STATUS-4 | `wb status` shall take the issues and their titles from the pattern `open_issues` of `watchbill.yml`, one to a line as a number, a tab and a title, and without it the issues that have a branch of the pattern `branch` or a file under `.watchbill/issues/`, with no title. Issue: #139. | T | Draft |
| WB-STATUS-5 | `wb status <issue>` shall print the heading and the line of that issue and the lines it was worked out from. Issue: #139. | T | Draft |
| WB-STATUS-6 | `wb status --stage <heading>` shall print only that heading, matched without regard to case, and a heading that is not known shall end with 2. Issue: #139. | T | Draft |
| WB-STATUS-7 | When a pattern fails for one issue, `wb status` shall list it under `Not read` with why, print the others, and end with 1. Issue: #139. | T | Draft |
| WB-STATUS-8 | `wb status` shall write nothing, switch no branch, and ask the network only through `open_issues` and `issue_comments`. Issue: #139. | T | Draft |
| WB-STATUS-9 | After the steps, `wb status` shall list under `Questions open` each issue that has an open question, with the names of its open questions, and the issue shall stay under its step. Issue: #139. | T | Draft |
