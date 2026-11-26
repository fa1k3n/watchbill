# How the work is run

This is the practice behind the rules of [CLAUDE.md](../CLAUDE.md). It holds for every session, on any computer and with any model. Where it and CLAUDE.md differ, CLAUDE.md holds.

## The scope of a milestone

- The issues of a milestone are fixed when it starts. No new issue joins a milestone under way (the owner, 2026-11-26).
- A review that finds a defect in a piece of the milestone may give an issue in it. The issue says which piece it mends.
- A new feature goes to the next milestone. Where the owner wants it sooner, it is folded narrowly into an issue of the milestone, with the owner's yes, and written into that issue.
- Before filing an issue into a milestone under way, ask the owner.
- Each piece gets its own range of row IDs when it is planned, written in the milestone's description. Two pieces that look for "the next free number" at once take the same one.

## The session that runs a release

The owner may give one session the running of a milestone. That session:

- briefs a builder for each piece, in a worktree of its own, as its working folder ([Work that can be followed](#work-that-can-be-followed));
- has each piece reviewed by a fresh session, then answered, then looked at a second time;
- runs `packaging/rpm.sh` once at the head, posts `[ready]`, and merges with a squash.

It never merges a change to CLAUDE.md or to this file, never merges a piece the owner reviews, and never tags a release. Those are the owner's.

- **One piece at a time** on a machine of about 8 GB. See [The machine](#the-machine).
- **A question for the owner** goes with a recommendation, as one choice. When the owner is away and has said "accept the recommendations", the session takes the recommended option. It writes it into the issue as the owner's word given in advance, and lists it in its report.
- **After the two looks:** where the second look still finds something that blocks, the session decides the fix, has it built, and reads the new commits itself before it merges. The `[ready]` comment says so.
- **Before `[ready]`:** the branch holds the latest `main`, and `gh pr view <n> --json mergeable` says it can be merged. Wait out `UNKNOWN`.
- **The steps ready, merge and close** are joined with `&&`, so that one that fails stops the rest. An issue is closed only after the pull request says `MERGED`. A squash merged through the REST API does not always close the issue it names; close it then.
- **When GitHub fails** (502, 504, a GraphQL error), wait and retry, or use the REST API: `gh api repos/<owner>/<repo>/issues/<n>/comments -F body=@<file>`. Then check that one comment landed, not two.
- **After a batch of merges,** run the full `cargo test` on `main` once in each container. Pieces that merge cleanly can still break together.

## The builder's brief

Every builder is given this, with the issue's number, its branch from the current `origin/main`, and its row IDs.

- Read CLAUDE.md and the whole issue with its comments (`gh issue view <n> --comments`). A later `[session]` comment overrides the body.
- **Tests first:** one commit with the new tests, red, the failing names in its message. Then the code, until green. A test that cannot be red before its code says why in the commit message.
- **Rows:** only the IDs given. Rows reserved as Draft for the issue are used as they stand. A changed row is named in the pull request.
- **An existing test** is changed only where the issue allows it. Each is named in a `[session]` comment on the issue and in the pull request, with why. No test may come to check less than it did.
- **Where the issue leaves a choice,** take the narrowest and most restrictive reading, and write it into the issue as one `[session]` comment, before or with the code.
- **Trust:** every git read that decides what counts passes `--no-replace-objects`, turns grafts off and fetches nothing (`signers::git`, `plan::git`, `status::git`). A test makes its own keys and sets its own `HOME`, `XDG_CONFIG_HOME` and `GIT_CONFIG_GLOBAL`. It never reads the person's keys, agent or `/etc` files.
- **During the rounds,** run the tests of what is touched. At the head, run the full suite once in the Enterprise Linux 9 container, then once in the Fedora container, with `--network=none`. Do not run `rpm.sh`: the session that runs the release does that once.
- **At the end:** push the branch by name, and open the pull request as a draft (ready where the owner reviews the piece). Do not mark it ready, merge, or post `[ready]`. Report the number, the head, the red output, what was run, what was decided, and what was left.

## The reviewer's brief

- A fresh session, one level above the builder (CLAUDE.md). It starts from the issue and the diff. It reads the pull request's body only to check its claims.
- Work in a worktree of its own on the piece's branch, detached, as its working folder, never the builder's ([Work that can be followed](#work-that-can-be-followed)). Remove it at the end.
- Look for:
  - what the issue asks that is missing or different;
  - bugs;
  - tests that do not test what their row says, or that pass without the code;
  - rows out of range or not Implemented;
  - an existing test changed without leave;
  - the rules of CLAUDE.md broken: exit statuses, a command that reads and writes, the network, crates, flags as git's, British spelling, plain sentences.
- For a piece about trust, try to break it with probe tests in the worktree, never committed.
- Write one comment, its first line `[review]`, each finding numbered and marked **must** (blocks the merge) or **note**, with a suggested answer. Say what was run.
- **The second look** reads the author's `[answer]` and the new commits only. It ends with "nothing blocks", or with the findings that still block.
- A reviewer changes no code and pushes nothing.

## Releases

1. When a step fails, read its log and the artifacts themselves (`gh run download`) before guessing.
2. Write the cause as an issue in the release's milestone, with the decision the owner must take in it.
3. Mend it with a red test first, a review, and `[ready]`.
4. Give the owner one block to paste: the merge, the tag, the push.
5. After the run is green, download the release and check it with `sha256sum -c SHA256SUMS`. Then close the issues and the milestone.

## The machine

The numbers are for a laptop of 7.3 GB, shared with a browser and other sessions. A larger machine may raise them; a smaller one must lower them.

On the server of 8 cores and 31 GB, the numbers are these instead:

- **Two heavy builds at once at most:** a container, a capped `cargo test`, or `rpm.sh`. While Ollama has a model loaded, one at a time.
- **Each build:** `MemoryMax=4G` with `MemorySwapMax=0` on the host, `--memory=4g --memory-swap=4g` in a container, and `-j 4`.

The rest of this section holds on both.

- **One heavy build at a time, in total:** a container, a capped `cargo test`, or `rpm.sh`. Two capped builds of 2 GB at once ran the machine out of memory on 2026-11-25, and the kernel ended a session. Before a build, wait until `podman ps -q` is empty, and start no second builder until the first has handed back.
- **Every `cargo` on the host** runs in a capped scope, with two jobs:
  `systemd-run --user --scope -p MemoryMax=2G -p MemorySwapMax=0 cargo test -j 2 …`
  Without `MemorySwapMax=0` the scope still swaps for minutes.
- **Every container:** `--memory=2g --memory-swap=2g`. `--memory` alone still swaps. Run the Enterprise Linux 9 and the Fedora checks one after the other, never at once: a shared `:Z` mount races too.
- **A test that can read without end** (a device, a FIFO, a huge file) runs `wb` as a child under `wb_within`, never library code inside the test process, which no cap reaches.
- **Disk:** `/tmp` is a small tmpfs. Build output and `TMPDIR` go under `/var/tmp`: `CARGO_TARGET_DIR=/var/tmp/wb-<n>-target TMPDIR=/var/tmp/wb-<n>-tmp`. Remove them when the piece is merged.
- **Suspend:** a laptop that suspends when idle freezes every background build, and an agent looks stuck for hours. While agents run, hold `systemd-inhibit --what=idle:sleep --mode=block --why="…" sleep 8h`. A closed lid still suspends. When an agent seems slow, check `journalctl | grep 'will suspend'` first.
- **Containers:** reuse the ready images (`podman images`) and install nothing per run. `packaging/rpm.sh` needs a git checkout: clone the branch, mount it read-only, and copy it inside. `timeout` on `podman run` stops the client, not the container. Give each container a name, and stop it by name.

## Agents

- **Run checks in the foreground.** An agent that waits for a background job to notify it hands back before the work is done, or never wakes. A step that runs past 20 minutes is stopped and said, not repeated.
- **Leave nothing running** when handing back.
- Never `pkill`, stop a container, or delete what another session made.
- An agent that has reported can wake on an old message. Stop it before its work is given to another, or two will push to one branch.
- When a session is ended by the machine (`journalctl -k | grep -i 'killed process'`), the last command in its record is usually the cause. Start again from the state of GitHub: the pull requests, their comments, and what is pushed.

## Work that can be followed

How `wb` is developed is shaped so that a tool that only watches can tell which issue and which role each step belongs to. It is this project's own choice: `wb` asks none of it of a team, and none of it is in what `wb` installs.

1. **Each agent works in its own worktree, as its working folder.** The session that runs a release starts each builder and reviewer in the agents' worktree mode. It never has one `cd` into a folder from the main checkout, which shows every step as the main checkout's. Build output and `TMPDIR` stay under `/var/tmp` (see [The machine](#the-machine)).
2. **A reviewer has a worktree of its own** on the piece's branch, detached, never the builder's.
3. **Each commit names its issue and its role** in two trailers, beside `Co-Authored-By`:
   ```
   Issue: #<n>
   Role: <tester | build | review | fix | release>
   ```
   A squash merge keeps the pull request's commits on GitHub, so the trailers can still be read there.
4. **Each step on GitHub says what it is** on the first line of its comment: `[review]`, `[answer]`, `[ready]` on a pull request, and `[session]` on an issue.
5. **The session that runs a release works on `main`** and coordinates. Its own commits are never on an issue's branch.

## Words

- Flags take git's word, then docker's, and git's where they differ (CLAUDE.md).
- A packaged crate is preferred to lines written by hand, where it does most of the job (CLAUDE.md).
- Fable is spared: reviews and agents use opus, unless the owner says otherwise. Where an issue's Run line names fable, write the change into the issue.
