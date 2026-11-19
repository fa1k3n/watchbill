#!/bin/sh
# Checks a release tag: packaging/check-tag.sh <tag> [<folder of the checkout>]
# The tag must be the version in Cargo.toml, revisions.txt must have a line for it, and the hash
# on that line must be the SHA-256 of share/watchbill/revision.sha256.
# Exit 0 when it holds, 1 when it does not, 2 when it is used wrongly.
set -eu

if [ $# -lt 1 ] || [ $# -gt 2 ]; then
  echo "usage: check-tag.sh <tag> [<folder of the checkout>]" >&2
  exit 2
fi
tag=$1
here=${2:-$(cd "$(dirname "$0")/.." && pwd)}

form="check-tag: the tag '$tag' is not of the form <digits>.<digits>.<digits>"
case $tag in
  *[!0-9.]* | .* | *. | *..* | '') echo "$form" >&2; exit 1 ;;
esac
if [ "$(printf '%s' "$tag" | tr -cd '.' | wc -c | tr -d ' ')" != 2 ]; then
  echo "$form" >&2
  exit 1
fi

version=$(sed -n '/^\[package\]/,/^\[/{s/^version = "\(.*\)"$/\1/p}' "$here/Cargo.toml")
if [ "$tag" != "$version" ]; then
  echo "check-tag: the tag is $tag but the version in Cargo.toml is $version" >&2
  exit 1
fi

listed=$(awk -v v="$tag" '$1 == v { print $2 }' "$here/share/watchbill/revisions.txt")
if [ -z "$listed" ]; then
  echo "check-tag: share/watchbill/revisions.txt has no line for $tag" >&2
  exit 1
fi

computed=$(sha256sum "$here/share/watchbill/revision.sha256" | cut -d' ' -f1)
if [ "$listed" != "$computed" ]; then
  echo "check-tag: the hash of $tag in revisions.txt is $listed but the hash of the manifest is $computed" >&2
  exit 1
fi
echo "check-tag: $tag is the version, is in revisions.txt, and its hash is $computed" >&2
