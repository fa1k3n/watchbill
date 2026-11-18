# Confidentiality

Where an assignment may run. The levels and the runners are the organisation's; a project states its own level. What was decided: [docs/revision.md](../docs/revision.md), point 11.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-CONF-1 | The levels of confidentiality shall be read, as an ordered list of names, from the organisation's file: `/etc/watchbill/organisation.yml`, or the path in `WATCHBILL_ORGANISATION`. Issue: #11. | T | Draft |
| WB-CONF-2 | The organisation's file shall name each runner with where its tools run, which model answers, under which agreement, and the highest level it may be given. Issue: #11. | T | Draft |
| WB-CONF-3 | With no organisation's file, `wb prompt` shall hand out no prompt, name the file it looked for and end with 1. Issue: #11. | T | Draft |
| WB-CONF-4 | The level of a repository shall be the line `confidentiality:` of `.watchbill/watchbill.yml`, and the most restrictive level where the line is missing. Issue: #11. | T | Draft |
| WB-CONF-5 | A repository whose level is not on the organisation's list shall be refused by name. Issue: #11. | T | Draft |
| WB-CONF-6 | Levels or runners written in a repository's settings shall be refused. Issue: #11. | T | Draft |
| WB-CONF-7 | The level of an assignment shall be the higher of the level of its issue and the level of the repository. Issue: #11. | T | Draft |
| WB-CONF-8 | `wb prompt` shall take the runner as `--runner <name>` and refuse when none is named. Issue: #11. | T | Draft |
| WB-CONF-9 | `wb prompt` shall print nothing on standard output, name the level and the runner on standard error and end with 1 when the runner is not cleared for the level of the assignment. Issue: #11. | T | Draft |
| WB-CONF-10 | A runner or a level that the organisation's file does not name shall be refused, with the names it has. Issue: #11. | T | Draft |
| WB-CONF-11 | An organisation's file with a runner that names no highest level, a level written twice or a key that is not known shall be refused, and the place shall be named. Issue: #11. | T | Draft |
| WB-CONF-12 | `wb init` shall ask for the level of the repository and propose the most restrictive of the organisation's list, and with no organisation's file it shall write no level and say that no prompt will be handed out. Issue: #11. | T | Draft |
| WB-CONF-13 | `wb prompt --parts` shall add the level in force and the runner to what it prints. Issue: #11. | T | Draft |
