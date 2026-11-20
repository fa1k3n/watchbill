# The rule table and the model table

The two tables of the first revision and the one loader that reads YAML. What a revision is: [docs/revision.md](../docs/revision.md).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-RULE-1 | The rule table `revision/rules.yml` shall have a row for each of the roles tester, build, review, fix and officer. Replaced by WB-RULE-39. Issue: #5. | T | Deprecated |
| WB-RULE-2 | A role's row shall give the step it performs and what it produces, what it may read, what it may do, the roles the same team member may not also hold on one change, and its prompt file or `none`. Issue: #5. | T | Implemented |
| WB-RULE-3 | The rule table shall give the greatest number of review rounds, and the first table shall give two. Issue: #5. | T | Implemented |
| WB-RULE-4 | The rule table shall list what always goes to a person. Issue: #5. | T | Implemented |
| WB-RULE-5 | The rule table shall say that a change that touches classed code needs a person as its verifier. Issue: #5. | T | Implemented |
| WB-RULE-6 | The officer's row shall say that the officer holds no other role on an issue it has handed out and never merges. Issue: #5. | T | Implemented |
| WB-RULE-7 | The rule table shall name the queues the officer may take an unassigned issue from, and the first table shall name none. Issue: #5. | T | Implemented |
| WB-RULE-8 | The model table `revision/models.yml` shall give one model ID and one effort for each role that has a prompt, and for no other role. Replaced by WB-RULE-40. Issue: #5. | T | Deprecated |
| WB-RULE-9 | The first model table shall give `claude-sonnet-5-5` at medium effort for tester, build and fix, and `claude-opus-5-5` at high effort for review. Issue: #5. | T | Implemented |
| WB-RULE-10 | A rule table in which one role excludes another and the other does not exclude the first shall be refused. Issue: #5. | T | Implemented |
| WB-RULE-11 | A role's row that lacks one of its fields shall be refused, and the message shall name the role and the field. Issue: #5. | T | Implemented |
| WB-RULE-12 | A key that is not known shall be refused at any depth, and the message shall name where it stands. Issue: #5. | T | Implemented |
| WB-RULE-13 | A number of review rounds that is not a whole number of at least 1 shall be refused. Issue: #5. | T | Implemented |
| WB-RULE-14 | A model ID that is not exact, such as `latest` or a name without a version, shall be refused. Issue: #5. | T | Implemented |
| WB-RULE-15 | A file that is not YAML shall be refused with the file named and without a stack trace. Issue: #5. | T | Implemented |
| WB-RULE-16 | A key written twice in a YAML file shall be refused. Issue: #5. | T | Implemented |
| WB-RULE-17 | A value that YAML reads as another type than its field asks for shall be refused, and the message shall say to put it in quotes. Issue: #5. | T | Implemented |
| WB-RULE-18 | Every YAML file shall be read with the safe loader of the library, through one loader in `wb`. Issue: #5. | I | Implemented |
| WB-RULE-19 | The package shall name the YAML library as what it needs, so that `dnf` installs it with `wb`. Issue: #5. | D | Implemented |
| WB-RULE-20 | An effort that is not one of low, medium, high, xhigh and max shall be refused, and the message shall name the role. Issue: #5. | T | Implemented |
| WB-RULE-21 | A table that is not there or cannot be read shall be refused with the file named and without a stack trace. Issue: #5. | T | Implemented |
| WB-RULE-22 | A role named in a role's list of roles it may not also hold that is not one of the five, or is the role itself, shall be refused. Replaced by WB-RULE-41. Issue: #5. | T | Deprecated |
| WB-RULE-23 | Nothing in `wb` but `rules.py` shall name the two tables. Replaced by WB-RULE-29. Issue: #5. | T | Deprecated |
| WB-RULE-24 | A role's prompt that is not `none` or a relative path that ends in `.md` and has no `..` part shall be refused, and the message shall name the role. Issue: #5. | T | Implemented |
| WB-RULE-25 | A name written twice in the list of what always goes to a person shall be refused. Issue: #5. | T | Implemented |
| WB-RULE-26 | A merge key (`<<`) in a YAML file shall be refused, and the message shall name the file and the line. Issue: #5. | T | Implemented |
| WB-RULE-27 | A rule table whose list of what always goes to a person lacks `merge` shall be refused. Issue: #5. | T | Implemented |
| WB-RULE-28 | A role named twice in a role's list of roles it may not also hold shall be refused. Issue: #5. | T | Implemented |
| WB-RULE-29 | Nothing in `wb` but `src/rules.rs` shall name the two tables. Issue: #23. | T | Implemented |
| WB-RULE-30 | An anchor or an alias in a YAML file shall be refused, with the line. Issue: #23. | T | Implemented |
| WB-RULE-31 | The rule table shall give how many times a session is asked for a last message that has the fields its role names, and the first table shall give two. Issue: #45. | T | Implemented |
| WB-RULE-32 | The first rule table shall send to a person a last message that still lacks its fields after the last ask. Issue: #45. | T | Implemented |
| WB-RULE-33 | A number of asks for the last message that is not a whole number of at least 1 shall be refused, and the message shall name the field. Issue: #45. | T | Implemented |
| WB-RULE-34 | The rule table shall name, in order, the roles that a requirement a person adds to a change under review goes to on the same change, and the first table shall name tester, then build. Issue: #49. | T | Implemented |
| WB-RULE-35 | A role in the list for an added requirement that is not a role with a prompt, or that is written twice, and a list with no role, shall be refused, and the message shall name the field. Issue: #49. | T | Implemented |
| WB-RULE-36 | The first rule table shall send to a person a requirement added to a change. Issue: #49. | T | Implemented |
| WB-RULE-37 | The first rule table shall send to a person the check of a fix made after the last round. Issue: #49. | T | Implemented |
| WB-RULE-38 | Where an issue or a project names another model or effort than the model table, the table shall win: an issue may ask for a stronger model or a higher effort, never less, and what ran is recorded by the stamp of #31. `docs/revision.md`, point 5, and the model table shall say so. Issue: #51. | I | Implemented |
| WB-RULE-39 | The rule table `revision/rules.yml` shall have a row for each of the roles breakdown, tester, build, review, fix and officer, and for no other role. Issue: #75. | T | Implemented |
| WB-RULE-40 | The model table `revision/models.yml` shall give one model ID and one effort under `models` for each role that has a prompt but the breakdown, and for no other role. Issue: #75. | T | Implemented |
| WB-RULE-41 | A role named in a role's list of roles it may not also hold that is not one of the six, or is the role itself, shall be refused. Issue: #75. | T | Implemented |
| WB-RULE-42 | The breakdown's row of the rule table shall say that the breakdown comes before the tester. Issue: #75. | T | Implemented |
| WB-RULE-43 | The breakdown's row of the rule table shall say that no other role of an issue is started until a person has accepted the issue's file. Issue: #75. | T | Implemented |
| WB-RULE-44 | The rule table shall say that the breakdown may not also hold any other role, and that no other role may also hold the breakdown. Issue: #75. | T | Implemented |
| WB-RULE-45 | The model table shall have the key `local_models`, the place for the models of local runners, and the first table that has it shall leave it empty. Issue: #75. | T | Implemented |
| WB-RULE-46 | A model table with anything under `local_models` shall be refused, and the message shall name the field. Issue: #75. | T | Implemented |
| WB-RULE-47 | The breakdown's row of the rule table shall allow it to post its questions on the issue, one comment each, and nothing else, and shall say that it reads the issue with its comments. Replaced by WB-RULE-50. Issue: #95, #145. | T | Deprecated |
| WB-RULE-48 | The rows of the breakdown, the tester, the build and the fix in the rule table shall each allow it to push to the branch of the issue, and the rows of the review and the officer shall allow no push. Replaced by WB-RULE-51 and WB-RULE-52. Issue: #98, #145. | T | Deprecated |
| WB-RULE-49 | The rows of the tester, the build, the review and the fix in the rule table shall allow each to post its questions on the issue, one comment each, and shall say that each reads the issue with its comments. Replaced by WB-RULE-53. Issue: #113, #145. | T | Deprecated |
| WB-RULE-50 | The breakdown's row of the rule table shall allow it to post its questions on the issue, in one comment per session, and nothing else, and shall say that it reads the issue with its comments. Issue: #145. | T | Draft |
| WB-RULE-51 | The breakdown's row of the rule table shall allow it to push to the branch of the issue, and the rows of the tester, the build, the review, the fix and the officer shall allow no push. Issue: #145. | T | Draft |
| WB-RULE-52 | No row of the rule table shall allow a role to open the change. Issue: #145. | T | Draft |
| WB-RULE-53 | The rows of the tester, the build, the review and the fix in the rule table shall allow each to post its questions on the issue, in one comment per session, and shall say that each reads the issue with its comments. Issue: #145. | T | Draft |
