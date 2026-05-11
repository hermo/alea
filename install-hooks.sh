#!/bin/sh
set -e

REPO_ROOT=$(git rev-parse --show-toplevel)
ln -sf "$REPO_ROOT/hooks/pre-commit" "$REPO_ROOT/.git/hooks/pre-commit"
echo "installed: .git/hooks/pre-commit"
