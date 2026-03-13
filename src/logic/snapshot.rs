//! RenderFrame snapshot — pure-data view of current AppLogic state.
//!
//! `render_frame()` calls `prepare_ui_tree()` then bakes all widget geometry
//! into plain data structs so the renderer has zero dependency on widget types.

use super::AppLogic;
use crate::render_frame::{
    CursorData, FrameRect, LineData,
    RenameData, RenderFrame, ScrollbarData, SelectionData,
    TabFrameData,
};

impl AppLogic {
    /// Produce a pure-data snapshot of the current UI state.
    ///
    /// Syncs the retained `ui_tree` first so all geometry is up-to-date,
    /// then bakes widget geometry into plain `FrameRect` / hover data.
    pub fn render_frame(&mut self) -> RenderFrame {
        self.prepare_ui_tree();

        // ── Tab data ──────────────────────────────────────────────────────
        let renaming_tab_index = self.rename_input.as_ref().map(|(idx, _)| *idx);
        let rename_text = self.rename_input.as_ref().map(|(_, inp)| inp.text().to_string());

        let tabs: Vec<TabFrameData> = self.tabs.iter().enumerate()
            .map(|(i, t)| {
                let title = if Some(i) == renaming_tab_index {
                    rename_text.clone().unwrap_or_else(|| t.title().to_string())
                } else {
                    t.title().to_string()
                };
                TabFrameData { title, is_active: i == self.active_tab }
            })
            .collect();

        // ── Text content ──────────────────────────────────────────────────
        let current_tab = &self.tabs[self.active_tab];
        let scroll_offset   = current_tab.scroll_offset();
        let total_lines     = current_tab.total_lines();
        let visible_count   = self.visible_line_count();
        let max_scroll_offset = total_lines.saturating_sub(visible_count);

        let visible_lines: Vec<LineData> = current_tab.content().lines()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_count)
            .map(|(idx, text)| LineData { line_index: idx, text: text.to_string() })
            .collect();

        let cursor = CursorData {
            line:    current_tab.cursor_line(),
            col:     current_tab.cursor_col(),
            visible: self.cursor_visible,
        };

        let selection = current_tab.selection_range_line_col()
            .map(|(start, end)| SelectionData { start, end });

        // ── Scrollbar ratios (for tests / logic layer) ────────────────────
        let scrollbar = if total_lines > visible_count {
            Some(ScrollbarData {
                thumb_top_ratio:    scroll_offset as f32 / total_lines as f32,
                thumb_height_ratio: visible_count as f32 / total_lines as f32,
            })
        } else {
            None
        };

        // ── Notes picker (full geometry bake) ────────────────────────────
        let notes_picker = self.notes_picker.as_ref().map(|p| {
            let window = crate::ui::Rect { x: 0.0, y: 0.0, width: self.width, height: self.height };
            p.snapshot(window, self.scale, self.cursor_visible)
        });

        // ── Rename ────────────────────────────────────────────────────────
        let rename = self.rename_input.as_ref().map(|(tab_index, inp)| RenameData {
            tab_index: *tab_index,
            text:   inp.text().to_string(),
            cursor: inp.state.cursor,
        });

        // ── Flame positions ───────────────────────────────────────────────
        let flame_positions = self.typing_flame_positions.clone();

        // ── Tab bar geometry (baked via component) ───────────────────────
        let tab_bar = crate::components::tab_bar::snapshot(&self.ui_tree.tab_bar);

        // ── Content area geometry (baked via component) ──────────────────
        let ca = &self.ui_tree.content_area;
        let content       = crate::components::text_editor::snapshot(ca);
        let scrollbar_geom = crate::components::text_editor::scrollbar_snapshot(
            ca, total_lines, visible_count, scroll_offset,
        );

        RenderFrame {
            tabs,
            active_tab:        self.active_tab,
            visible_lines,
            total_lines,
            scroll_offset,
            max_scroll_offset,
            scroll_offset_x:   current_tab.scroll_offset_x(),
            word_wrap:         current_tab.word_wrap(),
            cursor,
            selection,
            scrollbar,
            notes_picker,
            rename,
            flame_positions,
            width:  self.width,
            height: self.height,
            scale:  self.scale,
            tab_bar,
            content,
            scrollbar_geom,
        }
    }
}

fn rect_from(r: crate::ui::Rect) -> FrameRect {
    FrameRect { x: r.x, y: r.y, width: r.width, height: r.height }
}

