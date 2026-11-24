# The words a prompt is filled in with

A role prompt is filled in with these words and no others. Each stands in the prompt in braces, and whoever starts the session gives its value.

- `{issue}`: the issue, as the tracker names it. For a review or a fix of a change made with no issue, `none`.
- `{change}`: the change under review or being fixed, as the git server names it.
- `{requirements}`: the IDs of the requirements the issue names.
- `{round}`: the review round, 1 or 2.
- `{checkout}`: the folder of the checkout the session works in.
- `{branch}`: the branch the session works on.
- `{issue_address}`: where the issue is read: its address at the tracker, or the command that shows it. `none` where `{issue}` is.
- `{change_address}`: where the change is read and posted on: its address at the git server, or the command that shows it.
- `{classed}`: the paths of the project's `classed` list, one to a line, or `none` where the list is empty. Where the project has no `classed` list, it says that every path is classed. Where it stands after spaces only, the lines after the first are indented to the line it stands in.
- `{classed_touched}`: the paths of the change under a classed path, worked out by the program from the diff of the change's branch against its merge base with the default branch (the local branch when there is one, else the branch at `origin`; the process's own files `.watchbill/issues/<issue>.yml` and `.accepted` left out), one to a line, or `none`. Where it stands after spaces only, the lines after the first are indented to the line it stands in. Where the diff cannot be had, it says `cannot be told` and why.
- `{description}`: the file the session writes the change's description to, `/run/watchbill/description`. It is in the sandbox and outside the checkout, so it is never committed. The program reads it after the session, and opens the change with it.
- `{brought}`: the folder of what the program read from the tracker and the git server for the session, `/run/watchbill/in`, read-only: `issue.text`, the issue's text; `questions`, the role's own questions and the answers to them; and for the review and the fix `change.text`, the change's description, `change.comments`, the comments of the review and the fix on it, and `change.diff`, its diff against the base, of the files the issue names. It is in the sandbox and outside the checkout. The session holds no login, and needs none.
- `{questions}`: the file the session writes its questions to, in full, `/run/watchbill/questions`. The program posts the new ones after the session.
- `{change_comment}`: the file the review writes its findings to, and the fix its answers, `/run/watchbill/change_comment`. The program posts it on the change after the session.
- `{needs_person}`: the file the session writes to what only a person can settle, `/run/watchbill/needs_person`. The program posts it on the issue after the session.

A value is put in as it is given, but for the lines of `{classed}` and `{classed_touched}`. It is never itself read for placeholders.
