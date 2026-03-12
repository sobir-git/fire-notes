#![allow(dead_code)]
//! RenderFrame snapshot — pure-data view of current AppLogic state.

use super::AppLogic;
use crate::render_frame::{
    CursorData, LineData, NotesPickerData, NotesPickerEntry, RenameData, RenderFrame,
    ScrollbarData, SelectionData, TabFrameData,
};

impl AppLogic {
    /// Produce a pure-data snapshot of the current UI state.
    /// Used by the renderer and by tests.
    pub fn render_frame(&self) -> RenderFrame {
        let renaming_tab_index = self.focus.renaming_tab_index();
        let rename_input = self.focus.rename_input();

        let tabs: Vec<TabFrameData> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let title = if Some(i) == renaming_tab_index {
                    rename_input
                        .map(|inp| inp.text().to_string())
                        .unwrap_or_else(|| t.title().to_string())
                } else {
                    t.title().to_string()
                };
                TabFrameData { title, is_active: i == self.active_tab }
            })
            .collect();

        let current_tab = &self.tabs[self.active_tab];
        let scroll_offset = current_tab.scroll_offset();
        let total_lines = current_tab.total_lines();
        let visible_count = self.visible_line_count();
        let max_scroll_offset = total_lines.saturating_sub(visible_count);

        let visible_lines: Vec<LineData> = current_tab
            .content()
            .lines()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_count)
            .map(|(idx, text): (usize, &str)| LineData {
                line_index: idx,
                text: text.to_string(),
            })
            .collect();

        let cursor = CursorData {
            line: current_tab.cursor_line(),
            col: current_tab.cursor_col(),
            visible: self.ui_state.cursor_visible,
        };

        let selection = current_tab
            .selection_range_line_col()
            .map(|(start, end): ((usize, usize), (usize, usize))| SelectionData { start, end });

        let scrollbar = if total_lines > visible_count {
            Some(ScrollbarData {
                thumb_top_ratio: scroll_offset as f32 / total_lines as f32,
                thumb_height_ratio: visible_count as f32 / total_lines as f32,
            })
        } else {
            None
        };

        let notes_picker = self.focus.notes_picker_state().map(|(input, list)| {
            let entries = list
                .items()
                .iter()
                .map(|e| NotesPickerEntry {
                    title: e.title.clone(),
                    path: e.path.to_string_lossy().to_string(),
                    is_open: e.is_open,
                })
                .collect();
            NotesPickerData {
                query: input.text().to_string(),
                entries,
                selected_index: list.selected_index(),
                scroll_offset: list.scroll_offset(),
            }
        });

        let rename = renaming_tab_index.and_then(|tab_index| {
            rename_input.map(|inp| RenameData {
                tab_index,
                text: inp.text().to_string(),
                cursor: inp.cursor,
            })
        });

        let flame_positions = self
            .ui_state
            .typing_flame_positions
            .iter()
            .map(|(line, col, _)| (*line, *col))
            .collect();

        RenderFrame {
            tabs,
            active_tab: self.active_tab,
            visible_lines,
            total_lines,
            scroll_offset,
            max_scroll_offset,
            scroll_offset_x: current_tab.scroll_offset_x(),
            word_wrap: current_tab.word_wrap(),
            cursor,
            selection,
            scrollbar,
            notes_picker,
            rename,
            flame_positions,
            width: self.width,
            height: self.height,
            scale: self.scale,
        }
    }
}
