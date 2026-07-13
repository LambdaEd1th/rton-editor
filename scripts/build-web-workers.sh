#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
worker_crate="$repo_root/crates/worker"
asset_root="$repo_root/app/assets/worker"

mkdir -p "$asset_root/single" "$asset_root/threaded"

wasm-pack build "$worker_crate" \
  --target web \
  --release \
  --out-dir "$asset_root/single" \
  --out-name rton_editor_worker

CARGO_TARGET_DIR="$repo_root/target/wasm-threaded" \
RUSTUP_TOOLCHAIN=nightly-2025-07-01 \
RUSTFLAGS='-C target-feature=+atomics,+bulk-memory' \
wasm-pack build "$worker_crate" \
  --target web \
  --release \
  --out-dir "$asset_root/threaded" \
  --out-name rton_editor_worker \
  -- \
  -Z build-std=panic_abort,std \
  --features wasm-threads

rm -f "$asset_root/single/.gitignore" "$asset_root/threaded/.gitignore"
node "$repo_root/scripts/verify-shared-worker.mjs" \
  "$asset_root/threaded/rton_editor_worker_bg.wasm"
