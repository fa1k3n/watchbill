# The starter

What `wb build` checks before it starts a session. One path for a person and for the listener ([listen.md](listen.md)).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-BUILD-1 | `wb build --issue <n>` shall refuse an issue whose file is not accepted, name the fault, and end with 1. Issue: #84. | T | Draft |
| WB-BUILD-2 | `wb build` shall refuse when the rule table does not let the role start on this change now, name the rule, and end with 1. Issue: #84. | T | Draft |
| WB-BUILD-3 | `wb build` shall refuse a change whose paths hold classed code, name #65, and end with 1. Issue: #84. | T | Draft |
| WB-BUILD-4 | `wb build` shall make the checks in the order: the acceptance, the rule table, the class; the first fault ends it. Issue: #84. | T | Draft |
| WB-BUILD-5 | `wb build` shall hand the session its prompts by `wb prompt`, with a stamp that says an agent wrote the change. Issue: #84. | T | Draft |
| WB-BUILD-6 | `wb build` shall record the person's git account as the one that holds build on the change. Issue: #84. | T | Draft |
| WB-BUILD-7 | `wb build` shall refuse a second start for an issue that has a build running, and end with 1. Issue: #84. | T | Draft |
| WB-BUILD-8 | `wb build` shall ask nothing of the network. Issue: #84. | T | Draft |
| WB-BUILD-9 | `wb build` shall write nothing outside the working folder of the session and `.watchbill/`. Issue: #84. | T | Draft |
