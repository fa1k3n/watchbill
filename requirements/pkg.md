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
