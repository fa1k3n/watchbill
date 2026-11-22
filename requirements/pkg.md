# The package

How `wb` is installed and what it needs. What the package carries of the revision: [rev.md](rev.md).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-PKG-1 | `wb` shall run on Python 3.9 or later. Replaced by WB-PKG-5. Issue: #2. | D | Deprecated |
| WB-PKG-2 | `wb` shall be installed with `dnf` from an RPM that `packaging/rpm.sh` builds, and then give a working `wb` command. Issue: #2. | D | Implemented |
| WB-PKG-3 | `wb` shall need nothing from outside the distribution's own repositories, to build, to install or to run. Replaced by WB-PKG-6 and WB-PKG-7. Issue: #2. | I | Deprecated |
| WB-PKG-4 | The package shall give the two commands `wb` and `watchbill`, the same program. Issue: #2. | D | Implemented |
| WB-PKG-5 | `wb` shall be built with the Rust that the distribution packages, 1.97 or later. Issue: #21. | D | Implemented |
| WB-PKG-6 | A build of `wb` shall take every crate from the distribution's packages, with EPEL on Enterprise Linux 9, and ask nothing of the network. Issue: #21. | D | Implemented |
| WB-PKG-7 | The package of `wb` shall require `git-core`, and no Python and no crate. Issue: #25. | D | Implemented |
| WB-PKG-8 | The version in `Cargo.toml` shall be the version in `src/watchbill/__init__.py`, as long as both are there. Replaced by WB-PKG-9: the Python is removed. Issue: #21. | T | Deprecated |
| WB-PKG-9 | The version in `Cargo.toml` shall be the version that `wb --version` prints, and the only place the version of the package is written. Issue: #25. | T | Implemented |
| WB-PKG-10 | A fault in `wb` itself shall end the command with 1 and one line, and print no panic message. Issue: #25. | T | Implemented |
| WB-PKG-11 | The package shall name the licence of Watchbill in its metadata, and the `License:` of the spec and the `license` of `Cargo.toml` shall be the same SPDX name, `Apache-2.0`. Issue: #39. | T | Implemented |
| WB-PKG-12 | The package shall install the files `LICENSE` and `NOTICE` as licence files. Issue: #39. | T | Implemented |
| WB-PKG-13 | The crates `wb` is built from shall be those that the list in `CLAUDE.md` names, no more and no fewer. Issue: #34. | T | Implemented |
| WB-PKG-14 | `packaging/rpm.sh` shall date the package by the commit it builds: two builds of one commit give packages whose files have the same dates, and the build warns of no date. Issue: #63. | T | Implemented |
| WB-PKG-15 | `packaging/rpm.sh` shall pack only files: a folder that git lists, such as an untracked folder holding a repository of its own, shall be left out and named on standard error. Issue: #99. | T | Implemented |
| WB-PKG-16 | `packaging/rpm.sh` shall say on standard error how many files it packs from which checkout before it packs, and that the build starts before `rpmbuild`. Issue: #99. | T | Implemented |
| WB-PKG-17 | A pushed tag of the form `<digits>.<digits>.<digits>` shall start the release workflow, and no other tag or push shall. Issue: #111. | T | Implemented |
| WB-PKG-18 | Before it builds, the release workflow shall stop and say why when the tag is not the version in `Cargo.toml`, or `revisions.txt` has no line for it, or that line's hash is not the SHA-256 of the manifest. Issue: #111. | T | Implemented |
| WB-PKG-19 | The release workflow shall build the RPM with `packaging/rpm.sh` on Fedora and on Enterprise Linux 9, install it there, and stop unless `wb revision` gives the tag's version and hash. Issue: #111. | T | Implemented |
| WB-PKG-20 | Only when both builds pass shall the release workflow make a GitHub release of the tag with the two binary RPMs, the source RPM and `SHA256SUMS`, and no debug package; it shall never write over a release that is there, and ask for no permission beyond `contents: write`. Issue: #111. | T | Implemented |
| WB-PKG-21 | The release workflow shall install the packages of the build with the documents kept (`--setopt=tsflags=`), as the container images of Fedora leave them out and a crate includes one. Issue: #120. | T | Implemented |
| WB-PKG-22 | The RPM shall install `packaging/wb.bash` as `/usr/share/bash-completion/completions/wb`, with a link `watchbill` beside it, shall own the two folders, and shall not require `bash-completion`. Issue: #220. | T | Implemented |
| WB-PKG-23 | The RPM shall install `wb.1` and the page of each built command under `%{_mandir}/man1`, and own them, with no tool fetched to make them. Issue: #182. | T | Implemented |
