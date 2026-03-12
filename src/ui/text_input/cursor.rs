#![allow(dead_code)]
//! Cursor and selection movement.

use super::TextInput;

impl TextInput {
    pub fn move_left(&mut self, selecting: bool) {
        if !selecting {
            if let Some((start, _end)) = self.selection_range() {
                self.cursor = start;
                self.selection_anchor = None;
                return;
            }
        }
        if self.cursor > 0 {
            if selecting && self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor);
            }
            let prev_char = self.text[..self.cursor].chars().last().unwrap();
            self.cursor -= prev_char.len_utf8();
            if !selecting {
                self.selection_anchor = None;
            }
        }
    }

    pub fn move_right(&mut self, selecting: bool) {
        if !selecting {
            if let Some((_, end)) = self.selection_range() {
                self.cursor = end;
                self.selection_anchor = None;
                return;
            }
        }
        if self.cursor < self.text.len() {
            if selecting && self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor);
            }
            let next_char = self.text[self.cursor..].chars().next().unwrap();
            self.cursor += next_char.len_utf8();
            if !selecting {
                self.selection_anchor = None;
            }
        }
    }

    pub fn move_word_left(&mut self, selecting: bool) {
        if selecting && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        self.cursor = self.find_word_boundary_left();
        if !selecting {
            self.selection_anchor = None;
        }
    }

    pub fn move_word_right(&mut self, selecting: bool) {
        if selecting && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        self.cursor = self.find_word_boundary_right();
        if !selecting {
            self.selection_anchor = None;
        }
    }

    pub fn move_to_start(&mut self, selecting: bool) {
        if selecting && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        self.cursor = 0;
        if !selecting {
            self.selection_anchor = None;
        }
    }

    pub fn move_to_end(&mut self, selecting: bool) {
        if selecting && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        self.cursor = self.text.len();
        if !selecting {
            self.selection_anchor = None;
        }
    }

    pub fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.cursor = self.text.len();
    }

    pub fn selected_text(&self) -> &str {
        if let Some((start, end)) = self.selection_range() {
            &self.text[start..end]
        } else {
            ""
        }
    }

    pub fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.selection_range() {
            self.text.drain(start..end);
            self.cursor = start;
            self.selection_anchor = None;
            true
        } else {
            false
        }
    }

    pub(super) fn find_word_boundary_left(&self) -> usize {
        let text = &self.text[..self.cursor];
        let mut chars = text.char_indices().rev().peekable();
        while let Some(&(_, ch)) = chars.peek() {
            if !ch.is_whitespace() { break; }
            chars.next();
        }
        let mut last_idx = 0;
        while let Some((idx, ch)) = chars.next() {
            if ch.is_whitespace() {
                last_idx = idx + ch.len_utf8();
                break;
            }
            last_idx = idx;
        }
        last_idx
    }

    pub(super) fn find_word_boundary_right(&self) -> usize {
        let text = &self.text[self.cursor..];
        let mut chars = text.char_indices().peekable();
        while let Some(&(_, ch)) = chars.peek() {
            if ch.is_whitespace() { break; }
            chars.next();
        }
        while let Some(&(_, ch)) = chars.peek() {
            if !ch.is_whitespace() { break; }
            chars.next();
        }
        if let Some(&(idx, _)) = chars.peek() {
            self.cursor + idx
        } else {
            self.text.len()
        }
    }

    pub fn set_cursor_from_x(&mut self, x: f32, char_width: f32, selecting: bool) {
        if selecting && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        let adjusted_x = x + self.scroll_offset;
        let char_index = (adjusted_x / char_width).round() as usize;
        let mut byte_idx = 0;
        for (i, ch) in self.text.chars().enumerate() {
            if i >= char_index { break; }
            byte_idx += ch.len_utf8();
        }
        self.cursor = byte_idx.min(self.text.len());
        if !selecting {
            self.selection_anchor = None;
        }
    }

    pub fn ensure_cursor_visible(&mut self, visible_width: f32, char_width: f32) {
        let cursor_char_idx = self.text[..self.cursor].chars().count();
        let cursor_x = cursor_char_idx as f32 * char_width;
        if cursor_x < self.scroll_offset {
            self.scroll_offset = cursor_x;
        }
        if cursor_x + char_width > self.scroll_offset + visible_width {
            self.scroll_offset = cursor_x - visible_width + char_width * 2.0;
        }
        self.scroll_offset = self.scroll_offset.max(0.0);
    }
}
