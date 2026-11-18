# The starter

What `wb <role> <issue>` checks, posts and starts. One path for a person and, after the sandbox, for the listener ([listen.md](listen.md)).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-BUILD-1 | `wb <role> <issue>` shall refuse what `wb prompt` refuses for the same words, with the same message and status, before it posts or starts anything. Issue: #84. | T | Draft |
| WB-BUILD-2 | `wb <role>` shall refuse when the rule table does not let the role start on this issue now, name the rule, and end with 1. Issue: #84. | T | Draft |
| WB-BUILD-3 | `wb <role>` shall refuse an issue whose `changes` hold classed code, name #65, and end with 1. Issue: #84. | T | Draft |
| WB-BUILD-4 | `wb <role>` shall make its checks in the order: those of `wb prompt`, the rule table, the class, the runner's `start`, the terminal; the first fault ends it, and nothing is posted or started. Issue: #84. | T | Draft |
| WB-BUILD-5 | `wb <role>` shall say on standard error what it will post, then post the stamp by running `issue_comment` of `watchbill.yml` with `{issue}` filled and the stamp on standard input, its first line `[stamp] <role> round <n>`; when that command does not end with 0, it shall start nothing and end with 1. Issue: #84. | T | Draft |
| WB-BUILD-6 | `wb <role>` shall start the session by the runner's `start` of the organisation's file, with `{model}`, `{effort}` and `{prompt}` filled, each word as one argument and no shell, in the root of the checkout, in the foreground; it shall end with 0 when the session ends with 0, and with 1 otherwise. Issue: #84. | T | Draft |
| WB-BUILD-7 | `wb <role>` shall refuse with 1 when standard input or standard output is not a terminal, naming #65, and a runner that has no `start`, naming `--prompt`. Issue: #84. | T | Draft |
| WB-BUILD-8 | `wb <role> --prompt` shall print what `wb prompt` prints for the same words, and post and start nothing. Issue: #84. | T | Draft |
| WB-BUILD-9 | `wb <role>` shall ask nothing of the network but through `issue_comment`, `issue_comments`, `change_of_branch` and the runner's `start`. Issue: #84. | T | Draft |
| WB-BUILD-10 | `wb <role>` shall take the runner of `--runner`, else the organisation's `default`, and refuse with 2 and the usage when there is neither. Issue: #84. | T | Draft |
| WB-BUILD-11 | Where `--round` is not given, `wb <role>` and `wb prompt` shall take the round from the `[stamp]` lines that `issue_comments` prints: one more than the stamps of the role for the breakdown, the tester, the build and the review, and the number of review stamps for the fix; a fix with no review stamp, or a review beyond the rule table's greatest number of rounds, shall be refused with 1. Issue: #84. | T | Draft |
| WB-BUILD-12 | Where `--change` is not given, `wb <role>` and `wb prompt` shall take the one change that `change_of_branch` prints for the branch; none or more than one shall be refused with 1, naming what was found and `--change`. Issue: #84. | T | Draft |
| WB-BUILD-13 | `wb <role>` shall name on standard error the round and the change it worked out, and from what, before it posts the stamp. Issue: #84. | T | Draft |
