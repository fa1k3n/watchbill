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
| WB-PROMPT-18 | `wb prompt` shall take one option for each placeholder on the list of the revision, named as the placeholder, and no other placeholder option. Issue: #43. | T | Implemented |
| WB-PROMPT-19 | `wb prompt <role> --stamp` shall print, in place of the text, the stamp of the assignment as lines of `name: value` in the order of `docs/revision.md` point 12. Issue: #68. | T | Implemented |
| WB-PROMPT-20 | The `model` and `effort` of a stamp shall be those of the model table for the role. Issue: #68. | T | Implemented |
| WB-PROMPT-21 | A stamp shall have one `part:` line for each part of the prompt, as `--parts` gives it, in order. Issue: #68. | T | Implemented |
| WB-PROMPT-22 | A value with a line break shall be refused by name in a stamp, with status 2. Issue: #68. | T | Implemented |
| WB-PROMPT-23 | `wb prompt` given both `--stamp` and `--parts` shall end with 2. Issue: #68. | T | Implemented |
| WB-PROMPT-24 | `wb prompt --stamp` shall print nothing on standard output and end with 1 when the check of `wb check` does not hold. Issue: #68. | T | Implemented |
| WB-PROMPT-25 | When an issue's file is accepted, `wb prompt` shall fill `{requirements}` of the tester, build and review with its IDs and texts. Issue: #76. | T | Implemented |
| WB-PROMPT-26 | When an issue's file is accepted, `wb prompt` shall add the interface and what the role may change and read as parts after the role's text. Issue: #76. | T | Implemented |
| WB-PROMPT-27 | When an issue's file is there and not accepted, or changed since its acceptance, `wb prompt` shall print nothing on standard output and end with 1, with the reason on standard error. Issue: #76. | T | Implemented |
| WB-PROMPT-28 | When an issue has no file, `wb prompt` shall hand out its prompts as before. Replaced by WB-PROMPT-50. Issue: #76. | T | Deprecated |
| WB-PROMPT-29 | `wb prompt breakdown <issue>` shall hand out the breakdown's prompt without an issue's file. Replaced by WB-PROMPT-52. Issue: #76. | T | Deprecated |
| WB-PROMPT-30 | The stamp and `--parts` shall name the issue's file and its SHA-256 as a part, when one is used. Issue: #76. | T | Implemented |
| WB-PROMPT-31 | `wb prompt <role> <issue>` shall take the first word after the role as the issue, as `--issue` does; both together, a third word, or an empty issue shall end with 2. Issue: #72. | T | Implemented |
| WB-PROMPT-32 | `wb prompt` shall fill `checkout` with the root of the repository it runs in, unless `--checkout` is given. Issue: #72. | T | Implemented |
| WB-PROMPT-33 | `wb prompt` shall fill `issue_address` and `change_address` from the patterns of `watchbill.yml` with the issue and the change, unless the option is given. Issue: #72. | T | Implemented |
| WB-PROMPT-34 | For the build, the review and the fix, `wb prompt` shall fill `branch` with the one local branch whose name is the pattern or starts with it and a `-`, unless `--branch` is given; none or several shall be refused with 1, naming the branches found, with nothing on standard output; the tester takes the pattern filled. Replaced by WB-PROMPT-49. Issue: #72. | T | Deprecated |
| WB-PROMPT-35 | `wb prompt` shall fill `requirements` with "the numbered requirements of issue {issue}", unless `--requirements` is given. Replaced by WB-PROMPT-51. Issue: #72. | T | Deprecated |
| WB-PROMPT-36 | An option given to `wb prompt` shall win over the value derived for it. Issue: #72. | T | Implemented |
| WB-PROMPT-37 | A `watchbill.yml` with no patterns shall leave the options needed as before, and `wb check` shall not fail on it; a pattern that is not a text shall be refused by the loader. Issue: #72. | T | Implemented |
| WB-PROMPT-38 | The stamp shall hold the values as handed out, derived or given; a derived value that is not one plain line shall be refused with 1, naming where it came from. Issue: #72. | T | Implemented |
| WB-PROMPT-39 | An issue or change that fills a pattern of `wb prompt` shall be plain (letters, digits, `.`, `_`, `-`; no `..`, not starting with `.` or `-`, not ending with `.` or `.lock`), or be refused by its option name with 2 and nothing on standard output. Issue: #72. | T | Implemented |
| WB-PROMPT-40 | `wb prompt` shall refuse, by the name of its list and with status 1, an issue's file whose path holds a control character, U+2028, U+2029 or U+202E, and print nothing on standard output. Issue: #76. | T | Implemented |
| WB-PROMPT-41 | An option `--requirements` given to `wb prompt` for an issue with a file shall win over the file, and the stamp shall say so with the line `requirements: given` before its part lines. Issue: #76. | T | Implemented |
| WB-PROMPT-42 | When an issue's file is accepted, `wb prompt` shall give the fix the interface and what it may change and read, and no requirement, as its text holds no `{requirements}`. Issue: #76. | T | Implemented |
| WB-PROMPT-43 | `wb prompt` shall refuse, by the name of the field and with status 1, an issue's file whose interface, requirement ID or text, or path holds a control character (a line break and a tab are allowed in the interface only), U+2028, U+2029, U+202A to U+202E or U+2066 to U+2069. Issue: #76. | T | Implemented |
| WB-PROMPT-44 | `wb prompt` shall read an issue's file once, use those bytes for its hash, its form and the prompt, and refuse a file over 1 MiB with a message and status 1. Issue: #76. | T | Implemented |
| WB-PROMPT-45 | `wb prompt` shall refuse the options `--task` and `--task_address` as options that are not known, with status 2 and the usage. Issue: #78. | T | Implemented |
| WB-PROMPT-46 | `wb prompt` shall refuse, with status 1, a `watchbill.yml` that holds the key `task_address`, naming the key and saying to write `issue_address` in its place. Issue: #78. | T | Implemented |
| WB-PROMPT-47 | `wb prompt` shall refuse, with status 1, a `watchbill.yml` whose pattern `issue_address`, `change_address` or `branch` holds `{task}`, naming the key and saying to write `{issue}` in its place. Issue: #78. | T | Implemented |
| WB-PROMPT-48 | The stamp shall name the issue on the line `issue:` and hold no line `task:`. Issue: #78. | T | Implemented |
| WB-PROMPT-49 | `wb prompt` shall fill `branch` for the breakdown with the pattern filled, and for the tester, the build, the review and the fix with the one local branch whose name is the pattern or starts with it and a `-`, unless `--branch` is given; none or several shall be refused with 1, naming the branches found, with nothing on standard output. Replaces WB-PROMPT-34. Issue: #92. | T | Implemented |
| WB-PROMPT-50 | When `watchbill.yml` says `breakdown: none` and an issue has no file, `wb prompt` shall hand out its prompts as before. Replaced by WB-TASK-17. Issue: #97. | T | Deprecated |
| WB-PROMPT-51 | When `watchbill.yml` says `breakdown: none` and an issue has no file, `wb prompt` shall fill `requirements` with "the numbered requirements of issue {issue}", unless `--requirements` is given. Replaced by WB-TASK-17 and WB-PROMPT-53. Issue: #97. | T | Deprecated |
| WB-PROMPT-52 | Given `--runner` with a runner that the organisation has cleared for the breakdown up to the repository's level, `wb prompt breakdown <issue>` shall hand out the breakdown's prompt without an issue's file. Replaces WB-PROMPT-29. Issue: #109. | T | Implemented |
| WB-PROMPT-53 | `wb prompt review --change <n>` and `wb prompt fix --change <n>` shall be handed out with no issue: the stamp's `issue:` is empty, and `requirements` is "what the description of change {change} says the change does" unless `--requirements` is given. Replaces WB-PROMPT-51. Issue: #116. | T | Implemented |
| WB-PROMPT-54 | With no issue, `wb prompt fix` shall take `{branch}` from `--branch` or the branch checked out when it is the change's, and refuse with 2 otherwise, saying to give `--branch`; `wb prompt tester` and `wb prompt build` with no issue shall be refused with 2 and the usage. Issue: #116. | T | Implemented |
| WB-PROMPT-55 | `wb prompt --stamp` shall write on `round:` the round that `--round` gives, for every role; or, for a role whose text holds `{round}`, the round worked out from the patterns of `watchbill.yml`; with neither, `round:` shall be empty, and for the tester and the build no pattern shall be run. Issue: #136. | T | Implemented |
| WB-PROMPT-56 | `wb prompt` and `wb start` shall take the project's text as `HEAD` has it, and refuse with 1, naming the file, a part that differs from `HEAD`, is not in `HEAD`, or is in `HEAD` and not in the working tree. Issue: #211. | T | Implemented |
| WB-PROMPT-57 | No flag shall lift WB-PROMPT-56. Issue: #211. | T | Implemented |
| WB-PROMPT-58 | `wb prompt --stamp` shall print a line `job: <hash>`, besides the lines it printed before. Issue: #236. | T | Draft |
| WB-PROMPT-59 | `wb prompt --stamp` shall print a line `base: <commit>`, the commit of `HEAD` the view was made from, besides the lines it printed before. Issue: #236. | T | Draft |
| WB-PROMPT-60 | `wb prompt <role> <issue> --stamp` for a role that runs in a view shall print the line `level: <level>` after the line `view:`, the level of the repository as `--parts` of the breakdown gives it, from the function the stamp of `wb start` is made by. Issue: #209. | T | Implemented |
| WB-PROMPT-61 | A stamp of a session that runs without a view shall hold no line `level:`. Issue: #209. | T | Implemented |
| WB-PROMPT-62 | Where the organisation's file can be read and `watchbill.yml` gives no level that holds (a level not on the organisation's list, or the key `levels` or `runners`), the `level:` of the stamp shall be the organisation's most restrictive level, and `wb prompt` shall say why on standard error. Issue: #209. | T | Implemented |
| WB-PROMPT-63 | Where there is no organisation's file and `watchbill.yml` has no line `confidentiality:`, the `level:` of the stamp shall be `not known`, with nothing added to standard error. Issue: #209. | T | Implemented |
| WB-PROMPT-64 | The `level:` and `view:` lines of `wb prompt <role> <issue> --stamp` shall equal those that `wb start <role> <issue>` posts for the same `HEAD`. Issue: #209. | T | Implemented |
| WB-PROMPT-65 | `wb prompt` shall fill `{classed}` with the paths of `classed` of `watchbill.yml`, one to a line, and with `none` where the list is `classed: []`. Issue: #227. | T | Implemented |
| WB-PROMPT-66 | `wb prompt` shall fill `{classed}` with ``every path (watchbill.yml has no `classed`)`` where `watchbill.yml` has no `classed`, and `{classed_touched}` then with every path of the change but the issue's own files. Issue: #227. | T | Implemented |
| WB-PROMPT-67 | `wb prompt` shall fill `{classed_touched}` with the paths of the change, from the diff of its branch against the merge base with the default branch, that are a classed path or lie below one, one to a line, and with `none` where there are none. Only the issue's own files `.watchbill/issues/<issue>.yml` and `.watchbill/issues/<issue>.accepted` are not among them: those of another issue are, and where there is no issue none is left out. Issue: #227. | T | Implemented |
| WB-PROMPT-68 | `wb prompt` shall fill `{classed_touched}` with `cannot be told: ` and the reason where the diff cannot be had: the branch of the change is not known, no default branch is there to compare with, the branch `<name>` is not there or the branch has no merge base with the default branch. Issue: #227. | T | Implemented |
| WB-PROMPT-69 | `{classed}` and `{classed_touched}` shall be on the list of `placeholders.md`, and `wb check` shall take them in the project's own text. Issue: #227. | T | Implemented |
| WB-PROMPT-70 | `wb prompt` shall put in the value of every placeholder but `{classed}` and `{classed_touched}` as it is given, with the lines of a value of several lines not indented, where the placeholder stands after spaces. Issue: #227. | T | Implemented |
