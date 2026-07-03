# RTON Editor

Rust/Dioxus rewrite of the RTON editor for WebAssembly web builds and native
desktop builds.

## Layout

```text
rton-editor/
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
cd rton-editor
cargo run -p rton-editor-app
```

## Run Web

```bash
cd rton-editor
dx serve --platform web --package rton-editor-app
```

## Build Web

```bash
cd rton-editor
dx build --platform web --package rton-editor-app --release
```

The Dioxus project is configured to emit the static web build under `dist/`.

## Runtime i18n

Fluent files are loaded at startup instead of being embedded into the Rust
binary or wasm module. Locale files live in one external directory:

```text
app/assets/i18n/en-US.ftl
app/assets/i18n/zh-CN.ftl
app/assets/i18n/fr-FR.ftl
app/assets/i18n/ru-RU.ftl
app/assets/i18n/es-ES.ftl
```

Users can edit these files directly or add more `.ftl` files.

Desktop builds read one `assets/i18n/` directory at startup. During development
that is usually `app/assets/i18n/`; packaged apps can place the same directory
next to the executable:

```text
assets/i18n/de-DE.ftl
assets/i18n/ja-JP.ftl
```

Web builds read translations from the static server at startup. Files that
exist in `app/assets/i18n/` at build time are included in the generated i18n
manifest; additional files can be discovered from the server directory listing
when available:

```text
assets/i18n/de-DE.ftl
assets/i18n/ja-JP.ftl
```

When serving a release build, the same files should be placed under
`dist/assets/i18n/`. If the server does not expose directory listings, web
builds can still load the manifest files but cannot discover arbitrary new
locale files automatically. Missing translation keys fall back to English when
`en-US.ftl` is loaded.

## Direct Checks

```bash
cd rton-editor
cargo check -p rton_editor_core
cargo check -p rton-editor-app
cargo check -p rton-editor-app --target wasm32-unknown-unknown
```
