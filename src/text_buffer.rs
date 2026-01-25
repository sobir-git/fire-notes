//! Efficient text buffer using ropey (rope data structure)
//! O(log n) insertions and deletions

use ropey::Rope;

#[derive(Clone, Debug)]
enum Action {
    Insert {
        start: usize,
        text: String,
    },
    Delete {
        start: usize,
        text: String,
    },
    #[allow(dead_code)]
    Replace {
        start: usize,
        old_text: String,
        new_text: String,
    },
}

pub struct TextBuffer {
    rope: Rope,
    cursor: usize,                   // Character position (also end of selection)
    selection_anchor: Option<usize>, // Start of selection (None = no selection)
    undo_stack: Vec<Action>,
    redo_stack: Vec<Action>,
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            cursor: 0,
            selection_anchor: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            cursor: 0,
            selection_anchor: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn content(&self) -> &str {
        // For small buffers, this is fine. For large ones, we'd iterate chunks.
        // Using a temporary solution that works for most use cases
        // In production, we'd use rope's slice/chunk iteration
        self.rope.slice(..).as_str().unwrap_or("")
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.rope.len_chars()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn insert(&mut self, ch: char) {
        if self.has_selection() {
            self.delete_selection();
        }
        self.record_action(Action::Insert {
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
        self.record_action(Action::Insert {
            start: self.cursor,
            text: text.to_string(),
        });
        self.rope.insert(self.cursor, text);
        self.cursor += text.chars().count();
    }

    pub fn undo(&mut self) {
        if let Some(action) = self.undo_stack.pop() {
            match action.clone() {
                Action::Insert { start, text } => {
                    // Undo insert = delete
                    let char_count = text.chars().count();
                    self.rope.remove(start..start + char_count);
                    self.cursor = start;
                }
                Action::Delete { start, text } => {
                    // Undo delete = insert
                    self.rope.insert(start, &text);
                    self.cursor = start + text.chars().count();
                }
                Action::Replace {
                    start,
                    old_text,
                    new_text,
                } => {
                    // Undo replace = delete new, insert old
                    let new_len = new_text.chars().count();
                    self.rope.remove(start..start + new_len);
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
                Action::Insert { start, text } => {
                    // Redo insert = insert
                    self.rope.insert(start, &text);
                    self.cursor = start + text.chars().count();
                }
                Action::Delete { start, text } => {
                    // Redo delete = delete
                    let char_count = text.chars().count();
                    self.rope.remove(start..start + char_count);
                    self.cursor = start;
                }
                Action::Replace {
                    start,
                    old_text,
                    new_text,
                } => {
                    // Redo replace = delete old, insert new
                    let old_len = old_text.chars().count();
                    self.rope.remove(start..start + old_len);
                    self.rope.insert(start, &new_text);
                    self.cursor = start + new_text.chars().count();
                }
            }
            self.undo_stack.push(action);
            self.selection_anchor = None;
        }
    }

    fn record_action(&mut self, action: Action) {
        self.undo_stack.push(action);
        self.redo_stack.clear();
    }

    pub fn backspace(&mut self) {
        if self.has_selection() {
            self.delete_selection();
        } else if self.cursor > 0 {
            let char_to_delete = self.rope.slice(self.cursor - 1..self.cursor).to_string();
            self.record_action(Action::Delete {
                start: self.cursor - 1,
                text: char_to_delete,
            });
            self.cursor -= 1;
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

        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
        let is_whitespace = |c: char| c.is_whitespace();
        let category_check = |c: char| -> u8 {
            if is_word_char(c) {
                1
            } else if is_whitespace(c) {
                2
            } else {
                3
            }
        };

        let mut start = self.cursor;

        // Count consecutive spaces immediately before cursor
        let mut space_count = 0;
        while start > 0 && category_check(self.rope.char(start - 1)) == 2 {
            start -= 1;
            space_count += 1;
        }

        if space_count > 1 {
            // More than one space: treat spaces as a word, already deleted above
        } else {
            // Single space or not space: normal word deletion
            // If we deleted a single space, restore it and do normal word deletion
            if space_count == 1 {
                start = self.cursor;
            }
            // Normal word deletion: skip trailing whitespace first
            while start > 0 && category_check(self.rope.char(start - 1)) == 2 {
                start -= 1;
            }

            if start > 0 {
                let category = category_check(self.rope.char(start - 1));
                while start > 0 && category_check(self.rope.char(start - 1)) == category {
                    start -= 1;
                }
            }
        }

        if start < self.cursor {
            let removed_text = self.rope.slice(start..self.cursor).to_string();
            self.record_action(Action::Delete {
                start,
                text: removed_text,
            });
            self.rope.remove(start..self.cursor);
            self.cursor = start;
        }
    }

    pub fn delete(&mut self) {
        if self.has_selection() {
            self.delete_selection();
        } else if self.cursor < self.rope.len_chars() {
            let char_to_delete = self.rope.slice(self.cursor..self.cursor + 1).to_string();
            self.record_action(Action::Delete {
                start: self.cursor,
                text: char_to_delete,
            });
            self.rope.remove(self.cursor..self.cursor + 1);
        }
    }

    pub fn move_left(&mut self, selecting: bool) {
        if selecting {
            self.start_selection();
        } else {
            self.clear_selection();
        }

        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn move_right(&mut self, selecting: bool) {
        if selecting {
            self.start_selection();
        } else {
            self.clear_selection();
        }

        if self.cursor < self.rope.len_chars() {
            self.cursor += 1;
        }
    }

    pub fn move_word_left(&mut self, selecting: bool) {
        if selecting {
            self.start_selection();
        } else {
            self.clear_selection();
        }

        if self.cursor == 0 {
            return;
        }

        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
        let is_whitespace = |c: char| c.is_whitespace();
        let category_check = |c: char| -> u8 {
            if is_word_char(c) {
                1
            } else if is_whitespace(c) {
                2
            } else {
                3
            }
        };

        let mut pos = self.cursor;

        // 1. Skip whitespace backwards
        while pos > 0 && category_check(self.rope.char(pos - 1)) == 2 {
            pos -= 1;
        }

        if pos > 0 {
            // 2. Determine category of what's left
            let cat = category_check(self.rope.char(pos - 1));
            // 3. Skip same category
            while pos > 0 && category_check(self.rope.char(pos - 1)) == cat {
                pos -= 1;
            }
        }

        self.cursor = pos;
    }

    pub fn move_word_right(&mut self, selecting: bool) {
        if selecting {
            self.start_selection();
        } else {
            self.clear_selection();
        }

        let len = self.rope.len_chars();
        if self.cursor >= len {
            return;
        }

        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
        let is_whitespace = |c: char| c.is_whitespace();
        let category_check = |c: char| -> u8 {
            if is_word_char(c) {
                1
            } else if is_whitespace(c) {
                2
            } else {
                3
            }
        };

        let mut pos = self.cursor;

        // 1. Skip whitespace forwards
        while pos < len && category_check(self.rope.char(pos)) == 2 {
            pos += 1;
        }

        if pos < len {
            // 2. Determine category of token
            let cat = category_check(self.rope.char(pos));
            // 3. Skip same category
            while pos < len && category_check(self.rope.char(pos)) == cat {
                pos += 1;
            }
        }

        self.cursor = pos;
    }

    pub fn move_up(&mut self, selecting: bool) {
        if selecting {
            self.start_selection();
        } else {
            self.clear_selection();
        }

        // Find current line and column
        let line = self.rope.char_to_line(self.cursor);
        if line == 0 {
            self.cursor = 0;
            return;
        }

        let line_start = self.rope.line_to_char(line);
        let col = self.cursor - line_start;

        // Move to previous line, same column if possible
        let prev_line_start = self.rope.line_to_char(line - 1);
        let prev_line_len = self.rope.line(line - 1).len_chars().saturating_sub(1); // Exclude newline
        self.cursor = prev_line_start + col.min(prev_line_len);
    }

    pub fn move_down(&mut self, selecting: bool) {
        if selecting {
            self.start_selection();
        } else {
            self.clear_selection();
        }

        let line = self.rope.char_to_line(self.cursor);
        let total_lines = self.rope.len_lines();

        if line >= total_lines.saturating_sub(1) {
            self.cursor = self.rope.len_chars();
            return;
        }

        let line_start = self.rope.line_to_char(line);
        let col = self.cursor - line_start;

        // Move to next line, same column if possible
        let next_line_start = self.rope.line_to_char(line + 1);
        let next_line_len = if line + 1 < total_lines - 1 {
            self.rope.line(line + 1).len_chars().saturating_sub(1)
        } else {
            self.rope.line(line + 1).len_chars()
        };
        self.cursor = next_line_start + col.min(next_line_len);
    }

    pub fn move_to_line_start(&mut self, selecting: bool) {
        if selecting {
            self.start_selection();
        } else {
            self.clear_selection();
        }

        let line = self.rope.char_to_line(self.cursor);
        self.cursor = self.rope.line_to_char(line);
    }

    pub fn move_to_line_end(&mut self, selecting: bool) {
        if selecting {
            self.start_selection();
        } else {
            self.clear_selection();
        }

        let line = self.rope.char_to_line(self.cursor);
        let line_len = self.rope.line(line).len_chars();

        let line_start = self.rope.line_to_char(line);
        let mut end = line_start + line_len;

        if end > line_start {
            let prev = end - 1;
            if self.rope.char(prev) == '\n' {
                end = prev;
            }
        }

        self.cursor = end;
    }

    pub fn move_to_start(&mut self, selecting: bool) {
        if selecting {
            self.start_selection();
        } else {
            self.clear_selection();
        }
        self.cursor = 0;
    }

    pub fn move_to_end(&mut self, selecting: bool) {
        if selecting {
            self.start_selection();
        } else {
            self.clear_selection();
        }
        self.cursor = self.rope.len_chars();
    }

    /// Set cursor by line and column number
    pub fn set_cursor_by_line_col(&mut self, line: usize, col: usize, selecting: bool) {
        if selecting {
            self.start_selection();
        } else {
            self.clear_selection();
        }
        let total_lines = self.rope.len_lines();
        if total_lines == 0 {
            self.cursor = 0;
            return;
        }

        // Clamp line to valid range
        let target_line = line.min(total_lines.saturating_sub(1));

        // Get line start and length
        let line_start = self.rope.line_to_char(target_line);
        let line_content = self.rope.line(target_line);
        let line_len = line_content.len_chars();

        // Handle newline character at end of line
        let effective_line_len = if target_line < total_lines - 1 {
            line_len.saturating_sub(1) // Exclude newline
        } else {
            line_len // Last line might not have newline
        };

        // Clamp column to line length
        let target_col = col.min(effective_line_len);

        self.cursor = line_start + target_col;
    }

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
        if let Some(anchor) = self.selection_anchor {
            if anchor == self.cursor {
                None
            } else if anchor < self.cursor {
                Some((anchor, self.cursor))
            } else {
                Some((self.cursor, anchor))
            }
        } else {
            None
        }
    }

    pub fn selected_text(&self) -> String {
        if let Some((start, end)) = self.selection_range() {
            self.rope.slice(start..end).to_string()
        } else {
            String::new()
        }
    }

    pub fn select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.cursor = self.rope.len_chars();
    }

    pub fn delete_selection(&mut self) {
        if let Some((start, end)) = self.selection_range() {
            let text = self.rope.slice(start..end).to_string();
            self.record_action(Action::Delete { start, text });
            self.rope.remove(start..end);
            self.cursor = start;
            self.selection_anchor = None;
        }
    }

    pub fn select_word_at_cursor(&mut self) {
        let len = self.rope.len_chars();
        if len == 0 {
            return;
        }

        // If cursor is at end, select backward from last char
        // Otherwise select based on char after cursor (which acts as "under" cursor)
        let check_idx = if self.cursor == len {
            len - 1
        } else {
            self.cursor
        };
        let char_at_cursor = self.rope.char(check_idx);

        // Define word character categories
        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
        let is_whitespace = |c: char| c.is_whitespace();

        // Determine category of clicked character
        let category_check = |c: char| -> u8 {
            if is_word_char(c) {
                1
            } else if is_whitespace(c) {
                2
            } else {
                3
            }
        };

        let target_category = category_check(char_at_cursor);

        // Scan backwards
        let mut start = check_idx;
        while start > 0 {
            let prev_char = self.rope.char(start - 1);
            if category_check(prev_char) != target_category {
                break;
            }
            start -= 1;
        }

        // Scan forwards
        let mut end = check_idx + 1;
        while end < len {
            let next_char = self.rope.char(end);
            if category_check(next_char) != target_category {
                break;
            }
            end += 1;
        }

        self.selection_anchor = Some(start);
        self.cursor = end;
    }

    pub fn select_line_at_cursor(&mut self) {
        let len = self.rope.len_chars();
        if len == 0 {
            return;
        }

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

    pub fn char_to_line_col(&self, char_idx: usize) -> (usize, usize) {
        let line = self.rope.char_to_line(char_idx);
        let line_start = self.rope.line_to_char(line);
        let col = char_idx - line_start;
        (line, col)
    }

    /// Move current line or selected lines up one line
    pub fn move_lines_up(&mut self) {
        let (start_line, _end_line) = self.get_line_range_to_move();

        if start_line == 0 {
            return; // Cannot move top line up
        }

        let swap_target_line = start_line - 1;

        // Ensure we have clean newline boundaries
        // 1. Check if the file ends with newline. If not, and we are touching the last line, append one.
        if self.rope.len_chars() > 0 {
            let last_char_idx = self.rope.len_chars() - 1;
            if self.rope.char(last_char_idx) != '\n' {
                self.rope.insert_char(self.rope.len_chars(), '\n');
                // If selection encompasses end, adjust it
                if let Some(anchor) = self.selection_anchor {
                    if anchor > self.cursor {
                        // Anchor was at end, now it's before the newline we added?
                        // Actually if we append newline, existing indices are valid.
                        // But we want to ensure "last line" conceptually has a newline for swapping.
                    }
                }
            }
        }

        // Re-calculate lines because index might have changed if we inserted newline
        let (start_line, end_line) = self.get_line_range_to_move();

        let target_start_char = self.rope.line_to_char(swap_target_line);
        let block_start_char = self.rope.line_to_char(start_line);
        let block_end_char = self.rope.line_to_char(end_line + 1);

        let block_text = self
            .rope
            .slice(block_start_char..block_end_char)
            .to_string();

        // Remove block
        self.rope.remove(block_start_char..block_end_char);

        // Insert at target
        self.rope.insert(target_start_char, &block_text);

        // Adjust cursor/selection
        let move_amount = block_start_char - target_start_char;
        self.cursor -= move_amount;
        if let Some(anchor) = self.selection_anchor {
            self.selection_anchor = Some(anchor - move_amount);
        }
    }

    /// Move current line or selected lines down one line
    pub fn move_lines_down(&mut self) {
        let (_start_line, end_line) = self.get_line_range_to_move();
        let total_lines = self.rope.len_lines();

        if end_line + 1 >= total_lines {
            return;
        }

        // Ensure newline at EOF if needed to simplify logic
        if self.rope.len_chars() > 0 {
            let last_char_idx = self.rope.len_chars() - 1;
            if self.rope.char(last_char_idx) != '\n' {
                self.rope.insert_char(self.rope.len_chars(), '\n');
            }
        }

        // Recalculate ranges
        let (start_line, end_line) = self.get_line_range_to_move();
        let total_lines = self.rope.len_lines();

        if end_line + 1 >= total_lines {
            return;
        }

        let block_start_char = self.rope.line_to_char(start_line);
        let block_end_char = self.rope.line_to_char(end_line + 1);
        let block_len = block_end_char - block_start_char;

        let block_text = self
            .rope
            .slice(block_start_char..block_end_char)
            .to_string();

        // Calculate insertion point (after the line below)
        let target_line_below = end_line + 1;
        let insertion_char_idx = self.rope.line_to_char(target_line_below + 1);

        // Remove block
        self.rope.remove(block_start_char..block_end_char);

        // Insert block
        // We removed text *before* insertion point, so split index shifts
        let new_insertion_idx = insertion_char_idx - block_len;
        self.rope.insert(new_insertion_idx, &block_text);

        // Adjust cursor/selection
        let move_up_len = new_insertion_idx - block_start_char;
        self.cursor += move_up_len;
        if let Some(anchor) = self.selection_anchor {
            self.selection_anchor = Some(anchor + move_up_len);
        }
    }

    /// Helper to get the line range involved in operation
    fn get_line_range_to_move(&self) -> (usize, usize) {
        if let Some((start, end)) = self.selection_range() {
            let start_line = self.rope.char_to_line(start);
            let mut end_line = self.rope.char_to_line(end);

            // If selection ends exactly at start of next line, don't include that line
            if end > 0 && end == self.rope.line_to_char(end_line) {
                end_line = end_line.saturating_sub(1);
            }
            (start_line, end_line)
        } else {
            // Just current line
            let line = self.rope.char_to_line(self.cursor);
            (line, line)
        }
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer() {
        let buf = TextBuffer::new();
        assert!(buf.is_empty());
        assert_eq!(buf.cursor(), 0);
    }

    #[test]
    fn test_insert_single() {
        let mut buf = TextBuffer::new();
        buf.insert('a');
        assert_eq!(buf.content(), "a");
        assert_eq!(buf.cursor(), 1);
    }

    #[test]
    fn test_insert_multiple() {
        let mut buf = TextBuffer::new();
        buf.insert('H');
        buf.insert('e');
        buf.insert('l');
        buf.insert('l');
        buf.insert('o');
        assert_eq!(buf.content(), "Hello");
        assert_eq!(buf.cursor(), 5);
    }

    #[test]
    fn test_insert_str() {
        let mut buf = TextBuffer::new();
        buf.insert_str("Hello World");
        assert_eq!(buf.content(), "Hello World");
        assert_eq!(buf.cursor(), 11);
    }

    #[test]
    fn test_backspace() {
        let mut buf = TextBuffer::from_str("Hello");
        buf.cursor = 5;
        buf.backspace();
        assert_eq!(buf.content(), "Hell");
        buf.backspace();
        assert_eq!(buf.content(), "Hel");
    }

    #[test]
    fn test_backspace_at_start() {
        let mut buf = TextBuffer::from_str("Hello");
        buf.cursor = 0;
        buf.backspace();
        assert_eq!(buf.content(), "Hello"); // No change
        assert_eq!(buf.cursor(), 0);
    }

    #[test]
    fn test_delete() {
        let mut buf = TextBuffer::from_str("Hello");
        buf.cursor = 0;
        buf.delete();
        assert_eq!(buf.content(), "ello");
    }

    #[test]
    fn test_move_left_right() {
        let mut buf = TextBuffer::from_str("Hello");
        buf.cursor = 3;
        buf.move_left(false);
        assert_eq!(buf.cursor(), 2);
        buf.move_right(false);
        assert_eq!(buf.cursor(), 3);
    }

    #[test]
    fn test_move_left_at_start() {
        let mut buf = TextBuffer::from_str("Hello");
        buf.cursor = 0;
        buf.move_left(false);
        assert_eq!(buf.cursor(), 0); // No change
    }

    #[test]
    fn test_move_right_at_end() {
        let mut buf = TextBuffer::from_str("Hello");
        buf.cursor = 5;
        buf.move_right(false);
        assert_eq!(buf.cursor(), 5); // No change
    }

    #[test]
    fn test_move_word_jump() {
        let mut buf = TextBuffer::from_str("word1 word2  word3");

        // Test move_word_right
        buf.cursor = 0;
        buf.move_word_right(false);
        assert_eq!(buf.cursor(), 5); // "word1|"

        buf.move_word_right(false);
        assert_eq!(buf.cursor(), 11); // "word1 word2|"

        buf.move_word_right(false);
        assert_eq!(buf.cursor(), 18); // "word1 word2  word3|"

        // Test move_word_left
        buf.move_word_left(false);
        assert_eq!(buf.cursor(), 13); // "word1 word2  |word3"

        buf.move_word_left(false);
        assert_eq!(buf.cursor(), 6); // "word1 |word2  word3"

        buf.move_word_left(false);
        assert_eq!(buf.cursor(), 0); // "|word1 word2  word3"

        // Test with punctuation
        let mut buf2 = TextBuffer::from_str("hello... world!!!");
        buf2.cursor = 0;
        buf2.move_word_right(false);
        assert_eq!(buf2.cursor(), 5); // "hello|"
        buf2.move_word_right(false);
        assert_eq!(buf2.cursor(), 8); // "hello...|"
        buf2.move_word_right(false);
        assert_eq!(buf2.cursor(), 14); // "hello... world|"
    }

    #[test]
    fn test_multiline_navigation() {
        let mut buf = TextBuffer::from_str("Line1\nLine2\nLine3");
        buf.cursor = 7; // Position in Line2
        buf.move_up(false);
        assert!(buf.cursor() < 6); // Should be in Line1
        buf.move_down(false);
        buf.move_down(false);
        // Should be in Line3 now
        let line = buf.rope.char_to_line(buf.cursor());
        assert_eq!(line, 2);
    }

    #[test]
    fn test_from_str() {
        let buf = TextBuffer::from_str("Initial content");
        assert_eq!(buf.content(), "Initial content");
        assert_eq!(buf.len(), 15);
    }

    #[test]
    fn test_large_text() {
        let large_text = "a".repeat(100_000);
        let mut buf = TextBuffer::from_str(&large_text);
        assert_eq!(buf.len(), 100_000);

        // Insert in middle - should be fast with rope
        buf.cursor = 50_000;
        buf.insert('X');
        assert_eq!(buf.len(), 100_001);
    }

    #[test]
    fn test_selection_basic() {
        let mut buf = TextBuffer::from_str("Hello World");
        buf.cursor = 0;
        buf.move_right(true); // Select 'H'
        buf.move_right(true); // Select 'e'
        assert!(buf.has_selection());
        assert_eq!(buf.selected_text(), "He");

        buf.move_right(false); // Clear selection
        assert!(!buf.has_selection());
        assert_eq!(buf.selected_text(), "");
    }

    #[test]
    fn test_move_line_up() {
        let mut buf = TextBuffer::from_str("Line 1\nLine 2\nLine 3");
        buf.set_cursor_by_line_col(1, 0, false); // Cursor at Line 2
        buf.move_lines_up();
        assert_eq!(buf.content(), "Line 2\nLine 1\nLine 3\n");
        let (line, _) = buf.char_to_line_col(buf.cursor());
        assert_eq!(line, 0); // Should be on top line now
    }

    #[test]
    fn test_move_line_down() {
        let mut buf = TextBuffer::from_str("Line 1\nLine 2\nLine 3");
        buf.set_cursor_by_line_col(1, 0, false); // Cursor at Line 2
        buf.move_lines_down();
        assert_eq!(buf.content(), "Line 1\nLine 3\nLine 2\n");
        let (line, _) = buf.char_to_line_col(buf.cursor());
        assert_eq!(line, 2); // Should be on bottom line now
    }
    #[test]
    fn test_move_line_down_eof_no_newline() {
        // "Line 1\nLine 2" -> Line 2 has no newline
        let mut buf = TextBuffer::from_str("Line 1\nLine 2");
        buf.cursor = 0; // Cursor at Line 1
        buf.move_lines_down();
        // Expect Line 1 to move down, becoming last line.
        // Logic ensures Line 2 gets newline, and Line 1 (now at end) might not have one.
        // Result: "Line 2\nLine 1" (or "Line 2\nLine 1\n" if we enforce it)
        // With current logic, we append newline to Line 2 if missing before swap.
        // So "Line 1\nLine 2\n". Swap -> "Line 2\nLine 1\n"
        assert_eq!(buf.content(), "Line 2\nLine 1\n");
        let (line, _) = buf.char_to_line_col(buf.cursor());
        assert_eq!(line, 1);
    }
    #[test]
    fn test_undo_redo_insert() {
        let mut buf = TextBuffer::new();
        buf.insert('a');
        buf.insert('b');
        assert_eq!(buf.content(), "ab");

        buf.undo();
        assert_eq!(buf.content(), "a");
        buf.undo();
        assert_eq!(buf.content(), "");

        buf.redo();
        assert_eq!(buf.content(), "a");
        buf.redo();
        assert_eq!(buf.content(), "ab");
    }

    #[test]
    fn test_undo_redo_delete() {
        let mut buf = TextBuffer::from_str("abc");
        buf.cursor = 3;
        buf.backspace(); // deletes 'c', record Action::Delete { start: 2, text: "c" }
        assert_eq!(buf.content(), "ab");

        buf.undo();
        assert_eq!(buf.content(), "abc");

        buf.redo();
        assert_eq!(buf.content(), "ab");
    }

    #[test]
    fn test_undo_selection_delete() {
        let mut buf = TextBuffer::from_str("hello world");
        buf.cursor = 0;
        buf.move_right(true); // 'h'
        buf.move_right(true); // 'e'
        buf.move_right(true); // 'l'
        buf.move_right(true); // 'l'
        buf.move_right(true); // 'o'
        // Selection is "hello"
        buf.delete_selection();
        assert_eq!(buf.content(), " world");

        buf.undo();
        assert_eq!(buf.content(), "hello world");

        buf.redo();
        assert_eq!(buf.content(), " world");
    }
}
