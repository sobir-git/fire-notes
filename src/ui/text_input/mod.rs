//! Single-line text input widget.

mod cursor;
mod edit;

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
        Self { text, cursor, selection_anchor: None, scroll_offset: 0.0 }
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
}
