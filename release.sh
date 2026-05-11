#!/bin/sh
set -e

if [ -z "$1" ]; then
  echo "usage: $0 <version>" >&2
  echo "  example: $0 1.0.0" >&2
  exit 1
fi

VERSION="$1"
VERSION="${VERSION#v}"
TAG="v${VERSION}"

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "error: uncommitted changes — commit or stash them first" >&2
  exit 1
fi

awk -v ver="$VERSION" '{if(/^version = "/) print "version = \""ver"\""; else print}' Cargo.toml > Cargo.toml.tmp && mv Cargo.toml.tmp Cargo.toml
cargo update --workspace --quiet
git add Cargo.toml Cargo.lock
git commit -m "Bump version to ${TAG}"
git tag "${TAG}"
git push origin main "${TAG}"

echo "released: ${TAG}"
