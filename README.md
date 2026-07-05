# RTON Editor

Rust/Dioxus editor for PopCap RTON resources. It targets both WebAssembly web
builds and native desktop builds from the same codebase.

This project is the Rust rewrite of the original React RTON editor, with the UI
and workflows ported toward a large-file friendly architecture.

## Features

- Open individual files or folders, including drag-and-drop folder import.
- Keep imported folders as a tree in the left file panel.
- Open files on demand instead of eagerly loading every imported file.
- Convert between RTON, JSON, YAML, and TOML.
- Decode and encode Compact RTON.
- Decode and export PvZ2 encrypted RTON.
- Edit text through a virtualized text editor designed for large documents.
- Inspect and edit bytes through a virtualized hex editor.
- Search text, hex, and parsed RTON values.
- Navigate from the value index to the matching text line or RTON byte offset.
- Select files in the file panel and batch export them.
- Undo and redo text and hex edits.
- Use light, dark, or system theme preferences.
- Use external Fluent `.ftl` files for localization.
- Build as a static web app or as native desktop binaries.

## Live Web Build

GitHub Pages builds are published by the release workflow:

```text
https://lambdaed1th.github.io/rton-editor/
```

## Workspace Layout

```text
rton-editor/
  app/             Dioxus UI, desktop/web platform glue, assets
  crates/core/     UI-free RTON editor core and worker protocol
  crates/worker/   Wasm worker exports used by the web app
```

## Requirements

- Rust stable
- `wasm32-unknown-unknown` Rust target for web builds
- Dioxus CLI 0.7.9 for serving and building the web app

```bash
rustup target add wasm32-unknown-unknown
cargo install dioxus-cli --version 0.7.9 --locked
```

Linux desktop builds need WebKit/GTK dependencies. On Ubuntu:

```bash
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  libayatana-appindicator3-dev \
  libgtk-3-dev \
  librsvg2-dev \
  libwebkit2gtk-4.1-dev \
  pkg-config
```

## Run

Run the web app during development:

```bash
dx serve --platform web --package rton-editor-app
```

Run the desktop app:

```bash
cargo run --package rton-editor-app --bin rton-editor
```

## Build

Build the web app:

```bash
dx build --platform web --package rton-editor-app --release --debug-symbols=false
```

For GitHub Pages or another subpath deployment, pass a base path:

```bash
dx build \
  --platform web \
  --package rton-editor-app \
  --release \
  --debug-symbols=false \
  --base-path /rton-editor/
```

The current Dioxus release output is written under:

```text
target/dx/rton-editor/release/web/public
```

Build a native release binary:

```bash
cargo build --release --package rton-editor-app --bin rton-editor
```

Build native bundles with `cargo-bundle`:

```bash
cargo install cargo-bundle --locked --version 0.10.0
cargo bundle --release --package rton-editor-app --bin rton-editor --format osx
cargo bundle --release --package rton-editor-app --bin rton-editor --format deb
```

## Localization

Localization is externalized through Fluent files. Built-in locale files live in:

```text
app/assets/i18n/en-US.ftl
app/assets/i18n/zh-CN.ftl
app/assets/i18n/fr-FR.ftl
app/assets/i18n/ru-RU.ftl
app/assets/i18n/es-ES.ftl
```

Users can add or edit `.ftl` files directly. Language names are read from each
locale file through `language-self`, so custom languages can display their own
native name in the language menu.

Desktop builds look for an external `assets/i18n/` directory at startup. During
development, `app/assets/i18n/` is used. Packaged apps should ship the same
directory next to the executable, or inside the platform bundle where the
release workflow places it.

Web builds load locale files from the static `assets/i18n/` directory exposed by
the server. If the server exposes directory listings, additional `.ftl` files can
be discovered without changing Rust code.

Missing locale files or missing translation keys fall back to English when
`en-US.ftl` is available.

## Release Workflow

The release workflow runs when a tag matching `v*` is pushed. It builds:

```text
rton-editor-linux-x86_64-vX.Y.Z-deb.zip
rton-editor-linux-aarch64-vX.Y.Z-deb.zip
rton-editor-macos-aarch64-app-vX.Y.Z.zip
rton-editor-windows-x86_64-vX.Y.Z.zip
rton-editor-windows-aarch64-vX.Y.Z.zip
rton-editor-web-vX.Y.Z.zip
```

It also publishes a GitHub Release and deploys the web build to GitHub Pages.

## Checks

Run the same checks used by CI:

```bash
cargo fmt --all --check
cargo check --workspace --all-targets
cargo check --workspace --target wasm32-unknown-unknown
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
dx build --platform web --package rton-editor-app --release --debug-symbols=false
```

## Notes

- The core editor logic is kept in `rton_editor_core` so it can be reused by the
  desktop app, web app, and wasm worker.
- The web build uses static assets under `app/assets/`, including the worker
  wrapper in `app/assets/worker/rton-worker.js`.
- The UI is optimized for large files with virtual scrolling and deferred
  parsing/conversion paths, but browser memory limits still apply to web builds.

## License

AGPL-3.0-or-later.
