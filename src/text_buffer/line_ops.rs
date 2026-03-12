//! Line move operations (move line up / down).

use super::TextBuffer;

impl TextBuffer {
    pub fn move_lines_up(&mut self) {
        let (start_line, _) = self.get_line_range_to_move();
        if start_line == 0 { return; }
        let swap_target_line = start_line - 1;

        self.ensure_trailing_newline();
        let (start_line, end_line) = self.get_line_range_to_move();

        let target_start_char = self.rope.line_to_char(swap_target_line);
        let block_start_char  = self.rope.line_to_char(start_line);
        let block_end_char    = self.rope.line_to_char(end_line + 1);
        let block_text = self.rope.slice(block_start_char..block_end_char).to_string();

        self.rope.remove(block_start_char..block_end_char);
        self.rope.insert(target_start_char, &block_text);

        let move_amount = block_start_char - target_start_char;
        self.cursor -= move_amount;
        if let Some(anchor) = self.selection_anchor {
            self.selection_anchor = Some(anchor - move_amount);
        }
    }

    pub fn move_lines_down(&mut self) {
        let (_, end_line) = self.get_line_range_to_move();
        if end_line + 1 >= self.rope.len_lines() { return; }

        self.ensure_trailing_newline();
        let (start_line, end_line) = self.get_line_range_to_move();
        if end_line + 1 >= self.rope.len_lines() { return; }

        let block_start_char = self.rope.line_to_char(start_line);
        let block_end_char   = self.rope.line_to_char(end_line + 1);
        let block_len  = block_end_char - block_start_char;
        let block_text = self.rope.slice(block_start_char..block_end_char).to_string();

        let insertion_char_idx = self.rope.line_to_char(end_line + 2);
        self.rope.remove(block_start_char..block_end_char);
        let new_insertion_idx = insertion_char_idx - block_len;
        self.rope.insert(new_insertion_idx, &block_text);

        let move_up_len = new_insertion_idx - block_start_char;
        self.cursor += move_up_len;
        if let Some(anchor) = self.selection_anchor {
            self.selection_anchor = Some(anchor + move_up_len);
        }
    }

    fn ensure_trailing_newline(&mut self) {
        let len = self.rope.len_chars();
        if len > 0 && self.rope.char(len - 1) != '\n' {
            self.rope.insert_char(len, '\n');
        }
    }

    pub(crate) fn get_line_range_to_move(&self) -> (usize, usize) {
        if let Some((start, end)) = self.selection_range() {
            let start_line = self.rope.char_to_line(start);
            let mut end_line = self.rope.char_to_line(end);
            if end > 0 && end == self.rope.line_to_char(end_line) {
                end_line = end_line.saturating_sub(1);
            }
            (start_line, end_line)
        } else {
            let line = self.rope.char_to_line(self.cursor);
            (line, line)
        }
    }
}

impl Default for TextBuffer {
    fn default() -> Self { Self::new() }
}
