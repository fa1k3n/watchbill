# The push after a session

What `wb` does with the issue's branch and the change once the commits of a session are taken back. The session holds no login to the code host.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-PUSH-1 | After the commits of a session are taken back and nothing was refused, `wb` shall push the issue's branch to `origin` through the push pattern of `watchbill.yml`. Issue: #151. | T | Draft |
| WB-PUSH-2 | `wb` shall push only the issue's branch, and never with force. Issue: #151. | T | Draft |
| WB-PUSH-3 | `wb` shall say on standard error what it will push, and through which pattern, before it pushes. Issue: #151. | T | Draft |
| WB-PUSH-4 | Where a commit was refused at the taking back, or work was left uncommitted, `wb` shall push nothing and say so. Issue: #151. | T | Draft |
| WB-PUSH-5 | A push that ends with anything but 0 shall end `wb` with 1, saying why, with no change opened. Issue: #151. | T | Draft |
| WB-PUSH-6 | After a build, when no change is open for the branch, `wb` shall open one as a draft through the pattern, with the description on standard input. Issue: #151. | T | Draft |
| WB-PUSH-7 | After the tester, after the fix, and after a build when a change is open for the branch, `wb` shall push and open no change. Issue: #151. | T | Draft |
| WB-PUSH-8 | `wb` shall fill `{description}` with a file that lies inside the sandbox and outside the view's tree. Issue: #151. | T | Draft |
| WB-PUSH-9 | Where the description file is missing or empty, the change shall be opened with a description that names the issue and says that the session wrote none. Issue: #151. | T | Draft |
| WB-PUSH-10 | Without the patterns, or with `--no-push`, `wb` shall print the push command and the command that opens the change, and run neither. Issue: #151. | T | Draft |
| WB-PUSH-11 | The session's folder shall hold `pushed:` with the commit pushed, and `change:` with the change's number, or why nothing was pushed. Issue: #151. | T | Draft |
| WB-PUSH-12 | No login of the code host shall be inside the sandbox of a session. Issue: #151. | T | Draft |
