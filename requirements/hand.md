# The handover

What `wb handover` does when a session ends. The rule table decides, never the session.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-HAND-1 | `wb handover` shall hand on nothing for a session that has no `[stamp]` comment of the starter's identity on the issue, and end with 1. Issue: #88. | T | Draft |
| WB-HAND-2 | `wb handover` shall ask again for a last message that lacks a field its role's prompt names, up to the number of asks of the rule table. Issue: #88. | T | Draft |
| WB-HAND-3 | `wb handover` shall give the issue to a person after the last ask when the last message still lacks a field. Issue: #88. | T | Draft |
| WB-HAND-4 | `wb handover` shall assign the issue to build when the tester ended, and to review in round 1 when build ended. Issue: #88. | T | Draft |
| WB-HAND-5 | `wb handover` shall assign the issue to fix when a review ended with a `must` finding and a round is left. Issue: #88. | T | Draft |
| WB-HAND-6 | `wb handover` shall assign the issue to review in the next round when fix ended. Issue: #88. | T | Draft |
| WB-HAND-7 | `wb handover` shall give the issue to a person as ready for review when a review ended with no `must` finding and `needs_person: none`. Issue: #88. | T | Draft |
| WB-HAND-8 | `wb handover` shall give the issue to a person when a review ended with a `must` finding in the last round. Issue: #88. | T | Draft |
| WB-HAND-9 | `wb handover` shall give the issue to a person when `needs_person` is not `none`. Issue: #88. | T | Draft |
| WB-HAND-10 | `wb handover` shall give the issue to a person for each case of `always_to_a_person` of the rule table, whatever role the table of steps names. Issue: #88. | T | Draft |
| WB-HAND-11 | `wb handover` shall give the issue to a person when the next role's account may not hold the role on this change, and say why. Issue: #88. | T | Draft |
| WB-HAND-12 | `wb handover` shall give the issue to a person when it cannot read the level of the findings of a review. Issue: #88. | T | Draft |
| WB-HAND-13 | `wb handover` shall post one comment from the starter's identity whose first line is `[handover] <from> -> <to> round <n>`. Issue: #88. | T | Draft |
| WB-HAND-14 | `wb handover` shall never print a token or write one in a comment. Issue: #88. | T | Draft |
| WB-HAND-15 | `wb handover` shall never change the text of the issue. Issue: #88. | T | Draft |
| WB-HAND-16 | `wb handover` shall do nothing for a session it has handed on already, and end with 1. Issue: #88. | T | Draft |
| WB-HAND-17 | `wb handover` shall end with 0 when the issue was handed to a role, with 1 when it was given to a person or refused, and with 2 when it was used wrongly or the repository is not set up. Issue: #88. | T | Draft |
