# The words a prompt is filled in with

A role prompt is filled in with these words and no others. Each stands in the prompt in braces, and whoever starts the session gives its value.

- `{task}`: the task, as the tracker names it.
- `{change}`: the change under review or being fixed, as the git server names it.
- `{requirements}`: the IDs of the requirements the task names.
- `{round}`: the review round, 1 or 2.

A value is put in as it is given. It is never itself read for placeholders.
