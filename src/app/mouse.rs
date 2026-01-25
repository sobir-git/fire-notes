//! Mouse event handling

use std::time::Duration;

use crate::config::{layout, timing};
use crate::ui::{UiAction, UiDragAction, UiNode, UiTree};

use super::state::{AppResult, MouseInteraction};
use super::App;

impl App {
    pub fn handle_mouse_move(&mut self, x: f32, y: f32) -> AppResult {
        self.state.last_mouse_x = x;
        self.state.last_mouse_y = y;

        let tab_info: Vec<(&str, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| (t.title(), i == self.active_tab))
            .collect();

        let prev_hovered_tab_index = self.state.hovered_tab_index;
        let prev_hovered_plus = self.state.hovered_plus;
        let prev_hovered_scrollbar = self.state.hovered_scrollbar;
        let prev_hovered_minimize = self.state.hovered_window_minimize;
        let prev_hovered_maximize = self.state.hovered_window_maximize;
        let prev_hovered_close = self.state.hovered_window_close;
        let prev_hovered_resize_edge = self.state.hovered_resize_edge;

        let total_lines = self.tabs[self.active_tab].total_lines();
        let visible_lines = self.visible_lines();
        let scroll_offset = self.tabs[self.active_tab].scroll_offset();
        let ui_tree = UiTree::new(
            self.width,
            self.height,
            self.scale,
            self.state.tab_scroll_x,
            &tab_info,
        );
        let hover = ui_tree.hover(x, y, total_lines, visible_lines, scroll_offset);
        self.state.hovered_tab_index = hover.tab_index;
        self.state.hovered_plus = hover.plus;
        self.state.hovered_scrollbar = hover.scrollbar;
        self.state.hovered_window_minimize = hover.window_minimize;
        self.state.hovered_window_maximize = hover.window_maximize;
        self.state.hovered_window_close = hover.window_close;
        self.state.hovered_resize_edge = hover.resize_edge;

        if prev_hovered_tab_index != self.state.hovered_tab_index
            || prev_hovered_plus != self.state.hovered_plus
            || prev_hovered_scrollbar != self.state.hovered_scrollbar
            || prev_hovered_minimize != self.state.hovered_window_minimize
            || prev_hovered_maximize != self.state.hovered_window_maximize
            || prev_hovered_close != self.state.hovered_window_close
            || prev_hovered_resize_edge != self.state.hovered_resize_edge
        {
            AppResult::Redraw
        } else {
            AppResult::Ok
        }
    }

    pub fn click_at(&mut self, x: f32, y: f32, selecting: bool) -> AppResult {
        let tab_info: Vec<(&str, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| (t.title(), i == self.active_tab))
            .collect();
        let total_lines = self.tabs[self.active_tab].total_lines();
        let visible_lines = self.visible_lines();
        let scroll_offset = self.tabs[self.active_tab].scroll_offset();
        let ui_tree = UiTree::new(
            self.width,
            self.height,
            self.scale,
            self.state.tab_scroll_x,
            &tab_info,
        );

        match ui_tree.click(x, y, total_lines, visible_lines, scroll_offset, selecting) {
            UiAction::ActivateTab(i) => {
                self.active_tab = i;
                self.auto_scroll();
                self.state.mouse_interaction = MouseInteraction::TabDrag { tab_index: i };
                return AppResult::Redraw;
            }
            UiAction::NewTab => {
                return self.new_tab();
            }
            UiAction::StartScrollbarDrag { drag_offset } => {
                self.state.mouse_interaction = MouseInteraction::ScrollbarDrag { drag_offset };
                return AppResult::Ok;
            }
            UiAction::ScrollbarJump { ratio } => {
                self.state.mouse_interaction = MouseInteraction::None;
                return self.jump_scrollbar_to_ratio(ratio);
            }
            UiAction::WindowMinimize => {
                return AppResult::WindowMinimize;
            }
            UiAction::WindowMaximize => {
                return AppResult::WindowMaximize;
            }
            UiAction::WindowClose => {
                return AppResult::WindowClose;
            }
            UiAction::WindowDrag => {
                self.state.mouse_interaction = MouseInteraction::WindowDrag;
                return AppResult::WindowDrag;
            }
            UiAction::WindowResize(edge) => {
                self.state.mouse_interaction = MouseInteraction::WindowResize(edge);
                return AppResult::WindowResize(edge);
            }
            UiAction::None => {
                return AppResult::Ok;
            }
            UiAction::TextClick => {
                self.state.mouse_interaction = MouseInteraction::TextSelection;
            }
        }

        // Calculate which line was clicked
        let content_start_y = self.content_start_y();

        if !selecting && y < content_start_y {
            return AppResult::Ok;
        }

        let height = self.visible_lines() as isize;
        let relative_y = y - content_start_y;
        let mut clicked_visual_line =
            (relative_y / (layout::LINE_HEIGHT * self.scale)).floor() as isize;

        if selecting {
            if clicked_visual_line < 0 || clicked_visual_line >= height {
                if self.state.last_drag_scroll.elapsed()
                    < Duration::from_millis(timing::DRAG_SCROLL_THROTTLE_MS)
                {
                    return AppResult::Ok;
                }
                self.state.last_drag_scroll = Instant::now();

                if clicked_visual_line < 0 {
                    clicked_visual_line = -1;
                } else {
                    clicked_visual_line = height;
                }
            }
        }

        let scroll_offset = self.tabs[self.active_tab].scroll_offset();
        let clicked_line = (scroll_offset as isize + clicked_visual_line).max(0) as usize;

        let char_width = self.renderer.get_char_width();
        let scroll_offset_x = self.tabs[self.active_tab].scroll_offset_x();
        let relative_x = (x - layout::PADDING * self.scale + scroll_offset_x).max(0.0);
        let clicked_col = (relative_x / char_width).round() as usize;

        self.tabs[self.active_tab].set_cursor_position(clicked_line, clicked_col, selecting);

        if selecting {
            self.auto_scroll();
        }

        self.state.reset_cursor_blink();
        AppResult::Redraw
    }

    pub fn handle_double_click(&mut self, x: f32, y: f32) -> AppResult {
        let tab_info: Vec<(&str, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| (t.title(), i == self.active_tab))
            .collect();
        let total_lines = self.tabs[self.active_tab].total_lines();
        let visible_lines = self.visible_lines();
        let scroll_offset = self.tabs[self.active_tab].scroll_offset();
        let ui_tree = UiTree::new(
            self.width,
            self.height,
            self.scale,
            self.state.tab_scroll_x,
            &tab_info,
        );

        match ui_tree.double_click(x, y, total_lines, visible_lines, scroll_offset) {
            UiAction::ActivateTab(i) => {
                self.active_tab = i;
                self.auto_scroll();
                return AppResult::Redraw;
            }
            UiAction::NewTab => {
                return self.new_tab();
            }
            UiAction::TextClick => {
                let _ = self.click_at(x, y, false);
                self.tabs[self.active_tab].select_word_at_cursor();
                return AppResult::Redraw;
            }
            _ => return AppResult::Ok,
        }
    }

    pub fn handle_triple_click(&mut self, x: f32, y: f32) -> AppResult {
        let tab_info: Vec<(&str, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| (t.title(), i == self.active_tab))
            .collect();
        let total_lines = self.tabs[self.active_tab].total_lines();
        let visible_lines = self.visible_lines();
        let scroll_offset = self.tabs[self.active_tab].scroll_offset();
        let ui_tree = UiTree::new(
            self.width,
            self.height,
            self.scale,
            self.state.tab_scroll_x,
            &tab_info,
        );

        match ui_tree.triple_click(x, y, total_lines, visible_lines, scroll_offset) {
            UiAction::ActivateTab(i) => {
                self.active_tab = i;
                self.auto_scroll();
                return AppResult::Redraw;
            }
            UiAction::NewTab => {
                return self.new_tab();
            }
            UiAction::TextClick => {
                let _ = self.click_at(x, y, false);
                self.tabs[self.active_tab].select_line_at_cursor();
                return AppResult::Redraw;
            }
            _ => return AppResult::Ok,
        }
    }

    pub fn right_click_at(&mut self, x: f32, y: f32) -> AppResult {
        let tab_info: Vec<(&str, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| (t.title(), i == self.active_tab))
            .collect();

        let ui_tree = UiTree::new(
            self.width,
            self.height,
            self.scale,
            self.state.tab_scroll_x,
            &tab_info,
        );
        match ui_tree.hit_test(x, y) {
            UiNode::Tab(i) => {
                self.start_rename(i);
                AppResult::Redraw
            }
            _ => AppResult::Ok,
        }
    }

    pub fn drag_at(&mut self, x: f32, y: f32) -> AppResult {
        // Handle drag based on current mouse interaction state
        match self.state.mouse_interaction {
            MouseInteraction::None => {
                // No active interaction, nothing to do
                AppResult::Ok
            }
            MouseInteraction::WindowDrag | MouseInteraction::WindowResize(_) => {
                // Window operations are handled by the OS, nothing to do here
                AppResult::Ok
            }
            MouseInteraction::TabDrag { tab_index } => {
                if y < layout::TAB_HEIGHT * self.scale {
                    self.reorder_tab_at(x, y, tab_index)
                } else {
                    AppResult::Ok
                }
            }
            MouseInteraction::ScrollbarDrag { drag_offset } => {
                let total_lines = self.tabs[self.active_tab].total_lines();
                let visible_lines = self.visible_lines();
                let scroll_offset = self.tabs[self.active_tab].scroll_offset();
                let ui_tree = UiTree::new(
                    self.width,
                    self.height,
                    self.scale,
                    self.state.tab_scroll_x,
                    &self.tab_titles(),
                );
                match ui_tree.drag_scrollbar(
                    y,
                    total_lines,
                    visible_lines,
                    scroll_offset,
                    drag_offset,
                ) {
                    UiDragAction::ScrollbarDrag { ratio } => self.jump_scrollbar_to_ratio(ratio),
                    UiDragAction::None => AppResult::Ok,
                }
            }
            MouseInteraction::TextSelection => {
                self.handle_text_selection_drag(x, y)
            }
        }
    }

    fn handle_text_selection_drag(&mut self, x: f32, y: f32) -> AppResult {
        let content_start_y = self.content_start_y();
        let height = self.visible_lines() as isize;
        let relative_y = y - content_start_y;
        let mut clicked_visual_line =
            (relative_y / (layout::LINE_HEIGHT * self.scale)).floor() as isize;

        if clicked_visual_line < 0 || clicked_visual_line >= height {
            if self.state.last_drag_scroll.elapsed()
                < Duration::from_millis(timing::DRAG_SCROLL_THROTTLE_MS)
            {
                return AppResult::Ok;
            }
            self.state.last_drag_scroll = Instant::now();

            if clicked_visual_line < 0 {
                clicked_visual_line = -1;
            } else {
                clicked_visual_line = height;
            }
        }

        let scroll_offset = self.tabs[self.active_tab].scroll_offset();
        let clicked_line = (scroll_offset as isize + clicked_visual_line).max(0) as usize;

        let char_width = self.renderer.get_char_width();
        let scroll_offset_x = self.tabs[self.active_tab].scroll_offset_x();
        let relative_x = (x - layout::PADDING * self.scale + scroll_offset_x).max(0.0);
        let clicked_col = (relative_x / char_width).round() as usize;

        self.tabs[self.active_tab].set_cursor_position(clicked_line, clicked_col, true);
        self.auto_scroll();
        self.state.reset_cursor_blink();
        AppResult::Redraw
    }

    pub fn end_drag(&mut self) {
        self.state.mouse_interaction = MouseInteraction::None;
    }

    pub(super) fn reorder_tab_at(&mut self, x: f32, y: f32, from_index: usize) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }

        let tab_info: Vec<(&str, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| (t.title(), i == self.active_tab))
            .collect();

        let ui_tree = UiTree::new(
            self.width,
            self.height,
            self.scale,
            self.state.tab_scroll_x,
            &tab_info,
        );
        if let UiNode::Tab(to_index) = ui_tree.hit_test(x, y) {
            if to_index != from_index && from_index < self.tabs.len() && to_index < self.tabs.len()
            {
                let tab = self.tabs.remove(from_index);
                self.tabs.insert(to_index, tab);

                if self.active_tab == from_index {
                    self.active_tab = to_index;
                } else if from_index < self.active_tab && to_index >= self.active_tab {
                    self.active_tab = self.active_tab.saturating_sub(1);
                } else if from_index > self.active_tab && to_index <= self.active_tab {
                    self.active_tab = (self.active_tab + 1).min(self.tabs.len() - 1);
                }

                if let Some(rename_index) = self.state.renaming_tab {
                    self.state.renaming_tab = if rename_index == from_index {
                        Some(to_index)
                    } else if from_index < rename_index && to_index >= rename_index {
                        Some(rename_index - 1)
                    } else if from_index > rename_index && to_index <= rename_index {
                        Some(rename_index + 1)
                    } else {
                        Some(rename_index)
                    };
                }

                self.state.mouse_interaction = MouseInteraction::TabDrag { tab_index: to_index };
                return AppResult::Redraw;
            }
        }

        AppResult::Ok
    }

    pub(super) fn jump_scrollbar_to_ratio(&mut self, ratio: f32) -> AppResult {
        let total_lines = self.tabs[self.active_tab].total_lines();
        let visible_lines = self.visible_lines();
        if total_lines <= visible_lines {
            return AppResult::Ok;
        }
        let max_scroll = total_lines.saturating_sub(visible_lines);
        let scroll_offset = (ratio.clamp(0.0, 1.0) * max_scroll as f32).round() as usize;
        if self.tabs[self.active_tab].set_scroll_offset(scroll_offset) {
            return AppResult::Redraw;
        }
        AppResult::Ok
    }
}

use std::time::Instant;
