# The issue's file

What `wb` reads of an issue's file and how a person accepts it. What the file is: [docs/revision.md](../docs/revision.md), point 13. The area keeps the name `WB-TASK`, as an ID is never used again: until revision 0.0.6 the issue was called the task (#78).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-TASK-1 | `wb accept <issue>` shall read `.watchbill/issues/<issue>.yml` through the one loader. Issue: #74. | T | Implemented |
| WB-TASK-2 | `wb accept` shall name a key of the issue's file that is missing, out of order or not known, and end with 1. Issue: #74. | T | Implemented |
| WB-TASK-3 | `wb accept` shall name a requirement ID that stands twice in the issue's file, and end with 1. Issue: #74. | T | Implemented |
| WB-TASK-4 | `wb accept` shall name a path of the issue's file that is absolute, holds `..` or lies under `.watchbill/`, and end with 1. Issue: #74. | T | Implemented |
| WB-TASK-5 | `wb accept` shall refuse an issue's file that holds a question, name the questions, and end with 1. Issue: #74. | T | Implemented |
| WB-TASK-6 | `wb accept` shall say what it will write before it writes `.watchbill/issues/<issue>.accepted`. Issue: #74. | T | Implemented |
| WB-TASK-7 | The acceptance shall hold the SHA-256 of the issue's file, the git `user.name` and `user.email`, and the time in UTC. Issue: #74. | T | Implemented |
| WB-TASK-8 | `wb accept` shall never write over an acceptance that is there, and shall end with 1. Issue: #74. | T | Implemented |
| WB-TASK-9 | `wb accept` shall refuse when git gives no `user.name` or `user.email`, and end with 1. Issue: #74. | T | Implemented |
| WB-TASK-10 | An issue's file shall count as accepted only while the SHA-256 in its acceptance is that of the file as it is. Issue: #74. | T | Implemented |
| WB-TASK-11 | `wb check` shall not fail on the files under `.watchbill/issues/`. Issue: #74. | T | Implemented |
| WB-TASK-12 | `wb accept` shall write nothing but the acceptance, and ask nothing of the network. Issue: #74. | T | Implemented |
| WB-TASK-13 | For the tester, the build, the review and the fix, `wb prompt` shall end with 1, print nothing on standard output and name the file when the issue's file or its acceptance is not committed in `HEAD` or differs from it in the working tree. Issue: #97. | T | Implemented |
| WB-TASK-14 | For the tester, the build, the review and the fix, `wb prompt` shall end with 1, print nothing on standard output and name the missing file when the issue has no file, unless `watchbill.yml` says `breakdown: none`. Issue: #97. | T | Implemented |
| WB-TASK-15 | `wb` shall refuse `breakdown: none` in `watchbill.yml` when the revision's rule table holds the breakdown. Issue: #97. | T | Implemented |
| WB-TASK-16 | `wb accept` shall end by saying that the acceptance is to be committed alone, with the command. Issue: #97. | T | Implemented |
