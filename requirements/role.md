# The role prompts

The role prompts: breakdown, tester, build, review and fix. The first revision had the last four. What a revision is: [docs/revision.md](../docs/revision.md).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-ROLE-1 | The revision shall hold one prompt file, `roles/<role>.md`, for each of the roles tester, build, review and fix, and for no other role. Replaced by WB-ROLE-47. Issue: #4. | T | Deprecated |
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
| WB-ROLE-16 | The builder's prompt shall say that the builder never changes a file the issue does not name, and the reviewer's that a file changed outside those the issue names is a finding. Issue: #4. | I | Implemented |
| WB-ROLE-17 | The list of placeholders shall hold `{checkout}`, `{branch}`, `{issue_address}` and `{change_address}`: where the session works and where its issue and its change are read. Issue: #43. | T | Implemented |
| WB-ROLE-18 | Every role prompt shall name the checkout and the branch it works in, and the address its issue is read at. Issue: #43. | T | Implemented |
| WB-ROLE-19 | The reviewer's and the fixer's prompts shall name the address the change is read and posted at. Issue: #43. | T | Implemented |
| WB-ROLE-20 | Every role prompt shall say that how work is pushed and posted is taken from the project's own text. Issue: #43. | T | Implemented |
| WB-ROLE-21 | Every role prompt shall say that a search opens every file it reads, and that a search is kept to the files the role may read. Issue: #44. | T | Implemented |
| WB-ROLE-22 | Every role prompt shall say that a last message without its fields is refused, and that the session is asked for it again. Issue: #45. | T | Implemented |
| WB-ROLE-23 | The tester's prompt shall say that a test of a clause that holds already before the change is a guard, which is written, run and seen to pass. Issue: #46. | T | Implemented |
| WB-ROLE-24 | The tester's last message shall have the field `guards`, after `red`. Issue: #46. | T | Implemented |
| WB-ROLE-25 | The builder's prompt shall say that the guards pass before the change and after it. Issue: #46. | T | Implemented |
| WB-ROLE-26 | The tester's prompt shall say that the tester reads the interface the issue names, and not the rest of a file that also holds the body. Issue: #47. | T | Implemented |
| WB-ROLE-27 | Every role prompt shall say that the role may read the helpers of the tests it may read. Issue: #47. | T | Implemented |
| WB-ROLE-28 | The prompts of the tester, the builder and the fixer shall say that a helper a test uses is part of that test. Issue: #47. | T | Implemented |
| WB-ROLE-29 | The reviewer's prompt shall say that the reviewer may read what the changed code calls, and still nothing of how the change was made. Issue: #47. | T | Implemented |
| WB-ROLE-30 | The tester's prompt shall say that where a requirement says "the same as" or "any" and does not name its cases, the tester asks which cases there are under `needs_person`, and does not guess. Issue: #48. | T | Implemented |
| WB-ROLE-31 | The tester's prompt shall say that where a requirement about a message does not give its words, and does not say that any words do, the tester holds the message to no words of its own and asks. Issue: #48. | T | Implemented |
| WB-ROLE-32 | The reviewer's prompt shall say that in round 2 a requirement added to the change since round 1 is reviewed as in round 1. Issue: #49. | T | Implemented |
| WB-ROLE-33 | The fixer's prompt shall say that after the last round a fix is made only where a person has allowed it, that this person checks it with the test that shows it, and that it is not a round of review. Issue: #49. | T | Implemented |
| WB-ROLE-34 | The tester's prompt shall say that after its commit the tester runs what the project runs before a change is handed over, and reports what fails for a reason other than the missing code. Issue: #50. | T | Implemented |
| WB-ROLE-35 | The tester's last message shall have the field `checks`, after `guards`. Issue: #50. | T | Implemented |
| WB-ROLE-36 | Each role's prompt shall name `model` and `effort` of the last message as what the session says of itself. Issue: #69. | T | Implemented |
| WB-ROLE-37 | Each role's prompt shall say that the stamp on the issue is what the session was handed out as. Issue: #69. | T | Implemented |
| WB-ROLE-38 | Each role's prompt shall ask the session to say under `model` and `effort` when its own word differs from the stamp. Issue: #69. | T | Implemented |
| WB-ROLE-39 | The tester's prompt shall say that a file of tests that does not load, because the one name it misses is the one the issue asks for, is red for every test in it. Issue: #60. | T | Implemented |
| WB-ROLE-40 | The tester's prompt shall say that the tester names the missing names under `red`. Issue: #60. | T | Implemented |
| WB-ROLE-41 | The tester's prompt shall say that, where the language allows it and the project's own text does not say otherwise, the tester also runs each test against a stand-in that gives a wrong answer, in a scratch copy it never commits, and says so. Issue: #60. | T | Implemented |
| WB-ROLE-42 | The reviewer's prompt shall say that where the red of the new tests was for a whole file, the reviewer sees each of them fail on its own, by breaking the code. Issue: #60. | T | Implemented |
| WB-ROLE-43 | The tester's prompt shall say that an existing test the issue names as the guard of a clause is run as it is and listed under `guards` by its name. Issue: #62. | T | Implemented |
| WB-ROLE-44 | The tester's prompt shall say that the tester writes no copy of an existing test the issue names as a guard. Issue: #62. | T | Implemented |
| WB-ROLE-45 | The tester's prompt shall say that where the tester thinks an existing test the issue names as a guard does not cover its clause, it says so under `untested`. Issue: #62. | T | Implemented |
| WB-ROLE-46 | The reviewer's prompt shall say that a new test that repeats an existing one is a `note`. Issue: #62. | T | Implemented |
| WB-ROLE-47 | The revision shall hold one prompt file, `roles/<role>.md`, for each of the roles breakdown, tester, build, review and fix, and for no other role. Issue: #75. | T | Implemented |
| WB-ROLE-48 | The breakdown's prompt shall say that the breakdown reads the issue at `{issue_address}` and the code in `{checkout}`. Issue: #75. | T | Implemented |
| WB-ROLE-49 | The breakdown's prompt shall say that the breakdown writes `.watchbill/issues/{issue}.yml` in the form of the issue's file, with every key in its order. Issue: #75. | T | Implemented |
| WB-ROLE-50 | The breakdown's prompt shall say that the breakdown commits the issue's file alone on `{branch}`. Issue: #75. | T | Implemented |
| WB-ROLE-51 | The breakdown's prompt shall say that the breakdown never changes the issue, the code or a test. Issue: #75. | T | Implemented |
| WB-ROLE-52 | The breakdown's prompt shall say that the breakdown puts what the issue leaves open under `questions` and does not answer it. Issue: #75. | T | Implemented |
| WB-ROLE-53 | The breakdown's prompt shall say that the breakdown finds the guards among the existing tests. Issue: #75. | T | Implemented |
| WB-ROLE-54 | The breakdown's prompt shall say that the breakdown lists for each role what it may read, as narrow as the work allows. Issue: #75. | T | Implemented |
| WB-ROLE-55 | The breakdown's prompt shall say that the breakdown reads the settings of the project's checks, so that what a check needs, such as a built page or a generated file, is in the issue. Issue: #75. | T | Implemented |
| WB-ROLE-56 | The first lines of the breakdown's prompt shall say that it runs on a local model unless a person has cleared another runner for the breakdown. Issue: #75. | T | Implemented |
| WB-ROLE-57 | The breakdown's last message shall have the fields `model`, `effort`, `file`, `requirements` (how many), `questions` and `needs_person`, in this order. Issue: #75. | T | Implemented |
| WB-ROLE-58 | The list of placeholders shall hold `{issue}` and `{issue_address}`, and not `{task}` or `{task_address}`. Issue: #78. | T | Implemented |
| WB-ROLE-59 | No file of the roles shall hold the placeholder `{task}` or `{task_address}`. Issue: #78. | T | Implemented |
| WB-ROLE-60 | The breakdown's prompt shall say that each question is also posted as its own comment on the issue, whose first line is `[question] {issue}.q<n>`, and that a question is never posted twice. Issue: #95. | T | Draft |
| WB-ROLE-61 | The breakdown's prompt shall say that a person answers in a comment whose first line is `[answer] {issue}.q<n>`, or in the question's thread. Issue: #95. | T | Draft |
| WB-ROLE-62 | The breakdown's prompt shall say that a second, fresh session folds the answers of a person into the issue's file, removes the questions answered, keeps those with no answer, and commits the file alone. Issue: #95. | T | Draft |
| WB-ROLE-63 | The breakdown's last message shall have the field `asked`, naming the comments it posted. Issue: #95. | T | Draft |
| WB-ROLE-64 | The prompts of the breakdown, the tester, the build and the fix shall say to commit and then push to `{branch}`, and their last messages shall name the commit that was pushed. Issue: #98. | T | Draft |
| WB-ROLE-65 | The breakdown's prompt shall say that a requirement is what the change must do and a test or a reading of the code can show, and that how an existing test changes goes under `existing_tests_changed`, never as a requirement. Issue: #98. | T | Draft |
| WB-ROLE-66 | The line "Draft: this prompt has not yet run on a project." shall stand in `fix.md` and in no other role file. Issue: #98. | T | Draft |
| WB-ROLE-67 | The review's prompt shall say that where its address shows no description of the change, the review says so as a finding. Issue: #98. | T | Draft |
| WB-ROLE-68 | The review's prompt shall say that each finding starts with its number and its level, `<n>. must` or `<n>. note`, where it is posted and in the last message, and the fix's prompt shall read a finding in that form. Issue: #82. | T | Implemented |
