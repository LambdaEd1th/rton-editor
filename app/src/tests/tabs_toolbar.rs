use super::*;

#[test]
fn reorders_tab_after_target() {
    let mut tabs = vec![tab(1), tab(2), tab(3)];

    reorder_tabs_by_id(&mut tabs, 1, 2, DropPlacement::After);

    assert_eq!(tab_ids(&tabs), vec![2, 1, 3]);
}

#[test]
fn reorders_tab_before_target() {
    let mut tabs = vec![tab(1), tab(2), tab(3)];

    reorder_tabs_by_id(&mut tabs, 3, 1, DropPlacement::Before);

    assert_eq!(tab_ids(&tabs), vec![3, 1, 2]);
}

#[test]
fn clamps_panel_width_to_original_bounds() {
    assert_eq!(clamp_panel_width(120.0), PANEL_MIN_WIDTH);
    assert_eq!(clamp_panel_width(333.4), 333);
    assert_eq!(clamp_panel_width(620.0), PANEL_MAX_WIDTH);
}

#[test]
fn maps_theme_preferences_to_shell_and_editor_themes() {
    assert_eq!(
        ThemePreference::from_code("system"),
        ThemePreference::System
    );
    assert_eq!(ThemePreference::from_code("light"), ThemePreference::Light);
    assert_eq!(ThemePreference::from_code("dark"), ThemePreference::Dark);
    assert_eq!(
        ThemePreference::from_code("unknown"),
        ThemePreference::System
    );

    assert_eq!(
        ThemePreference::System.shell_class(),
        "app-shell font-sans system-theme"
    );
}

#[test]
fn normalizes_toolbar_rows_and_appends_missing_groups() {
    let rows = normalize_toolbar_rows(Some(
        r#"[["prefs","file","file","unknown"],["format"],["edit"]]"#,
    ));

    assert_eq!(
        rows,
        vec![
            vec![ToolbarGroupId::Preferences, ToolbarGroupId::File],
            vec![
                ToolbarGroupId::Format,
                ToolbarGroupId::Edit,
                ToolbarGroupId::TextExport,
                ToolbarGroupId::RtonExport,
                ToolbarGroupId::About,
            ],
        ]
    );
    assert_eq!(
        normalize_toolbar_rows(Some("not json")),
        default_toolbar_rows()
    );
}

#[test]
fn moves_toolbar_groups_across_rows() {
    let mut rows = default_toolbar_rows();

    move_toolbar_group(
        &mut rows,
        ToolbarGroupId::Preferences,
        ToolbarGroupId::File,
        DropPlacement::Before,
    );
    assert_eq!(
        rows,
        vec![
            vec![
                ToolbarGroupId::Preferences,
                ToolbarGroupId::File,
                ToolbarGroupId::Edit,
                ToolbarGroupId::Format,
            ],
            vec![
                ToolbarGroupId::TextExport,
                ToolbarGroupId::RtonExport,
                ToolbarGroupId::About,
            ],
        ]
    );

    move_toolbar_group_to_row_end(&mut rows, ToolbarGroupId::File, 1);
    assert_eq!(
        rows,
        vec![
            vec![
                ToolbarGroupId::Preferences,
                ToolbarGroupId::Edit,
                ToolbarGroupId::Format,
            ],
            vec![
                ToolbarGroupId::TextExport,
                ToolbarGroupId::RtonExport,
                ToolbarGroupId::About,
                ToolbarGroupId::File,
            ],
        ]
    );

    let encoded = toolbar_rows_to_json(&rows);
    assert_eq!(normalize_toolbar_rows(Some(&encoded)), rows);
}
