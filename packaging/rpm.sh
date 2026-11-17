#!/bin/sh
# Builds the RPM of this checkout: packaging/rpm.sh [<folder for the result>]
# The default folder is build/rpm. The package is then installed with
#   sudo dnf install build/rpm/RPMS/$(uname -m)/watchbill-[0-9]*.rpm
# Needs: rpm-build git-core cargo rust cargo-rpm-macros rust-yaml-rust2-devel rust-sha2-devel rust-lexopt-devel
# On Enterprise Linux 9 cargo-rpm-macros and the crates are in EPEL.
set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-$here/build/rpm}
mkdir -p "$out/SOURCES"
out=$(cd "$out" && pwd)

# The version is in one place: Cargo.toml.
version=$(sed -n '/^\[package\]/,/^\[/{s/^version = "\(.*\)"$/\1/p}' "$here/Cargo.toml")

# The date of the package is the date of the commit, so two builds of one commit agree.
# A tree with uncommitted changes takes the date of its last commit. There is no %changelog.
SOURCE_DATE_EPOCH=$(git -C "$here" log -1 --format=%ct)
export SOURCE_DATE_EPOCH

# The files git knows or would take, so no build output and no cache goes into the package.
(cd "$here" && git ls-files -co --exclude-standard -z |
  tar --null -T - --sort=name --mtime="@$SOURCE_DATE_EPOCH" --owner=0 --group=0 --numeric-owner \
    --transform "s,^,watchbill-$version/," -czf "$out/SOURCES/watchbill-$version.tar.gz")

rpmbuild -ba "$here/packaging/watchbill.spec" --define "_topdir $out" --define "wb_version $version" \
  --define "use_source_date_epoch_as_buildtime 1" --define "clamp_mtime_to_source_date_epoch 1" \
  --define "source_date_epoch_from_changelog 0"
find "$out/RPMS" -name '*.rpm'
