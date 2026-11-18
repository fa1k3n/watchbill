# The listener

What `wb listen` does with the tracker. The tracker is its only state.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-LISTEN-1 | `wb listen` shall ask the tracker at the interval the organisation's file gives, with each role's own token, for the issues assigned to that role's account. Issue: #85. | T | Draft |
| WB-LISTEN-2 | `wb listen` shall start a session only for an assignment note of the server, never for the words of a comment. Issue: #85. | T | Draft |
| WB-LISTEN-3 | `wb listen` shall start a session only when the author of the last assignment note is a person whose role at the project is at least the role the rule table names, or the officer's account. Issue: #85. | T | Draft |
| WB-LISTEN-4 | `wb listen` shall start a tester only when a comment from a person of enough role, whose first line starts with `[accepted]`, holds the hash of the issue's file as it is. Issue: #85. | T | Draft |
| WB-LISTEN-5 | `wb listen` shall start one session for an issue found on more than one poll. Issue: #85. | T | Draft |
| WB-LISTEN-6 | `wb listen` shall post a refusal of the starter as one comment from the starter's identity and assign the issue back to the person. Issue: #85. | T | Draft |
| WB-LISTEN-7 | `wb listen` shall give a session the token of its own role and no other. Issue: #85. | T | Draft |
| WB-LISTEN-8 | `wb listen` shall never print a token, nor write one in a comment. Issue: #85. | T | Draft |
| WB-LISTEN-9 | `wb listen` shall open no port. Issue: #85. | T | Draft |
