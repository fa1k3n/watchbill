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
