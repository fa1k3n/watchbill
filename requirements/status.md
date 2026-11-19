# Where each issue stands

What `wb status` says of the issues in the flow. It only reads.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-STATUS-1 | `wb status` shall print a heading for each step that has issues, and under it one line per issue: its number and its title. Issue: #139. | T | Implemented |
| WB-STATUS-2 | `wb status` shall put an issue under the first heading that holds: `Fixed` (the last stamp is `[stamp] fix`), `Reviewed` (the last stamp is `[stamp] review`), `Built` (a `[stamp] build`), `Tested` (a `[stamp] tester`), `Accepted` (the issue's file and an acceptance that holds for it, on the issue's branch), `Broken down` (the issue's file on its branch), `Not broken down`. Issue: #139. | T | Implemented |
| WB-STATUS-3 | `wb status` shall count a `[question]` as open until an `[answer]` of a person to its name closes it. Issue: #139. | T | Implemented |
| WB-STATUS-4 | `wb status` shall take the issues and their titles from the pattern `open_issues` of `watchbill.yml`, and without it the issues that have a branch of the pattern `branch` or a file under `.watchbill/issues/`, with no title. Issue: #139. | T | Implemented |
| WB-STATUS-5 | `wb status <issue>` shall print the heading and the line of that issue and, under them, the lines it was worked out from. Issue: #139. | T | Implemented |
| WB-STATUS-6 | `wb status --stage <heading>` shall print only that heading, the name matched without regard to case. Issue: #139. | T | Implemented |
| WB-STATUS-7 | When a pattern fails for one issue, `wb status` shall list that issue under `Not read` with why, print the others, and end with 1. Issue: #139. | T | Implemented |
| WB-STATUS-8 | `wb status` shall write nothing. Issue: #139. | T | Implemented |
| WB-STATUS-9 | After the steps, `wb status` shall print `Questions open`: each issue that has an open question, with the names of those questions in the order they were asked. Issue: #139. | T | Implemented |
| WB-STATUS-10 | `wb status` shall print a title with its control characters escaped. Issue: #139. | T | Implemented |
| WB-STATUS-11 | `wb status` shall not print a heading that has no issue. Issue: #139. | T | Implemented |
| WB-STATUS-12 | `wb status` shall print no line that holds a round, a command to run or a word for a step going on. Issue: #139. | T | Implemented |
| WB-STATUS-13 | `wb status` shall print the steps in the order of the flow, `Not broken down` first and `Fixed` last. Issue: #139. | T | Implemented |
| WB-STATUS-14 | `wb status` shall never take a line that starts with `[question]` for an answer. Issue: #139. | T | Implemented |
| WB-STATUS-15 | Until `wb` reads who wrote a comment, `wb status` shall take every `[answer]` line for a person's. Issue: #139. | T | Implemented |
| WB-STATUS-16 | `wb status --stage` with a heading that is not known shall end with 2. Issue: #139. | T | Implemented |
| WB-STATUS-17 | `wb status` shall switch no branch. Issue: #139. | T | Implemented |
| WB-STATUS-18 | `wb status` shall read the issue's file and its acceptance from the branch with `git show`, so that an acceptance that is written and not committed leaves the issue under `Broken down`. Issue: #139. | T | Implemented |
| WB-STATUS-19 | `wb status` shall run no pattern but `open_issues` and `issue_comments`. Issue: #139. | T | Implemented |
| WB-STATUS-20 | `wb status` used wrongly, with an option or a word that is not known, shall end with 2. Issue: #139. | T | Implemented |
| WB-STATUS-21 | `wb status` in a folder that is not in a git repository, or where Watchbill is not set up, shall end with 2. Issue: #139. | T | Implemented |
| WB-STATUS-22 | `wb status` shall keep an issue that has an open question under its step. Issue: #139. | T | Implemented |
| WB-STATUS-23 | When `open_issues` fails, `wb status` shall say so on standard error, list no issue, and end with 1. Issue: #139. | T | Implemented |
| WB-STATUS-24 | `wb status <issue>` shall print the lines it worked out from under the heading `Worked out from:`. Issue: #139. | T | Implemented |
| WB-STATUS-25 | `wb status` shall list a line of `open_issues` whose first field is not a plain issue name under `Not read`, with why, and end with 1. Issue: #139. | T | Implemented |
| WB-STATUS-26 | Without `issue_comments`, `wb status` shall say so once on standard error. Issue: #139. | T | Implemented |
| WB-STATUS-27 | `wb --help` shall name `wb status`. Issue: #139. | T | Implemented |
| WB-STATUS-28 | `wb status` shall run from any folder inside the repository. Issue: #139. | T | Implemented |
| WB-STATUS-29 | `wb status` shall take `[answer] <issue> suggested` as an answer to every question asked before it, and to none asked after it. Issue: #139. | T | Implemented |
| WB-STATUS-30 | `wb status` shall close a question by an `[answer]` of its name whatever follows the name, a choice number too. Issue: #139. | T | Implemented |
