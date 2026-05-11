#!/bin/sh
set -e

REPO="hermo/alea"
INSTALL_DIR="/usr/local/bin"

# Detect platform
OS=$(uname -s)
ARCH=$(uname -m)

case "${OS}_${ARCH}" in
  Darwin_arm64)  TARGET="aarch64-apple-darwin" ;;
  Linux_x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
  *)
    echo "error: unsupported platform: ${OS} ${ARCH}" >&2
    echo "supported: macOS ARM64, Linux x86_64" >&2
    exit 1
    ;;
esac

# Get latest release tag
LATEST=$(curl -sSf "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
if [ -z "$LATEST" ]; then
  echo "error: could not determine latest release" >&2
  exit 1
fi

ARCHIVE="alea-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/download/${LATEST}/${ARCHIVE}"
SHA_URL="${URL}.sha256"

echo "Installing alea ${LATEST} for ${TARGET}..."

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT

# Download binary and checksum
curl -sSfL "$URL" -o "${TMPDIR}/${ARCHIVE}"
curl -sSfL "$SHA_URL" -o "${TMPDIR}/${ARCHIVE}.sha256"

# Verify checksum
cd "$TMPDIR"
if command -v sha256sum >/dev/null 2>&1; then
  sha256sum -c "${ARCHIVE}.sha256"
elif command -v shasum >/dev/null 2>&1; then
  shasum -a 256 -c "${ARCHIVE}.sha256"
else
  echo "warning: no sha256sum or shasum found, skipping verification" >&2
fi

# Extract and install
tar xzf "$ARCHIVE"
chmod +x "alea-${LATEST#v}/alea"

if [ -w "$INSTALL_DIR" ]; then
  mv "alea-${LATEST#v}/alea" "$INSTALL_DIR/alea"
else
  echo "Installing to ${INSTALL_DIR} (requires sudo)..."
  sudo mv "alea-${LATEST#v}/alea" "$INSTALL_DIR/alea"
fi

echo "installed: $(${INSTALL_DIR}/alea --version 2>&1)"
