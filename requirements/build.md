# The starter

What `wb start <role> <issue>` checks, posts and starts. One path for a person and, after the sandbox, for the listener ([listen.md](listen.md)).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-BUILD-1 | `wb start <role> <issue>` shall refuse what `wb prompt` refuses for the same words, with the same message and status, before it posts or starts anything. Issue: #84. | T | Implemented |
| WB-BUILD-2 | `wb start <role>` shall refuse when the rule table does not let the role start on this issue now, name the rule, and end with 1; the number of review rounds is not such a rule for a person. Issue: #84. | T | Implemented |
| WB-BUILD-3 | `wb start <role>` shall refuse an issue whose `changes` hold classed code, name #65, and end with 1. Issue: #84. | T | Implemented |
| WB-BUILD-4 | `wb start <role>` shall make its checks in the order: those of `wb prompt`, the rule table, the class, the runner's `start`, the terminal; the first fault ends it, and nothing is posted or started. Issue: #84. | T | Implemented |
| WB-BUILD-5 | `wb start <role>` shall say on standard error what it will post, then post the stamp by running `issue_comment` of `watchbill.yml` with `{issue}` filled and the stamp on standard input, its first line `[stamp] <role> round <n>`; when that command does not end with 0, it shall start nothing and end with 1. Issue: #84. | T | Implemented |
| WB-BUILD-6 | `wb start <role>` shall start the session by the runner's `start` of the organisation's file, with `{model}`, `{effort}` and `{prompt}` filled, each word as one argument and no shell, in the root of the checkout, in the foreground; it shall end with 0 when the session ends with 0, and with 1 otherwise. Issue: #84. | T | Implemented |
| WB-BUILD-7 | `wb start <role>` shall refuse with 1 when standard input or standard output is not a terminal, naming #65, and a runner that has no `start`, naming `--prompt`. Issue: #84. | T | Implemented |
| WB-BUILD-8 | `wb start <role> --prompt` shall print what `wb prompt` prints for the same words, and post and start nothing. Issue: #84. | T | Implemented |
| WB-BUILD-9 | `wb start <role>` shall ask nothing of the network but through `issue_comment`, `issue_comments`, `change_of_branch` and the runner's `start`. Issue: #84. | T | Implemented |
| WB-BUILD-10 | `wb start <role>` shall take the runner of `--runner`, else the organisation's `default`, and refuse with 2 and the usage when there is neither. Issue: #84. | T | Implemented |
| WB-BUILD-11 | Where `--round` is not given, `wb start <role>` and `wb prompt` shall take the round from the `[stamp]` lines that `issue_comments` prints: one more than the stamps of the role for the breakdown, the tester, the build and the review, and the number of review stamps for the fix; a fix with no review stamp shall be refused with 1. A round past the rule table's greatest number shall not be refused when a person starts the role. Issue: #84. | T | Implemented |
| WB-BUILD-12 | Where `--change` is not given, `wb start <role>` and `wb prompt` shall take the one change that `change_of_branch` prints for the branch; none or more than one shall be refused with 1, naming what was found and `--change`. Issue: #84. | T | Implemented |
| WB-BUILD-13 | `wb start <role>` shall name on standard error the round and the change it worked out, and from what, before it posts the stamp. Issue: #84. | T | Implemented |
| WB-BUILD-14 | `wb start review` shall read `change_checks` before it posts the stamp: `pending` and `red` shall be refused with 1, `red` saying that the change goes to a person; `green` shall be started. Issue: #127. | T | Implemented |
| WB-BUILD-15 | `wb start review` shall refuse with 1, naming it, a `change_checks` that prints anything but `green`, `red` or `pending`, or ends with anything but 0. Issue: #127. | T | Implemented |
| WB-BUILD-16 | Where `watchbill.yml` has no `change_checks`, `wb start review` shall start, and say on standard error that the CI was not read. Issue: #127. | T | Implemented |
| WB-BUILD-17 | For a review or a fix with no issue, `wb start <role>` shall post the stamp by running `change_comment` of `watchbill.yml` with `{change}` filled and the stamp on standard input, and run no `issue_comment`. Issue: #84, #116. | T | Implemented |
| WB-BUILD-18 | For a review or a fix with no issue, where `--round` is not given, `wb start <role>` and `wb prompt` shall take the round from the `[stamp]` lines that `change_comments` prints, as from `issue_comments` for an issue; a fix with no review stamp on the change shall be refused with 1. Issue: #84, #116. | T | Implemented |
| WB-BUILD-19 | A path of the issue's `changes` shall be classed when it is a path of the list `classed` of `watchbill.yml`, lies below one, or is a folder above one; with no `classed`, every path shall be classed; a `classed` that is not a list of relative paths without `..` shall be refused with 1 and its place. Issue: #84. | T | Implemented |
| WB-BUILD-20 | `wb start <role>` for any role but the breakdown, with no issue's file, shall be refused with 1, naming #65, unless `classed` of `watchbill.yml` is an empty list. Issue: #84. | T | Implemented |
| WB-BUILD-21 | `wb start <role>` and `wb prompt` shall run each pattern of `watchbill.yml` they use as one line of `/bin/sh -c`, in the root of the checkout, and refuse a value filled into it that is not plain: with 2, by its option, when it was given, and with 1, saying to give `--change`, when it is a branch worked out. Issue: #84. | T | Implemented |
| WB-BUILD-22 | `wb start <role>` shall refuse with 2 and the usage a `--round` that is not a whole number from 1 written without a leading zero. Issue: #84. | T | Implemented |
| WB-BUILD-23 | `wb start <role>` shall refuse with 1, naming it, a `watchbill.yml` without `issue_comment`, or without `change_comment` for a change with no issue, before it posts or starts anything. Issue: #84. | T | Implemented |
| WB-BUILD-24 | `wb start <role>` shall refuse with 1, naming `--prompt`, a word of `start` that is 128 KiB or more once filled, before it posts or starts anything. Issue: #84. | T | Implemented |
| WB-BUILD-25 | While the session runs, an interrupt (SIGINT) or a quit (SIGQUIT) sent to `wb start <role>` shall not end it, and it shall end with the status that WB-BUILD-6 gives. Issue: #84. | T | Implemented |
| WB-BUILD-26 | The README shall show a worked `start` for Claude Code and one for goose with an Ollama model, and say that where a `start` leaves out `{model}` or `{effort}`, the stamp's model or effort is not what ran. Issue: #84. | I | Implemented |
| WB-BUILD-27 | The stamp that `wb start <role>` posts shall hold on its line `round:` the round of its first line, for every role, also where the text of the role has no `{round}`. Issue: #136. | T | Implemented |
| WB-BUILD-28 | `wb start review <issue>` and `wb start fix <issue>` shall say on standard error, before the checks, the issue, the change, and whether the change was found from the branch, naming it, or given. Issue: #162. | T | Implemented |
| WB-BUILD-29 | `wb start review --change <n>` and `wb start fix --change <n>` with no issue shall say the change on standard error before the checks. Issue: #162. | T | Implemented |
| WB-BUILD-30 | Before the stamp is posted, for a role that runs in a view, `wb` shall name each path of the view whose file in the working tree differs from `HEAD`, and end with 1. Issue: #172. | T | Draft |
| WB-BUILD-31 | Before the stamp is posted, for a role that runs in a view, `wb` shall name each untracked file under a path of `changes`, and end with 1. Issue: #172. | T | Draft |
| WB-BUILD-32 | `--ignore-local` shall let the start go on when every local change is under the role's `reads` only. Issue: #172. | T | Draft |
| WB-BUILD-33 | A local change under `changes` shall be refused with 1 also with `--ignore-local`, saying that it cannot be overridden. Issue: #172. | T | Draft |
| WB-BUILD-34 | A start that `--ignore-local` let go on shall have the line `local: ignored <n>` in its stamp, after `view:`, and the file `local` with the paths in its session's folder. Issue: #172. | T | Draft |
| WB-BUILD-35 | The refusal of a local change shall say that it is a guard, name the paths and their list, and name `--ignore-local`. Issue: #172. | T | Draft |
| WB-BUILD-36 | `wb start <role> <issue>` shall do what `wb <role> <issue>` did, with the same options. Issue: #175. | T | Implemented |
| WB-BUILD-37 | `wb <role> …` for a role of the revision shall end with 2 and say the form `wb start <role> …` with the same words. Issue: #175. | T | Implemented |
| WB-BUILD-38 | `wb start` with a role that is not known shall end with 2 and name the roles. Issue: #175. | T | Implemented |
| WB-BUILD-39 | Every command that runs a pattern of `watchbill.yml`, or reads `needs` or `build_files` from it, shall first compare the file in the working tree with `HEAD`, and refuse with 1, naming the file, where it differs or is not committed, before any pattern runs or anything is posted. Issue: #186. | T | Implemented |
| WB-BUILD-40 | No flag shall lift the refusal of WB-BUILD-39. Issue: #186. | T | Implemented |
| WB-BUILD-41 | The refusal of WB-BUILD-39 shall say that a person reads the change and commits it, or takes it back with git, before the next command. Issue: #186. | T | Implemented |
| WB-BUILD-42 | `wb init` and `wb upgrade` shall not be held by WB-BUILD-39, and `wb upgrade` shall say that the file is to be committed. Issue: #186. | T | Draft |
