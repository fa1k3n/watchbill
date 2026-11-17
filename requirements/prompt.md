# The prompt of an assignment

How a prompt is put together from the revision and the project's own text, and the command that prints it. What a revision is: [docs/revision.md](../docs/revision.md).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-PROMPT-1 | The prompt of an assignment shall be the role's text of the pinned revision, unchanged, then one fixed line, then `.watchbill/project/all.md`, then `.watchbill/project/<role>.md`, each where it exists. Issue: #9. | T | Implemented |
| WB-PROMPT-2 | With no project text, the prompt shall be the role's text of the revision with its placeholders filled, and nothing more. Issue: #9. | T | Implemented |
| WB-PROMPT-3 | A placeholder that is not on the list shall be refused in any part, and the file shall be named. Issue: #9. | T | Implemented |
| WB-PROMPT-4 | A value that is filled in shall not itself be read for placeholders. Issue: #9. | T | Implemented |
| WB-PROMPT-5 | A project file that is not UTF-8 shall be refused by name. Issue: #9. | T | Implemented |
| WB-PROMPT-6 | A role the revision does not have shall be refused, with the names of the roles it has. Issue: #9. | T | Implemented |
| WB-PROMPT-7 | For each part of a prompt its path and SHA-256 shall be given, in order, and the hash of the role's part shall be the one on its line of the manifest. Issue: #9. | T | Implemented |
| WB-PROMPT-8 | `wb check` shall not fail on a file under `.watchbill/project/`, changed or not. Issue: #9. | T | Implemented |
| WB-PROMPT-9 | `wb init` shall leave `.watchbill/project/README.md`, saying what the project's files are for and that they never lift a limit of a role. Issue: #9. | T | Implemented |
| WB-PROMPT-10 | `wb prompt <role>` shall print the prompt on standard output and nothing else, and end with 0. Issue: #10. | T | Implemented |
| WB-PROMPT-11 | `wb prompt <role> --parts` shall print, in place of the text, one line per part in the form `sha256sum` prints. Issue: #10. | T | Implemented |
| WB-PROMPT-12 | `wb prompt` shall print nothing on standard output, say why on standard error and end with 1 when the check of `wb check` fails. Issue: #10. | T | Implemented |
| WB-PROMPT-13 | `wb prompt` shall end with 2 in a repository that is not set up. Issue: #10. | T | Implemented |
| WB-PROMPT-14 | A placeholder that the prompt uses and that got no value shall be refused by name, with nothing on standard output. Issue: #10. | T | Implemented |
| WB-PROMPT-15 | An option of `wb prompt` that is no placeholder shall end with 2 and the usage. Issue: #10. | T | Implemented |
| WB-PROMPT-16 | A value that starts with `-` or holds a line break shall arrive in the prompt as it was given. Issue: #10. | T | Implemented |
| WB-PROMPT-17 | `wb prompt` shall write nothing and ask nothing of the network. Issue: #10. | T | Implemented |
