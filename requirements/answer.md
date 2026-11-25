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
| WB-ANSWER-9 | Every role's prompt shall ask its questions in one comment per session whose first line is `[questions] <issue>.<role>` and the names of its questions, each question one line with the rest under `<details>`, and the line `changes the issue's file: yes` or `no`. Replaced by WB-ANSWER-43. Issue: #145. | T | Deprecated |
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
| WB-ANSWER-21 | When a session ends, `wb start` shall write its `[questions]` as a commit on the job's ref, after its stamp, with the questions in the form point 13 gives them. Issue: #238. | T | Draft |
| WB-ANSWER-22 | `wb answer` shall write the answers as a commit on the job's ref, signed by the person with a key that `.watchbill/signers` lists with the role `answer`, and push it. Issue: #238. | T | Draft |
| WB-ANSWER-23 | After it has written the commit, `wb answer` shall post the `[answers]` copy where a tracker is set up. Issue: #238. | T | Draft |
| WB-ANSWER-24 | An `[answers]` comment on the tracker alone shall answer no question. Issue: #238. | T | Draft |
| WB-ANSWER-25 | For an issue whose job has no questions commit on its ref, or that has no job ref, `wb` shall still read the `[questions]` and `[answers]` forms of 0.0.8 and 0.0.9 from the tracker. Issue: #238. | T | Draft |
| WB-ANSWER-26 | For an issue whose job has a questions commit on its ref, `wb` shall read no `[questions]` or `[answers]` from the tracker. Issue: #238. | T | Draft |
| WB-ANSWER-27 | Where `.watchbill/signers` is there, an answer commit that is unsigned or signed by a key without `answer` shall answer nothing, and `wb status` shall say so. Issue: #238. | T | Draft |
| WB-ANSWER-28 | Where `.watchbill/signers` is not there, an answer commit shall answer nothing, and `wb status` shall say once that nothing is signed, so nothing counts (#323). Issue: #238, #323. | T | Draft |
| WB-ANSWER-29 | `wb answer <issue> --show` shall print each open question of the issue in full, in the order asked, as `wb answer` lays it out when it asks, to standard output, ask nothing, post nothing and end with 0. Issue: #225. | T | Implemented |
| WB-ANSWER-30 | `wb answer <issue> --show` shall print only the questions that are open, none that an answer names. Issue: #225. | T | Implemented |
| WB-ANSWER-31 | `wb answer <issue> --show` shall say that no question is open, and end with 0, when none is. Issue: #225. | T | Implemented |
| WB-ANSWER-32 | `wb answer --show` shall end with 2 when it is given `--suggested`, or a question and its answer. Issue: #225. | T | Implemented |
| WB-ANSWER-33 | `wb answer --show --change <n>` shall print the open questions of a change with no issue as `--show` does for an issue. Issue: #225. | T | Implemented |
| WB-ANSWER-34 | `wb answer --show` shall work where `watchbill.yml` has no `issue_comment` (`change_comment` with `--change`), as it posts nothing. Issue: #225. | T | Implemented |
| WB-ANSWER-35 | `wb answer <issue> <question> <answer> <question> <answer> …` shall answer each question named without asking, and post one `[answers]` comment per role, in the order the questions are named. Issue: #181. | T | Implemented |
| WB-ANSWER-36 | `wb answer <issue> --suggested <question> <answer> …` shall answer the questions named with their answers and take the suggestion for every other open question. Issue: #181. | T | Implemented |
| WB-ANSWER-37 | `wb answer` given several pairs shall end with 1 and post nothing when any pair names a question that is not open or gives a number that is not a choice of it. Issue: #181. | T | Implemented |
| WB-ANSWER-38 | `wb answer` shall end with 2 and post nothing when one question is named twice, by the same word or by two words for it (`q1` and `planner.q1`). Issue: #181. | T | Implemented |
| WB-ANSWER-39 | `wb answer` shall write the role of the choice after each question in the first line it posts, as `[answers] 54.build q3>tester q4>person`, whether the answers are typed one by one, given as pairs, or taken with `--suggested`. Issue: #226. | T | Implemented |
| WB-ANSWER-40 | `wb answer` shall, for an answer in words or a choice with no role, ask which role acts and take the asking role on Enter, and shall take the asking role without asking for such an answer given as a pair or taken with `--suggested`. Issue: #226. | T | Implemented |
| WB-ANSWER-41 | `wb answer` shall say after posting that the issue's file is to be changed and accepted again only for an answer whose role is `person`, where the choices of the question name roles. Narrows WB-ANSWER-8 to the questions whose choices name no role. Issue: #226. | T | Implemented |
| WB-ANSWER-42 | `wb answer` shall answer a question whose choices name no role (0.10.0 and before) with no role written and none asked, and say that the issue's file is to be changed where the question's line says `changes the issue's file: yes` (WB-ANSWER-8). Issue: #226. | T | Implemented |
| WB-ANSWER-43 | Every role's prompt shall ask its questions in one comment per session whose first line is `[questions] <issue>.<role>` and the names of its questions, each question one line with the rest under `<details>`, and no line `changes the issue's file`; that the program posts the questions in that comment, after the session (#295, #153). Replaces WB-ANSWER-9. Issue: #226, #295, #153. | T | Implemented |
| WB-ANSWER-44 | `wb answer` shall count a question as answered by an `[answers]` first line whose words name a role, as `q3>tester`, and show no question it closes. Issue: #226. | T | Implemented |
| WB-ANSWER-45 | `wb answer` shall count as the role of a choice only a word of the roles of the revision (`planner`, `tester`, `build`, `review`, `fix`) or `person`; any other word in brackets is a choice with no role, and the person is asked which role acts. Issue: #226. | T | Implemented |
| WB-ANSWER-46 | `wb answer <issue> --suggested` shall take the suggestion of each question that `wb start planner` posted from the issue's file. Issue: #295. | T | Implemented |
