# The lines a start leaves

What `wb start <role>` leaves on the machine where a session is started or refused. They are diagnostics, not a record. Only `wb start` and `wb status` read them back, to name a view kept with work not taken back and a session cut off (#222).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-MON-1 | `wb start <role>` shall write, for each start, a new folder `<state>/watchbill/sessions/<time>-<role>-<issue>/`, with `<state>` `$XDG_STATE_HOME` or else `~/.local/state`, and never write over one that is there. Issue: #124. | T | Implemented |
| WB-MON-2 | `wb start <role>` shall say on standard error the folder it will write, before it writes it. Issue: #124. | T | Implemented |
| WB-MON-3 | The folder shall hold `stamp`, byte for byte the stamp posted, or that would have been posted. Issue: #124. | T | Implemented |
| WB-MON-4 | The folder shall hold `outcome`: `refused` with the check that refused (`prompt`, `rule`, `class`, `start`, `terminal` or `ci`), the status and the first line of the message; `not posted`; or `ended` with the session's exit status and the seconds it ran. Issue: #124. | T | Implemented |
| WB-MON-5 | A folder that cannot be written shall be said on standard error, and shall change neither the status nor whether the session starts. Issue: #124. | T | Implemented |
| WB-MON-6 | `wb prompt`, `wb check` and `wb start <role> --prompt` shall write no folder. Issue: #124. | T | Implemented |
| WB-MON-7 | The folder of a session started in a view shall hold `view`, the list of its paths, and `back`, what was taken back and what was refused and why. Issue: #126. | T | Implemented |
| WB-MON-8 | The folder of a review that reaches the check of the CI (one not refused at `prompt`, `rule`, `class`, `start` or `terminal`) shall hold `ci`: `ci: <word>` with the word `change_checks` printed, `ci: not read` where there is none, and `ci: unread` where it printed another word or failed. Issue: #127. | T | Implemented |
| WB-MON-9 | Where the session ended by SIGHUP or SIGTERM, `outcome` shall hold the line `signal: SIGHUP` or `signal: SIGTERM`. Issue: #221. | T | Implemented |
| WB-MON-10 | `wb start <role>` shall write `outcome` first as `outcome: running`, with the lines `pid: <process ID of wb>` and `started: <its start time in clock ticks>`, before the session starts, so that a `wb` ended before its session leaves it. Issue: #222. | T | Implemented |
| WB-MON-11 | `wb start <role>` shall replace `outcome` when the start ends with the lines of WB-MON-4, with no `pid` line, so that no start that ended leaves `running`. Issue: #222. | T | Implemented |
| WB-MON-12 | The folder of a session started in a view shall hold `folder`, written before the session starts, with the lines `folder: <the view's folder>`, `base: <its first commit>`, `head: <the commit of the checkout it was made from>`, `branch: <refs/heads/name>` and `issue: <issue>`. Issue: #222. | T | Implemented |
| WB-MON-13 | `wb start` and `wb status` shall not read, and shall not wait for, a file of the lines (`outcome`, `stamp`, `back`, `folder`) that is not a plain file, or is over 1 MiB. Issue: #222. | T | Implemented |
| WB-MON-14 | A take-back shall append to `back` of the session's folder a line `take-back` and the lines of WB-MON-7 for what it did, the last line `view: kept` or `view: removed` being the new one; what `wb status` names as not taken shall be read from the lines after the last `take-back`. Issue: #223. | T | Draft |
