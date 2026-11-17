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
| WB-ROLE-17 | The list of placeholders shall hold `{checkout}`, `{branch}`, `{task_address}` and `{change_address}`: where the session works and where its task and its change are read. Issue: #43. | T | Implemented |
| WB-ROLE-18 | Every role prompt shall name the checkout and the branch it works in, and the address its task is read at. Issue: #43. | T | Implemented |
| WB-ROLE-19 | The reviewer's and the fixer's prompts shall name the address the change is read and posted at. Issue: #43. | T | Implemented |
| WB-ROLE-20 | Every role prompt shall say that how work is pushed and posted is taken from the project's own text. Issue: #43. | T | Implemented |
| WB-ROLE-21 | Every role prompt shall say that a search opens every file it reads, and that a search is kept to the files the role may read. Issue: #44. | T | Implemented |
| WB-ROLE-22 | Every role prompt shall say that a last message without its fields is refused, and that the session is asked for it again. Issue: #45. | T | Implemented |
| WB-ROLE-23 | The tester's prompt shall say that a test of a clause that holds already before the change is a guard, which is written, run and seen to pass. Issue: #46. | T | Implemented |
| WB-ROLE-24 | The tester's last message shall have the field `guards`, after `red`. Issue: #46. | T | Implemented |
| WB-ROLE-25 | The builder's prompt shall say that the guards pass before the change and after it. Issue: #46. | T | Implemented |
| WB-ROLE-26 | The tester's prompt shall say that the tester reads the interface the task names, and not the rest of a file that also holds the body. Issue: #47. | T | Implemented |
| WB-ROLE-27 | Every role prompt shall say that the role may read the helpers of the tests it may read. Issue: #47. | T | Implemented |
| WB-ROLE-28 | The prompts of the tester, the builder and the fixer shall say that a helper a test uses is part of that test. Issue: #47. | T | Implemented |
| WB-ROLE-29 | The reviewer's prompt shall say that the reviewer may read what the changed code calls, and still nothing of how the change was made. Issue: #47. | T | Implemented |
| WB-ROLE-30 | The tester's prompt shall say that where a requirement says "the same as" or "any" and does not name its cases, the tester asks which cases there are under `needs_person`, and does not guess. Issue: #48. | T | Implemented |
| WB-ROLE-31 | The tester's prompt shall say that where a requirement about a message does not give its words, and does not say that any words do, the tester holds the message to no words of its own and asks. Issue: #48. | T | Implemented |
| WB-ROLE-32 | The reviewer's prompt shall say that in round 2 a requirement added to the change since round 1 is reviewed as in round 1. Issue: #49. | T | Draft |
| WB-ROLE-33 | The fixer's prompt shall say that after the last round a fix is made only where a person has allowed it, that this person checks it with the test that shows it, and that it is not a round of review. Issue: #49. | T | Draft |
