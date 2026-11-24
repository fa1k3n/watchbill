# The command layout

Which commands `wb` has, how they are grouped, and what a planned one says.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-CMD-1 | `wb` shall have the commands `init`, `check`, `doctor`, `upgrade`, `plan`, `accept`, `start`, `run`, `status`, `view`, `log`, `answer`, `revert`, `clean`, `prompt`, `revision` and `compare`. Issue: #171, #234. | T | Implemented |
| WB-CMD-2 | A command that is not built yet shall end with 2, write nothing, and ask nothing of the network. Issue: #171. | T | Implemented |
| WB-CMD-3 | A command that is not built yet shall say on standard error the release it is planned for and the issue that builds it. Issue: #171. | T | Implemented |
| WB-CMD-4 | `wb help` shall print the groups set up, work, see, take back and plumbing, in that order, each with its commands. Issue: #171. | T | Implemented |
| WB-CMD-5 | `wb help` shall name the release and the issue of each command that is not built yet. Issue: #171. | T | Implemented |
| WB-CMD-6 | `wb help <command>` shall print the usage and the help of that command. Issue: #171. | T | Implemented |
| WB-CMD-7 | `wb help` with a command that is not known shall end with 2. Issue: #171. | T | Implemented |
| WB-CMD-8 | `wb --help`, and `wb` with no command, shall print what `wb help` prints. Issue: #171. | T | Implemented |
| WB-CMD-9 | `wb help` and the commands `wb` takes shall be made from one list of the commands, and the usage text `USAGE` shall name the same built commands, held by a test. Issue: #171. | T | Implemented |
| WB-CMD-10 | `wb help` shall list `status` in the group see, and `wb help status` shall print its usage. Replaces WB-STATUS-27. Issue: #171. | T | Implemented |
| WB-CMD-11 | `wb help <command>` with a word more shall end with 2 and name the word. Issue: #171. | T | Implemented |
| WB-CMD-12 | `wb --help` and `wb -h` shall read the word after them as `wb help` does. Issue: #171. | T | Implemented |
| WB-CMD-13 | `wb help help` shall print the usage of `help`. Issue: #171. | T | Implemented |
| WB-CMD-14 | The usage that `wb` prints on a wrong word shall name each built command once, at the start of a line, with its forms as alternatives on that line. Issue: #208. | T | Implemented |
| WB-CMD-15 | The usage that `wb` prints on a wrong word shall list the built commands in the order of `wb help`. Issue: #208. | T | Implemented |
| WB-CMD-16 | The bash completion of `wb` and `watchbill` shall offer the built commands of the one list, and no other, after `wb`. Issue: #220. | T | Implemented |
| WB-CMD-17 | The completion shall offer, for each command, the flags `USAGE` names for it, and no other. Issue: #220. | T | Implemented |
| WB-CMD-18 | The completion shall offer the roles of the revision where a role goes, and the numbers of the files in `.watchbill/issues/` of the checkout where an issue goes. Issue: #220. | T | Implemented |
| WB-CMD-19 | The completion shall run no `wb` and no pattern, ask nothing of the network, and write nothing. Issue: #220. | I | Implemented |
| WB-CMD-20 | No word list that the completion gives to `compgen -W` shall be made by a command or hold a name read from disk: it shall be made of the fixed lists of the file, the flags of a command of those, and literal words; names of issues and paths shall be matched in bash. Issue: #220. | T | Implemented |
| WB-CMD-21 | The headings the completion offers after `--stage` shall be those of `wb status`: `STEPS`, `QUESTIONS` and `NOT_READ`. Issue: #220. | T | Implemented |
| WB-CMD-22 | Each built command shall have a man page of section 1 in `packaging/man/`, `wb-<command>.1`, with its name and a line of NAME, that names every flag `wb help <command>` names for it, and no page shall be there for a command that is not built. Issue: #182. | T | Implemented |
| WB-CMD-23 | The page `wb.1` shall name each built command and no other, each as a reference to its page. Issue: #182. | T | Implemented |
| WB-CMD-24 | The SYNOPSIS of each man page shall be the usage line of its command, word for word, and the `.B` lines of its OPTIONS shall name exactly the flags `wb help <command>` names. Issue: #182. | T | Implemented |
| WB-CMD-25 | Each sentence of the paragraph of `wb help <command>` shall be in the man page of the command, with the roff escapes out and the lines joined; a sentence only the page has is allowed. Issue: #182. | T | Implemented |
| WB-CMD-26 | The EXIT STATUS of each man page shall name 0, 1 and 2. Issue: #182. | T | Implemented |
| WB-CMD-27 | Each refusal of a command, which ends it with 1, shall write on standard error a line that starts with the command's name and `: limit: ` or `: guard: `, the first line of its message. Cases: `wb init`, `wb upgrade`, `wb prompt`, `wb start <role>`, `wb accept`, `wb view`, `wb answer`, `wb revert`, `wb revision`; and an end with 2 has no label. Issue: #180. | T | Implemented |
| WB-CMD-28 | The message of a guard shall name the flag that passes it, and that flag shall be one `wb help <command>` names for its command: `--ignore-local` and `--ignore-kept` of `wb start`, `--no-commit` of `wb accept`; no command shall have `--force`. Issue: #180. | T | Implemented |
| WB-CMD-29 | Each flag that passes a guard shall pass that guard alone: the same words with the flag shall go past it, and a later refusal is still said. Cases: `--ignore-local`, `--ignore-kept`, `--no-commit`. Issue: #180. | T | Implemented |
| WB-CMD-30 | A refusal of `wb prompt` or `wb revision` shall leave standard output empty, and `wb check` shall print no label on either stream where it finds a difference, its last line on standard output as before. Issue: #180. | T | Implemented |
| WB-CMD-31 | Each place in `src/` that ends a command with 1 shall write its refusal by `refusal::limit`, `refusal::guard` or `refusal::say`, or be marked on its line or the line above by `refusal::answer()` or a comment `// not a refusal:`. Shapes: `return 1`, `Err(1)`, `code = 1;`, `=> 1,`, a line `1`, `{ 1 }`, `; 1 }`, `exit(1)`, `_exit(1)`, `1 as`. Issue: #180. | T | Implemented |
| WB-CMD-32 | A command that posts shall check every part before its first post: `wb answer` given an answer for each of two roles, one of them wrong, shall post none and say it as a limit. Issue: #180. | T | Implemented |
| WB-CMD-33 | The README shall state the rules of the layout: each refusal a limit or a guard, a guard named by its flag, no general `--force`, the plumbing's fixed output, a command that posts refuses all when one part is wrong, no `wb config`, and no auto-accept flag. Issue: #180. | I | Implemented |
| WB-CMD-34 | Every command's status shall pass one place, `refusal::ended`, which in a test build (debug assertions) shall fail a command that ends with 1 with neither a refusal written by `refusal` nor a mark of an answer (`refusal::answer`): it says so on standard error, and the program ends with no status (abort). Issue: #180. | T | Implemented |
| WB-CMD-35 | `wb accept` that cannot make the commit at all (no pattern `branch` with `{issue}`, a pattern that failed, git not run) shall refuse with a limit, `limit:`, that names `--no-commit`, which skips the commit. Issue: #180. | T | Implemented |
| WB-CMD-36 | `docs/words.md` shall define a limit and a guard as the README does: a fault `wb` cannot go past is a limit, the three flags that pass a guard are named, and after `--no-commit` the limit that the acceptance be committed (#97) still holds. Issue: #180. | I | Implemented |
