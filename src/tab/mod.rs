//! Tab state — represents a single open document.

mod io;
mod scroll;
mod selection;

#[cfg(test)]
mod tests;

use crate::persistence::TabState;
use crate::text_buffer::TextBuffer;
use std::path::PathBuf;

pub struct Tab {
    pub(crate) buffer: TextBuffer,
    pub(crate) path: Option<PathBuf>,
    pub(crate) title: String,
    pub(crate) modified: bool,
    pub(crate) scroll_offset: usize,
    pub(crate) scroll_offset_x: f32,
    pub(crate) word_wrap: bool,
}

impl Tab {
    pub fn new_untitled() -> Self {
        static COUNTER: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(1);
        let num = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            buffer: TextBuffer::new(),
            path: None,
            title: format!("Untitled-{}", num),
            modified: false,
            scroll_offset: 0,
            scroll_offset_x: 0.0,
            word_wrap: false,
        }
    }

    // =========================================================================
    // Accessors
    // =========================================================================

    #[allow(dead_code)]
    pub fn path(&self) -> Option<&PathBuf> { self.path.as_ref() }

    #[allow(dead_code)]
    pub fn is_modified(&self) -> bool { self.modified }

    pub fn title(&self) -> &str { &self.title }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
        if let Some(path) = &self.path {
            let _ = crate::persistence::save_note_title(path, &self.title);
        }
    }

    pub fn content(&self) -> &str { self.buffer.content() }

    pub fn cursor_position(&self) -> usize { self.buffer.cursor() }

    pub fn total_lines(&self) -> usize { self.buffer.len_lines() }

    pub fn word_wrap(&self) -> bool { self.word_wrap }

    #[allow(dead_code)]
    pub fn set_word_wrap(&mut self, wrap: bool) { self.word_wrap = wrap; }

    pub fn toggle_word_wrap(&mut self) { self.word_wrap = !self.word_wrap; }

    pub fn cursor_line(&self) -> usize {
        let text = self.buffer.content();
        let cursor = self.buffer.cursor();
        text.chars().take(cursor).filter(|&c| c == '\n').count()
    }

    pub fn cursor_col(&self) -> usize {
        let (_, col) = self.buffer.char_to_line_col(self.buffer.cursor());
        col
    }

    pub fn visual_col_to_char_col(&self, line: usize, visual_col: usize) -> usize {
        if let Some(line_content) = self.content().lines().nth(line) {
            let vl = crate::visual_position::VisualLine::new(line_content);
            vl.visual_col_to_char_col(visual_col)
        } else {
            0
        }
    }

    pub fn set_cursor_position(&mut self, line: usize, col: usize, selecting: bool) {
        self.buffer.set_cursor_by_line_col(line, col, selecting);
    }

    // =========================================================================
    // Edit operations — thin delegation to TextBuffer
    // =========================================================================

    pub fn insert_char(&mut self, ch: char) { self.buffer.insert(ch); self.modified = true; }
    pub fn backspace(&mut self) { self.buffer.backspace(); self.modified = true; }
    pub fn delete(&mut self) { self.buffer.delete(); self.modified = true; }
    pub fn delete_word_left(&mut self) { self.buffer.delete_word_left(); self.modified = true; }
    pub fn delete_word_right(&mut self) { self.buffer.delete_word_right(); self.modified = true; }
    pub fn move_left(&mut self, s: bool) { self.buffer.move_left(s); }
    pub fn move_right(&mut self, s: bool) { self.buffer.move_right(s); }
    pub fn move_word_left(&mut self, s: bool) { self.buffer.move_word_left(s); }
    pub fn move_word_right(&mut self, s: bool) { self.buffer.move_word_right(s); }
    pub fn move_up(&mut self, s: bool) { self.buffer.move_up(s); }
    pub fn move_down(&mut self, s: bool) { self.buffer.move_down(s); }
    pub fn move_to_line_start(&mut self, s: bool) { self.buffer.move_to_line_start(s); }
    pub fn move_to_line_end(&mut self, s: bool) { self.buffer.move_to_line_end(s); }
    /// Character count of `line` excluding any trailing newline.
    pub fn line_char_len(&self, line: usize) -> usize {
        self.content()
            .lines()
            .nth(line)
            .map(|l| l.chars().count())
            .unwrap_or(0)
    }

    pub fn move_to_start(&mut self, s: bool) { self.buffer.move_to_start(s); }
    pub fn move_to_end(&mut self, s: bool) { self.buffer.move_to_end(s); }
    pub fn move_lines_up(&mut self) -> bool { self.buffer.move_lines_up(); true }
    pub fn move_lines_down(&mut self) -> bool { self.buffer.move_lines_down(); true }
    pub fn undo(&mut self) -> bool { self.buffer.undo(); true }
    pub fn redo(&mut self) -> bool { self.buffer.redo(); true }

    // =========================================================================
    // Session state
    // =========================================================================

    pub fn export_state(&self) -> Option<TabState> {
        let path = self.path.clone()?;
        Some(TabState {
            path,
            cursor_line: self.cursor_line(),
            cursor_col: self.cursor_col(),
            scroll_offset: self.scroll_offset,
            scroll_offset_x: self.scroll_offset_x,
            word_wrap: self.word_wrap,
        })
    }

    pub fn apply_state(&mut self, state: &TabState) {
        self.set_cursor_position(state.cursor_line, state.cursor_col, false);
        self.scroll_offset = state.scroll_offset;
        self.scroll_offset_x = state.scroll_offset_x.max(0.0);
        self.word_wrap = state.word_wrap;
    }
}
