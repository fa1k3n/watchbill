# Confidentiality

Where an assignment may run. The levels and the runners are the organisation's; a project states its own level. What was decided: [docs/revision.md](../docs/revision.md), point 11.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-CONF-1 | The levels of confidentiality shall be read, as an ordered list of names, from the organisation's file, `/etc/watchbill/organisation.yml`, a path fixed in `wb`: no option and no environment variable changes it. Issue: #11, #93. | T | Implemented |
| WB-CONF-2 | The organisation's file shall name each runner with where its tools run, which model answers, under which agreement, and the highest level it may be given. Issue: #11. | T | Implemented |
| WB-CONF-3 | With no organisation's file, `wb prompt` shall hand out no prompt, name the file it looked for and end with 1. Issue: #11. | T | Draft |
| WB-CONF-4 | The level of a repository shall be the line `confidentiality:` of `.watchbill/watchbill.yml`, and the most restrictive level where the line is missing. Issue: #11. | T | Implemented |
| WB-CONF-5 | A repository whose level is not on the organisation's list shall be refused by name. Issue: #11. | T | Implemented |
| WB-CONF-6 | Levels or runners written in a repository's settings shall be refused. Issue: #11. | T | Implemented |
| WB-CONF-7 | The level of an assignment shall be the higher of the level of its issue and the level of the repository. Issue: #11. | T | Draft |
| WB-CONF-8 | `wb prompt` shall take the runner as `--runner <name>` and refuse when none is named. Issue: #11. | T | Draft |
| WB-CONF-9 | `wb prompt` shall print nothing on standard output, name the level and the runner on standard error and end with 1 when the runner is not cleared for the level of the assignment. Issue: #11. | T | Draft |
| WB-CONF-10 | A runner or a level that the organisation's file does not name shall be refused, with the names it has. Issue: #11. | T | Implemented |
| WB-CONF-11 | An organisation's file with a runner that names no highest level, a level written twice or a key that is not known shall be refused, and the place shall be named. Issue: #11. | T | Implemented |
| WB-CONF-12 | `wb init` shall ask for the level of the repository and propose the most restrictive of the organisation's list, and with no organisation's file it shall write no level and say that no prompt will be handed out. Issue: #11. | T | Draft |
| WB-CONF-13 | `wb prompt --parts` shall add the level in force and the runner to what it prints. Issue: #11. | T | Draft |
| WB-CONF-14 | The levels of the organisation's file shall be listed from the least to the most restrictive, each name once. Issue: #93. | T | Implemented |
| WB-CONF-15 | A runner of the organisation's file may name `planner` with `up_to`, `model` and `effort`, and a `planner.up_to` above the runner's own `up_to` shall be refused, with the place named. Issue: #93. | T | Implemented |
| WB-CONF-16 | A model of a runner's `planner` that is not exact, or an effort that is not known, shall be refused, with the place named. Issue: #93. | T | Implemented |
| WB-CONF-17 | `wb prompt planner` shall refuse, with 2 and the usage, when no `--runner` is given. Issue: #93. | T | Implemented |
| WB-CONF-18 | `wb prompt planner --runner <name>` shall hand out the prompt only when the organisation's file names the runner, the runner has `planner`, and the repository's level is no higher than the planner's `up_to`. Issue: #93. | T | Implemented |
| WB-CONF-19 | When `wb prompt planner` refuses a runner, it shall print nothing on standard output, name on standard error the file, the runner, or the level and the runner's `up_to`, and end with 1. Issue: #93. | T | Implemented |
| WB-CONF-20 | The stamp of the planner shall give `runner` as the runner named, and `model` and `effort` from the runner's `planner`. Issue: #93. | T | Implemented |
| WB-CONF-21 | For the tester, the build, the review and the fix, `--runner` shall be taken without a check, and the stamp shall name it. Issue: #93. | T | Implemented |
| WB-CONF-22 | `wb prompt --parts` for the planner shall add the level in force and the runner. Issue: #93. | T | Implemented |
| WB-CONF-23 | `wb prompt` shall refuse with 2 and the usage a `--runner` that is empty, given twice, or not one plain line: a control character, a line or paragraph separator, or white space at its start or end. Issue: #93. | T | Implemented |
| WB-CONF-24 | A runner of the organisation's file may have `start`, a list of plain words whose only placeholders are `{model}`, `{effort}` and `{prompt}`; the file may name a `default` runner that is one of its runners; anything else there is refused with the file and the place named. Issue: #84. | T | Implemented |
| WB-CONF-25 | A runner of the organisation's file may have `binds`, a list of paths the runner needs for itself, each read-only unless it is marked writable; anything else there is refused with the file and the place named. Issue: #126. | T | Implemented |
