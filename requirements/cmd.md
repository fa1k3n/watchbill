# The command layout

Which commands `wb` has, how they are grouped, and what a planned one says.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-CMD-1 | `wb` shall have the commands `init`, `check`, `doctor`, `upgrade`, `accept`, `start`, `run`, `status`, `view`, `log`, `answer`, `revert`, `clean`, `prompt`, `revision` and `compare`. Issue: #171. | T | Implemented |
| WB-CMD-2 | A command that is not built yet shall end with 2, write nothing, and ask nothing of the network. Issue: #171. | T | Implemented |
| WB-CMD-3 | A command that is not built yet shall say on standard error the release it is planned for and the issue that builds it. Issue: #171. | T | Implemented |
| WB-CMD-4 | `wb help` shall print the groups set up, work, see, take back and plumbing, in that order, each with its commands. Issue: #171. | T | Implemented |
| WB-CMD-5 | `wb help` shall name the release of each command that is not built yet. Issue: #171. | T | Implemented |
| WB-CMD-6 | `wb help <command>` shall print the usage and the help of that command. Issue: #171. | T | Implemented |
| WB-CMD-7 | `wb help` with a command that is not known shall end with 2. Issue: #171. | T | Implemented |
| WB-CMD-8 | `wb --help`, and `wb` with no command, shall print what `wb help` prints. Issue: #171. | T | Implemented |
| WB-CMD-9 | The usage, `wb help` and the commands `wb` takes shall be made from one list. Issue: #171. | T | Implemented |
