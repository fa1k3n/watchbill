# The job

What `wb plan add` takes in and how a job is kept in git: its text a blob under its hash, its name a ref. What was decided: [docs/revision.md](../docs/revision.md), point 15. `wb add` was folded into `wb plan add` on 2026-11-24 (#290).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-JOB-1 | `wb plan add -F -` shall store the text it reads from standard input as a git blob under the hash of that text, and end with 0. Issue: #234. | T | Implemented |
| WB-JOB-2 | `wb plan add` shall make the ref `refs/watchbill/<name>` point at a commit whose tree holds `job.yml` and the text of each source kept, under `sources/<hash>`. A name may hold `/`. Issue: #234. | T | Implemented |
| WB-JOB-3 | `job.yml` shall hold the name and, for each source in the order it was added, the hash of its text, whether the text is kept, the source (`--source <address>`, or `none`), the level, the person's git identity and the time. Issue: #234. | T | Implemented |
| WB-JOB-4 | `wb plan add -F <file>` and `wb plan add -m <text>` shall give the same hash as standard input does for the same text. A relative file is read from the folder `wb` runs in. Issue: #234. | T | Implemented |
| WB-JOB-5 | `wb plan add` shall end with 2 and write nothing when it is given none of `-F`, `-m` and `--from`, or more than one of them, or `-F` or `--from` twice. Issue: #234. | T | Implemented |
| WB-JOB-6 | With no name, `wb plan add` shall name the ref with the first 12 hex digits of the hash of the text. Issue: #234. | T | Implemented |
| WB-JOB-7 | `wb plan add` given text that is already a source of the job of that name shall write no new commit and end with 0. Issue: #234. | T | Implemented |
| WB-JOB-8 | `wb plan add` given other text under a name already taken shall add it as a source: a new commit on the ref that names the commit before it, and whose tree keeps the sources there and holds the new one. Issue: #234, #290. | T | Implemented |
| WB-JOB-9 | `wb plan add` given other text under a name already taken shall say the hashes of the sources there and the hash of the one it adds. Issue: #234, #290. | T | Implemented |
| WB-JOB-10 | `wb plan add` shall end with 1, name the rule and write nothing when `git check-ref-format` refuses `refs/watchbill/<name>`, as for a name with `:`, or when the name meets the name of a job as a folder. Issue: #234. | T | Implemented |
| WB-JOB-11 | `wb plan add` shall end with 1 and write nothing when there is no organisation's file, or `--level <level>` is not one of its levels or is above the repository's. With no `--level`, the level is the repository's. Issue: #234. | T | Implemented |
| WB-JOB-12 | `wb plan add --hash-only` shall end with 1 and write nothing unless the organisation's file has `jobs: hash_only: allowed`. Issue: #234. | T | Implemented |
| WB-JOB-13 | `wb plan add --hash-only`, where the organisation's file allows it, shall write no blob of the text, and `job.yml` shall hold the hash of the text and its source. Issue: #234. | T | Implemented |
| WB-JOB-14 | `wb plan add` shall say what it will write (the blob, `job.yml`, the tree and the commit on the ref) before it writes it. Issue: #234. | T | Implemented |
| WB-JOB-15 | `wb plan add` with `-F` or `-m` shall ask nothing of the network and run no pattern. Issue: #234. | T | Implemented |
| WB-JOB-16 | `wb plan add` shall only queue the job: it shall run no prompt and start no session. Issue: #234. | T | Implemented |
| WB-JOB-17 | `wb init` shall add the fetch line `+refs/watchbill/*:refs/watchbill/*` to the remote `origin` in `.git/config`, where `origin` is there, and say so before it writes. Issue: #234. | T | Implemented |
| WB-JOB-18 | `wb upgrade` shall add the fetch line `+refs/watchbill/*:refs/watchbill/*` to the remote `origin` where it is missing, and say so before it writes; `--dry-run` shall say it and write nothing. Issue: #234. | T | Implemented |
| WB-JOB-19 | `wb upgrade` shall leave the fetch line as it is where it is already there. Issue: #234. | T | Implemented |
| WB-JOB-20 | `wb help plan` shall say what `wb plan add` takes in, how it names a job, its options and its exit statuses. Issue: #234. | T | Implemented |
| WB-JOB-21 | `wb plan add -m <text>`, given more than once, shall join the texts as `git commit` joins them: each a paragraph, one blank line between, and one line break at the end. Issue: #234. | T | Implemented |
| WB-JOB-22 | `wb plan add` given, under a name that is not taken, text that is a source of a job already shall point the new ref at that job's commit and write nothing else: one job with two names. Issue: #234. | T | Implemented |
| WB-JOB-23 | `wb plan add --from <adapter> <number>` shall end with 2, say to pipe the text in with `-F -`, and run nothing, where no adapter of that name is set up: until adapters are built, where `tracker:` of `watchbill.yml` is not the adapter or the file has no `issue_text`. Issue: #234. | T | Implemented |
| WB-JOB-24 | `wb plan add --from <adapter> <number>` shall take the text that `issue_text` prints for the number, as `-F -` takes it, name the job `<adapter>/<number>`, and keep the command it ran as the source. A number that is not digits, a name, or `--source` given with it shall end with 2. Issue: #234. | T | Implemented |
| WB-JOB-25 | The organisation's file may have `jobs: hash_only:` with `allowed` or `refused`, read through the one loader; any other key or value under `jobs` shall be refused, with the place named. Issue: #234. | T | Implemented |
| WB-JOB-26 | `wb plan` with no subcommand, or one that is not `add`, shall end with 2 and name `add`. There shall be no command `wb add`. Issue: #234, #290. | T | Implemented |
