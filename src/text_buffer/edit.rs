//! Insert, delete, undo, redo operations.

use super::{TextBuffer, UndoAction};

impl TextBuffer {
    pub fn insert(&mut self, ch: char) {
        if self.has_selection() {
            self.delete_selection();
        }
        self.record_action(UndoAction::Insert {
            start: self.cursor,
            text: ch.to_string(),
        });
        self.rope.insert_char(self.cursor, ch);
        self.cursor += 1;
    }

    pub fn insert_str(&mut self, text: &str) {
        if self.has_selection() {
            self.delete_selection();
        }
        self.record_action(UndoAction::Insert {
            start: self.cursor,
            text: text.to_string(),
        });
        self.rope.insert(self.cursor, text);
        self.cursor += text.chars().count();
    }

    pub fn backspace(&mut self) {
        if self.has_selection() {
            self.delete_selection();
        } else if self.cursor > 0 {
            let deleted = self.rope.slice(self.cursor - 1..self.cursor).to_string();
            self.record_action(UndoAction::Delete { start: self.cursor - 1, text: deleted });
            self.cursor -= 1;
            self.rope.remove(self.cursor..self.cursor + 1);
        }
    }

    pub fn delete(&mut self) {
        if self.has_selection() {
            self.delete_selection();
        } else if self.cursor < self.rope.len_chars() {
            let deleted = self.rope.slice(self.cursor..self.cursor + 1).to_string();
            self.record_action(UndoAction::Delete { start: self.cursor, text: deleted });
            self.rope.remove(self.cursor..self.cursor + 1);
        }
    }

    pub fn delete_word_left(&mut self) {
        if self.has_selection() {
            self.delete_selection();
            return;
        }
        if self.cursor == 0 {
            return;
        }

        let cat = |c: char| -> u8 {
            if c.is_alphanumeric() || c == '_' { 1 } else if c.is_whitespace() { 2 } else { 3 }
        };

        let mut start = self.cursor;
        let mut space_count = 0;
        while start > 0 && cat(self.rope.char(start - 1)) == 2 {
            start -= 1;
            space_count += 1;
        }
        if space_count <= 1 {
            if space_count == 1 { start = self.cursor; }
            while start > 0 && cat(self.rope.char(start - 1)) == 2 { start -= 1; }
            if start > 0 {
                let c = cat(self.rope.char(start - 1));
                while start > 0 && cat(self.rope.char(start - 1)) == c { start -= 1; }
            }
        }

        if start < self.cursor {
            let removed = self.rope.slice(start..self.cursor).to_string();
            self.record_action(UndoAction::Delete { start, text: removed });
            self.rope.remove(start..self.cursor);
            self.cursor = start;
        }
    }

    pub fn delete_word_right(&mut self) {
        if self.has_selection() {
            self.delete_selection();
            return;
        }
        let len = self.rope.len_chars();
        if self.cursor >= len { return; }

        let cat = |c: char| -> u8 {
            if c.is_alphanumeric() || c == '_' { 1 } else if c.is_whitespace() { 2 } else { 3 }
        };

        let mut end = self.cursor;
        if end < len {
            let c = cat(self.rope.char(end));
            while end < len && cat(self.rope.char(end)) == c { end += 1; }
        }

        if end > self.cursor {
            let removed = self.rope.slice(self.cursor..end).to_string();
            self.record_action(UndoAction::Delete { start: self.cursor, text: removed });
            self.rope.remove(self.cursor..end);
        }
    }

    pub fn undo(&mut self) {
        if let Some(action) = self.undo_stack.pop() {
            match action.clone() {
                UndoAction::Insert { start, text } => {
                    let n = text.chars().count();
                    self.rope.remove(start..start + n);
                    self.cursor = start;
                }
                UndoAction::Delete { start, text } => {
                    self.rope.insert(start, &text);
                    self.cursor = start + text.chars().count();
                }
                UndoAction::Replace { start, old_text, new_text } => {
                    let n = new_text.chars().count();
                    self.rope.remove(start..start + n);
                    self.rope.insert(start, &old_text);
                    self.cursor = start + old_text.chars().count();
                }
            }
            self.redo_stack.push(action);
            self.selection_anchor = None;
        }
    }

    pub fn redo(&mut self) {
        if let Some(action) = self.redo_stack.pop() {
            match action.clone() {
                UndoAction::Insert { start, text } => {
                    self.rope.insert(start, &text);
                    self.cursor = start + text.chars().count();
                }
                UndoAction::Delete { start, text } => {
                    let n = text.chars().count();
                    self.rope.remove(start..start + n);
                    self.cursor = start;
                }
                UndoAction::Replace { start, old_text, new_text } => {
                    let n = old_text.chars().count();
                    self.rope.remove(start..start + n);
                    self.rope.insert(start, &new_text);
                    self.cursor = start + new_text.chars().count();
                }
            }
            self.undo_stack.push(action);
            self.selection_anchor = None;
        }
    }
}
