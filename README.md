# RTON Editor RS

Rust/Dioxus rewrite of the RTON editor for WebAssembly web builds and native
desktop builds.

## Layout

```text
rton-editor-rs/
  app/          Dioxus UI and platform file export glue
  crates/core/  UI-free RTON editor core
```

The core crate depends on the local `../serde_rton` checkout and provides:

- RTON decode/encode, compact encode, and PvZ2 encrypted RTON handling.
- JSON, YAML, and TOML text conversion.
- Hex text import/export for binary RTON inspection.
- Value statistics and a flattened value tree for the inspector panel.

## Current UI

The first Dioxus version keeps the original editor shape:

- Top toolbar with file open, format tabs, export controls, compact/encrypted
  switches, and validation.
- Drag files into the workspace to open them as tabs.
- Multiple open tabs with per-tab text, format, search, selection, and dirty
  state.
- Left open-file panel with tab activation and close controls.
- Center text/hex editor.
- Right RTON value inspector with stats, selected-node details, and search.
- Bottom status bar.

This is a functional baseline, not a line-by-line port of the React app yet.
Large-file remote indexing, the old virtualized hex editor, drag-reordering tabs,
folder import, and CodeMirror integration are intentionally left as follow-up
work.

## Requirements

- Rust stable with the `wasm32-unknown-unknown` target.
- Dioxus CLI for web serving and bundling:

```bash
cargo install dioxus-cli --locked
```

## Run Desktop

```bash
cd rton-editor-rs
cargo run -p rton-editor-rs-app
```

## Run Web

```bash
cd rton-editor-rs
dx serve --platform web
```

## Build Web

```bash
cd rton-editor-rs
dx build --platform web --release
```

The Dioxus project is configured to emit the static web build under `dist/`.

## Direct Checks

```bash
cd rton-editor-rs
cargo check -p rton_editor_core
cargo check -p rton-editor-rs-app
cargo check -p rton-editor-rs-app --target wasm32-unknown-unknown
```
