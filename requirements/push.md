# The push after a session

What `wb` does with the issue's branch and the change once the commits of a session are taken back. The session holds no login to the code host.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-PUSH-1 | After the commits of a session of an issue are taken back and nothing was refused, `wb start` shall push the issue's branch to `origin` by `git push origin <commit>:refs/heads/<branch>`, as the person and outside the sandbox, where `watchbill.yml` has `change_of_branch` and `draft_change`. Issue: #151. | T | Draft |
| WB-PUSH-2 | `wb start` shall push only the stamp's branch, at the last commit taken back, and never with force. Issue: #151. | T | Draft |
| WB-PUSH-3 | `wb start` shall say on standard error what it will push, the commit and the branch, and the command, before it pushes. Issue: #151. | T | Draft |
| WB-PUSH-4 | Where a commit was refused at the taking back, or work was left uncommitted, `wb start` shall push nothing, say why, and say the commands of WB-VIEW-14 and WB-VIEW-15. Issue: #151. | T | Draft |
| WB-PUSH-5 | A push that git refuses shall end `wb start` with 1, with what git said, and with no change opened. Issue: #151. | T | Draft |
| WB-PUSH-6 | After a build whose branch was pushed, where `change_of_branch` prints no change, `wb start` shall open one by `draft_change`, with the description on standard input, after it says the command and the description; where it prints one, or more than one, it shall open none. Issue: #151. | T | Draft |
| WB-PUSH-7 | After the tester and the fix, `wb start` shall push and run neither `change_of_branch` nor `draft_change`. Issue: #151. | T | Draft |
| WB-PUSH-8 | `{description}` shall be filled with `/run/watchbill/description.md`, a file of a new folder that `wb start` binds writable at `/run/watchbill` in the sandbox, outside the view's tree, and removes after it has read the file. Issue: #151. | T | Draft |
| WB-PUSH-9 | Where the description file is missing or holds only white space, the change shall be opened with a description that names the issue and says that the session wrote none. Issue: #151. | T | Draft |
| WB-PUSH-10 | Without `change_of_branch` or `draft_change` in `watchbill.yml`, or with `--no-push`, `wb start` shall push nothing, say why, say the commands of WB-VIEW-14 and WB-VIEW-15, and run none of them. Issue: #151. | T | Draft |
| WB-PUSH-11 | After a session of the tester, the build or the fix of an issue, the session's folder shall hold a file `push`: `pushed: <commit>` or `not pushed: <why>`, and after a build that was pushed `change: <number>` or `change: not opened: <why>`. Issue: #151. | T | Draft |
| WB-PUSH-12 | No login of the code host shall be inside the sandbox of a session: the variables of the logins of `gh`, `glab` and ssh are not handed in, and a runner's bind that holds or lies in a login of the code host is refused with 1 before anything is posted. Issue: #151. | T | Draft |
| WB-PUSH-13 | Each notice `wb` posts to a tracker (a stamp made, a session's findings, a handover, questions, answers, done) shall name the hash of the commit on the job's ref that it copies. Issue: #240. | T | Draft |
| WB-PUSH-14 | `wb` shall read back none of the notices it posts to a tracker. Issue: #240. | T | Draft |
| WB-PUSH-15 | Where the session did not end with 0, `wb start` shall push nothing and say why. Issue: #151. | T | Draft |
| WB-PUSH-16 | `wb start` shall push only after the tester, the build and the fix of an issue; after any other session it shall say the commands of WB-VIEW-14 and WB-VIEW-15, and run none of them. Issue: #151. | T | Draft |
| WB-PUSH-17 | A description file that is not a plain file, is over 1 MiB, is not UTF-8, or holds a control character but a line break or a tab, a line separator or a bidirectional control, shall open no change, and `wb start` shall end with 1, saying why. Issue: #151. | T | Draft |
| WB-PUSH-18 | `--description` given to `wb start` without `--prompt`, `--stamp` or `--parts` shall be refused as used wrongly, with 2. Issue: #151. | T | Draft |
| WB-PUSH-19 | `wb init` shall write a `draft_change` that reads the description from standard input: `--body-file -` for GitHub, `--description "$(cat)"` for GitLab. Issue: #151. | T | Draft |
| WB-PUSH-20 | `wb start <role> <issue> --take-back` shall push nothing, and say the commands of WB-VIEW-14 and WB-VIEW-15; `--no-push` with `--take-back` shall be used wrongly, with 2. Issue: #151. | T | Draft |
