# Answering the questions

What `wb answer` shows and posts, and the form the questions and answers take on the issue.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-ANSWER-1 | `wb answer <issue>` shall show each open question of the issue in full, one at a time, read from `issue_comment_bodies`. Issue: #145. | T | Implemented |
| WB-ANSWER-2 | `wb answer` shall take Enter as the suggested answer, a number that is one of the question's choices as that choice, and any other text as the answer in words. Issue: #145. | T | Implemented |
| WB-ANSWER-3 | `wb answer` shall ask again when a number is not one of the question's choices. Issue: #145. | T | Implemented |
| WB-ANSWER-4 | `wb answer` shall say what it will post, then post one comment per role whose first line is `[answers] <issue>.<role>` and the names of the questions it answers, and one line `q<n>: <answer>` for each. Issue: #145. | T | Implemented |
| WB-ANSWER-5 | `wb answer` shall post nothing until every question shown is answered, and nothing when it is stopped with Ctrl-C. Issue: #145. | T | Implemented |
| WB-ANSWER-6 | `wb answer <issue> --suggested` shall take every suggested answer, and name the questions that have none. Issue: #145. | T | Implemented |
| WB-ANSWER-7 | `wb answer <issue> <role>.q<n> <answer>` shall post that one answer without asking. Issue: #145. | T | Implemented |
| WB-ANSWER-8 | After posting an answer to a question marked `changes the issue's file: yes`, `wb answer` shall say that the issue's file is to be changed and accepted again before the role is started again. Issue: #145. | T | Implemented |
| WB-ANSWER-9 | Every role's prompt shall ask its questions in one comment per session whose first line is `[questions] <issue>.<role>` and the names of its questions, each question one line with the rest under `<details>`, and the line `changes the issue's file: yes` or `no`. Issue: #145. | T | Implemented |
| WB-ANSWER-10 | Every role's prompt shall number its questions within the role on the issue, from the highest already there. Issue: #145. | T | Implemented |
| WB-ANSWER-11 | `wb status` shall count a question of a `[questions]` comment as open until an `[answers]` comment names it, and shall still read the `[question]` and `[answer]` lines of revision 0.0.8. Issue: #145. | T | Implemented |
| WB-ANSWER-12 | `wb init` shall write the pattern `issue_comment_bodies` for a GitHub and a GitLab `origin`, and none for a server other than those. Issue: #145. | T | Implemented |
| WB-ANSWER-13 | `wb answer --change <n>` shall read the questions of a change with no issue from `change_comment_bodies`, and post the answers on the change through `change_comment`. Issue: #145. | T | Implemented |
| WB-ANSWER-14 | `wb init` shall write the pattern `change_comment_bodies` for a GitHub and a GitLab `origin`, and none for a server other than those. Issue: #145. | T | Implemented |
| WB-ANSWER-15 | `wb answer` shall show and answer a question of revision 0.0.8, posted as its own comment whose first line is `[question] <name>`, as it does a question of a `[questions]` comment. Issue: #145. | T | Implemented |
| WB-ANSWER-16 | `wb status` shall count no `[questions]` and no `[answers]` comment from a reverted stamp up to its `[revert]` marker. Issue: #145. | T | Implemented |
| WB-ANSWER-17 | `wb answer` shall show no question, and take no answer, from a reverted stamp up to its `[revert]` marker. Issue: #145. | T | Implemented |
| WB-ANSWER-18 | `wb answer` shall end with 1 and post nothing where `watchbill.yml` has no `issue_comment_bodies` or no `issue_comment` (`change_comment_bodies` or `change_comment` with `--change`), and name the line. Issue: #145. | T | Implemented |
| WB-ANSWER-19 | `wb answer` shall end with 2 when it is used wrongly: no issue and no `--change`, an issue and `--change` together, `--suggested` with a question named, a question named with no answer, an issue that is not a name, or a word it does not take. Issue: #145. | T | Implemented |
| WB-ANSWER-20 | `wb answer` shall end with 0, post nothing and say so when the issue has no open question. Issue: #145. | T | Implemented |
