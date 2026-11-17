#!/bin/sh
# Builds the RPM of this checkout: packaging/rpm.sh [<folder for the result>]
# The default folder is build/rpm. The package is then installed with
#   sudo dnf install build/rpm/RPMS/$(uname -m)/watchbill-[0-9]*.rpm
# Needs: rpm-build git-core cargo rust cargo-rpm-macros rust-yaml-rust2-devel rust-sha2-devel
# On Enterprise Linux 9 cargo-rpm-macros, rust-yaml-rust2-devel and rust-sha2-devel are in EPEL.
set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-$here/build/rpm}
mkdir -p "$out/SOURCES"
out=$(cd "$out" && pwd)

# The version is in one place: Cargo.toml.
version=$(sed -n '/^\[package\]/,/^\[/{s/^version = "\(.*\)"$/\1/p}' "$here/Cargo.toml")

# The files git knows or would take, so no build output and no cache goes into the package.
(cd "$here" && git ls-files -co --exclude-standard -z |
  tar --null -T - --transform "s,^,watchbill-$version/," -czf "$out/SOURCES/watchbill-$version.tar.gz")

rpmbuild -ba "$here/packaging/watchbill.spec" --define "_topdir $out" --define "wb_version $version"
find "$out/RPMS" -name '*.rpm'
