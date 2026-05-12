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

# Update version in main.c
sed "s/^#define VERSION \".*\"/#define VERSION \"${VERSION}\"/" src/main.c > src/main.c.tmp && mv src/main.c.tmp src/main.c

# Update version in man page
sed "s/^\.TH ALEA 1 .*/.TH ALEA 1 \"$(date +%Y-%m-%d)\" \"alea ${VERSION}\" \"User Commands\"/" alea.1 > alea.1.tmp && mv alea.1.tmp alea.1

make clean && make

git add src/main.c alea.1
git commit -m "Bump version to ${TAG}"
git tag "${TAG}"
git push origin main "${TAG}"

echo "released: ${TAG}"
