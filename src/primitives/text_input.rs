#![allow(dead_code)]
//! TextInput primitive — editing state + geometry, one file.
//!
//! Replaces `ui::TextInput` (state) + `ui::TextInputWidget` (geometry) + `fw::TextInput` (merged).

use crate::layout::Widget;
use crate::layout::node::{BoxStyle, Color, CursorNode, Node, SelectionNode, TextStyle};
use crate::ui::{CursorShape, Rect, TextInput as RawState};

// ── Primitive ─────────────────────────────────────────────────────────────────

/// Single-line text input — owns editing state and screen geometry.
#[derive(Debug, Clone)]
pub struct TextInput {
    /// Editing state: text, cursor, selection_anchor, scroll_offset.
    pub state: RawState,
    /// Bounding rect of the input box.
    pub rect: Rect,
    /// Font size in physical pixels.
    pub font_size: f32,
    /// X where text rendering starts (rect.x + padding).
    pub text_x: f32,
    /// Text baseline Y (vertically centred in rect).
    pub text_baseline_y: f32,
    pub scale: f32,
    /// Cached char width — set by caller after font measurement.
    pub char_width: f32,
}

// ── Widget impl ───────────────────────────────────────────────────────────────

/// `Widget` impl uses the stored `char_width`. Call `set_char_width` after
/// font measurement so on_pointer_down has correct hit-testing.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    /// Click was inside the input — cursor repositioned.
    Focused,
    /// Click was outside the input.
    Miss,
}

impl Widget for TextInput {
    type Event = InputEvent;
    fn on_pointer_down(&mut self, x: f32, y: f32) -> InputEvent {
        if self.on_pointer_down_with(x, y, self.char_width) {
            InputEvent::Focused
        } else {
            InputEvent::Miss
        }
    }
    fn on_hover(&mut self, _x: f32, _y: f32) -> bool { false }
    fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape {
        if self.rect.contains(x, y) { CursorShape::Text } else { CursorShape::Default }
    }
}

impl TextInput {
    pub fn new_empty() -> Self {
        Self {
            state:            RawState::new(String::new()),
            rect:             Rect::ZERO,
            font_size:        0.0,
            text_x:           0.0,
            text_baseline_y:  0.0,
            scale:            1.0,
            char_width:       8.0,
        }
    }

    pub fn set_char_width(&mut self, w: f32) { self.char_width = w; }

    pub fn new(text: String) -> Self {
        let mut s = Self::new_empty();
        s.state = RawState::new(text);
        s
    }

    /// Update geometry, preserving all editing state.
    pub fn relayout(&mut self, rect: Rect, scale: f32) {
        let padding   = 8.0 * scale;
        let font_size = 14.0 * scale;
        self.rect             = rect;
        self.scale            = scale;
        self.font_size        = font_size;
        self.text_x           = rect.x + padding;
        self.text_baseline_y  = rect.y + rect.height / 2.0 + font_size * 0.35;
    }

    // ── State accessors ───────────────────────────────────────────────────────

    pub fn text(&self)         -> &str { self.state.text() }
    pub fn insert(&mut self, c: char)  { self.state.insert_char(c); self.ensure_cursor_visible(self.char_width); }
    pub fn backspace(&mut self)        { use crate::app::input_handler::InputHandler; self.state.handle_backspace(); self.ensure_cursor_visible(self.char_width); }
    pub fn is_empty(&self)     -> bool { self.state.text().is_empty() }
    pub fn scroll_offset(&self)-> f32  { self.state.scroll_offset }
    pub fn cursor(&self)       -> usize { self.state.cursor() }

    pub fn move_left(&mut self, selecting: bool)       { self.state.move_left(selecting);       self.ensure_cursor_visible(self.char_width); }
    pub fn move_right(&mut self, selecting: bool)      { self.state.move_right(selecting);      self.ensure_cursor_visible(self.char_width); }
    pub fn move_word_left(&mut self, selecting: bool)  { self.state.move_word_left(selecting);  self.ensure_cursor_visible(self.char_width); }
    pub fn move_word_right(&mut self, selecting: bool) { self.state.move_word_right(selecting); self.ensure_cursor_visible(self.char_width); }
    pub fn move_to_start(&mut self, selecting: bool)   { self.state.move_to_start(selecting);   self.ensure_cursor_visible(self.char_width); }
    pub fn move_to_end(&mut self, selecting: bool)     { self.state.move_to_end(selecting);     self.ensure_cursor_visible(self.char_width); }

    // ── Events ────────────────────────────────────────────────────────────────

    /// Returns `true` if this widget claimed the event.
    pub fn on_pointer_down_with(&mut self, x: f32, y: f32, char_width: f32) -> bool {
        if !self.rect.contains(x, y) { return false; }
        let relative_x = (x - self.text_x + self.state.scroll_offset).max(0.0);
        self.state.set_cursor_from_x(relative_x, char_width, false);
        true
    }

    pub fn on_drag(&mut self, x: f32, char_width: f32) {
        let relative_x = (x - self.text_x + self.state.scroll_offset).max(0.0);
        self.state.set_cursor_from_x(relative_x, char_width, true);
    }

    pub fn ensure_cursor_visible(&mut self, char_width: f32) {
        let padding       = self.text_x - self.rect.x;
        let visible_width = (self.rect.width - padding * 2.0).max(0.0);
        self.state.ensure_cursor_visible(visible_width, char_width);
    }

    pub fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape {
        if self.rect.contains(x, y) { CursorShape::Text } else { CursorShape::Default }
    }

    // ── Node / Component ──────────────────────────────────────────────────────

    /// Declare this input as a `Node` subtree.
    /// `cursor_visible` — current blink state from the app clock.
    pub fn render(&self, cursor_visible: bool) -> Node {
        let text_style = TextStyle {
            font_size:  self.font_size,
            color:      Color::rgb(1.0, 1.0, 1.0),
            baseline_y: self.text_baseline_y,
            clip_x:     self.rect.x,
            scroll_x:   self.state.scroll_offset,
        };

        let cursor_rect = {
            let byte_pos = self.state.cursor();
            let chars    = self.state.text()[..byte_pos].chars().count();
            let x        = self.text_x + chars as f32 * self.char_width - self.state.scroll_offset;
            let v_pad    = 4.0;
            Rect { x, y: self.rect.y + v_pad, width: 2.0, height: self.rect.height - v_pad * 2.0 }
        };

        let mut children = vec![
            Node::Box {
                rect:  self.rect,
                style: BoxStyle::filled(Color::rgba(0.0, 0.0, 0.0, 0.85))
                    .with_border(Color::rgba(0.4, 0.6, 0.9, 0.7), 1.0)
                    .with_radius(4.0),
            },
        ];

        if let Some(anchor) = self.state.selection_anchor {
            let cursor = self.state.cursor;
            let (start, end) = (anchor.min(cursor), anchor.max(cursor));
            let start_chars = self.state.text()[..start].chars().count();
            let end_chars   = self.state.text()[..end].chars().count();
            let sel_x = self.text_x + start_chars as f32 * self.char_width - self.state.scroll_offset;
            let sel_w = (end_chars - start_chars) as f32 * self.char_width;
            let v_pad = 3.0;
            children.push(Node::Selection(SelectionNode {
                rect:  Rect { x: sel_x, y: self.rect.y + v_pad, width: sel_w, height: self.rect.height - v_pad * 2.0 },
                color: Color::rgba(0.39, 0.55, 0.82, 0.47),
            }));
        }

        let display_text = if self.state.text().is_empty() {
            "Search notes...".to_string()
        } else {
            self.state.text().to_string()
        };
        children.push(Node::Text { text: display_text, style: text_style, clip: Some(self.rect) });

        children.push(Node::Cursor(CursorNode {
            rect:    cursor_rect,
            color:   Color::rgba(0.4, 0.7, 1.0, 1.0),
            visible: cursor_visible,
        }));

        Node::layer(children)
    }

    /// Theme-aware render — derives colors from theme tokens.
    pub fn render_themed(&self, theme: &crate::theme::Theme, cursor_visible: bool) -> Node {
        let _ = theme;
        self.render(cursor_visible)
    }

    /// Render at `rect` with geometry computed on-the-fly — no prior `relayout` needed.
    pub fn render_at(&self, rect: Rect, scale: f32, placeholder: &str, cursor_visible: bool) -> Node {
        let padding          = 8.0 * scale;
        let font_size        = 14.0 * scale;
        let text_x           = rect.x + padding;
        let text_baseline_y  = rect.y + rect.height / 2.0 + font_size * 0.35;
        let char_width       = self.char_width;

        let text_style = TextStyle {
            font_size,
            color:      Color::rgb(1.0, 1.0, 1.0),
            baseline_y: text_baseline_y,
            clip_x:     rect.x,
            scroll_x:   self.state.scroll_offset,
        };

        let cursor_rect = {
            let byte_pos = self.state.cursor();
            let chars    = self.state.text()[..byte_pos].chars().count();
            let x        = text_x + chars as f32 * char_width - self.state.scroll_offset;
            let v_pad    = 4.0;
            Rect { x, y: rect.y + v_pad, width: 2.0, height: rect.height - v_pad * 2.0 }
        };

        let mut children = vec![
            Node::Box {
                rect,
                style: BoxStyle::filled(Color::rgba(0.0, 0.0, 0.0, 0.85))
                    .with_border(Color::rgba(0.4, 0.6, 0.9, 0.7), 1.0)
                    .with_radius(4.0),
            },
        ];

        if let Some(anchor) = self.state.selection_anchor {
            let cursor = self.state.cursor;
            let (start, end) = (anchor.min(cursor), anchor.max(cursor));
            let start_chars = self.state.text()[..start].chars().count();
            let end_chars   = self.state.text()[..end].chars().count();
            let sel_x = text_x + start_chars as f32 * char_width - self.state.scroll_offset;
            let sel_w = (end_chars - start_chars) as f32 * char_width;
            let v_pad = 3.0;
            children.push(Node::Selection(SelectionNode {
                rect:  Rect { x: sel_x, y: rect.y + v_pad, width: sel_w, height: rect.height - v_pad * 2.0 },
                color: Color::rgba(0.39, 0.55, 0.82, 0.47),
            }));
        }

        let display_text = if self.state.text().is_empty() {
            placeholder.to_string()
        } else {
            self.state.text().to_string()
        };
        children.push(Node::Text { text: display_text, style: text_style, clip: Some(rect) });
        children.push(Node::Cursor(CursorNode {
            rect:    cursor_rect,
            color:   Color::rgba(0.4, 0.7, 1.0, 1.0),
            visible: cursor_visible,
        }));

        Node::layer(children)
    }

    // ── Geometry helpers ──────────────────────────────────────────────────────

    pub fn cursor_rect(&self, char_width: f32) -> Rect {
        let byte_pos = self.state.cursor();
        let chars    = self.state.text()[..byte_pos].chars().count();
        let x        = self.text_x + chars as f32 * char_width - self.state.scroll_offset;
        let v_pad    = 4.0;
        Rect { x, y: self.rect.y + v_pad, width: 2.0, height: self.rect.height - v_pad * 2.0 }
    }

    pub fn selection_rect(&self, char_width: f32) -> Option<Rect> {
        let (start, end)  = self.state.selection_range()?;
        let start_chars   = self.state.text()[..start].chars().count();
        let end_chars     = self.state.text()[..end].chars().count();
        let x = self.text_x + start_chars as f32 * char_width - self.state.scroll_offset;
        let w = (end_chars - start_chars) as f32 * char_width;
        let v_pad = 3.0;
        Some(Rect { x, y: self.rect.y + v_pad, width: w, height: self.rect.height - v_pad * 2.0 })
    }
}
