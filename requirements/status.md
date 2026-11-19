# Where each issue stands

What `wb status` says of the issues in the flow. It only reads.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-STATUS-1 | `wb status` shall print a heading for each step that has issues, and under it one line per issue: its number and its title, with the control characters of the title escaped. A heading with no issue shall not be printed, and no line shall hold a round, a command to run or a word for a step going on. Issue: #139. | T | Implemented |
| WB-STATUS-2 | `wb status` shall put an issue under the first heading that holds: `Fixed` (the last stamp is `[stamp] fix`), `Reviewed` (the last stamp is `[stamp] review`), `Built` (a `[stamp] build`), `Tested` (a `[stamp] tester`), `Accepted` (the issue's file and an acceptance that holds for it, on the issue's branch), `Broken down` (the issue's file on its branch), `Not broken down`; and shall print the headings in the order of the flow, `Not broken down` first and `Fixed` last. Issue: #139. | T | Implemented |
| WB-STATUS-3 | `wb status` shall count a `[question]` as open until an `[answer]` of a person closes it, and a line that starts with `[question]` shall never be an answer. Until `wb` reads who wrote a comment, every `[answer]` line counts as a person's. Issue: #139. | T | Implemented |
| WB-STATUS-4 | `wb status` shall take the issues and their titles from the pattern `open_issues` of `watchbill.yml`, and without it the issues that have a branch of the pattern `branch` or a file under `.watchbill/issues/`, with no title. Issue: #139. | T | Implemented |
| WB-STATUS-5 | `wb status <issue>` shall print the heading and the line of that issue and, under them, the lines it was worked out from. Issue: #139. | T | Implemented |
| WB-STATUS-6 | `wb status --stage <heading>` shall print only that heading, the name matched without regard to case, and a heading that is not known shall end with 2. Issue: #139. | T | Implemented |
| WB-STATUS-7 | When a pattern fails for one issue, `wb status` shall list that issue under `Not read` with why, print the others, and end with 1. Issue: #139. | T | Implemented |
| WB-STATUS-8 | `wb status` shall write nothing, switch no branch, read the issue's file and its acceptance with `git show`, and ask the network only through `open_issues` and `issue_comments`; it shall end with 2 when used wrongly or when Watchbill is not set up. Issue: #139. | T | Implemented |
| WB-STATUS-9 | After the steps, `wb status` shall print `Questions open`: each issue that has an open question, with the names of those questions in the order they were asked; the issue shall also stay under its step. Issue: #139. | T | Implemented |
