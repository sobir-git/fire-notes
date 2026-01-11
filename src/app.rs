//! Application state and coordination

use crate::persistence;
use crate::renderer::Renderer;
use crate::tab::Tab;

const LINE_HEIGHT: f32 = 24.0;
const TAB_HEIGHT: f32 = 36.0;
const PADDING: f32 = 16.0;

use arboard::Clipboard;

pub struct App {
    renderer: Renderer,
    tabs: Vec<Tab>,
    active_tab: usize,
    width: f32,
    height: f32,
    clipboard: Option<Clipboard>,
    is_dragging_scrollbar: bool,
    scale: f32,
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
        self.renderer.render(&tab_info, current_tab);
    }

    pub fn new_tab(&mut self) {
        self.tabs.push(Tab::new_untitled());
        self.active_tab = self.tabs.len() - 1;
    }

    pub fn close_current_tab(&mut self) -> bool {
        if self.tabs.len() <= 1 {
            return false;
        }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        true
    }

    pub fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
    }

    pub fn handle_char(&mut self, ch: char) {
        self.tabs[self.active_tab].insert_char(ch);
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
    }

    pub fn handle_backspace(&mut self) {
        self.tabs[self.active_tab].backspace();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
    }

    pub fn handle_delete(&mut self) {
        self.tabs[self.active_tab].delete();
    }

    pub fn move_cursor_left(&mut self, selecting: bool) {
        self.tabs[self.active_tab].move_left(selecting);
        self.auto_scroll();
    }

    pub fn move_cursor_right(&mut self, selecting: bool) {
        self.tabs[self.active_tab].move_right(selecting);
        self.auto_scroll();
    }

    pub fn move_cursor_word_left(&mut self, selecting: bool) {
        self.tabs[self.active_tab].move_word_left(selecting);
        self.auto_scroll();
    }

    pub fn move_cursor_word_right(&mut self, selecting: bool) {
        self.tabs[self.active_tab].move_word_right(selecting);
        self.auto_scroll();
    }

    pub fn move_cursor_up(&mut self, selecting: bool) {
        self.tabs[self.active_tab].move_up(selecting);
        self.auto_scroll();
    }

    pub fn move_cursor_down(&mut self, selecting: bool) {
        self.tabs[self.active_tab].move_down(selecting);
        self.auto_scroll();
    }

    pub fn scroll_up(&mut self) {
        self.tabs[self.active_tab].scroll_up(3);
    }

    pub fn scroll_down(&mut self) {
        self.tabs[self.active_tab].scroll_down(3);
    }

    pub fn move_cursor_to_line_start(&mut self, selecting: bool) {
        self.tabs[self.active_tab].move_to_line_start(selecting);
        self.auto_scroll();
    }

    pub fn move_cursor_to_line_end(&mut self, selecting: bool) {
        self.tabs[self.active_tab].move_to_line_end(selecting);
        self.auto_scroll();
    }

    pub fn move_cursor_to_start(&mut self, selecting: bool) {
        self.tabs[self.active_tab].move_to_start(selecting);
        self.auto_scroll();
    }

    pub fn move_cursor_to_end(&mut self, selecting: bool) {
        self.tabs[self.active_tab].move_to_end(selecting);
        self.auto_scroll();
    }

    pub fn save_current(&mut self) {
        self.tabs[self.active_tab].save();
    }

    pub fn open_file(&mut self) {
        if let Some(tab) = Tab::open() {
            self.tabs.push(tab);
            self.active_tab = self.tabs.len() - 1;
        }
    }

    /// Handle mouse click - position cursor at clicked location
    pub fn click_at(&mut self, x: f32, y: f32, selecting: bool) {
        let scrollbar_width = 12.0 * self.scale;

        // Check if clicked on scrollbar
        if x > self.width - scrollbar_width {
            let content_start_y = TAB_HEIGHT * self.scale;
            let _ = self.height - TAB_HEIGHT * self.scale;
            if y >= content_start_y {
                // Clicked on scrollbar track
                self.is_dragging_scrollbar = true;
                self.drag_at(x, y);
                return;
            }
        } else {
            self.is_dragging_scrollbar = false;
        }

        // Calculate which line was clicked
        let content_start_y = TAB_HEIGHT * self.scale + PADDING * self.scale;

        if y < content_start_y {
            // Check if clicked on New Tab button
            if let Some((bx, by, bw, bh)) = self.renderer.new_tab_button_bounds() {
                if x >= bx && x <= bx + bw && y >= by && y <= by + bh {
                    self.new_tab();
                    return;
                }
            }
            return; // Clicked in tab bar (not on button)
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
        self.auto_scroll();
    }

    pub fn handle_double_click(&mut self, x: f32, y: f32) {
        // First, place cursor (and clear previous selection)
        self.click_at(x, y, false);
        // Then select word at that cursor position
        self.tabs[self.active_tab].select_word_at_cursor();
    }

    pub fn drag_at(&mut self, x: f32, y: f32) {
        if self.is_dragging_scrollbar {
            let start_y = TAB_HEIGHT * self.scale;
            let scroll_area_height = self.height - TAB_HEIGHT * self.scale;

            // Calculate scroll ratio from Y position
            let relative_y = (y - start_y).clamp(0.0, scroll_area_height);
            let ratio = relative_y / scroll_area_height;

            let total_lines = self.tabs[self.active_tab].total_lines();
            let visible_lines = self.visible_lines();

            let max_scroll = total_lines.saturating_sub(visible_lines);
            if max_scroll > 0 {
                let new_offset = (max_scroll as f32 * ratio).round() as usize;
                self.tabs[self.active_tab].set_scroll_offset(new_offset);
            }
        } else {
            // Dragging implies selecting text
            self.click_at(x, y, true);
        }
    }

    pub fn handle_copy(&mut self) {
        if let Some(text) = self.tabs[self.active_tab].copy_selection() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
        }
    }

    pub fn handle_cut(&mut self) {
        if let Some(text) = self.tabs[self.active_tab].cut_selection() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
        }
    }

    pub fn handle_paste(&mut self) {
        if let Some(clipboard) = &mut self.clipboard {
            if let Ok(text) = clipboard.get_text() {
                self.tabs[self.active_tab].paste(&text);
                self.auto_scroll();
            }
        }
    }

    pub fn handle_select_all(&mut self) {
        self.tabs[self.active_tab].select_all();
    }
    pub fn handle_move_lines_up(&mut self) {
        self.tabs[self.active_tab].move_lines_up();
        self.auto_scroll();
    }

    pub fn handle_move_lines_down(&mut self) {
        self.tabs[self.active_tab].move_lines_down();
        self.auto_scroll();
    }

    pub fn handle_undo(&mut self) {
        self.tabs[self.active_tab].undo();
        self.auto_scroll();
    }

    pub fn handle_redo(&mut self) {
        self.tabs[self.active_tab].redo();
        self.auto_scroll();
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
