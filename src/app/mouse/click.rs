//! Mouse click handlers — single, double, triple, right-click.

use crate::ui::{UiAction, UiNode};

use super::super::state::AppResult;
use super::super::App;

impl App {
    pub fn click_at(&mut self, x: f32, y: f32, selecting: bool) -> AppResult {
        if self.logic.focus.is_overlay() {
            return self.logic.dispatch_overlay(crate::app::overlay_event::OverlayEvent::PointerDown { x, y });
        }

        self.logic.prepare_ui_tree();
        let total_lines = self.logic.tabs[self.logic.active_tab].total_lines();
        let visible_lines = self.visible_lines();
        let scroll_offset = self.logic.tabs[self.logic.active_tab].scroll_offset();

        match self.logic.ui_tree.click(x, y, total_lines, visible_lines, scroll_offset, selecting) {
            UiAction::ActivateTab(i) => {
                self.logic.activate_tab(i);
                self.logic.ui_tree.tab_bar.start_tab_drag(i);
                return AppResult::Redraw;
            }
            UiAction::NewTab => return self.new_tab(),
            UiAction::StartScrollbarDrag { drag_offset } => {
                self.logic.ui_tree.content_area.scrollbar.start_drag(drag_offset);
                return AppResult::Ok;
            }
            UiAction::ScrollbarJump { ratio } => {
                return self.jump_scrollbar_to_ratio(ratio);
            }
            UiAction::WindowMinimize => return AppResult::WindowMinimize,
            UiAction::WindowMaximize => return AppResult::WindowMaximize,
            UiAction::WindowClose => return AppResult::WindowClose,
            UiAction::WindowDrag => {
                return AppResult::WindowDrag;
            }
            UiAction::WindowResize(edge) => {
                return AppResult::WindowResize(edge);
            }
            UiAction::None => return AppResult::Ok,
            UiAction::TextClick => {
                self.logic.ui_tree.content_area.is_text_selecting = true;
            }
        }

        let text_area = &self.logic.ui_tree.content_area.text;
        if !selecting && !text_area.hit_test(x, y) && y >= text_area.rect.y { return AppResult::Ok; }
        if !selecting && y < text_area.rect.y { return AppResult::Ok; }

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

        self.logic.tabs[self.logic.active_tab].set_cursor_position(clicked_line, clicked_col, selecting);
        if selecting { self.auto_scroll(); }
        self.logic.reset_cursor_blink();
        AppResult::Redraw
    }

    pub fn handle_double_click(&mut self, x: f32, y: f32) -> AppResult {
        if self.logic.focus.is_overlay() {
            return self.logic.dispatch_overlay(crate::app::overlay_event::OverlayEvent::PointerDoubleClick { x, y });
        }
        self.logic.prepare_ui_tree();
        let total_lines = self.logic.tabs[self.logic.active_tab].total_lines();
        let visible_lines = self.visible_lines();
        let scroll_offset = self.logic.tabs[self.logic.active_tab].scroll_offset();

        match self.logic.ui_tree.double_click(x, y, total_lines, visible_lines, scroll_offset) {
            UiAction::ActivateTab(i) => {
                self.logic.activate_tab(i);
                AppResult::Redraw
            }
            UiAction::NewTab => self.new_tab(),
            UiAction::TextClick => {
                let _ = self.click_at(x, y, false);
                self.logic.tabs[self.logic.active_tab].select_word_at_cursor();
                AppResult::Redraw
            }
            _ => AppResult::Ok,
        }
    }

    pub fn handle_triple_click(&mut self, x: f32, y: f32) -> AppResult {
        if self.logic.focus.is_overlay() {
            return self.logic.dispatch_overlay(crate::app::overlay_event::OverlayEvent::PointerTripleClick { x, y });
        }
        self.logic.prepare_ui_tree();
        let total_lines = self.logic.tabs[self.logic.active_tab].total_lines();
        let visible_lines = self.visible_lines();
        let scroll_offset = self.logic.tabs[self.logic.active_tab].scroll_offset();

        match self.logic.ui_tree.triple_click(x, y, total_lines, visible_lines, scroll_offset) {
            UiAction::ActivateTab(i) => {
                self.logic.activate_tab(i);
                AppResult::Redraw
            }
            UiAction::NewTab => self.new_tab(),
            UiAction::TextClick => {
                let _ = self.click_at(x, y, false);
                self.logic.tabs[self.logic.active_tab].select_line_at_cursor();
                AppResult::Redraw
            }
            _ => AppResult::Ok,
        }
    }

    pub fn right_click_at(&mut self, x: f32, y: f32) -> AppResult {
        self.logic.prepare_ui_tree();
        match self.logic.ui_tree.hit_test(x, y) {
            UiNode::Tab(i) => { self.logic.start_rename(i); AppResult::Redraw }
            _ => AppResult::Ok,
        }
    }

    pub(super) fn jump_scrollbar_to_ratio(&mut self, ratio: f32) -> AppResult {
        let max_scroll = self.logic.tabs[self.logic.active_tab].max_scroll_offset();
        if max_scroll == 0 { return AppResult::Ok; }
        let scroll_offset = (ratio.clamp(0.0, 1.0) * max_scroll as f32).round() as usize;
        if self.logic.tabs[self.logic.active_tab].set_scroll_offset(scroll_offset) {
            return AppResult::Redraw;
        }
        AppResult::Ok
    }
}
