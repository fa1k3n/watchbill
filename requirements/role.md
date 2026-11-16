# The role prompts

The prompts of the first revision: tester, build, review and fix. What a revision is: [docs/revision.md](../docs/revision.md).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-ROLE-1 | The revision shall hold one prompt file, `roles/<role>.md`, for each of the roles tester, build, review and fix, and for no other role. Issue: #4. | T | Implemented |
| WB-ROLE-2 | Each role prompt shall have these four parts in this order: what the role produces, what it reads and must not read, what it must never do, and how it ends. Issue: #4. | T | Implemented |
| WB-ROLE-3 | The words a prompt is filled in with shall be one fixed list of placeholders, written in `revision/roles/placeholders.md`. Issue: #4. | T | Implemented |
| WB-ROLE-4 | A role prompt shall use no placeholder that is not on the list. Issue: #4. | T | Implemented |
| WB-ROLE-5 | Every placeholder on the list shall be used by at least one role prompt. Issue: #4. | T | Implemented |
| WB-ROLE-6 | No role prompt shall name a product of a git server, a tracker or a CI driver. Issue: #4. | T | Implemented |
| WB-ROLE-7 | Every role prompt shall be UTF-8 with LF line endings and end with one line break. Issue: #4. | T | Implemented |
| WB-ROLE-8 | Each role prompt of the first revision shall start with a line saying that it is a draft that has not yet run on a project. Issue: #4. | T | Implemented |
| WB-ROLE-9 | The tester's prompt shall say that the tester never reads the implementation. Issue: #4. | I | Implemented |
| WB-ROLE-10 | The reviewer's prompt shall say that the reviewer only reports, never fixes, and never reads how the change was made. Issue: #4. | I | Implemented |
| WB-ROLE-11 | The tester's and the fixer's prompts shall say that an existing test is never changed without a person's word. Issue: #4. | I | Implemented |
| WB-ROLE-12 | Every role prompt shall say that an agent never merges. Issue: #4. | I | Implemented |
| WB-ROLE-13 | Every role prompt shall end by naming the same fields for the role's last message every time. Issue: #4. | I | Implemented |
| WB-ROLE-14 | The last message of every role shall start with the fields `model` and `effort`: what the session states that it runs as, or `unknown`. Issue: #4. | T | Implemented |
| WB-ROLE-15 | Every role prompt shall say that the role opens no file outside those the prompt lists, and that where it needs another it stops and asks before it opens it. Issue: #4. | T | Implemented |
| WB-ROLE-16 | The builder's prompt shall say that the builder never changes a file the task does not name, and the reviewer's that a file changed outside those the task names is a finding. Issue: #4. | I | Implemented |
