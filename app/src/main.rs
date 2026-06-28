use dioxus::prelude::*;
use dioxus_codemirror::{
    CodeMirror, Language as CodeMirrorLanguage, Theme as CodeMirrorTheme, ThemeColor, ThemeColors,
};
use dioxus_html::HasFileData;
use rton_editor_core::{
    BinaryEncoding, CoreError, DecodedDocument, EncodeOptions, SourceFormat, TREE_ROW_LIMIT,
    TextFormat, TreeRows, ValueRow, ValueStats, bytes_to_hex, decode_hex_rton, decode_rton_bytes,
    edit_value_at_path, encode_rton_bytes, flatten_value_tree, format_bytes, parse_text,
    scalar_edit_text, value_at_path, value_to_text,
};

mod platform;

const APP_CSS: &str = include_str!("../assets/style.css");

const SAMPLE_JSON: &str = r#"{
  "aliases": [
    "RTON Editor RS",
    "Dioxus"
  ],
  "project": "PvZ2 tooling",
  "enabled": true,
  "version": 1,
  "resource": "RTID(0)"
}"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorMode {
    RtonHex,
    Json,
    Yaml,
    Toml,
}

impl EditorMode {
    fn label(self) -> &'static str {
        match self {
            EditorMode::RtonHex => "RTON",
            EditorMode::Json => "JSON",
            EditorMode::Yaml => "YAML",
            EditorMode::Toml => "TOML",
        }
    }

    fn text_format(self) -> Option<TextFormat> {
        match self {
            EditorMode::RtonHex => None,
            EditorMode::Json => Some(TextFormat::Json),
            EditorMode::Yaml => Some(TextFormat::Yaml),
            EditorMode::Toml => Some(TextFormat::Toml),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tone {
    Info,
    Ok,
    Warn,
    Error,
}

impl Tone {
    fn class(self) -> &'static str {
        match self {
            Tone::Info => "info",
            Tone::Ok => "ok",
            Tone::Warn => "warn",
            Tone::Error => "error",
        }
    }
}

#[derive(Debug, Clone)]
struct Status {
    message: String,
    tone: Tone,
}

impl Status {
    fn new(message: impl Into<String>, tone: Tone) -> Self {
        Self {
            message: message.into(),
            tone,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct EditorTabState {
    id: usize,
    file_name: String,
    doc: Option<DecodedDocument>,
    editor_text: String,
    mode: EditorMode,
    search_query: String,
    selected_path: String,
    selected_edit_text: String,
    dirty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TabHeader {
    id: usize,
    file_name: String,
    mode: EditorMode,
    dirty: bool,
    closeable: bool,
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let initial_tab = create_text_tab(
        1,
        "sample.json".to_string(),
        SAMPLE_JSON.to_string(),
        TextFormat::Json,
    )
    .unwrap_or_else(|error| failed_tab(1, "sample.json".to_string(), error));
    let initial_editor_text = initial_tab.editor_text.clone();

    let mut tabs = use_signal(move || vec![initial_tab.clone()]);
    let mut active_tab_id = use_signal(|| 1_usize);
    let mut next_tab_id = use_signal(|| 2_usize);
    let mut compact_output = use_signal(|| false);
    let mut encrypt_output = use_signal(|| false);
    let mut dragging_files = use_signal(|| false);
    let mut light_theme = use_signal(|| false);
    let mut language = use_signal(|| "en".to_string());
    let mut line_wrapping = use_signal(|| false);
    let mut editor_search_panel_visible = use_signal(|| false);
    let mut editor_search_text = use_signal(|| String::new());
    let mut file_search_query = use_signal(|| String::new());
    let mut selected_tab_ids = use_signal(Vec::<usize>::new);
    let mut code_editor_text = use_signal(move || initial_editor_text.clone());
    let mut code_editor_programmatic_sync = use_signal(|| None::<(usize, String)>);
    let mut status = use_signal(|| Status::new("Ready", Tone::Ok));

    let tabs_snapshot = tabs.read().clone();
    let active_id_snapshot = *active_tab_id.read();
    let active_tab_snapshot = tabs_snapshot
        .iter()
        .find(|tab| tab.id == active_id_snapshot)
        .cloned()
        .or_else(|| tabs_snapshot.first().cloned());
    let active_doc_snapshot = active_tab_snapshot.as_ref().and_then(|tab| tab.doc.clone());
    let compact_snapshot = *compact_output.read();
    let encrypt_snapshot = *encrypt_output.read();
    let dragging_snapshot = *dragging_files.read();
    let light_theme_snapshot = *light_theme.read();
    let language_snapshot = language.read().clone();
    let line_wrapping_snapshot = *line_wrapping.read();
    let editor_search_panel_visible_snapshot = *editor_search_panel_visible.read();
    let editor_search_text_snapshot = editor_search_text.read().clone();
    let file_search_snapshot = file_search_query.read().clone();
    let selected_tab_ids_snapshot = selected_tab_ids.read().clone();
    let status_snapshot = status.read().clone();
    let tab_headers = tabs_snapshot
        .iter()
        .map(|tab| TabHeader {
            id: tab.id,
            file_name: tab.file_name.clone(),
            mode: tab.mode,
            dirty: tab.dirty,
            closeable: tabs_snapshot.len() > 1,
        })
        .collect::<Vec<_>>();
    let file_search_needle = file_search_snapshot.trim().to_ascii_lowercase();
    let filtered_tab_headers = if file_search_needle.is_empty() {
        tab_headers.clone()
    } else {
        tab_headers
            .iter()
            .filter(|tab| {
                tab.file_name
                    .to_ascii_lowercase()
                    .contains(&file_search_needle)
                    || tab
                        .mode
                        .label()
                        .to_ascii_lowercase()
                        .contains(&file_search_needle)
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    let visible_tab_ids_snapshot = filtered_tab_headers
        .iter()
        .map(|tab| tab.id)
        .collect::<Vec<_>>();
    let selected_file_count = selected_tab_ids_snapshot.len();
    let selected_visible_file_count = visible_tab_ids_snapshot
        .iter()
        .filter(|id| selected_tab_ids_snapshot.contains(id))
        .count();
    let file_list_subtitle = if file_search_needle.is_empty() {
        format!("{} open", tab_headers.len())
    } else {
        format!(
            "{} of {} visible",
            filtered_tab_headers.len(),
            tab_headers.len()
        )
    };
    let active_search = active_tab_snapshot
        .as_ref()
        .map(|tab| tab.search_query.clone())
        .unwrap_or_default();
    let selected_path_snapshot = active_tab_snapshot
        .as_ref()
        .map(|tab| tab.selected_path.clone())
        .unwrap_or_else(|| "$".to_string());
    let tree_rows = active_doc_snapshot
        .as_ref()
        .map(|doc| flatten_value_tree(&doc.value, &active_search, TREE_ROW_LIMIT))
        .unwrap_or_else(|| TreeRows {
            rows: Vec::new(),
            truncated: false,
        });
    let selected_row_snapshot = tree_rows
        .rows
        .iter()
        .find(|row| row.path == selected_path_snapshot)
        .cloned();
    let selected_edit_text_snapshot = active_tab_snapshot
        .as_ref()
        .map(|tab| tab.selected_edit_text.clone())
        .unwrap_or_default();
    let selected_editable_snapshot = selected_row_snapshot.is_some()
        && active_doc_snapshot
            .as_ref()
            .and_then(|doc| value_at_path(&doc.value, &selected_path_snapshot).ok())
            .is_some_and(|value| scalar_edit_text(value).is_some());

    use_effect(move || {
        let active_id = *active_tab_id.read();
        let next_text = tabs
            .read()
            .iter()
            .find(|tab| tab.id == active_id)
            .map(|tab| tab.editor_text.clone())
            .unwrap_or_default();

        if code_editor_text.peek().as_str() != next_text.as_str() {
            code_editor_programmatic_sync.set(Some((active_id, next_text.clone())));
            code_editor_text.set(next_text);
        }
    });

    use_effect(move || {
        let next_text = code_editor_text.read().clone();
        let active_id = *active_tab_id.peek();

        if code_editor_programmatic_sync
            .peek()
            .as_ref()
            .is_some_and(|(id, text)| *id == active_id && text == &next_text)
        {
            code_editor_programmatic_sync.set(None);
            return;
        }

        let current_text_matches = tabs
            .peek()
            .iter()
            .find(|tab| tab.id == active_id)
            .is_some_and(|tab| tab.editor_text == next_text);
        if !current_text_matches {
            update_active_text(next_text, tabs, active_tab_id);
        }
    });

    let parse_current = move |_| {
        validate_active_tab(tabs, active_tab_id, status);
    };

    let load_sample = move |_| {
        let id = *next_tab_id.read();
        next_tab_id.set(id + 1);
        match create_text_tab(
            id,
            format!("sample-{id}.json"),
            SAMPLE_JSON.to_string(),
            TextFormat::Json,
        ) {
            Ok(tab) => {
                let name = tab.file_name.clone();
                tabs.write().push(tab);
                active_tab_id.set(id);
                status.set(Status::new(format!("Sample tab opened: {name}"), Tone::Ok));
            }
            Err(error) => status.set(Status::new(error.to_string(), Tone::Error)),
        }
    };

    let activate_tab = move |id: usize| {
        if tabs.read().iter().any(|tab| tab.id == id) {
            active_tab_id.set(id);
            status.set(Status::new("Tab activated", Tone::Info));
        }
    };

    let close_tab = move |id: usize| {
        close_tab_by_id(id, tabs, active_tab_id, status);
        selected_tab_ids
            .write()
            .retain(|selected_id| *selected_id != id);
    };

    let switch_mode = move |next_mode: EditorMode| {
        switch_active_mode(next_mode, tabs, active_tab_id, status);
    };

    let select_all_visible_files = {
        let visible_tab_ids_snapshot = visible_tab_ids_snapshot.clone();
        move |_| {
            selected_tab_ids.set(visible_tab_ids_snapshot.clone());
            status.set(Status::new("Visible files selected", Tone::Info));
        }
    };

    let clear_selected_files = move |_| {
        selected_tab_ids.set(Vec::new());
        status.set(Status::new("File selection cleared", Tone::Info));
    };

    let toggle_selected_file = move |id: usize| {
        let mut selected = selected_tab_ids.write();
        if selected.contains(&id) {
            selected.retain(|selected_id| *selected_id != id);
        } else {
            selected.push(id);
        }
    };

    let mut batch_export_placeholder = move |format: &'static str| {
        status.set(Status::new(
            format!("Batch {format} export is not implemented yet in the Rust port"),
            Tone::Info,
        ));
    };

    let update_selected_edit = move |text: String| {
        update_selected_edit_text(text, tabs, active_tab_id);
    };

    let apply_selected_edit = move |_| {
        apply_selected_value_edit(tabs, active_tab_id, status);
    };

    let export_rton = move |_| {
        let Some(active_tab) = active_tab(tabs, active_tab_id) else {
            status.set(Status::new("No document to export", Tone::Warn));
            return;
        };
        let Some(doc) = active_tab.doc else {
            status.set(Status::new("No parsed document to export", Tone::Warn));
            return;
        };
        let options = EncodeOptions {
            encoding: if *compact_output.read() {
                BinaryEncoding::Compact
            } else {
                BinaryEncoding::Standard
            },
            encrypted: *encrypt_output.read(),
        };
        match encode_rton_bytes(&doc.value, options) {
            Ok(bytes) => {
                let default_name = export_rton_name(&active_tab.file_name, options);
                match platform::save_bytes(&default_name, &bytes) {
                    Ok(true) => status.set(Status::new(
                        format!("Exported {default_name} ({})", format_bytes(bytes.len())),
                        Tone::Ok,
                    )),
                    Ok(false) => status.set(Status::new("Export cancelled", Tone::Info)),
                    Err(error) => status.set(Status::new(error, Tone::Error)),
                }
            }
            Err(error) => status.set(Status::new(error.to_string(), Tone::Error)),
        }
    };

    let mut export_text = move |format: TextFormat| {
        let Some(active_tab) = active_tab(tabs, active_tab_id) else {
            status.set(Status::new("No document to export", Tone::Warn));
            return;
        };
        let Some(doc) = active_tab.doc else {
            status.set(Status::new("No parsed document to export", Tone::Warn));
            return;
        };
        match value_to_text(&doc.value, format) {
            Ok(text) => {
                let default_name = export_text_name(&active_tab.file_name, format);
                match platform::save_text(&default_name, &text) {
                    Ok(true) => {
                        status.set(Status::new(format!("Exported {default_name}"), Tone::Ok))
                    }
                    Ok(false) => status.set(Status::new("Export cancelled", Tone::Info)),
                    Err(error) => status.set(Status::new(error, Tone::Error)),
                }
            }
            Err(error) => status.set(Status::new(error.to_string(), Tone::Error)),
        }
    };

    rsx! {
        document::Stylesheet {
            href: asset!("/assets/tailwind.css")
        }
        style { {APP_CSS} }
        main {
            class: if light_theme_snapshot { "app-shell font-sans light-theme" } else { "app-shell font-sans" },
            header { class: "rton-toolbar",
                div { class: "rton-toolbar-row",
                    ToolbarGroup { label: "File",
                        div { class: "rton-toolbar-group",
                            label { class: button_class("primary"),
                                input {
                                    class: "file-input",
                                    r#type: "file",
                                    multiple: true,
                                    accept: ".rton,.dat,.json,.yaml,.yml,.toml",
                                    onchange: move |event| async move {
                                        let files = event.files();
                                        if files.is_empty() {
                                            return;
                                        }

                                        let mut opened = 0usize;
                                        for file in files {
                                            let name = file.name();
                                            match file.read_bytes().await {
                                                Ok(bytes) => {
                                                    let id = *next_tab_id.read();
                                                    next_tab_id.set(id + 1);
                                                    match create_tab_from_bytes(id, name.clone(), bytes.as_ref()) {
                                                        Ok(tab) => {
                                                            tabs.write().push(tab);
                                                            active_tab_id.set(id);
                                                            opened += 1;
                                                        }
                                                        Err(error) => status.set(Status::new(format!("{name}: {error}"), Tone::Error)),
                                                    }
                                                }
                                                Err(error) => status.set(Status::new(format!("Unable to read {name}: {error}"), Tone::Error)),
                                            }
                                        }

                                        if opened > 0 {
                                            status.set(Status::new(format!("Opened {opened} file(s)"), Tone::Ok));
                                        }
                                    }
                                }
                                span { class: "button-icon", "↑" }
                                span { "Open" }
                            }
                            button {
                                class: button_class("secondary"),
                                onclick: move |_| status.set(Status::new("Folder import is not implemented yet in the Rust port", Tone::Info)),
                                span { class: "button-icon", "□" }
                                "Folder"
                            }
                            button {
                                class: button_class("secondary"),
                                onclick: load_sample,
                                span { class: "button-icon", "S" }
                                "Sample"
                            }
                            span { class: "current-file",
                                {active_tab_snapshot.as_ref().map(|tab| tab.file_name.as_str()).unwrap_or("No file")}
                            }
                        }
                    }

                    ToolbarGroup { label: "Edit",
                        div { class: "rton-toolbar-group",
                            button {
                                class: button_class("secondary"),
                                disabled: true,
                                title: "Undo is not implemented yet",
                                span { class: "button-icon", "↶" }
                                "Undo"
                            }
                            button {
                                class: button_class("secondary"),
                                disabled: true,
                                title: "Redo is not implemented yet",
                                span { class: "button-icon", "↷" }
                                "Redo"
                            }
                        }
                    }

                    ToolbarGroup { label: "Format",
                        div { class: "rton-toolbar-group",
                            for mode in [EditorMode::RtonHex, EditorMode::Json, EditorMode::Yaml, EditorMode::Toml] {
                                button {
                                    class: mode_button_class(active_tab_snapshot.as_ref().is_some_and(|tab| tab.mode == mode)),
                                    disabled: active_tab_snapshot.is_none(),
                                    onclick: move |_| switch_mode(mode),
                                    "{mode.label()}"
                                }
                            }
                        }
                    }
                }

                div { class: "rton-toolbar-row",
                    ToolbarGroup { label: "Text export",
                        div { class: "rton-toolbar-group",
                            button {
                                class: button_class("secondary"),
                                disabled: active_doc_snapshot.is_none(),
                                onclick: move |_| export_text(TextFormat::Json),
                                span { class: "button-icon", "J" }
                                "JSON"
                            }
                            button {
                                class: button_class("secondary"),
                                disabled: active_doc_snapshot.is_none(),
                                onclick: move |_| export_text(TextFormat::Yaml),
                                span { class: "button-icon", "Y" }
                                "YAML"
                            }
                            button {
                                class: button_class("secondary"),
                                disabled: active_doc_snapshot.is_none(),
                                onclick: move |_| export_text(TextFormat::Toml),
                                span { class: "button-icon", "T" }
                                "TOML"
                            }
                        }
                    }

                    ToolbarGroup { label: "RTON export",
                        div { class: "rton-toolbar-group",
                            label { class: "rton-switch",
                                input {
                                    class: "rton-switch-input",
                                    r#type: "checkbox",
                                    checked: compact_snapshot,
                                    disabled: active_doc_snapshot.is_none(),
                                    onchange: move |event| compact_output.set(event.checked())
                                }
                                span { class: "rton-switch-label", "Compact" }
                                span { class: "rton-switch-track" }
                            }
                            label { class: "rton-switch",
                                input {
                                    class: "rton-switch-input",
                                    r#type: "checkbox",
                                    checked: encrypt_snapshot,
                                    disabled: active_doc_snapshot.is_none(),
                                    onchange: move |event| encrypt_output.set(event.checked())
                                }
                                span { class: "rton-switch-label", "Encrypted" }
                                span { class: "rton-switch-track" }
                            }
                            button {
                                class: button_class("secondary"),
                                disabled: active_tab_snapshot.is_none(),
                                onclick: parse_current,
                                span { class: "button-icon", "✓" }
                                "Validate"
                            }
                            button {
                                class: button_class("primary"),
                                disabled: active_doc_snapshot.is_none(),
                                onclick: export_rton,
                                span { class: "button-icon", "↓" }
                                "RTON"
                            }
                        }
                    }

                    ToolbarGroup { label: "Preferences",
                        div { class: "rton-toolbar-group",
                            label { class: "rton-theme-label",
                                span { "Theme" }
                                select {
                                    class: "rton-theme-select",
                                    value: if light_theme_snapshot { "light" } else { "dark" },
                                    onchange: move |event| light_theme.set(event.value() == "light"),
                                    option { value: "dark", selected: !light_theme_snapshot, "Dark" }
                                    option { value: "light", selected: light_theme_snapshot, "Light" }
                                }
                            }
                            label { class: "rton-theme-label",
                                span { "Language" }
                                select {
                                    class: "rton-theme-select",
                                    value: "{language_snapshot}",
                                    onchange: move |event| language.set(event.value()),
                                    option { value: "en", selected: language_snapshot == "en", "English" }
                                    option { value: "zh-CN", selected: language_snapshot == "zh-CN", "中文" }
                                }
                            }
                            label { class: "rton-switch",
                                input {
                                    class: "rton-switch-input",
                                    r#type: "checkbox",
                                    checked: line_wrapping_snapshot,
                                    onchange: move |event| line_wrapping.set(event.checked())
                                }
                                span { class: "rton-switch-label", "Wrap" }
                                span { class: "rton-switch-track" }
                            }
                            label { class: "rton-switch",
                                input {
                                    class: "rton-switch-input",
                                    r#type: "checkbox",
                                    checked: editor_search_panel_visible_snapshot,
                                    disabled: active_tab_snapshot.is_none(),
                                    onchange: move |event| editor_search_panel_visible.set(event.checked())
                                }
                                span { class: "rton-switch-label", "Search" }
                                span { class: "rton-switch-track" }
                            }
                        }
                    }
                }
            }

            div {
                class: if dragging_snapshot { "rton-workspace-shell dragging-files" } else { "rton-workspace-shell" },
                style: "--rton-left-panel-width: 260px; --rton-right-panel-width: 380px;",
                ondragover: move |event| {
                    event.prevent_default();
                    dragging_files.set(true);
                },
                ondragleave: move |_| {
                    dragging_files.set(false);
                },
                ondrop: move |event| async move {
                    event.prevent_default();
                    dragging_files.set(false);
                    let files = event.files();
                    if files.is_empty() {
                        return;
                    }

                    let mut opened = 0usize;
                    for file in files {
                        let name = file.name();
                        match file.read_bytes().await {
                            Ok(bytes) => {
                                let id = *next_tab_id.read();
                                next_tab_id.set(id + 1);
                                match create_tab_from_bytes(id, name.clone(), bytes.as_ref()) {
                                    Ok(tab) => {
                                        tabs.write().push(tab);
                                        active_tab_id.set(id);
                                        opened += 1;
                                    }
                                    Err(error) => status.set(Status::new(format!("{name}: {error}"), Tone::Error)),
                                }
                            }
                            Err(error) => status.set(Status::new(format!("Unable to read {name}: {error}"), Tone::Error)),
                        }
                    }

                    if opened > 0 {
                        status.set(Status::new(format!("Dropped {opened} file(s)"), Tone::Ok));
                    }
                },
                TabStrip {
                    tabs: tab_headers.clone(),
                    active_tab_id: active_id_snapshot,
                    on_activate: activate_tab,
                    on_close: close_tab
                }

                section { class: "rton-main-content",
                    aside { class: "rton-side-panel rton-side-panel-left",
                        PanelHeader {
                            icon: "▣",
                            title: "Files",
                            subtitle: file_list_subtitle.clone()
                        }
                        div { class: "file-panel-actions",
                            div { class: "file-selection-actions",
                                button {
                                    class: button_class("secondary"),
                                    disabled: filtered_tab_headers.is_empty(),
                                    onclick: select_all_visible_files,
                                    span { class: "button-icon", "✓" }
                                    "All"
                                }
                                button {
                                    class: button_class("secondary"),
                                    disabled: selected_visible_file_count == 0,
                                    onclick: clear_selected_files,
                                    span { class: "button-icon", "□" }
                                    "None"
                                }
                            }
                            div { class: "batch-export-grid",
                                for format in ["RTON", "JSON", "YAML", "TOML"] {
                                    button {
                                        class: if format == "RTON" { button_class("primary") } else { button_class("secondary") },
                                        disabled: selected_file_count == 0,
                                        onclick: move |_| batch_export_placeholder(format),
                                        "{format}"
                                    }
                                }
                            }
                        }
                        div { class: "file-search-box",
                            div { class: "file-search-inner",
                                span { class: "search-icon", "⌕" }
                                input {
                                    r#type: "search",
                                    placeholder: "Search files",
                                    value: "{file_search_snapshot}",
                                    disabled: tabs_snapshot.is_empty(),
                                    oninput: move |event| file_search_query.set(event.value())
                                }
                                if !file_search_snapshot.is_empty() {
                                    button {
                                        class: "file-search-clear",
                                        title: "Clear search",
                                        onclick: move |_| file_search_query.set(String::new()),
                                        "×"
                                    }
                                }
                            }
                        }
                        FileList {
                            tabs: filtered_tab_headers.clone(),
                            active_tab_id: active_id_snapshot,
                            selected_tab_ids: selected_tab_ids_snapshot,
                            on_activate: activate_tab,
                            on_close: close_tab,
                            on_toggle_selected: toggle_selected_file
                        }
                    }

                    div { class: "rton-resize-handle rton-resize-handle-left" }

                    section { class: "rton-editor-stage",
                        if let Some(active_tab) = active_tab_snapshot.as_ref() {
                            div {
                                key: "{active_tab.id}-{active_tab.mode.label()}-{line_wrapping_snapshot}",
                                class: "editor-surface",
                                if editor_search_panel_visible_snapshot {
                                    div { class: "editor-search-panel",
                                        input {
                                            class: "editor-search-field",
                                            r#type: "search",
                                            placeholder: "Find",
                                            value: "{editor_search_text_snapshot}",
                                            oninput: move |event| editor_search_text.set(event.value())
                                        }
                                        button { class: button_class("secondary"), disabled: true, "Previous" }
                                        button { class: button_class("secondary"), disabled: true, "Next" }
                                        label { class: "editor-search-check",
                                            input { r#type: "checkbox", disabled: true }
                                            span { "Case" }
                                        }
                                        span { class: "editor-search-status", "0 matches" }
                                        button {
                                            class: "editor-search-close",
                                            title: "Close search",
                                            onclick: move |_| editor_search_panel_visible.set(false),
                                            "×"
                                        }
                                    }
                                }
                                div { class: "codemirror-shell",
                                    CodeMirror {
                                        value: code_editor_text,
                                        language: codemirror_language(active_tab.mode),
                                        theme: codemirror_theme(light_theme_snapshot),
                                        theme_colors: codemirror_theme_colors(),
                                        line_numbers: true,
                                        line_wrapping: line_wrapping_snapshot,
                                        allow_multiple_selections: true,
                                        bracket_matching: true,
                                        close_brackets: true,
                                        code_folding: true,
                                        highlight_active_line: true,
                                        highlight_selection_matches: true,
                                        indent_on_input: true,
                                        indent_with_tab: true,
                                        tab_size: Some(2),
                                    }
                                }
                            }
                        } else {
                            div { class: "rton-empty-drop-stage",
                                div { class: "empty-editor-title", "Open a file to start editing." }
                                div { class: "empty-editor-subtitle", "Drop .rton, .dat, JSON, YAML, or TOML files here." }
                            }
                        }
                    }

                    div { class: "rton-resize-handle rton-resize-handle-right" }

                    aside { class: "rton-side-panel rton-side-panel-right",
                        div { class: "rton-inspector-summary",
                            PanelHeader {
                                icon: "▣",
                                title: "File properties",
                                subtitle: "Current file".to_string()
                            }
                            dl { class: "meta-list",
                                MetaItem {
                                    label: "Name",
                                    value: active_tab_snapshot.as_ref().map(|tab| tab.file_name.clone()).unwrap_or_else(|| "No file".to_string())
                                }
                                MetaItem {
                                    label: "Input",
                                    value: active_tab_snapshot.as_ref().map(|tab| tab.mode.label().to_string()).unwrap_or_else(|| "-".to_string())
                                }
                                MetaItem {
                                    label: "Output",
                                    value: if active_doc_snapshot.is_some() { "Ready".to_string() } else { "-".to_string() }
                                }
                            }
                            if let Some(doc) = active_doc_snapshot.as_ref() {
                                div { class: "stats-title",
                                    span { class: "panel-header-icon muted", "∑" }
                                    h2 { "Stats" }
                                }
                                StatsGrid { stats: doc.stats.clone() }
                            }
                        }

                        div { class: "rton-index-panel",
                            div { class: "rton-index-header",
                                div { class: "rton-index-title",
                                    div { class: "rton-index-title-line",
                                        span { class: "panel-header-icon", "☷" }
                                        h2 { "Inspector" }
                                    }
                                    p { "RtonValue" }
                                }
                                div { class: "rton-index-search",
                                    span { class: "search-icon", "⌕" }
                                    input {
                                        r#type: "search",
                                        placeholder: "Search",
                                        value: "{active_search}",
                                        disabled: active_doc_snapshot.is_none(),
                                        oninput: move |event| update_active_search(event.value(), tabs, active_tab_id)
                                    }
                                }
                            }
                            if active_doc_snapshot.is_some() {
                                SelectedValueDetails {
                                    row: selected_row_snapshot,
                                    edit_text: selected_edit_text_snapshot,
                                    editable: selected_editable_snapshot,
                                    on_edit: update_selected_edit,
                                    on_apply: apply_selected_edit
                                }
                                ValueTree {
                                    rows: tree_rows,
                                    selected_path: selected_path_snapshot,
                                    on_select: move |path| update_selected_path(path, tabs, active_tab_id)
                                }
                            } else {
                                div { class: "empty-state",
                                    strong { "No RTON value" }
                                    p { "Open a file or paste text, then validate it." }
                                }
                            }
                        }
                    }
                }
            }

            footer { class: "status-bar {status_snapshot.tone.class()}",
                span { class: "status-file",
                    {active_tab_snapshot.as_ref().map(|tab| tab.file_name.as_str()).unwrap_or("No file")}
                }
                span { class: "status-message", "{status_snapshot.message}" }
                span { class: "status-output-label", "Output" }
                span { class: "status-output-value",
                    {if active_doc_snapshot.is_some() { "Ready" } else { "-" }}
                }
            }
        }
    }
}

#[component]
fn ToolbarGroup(label: &'static str, children: Element) -> Element {
    rsx! {
        div { class: "rton-toolbar-group-shell",
            button {
                class: "rton-toolbar-group-drag-handle",
                title: "Move {label}",
                "⋮"
            }
            {children}
        }
    }
}

#[component]
fn PanelHeader(icon: &'static str, title: &'static str, subtitle: String) -> Element {
    rsx! {
        header { class: "panel-header",
            div { class: "panel-header-main",
                div { class: "panel-header-title-line",
                    span { class: "panel-header-icon", "{icon}" }
                    h2 { "{title}" }
                }
                p { "{subtitle}" }
            }
        }
    }
}

#[component]
fn MetaItem(label: &'static str, value: String) -> Element {
    rsx! {
        div { class: "meta-item",
            dt { "{label}" }
            dd { "{value}" }
        }
    }
}

#[component]
fn FileList(
    tabs: Vec<TabHeader>,
    active_tab_id: usize,
    selected_tab_ids: Vec<usize>,
    on_activate: EventHandler<usize>,
    on_close: EventHandler<usize>,
    on_toggle_selected: EventHandler<usize>,
) -> Element {
    rsx! {
        div { class: "file-list",
            if tabs.is_empty() {
                div { class: "file-list-empty", "No matching files" }
            } else {
                for tab in tabs {
                    div { class: if tab.id == active_tab_id { "file-item active" } else { "file-item" },
                        label { class: "file-item-check",
                            input {
                                r#type: "checkbox",
                                checked: selected_tab_ids.contains(&tab.id),
                                aria_label: "Select {tab.file_name}",
                                onclick: move |event| event.stop_propagation(),
                                onchange: move |_| on_toggle_selected.call(tab.id)
                            }
                        }
                        button {
                            class: "file-item-main",
                            onclick: move |_| on_activate.call(tab.id),
                            span { class: "file-name",
                                "{leaf_display_name(&tab.file_name)}"
                                if tab.dirty {
                                    span { class: "file-dirty", "*" }
                                }
                            }
                            span { class: "file-detail", "{tab.mode.label()}" }
                        }
                        button {
                            class: "file-item-close",
                            disabled: !tab.closeable,
                            title: "Close tab",
                            onclick: move |_| on_close.call(tab.id),
                            "×"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TabStrip(
    tabs: Vec<TabHeader>,
    active_tab_id: usize,
    on_activate: EventHandler<usize>,
    on_close: EventHandler<usize>,
) -> Element {
    rsx! {
        nav { class: "rton-tab-strip",
            div { class: "rton-file-tabs",
                for tab in tabs {
                    div { class: if tab.id == active_tab_id { "rton-file-tab active" } else { "rton-file-tab" },
                        button {
                            class: "rton-file-tab-label",
                            onclick: move |_| on_activate.call(tab.id),
                            title: "{tab.file_name}",
                            span { "{leaf_display_name(&tab.file_name)}" }
                            if tab.dirty {
                                span { class: "file-dirty", "*" }
                            }
                        }
                        button {
                            class: "rton-file-tab-close",
                            disabled: !tab.closeable,
                            title: "Close tab",
                            onclick: move |_| on_close.call(tab.id),
                            "×"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatsGrid(stats: ValueStats) -> Element {
    rsx! {
        div { class: "stats-grid",
            StatCell { label: "Nodes", value: stats.nodes }
            StatCell { label: "Objects", value: stats.objects }
            StatCell { label: "Arrays", value: stats.arrays }
            StatCell { label: "Strings", value: stats.strings }
            StatCell { label: "Numbers", value: stats.numbers }
            StatCell { label: "Depth", value: stats.max_depth }
        }
    }
}

#[component]
fn SelectedValueDetails(
    row: Option<ValueRow>,
    edit_text: String,
    editable: bool,
    on_edit: EventHandler<String>,
    on_apply: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "selected-value-panel",
            if let Some(row) = row {
                div { class: "selected-value-title",
                    span { "{row.label}" }
                    code { "{row.kind}" }
                }
                dl { class: "selected-value-meta",
                    dt { "Path" }
                    dd { "{row.path}" }
                    dt { "Preview" }
                    dd { "{row.preview}" }
                    if row.child_count > 0 {
                        dt { "Children" }
                        dd { "{row.child_count}" }
                    }
                }
                if editable {
                    div { class: "selected-value-edit",
                        input {
                            r#type: "text",
                            value: "{edit_text}",
                            oninput: move |event| on_edit.call(event.value())
                        }
                        button {
                            class: "button compact",
                            onclick: move |_| on_apply.call(()),
                            "Apply"
                        }
                    }
                }
            } else {
                div { class: "selected-value-empty", "Select a visible value to inspect it." }
            }
        }
    }
}

#[component]
fn StatCell(label: &'static str, value: usize) -> Element {
    rsx! {
        div { class: "stat-cell",
            span { "{label}" }
            strong { "{value}" }
        }
    }
}

#[component]
fn ValueTree(rows: TreeRows, selected_path: String, on_select: EventHandler<String>) -> Element {
    rsx! {
        div { class: "value-tree",
            if rows.rows.is_empty() {
                div { class: "empty-state compact",
                    "No matching values"
                }
            }
            for row in rows.rows {
                ValueTreeRow {
                    selected: row.path == selected_path,
                    row,
                    on_select
                }
            }
            if rows.truncated {
                div { class: "tree-truncated",
                    "Results truncated. Refine the search to narrow the tree."
                }
            }
        }
    }
}

#[component]
fn ValueTreeRow(row: ValueRow, selected: bool, on_select: EventHandler<String>) -> Element {
    let path = row.path.clone();
    let class = if selected {
        "value-row selected"
    } else {
        "value-row"
    };
    let indent = 10 + row.depth * 16;

    rsx! {
        button {
            class,
            style: "padding-left: {indent}px",
            onclick: move |_| on_select.call(path.clone()),
            span { class: "row-label", "{row.label}" }
            span { class: "row-kind", "{row.kind}" }
            span { class: "row-preview", "{row.preview}" }
            if row.child_count > 0 {
                span { class: "row-count", "{row.child_count}" }
            }
        }
    }
}

fn validate_active_tab(
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
    mut status: Signal<Status>,
) {
    let Some(tab) = active_tab(tabs, active_tab_id) else {
        status.set(Status::new("No active tab", Tone::Warn));
        return;
    };

    match parse_editor_text(&tab.editor_text, tab.mode) {
        Ok(doc) => {
            let selected_edit_text = selected_edit_text_for_doc(&doc, "$");
            update_tab(tabs, tab.id, |active| {
                active.doc = Some(doc);
                active.selected_path = "$".to_string();
                active.selected_edit_text = selected_edit_text;
                active.dirty = false;
            });
            status.set(Status::new("Document parsed successfully", Tone::Ok));
        }
        Err(error) => status.set(Status::new(error.to_string(), Tone::Error)),
    }
}

fn switch_active_mode(
    next_mode: EditorMode,
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
    mut status: Signal<Status>,
) {
    let Some(tab) = active_tab(tabs, active_tab_id) else {
        return;
    };
    if tab.mode == next_mode {
        return;
    }

    let doc = if tab.dirty {
        match parse_editor_text(&tab.editor_text, tab.mode) {
            Ok(doc) => doc,
            Err(error) => {
                status.set(Status::new(
                    format!("Cannot switch view until current text parses: {error}"),
                    Tone::Error,
                ));
                return;
            }
        }
    } else {
        let Some(doc) = tab.doc else {
            status.set(Status::new("No parsed document for this tab", Tone::Warn));
            return;
        };
        doc
    };

    match text_for_document(&doc, next_mode) {
        Ok(text) => {
            let selected_edit_text = selected_edit_text_for_doc(&doc, "$");
            update_tab(tabs, tab.id, |active| {
                active.doc = Some(doc);
                active.editor_text = text;
                active.mode = next_mode;
                active.selected_path = "$".to_string();
                active.selected_edit_text = selected_edit_text;
                active.dirty = false;
            });
            status.set(Status::new(
                format!("Switched to {}", next_mode.label()),
                Tone::Info,
            ));
        }
        Err(error) => status.set(Status::new(error.to_string(), Tone::Error)),
    }
}

fn close_tab_by_id(
    id: usize,
    mut tabs: Signal<Vec<EditorTabState>>,
    mut active_tab_id: Signal<usize>,
    mut status: Signal<Status>,
) {
    let (closed_name, next_active) = {
        let mut items = tabs.write();
        if items.len() <= 1 {
            status.set(Status::new("Keep at least one tab open", Tone::Warn));
            return;
        }

        let Some(index) = items.iter().position(|tab| tab.id == id) else {
            return;
        };
        let closed_name = items[index].file_name.clone();
        let was_active = items[index].id == *active_tab_id.read();
        items.remove(index);
        let next_active = if was_active {
            let next_index = index.min(items.len().saturating_sub(1));
            Some(items[next_index].id)
        } else {
            None
        };
        (closed_name, next_active)
    };

    if let Some(next_active) = next_active {
        active_tab_id.set(next_active);
    }
    status.set(Status::new(format!("Closed {closed_name}"), Tone::Info));
}

fn update_active_text(
    text: String,
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
) {
    let active_id = *active_tab_id.read();
    update_tab(tabs, active_id, |tab| {
        tab.editor_text = text;
        tab.dirty = true;
    });
}

fn update_active_search(
    query: String,
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
) {
    let active_id = *active_tab_id.read();
    update_tab(tabs, active_id, |tab| {
        tab.search_query = query;
    });
}

fn update_selected_path(
    path: String,
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
) {
    let active_id = *active_tab_id.read();
    update_tab(tabs, active_id, |tab| {
        let selected_edit_text = tab
            .doc
            .as_ref()
            .map(|doc| selected_edit_text_for_doc(doc, &path))
            .unwrap_or_default();
        tab.selected_path = path;
        tab.selected_edit_text = selected_edit_text;
    });
}

fn update_selected_edit_text(
    text: String,
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
) {
    let active_id = *active_tab_id.read();
    update_tab(tabs, active_id, |tab| {
        tab.selected_edit_text = text;
    });
}

fn apply_selected_value_edit(
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
    mut status: Signal<Status>,
) {
    let Some(tab) = active_tab(tabs, active_tab_id) else {
        status.set(Status::new("No active tab", Tone::Warn));
        return;
    };
    let Some(mut doc) = tab.doc else {
        status.set(Status::new("No parsed document to edit", Tone::Warn));
        return;
    };

    let path = tab.selected_path.clone();
    match edit_value_at_path(&mut doc.value, &path, &tab.selected_edit_text) {
        Ok(()) => {
            doc.stats = ValueStats::from_value(&doc.value);
            match text_for_document(&doc, tab.mode) {
                Ok(text) => {
                    let selected_edit_text = selected_edit_text_for_doc(&doc, &path);
                    update_tab(tabs, tab.id, |active| {
                        active.doc = Some(doc);
                        active.editor_text = text;
                        active.selected_path = path.clone();
                        active.selected_edit_text = selected_edit_text;
                        active.dirty = true;
                    });
                    status.set(Status::new(format!("Updated {path}"), Tone::Ok));
                }
                Err(error) => status.set(Status::new(error.to_string(), Tone::Error)),
            }
        }
        Err(error) => status.set(Status::new(error.to_string(), Tone::Error)),
    }
}

fn update_tab(
    mut tabs: Signal<Vec<EditorTabState>>,
    id: usize,
    update: impl FnOnce(&mut EditorTabState),
) {
    if let Some(tab) = tabs.write().iter_mut().find(|tab| tab.id == id) {
        update(tab);
    }
}

fn active_tab(
    tabs: Signal<Vec<EditorTabState>>,
    active_tab_id: Signal<usize>,
) -> Option<EditorTabState> {
    let active_id = *active_tab_id.read();
    tabs.read().iter().find(|tab| tab.id == active_id).cloned()
}

fn create_tab_from_bytes(
    id: usize,
    name: String,
    bytes: &[u8],
) -> Result<EditorTabState, CoreError> {
    match SourceFormat::from_file_name(&name) {
        SourceFormat::Rton | SourceFormat::Unknown => {
            let doc = decode_rton_bytes(bytes)?;
            let selected_edit_text = selected_edit_text_for_doc(&doc, "$");
            Ok(EditorTabState {
                id,
                file_name: name,
                doc: Some(doc),
                editor_text: bytes_to_hex(bytes),
                mode: EditorMode::RtonHex,
                search_query: String::new(),
                selected_path: "$".to_string(),
                selected_edit_text,
                dirty: false,
            })
        }
        SourceFormat::Json => create_text_tab(
            id,
            name,
            String::from_utf8_lossy(bytes).to_string(),
            TextFormat::Json,
        ),
        SourceFormat::Yaml => create_text_tab(
            id,
            name,
            String::from_utf8_lossy(bytes).to_string(),
            TextFormat::Yaml,
        ),
        SourceFormat::Toml => create_text_tab(
            id,
            name,
            String::from_utf8_lossy(bytes).to_string(),
            TextFormat::Toml,
        ),
    }
}

fn create_text_tab(
    id: usize,
    file_name: String,
    text: String,
    format: TextFormat,
) -> Result<EditorTabState, CoreError> {
    let doc = parse_text(&text, format)?;
    let selected_edit_text = selected_edit_text_for_doc(&doc, "$");
    let mode = match format {
        TextFormat::Json => EditorMode::Json,
        TextFormat::Yaml => EditorMode::Yaml,
        TextFormat::Toml => EditorMode::Toml,
    };
    Ok(EditorTabState {
        id,
        file_name,
        doc: Some(doc),
        editor_text: text,
        mode,
        search_query: String::new(),
        selected_path: "$".to_string(),
        selected_edit_text,
        dirty: false,
    })
}

fn failed_tab(id: usize, file_name: String, error: CoreError) -> EditorTabState {
    EditorTabState {
        id,
        file_name,
        doc: None,
        editor_text: error.to_string(),
        mode: EditorMode::Json,
        search_query: String::new(),
        selected_path: "$".to_string(),
        selected_edit_text: String::new(),
        dirty: true,
    }
}

fn parse_editor_text(text: &str, mode: EditorMode) -> Result<DecodedDocument, CoreError> {
    match mode {
        EditorMode::RtonHex => decode_hex_rton(text),
        EditorMode::Json => parse_text(text, TextFormat::Json),
        EditorMode::Yaml => parse_text(text, TextFormat::Yaml),
        EditorMode::Toml => parse_text(text, TextFormat::Toml),
    }
}

fn text_for_document(doc: &DecodedDocument, mode: EditorMode) -> Result<String, CoreError> {
    match mode {
        EditorMode::RtonHex => {
            let bytes = encode_rton_bytes(&doc.value, EncodeOptions::default())?;
            Ok(bytes_to_hex(&bytes))
        }
        EditorMode::Json | EditorMode::Yaml | EditorMode::Toml => {
            let format = mode.text_format().expect("text mode has format");
            value_to_text(&doc.value, format)
        }
    }
}

fn selected_edit_text_for_doc(doc: &DecodedDocument, path: &str) -> String {
    value_at_path(&doc.value, path)
        .ok()
        .and_then(scalar_edit_text)
        .unwrap_or_default()
}

fn export_rton_name(source_name: &str, options: EncodeOptions) -> String {
    let stem = file_stem(source_name);
    let flavor = match (options.encoding, options.encrypted) {
        (BinaryEncoding::Standard, false) => "standard",
        (BinaryEncoding::Compact, false) => "compact",
        (BinaryEncoding::Standard, true) => "standard-encrypted",
        (BinaryEncoding::Compact, true) => "compact-encrypted",
    };
    format!("{stem}.{flavor}.rton")
}

fn export_text_name(source_name: &str, format: TextFormat) -> String {
    format!("{}.{}", file_stem(source_name), format.extension())
}

fn file_stem(source_name: &str) -> String {
    let file = source_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(source_name);
    file.rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or(file)
        .to_string()
}

fn leaf_display_name(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .filter(|part| !part.is_empty())
        .next_back()
        .unwrap_or(path)
        .to_string()
}

fn codemirror_language(mode: EditorMode) -> Option<CodeMirrorLanguage> {
    match mode {
        EditorMode::Json => Some(CodeMirrorLanguage::Javascript),
        EditorMode::Yaml => Some(CodeMirrorLanguage::Yaml),
        EditorMode::RtonHex | EditorMode::Toml => None,
    }
}

fn codemirror_theme(light_theme: bool) -> CodeMirrorTheme {
    if light_theme {
        CodeMirrorTheme::Light
    } else {
        CodeMirrorTheme::Dark
    }
}

fn codemirror_theme_colors() -> ThemeColors {
    ThemeColors {
        bg: cm_color("var(--color-stage)"),
        fg: cm_color("var(--color-text-strong)"),
        caret: cm_color("var(--color-accent-text)"),
        selection: cm_color("var(--color-editor-selection)"),
        selection_focused: cm_color("var(--color-editor-selection)"),
        selection_match: cm_color("var(--color-editor-search-match)"),
        selection_match_main: cm_color("var(--color-editor-search-current)"),
        gutter_bg: cm_color("var(--color-stage)"),
        gutter_fg: cm_color("var(--color-text-subtle)"),
        highlight_space: cm_color("var(--color-text-subtle)"),
        active_line: cm_color("var(--color-editor-active-line)"),
        active_line_gutter_bg: cm_color("var(--color-surface-soft)"),
        active_line_selected: cm_color("var(--color-editor-selection)"),
        border: cm_color("var(--color-border)"),
        tooltip_bg: cm_color("var(--color-surface-raised)"),
        tooltip_fg: cm_color("var(--color-text-strong)"),
        tooltip_selected_bg: cm_color("var(--color-control-hover)"),
        tooltip_selected_fg: cm_color("var(--color-accent-text)"),
        tooltip_info_bg: cm_color("var(--color-surface-soft)"),
        syntax_keyword: cm_color("var(--color-code-keyword)"),
        syntax_string: cm_color("var(--color-code-string)"),
        syntax_comment: cm_color("var(--color-code-comment)"),
        syntax_number: cm_color("var(--color-code-number)"),
        syntax_function: cm_color("var(--color-code-name)"),
        syntax_type: cm_color("var(--color-code-meta)"),
        syntax_constant: cm_color("var(--color-code-bool)"),
        syntax_operator: cm_color("var(--color-code-operator)"),
        syntax_property: cm_color("var(--color-code-property)"),
        syntax_heading: cm_color("var(--color-code-property)"),
        syntax_link: cm_color("var(--color-accent-text)"),
        syntax_invalid: cm_color("var(--color-error)"),
    }
}

fn cm_color(value: &'static str) -> ThemeColor {
    ThemeColor::new(value, value)
}

fn button_class(variant: &'static str) -> &'static str {
    match variant {
        "primary" => "rton-button primary",
        _ => "rton-button secondary",
    }
}

fn mode_button_class(active: bool) -> &'static str {
    if active {
        "rton-mode-button active"
    } else {
        "rton-mode-button"
    }
}
