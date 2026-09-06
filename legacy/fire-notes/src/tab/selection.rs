//! Selection and clipboard helpers for Tab.

use super::Tab;

impl Tab {
    #[allow(dead_code)]
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.buffer.selection_range()
    }

    pub fn selection_range_line_col(&self) -> Option<((usize, usize), (usize, usize))> {
        let (start, end) = self.buffer.selection_range()?;
        Some((
            self.buffer.char_to_line_col(start),
            self.buffer.char_to_line_col(end),
        ))
    }

    pub fn select_all(&mut self) { self.buffer.select_all(); }
    pub fn select_word_at_cursor(&mut self) { self.buffer.select_word_at_cursor(); }
    pub fn select_line_at_cursor(&mut self) { self.buffer.select_line_at_cursor(); }

    pub fn copy_selection(&self) -> Option<String> {
        let text = self.buffer.selected_text();
        if text.is_empty() { None } else { Some(text) }
    }

    pub fn cut_selection(&mut self) -> Option<String> {
        let text = self.copy_selection();
        if text.is_some() {
            self.buffer.delete_selection();
            self.modified = true;
        }
        text
    }

    pub fn paste_text(&mut self, text: &str) -> bool {
        if !text.is_empty() {
            self.buffer.insert_str(text);
            self.modified = true;
            return true;
        }
        false
    }
}
