# The view

What a session of the tester, the build, the review or the fix sees when `wb start <role>` starts it for an issue with an accepted file, and what comes back from it.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-VIEW-1 | `wb start <role>` shall make the view a new git repository holding only the paths of `changes` and of the role's `reads` as they are in `HEAD`, in one commit, with no remote and no other object of the repository. Issue: #126. | T | Implemented |
| WB-VIEW-2 | `wb start <role>` shall run the runner's `start` under `bwrap`, with the view writable at the path of the checkout, the system read-only, and no other folder of the person's home but the runner's `binds`. Issue: #126. | T | Implemented |
| WB-VIEW-3 | A path of the checkout that is not listed shall not be readable from inside the view, neither as a file nor from the history. Issue: #126. | T | Implemented |
| WB-VIEW-4 | The checkout's `.git`, its `.watchbill/` and the organisation's tokens shall not be inside the view. Issue: #126. | T | Implemented |
| WB-VIEW-5 | When the session ends, `wb start <role>` shall refuse with 1 a commit made in the view that touches a path outside `changes`, under `.watchbill/` or under `build_files`, name the paths, and put neither it nor any later commit on the branch. Issue: #126. | T | Implemented |
| WB-VIEW-6 | When the session ends, `wb start <role>` shall put the commits made in the view that keep to `changes` on the issue's branch in the checkout, in order, with their messages and authors, and push nothing. Issue: #126. | T | Implemented |
| WB-VIEW-7 | `wb start <role>` shall name work left uncommitted in the view, and not take it. Issue: #126. | T | Implemented |
| WB-VIEW-8 | `wb start <role>` shall remove the view when all was taken back, and keep and name it when anything was refused. Issue: #126. | T | Implemented |
| WB-VIEW-9 | The stamp shall have the line `view:`, with the SHA-256 of the list of paths, or `none` for the breakdown and for a role started without an issue's file. Issue: #126. | T | Implemented |
| WB-VIEW-10 | A role started without an issue's file shall run without a view, and `wb start <role>` shall say so on standard error before it starts. Issue: #126. | T | Implemented |
| WB-VIEW-11 | With no `bwrap`, or with user namespaces off, `wb start <role>` shall refuse with 1 and say why, before anything is posted. Issue: #126. | T | Implemented |
| WB-VIEW-12 | The package shall require `bubblewrap`. Issue: #126. | T | Implemented |
| WB-VIEW-13 | The README shall say what still leaks from a view: the listed files, one bit for each pass or fail of a test, the runner's own settings, and what a person pastes elsewhere. Issue: #126. | I | Implemented |
| WB-VIEW-14 | After the commits of a session in a view are taken onto the branch, `wb start <role>` shall say on standard error the command that pushes the branch, `git push origin <branch>`, and shall not run it. Issue: #145. | T | Implemented |
| WB-VIEW-15 | After a session of the build whose commits were taken onto the branch, `wb start build` shall also say on standard error the command that opens the change as a draft, the pattern `draft_change` of `watchbill.yml` with `{branch}` filled, and shall not run it; without the pattern, it shall say that the line is not there. Issue: #145. | T | Implemented |
| WB-VIEW-16 | After a session of the tester, the build or the fix without a view, where the branch checked out has new commits, `wb start <role>` shall say the same commands as after a view. Issue: #145. | T | Implemented |
| WB-VIEW-17 | `wb` shall not count as work left in a view a path that the ignore rules of the checkout ignore: every `.gitignore` of `HEAD` and the checkout's `.git/info/exclude`. Issue: #159. | T | Implemented |
| WB-VIEW-18 | `wb` shall read those ignore rules from the checkout, and never from the view. Issue: #159. | T | Implemented |
| WB-VIEW-19 | Where more than 20 paths are left in a view, `wb` shall say how many, the first 20, and that the whole list is in the file `back` of the session's folder. Issue: #159. | T | Implemented |
| WB-VIEW-20 | After the commits of a view are taken back, `wb` shall say on standard error `git show --stat <commit>` for each commit taken. Issue: #159. | T | Implemented |
| WB-VIEW-21 | `wb` shall say on standard error, and write into `back`, each path from a view with its control characters escaped. Issue: #167. | T | Implemented |
| WB-VIEW-22 | `back` shall hold one line for each path from a view, whatever its name holds. Issue: #167. | T | Implemented |
| WB-VIEW-23 | `wb view <issue> <role>` shall print the paths of that role's view, one to a line, the same list its start makes the view from. Issue: #173. | T | Implemented |
| WB-VIEW-24 | `wb view <issue> <role>` shall print the level and the line `view: <hash>` that the role's stamp would hold. Issue: #173. | T | Implemented |
| WB-VIEW-25 | `wb view <issue> <role> --out` shall print the folders of `HEAD` that the view leaves out, each with a count of its paths. Issue: #173. | T | Implemented |
| WB-VIEW-26 | `wb view` with both `--in` and `--out` shall end with 2. Issue: #173. | T | Implemented |
| WB-VIEW-27 | `wb view <issue> <path>` shall say, for each role, whether the path is in its view, and when in, the list and the line of the issue's file that holds it. Issue: #173. | T | Implemented |
| WB-VIEW-28 | `wb view <issue> <path>` shall say, for a role whose view leaves the path out, why: not listed, under `.watchbill/`, under `build_files`, ignored by the checkout, or not in `HEAD`. Issue: #173. | T | Implemented |
| WB-VIEW-29 | `wb view <issue> <path>` shall end with 0 when the path is in the view of at least one role, and 1 when in none. Issue: #173. | T | Implemented |
| WB-VIEW-30 | `wb view <issue> <role>` shall print the local changes that the guard of the start would name. Issue: #173. | T | Implemented |
| WB-VIEW-31 | `wb view` shall make no view, post nothing, write nothing, and ask nothing of the network. Issue: #173. | T | Implemented |
| WB-VIEW-32 | `wb view <issue> <role>` for an issue with no accepted, committed file shall end with 1, saying the role would run without a view. Issue: #173. | T | Implemented |
| WB-VIEW-33 | An entry of `changes` that is a file in `HEAD`, also when written with a `/`, or that is not in `HEAD` and does not end with `/`, shall cover that path only, and never a path under it; a folder shall cover the paths under it. Issue: #192. | T | Implemented |
| WB-VIEW-34 | `wb` shall refuse with 1, naming the path, a commit of a view that holds a path with a part `.git` in any case, or a path git refuses to check out. Issue: #192. | T | Implemented |
| WB-VIEW-35 | Before it puts a commit of a view on the branch, `wb` shall refuse, naming the commit, one with a tree not in the form git writes (a name twice, a mode git rewrites, entries not in git's order), and shall compare what the commit it made changes with what the view's commit changed, path for path, mode and file, and where they differ refuse that commit and the ones after it, name the commit and end with 1. Issue: #192. | T | Implemented |
| WB-VIEW-36 | `wb` shall write a note with `git notes --ref=refs/notes/watchbill` on each commit it takes back from a view, holding `stamp: <hash of the stamp commit>` and the role. Issue: #237. | T | Draft |
| WB-VIEW-37 | `wb` shall write no note for a session that has no commit to take back. Issue: #237. | T | Draft |
| WB-VIEW-38 | `wb init` and `wb upgrade` shall add the fetch line `+refs/notes/watchbill:refs/notes/watchbill` to the fetch lines of `origin`. Issue: #237. | T | Draft |
| WB-VIEW-39 | The commits of a session, those whose note names its stamp, shall be found from a clone with no tracker set up. Issue: #237. | T | Draft |
