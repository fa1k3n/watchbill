# The package

How `wb` is installed and what it needs. What the package carries of the revision: [rev.md](rev.md).

| ID | Requirement | Ver | Status |
|---|---|---|---|
| WB-PKG-1 | `wb` shall run on Python 3.9 or later. Issue: #2. | D | Implemented |
| WB-PKG-2 | `wb` shall be installed with `dnf` from an RPM that `packaging/rpm.sh` builds, and then give a working `wb` command. Issue: #2. | D | Implemented |
| WB-PKG-3 | `wb` shall need nothing from outside the distribution's own repositories, to build, to install or to run. Issue: #2. | I | Implemented |
| WB-PKG-4 | The package shall give the two commands `wb` and `watchbill`, the same program. Issue: #2. | D | Implemented |
