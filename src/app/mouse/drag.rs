//! Mouse drag handlers — text selection, scrollbar, tab reorder, window.

use std::time::Duration;

use crate::config::timing;
use crate::ui::{UiDragAction, UiNode};

use super::super::state::AppResult;
use super::super::App;

impl App {
    pub fn drag_at(&mut self, x: f32, y: f32) -> AppResult {
        // Picker scrollbar drag is tracked inside NotesPicker.
        if self.logic.focus.is_notes_picker() {
            if let Some(picker) = &mut self.logic.notes_picker {
                if picker.list.is_scrollbar_dragging() {
                    let changed = picker.list.continue_scrollbar_drag(y);
                    return if changed { AppResult::Redraw } else { AppResult::Ok };
                }
            }
        }

        // Scrollbar drag — state lives in fw::Scrollbar.
        if self.logic.ui_tree.content_area.scrollbar.is_dragging() {
            let total_lines = self.logic.tabs[self.logic.active_tab].total_lines();
            let visible_lines = self.visible_lines();
            let scroll_offset = self.logic.tabs[self.logic.active_tab].scroll_offset();
            return match self.logic.ui_tree.drag_scrollbar(y, total_lines, visible_lines, scroll_offset) {
                UiDragAction::ScrollbarDrag { ratio } => self.jump_scrollbar_to_ratio(ratio),
                UiDragAction::None => AppResult::Ok,
            };
        }

        // Tab drag — state lives in TabBar.
        if let Some(tab_index) = self.logic.ui_tree.tab_bar.dragging_tab_index() {
            if y < self.logic.ui_tree.tab_bar.rect.height {
                return self.reorder_tab_at(x, y, tab_index);
            }
            return AppResult::Ok;
        }

        if self.logic.ui_tree.content_area.is_text_selecting {
            if self.logic.focus.is_notes_picker() {
                return self.drag_picker_input(x);
            } else {
                return self.handle_text_selection_drag(x, y);
            }
        }

        AppResult::Ok
    }

    pub fn end_drag(&mut self) {
        self.logic.ui_tree.content_area.scrollbar.end_drag();
        self.logic.ui_tree.tab_bar.end_tab_drag();
        self.logic.ui_tree.content_area.is_text_selecting = false;
        if let Some(picker) = &mut self.logic.notes_picker {
            picker.list.end_drag();
        }
    }

    fn handle_text_selection_drag(&mut self, x: f32, y: f32) -> AppResult {
        let text_area = &self.logic.ui_tree.content_area.text;

        // Throttle drag-scroll when pointer leaves the viewport.
        let height = self.visible_lines() as isize;
        let visual_line = text_area.hit_to_visual_line_clamped(y);
        if visual_line < 0 || visual_line >= height {
            if self.logic.last_drag_scroll.elapsed()
                < Duration::from_millis(timing::DRAG_SCROLL_THROTTLE_MS)
            {
                return AppResult::Ok;
            }
            self.logic.last_drag_scroll = std::time::Instant::now();
        }

        let scroll_offset = self.logic.tabs[self.logic.active_tab].scroll_offset();
        let scroll_offset_x = self.logic.tabs[self.logic.active_tab].scroll_offset_x();
        let char_width = self.renderer.get_char_width();
        let content_lines = self.logic.tabs[self.logic.active_tab].content_line_count();
        let (clicked_line, visual_col, below_last) =
            text_area.hit_to_doc_position(x, y, scroll_offset, scroll_offset_x, char_width, content_lines);
        let clicked_col = if below_last {
            self.logic.tabs[self.logic.active_tab].line_char_len(clicked_line)
        } else {
            self.logic.tabs[self.logic.active_tab].visual_col_to_char_col(clicked_line, visual_col)
        };

        self.logic.tabs[self.logic.active_tab].set_cursor_position(clicked_line, clicked_col, true);
        self.auto_scroll();
        self.logic.reset_cursor_blink();
        AppResult::Redraw
    }

    pub(super) fn reorder_tab_at(&mut self, x: f32, y: f32, from_index: usize) -> AppResult {
        if self.logic.focus.is_renaming() { return AppResult::Ok; }

        if let UiNode::Tab(to_index) = self.logic.ui_tree.hit_test(x, y) {
            if to_index != from_index && from_index < self.logic.tabs.len() && to_index < self.logic.tabs.len() {
                let tab = self.logic.tabs.remove(from_index);
                self.logic.tabs.insert(to_index, tab);

                if self.logic.active_tab == from_index {
                    self.logic.activate_tab(to_index);
                } else if from_index < self.logic.active_tab && to_index >= self.logic.active_tab {
                    self.logic.active_tab = self.logic.active_tab.saturating_sub(1);
                } else if from_index > self.logic.active_tab && to_index <= self.logic.active_tab {
                    self.logic.active_tab = (self.logic.active_tab + 1).min(self.logic.tabs.len() - 1);
                }

                self.logic.ui_tree.tab_bar.start_tab_drag(to_index);
                return AppResult::Redraw;
            }
        }
        AppResult::Ok
    }
}
