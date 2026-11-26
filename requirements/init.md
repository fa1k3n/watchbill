# `wb init`

What `wb init` does today: it sets Watchbill up in a git repository. What it writes of the revision: [pin.md](pin.md).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-INIT-1 | `wb init` shall propose the name of the folder of the repository, made safe as a name, as the project. Issue: #2. | T | Implemented |
| WB-INIT-2 | `wb init` shall propose the kind of git server that the host and port of `origin` show as the git server. Issue: #2. | T | Implemented |
| WB-INIT-3 | `wb init` shall propose the path of the address of `origin` as the repository, when the address is `scheme://host/path` or `host:path`, with or without a user name before the host. Issue: #2. | T | Implemented |
| WB-INIT-4 | `wb init` shall propose the git server as the tracker when it is github or gitlab, and none otherwise. Issue: #2. | T | Implemented |
| WB-INIT-5 | `wb init` shall propose the first of the folders `requirements`, `docs/requirements` and `doc/requirements`, in that order, that holds a file, with the file ending most of its files have, as the requirements. Issue: #2. | T | Implemented |
| WB-INIT-6 | `wb init` shall propose the CI driver that the file or folder in the repository shows as the CI driver. Issue: #2. | T | Implemented |
| WB-INIT-7 | `wb init` shall propose none for each answer that the repository does not show. Issue: #2. | T | Implemented |
| WB-INIT-8 | `wb init` shall ask nothing of the network to find what it proposes. Issue: #2. | I | Implemented |
| WB-INIT-9 | `wb init` shall ask one line per answer. Issue: #2. | T | Implemented |
| WB-INIT-10 | `wb init` shall not ask for the repository when the git server is none. Issue: #2. | T | Implemented |
| WB-INIT-11 | `wb init` shall keep the proposal for each answer not yet given when its input ends. Issue: #2. | T | Implemented |
| WB-INIT-12 | `wb init` shall say that an answer is not taken and ask again when it is not a usable project name. Issue: #2. | T | Implemented |
| WB-INIT-13 | `wb init` shall say that an answer is not taken and ask again when it is over 200 characters or holds a control character. Issue: #2. | T | Implemented |
| WB-INIT-14 | `wb init --yes`, also written `-y`, shall ask nothing. Issue: #2. | T | Implemented |
| WB-INIT-15 | `wb init` shall show the file it is about to write before it asks whether that is OK. Issue: #2. Changed by #7: it names every file it is about to write, as the row WB-PIN-5 says. | T | Implemented |
| WB-INIT-16 | `wb init` shall write nothing and end with 1 when the answer to whether it is OK is not an empty line, `y` or `yes`. Issue: #2. | T | Implemented |
| WB-INIT-17 | `wb init` shall write `.watchbill/watchbill.yml` at the root of the repository, also when it is run from a folder inside it. Issue: #2. Changed by #7: the files of the revision are written beside it. | T | Implemented |
| WB-INIT-18 | `wb init` shall write the version of the package as the line `watchbill:` of the file. Issue: #2. Changed by #7: the line `revision:` follows it (WB-PIN-3). | T | Implemented |
| WB-INIT-19 | `wb init` shall leave a file that is already there as it is, say so and end with 1. Issue: #2. Changed by #7: when any file it would write is there, it writes nothing at all (WB-PIN-4). | T | Implemented |
| WB-INIT-20 | `wb init` shall write nothing, say so on standard error and end with 1 when it is run outside a git repository. Issue: #2. | T | Implemented |
| WB-INIT-21 | `wb init` shall keep no user name, password or token of the address of `origin`, in the file or on the screen. Issue: #2. | T | Implemented |
| WB-INIT-22 | `wb init` shall write a value plain only when YAML is sure to read it back as the same text, and in single quotes otherwise. Issue: #2. | T | Implemented |
| WB-INIT-23 | `wb init` shall say which file it wrote. Issue: #2. Changed by #7: it says each file it wrote. | T | Implemented |
| WB-INIT-24 | `wb init` shall end with 0 when it has written the file. Issue: #2. | T | Implemented |
| WB-INIT-25 | `wb` shall print the usage and end with 2 when it is given no command, a command it does not know, or an option `wb init` does not know. Issue: #2. | T | Implemented |
| WB-INIT-26 | `wb --version` shall print the version of the package and end with 0. Issue: #2. | T | Implemented |
| WB-INIT-27 | `wb --help` shall print the usage and end with 0. Issue: #2. | T | Implemented |
| WB-INIT-28 | `wb init` shall keep the proposal for an answer when the line is empty. Issue: #2. | T | Implemented |
| WB-INIT-29 | `wb init` shall write none as the repository when the git server is none. Issue: #2. | T | Implemented |
| WB-INIT-30 | `wb init --yes` shall write what it proposes. Issue: #2. | T | Implemented |
| WB-INIT-31 | `wb init` shall write nothing and end with 1 when its input ends at the question whether it is OK. Issue: #2. | T | Implemented |
| WB-INIT-32 | `wb init` shall end with 1 when it is interrupted. Issue: #2. | T | Implemented |
| WB-INIT-33 | `wb init` shall print `Nothing written.` and no traceback when it is interrupted, also while it asks git. Issue: #2. | T | Implemented |
| WB-INIT-34 | `wb init --yes` shall print the file before it writes it. Issue: #2. | T | Implemented |
| WB-INIT-35 | `wb help` and `wb -h` shall do as `wb --help`. Issue: #2. | T | Implemented |
| WB-INIT-36 | `wb -v` shall do as `wb --version`. Issue: #2. | T | Implemented |
| WB-INIT-37 | `wb init` shall take any usable text for the git server, the tracker and the CI driver, not only the names its question lists. Issue: #2. | T | Implemented |
| WB-INIT-38 | `wb init` shall refuse an answer that is not UTF-8 in words, without a panic. Issue: #22. | T | Implemented |
| WB-INIT-39 | `wb` shall take `--` as the end of the options, in every command: a word after it is never an option, `wb -- init` does as `wb init`, `wb init --` as `wb init`, and `wb --` as `wb`. Issue: #34. | T | Implemented |
| WB-INIT-40 | `wb init` shall take short options written together, so that `-yy` does as `-y`. Issue: #34. | T | Implemented |
| WB-INIT-41 | `wb` shall refuse an option given a value it does not take, such as `--yes=x`, `-y=x` or `--version=1`, as an option it does not know, and name it with the value; an option it does not know is named with its value too, `--force=1` and `-x=1`. Issue: #34. | T | Implemented |
| WB-INIT-42 | `wb init` and `wb revision` shall name an option they do not know alone, also when it is written with others (`-yx` names `-x`), and write a byte of its name that is not UTF-8 as `�`. Issue: #34. | T | Implemented |
| WB-INIT-43 | `wb init` shall write `issue_address` and `change_address` patterns for a GitHub `origin`, from the repository it found, when that is plain text, read through the loader. Issue: #72. | T | Implemented |
| WB-INIT-44 | `wb init` shall write no address pattern for a server other than GitHub and GitLab, for none, or for a repository that is not plain text. Issue: #72. | T | Implemented |
| WB-INIT-45 | `wb init` shall write the pattern `branch: 'issue-{issue}'` for every server. Issue: #72. | T | Implemented |
| WB-INIT-46 | The `issue_address` and `change_address` patterns `wb init` writes for a GitHub `origin` shall each run two commands: one that shows the issue's text or the change's description, and one that shows the comments. Issue: #96. | T | Implemented |
| WB-INIT-47 | `wb init` shall write the pattern `issue_comment: 'gh issue comment {issue} --repo <repo> --body-file -'` for a GitHub `origin`, and none for a server other than GitHub and GitLab. Issue: #84. | T | Implemented |
| WB-INIT-48 | `wb init` shall write the patterns `issue_comments` and `change_of_branch` for a GitHub `origin`, and none for a server other than GitHub and GitLab. Issue: #84. | T | Implemented |
| WB-INIT-49 | For a GitLab `origin` whose repository is plain, `wb init` shall write `issue_address`, `change_address`, `issue_comment`, `issue_comments`, `change_comment`, `change_comments` and `change_of_branch` as `glab` commands with the repository filled in, `issue_comment` and `change_comment` through `glab api --method POST` and `-F body=@-`, and none of them for a repository that is not plain. Issue: #123. | T | Implemented |
| WB-INIT-50 | For a GitLab `origin`, the `issue_comments` that `wb init` writes shall print the first line of each comment and of no system note. Issue: #123. | D | Draft |
| WB-INIT-51 | `wb init` shall propose `build_files` with the CI file it found, and write no `build_files` when it found none. Issue: #125. | T | Implemented |
| WB-INIT-52 | `wb init` shall write `change_checks` for a GitHub and a GitLab `origin`, and none for another server. Issue: #127. | T | Implemented |
| WB-INIT-53 | `wb init` shall write the patterns `change_comment` and `change_comments` for a GitHub `origin`, and none for a server other than GitHub and GitLab. Issue: #84, #116. | T | Implemented |
| WB-INIT-54 | `wb init` shall ask for `classed` after the CI driver, propose `[]`, and write the answer as the line `classed:` of `watchbill.yml`, `classed: []` with `--yes`; a path that starts with `/`, holds `..` or is the root shall not be taken, and the question shall be asked again. Issue: #84. | T | Implemented |
| WB-INIT-55 | The `glab` commands that `wb init` writes for a GitLab `origin` shall name the host of `origin`, as `--repo https://<host>/<repo>` and `--hostname <host>`, without a user name or a token, and none of them shall be written for an `origin` with a port. Issue: #123. | T | Implemented |
| WB-INIT-56 | `wb init` shall write the pattern `open_issues` for a GitHub and a GitLab `origin`, and none for a server other than those. Replaced by WB-INIT-59. Issue: #139, #155. | T | Deprecated |
| WB-INIT-57 | The pattern `open_issues` that `wb init` writes shall print a line break in a title as a space, so that one issue is one line. `wb init` no longer writes `open_issues` (#155), WB-INIT-59. Issue: #139, #155. | T | Deprecated |
| WB-INIT-58 | `wb init` shall write the pattern `draft_change` for a GitHub and a GitLab `origin`, reading the description from standard input (WB-PUSH-19), and none for a server other than those. Issue: #145, #151. | T | Implemented |
| WB-INIT-59 | `wb init` shall write no pattern `open_issues`. Issue: #155. | T | Implemented |
| WB-INIT-60 | `wb init` shall write the pattern `issue_text`, which prints the issue's text alone, for a GitHub `origin` (`gh issue view {issue} --repo <repository> --json body --jq .body`) and a GitLab `origin` (`glab issue view {issue} --repo <address> -F json \| jq -r .description`), and none for another server. Issue: #295. | T | Implemented |
| WB-INIT-61 | `wb init` shall write `.watchbill/project/all.md` with the rules of the project's own style files as `HEAD` has them (`.editorconfig`, `.clang-format`, `_clang-format`, `.clang-tidy`, `rustfmt.toml`, `.rustfmt.toml`, `.pylintrc`, `pylintrc`, `.flake8`, `uncrustify.cfg`, `.astylerc`, a file whose name holds `misra`, `CODING_STYLE.md`, `CONTRIBUTING.md`), each under its path, and show it among what it will write before it writes. Issue: #289. | T | Implemented |
| WB-INIT-62 | Of `.clang-tidy`, `wb init` shall copy only its naming rules: the options of `readability-identifier-naming`, each as `<option>: <value>`. Issue: #289. | T | Implemented |
| WB-INIT-63 | `wb init` shall write into `all.md`, for each kind of code file, the facts of a bounded sample of the files of `HEAD` (at most 200 files of 256 KiB or less): the files indented with spaces and with tabs, the indent in spaces, the line endings, and the longest line with its path, each with the number of files it was seen in. Issue: #289. | T | Implemented |
| WB-INIT-64 | `wb init` shall leave out of `all.md` a line of a style file that a prompt would read as holding a placeholder, or that holds a control character, and say how many it left out; a style file over 64 KiB, or not UTF-8, is named and not copied, and at most 200 lines of one file are copied. Issue: #289. | T | Implemented |
| WB-INIT-65 | `wb init` shall read the style files and the sample from `HEAD` alone: a file not in `HEAD` is not read. Where `HEAD` gives nothing, no `all.md` is written. An `all.md` that is there already is in the way, and nothing is written. Issue: #289. | T | Implemented |
| WB-INIT-66 | Where no runner of the organisation's file has `local: true`, or there is no organisation's file, `wb init` shall say so on standard error, with the file that sets it and what to put there, and end with 0 having written the rest; with a runner `local: true` it says nothing of it. Issue: #289. | T | Implemented |
| WB-INIT-67 | `wb init --survey` shall refuse with 1, in the same words as `wb init` warns, where no runner of the organisation's file has `local: true` or there is no organisation's file, and run nothing. Issue: #289. | T | Implemented |
| WB-INIT-68 | `wb init --survey` shall run on the runner `--runner` names, or else the `default` of the organisation's file, and refuse with 1, naming it, a runner that is not there, is not `local: true`, or has no `start`. Issue: #289. | T | Implemented |
| WB-INIT-69 | `wb init --survey` shall refuse with 1 a local runner whose `reach` holds an endpoint that is not on this machine, naming the endpoint. Issue: #289. | T | Implemented |
| WB-INIT-70 | `wb init --survey` shall refuse with 1, saying why, where bubblewrap cannot make a sandbox with a network of its own (#195). Issue: #289. | T | Implemented |
| WB-INIT-71 | `wb init --survey` shall run the session in a view of the whole tree of `HEAD`, in bubblewrap, read-only but `.watchbill/project/`, and take back only `.watchbill/project/style.md`, as a plain UTF-8 file of 1 MiB or less, into the working tree, committed by nobody, with each line that a prompt would read as a placeholder, or that holds a control character, left out and how many said; the view is removed after. A `.watchbill/project` of `HEAD` that is a link is refused with 1 before the session. Issue: #289. | T | Implemented |
| WB-INIT-72 | `wb init --survey` shall refuse with 1, before anything runs, where `.watchbill/project/style.md` is in the working tree or in `HEAD`: it never writes over it. Issue: #289. | T | Implemented |
| WB-INIT-73 | `wb init --survey` shall make nothing in the working tree before the session, and write nothing and end with 1 where the session does not end with 0, or leaves no `.watchbill/project/style.md` that is a plain UTF-8 file. Issue: #289. | T | Implemented |
| WB-INIT-74 | `wb init --survey` shall end with 2 and the usage where it is given with `--yes`, and where `--runner` is given without `--survey`; and with 2 where the repository is not set up. Issue: #289. | T | Implemented |
| WB-INIT-75 | `wb init --survey` shall fill `{model}` and `{effort}` of the runner's `start` from its `planner`, and refuse with 1 a `start` that holds either where the runner has no `planner`. Issue: #289. | T | Implemented |
