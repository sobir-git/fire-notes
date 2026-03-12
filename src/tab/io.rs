//! File I/O for Tab — open, save, auto-save.

use std::fs;
use std::path::PathBuf;
use native_dialog::FileDialog;
use super::Tab;

impl Tab {
    pub fn from_file(path: PathBuf) -> Option<Self> {
        let content = fs::read_to_string(&path).ok()?;
        let title = crate::persistence::load_note_title(&path).unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string()
        });
        Some(Self {
            buffer: crate::text_buffer::TextBuffer::from_str(&content),
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
            None => match FileDialog::new()
                .add_filter("Markdown", &["md"])
                .set_filename(&self.title)
                .show_save_single_file()
            {
                Ok(Some(p)) => {
                    self.title = p.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string();
                    self.path = Some(p.clone());
                    p
                }
                _ => return,
            },
        };
        if fs::write(&path, self.buffer.content()).is_ok() {
            let _ = crate::persistence::save_note_title(&path, &self.title);
            self.modified = false;
        }
    }

    pub fn auto_save(&mut self) {
        if let Some(ref path) = self.path {
            let _ = fs::write(path, self.buffer.content());
            let _ = crate::persistence::save_note_title(path, &self.title);
            self.modified = false;
            return;
        }
        let filename = crate::persistence::generate_note_filename();
        if let Ok(path) = crate::persistence::save_note(&filename, self.buffer.content()) {
            self.path = Some(path.clone());
            let _ = crate::persistence::save_note_title(&path, &self.title);
            self.modified = false;
        }
    }
}
