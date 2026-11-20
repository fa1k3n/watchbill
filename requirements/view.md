# The view

What a session of the tester, the build, the review or the fix sees when `wb <role>` starts it for an issue with an accepted file, and what comes back from it.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-VIEW-1 | `wb <role>` shall make the view a new git repository holding only the paths of `changes` and of the role's `reads` as they are in `HEAD`, in one commit, with no remote and no other object of the repository. Issue: #126. | T | Implemented |
| WB-VIEW-2 | `wb <role>` shall run the runner's `start` under `bwrap`, with the view writable at the path of the checkout, the system read-only, and no other folder of the person's home but the runner's `binds`. Issue: #126. | T | Implemented |
| WB-VIEW-3 | A path of the checkout that is not listed shall not be readable from inside the view, neither as a file nor from the history. Issue: #126. | T | Implemented |
| WB-VIEW-4 | The checkout's `.git`, its `.watchbill/` and the organisation's tokens shall not be inside the view. Issue: #126. | T | Implemented |
| WB-VIEW-5 | When the session ends, `wb <role>` shall refuse with 1 a commit made in the view that touches a path outside `changes`, under `.watchbill/` or under `build_files`, name the paths, and put neither it nor any later commit on the branch. Issue: #126. | T | Implemented |
| WB-VIEW-6 | When the session ends, `wb <role>` shall put the commits made in the view that keep to `changes` on the issue's branch in the checkout, in order, with their messages and authors, and push nothing. Issue: #126. | T | Implemented |
| WB-VIEW-7 | `wb <role>` shall name work left uncommitted in the view, and not take it. Issue: #126. | T | Implemented |
| WB-VIEW-8 | `wb <role>` shall remove the view when all was taken back, and keep and name it when anything was refused. Issue: #126. | T | Implemented |
| WB-VIEW-9 | The stamp shall have the line `view:`, with the SHA-256 of the list of paths, or `none` for the breakdown and for a role started without an issue's file. Issue: #126. | T | Implemented |
| WB-VIEW-10 | A role started without an issue's file shall run without a view, and `wb <role>` shall say so on standard error before it starts. Issue: #126. | T | Implemented |
| WB-VIEW-11 | With no `bwrap`, or with user namespaces off, `wb <role>` shall refuse with 1 and say why, before anything is posted. Issue: #126. | T | Implemented |
| WB-VIEW-12 | The package shall require `bubblewrap`. Issue: #126. | T | Implemented |
| WB-VIEW-13 | The README shall say what still leaks from a view: the listed files, one bit for each pass or fail of a test, the runner's own settings, and what a person pastes elsewhere. Issue: #126. | I | Implemented |
| WB-VIEW-14 | After the commits of a session in a view are taken onto the branch, `wb <role>` shall say on standard error the command that pushes the branch, `git push origin <branch>`, and shall not run it. Issue: #145. | T | Implemented |
| WB-VIEW-15 | After a session of the build whose commits were taken onto the branch, `wb build` shall also say on standard error the command that opens the change as a draft, the pattern `draft_change` of `watchbill.yml` with `{branch}` filled, and shall not run it; without the pattern, it shall say that the line is not there. Issue: #145. | T | Implemented |
| WB-VIEW-16 | After a session of the tester, the build or the fix without a view, where the branch checked out has new commits, `wb <role>` shall say the same commands as after a view. Issue: #145. | T | Implemented |
| WB-VIEW-17 | `wb` shall not count as work left in a view a path that the ignore rules of the checkout ignore: every `.gitignore` of `HEAD` and the checkout's `.git/info/exclude`. Issue: #159. | T | Draft |
| WB-VIEW-18 | `wb` shall read those ignore rules from the checkout, and never from the view. Issue: #159. | T | Draft |
| WB-VIEW-19 | Where more than 20 paths are left in a view, `wb` shall say how many, the first 20, and that the whole list is in the file `back` of the session's folder. Issue: #159. | T | Draft |
| WB-VIEW-20 | After the commits of a view are taken back, `wb` shall say on standard error `git show --stat <commit>` for each commit taken. Issue: #159. | T | Draft |
| WB-VIEW-21 | `wb` shall say on standard error, and write into `back`, each path from a view with its control characters escaped. Issue: #167. | T | Draft |
| WB-VIEW-22 | `back` shall hold one line for each path from a view, whatever its name holds. Issue: #167. | T | Draft |
