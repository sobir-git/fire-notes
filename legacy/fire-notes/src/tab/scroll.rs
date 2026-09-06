//! Scroll and cursor-visibility management for Tab.

use super::Tab;

impl Tab {
    pub fn scroll_offset(&self) -> usize { self.scroll_offset }
    pub fn scroll_offset_x(&self) -> f32 { self.scroll_offset_x }

    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    pub fn scroll_down(&mut self, lines: usize, _visible_lines: usize) {
        let max_scroll = self.buffer.len_lines().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + lines).min(max_scroll);
    }

    /// Maximum scroll offset: last line scrolled to top of viewport.
    pub fn max_scroll_offset(&self) -> usize {
        self.buffer.len_lines().saturating_sub(1)
    }

    pub fn set_scroll_offset(&mut self, offset: usize) -> bool {
        if self.scroll_offset != offset { self.scroll_offset = offset; true } else { false }
    }

    pub fn ensure_cursor_visible(
        &mut self,
        visible_lines: usize,
        visible_width: f32,
        char_width: f32,
    ) {
        let cursor_line = self.cursor_line();

        if cursor_line < self.scroll_offset {
            self.scroll_offset = cursor_line;
        }
        if cursor_line >= self.scroll_offset + visible_lines {
            self.scroll_offset = cursor_line.saturating_sub(visible_lines - 1);
        }

        if !self.word_wrap {
            let cursor_x = self.cursor_col() as f32 * char_width;
            if cursor_x < self.scroll_offset_x {
                self.scroll_offset_x = cursor_x;
            }
            if cursor_x + char_width > self.scroll_offset_x + visible_width {
                self.scroll_offset_x = cursor_x - visible_width + char_width * 2.0;
            }
        } else {
            self.scroll_offset_x = 0.0;
        }
    }
}
