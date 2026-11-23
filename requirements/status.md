# Where each issue stands

What `wb status` says of the issues in the flow. It only reads: it writes nothing but a temporary repository in `$TMPDIR`, removed after, where it reads a view kept with work not taken back (#222).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-STATUS-1 | `wb status` shall print a heading for each step that has issues, and under it one line per issue: its number and its title. Replaced by WB-STATUS-36. Issue: #139, #155. | T | Deprecated |
| WB-STATUS-2 | `wb status` shall put an issue under the first heading that holds: `Fixed` (the last stamp is `[stamp] fix`), `Reviewed` (the last stamp is `[stamp] review`), `Built` (a `[stamp] build`), `Tested` (a `[stamp] tester`), `Accepted` (the issue's file and an acceptance that holds for it, on the issue's branch), `Broken down` (the issue's file on its branch), `Not broken down`. Issue: #139. | T | Implemented |
| WB-STATUS-3 | `wb status` shall count a `[question]` as open until an `[answer]` of a person to its name closes it. Issue: #139. | T | Implemented |
| WB-STATUS-4 | `wb status` shall take the issues and their titles from the pattern `open_issues` of `watchbill.yml`, and without it the issues that have a branch of the pattern `branch` or a file under `.watchbill/issues/`, with no title. Replaced by WB-STATUS-32 and WB-STATUS-33. Issue: #139, #155. | T | Deprecated |
| WB-STATUS-5 | `wb status <issue>` shall print the heading and the line of that issue and, under them, the lines it was worked out from. Issue: #139. | T | Implemented |
| WB-STATUS-6 | `wb status --stage <heading>` shall print only that heading, the name matched without regard to case. Issue: #139. | T | Implemented |
| WB-STATUS-7 | When a pattern fails for one issue, `wb status` shall list that issue under `Not read` with why, print the others, and end with 1. Issue: #139. | T | Implemented |
| WB-STATUS-8 | `wb status` shall write nothing. Issue: #139. | T | Implemented |
| WB-STATUS-9 | After the steps, `wb status` shall print `Questions open`: each issue that has an open question, with the names of those questions in the order they were asked. Issue: #139. | T | Implemented |
| WB-STATUS-10 | `wb status` shall print a title with its control characters escaped. Replaced by WB-STATUS-37. Issue: #139, #155. | T | Deprecated |
| WB-STATUS-11 | `wb status` shall not print a heading that has no issue. Issue: #139. | T | Implemented |
| WB-STATUS-12 | `wb status` shall print no line that holds a round, a command to run or a word for a step going on. Issue: #139. | T | Implemented |
| WB-STATUS-13 | `wb status` shall print the steps in the order of the flow, `Not broken down` first and `Fixed` last. Issue: #139. | T | Implemented |
| WB-STATUS-14 | `wb status` shall never take a line that starts with `[question]` for an answer. Issue: #139. | T | Implemented |
| WB-STATUS-15 | Until `wb` reads who wrote a comment, `wb status` shall take every `[answer]` line for a person's. Issue: #139. | T | Implemented |
| WB-STATUS-16 | `wb status --stage` with a heading that is not known shall end with 2. Issue: #139. | T | Implemented |
| WB-STATUS-17 | `wb status` shall switch no branch. Issue: #139. | T | Implemented |
| WB-STATUS-18 | `wb status` shall read the issue's file and its acceptance from the branch with `git show`, so that an acceptance that is written and not committed leaves the issue under `Broken down`. Issue: #139. | T | Implemented |
| WB-STATUS-19 | `wb status` shall run no pattern but `open_issues` and `issue_comments`. Replaced by WB-STATUS-38. Issue: #139, #155. | T | Deprecated |
| WB-STATUS-20 | `wb status` used wrongly, with an option or a word that is not known, shall end with 2. Issue: #139. | T | Implemented |
| WB-STATUS-21 | `wb status` in a folder that is not in a git repository, or where Watchbill is not set up, shall end with 2. Issue: #139. | T | Implemented |
| WB-STATUS-22 | `wb status` shall keep an issue that has an open question under its step. Issue: #139. | T | Implemented |
| WB-STATUS-23 | When `open_issues` fails, `wb status` shall say so on standard error, list no issue, and end with 1. `open_issues` is no longer read (#155). Issue: #139, #155. | T | Deprecated |
| WB-STATUS-24 | `wb status <issue>` shall print the lines it worked out from under the heading `Worked out from:`. Issue: #139. | T | Implemented |
| WB-STATUS-25 | `wb status` shall list a line of `open_issues` whose first field is not a plain issue name under `Not read`, with why, and end with 1. `open_issues` is no longer read (#155). Issue: #139, #155. | T | Deprecated |
| WB-STATUS-26 | Without `issue_comments`, `wb status` shall say so once on standard error. Issue: #139. | T | Implemented |
| WB-STATUS-27 | `wb --help` shall name `wb status`. Replaced by WB-CMD-10. Issue: #139. | D | Deprecated |
| WB-STATUS-28 | `wb status` shall run from any folder inside the repository. Issue: #139. | T | Implemented |
| WB-STATUS-29 | `wb status` shall take `[answer] <issue> suggested` as an answer to every question asked before it, and to none asked after it. Issue: #139. | T | Implemented |
| WB-STATUS-30 | `wb status` shall close a question by an `[answer]` of its name whatever follows the name, a choice number too. Issue: #139. | T | Implemented |
| WB-STATUS-31 | `wb status` shall let only the stamps after the last `[stamp] breakdown` decide the step, so that a new breakdown takes the issue back to `Accepted` or `Broken down` by its file. Issue: #139. | T | Implemented |
| WB-STATUS-32 | `wb status` shall list an issue only when its branch, by the pattern `branch`, local or at `origin`, holds a commit that the default branch of `origin` does not, and the branch is not done: its whole change is not one commit on the default branch (the same `git patch-id --stable` from the merge base), and it is not a local branch whose upstream at `origin` is gone. A listing of the branches that fails ends with 2. Issue: #155. | T | Implemented |
| WB-STATUS-33 | `wb status` shall find the issues it lists with git alone, with no pattern. Issue: #155. | T | Implemented |
| WB-STATUS-34 | `wb status` shall put under `Not broken down` an issue whose branch holds no issue's file. Issue: #155. | T | Implemented |
| WB-STATUS-35 | `wb status <issue>` shall read an issue that is not in the flow, and say that it is not. Issue: #155. | T | Implemented |
| WB-STATUS-36 | `wb status` shall print one line per issue in the list, with its number and its branch, and no title. Issue: #155. | T | Implemented |
| WB-STATUS-37 | `wb status <issue>` shall print the issue's title from the first line of `issue_address`, with its control characters escaped. Issue: #155. | T | Implemented |
| WB-STATUS-38 | `wb status` shall run no pattern but `issue_comments` and `change_of_branch`, each once for each issue it lists, and `wb status <issue>` also `issue_address`. Issue: #155. | T | Implemented |
| WB-STATUS-39 | `wb status` shall take the default branch from `refs/remotes/origin/HEAD`, else `main`, else `master`, and end with 2 when none is there. Issue: #155. | T | Implemented |
| WB-STATUS-40 | Where the local branch of an issue holds commits that its branch at `origin` does not, `wb status` shall add how many to the issue's line, as `2 not pushed`. Issue: #155. | T | Implemented |
| WB-STATUS-41 | `wb status` shall read the refs as they are, and fetch nothing. Issue: #155. | T | Implemented |
| WB-STATUS-42 | `wb status` shall show on the line of an issue the change that `change_of_branch` prints for its branch, as `#<n>`, and none where it prints none. Issue: #155. | T | Implemented |
| WB-STATUS-43 | `wb status` shall show the open questions of an issue as they are on the job's ref, with no tracker set up. Issue: #238. | T | Draft |
| WB-STATUS-44 | `wb status` shall show a question as answered once `wb answer` has written a signed commit with its answer. Issue: #238. | T | Draft |
| WB-STATUS-45 | `wb status` shall say, where it sees an open question, that an answer typed at the tracker is not read and is given with `wb answer`. Issue: #238. | T | Draft |
| WB-STATUS-46 | `wb status --session <hash>`, the hash of a stamp commit or a unique prefix of 12 or more hex digits, shall print the stamp's lines, the base commit, the commits whose note names the stamp, newest last, and the last of them, and end with 0. Issue: #239. | T | Draft |
| WB-STATUS-47 | `wb status --session` shall say that a session has no commits, and end with 0, when no note names its stamp. Issue: #239. | T | Draft |
| WB-STATUS-48 | `wb status --session` shall say "not known on this machine" of how a session ended where the session did not run on this machine, and read how it ended from the lines of the start where it did. Issue: #239. | T | Draft |
| WB-STATUS-49 | `wb status --session` shall end with 2 when the prefix it is given has fewer than 12 hex digits. Issue: #239. | T | Draft |
| WB-STATUS-50 | `wb status --session` shall end with 1 and say which, when the prefix is not unique or names no stamp. Issue: #239. | T | Draft |
| WB-STATUS-51 | `wb status --session` shall write nothing, and give the same output with the network off. Issue: #239. | T | Draft |
| WB-STATUS-52 | `wb status` shall say of a job whose source has changed since `wb add` took it in, where the team gives a pattern that reads the source, that it has changed, and shall take nothing in. Issue: #240. | T | Draft |
| WB-STATUS-53 | `wb status <issue>` shall print `waits for: <role> (<question>)` for the newest answer that names a role, `person` included, and no such line for the list. Issue: #226. | T | Implemented |
| WB-STATUS-54 | `wb status <issue>` shall print no `waits for` line where no `[answers]` first line names a role. Issue: #226. | T | Implemented |
| WB-STATUS-55 | `wb status <issue>` shall run `issue_comments` once for the issue, as before, also where it prints `waits for`. Issue: #226. | T | Implemented |
| WB-STATUS-56 | `wb status <issue>` shall not print `waits for: <role>` once the issue has a stamp of that role newer than the answer, and shall still print it for a stamp of another role; `person` has no stamp and is never cleared. Issue: #226. | T | Implemented |
| WB-STATUS-57 | `wb status <issue>` shall count as a role in an `[answers]` first line only a word of the roles of the revision or `person`; for another word, as `q3>optional`, it closes the question and prints no `waits for` line. Issue: #226. | T | Implemented |
| WB-STATUS-58 | `wb status <issue>` shall print, for each view of the issue kept with work not taken back, from any role, a line `kept: <role> round <n>, <k> commits not taken, <folder>` (`1 commit` for one, and `, <m> paths uncommitted` where paths were left), and none for a view with nothing in it. Issue: #222. | T | Implemented |
| WB-STATUS-59 | `wb status` with no issue shall say once on standard error how many views with work not taken back are kept on this machine, and say nothing of them where there are none. Issue: #222. | T | Implemented |
| WB-STATUS-60 | `wb status <issue>` shall print `cut off: <role> round <n>, <folder>` for a session of the issue whose `outcome` says `running` and whose process ID is gone, or is not `wb`, or is of another start time. Issue: #222. | T | Implemented |
| WB-STATUS-61 | `wb status <issue>` shall not call a session cut off whose `outcome` says `running` while a `wb` of that process ID is alive. Issue: #222. | T | Implemented |
| WB-STATUS-62 | `wb status` shall read a kept view only through a repository of `wb`'s own: it shall run no hook and no setting of the view, and write nothing in it or in the state folder. Issue: #222. | T | Implemented |
| WB-STATUS-63 | `wb status <issue>` shall find the commits not taken and the paths left uncommitted of a view whose session left no `back`, from the objects of the view. Issue: #222. | T | Implemented |
| WB-STATUS-64 | Every git run on a kept view shall end after 3 seconds, the child killed and waited for, and the view shall count as `not read`: `wb status <issue>` shall print `kept: <role> round <n>, not read: <why>, <folder>` and `wb start` shall refuse naming it, in less than 10 seconds, also where an object of the view is a FIFO. Issue: #222. | T | Implemented |
| WB-STATUS-65 | `wb status` shall skip a session whose `outcome` says `running` while its `wb` is alive: no `kept:` line for its view, no `cut off:` line, and no count of it on standard error. Issue: #222. | T | Implemented |
| WB-STATUS-66 | `wb status` and the refusal of `wb start` shall read the subject of a commit of a kept view with a cap: a commit over 64 KiB is named by its size, and a subject is cut at 200 characters. Issue: #222. | T | Implemented |
| WB-STATUS-67 | No filter driver or textconv defined in a config of the person or of the system shall run on a file of a kept view: every git run on a view reads neither, nor the view's own config. Issue: #222. | T | Implemented |
| WB-STATUS-68 | `wb status` shall write nothing but a temporary repository in `$TMPDIR`, removed after, where it reads a kept view: the folder is not left behind. Issue: #222. | T | Implemented |
| WB-STATUS-69 | `wb status` and the refusal of `wb start` shall say a kept view of more than 200 commits not taken as the first 200 commits and `and <k> more`, not as `200 commits not taken`. Issue: #265. | T | Draft |
| WB-STATUS-70 | `wb status` shall say, for a view whose commit is a FIFO so that `merge-base` is ended after its bound, that the read of `merge-base` took more than its bound, and not that the view's HEAD does not come from the commit the view was made with. Issue: #265. | T | Draft |
| WB-STATUS-71 | `wb status` shall end within one deadline of 20 seconds for each view that is slow on every git run, each run below its bound, and say the view is `not read`. Issue: #265. | T | Draft |
| WB-STATUS-72 | The message of a git run that ended after its bound shall name the git command (the first word that is not an option), never `-c`. Issue: #265. | T | Draft |
| WB-STATUS-73 | The usage of `wb status`, `wb-status.1`, `requirements/status.md` and the code shall call what `wb status` writes in `$TMPDIR` "a temporary repository". Issue: #265. | I | Draft |
| WB-STATUS-74 | The README shall say that a Ctrl-C, a termination or a kill of `wb status` can leave the temporary repository. Issue: #265. | I | Draft |
