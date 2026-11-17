#!/bin/sh
# Builds the RPM of this checkout: packaging/rpm.sh [<folder for the result>]
# The default folder is build/rpm. The package is then installed with
#   sudo dnf install build/rpm/RPMS/noarch/watchbill-*.noarch.rpm
# Needs: rpm-build python3-devel pyproject-rpm-macros python3-setuptools python3-pip python3-wheel python3-pyyaml git-core
# cargo rust cargo-rpm-macros
# and on Enterprise Linux 9 also python3-tomli, with the CRB repository enabled, and
# cargo-rpm-macros from EPEL (and later the crates of the distribution, also from EPEL).
set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-$here/build/rpm}
mkdir -p "$out/SOURCES"
out=$(cd "$out" && pwd)

# The revision is in one place: the package itself.
version=$(sed -n 's/^__version__ = "\(.*\)"$/\1/p' "$here/src/watchbill/__init__.py")

# The files git knows or would take, so no build output and no cache goes into the package.
(cd "$here" && git ls-files -co --exclude-standard -z |
  tar --null -T - --transform "s,^,watchbill-$version/," -czf "$out/SOURCES/watchbill-$version.tar.gz")

rpmbuild -ba "$here/packaging/watchbill.spec" --define "_topdir $out" --define "wb_version $version"
find "$out/RPMS" -name '*.rpm'
