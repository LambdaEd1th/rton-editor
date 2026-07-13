#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

./scripts/build-web-workers.sh
exec dx serve \
  --package rton-editor-app \
  --platform web \
  --cross-origin-policy \
  "$@"
