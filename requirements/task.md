# The task's file

What `wb` reads of a task's file and how a person accepts it. What the file is: [docs/revision.md](../docs/revision.md), point 13.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-TASK-1 | `wb accept <task>` shall read `.watchbill/tasks/<task>.yml` through the one loader. Issue: #74. | T | Implemented |
| WB-TASK-2 | `wb accept` shall name a key of the task's file that is missing, out of order or not known, and end with 1. Issue: #74. | T | Implemented |
| WB-TASK-3 | `wb accept` shall name a requirement ID that stands twice in the task's file, and end with 1. Issue: #74. | T | Implemented |
| WB-TASK-4 | `wb accept` shall name a path of the task's file that is absolute, holds `..` or lies under `.watchbill/`, and end with 1. Issue: #74. | T | Implemented |
| WB-TASK-5 | `wb accept` shall refuse a task's file that holds a question, name the questions, and end with 1. Issue: #74. | T | Implemented |
| WB-TASK-6 | `wb accept` shall say what it will write before it writes `.watchbill/tasks/<task>.accepted`. Issue: #74. | T | Implemented |
| WB-TASK-7 | The acceptance shall hold the SHA-256 of the task's file, the git `user.name` and `user.email`, and the time in UTC. Issue: #74. | T | Implemented |
| WB-TASK-8 | `wb accept` shall never write over an acceptance that is there, and shall end with 1. Issue: #74. | T | Implemented |
| WB-TASK-9 | `wb accept` shall refuse when git gives no `user.name` or `user.email`, and end with 1. Issue: #74. | T | Implemented |
| WB-TASK-10 | A task's file shall count as accepted only while the SHA-256 in its acceptance is that of the file as it is. Issue: #74. | T | Implemented |
| WB-TASK-11 | `wb check` shall not fail on the files under `.watchbill/tasks/`. Issue: #74. | T | Implemented |
| WB-TASK-12 | `wb accept` shall write nothing but the acceptance, and ask nothing of the network. Issue: #74. | T | Implemented |
