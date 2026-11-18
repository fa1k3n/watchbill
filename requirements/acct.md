# The accounts of the roles

What `wb accounts check` reads at the tracker and says. What an account is: [docs/revision.md](../docs/revision.md), the point on accounts (#82).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-ACCT-1 | `wb accounts check` shall name, for each role in use, the account the organisation's file gives it, and say `ok` or the fault. Issue: #83. | T | Draft |
| WB-ACCT-2 | `wb accounts check` shall name a role that has no account in the organisation's file, and end with 1. Issue: #83. | T | Draft |
| WB-ACCT-3 | `wb accounts check` shall name an account that two roles share, and end with 1. Issue: #83. | T | Draft |
| WB-ACCT-4 | `wb accounts check` shall name an account that is not a bot account, and end with 1. Issue: #83. | T | Draft |
| WB-ACCT-5 | `wb accounts check` shall name an account that is not a member of this project, and end with 1. Issue: #83. | T | Draft |
| WB-ACCT-6 | `wb accounts check` shall name an account that is a member of another project the token can see, and end with 1. Issue: #83. | T | Draft |
| WB-ACCT-7 | `wb accounts check` shall name a review or officer account whose role at the project is Developer or higher, and end with 1. Issue: #83. | T | Draft |
| WB-ACCT-8 | `wb accounts check` shall name any other role's account whose role at the project is above Developer, and end with 1. Issue: #83. | T | Draft |
| WB-ACCT-9 | `wb accounts check` shall end with 2 when the organisation's file has no accounts, no git server host, or the person's token is missing. Issue: #83. | T | Draft |
| WB-ACCT-10 | `wb accounts check` shall never print a token. Issue: #83. | T | Draft |
| WB-ACCT-11 | `wb accounts check` shall write nothing. Issue: #83. | T | Draft |
| WB-ACCT-12 | The repository shall hold `docs/accounts.md`, a recipe by which a person makes the accounts, their roles and their tokens by hand. Issue: #83. | I | Draft |
