# The package

How `wb` is installed and what it needs. What the package carries of the revision: [rev.md](rev.md).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-PKG-1 | `wb` shall run on Python 3.9 or later. Issue: #2. | D | Implemented |
| WB-PKG-2 | `wb` shall be installed with `dnf` from an RPM that `packaging/rpm.sh` builds, and then give a working `wb` command. Issue: #2. | D | Implemented |
| WB-PKG-3 | `wb` shall need nothing from outside the distribution's own repositories, to build, to install or to run. Issue: #2. | I | Implemented |
| WB-PKG-4 | The package shall give the two commands `wb` and `watchbill`, the same program. Issue: #2. | D | Implemented |
| WB-PKG-5 | `wb` shall be built with the Rust that the distribution packages, 1.97 or later. Issue: #21. | D | Draft |
| WB-PKG-6 | A build of `wb` shall take every crate from the distribution's packages, with EPEL on Enterprise Linux 9, and ask nothing of the network. Issue: #21. | D | Draft |
| WB-PKG-7 | The package of `wb` shall require `git-core`, and no Python and no crate. Issue: #25. | D | Draft |
