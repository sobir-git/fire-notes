//! Cursor movement methods.

use super::TextBuffer;

impl TextBuffer {
    pub fn move_left(&mut self, selecting: bool) {
        if selecting { self.start_selection(); } else { self.clear_selection(); }
        if self.cursor > 0 { self.cursor -= 1; }
    }

    pub fn move_right(&mut self, selecting: bool) {
        if selecting { self.start_selection(); } else { self.clear_selection(); }
        if self.cursor < self.rope.len_chars() { self.cursor += 1; }
    }

    pub fn move_word_left(&mut self, selecting: bool) {
        if selecting { self.start_selection(); } else { self.clear_selection(); }
        if self.cursor == 0 { return; }

        let cat = |c: char| -> u8 {
            if c.is_alphanumeric() || c == '_' { 1 } else if c.is_whitespace() { 2 } else { 3 }
        };
        let mut pos = self.cursor;
        while pos > 0 && cat(self.rope.char(pos - 1)) == 2 { pos -= 1; }
        if pos > 0 {
            let c = cat(self.rope.char(pos - 1));
            while pos > 0 && cat(self.rope.char(pos - 1)) == c { pos -= 1; }
        }
        self.cursor = pos;
    }

    pub fn move_word_right(&mut self, selecting: bool) {
        if selecting { self.start_selection(); } else { self.clear_selection(); }
        let len = self.rope.len_chars();
        if self.cursor >= len { return; }

        let cat = |c: char| -> u8 {
            if c.is_alphanumeric() || c == '_' { 1 } else if c.is_whitespace() { 2 } else { 3 }
        };
        let mut pos = self.cursor;
        while pos < len && cat(self.rope.char(pos)) == 2 { pos += 1; }
        if pos < len {
            let c = cat(self.rope.char(pos));
            while pos < len && cat(self.rope.char(pos)) == c { pos += 1; }
        }
        self.cursor = pos;
    }

    pub fn move_up(&mut self, selecting: bool) {
        if selecting { self.start_selection(); } else { self.clear_selection(); }
        let line = self.rope.char_to_line(self.cursor);
        if line == 0 { self.cursor = 0; return; }
        let line_start = self.rope.line_to_char(line);
        let col = self.cursor - line_start;
        let prev_start = self.rope.line_to_char(line - 1);
        let prev_len = self.rope.line(line - 1).len_chars().saturating_sub(1);
        self.cursor = prev_start + col.min(prev_len);
    }

    pub fn move_down(&mut self, selecting: bool) {
        if selecting { self.start_selection(); } else { self.clear_selection(); }
        let line = self.rope.char_to_line(self.cursor);
        let total = self.rope.len_lines();
        if line >= total.saturating_sub(1) { self.cursor = self.rope.len_chars(); return; }
        let line_start = self.rope.line_to_char(line);
        let col = self.cursor - line_start;
        let next_start = self.rope.line_to_char(line + 1);
        let next_len = if line + 1 < total - 1 {
            self.rope.line(line + 1).len_chars().saturating_sub(1)
        } else {
            self.rope.line(line + 1).len_chars()
        };
        self.cursor = next_start + col.min(next_len);
    }

    pub fn move_to_line_start(&mut self, selecting: bool) {
        if selecting { self.start_selection(); } else { self.clear_selection(); }
        let line = self.rope.char_to_line(self.cursor);
        self.cursor = self.rope.line_to_char(line);
    }

    pub fn move_to_line_end(&mut self, selecting: bool) {
        if selecting { self.start_selection(); } else { self.clear_selection(); }
        let line = self.rope.char_to_line(self.cursor);
        let line_start = self.rope.line_to_char(line);
        let line_len = self.rope.line(line).len_chars();
        let mut end = line_start + line_len;
        if end > line_start && self.rope.char(end - 1) == '\n' { end -= 1; }
        self.cursor = end;
    }

    pub fn move_to_start(&mut self, selecting: bool) {
        if selecting { self.start_selection(); } else { self.clear_selection(); }
        self.cursor = 0;
    }

    pub fn move_to_end(&mut self, selecting: bool) {
        if selecting { self.start_selection(); } else { self.clear_selection(); }
        self.cursor = self.rope.len_chars();
    }

    pub fn set_cursor_by_line_col(&mut self, line: usize, col: usize, selecting: bool) {
        if selecting { self.start_selection(); } else { self.clear_selection(); }
        let total = self.rope.len_lines();
        if total == 0 { self.cursor = 0; return; }
        let target_line = line.min(total.saturating_sub(1));
        let line_start = self.rope.line_to_char(target_line);
        let line_len = self.rope.line(target_line).len_chars();
        let effective_len = if target_line < total - 1 { line_len.saturating_sub(1) } else { line_len };
        self.cursor = line_start + col.min(effective_len);
    }
}
