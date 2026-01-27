//! Single-line text input widget

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct TextInput {
    pub text: String,
    pub cursor: usize,
    pub selection_anchor: Option<usize>,
    pub scroll_offset: f32,
}

#[allow(dead_code)]
impl TextInput {
    pub fn new(text: String) -> Self {
        let cursor = text.len();
        Self {
            text,
            cursor,
            selection_anchor: None,
            scroll_offset: 0.0,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.map(|anchor| {
            if anchor < self.cursor {
                (anchor, self.cursor)
            } else {
                (self.cursor, anchor)
            }
        })
    }

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
        // Find end of current word (don't include trailing whitespace)
        let text = &self.text[self.cursor..];
        let mut end = self.cursor;
        let mut chars = text.chars().peekable();
        
        // Skip current word characters
        while let Some(&ch) = chars.peek() {
            if ch.is_whitespace() {
                break;
            }
            end += ch.len_utf8();
            chars.next();
        }
        
        // If we started on whitespace, delete just the whitespace
        if end == self.cursor {
            while let Some(&ch) = chars.peek() {
                if !ch.is_whitespace() {
                    break;
                }
                end += ch.len_utf8();
                chars.next();
            }
        }
        
        self.text.drain(self.cursor..end);
    }

    fn find_word_boundary_left(&self) -> usize {
        let text = &self.text[..self.cursor];
        let mut chars = text.char_indices().rev().peekable();
        
        // Skip trailing whitespace
        while let Some(&(_, ch)) = chars.peek() {
            if !ch.is_whitespace() {
                break;
            }
            chars.next();
        }
        
        // Skip word characters
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

    fn find_word_boundary_right(&self) -> usize {
        let text = &self.text[self.cursor..];
        let mut chars = text.char_indices().peekable();
        
        // Skip current word
        while let Some(&(_, ch)) = chars.peek() {
            if ch.is_whitespace() {
                break;
            }
            chars.next();
        }
        
        // Skip whitespace
        while let Some(&(_, ch)) = chars.peek() {
            if !ch.is_whitespace() {
                break;
            }
            chars.next();
        }
        
        if let Some(&(idx, _)) = chars.peek() {
            self.cursor + idx
        } else {
            self.text.len()
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

    pub fn paste(&mut self, text: &str) {
        self.delete_selection();
        // Filter out newlines for single-line input
        let filtered: String = text.chars().filter(|&c| c != '\n' && c != '\r').collect();
        self.text.insert_str(self.cursor, &filtered);
        self.cursor += filtered.len();
    }

    pub fn copy(&self) -> Option<String> {
        let text = self.selected_text();
        if text.is_empty() {
            None
        } else {
            Some(text.to_string())
        }
    }

    pub fn cut(&mut self) -> Option<String> {
        let copied = self.copy();
        if copied.is_some() {
            self.delete_selection();
        }
        copied
    }

    pub fn set_cursor_from_x(&mut self, x: f32, char_width: f32, selecting: bool) {
        if selecting && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        
        let adjusted_x = x + self.scroll_offset;
        let char_index = (adjusted_x / char_width).round() as usize;
        
        // Convert char index to byte index
        let mut byte_idx = 0;
        for (i, ch) in self.text.chars().enumerate() {
            if i >= char_index {
                break;
            }
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
        
        // Scroll left if cursor is before visible area
        if cursor_x < self.scroll_offset {
            self.scroll_offset = cursor_x;
        }
        
        // Scroll right if cursor is after visible area
        if cursor_x + char_width > self.scroll_offset + visible_width {
            self.scroll_offset = cursor_x - visible_width + char_width * 2.0;
        }
        
        // Don't scroll past the start
        self.scroll_offset = self.scroll_offset.max(0.0);
    }
}

