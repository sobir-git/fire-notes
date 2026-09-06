//! Edit operations — insert, delete, paste, cut, copy.

use super::TextInput;

#[allow(dead_code)]
impl TextInput {
    pub fn insert_char(&mut self, ch: char) {
        if ch.is_control() {
            return;
        }
        self.delete_selection();
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor > 0 {
            let prev_char = self.text[..self.cursor].chars().last().unwrap();
            self.cursor -= prev_char.len_utf8();
            self.text.remove(self.cursor);
        }
    }

    pub fn delete(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor < self.text.len() {
            self.text.remove(self.cursor);
        }
    }

    pub fn delete_word_left(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor == 0 {
            return;
        }
        let start = self.find_word_boundary_left();
        self.text.drain(start..self.cursor);
        self.cursor = start;
    }

    pub fn delete_word_right(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor >= self.text.len() {
            return;
        }
        let text = &self.text[self.cursor..];
        let mut end = self.cursor;
        let mut chars = text.chars().peekable();
        while let Some(&ch) = chars.peek() {
            if ch.is_whitespace() { break; }
            end += ch.len_utf8();
            chars.next();
        }
        if end == self.cursor {
            while let Some(&ch) = chars.peek() {
                if !ch.is_whitespace() { break; }
                end += ch.len_utf8();
                chars.next();
            }
        }
        self.text.drain(self.cursor..end);
    }

    pub fn paste(&mut self, text: &str) {
        self.delete_selection();
        let filtered: String = text.chars().filter(|&c| c != '\n' && c != '\r').collect();
        self.text.insert_str(self.cursor, &filtered);
        self.cursor += filtered.len();
    }

    pub fn copy(&self) -> Option<String> {
        let text = self.selected_text();
        if text.is_empty() { None } else { Some(text.to_string()) }
    }

    pub fn cut(&mut self) -> Option<String> {
        let copied = self.copy();
        if copied.is_some() {
            self.delete_selection();
        }
        copied
    }
}
