//! Selection management and clipboard helpers.

use super::{TextBuffer, UndoAction};

impl TextBuffer {
    pub fn start_selection(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    pub fn has_selection(&self) -> bool {
        self.selection_anchor.is_some() && self.selection_anchor != Some(self.cursor)
    }

    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.and_then(|anchor| {
            if anchor == self.cursor { None }
            else if anchor < self.cursor { Some((anchor, self.cursor)) }
            else { Some((self.cursor, anchor)) }
        })
    }

    pub fn selected_text(&self) -> String {
        self.selection_range()
            .map(|(s, e)| self.rope.slice(s..e).to_string())
            .unwrap_or_default()
    }

    pub fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.cursor = self.rope.len_chars();
    }

    pub fn delete_selection(&mut self) {
        if let Some((start, end)) = self.selection_range() {
            let text = self.rope.slice(start..end).to_string();
            self.record_action(UndoAction::Delete { start, text });
            self.rope.remove(start..end);
            self.cursor = start;
            self.selection_anchor = None;
        }
    }

    pub fn select_word_at_cursor(&mut self) {
        let len = self.rope.len_chars();
        if len == 0 { return; }
        let check_idx = if self.cursor == len { len - 1 } else { self.cursor };
        let cat = |c: char| -> u8 {
            if c.is_alphanumeric() || c == '_' { 1 } else if c.is_whitespace() { 2 } else { 3 }
        };
        let target = cat(self.rope.char(check_idx));
        let mut start = check_idx;
        while start > 0 && cat(self.rope.char(start - 1)) == target { start -= 1; }
        let mut end = check_idx + 1;
        while end < len && cat(self.rope.char(end)) == target { end += 1; }
        self.selection_anchor = Some(start);
        self.cursor = end;
    }

    pub fn select_line_at_cursor(&mut self) {
        let len = self.rope.len_chars();
        if len == 0 { return; }
        let line_idx = self.rope.char_to_line(self.cursor);
        let start = self.rope.line_to_char(line_idx);
        let end = if line_idx + 1 < self.rope.len_lines() {
            self.rope.line_to_char(line_idx + 1)
        } else {
            len
        };
        self.selection_anchor = Some(start);
        self.cursor = end;
    }
}
