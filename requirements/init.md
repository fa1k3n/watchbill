# `wb init`

What `wb init` does today: it sets Watchbill up in a git repository. What it writes of the revision later: [pin.md](pin.md).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-INIT-1 | `wb init` shall propose the name of the folder of the repository, made safe as a name, as the project. Issue: #2. | T | Implemented |
| WB-INIT-2 | `wb init` shall propose the kind of git server that the host and port of `origin` show as the git server. Issue: #2. | T | Implemented |
| WB-INIT-3 | `wb init` shall propose the path of the address of `origin`, in each of the forms git takes, as the repository. Issue: #2. | T | Implemented |
| WB-INIT-4 | `wb init` shall propose the git server as the tracker when it is github or gitlab, and none otherwise. Issue: #2. | T | Implemented |
| WB-INIT-5 | `wb init` shall propose the first folder of requirements that holds a file, with the file ending most of its files have, as the requirements. Issue: #2. | T | Implemented |
| WB-INIT-6 | `wb init` shall propose the CI driver that the file or folder in the repository shows as the CI driver. Issue: #2. | T | Implemented |
| WB-INIT-7 | `wb init` shall propose none for each answer that the repository does not show. Issue: #2. | T | Implemented |
| WB-INIT-8 | `wb init` shall ask nothing of the network to find what it proposes. Issue: #2. | I | Implemented |
| WB-INIT-9 | `wb init` shall ask one line per answer and keep the proposal when the line is empty. Issue: #2. | T | Implemented |
| WB-INIT-10 | `wb init` shall not ask for the repository when the git server is none, and shall write none for it. Issue: #2. | T | Implemented |
| WB-INIT-11 | `wb init` shall keep the proposal for each answer not yet given when its input ends. Issue: #2. | T | Implemented |
| WB-INIT-12 | `wb init` shall say that an answer is not taken and ask again when it is not a usable project name. Issue: #2. | T | Implemented |
| WB-INIT-13 | `wb init` shall say that an answer is not taken and ask again when it is over 200 characters or holds a control character. Issue: #2. | T | Implemented |
| WB-INIT-14 | `wb init --yes`, also written `-y`, shall ask nothing and write what it proposes. Issue: #2. | T | Implemented |
| WB-INIT-15 | `wb init` shall show the file it is about to write before it asks whether that is OK. Issue: #2. | T | Implemented |
| WB-INIT-16 | `wb init` shall write nothing and end with 1 when the answer to whether it is OK is not an empty line, `y` or `yes`. Issue: #2. | T | Implemented |
| WB-INIT-17 | `wb init` shall write `.watchbill/watchbill.yml` at the root of the repository, also when it is run from a folder inside it. Issue: #2. | T | Implemented |
| WB-INIT-18 | `wb init` shall write the version of `wb` as the line `watchbill:` of the file. Issue: #2. | T | Implemented |
| WB-INIT-19 | `wb init` shall leave a file that is already there as it is, say so and end with 1. Issue: #2. | T | Implemented |
| WB-INIT-20 | `wb init` shall write nothing, say so on standard error and end with 1 when it is run outside a git repository. Issue: #2. | T | Implemented |
| WB-INIT-21 | `wb init` shall keep no user name, password or token of the address of `origin`, in the file or on the screen. Issue: #2. | T | Implemented |
| WB-INIT-22 | `wb init` shall write in single quotes a value that YAML would read as something other than the text. Issue: #2. | T | Implemented |
| WB-INIT-23 | `wb init` shall say which file it wrote. Issue: #2. | T | Implemented |
| WB-INIT-24 | `wb init` shall end with 0 when it has written the file. Issue: #2. | T | Implemented |
| WB-INIT-25 | `wb` shall print the usage and end with 2 when it is given no command, a command it does not know, or an option `wb init` does not know. Issue: #2. | T | Implemented |
| WB-INIT-26 | `wb --version` shall print the revision of `wb` and end with 0. Issue: #2. | T | Implemented |
| WB-INIT-27 | `wb --help` shall print the usage and end with 0. Issue: #2. | T | Implemented |
