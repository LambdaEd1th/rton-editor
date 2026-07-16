#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
worker_crate="$repo_root/crates/worker"
asset_root="$repo_root/app/assets/worker"

rm -rf "$asset_root/pkg" "$asset_root/single" "$asset_root/threaded"
mkdir -p "$asset_root/pkg"

RUSTUP_TOOLCHAIN=stable \
wasm-pack build "$worker_crate" \
  --target web \
  --release \
  --out-dir "$asset_root/pkg" \
  --out-name rton_editor_worker

rm -f "$asset_root/pkg/.gitignore"
