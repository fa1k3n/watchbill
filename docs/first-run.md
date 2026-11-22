# The first run

On 2026-11-17 two real tasks went through the roles of the first revision, each session started by hand. This page says what was run, what each session did, what was counted, and what the prompts got wrong. Each fault that needs a change is an issue of its own (#29).

## What was run

- **The revision:** package 0.0.1, `share/watchbill/revision.sha256` at `444202f`. The SHA-256 of that file is `be95f6e6d527ef954eaa5a8fc64e00236322aee8e89558e025e72258e51d2fab`. The prompts were used as they stand, with the placeholders filled in.
- **The project:** an open-source project of the owner's own, written in TypeScript and tested with `node:test`. Nothing of a client.
- **Classed code** in that project, settled with the owner: two code classes of its own.
- **The plain task:** a tile on its dashboard, shown only in one case. Code of no class. Three requirements.
- **The classed task:** a change to how often a refused result is asked about. Code of one of the two classes. Three requirements, a fourth added during the run.
- **Who started the sessions:** not the captain by hand, as #29 proposed. An agent session (Claude Opus 5.5) started them on the owner's word, one fresh session per role, and brought every `needs_person` to the owner, who decided each. #29's Run line asks for model fable; the session ran as Opus 5.5.
- **What a session got:** the role's file, filled in, then a short note from the starting session marked as not part of the prompt: where to work, how to read the task, how to post. The prompts have no place for that (F-1).

The project had no requirement IDs. Before the first session, the starting session numbered each task's requirements (R1.1 to R1.3, R2.1 to R2.4) and wrote into each task what may be read beside its files.

## The sessions

Model and effort are as started, from the model table: `claude-sonnet-5-5` at medium for tester, build and fix, `claude-opus-5-5` at high for review. The starting tool takes "sonnet" or "opus", not an exact ID. Every session stated its model as the table's. Every session stated its effort as `unknown`.

### The plain task

| # | Role | Time | What it did |
|---|---|---|---|
| 1 | tester | 1 min | Three tests of the figure. Two red for the right reason; one passed already, as a guard. Left every clause about the page untested: no test renders the page, and the task named no seam. Reported that a search had shown it lines of files outside its list. |
| 2 | tester, again | 1 min | After the owner named a seam: three tests of the page through it, all red for the right reason. Its first hand-back was one line, without the fields; they came when asked again. |
| 3 | build | 2 + 5 min | Built it, and stopped: the change failed the type-check in existing tests. Changing an existing test goes to a person. The owner allowed seven lines. It changed exactly those, opened the change, checks green. |
| 4 | review, round 1 | 12 min | No `must`, six `note`. Checked each clause by breaking the code and seeing its test fail. Classed: no. |

The fix role had nothing to answer, so it did not run on this task, and there was no round 2.

### The classed task

| # | Role | Time | What it did |
|---|---|---|---|
| 5 | tester | 1 min | Four tests; two red, two guards of "as today". Its first command searched the code under test; the host's permissions refused it. |
| 6 | build | 15 min | Built it in the one named file; listed three behaviours no test covers in the change's description. |
| 7 | review, round 1 | 3 min | One `must`: after a refusal the gap doubled twice per ask, a regression the tests did not cover. Five `note`, one of them for a person. Classed: yes. |
| 8 | fix, round 1 | 6 min | Fixed the `must`, test first. Also changed a helper of the tester's tests. |
| 9 | tester | 1 min | Tests for R2.4, which the owner added from a note of round 1. Both red. |
| 10 | build | 5 min | Built R2.4, adding a field to a stored file that the task did not name; said so in the description. |
| 11 | review, round 2 | 3 min | Round 1's `must` fixed. A new `must` open. Classed: yes. |
| 12 | fix, after round 2 | 11 min | The owner chose to fix the open `must`. Fixed, test first. The starting session checked it: the test fails on the code before and passes after. |

Times are the sessions' own, from start to last message.

## The counts

| | Plain task | Classed task |
|---|---|---|
| Findings, `must` | 0 | 2 |
| Findings, `note` | 6 | 5 |
| `must` answered with a fix | – | 2 (one after round 2) |
| `must` answered with a reason | – | 0 |
| Findings shown wrong | 0 | 0 |
| Open after round 2 | – | 1, to the owner, who chose a fix |
| To a person | the page's seam; seven existing test lines; a test file's place | a note made a requirement; the open `must`; a stored field; verification of classed code |
| Tests written | 6 | 9 |

Both changes reached the owner: the plain one ready to merge, the classed one for the owner's verification, as the rule table says.

## What held

- **The limits that go to a person held.** The builder stopped at an existing test. The fixer did not change the tester's tests to make a finding go. The review said "classed: yes" first on the classed change and did not stand in for the person.
- **Tests from the requirement, before the code.** Every test that could be red was red for the right reason before the code, and named its requirement.
- **Fresh sessions found what the last one missed.** Round 1 found a regression on the classed task that the tests and the build both missed; round 2 found another. Neither was a wrong finding.
- **The review showed its claims.** It broke the code to show that a clause's test fails without it, and reproduced each `must` before posting it.

## What to change

Each line names the issue that carries it, or says why it is left.

- **F-1. A prompt has no place for where to work.** The checkout, the branch, how to read the task, whether to push, and the project's own rules for posting (here: a review comment's first line, a draft change). Every session needed a note beside its prompt. Issue #43.
- **F-2. "Never open a file outside the list" does not name search.** Two testers searched the code under test before anything else; a search shows lines of every file it reads. Only the host's permissions stopped one. After a note that a search counts as opening every file it searches, it did not happen again. Issue #44.
- **F-3. The fixed last message is not checked.** One session ended with one line and no fields. The prompt says "every time"; nothing refuses an end without them. Issue #45.
- **F-4. Effort cannot be read back, and the exact model cannot be pinned by every starter.** Every session said effort `unknown`. Added to #31, which already proposes that the starter stamps them.
- **F-5. A clause that already holds cannot be red.** "Every new test fails" met three guards ("as today", and a count the code already gave). The tester wrote them and said so; the prompt has no word for them. Issue #46.
- **F-6. What may be read is not cut where the work needs it.** A task's file held both the interface and the body of the unit under test, so a tester opened the body to read the interface. Test helpers sit in files outside the list. A review may not read a function the change calls, and judged two "from how they are used". Issue #47.
- **F-7. Tests from the requirement miss the cases the requirement does not name.** A requirement said a rule held for every kind of a thing; there were three kinds, and the tests covered one. Both `must` findings sat in the cases not named. Issue #48.
- **F-8. No step for a requirement added during a change, or for a fix after the last round.** A note became a requirement; the fix role answers `must` findings only, so a tester and a builder were started again on the same change. A `must` open after round 2 was fixed on the owner's word, and nothing says who checks that fix. Issue #49.
- **F-9. The tester runs single test files, not the project's checks.** A task's "existing tests this changes: none" was wrong; only the type-check showed it, at the build. Issue #50.
- **F-10. A helper of a test is not named as part of it.** The fixer gave the tester's helper a new argument. The change was harmless, but the prompt does not say whether a helper is part of the test it may not change. Part of #47.
- **F-11. The project's own model and effort can differ from the table's.** The plain task named a lower effort than the table. The table was used; nothing says which wins. Issue #51.
- **F-12. A project with no requirement IDs.** The prompts assume a row and an ID per requirement. This project had none; the starting session made them. Left as it is: `wb init` already asks where the requirements are, and a project with none is to be refused or helped by a later piece. Said here so that it is not forgotten.
- **F-13. A build widened a stored format on its own.** It added a field to a file the program keeps, not named by the task, and said so in the change's description. The prompt asks for anything a reviewer would not expect to be said, and it was. Left as it is.
- **F-14. A test fixed words the requirement does not give.** A tester held a warning's text to a pattern of its own. Left as a line in #48: a requirement about a message should give its words.

## The model table

The figures are two tasks. They correct nothing yet. They show that the reviews at `claude-opus-5-5`, high, found every fault that was found, and that no finding was wrong.
