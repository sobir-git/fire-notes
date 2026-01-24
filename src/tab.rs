//! Tab state - represents a single open file

use crate::persistence::{self, TabState};
use crate::text_buffer::TextBuffer;
use native_dialog::FileDialog;
use std::fs;
use std::path::PathBuf;

pub struct Tab {
    buffer: TextBuffer,
    path: Option<PathBuf>,
    title: String,
    modified: bool,
    scroll_offset: usize, // Line offset for scrolling
    scroll_offset_x: f32, // Horizontal pixel offset
    word_wrap: bool,
}

impl Tab {
    pub fn new_untitled() -> Self {
        static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
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

    pub fn from_file(path: PathBuf) -> Option<Self> {
        let content = fs::read_to_string(&path).ok()?;
        let title = persistence::load_note_title(&path).unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string()
        });

        Some(Self {
            buffer: TextBuffer::from_str(&content),
            path: Some(path),
            title,
            modified: false,
            scroll_offset: 0,
            scroll_offset_x: 0.0,
            word_wrap: false,
        })
    }

    pub fn open() -> Option<Self> {
        let path = FileDialog::new()
            .add_filter("Markdown", &["md", "markdown", "txt"])
            .show_open_single_file()
            .ok()??;
        Self::from_file(path)
    }

    pub fn save(&mut self) {
        let path = match &self.path {
            Some(p) => p.clone(),
            None => {
                match FileDialog::new()
                    .add_filter("Markdown", &["md"])
                    .set_filename(&self.title)
                    .show_save_single_file()
                {
                    Ok(Some(p)) => {
                        self.path = Some(p.clone());
                        self.title = p
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        p
                    }
                    _ => return,
                }
            }
        };

        if fs::write(&path, self.buffer.content()).is_ok() {
            let _ = persistence::save_note_title(&path, &self.title);
            self.modified = false;
        }
    }

    /// Auto-save to data directory (silent, no dialog)
    pub fn auto_save(&mut self) {
        // If we have a path, save there
        if let Some(ref path) = self.path {
            let _ = fs::write(path, self.buffer.content());
            let _ = persistence::save_note_title(path, &self.title);
            self.modified = false;
            return;
        }

        // Otherwise, create a new file in data directory
        let filename = persistence::generate_note_filename();
        if let Ok(path) = persistence::save_note(&filename, self.buffer.content()) {
            self.path = Some(path.clone());
            let _ = persistence::save_note_title(&path, &self.title);
            self.modified = false;
        }
    }

    #[allow(dead_code)]
    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    #[allow(dead_code)]
    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn set_title(&mut self, title: String) {
        self.title = title;
        if let Some(path) = &self.path {
            let _ = persistence::save_note_title(path, &self.title);
        }
    }

    pub fn content(&self) -> &str {
        self.buffer.content()
    }

    pub fn cursor_position(&self) -> usize {
        self.buffer.cursor()
    }

    pub fn insert_char(&mut self, ch: char) {
        self.buffer.insert(ch);
        self.modified = true;
    }

    pub fn backspace(&mut self) {
        self.buffer.backspace();
        self.modified = true;
    }

    pub fn delete_word_left(&mut self) {
        self.buffer.delete_word_left();
        self.modified = true;
    }

    pub fn delete(&mut self) {
        self.buffer.delete();
        self.modified = true;
    }

    pub fn move_left(&mut self, selecting: bool) {
        self.buffer.move_left(selecting);
    }

    pub fn move_right(&mut self, selecting: bool) {
        self.buffer.move_right(selecting);
    }

    pub fn move_word_left(&mut self, selecting: bool) {
        self.buffer.move_word_left(selecting);
    }

    pub fn move_word_right(&mut self, selecting: bool) {
        self.buffer.move_word_right(selecting);
    }

    pub fn move_up(&mut self, selecting: bool) {
        self.buffer.move_up(selecting);
    }

    pub fn move_down(&mut self, selecting: bool) {
        self.buffer.move_down(selecting);
    }

    pub fn move_to_line_start(&mut self, selecting: bool) {
        self.buffer.move_to_line_start(selecting);
    }

    pub fn move_to_line_end(&mut self, selecting: bool) {
        self.buffer.move_to_line_end(selecting);
    }

    pub fn move_to_start(&mut self, selecting: bool) {
        self.buffer.move_to_start(selecting);
    }

    pub fn move_to_end(&mut self, selecting: bool) {
        self.buffer.move_to_end(selecting);
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn scroll_offset_x(&self) -> f32 {
        self.scroll_offset_x
    }

    pub fn word_wrap(&self) -> bool {
        self.word_wrap
    }

    #[allow(dead_code)]
    pub fn set_word_wrap(&mut self, wrap: bool) {
        self.word_wrap = wrap;
    }

    pub fn toggle_word_wrap(&mut self) {
        self.word_wrap = !self.word_wrap;
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    pub fn scroll_down(&mut self, lines: usize, visible_lines: usize) {
        let total_lines = self.buffer.len_lines();
        let max_scroll = total_lines.saturating_sub(visible_lines);
        self.scroll_offset = (self.scroll_offset + lines).min(max_scroll);
    }

    /// Get the current cursor line number
    pub fn cursor_line(&self) -> usize {
        let text = self.buffer.content();
        let cursor = self.buffer.cursor();
        text.chars().take(cursor).filter(|&c| c == '\n').count()
    }

    pub fn cursor_col(&self) -> usize {
        let (_, col) = self.buffer.char_to_line_col(self.buffer.cursor());
        col
    }

    /// Ensure cursor is visible by auto-scrolling
    pub fn ensure_cursor_visible(
        &mut self,
        visible_lines: usize,
        visible_width: f32,
        char_width: f32,
    ) {
        let cursor_line = self.cursor_line();

        // Scroll up if cursor is above visible area
        if cursor_line < self.scroll_offset {
            self.scroll_offset = cursor_line;
        }

        // Scroll down if cursor is below visible area
        if cursor_line >= self.scroll_offset + visible_lines {
            self.scroll_offset = cursor_line.saturating_sub(visible_lines - 1);
        }

        // Horizontal scrolling (only if wrap is off)
        if !self.word_wrap {
            let cursor_col = self.cursor_col();
            let cursor_x = cursor_col as f32 * char_width;

            // Scroll left
            if cursor_x < self.scroll_offset_x {
                self.scroll_offset_x = cursor_x;
            }

            // Scroll right
            if cursor_x + char_width > self.scroll_offset_x + visible_width {
                self.scroll_offset_x = cursor_x - visible_width + char_width * 2.0; // Add some padding
            }
        } else {
            self.scroll_offset_x = 0.0;
        }
    }

    /// Set cursor position by line and column
    pub fn set_cursor_position(&mut self, line: usize, col: usize, selecting: bool) {
        self.buffer.set_cursor_by_line_col(line, col, selecting);
    }

    #[allow(dead_code)]
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.buffer.selection_range()
    }

    pub fn selection_range_line_col(&self) -> Option<((usize, usize), (usize, usize))> {
        if let Some((start, end)) = self.buffer.selection_range() {
            Some((
                self.buffer.char_to_line_col(start),
                self.buffer.char_to_line_col(end),
            ))
        } else {
            None
        }
    }

    pub fn copy_selection(&self) -> Option<String> {
        let text = self.buffer.selected_text();
        if text.is_empty() { None } else { Some(text) }
    }

    pub fn cut_selection(&mut self) -> Option<String> {
        let text = self.copy_selection();
        if text.is_some() {
            self.buffer.delete_selection();
            self.modified = true;
        }
        text
    }

    pub fn paste_text(&mut self, text: &str) -> bool {
        if !text.is_empty() {
            self.buffer.insert_str(text);
            self.modified = true;
            return true;
        }
        false
    }

    pub fn select_all(&mut self) {
        self.buffer.select_all();
    }

    pub fn select_word_at_cursor(&mut self) {
        self.buffer.select_word_at_cursor();
    }

    pub fn move_lines_up(&mut self) -> bool {
        self.buffer.move_lines_up();
        // Assume buffering actions modify state for now, returns void in TextBuffer usually
        // But for AppResult::Redraw optimization, better to assume true or check hash?
        // Let's assume true for actions.
        true
    }

    pub fn move_lines_down(&mut self) -> bool {
        self.buffer.move_lines_down();
        true
    }

    pub fn undo(&mut self) -> bool {
        self.buffer.undo();
        true
    }

    pub fn redo(&mut self) -> bool {
        self.buffer.redo();
        true
    }

    pub fn total_lines(&self) -> usize {
        self.buffer.len_lines()
    }

    pub fn set_scroll_offset(&mut self, offset: usize) -> bool {
        if self.scroll_offset != offset {
            self.scroll_offset = offset;
            return true;
        }
        false
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_untitled() {
        let tab = Tab::new_untitled();
        assert!(tab.title().starts_with("Untitled-"));
        assert!(tab.content().is_empty());
    }

    #[test]
    fn test_insert_and_content() {
        let mut tab = Tab::new_untitled();
        tab.insert_char('H');
        tab.insert_char('i');
        assert_eq!(tab.content(), "Hi");
    }

    #[test]
    fn test_backspace() {
        let mut tab = Tab::new_untitled();
        tab.insert_char('A');
        tab.insert_char('B');
        tab.backspace();
        assert_eq!(tab.content(), "A");
    }
}
