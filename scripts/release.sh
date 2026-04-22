#!/usr/bin/env bash
set -euo pipefail

VERSION=${1:-}

if [[ -z "$VERSION" ]]; then
  echo "Usage: ./release.sh <version>  (e.g. ./release.sh 0.0.3)"
  exit 1
fi

sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml

git add Cargo.toml
git commit -m "chore: bump version to $VERSION"
git tag "v$VERSION"
git push origin main
git push origin "v$VERSION"

echo "Released v$VERSION"
