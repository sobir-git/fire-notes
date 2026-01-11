//! Application state and coordination

use crate::persistence;
use crate::renderer::{HitTestResult, Renderer};
use crate::tab::Tab;
use std::time::{Duration, Instant};

const LINE_HEIGHT: f32 = 24.0;
const TAB_HEIGHT: f32 = 40.0;
const PADDING: f32 = 16.0;

use arboard::Clipboard;

#[must_use = "Handle the AppResult to ensure the UI updates correctly"]
pub enum AppResult {
    Ok,
    Redraw,
    Quit,
}

impl AppResult {
    pub fn needs_redraw(&self) -> bool {
        matches!(self, AppResult::Redraw)
    }

    pub fn should_exit(&self) -> bool {
        matches!(self, AppResult::Quit)
    }
}

pub struct App {
    renderer: Renderer,
    tabs: Vec<Tab>,
    active_tab: usize,
    width: f32,
    height: f32,
    clipboard: Option<Clipboard>,
    is_dragging_scrollbar: bool,
    scale: f32,

    // UI State
    cursor_visible: bool,
    last_cursor_blink: Instant,
    hovered_tab: Option<usize>,
    hovered_plus: bool,
    tab_scroll_x: f32,
    last_mouse_x: f32,
    last_mouse_y: f32,
}

impl App {
    pub fn new(
        gl_renderer: femtovg::renderer::OpenGl,
        width: f32,
        height: f32,
        scale: f32,
    ) -> Self {
        let renderer = Renderer::new(gl_renderer, width, height, scale);
        let clipboard = Clipboard::new().ok();

        // Load existing notes or create a new one
        let tabs = match persistence::list_notes() {
            Ok(note_paths) if !note_paths.is_empty() => note_paths
                .into_iter()
                .filter_map(|path| Tab::from_file(path))
                .collect(),
            _ => vec![Tab::new_untitled()],
        };

        Self {
            renderer,
            tabs,
            active_tab: 0,
            width,
            height,
            clipboard,
            is_dragging_scrollbar: false,
            scale,
            cursor_visible: true,
            last_cursor_blink: Instant::now(),
            hovered_tab: None,
            hovered_plus: false,
            tab_scroll_x: 0.0,
            last_mouse_x: 0.0,
            last_mouse_y: 0.0,
        }
    }

    pub fn tick(&mut self) -> AppResult {
        if self.last_cursor_blink.elapsed() >= Duration::from_millis(500) {
            self.cursor_visible = !self.cursor_visible;
            self.last_cursor_blink = Instant::now();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn handle_mouse_move(&mut self, x: f32, y: f32) -> AppResult {
        self.props_mouse_move(x, y)
    }

    fn props_mouse_move(&mut self, x: f32, y: f32) -> AppResult {
        self.last_mouse_x = x;
        self.last_mouse_y = y;

        // Update hover state
        let tab_info: Vec<(&str, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| (t.title(), i == self.active_tab))
            .collect();

        let prev_hovered_tab = self.hovered_tab;
        let prev_hovered_plus = self.hovered_plus;

        match self.renderer.hit_test(x, y, &tab_info) {
            Some(HitTestResult::Tab(i)) => {
                self.hovered_tab = Some(i);
                self.hovered_plus = false;
            }
            Some(HitTestResult::NewTabButton) => {
                self.hovered_tab = None;
                self.hovered_plus = true;
            }
            None => {
                self.hovered_tab = None;
                self.hovered_plus = false;
            }
        }

        if prev_hovered_tab != self.hovered_tab || prev_hovered_plus != self.hovered_plus {
            AppResult::Redraw
        } else {
            AppResult::Ok
        }
    }

    fn visible_lines(&self) -> usize {
        let content_height = self.height - TAB_HEIGHT * self.scale - PADDING * 2.0 * self.scale;
        (content_height / (LINE_HEIGHT * self.scale))
            .floor()
            .max(1.0) as usize
    }

    fn auto_scroll(&mut self) {
        let visible = self.visible_lines();
        self.tabs[self.active_tab].ensure_cursor_visible(visible);
        // Reset blinking on action
        self.cursor_visible = true;
        self.last_cursor_blink = Instant::now();
    }

    pub fn resize(&mut self, width: f32, height: f32, scale: f32) {
        self.width = width;
        self.height = height;
        self.scale = scale;
        self.renderer.resize(width, height, scale);
    }

    pub fn render(&mut self) {
        let tab_info: Vec<(&str, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| (t.title(), i == self.active_tab))
            .collect();

        let current_tab = &self.tabs[self.active_tab];

        // Pass all UI state to renderer
        self.renderer.render(
            &tab_info,
            current_tab,
            self.cursor_visible,
            self.hovered_tab,
            self.hovered_plus,
        );
    }

    pub fn new_tab(&mut self) -> AppResult {
        self.tabs.push(Tab::new_untitled());
        self.active_tab = self.tabs.len() - 1;
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn close_current_tab(&mut self) -> AppResult {
        if self.tabs.len() <= 1 {
            return AppResult::Ok; // Cannot close the last tab
        }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn next_tab(&mut self) -> AppResult {
        if self.tabs.is_empty() {
            return AppResult::Ok;
        }
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_char(&mut self, ch: char) -> AppResult {
        self.tabs[self.active_tab].insert_char(ch);
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_backspace(&mut self) -> AppResult {
        self.tabs[self.active_tab].backspace();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_delete(&mut self) -> AppResult {
        self.tabs[self.active_tab].delete();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_left(&mut self, selecting: bool) -> AppResult {
        self.tabs[self.active_tab].move_left(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_right(&mut self, selecting: bool) -> AppResult {
        self.tabs[self.active_tab].move_right(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_word_left(&mut self, selecting: bool) -> AppResult {
        self.tabs[self.active_tab].move_word_left(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_word_right(&mut self, selecting: bool) -> AppResult {
        self.tabs[self.active_tab].move_word_right(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_up(&mut self, selecting: bool) -> AppResult {
        self.tabs[self.active_tab].move_up(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_down(&mut self, selecting: bool) -> AppResult {
        self.tabs[self.active_tab].move_down(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn scroll_up(&mut self) -> AppResult {
        if self.last_mouse_y < TAB_HEIGHT * self.scale {
            // Scroll tabs
            self.tab_scroll_x = (self.tab_scroll_x - 30.0 * self.scale).max(0.0);
            self.renderer.set_tab_scroll_x(self.tab_scroll_x);
            return AppResult::Redraw;
        } else {
            // Scroll content
            self.tabs[self.active_tab].scroll_up(3);
            AppResult::Redraw
        }
    }

    pub fn scroll_down(&mut self) -> AppResult {
        if self.last_mouse_y < TAB_HEIGHT * self.scale {
            // Scroll tabs
            let max_scroll = 1000.0; // TODO: Calculate max scroll based on tabs width
            self.tab_scroll_x = (self.tab_scroll_x + 40.0 * self.scale).min(max_scroll);
            self.renderer.set_tab_scroll_x(self.tab_scroll_x);
            return AppResult::Redraw;
        }
        let visible = self.visible_lines();
        self.tabs[self.active_tab].scroll_down(3, visible);
        AppResult::Redraw
    }

    pub fn move_cursor_to_line_start(&mut self, selecting: bool) -> AppResult {
        self.tabs[self.active_tab].move_to_line_start(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_line_end(&mut self, selecting: bool) -> AppResult {
        self.tabs[self.active_tab].move_to_line_end(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_start(&mut self, selecting: bool) -> AppResult {
        self.tabs[self.active_tab].move_to_start(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_end(&mut self, selecting: bool) -> AppResult {
        self.tabs[self.active_tab].move_to_end(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn save_current(&mut self) -> AppResult {
        self.tabs[self.active_tab].save();
        AppResult::Redraw // Title might change if saved as new file
    }

    pub fn open_file(&mut self) -> AppResult {
        if let Some(tab) = Tab::open() {
            self.tabs.push(tab);
            self.active_tab = self.tabs.len() - 1;
            self.auto_scroll();
            AppResult::Redraw
        } else {
            AppResult::Ok
        }
    }

    /// Handle mouse click - position cursor at clicked location
    pub fn click_at(&mut self, x: f32, y: f32, selecting: bool) -> AppResult {
        let scrollbar_width = 12.0 * self.scale;
        let tab_info: Vec<(&str, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| (t.title(), i == self.active_tab))
            .collect();

        // Check for tab interactions first
        if y < TAB_HEIGHT * self.scale {
            match self.renderer.hit_test(x, y, &tab_info) {
                Some(HitTestResult::Tab(i)) => {
                    self.active_tab = i;
                    self.auto_scroll();
                    return AppResult::Redraw;
                }
                Some(HitTestResult::NewTabButton) => {
                    return self.new_tab();
                }
                None => {
                    return AppResult::Ok; // Clicked in tab bar empty space
                }
            }
        }

        // Check if clicked on scrollbar
        if x > self.width - scrollbar_width {
            let content_start_y = TAB_HEIGHT * self.scale;
            if y >= content_start_y {
                // Clicked on scrollbar track
                self.is_dragging_scrollbar = true;
                return self.drag_at(x, y); // Drag at will handle the scroll and redraw
            }
        } else {
            self.is_dragging_scrollbar = false;
        }

        // Calculate which line was clicked
        let content_start_y = TAB_HEIGHT * self.scale + PADDING * self.scale;

        if y < content_start_y {
            return AppResult::Ok; // Clicked in tab bar empty space or padding
        }

        let relative_y = y - content_start_y;
        let clicked_visual_line = (relative_y / (LINE_HEIGHT * self.scale)).floor() as usize;
        let scroll_offset = self.tabs[self.active_tab].scroll_offset();
        let clicked_line = scroll_offset + clicked_visual_line;

        // Calculate which column was clicked (approximate based on char width)
        let char_width = 9.6 * self.scale; // Approximate monospace character width
        let relative_x = (x - PADDING * self.scale).max(0.0);
        let clicked_col = (relative_x / char_width).round() as usize;

        // Set cursor position
        self.tabs[self.active_tab].set_cursor_position(clicked_line, clicked_col, selecting);

        // Don't auto-scroll instantly on click unless necessary
        // self.auto_scroll();

        // Reset blinking
        self.cursor_visible = true;
        self.last_cursor_blink = Instant::now();
        AppResult::Redraw
    }

    pub fn handle_double_click(&mut self, x: f32, y: f32) -> AppResult {
        // First, place cursor (and clear previous selection)
        let _ = self.click_at(x, y, false);
        // Then select word at that cursor position
        self.tabs[self.active_tab].select_word_at_cursor();
        AppResult::Redraw
    }

    pub fn drag_at(&mut self, x: f32, y: f32) -> AppResult {
        if self.is_dragging_scrollbar {
            let start_y = TAB_HEIGHT * self.scale;
            let scroll_area_height = self.height - TAB_HEIGHT * self.scale;

            // Calculate scroll ratio from Y position
            let relative_y = (y - start_y).clamp(0.0, scroll_area_height);
            let scroll_ratio = relative_y / scroll_area_height;

            let total_lines = self.tabs[self.active_tab].total_lines();
            let visible_lines = self.visible_lines();

            let max_scroll = total_lines.saturating_sub(visible_lines);
            let scroll_offset = (scroll_ratio * max_scroll as f32).round() as usize;

            if self.tabs[self.active_tab].set_scroll_offset(scroll_offset) {
                return AppResult::Redraw;
            }
            return AppResult::Ok;
        }

        self.click_at(x, y, true)
    }

    pub fn handle_copy(&mut self) -> AppResult {
        if let Some(text) = self.tabs[self.active_tab].copy_selection() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
        }
        AppResult::Ok
    }

    pub fn handle_cut(&mut self) -> AppResult {
        if let Some(text) = self.tabs[self.active_tab].cut_selection() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
            self.tabs[self.active_tab].auto_save();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn handle_paste(&mut self) -> AppResult {
        if let Some(clipboard) = &mut self.clipboard {
            if let Ok(text) = clipboard.get_text() {
                self.tabs[self.active_tab].paste_text(&text);
                self.tabs[self.active_tab].auto_save();
                self.auto_scroll();
                return AppResult::Redraw;
            }
        }
        AppResult::Ok
    }

    pub fn handle_select_all(&mut self) -> AppResult {
        self.tabs[self.active_tab].select_all();
        AppResult::Redraw
    }

    pub fn handle_move_lines_up(&mut self) -> AppResult {
        if self.tabs[self.active_tab].move_lines_up() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn handle_move_lines_down(&mut self) -> AppResult {
        if self.tabs[self.active_tab].move_lines_down() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn handle_undo(&mut self) -> AppResult {
        if self.tabs[self.active_tab].undo() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn handle_redo(&mut self) -> AppResult {
        if self.tabs[self.active_tab].redo() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: GUI tests require actual OpenGL context
    // These are structure tests only

    #[test]
    fn test_tab_management_logic() {
        // We can't test the full App without OpenGL, but we can test Tab
        let tab = Tab::new_untitled();
        assert!(tab.title().starts_with("Untitled"));
    }
}
