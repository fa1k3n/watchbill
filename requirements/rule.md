# The rule table and the model table

The two tables of the first revision and the one loader that reads YAML. What a revision is: [docs/revision.md](../docs/revision.md).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-RULE-1 | The rule table `revision/rules.yml` shall have a row for each of the roles tester, build, review, fix and officer. Issue: #5. | T | Draft |
| WB-RULE-2 | A role's row shall give the step it performs and what it produces, what it may read, what it may do, the roles the same team member may not also hold on one change, and its prompt file or `none`. Issue: #5. | T | Draft |
| WB-RULE-3 | The rule table shall give the greatest number of review rounds, and the first table shall give two. Issue: #5. | T | Draft |
| WB-RULE-4 | The rule table shall list what always goes to a person. Issue: #5. | T | Draft |
| WB-RULE-5 | The rule table shall say that a change that touches classed code needs a person as its verifier. Issue: #5. | T | Draft |
| WB-RULE-6 | The officer's row shall say that the officer holds no other role on a task it has handed out and never merges. Issue: #5. | T | Draft |
| WB-RULE-7 | The rule table shall name the queues the officer may take an unassigned task from, and the first table shall name none. Issue: #5. | T | Draft |
| WB-RULE-8 | The model table `revision/models.yml` shall give one model ID and one effort for each role that has a prompt, and for no other role. Issue: #5. | T | Draft |
| WB-RULE-9 | The first model table shall give `claude-sonnet-5-5` at medium effort for tester, build and fix, and `claude-opus-5-5` at high effort for review. Issue: #5. | T | Draft |
| WB-RULE-10 | A rule table in which one role excludes another and the other does not exclude the first shall be refused. Issue: #5. | T | Draft |
| WB-RULE-11 | A role's row that lacks one of its fields shall be refused, and the message shall name the role and the field. Issue: #5. | T | Draft |
| WB-RULE-12 | A key that is not known shall be refused at any depth, and the message shall name where it stands. Issue: #5. | T | Draft |
| WB-RULE-13 | A number of review rounds that is not a whole number of at least 1 shall be refused. Issue: #5. | T | Draft |
| WB-RULE-14 | A model ID that is not exact, such as `latest` or a name without a version, shall be refused. Issue: #5. | T | Draft |
| WB-RULE-15 | A file that is not YAML shall be refused with the file named and without a stack trace. Issue: #5. | T | Draft |
| WB-RULE-16 | A key written twice in a YAML file shall be refused. Issue: #5. | T | Draft |
| WB-RULE-17 | A value that YAML reads as another type than its field asks for shall be refused, and the message shall say to put it in quotes. Issue: #5. | T | Draft |
| WB-RULE-18 | Every YAML file shall be read with the safe loader of the library, through one loader in `wb`. Issue: #5. | I | Draft |
| WB-RULE-19 | The package shall name the YAML library as what it needs, so that `dnf` installs it with `wb`. Issue: #5. | D | Draft |
| WB-RULE-20 | An effort that is not one of low, medium, high, xhigh and max shall be refused, and the message shall name the role. Issue: #5. | T | Draft |
| WB-RULE-21 | A table that is not there or cannot be read shall be refused with the file named and without a stack trace. Issue: #5. | T | Draft |
| WB-RULE-22 | A role named in a role's list of roles it may not also hold that is not one of the five, or is the role itself, shall be refused. Issue: #5. | T | Draft |
| WB-RULE-23 | Nothing in `wb` but `rules.py` shall name the two tables. Issue: #5. | T | Draft |
