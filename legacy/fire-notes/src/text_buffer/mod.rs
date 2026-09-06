//! Efficient text buffer using ropey (rope data structure).
//! O(log n) insertions and deletions.

mod cursor;
mod edit;
mod line_ops;
mod selection;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod proptest_tests;

use ropey::Rope;

#[derive(Clone, Debug)]
pub(crate) enum UndoAction {
    Insert { start: usize, text: String },
    Delete { start: usize, text: String },
    #[allow(dead_code)]
    Replace { start: usize, old_text: String, new_text: String },
}

pub struct TextBuffer {
    pub(crate) rope: Rope,
    pub(crate) cursor: usize,
    pub(crate) selection_anchor: Option<usize>,
    pub(crate) undo_stack: Vec<UndoAction>,
    pub(crate) redo_stack: Vec<UndoAction>,
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

    pub fn char_to_line_col(&self, char_idx: usize) -> (usize, usize) {
        let line = self.rope.char_to_line(char_idx);
        let line_start = self.rope.line_to_char(line);
        (line, char_idx - line_start)
    }

    pub(crate) fn record_action(&mut self, action: UndoAction) {
        self.undo_stack.push(action);
        self.redo_stack.clear();
    }
}
