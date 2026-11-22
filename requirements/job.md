# The job

What `wb add` takes in and how a job is kept in git: its text a blob under its hash, its name a ref. What was decided: [docs/revision.md](../docs/revision.md), point 15.

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-JOB-1 | `wb add` shall store the text it reads from standard input as a git blob under the hash of that text, and end with 0. Issue: #234. | T | Draft |
| WB-JOB-2 | `wb add` shall make the ref `refs/watchbill/<name>` point at a commit whose tree holds `job`, the blob of the text, and `job.yml`. Issue: #234. | T | Draft |
| WB-JOB-3 | `job.yml` shall hold the name, the hash of the text, the source (`--source <address>`, or `none`), the level, the person's git identity and the time. Issue: #234. | T | Draft |
| WB-JOB-4 | `wb add -F <file>` and `wb add "<text>"` shall give the same hash as standard input does for the same text. Issue: #234. | T | Draft |
| WB-JOB-5 | `wb add` shall end with 2 and write nothing when it is given none of standard input, `-F` and the argument, or any two or all three of them. Issue: #234. | T | Draft |
| WB-JOB-6 | With no `--name`, `wb add` shall name the ref with the first 12 hex digits of the hash of the text. Issue: #234. | T | Draft |
| WB-JOB-7 | `wb add` given the same text under the same name again shall write no new commit and end with 0. Issue: #234. | T | Draft |
| WB-JOB-8 | `wb add` given changed text under a name already taken shall write a new commit on the ref that names the commit before it and holds the new blob. Issue: #234. | T | Draft |
| WB-JOB-9 | `wb add` given changed text under a name already taken shall say the hash of the text before and after. Issue: #234. | T | Draft |
| WB-JOB-10 | `wb add` shall end with 1 and write nothing when the name is not a valid ref name. Issue: #234. | T | Draft |
| WB-JOB-11 | `wb add --level <level>` shall end with 1 and write nothing when the level is above the repository's; with no `--level` the level is the repository's. Issue: #234. | T | Draft |
| WB-JOB-12 | `wb add --hash-only` shall end with 1 and write nothing unless the organisation's file has `jobs: hash_only: allowed`, a key read through the one loader. Issue: #234. | T | Draft |
| WB-JOB-13 | `wb add --hash-only`, where the organisation's file allows it, shall write no blob of the text, and `job.yml` shall hold the hash of the text and its source. Issue: #234. | T | Draft |
| WB-JOB-14 | `wb add` shall say what it will write before it writes it. Issue: #234. | T | Draft |
| WB-JOB-15 | `wb add` shall ask nothing of the network, and shall run with the network off. Issue: #234. | T | Draft |
| WB-JOB-16 | `wb add` shall only queue the job: it shall run no prompt and start no session. Issue: #234. | T | Draft |
| WB-JOB-17 | `wb init` shall add the fetch line `+refs/watchbill/*:refs/watchbill/*` to the remote `origin` in `.git/config`, and say so before it writes. Issue: #234. | T | Draft |
| WB-JOB-18 | `wb upgrade` shall add the fetch line `+refs/watchbill/*:refs/watchbill/*` to the remote `origin` where it is missing, and say so before it writes. Issue: #234. | T | Draft |
| WB-JOB-19 | `wb upgrade` shall leave the fetch line as it is where it is already there. Issue: #234. | T | Draft |
| WB-JOB-20 | `wb help add` shall say what `wb add` takes in, how it names a job, its options and its exit statuses. Issue: #234. | T | Draft |
