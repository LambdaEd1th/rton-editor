use crate::domain::{TEXT_MAX_VIEWPORT_ROWS, TEXT_MAX_VIRTUAL_SCROLL_HEIGHT, TEXT_ROW_HEIGHT};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TextJumpTarget {
    pub(crate) id: u64,
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) selection_end_column: usize,
    pub(crate) line_count: usize,
    pub(crate) focus: bool,
}

pub(crate) fn scroll_text_editor_to_text_target(target: TextJumpTarget) {
    let line = target.line.max(1);
    let column = target.column;
    let selection_end_column = target.selection_end_column.max(column);
    let line_count = target.line_count.max(line);
    let focus = target.focus;
    let row_height = TEXT_ROW_HEIGHT;
    let max_viewport_rows = TEXT_MAX_VIEWPORT_ROWS;
    let max_scroll_height = TEXT_MAX_VIRTUAL_SCROLL_HEIGHT;
    dioxus::document::eval(&format!(
        r#"
        (() => {{
            const targetLine = {line};
            const targetColumn = {column};
            const selectionEndColumn = {selection_end_column};
            const rowCount = Math.max(1, {line_count});
            const shouldFocus = {focus};
            const fallbackRowHeight = {row_height};
            const maxViewportRows = {max_viewport_rows};
            const maxScrollHeight = {max_scroll_height};
            const virtualText = document.querySelector('.virtual-text-preview-content');
            if (!virtualText) return;

            const rowHeight = Number.parseFloat(getComputedStyle(virtualText).getPropertyValue('--virtual-text-row-height')) || fallbackRowHeight;
            const viewportRows = Math.max(1, Math.min(maxViewportRows, Math.ceil(virtualText.clientHeight / rowHeight) || 1));
            const rowIndex = Math.max(0, Math.min(targetLine - 1, rowCount - 1));
            const maxStartRow = Math.max(0, rowCount - viewportRows);
            const targetStartRow = Math.max(0, Math.min(maxStartRow, rowIndex - Math.floor(viewportRows / 2)));
            const logicalHeight = rowCount * rowHeight;
            let targetScrollTop = 0;
            if (logicalHeight <= maxScrollHeight) {{
                targetScrollTop = targetStartRow * rowHeight;
            }} else {{
                const viewportContentHeight = viewportRows * rowHeight;
                const scrollableHeight = Math.max(1, maxScrollHeight - viewportContentHeight);
                const scaledMaxStartRow = Math.max(1, rowCount - viewportRows);
                targetScrollTop = (targetStartRow / scaledMaxStartRow) * scrollableHeight;
            }}

            virtualText.scrollTop = Math.max(0, targetScrollTop);
            virtualText.scrollLeft = Math.max(0, targetColumn * 8 - virtualText.clientWidth / 2);
            if (shouldFocus) {{
                virtualText.focus?.();
            }}

            const focusTarget = (attempt) => {{
                const target = virtualText.querySelector(`[data-line-number="${{targetLine}}"]`);
                if (target) {{
                    if (shouldFocus) {{
                        const isTextInput = target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement;
                        if (isTextInput) {{
                            try {{
                                target.focus({{ preventScroll: true }});
                            }} catch (_error) {{
                                target.focus();
                            }}
                            const column = Math.max(0, Math.min(targetColumn, target.value?.length ?? 0));
                            const endColumn = Math.max(column, Math.min(selectionEndColumn, target.value?.length ?? 0));
                            target.setSelectionRange?.(column, endColumn);
                        }} else {{
                            try {{
                                target.focus({{ preventScroll: true }});
                            }} catch (_error) {{
                                target.focus();
                            }}
                            const textNode = Array.from(target.childNodes).find((node) => node.nodeType === Node.TEXT_NODE);
                            if (textNode) {{
                                const length = textNode.textContent?.length ?? 0;
                                const column = Math.max(0, Math.min(targetColumn, length));
                                const endColumn = Math.max(column, Math.min(selectionEndColumn, length));
                                const selection = window.getSelection?.();
                                const range = document.createRange();
                                range.setStart(textNode, column);
                                range.setEnd(textNode, endColumn);
                                selection?.removeAllRanges();
                                selection?.addRange(range);
                            }}
                        }}
                    }}
                }} else if (attempt < 8) {{
                    requestAnimationFrame(() => focusTarget(attempt + 1));
                }}
            }};

            requestAnimationFrame(() => focusTarget(0));
        }})();
        "#
    ));
}
