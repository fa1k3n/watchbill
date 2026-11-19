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

A value is put in as it is given. It is never itself read for placeholders.
