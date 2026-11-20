# The lines a start leaves

What `wb <role>` leaves on the machine where a session is started or refused. They are diagnostics, not a record, and `wb` never reads them back.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-MON-1 | `wb <role>` shall write, for each start, a new folder `<state>/watchbill/sessions/<time>-<role>-<issue>/`, with `<state>` `$XDG_STATE_HOME` or else `~/.local/state`, and never write over one that is there. Issue: #124. | T | Implemented |
| WB-MON-2 | `wb <role>` shall say on standard error the folder it will write, before it writes it. Issue: #124. | T | Implemented |
| WB-MON-3 | The folder shall hold `stamp`, byte for byte the stamp posted, or that would have been posted. Issue: #124. | T | Implemented |
| WB-MON-4 | The folder shall hold `outcome`: `refused` with the check that refused (`prompt`, `rule`, `class`, `start`, `terminal` or `ci`), the status and the first line of the message; `not posted`; or `ended` with the session's exit status and the seconds it ran. Issue: #124. | T | Implemented |
| WB-MON-5 | A folder that cannot be written shall be said on standard error, and shall change neither the status nor whether the session starts. Issue: #124. | T | Implemented |
| WB-MON-6 | `wb prompt`, `wb check` and `wb <role> --prompt` shall write no folder. Issue: #124. | T | Implemented |
| WB-MON-7 | The folder of a session started in a view shall hold `view`, the list of its paths, and `back`, what was taken back and what was refused and why. Issue: #126. | T | Implemented |
| WB-MON-8 | The folder of a review shall hold `ci`: `ci: <word>` with the word `change_checks` printed, `ci: not read` where there is none, and `ci: unread` where it printed another word or failed. Issue: #127. | T | Implemented |
