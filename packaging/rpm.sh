#!/bin/sh
# Builds the RPM of this checkout: packaging/rpm.sh [<folder for the result>]
# The default folder is build/rpm. The package is then installed with
#   sudo dnf install build/rpm/RPMS/$(uname -m)/watchbill-[0-9]*.rpm
# Needs: rpm-build git-core cargo rust cargo-rpm-macros rust-yaml-rust2-devel rust-sha2-devel rust-lexopt-devel
# On Enterprise Linux 9 cargo-rpm-macros and the crates are in EPEL.
set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
if ! git -C "$here" rev-parse --git-dir >/dev/null 2>&1; then
  echo "rpm.sh needs a git checkout: $here is not one" >&2
  exit 2
fi
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
# git lists an untracked folder that holds a repository of its own as the folder (path/); tar
# would pack it whole, so only files are kept and the folders are named. A submodule is listed
# as a file (no trailing /), so it is not left out here: a checkout with one is not built for.
# The list is NUL-separated from git to tar, so any file name is kept; this relies on the
# -z of GNU grep, which Fedora and Enterprise Linux 9 have. The tr -cd '\0' is POSIX.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
(cd "$here" && git ls-files -co --exclude-standard -z) >"$tmp/all"
grep -z '/$' "$tmp/all" >"$tmp/folders" || true
grep -zv '/$' "$tmp/all" >"$tmp/files" || true
nfolders=$(tr -cd '\0' <"$tmp/folders" | wc -c | tr -d ' ')
nfiles=$(tr -cd '\0' <"$tmp/files" | wc -c | tr -d ' ')
if [ "$nfolders" -gt 0 ]; then
  echo "rpm.sh: left out $nfolders folder(s) that hold a repository of their own: $(tr '\0' '\n' <"$tmp/folders" | paste -sd, -)" >&2
fi
echo "rpm.sh: packing $nfiles files from $here into $out/SOURCES/watchbill-$version.tar.gz" >&2
(cd "$here" &&
  tar --null -T - --sort=name --mtime="@$SOURCE_DATE_EPOCH" --owner=0 --group=0 --numeric-owner \
    --transform "s,^,watchbill-$version/," -czf "$out/SOURCES/watchbill-$version.tar.gz") <"$tmp/files"

echo "rpm.sh: the build starts: rpmbuild -ba packaging/watchbill.spec" >&2
rpmbuild -ba "$here/packaging/watchbill.spec" --define "_topdir $out" --define "wb_version $version" \
  --define "use_source_date_epoch_as_buildtime 1" --define "clamp_mtime_to_source_date_epoch 1" \
  --define "source_date_epoch_from_changelog 0"
find "$out/RPMS" -name '*.rpm'
