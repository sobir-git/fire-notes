#![allow(dead_code)]
//! `TextInput` primitive — single-line text field.
//!
//! # Design
//! Follows the same encapsulation contract as React controlled inputs:
//! - All state mutations go through named methods (`insert`, `backspace`, …).
//! - Rendering is a pure function: `input.view(rect, scale, props)` → `Node`.
//! - No internal geometry fields are exposed; callers never touch raw state.
//!
//! # Typical usage
//! ```ignore
//! // Construction
//! let mut input = TextInput::new("hello");
//! input.set_char_width(measured_cw);
//!
//! // Mutations (in event handler)
//! input.insert('!');
//! input.move_left(false);
//! input.select_all();
//! if let Some(text) = input.cut() { clipboard.set(text); }
//!
//! // Rendering (in build_node / render)
//! let node = input.view(rect, scale, ViewProps { placeholder: "Type here…", cursor_visible: true, focused: true });
//! ```

use crate::layout::node::{BoxStyle, Color, CursorNode, Node, SelectionNode, TextStyle};
use crate::ui::{CursorShape, Rect, TextInput as RawState};
use crate::view_ctx::ViewCtx;

// ── ViewProps ─────────────────────────────────────────────────────────────────

/// Display options passed to [`TextInput::view`].
/// Geometry (rect, scale) is provided separately so the same props struct can
/// be reused across reflows without touching it.
#[derive(Debug, Clone, Copy)]
pub struct ViewProps<'a> {
    /// Shown when the field is empty.
    pub placeholder: &'a str,
    /// Whether the cursor blink is currently on.
    pub cursor_visible: bool,
    /// Draws a focused border when `true`.
    pub focused: bool,
}

impl<'a> ViewProps<'a> {
    pub fn focused(placeholder: &'a str) -> Self {
        Self { placeholder, cursor_visible: true, focused: true }
    }
    pub fn unfocused(placeholder: &'a str) -> Self {
        Self { placeholder, cursor_visible: false, focused: false }
    }
}

// ── TextInput ─────────────────────────────────────────────────────────────────

/// Single-line text input — editing state + font metrics.
/// Geometry is not stored; pass a `Rect` at render time via [`view`].
#[derive(Debug, Clone)]
pub struct TextInput {
    state:      RawState,
    char_width: f32,
}


// ── Construction ─────────────────────────────────────────────────────────────

impl TextInput {
    /// Create a new field with the given initial text (cursor at end).
    pub fn new(text: impl Into<String>) -> Self {
        Self { state: RawState::new(text.into()), char_width: 8.0 }
    }

    /// Create an empty field.
    pub fn empty() -> Self { Self::new(String::new()) }

    /// Set the measured character width from the renderer.
    /// Must be called once after font loading before any pointer events or
    /// `view()` calls, and again on scale/font changes.
    pub fn set_char_width(&mut self, w: f32) { self.char_width = w; }
}

// ── State mutations ───────────────────────────────────────────────────────────

impl TextInput {
    pub fn insert(&mut self, ch: char) {
        self.state.insert_char(ch);
    }

    pub fn backspace(&mut self) {
        self.state.backspace();
    }

    pub fn delete(&mut self) { self.state.delete(); }

    pub fn delete_word_left(&mut self)  { self.state.delete_word_left(); }
    pub fn delete_word_right(&mut self) { self.state.delete_word_right(); }

    pub fn move_left(&mut self, selecting: bool)       { self.state.move_left(selecting); }
    pub fn move_right(&mut self, selecting: bool)      { self.state.move_right(selecting); }
    pub fn move_word_left(&mut self, selecting: bool)  { self.state.move_word_left(selecting); }
    pub fn move_word_right(&mut self, selecting: bool) { self.state.move_word_right(selecting); }
    pub fn move_to_start(&mut self, selecting: bool)   { self.state.move_to_start(selecting); }
    pub fn move_to_end(&mut self, selecting: bool)     { self.state.move_to_end(selecting); }

    pub fn select_all(&mut self)  { self.state.select_all(); }
    pub fn clear_selection(&mut self) { self.state.selection_anchor = None; }

    /// Copy the current selection. Returns `None` when nothing is selected.
    pub fn copy(&self)       -> Option<String> { self.state.copy() }
    /// Cut the current selection. Returns `None` when nothing is selected.
    pub fn cut(&mut self)    -> Option<String> { self.state.cut() }
    /// Paste `text`, replacing any existing selection. Strips newlines.
    pub fn paste(&mut self, text: &str)        { self.state.paste(text); }
}

// ── Read accessors ────────────────────────────────────────────────────────────

impl TextInput {
    pub fn text(&self)         -> &str  { self.state.text() }
    pub fn is_empty(&self)     -> bool  { self.state.text().is_empty() }
    pub fn cursor(&self)       -> usize { self.state.cursor() }
    pub fn scroll_offset(&self)-> f32   { self.state.scroll_offset }

    /// `true` when text is selected.
    pub fn has_selection(&self) -> bool { self.state.selection_anchor.is_some() }
}

// ── Pointer events ────────────────────────────────────────────────────────────

impl TextInput {
    /// Handle a pointer-down event. `rect` and `scale` must match what was
    /// last passed to [`view`]. `selecting` = true for shift+click.
    /// Returns `true` when the click landed inside.
    pub fn on_pointer_down(&mut self, x: f32, y: f32, rect: Rect, scale: f32, selecting: bool) -> bool {
        if !rect.contains(x, y) { return false; }
        let text_x     = Self::text_x(rect, scale);
        let relative_x = (x - text_x).max(0.0);
        self.state.set_cursor_from_x(relative_x, self.char_width, selecting);
        self.ensure_visible(rect, scale);
        true
    }

    /// Handle a double-click — selects the word under the pointer.
    /// Returns `true` when the click landed inside.
    pub fn on_double_click(&mut self, x: f32, y: f32, rect: Rect, scale: f32) -> bool {
        if !rect.contains(x, y) { return false; }
        let text_x     = Self::text_x(rect, scale);
        let relative_x = (x - text_x).max(0.0);
        self.state.select_word_at_x(relative_x, self.char_width);
        self.ensure_visible(rect, scale);
        true
    }

    /// Handle a triple-click — selects all text.
    /// Returns `true` when the click landed inside.
    pub fn on_triple_click(&mut self, x: f32, y: f32, rect: Rect, scale: f32) -> bool {
        if !rect.contains(x, y) { return false; }
        let _ = (x, y, scale);
        self.select_all();
        true
    }

    /// Handle a pointer-drag event (extend selection).
    pub fn on_drag(&mut self, x: f32, rect: Rect, scale: f32) {
        let text_x     = Self::text_x(rect, scale);
        let relative_x = (x - text_x).max(0.0);
        self.state.set_cursor_from_x(relative_x, self.char_width, true);
        self.ensure_visible(rect, scale);
    }

    /// Scroll so the cursor is visible inside `rect`.
    pub fn ensure_visible(&mut self, rect: Rect, scale: f32) {
        let padding = Self::padding(scale);
        let visible_width = (rect.width - padding * 2.0).max(0.0);
        self.state.ensure_cursor_visible(visible_width, self.char_width);
    }

    /// Cursor shape when the pointer is at (x, y) relative to the last `rect`.
    pub fn cursor_shape_at(&self, x: f32, y: f32, rect: Rect) -> CursorShape {
        if rect.contains(x, y) { CursorShape::Text } else { CursorShape::Default }
    }
}

// ── Rendering ─────────────────────────────────────────────────────────────────

impl TextInput {
    /// Produce the `Node` tree for this field.
    ///
    /// Geometry is computed from `rect` and `scale` on every call — no
    /// separate `relayout()` step is needed. This is the **only** render path.
    pub fn view(&self, rect: Rect, scale: f32, props: ViewProps<'_>) -> Node {
        let padding         = Self::padding(scale);
        let font_size       = Self::font_size(scale);
        let text_x          = Self::text_x(rect, scale);
        let baseline_y      = rect.y + rect.height / 2.0 + font_size * 0.35;
        let scroll          = self.state.scroll_offset;
        let cw              = self.char_width;

        let border_color = if props.focused {
            Color::rgba(0.4, 0.6, 0.9, 0.9)
        } else {
            Color::rgba(0.3, 0.3, 0.3, 0.8)
        };
        let border_width = if props.focused { 1.5 } else { 1.0 };

        let mut children = vec![
            Node::Box {
                rect,
                style: BoxStyle::filled(Color::rgba(0.08, 0.08, 0.10, 1.0))
                    .with_border(border_color, border_width)
                    .with_radius(5.0),
            },
        ];

        if let Some((start, end)) = self.state.selection_range() {
            let sc = self.state.text()[..start].chars().count();
            let ec = self.state.text()[..end].chars().count();
            let sel_x = text_x + sc as f32 * cw - scroll;
            let sel_w = (ec - sc) as f32 * cw;
            let vp = 3.0 * scale;
            children.push(Node::Selection(SelectionNode {
                rect:  Rect { x: sel_x, y: rect.y + vp, width: sel_w, height: rect.height - vp * 2.0 },
                color: Color::rgba(0.39, 0.55, 0.82, 0.45),
            }));
        }

        let display = if self.state.text().is_empty() {
            props.placeholder.to_string()
        } else {
            self.state.text().to_string()
        };
        let text_color = if self.state.text().is_empty() {
            Color::rgba(0.4, 0.4, 0.4, 1.0)
        } else {
            Color::rgb(1.0, 1.0, 1.0)
        };
        children.push(Node::Text {
            text: display,
            style: TextStyle { font_size, color: text_color, baseline_y, text_x, scroll_x: scroll },
            clip: Some(Rect { x: rect.x + padding, y: rect.y, width: rect.width - padding * 2.0, height: rect.height }),
        });

        let cursor_x = {
            let chars = self.state.text()[..self.state.cursor()].chars().count();
            text_x + chars as f32 * cw - scroll
        };
        let vp = 4.0 * scale;
        children.push(Node::Cursor(CursorNode {
            rect:    Rect { x: cursor_x, y: rect.y + vp, width: 2.0, height: rect.height - vp * 2.0 },
            color:   Color::rgba(0.4, 0.7, 1.0, 1.0),
            visible: props.cursor_visible,
        }));

        Node::layer(children)
    }

    /// Render using a [`ViewCtx`] from the runtime.
    ///
    /// `id` is the stable widget ID used to look up focus state in `ctx`.
    /// `placeholder` is shown when the field is empty.
    ///
    /// This is the preferred render method when using [`crate::runtime::Runtime`].
    /// Render using a [`ViewCtx`] from the runtime.
    ///
    /// `rect` is in **logical pixels**. Scale is applied internally.
    /// Registers the logical rect in `ctx` so event handlers can look it up.
    pub fn view_with_ctx(&mut self, rect: Rect, id: u64, ctx: &mut ViewCtx, placeholder: &str) -> Node {
        ctx.register_rect(id, rect);          // store logical
        self.char_width = ctx.char_width * ctx.scale; // physical char_width for internal draw
        let focused = ctx.is_focused(id);
        let props = ViewProps {
            placeholder,
            cursor_visible: focused && ctx.cursor_on,
            focused,
        };
        self.view(ctx.to_physical(rect), ctx.scale, props) // draw with physical rect
    }

    /// Pointer-down using a [`ViewCtx`] — for use in `App::handle_pointer_down`.
    /// `x`, `y`, and `logical_rect` are all in logical pixels; scale is read from `ctx`.
    pub fn on_pointer_down_ctx(&mut self, x: f32, y: f32, logical_rect: Rect, ctx: &crate::view_ctx::ViewCtx) -> bool {
        let s = ctx.scale;
        self.char_width = ctx.char_width * s;
        let phys = ctx.to_physical(logical_rect);
        self.on_pointer_down(x * s, y * s, phys, s, false)
    }

    /// Keyboard event using a [`ViewCtx`] — for use in `App::handle_key`.
    /// `logical_rect` is the cached logical rect from `ctx.rect_for(id)`.
    pub fn handle_key_ctx(&mut self, event: TextInputEvent<'_>, logical_rect: Rect, ctx: &crate::view_ctx::ViewCtx) -> EventOutcome {
        let s = ctx.scale;
        self.char_width = ctx.char_width * s;
        let phys = ctx.to_physical(logical_rect);
        self.handle_key(event, phys, s)
    }

    /// Ensure cursor visible using a [`ViewCtx`].
    pub fn ensure_visible_ctx(&mut self, logical_rect: Rect, ctx: &crate::view_ctx::ViewCtx) {
        let s = ctx.scale;
        self.char_width = ctx.char_width * s;
        self.ensure_visible(ctx.to_physical(logical_rect), s);
    }

    // ── Private geometry helpers ──────────────────────────────────────────────

    fn padding(scale: f32) -> f32 { 8.0 * scale }
    fn font_size(scale: f32) -> f32 { 14.0 * scale }
    fn text_x(rect: Rect, scale: f32) -> f32 { rect.x + Self::padding(scale) }
}

// ── Keyboard event handling ───────────────────────────────────────────────────

/// Direction for cursor / selection movement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveDir { Left, Right, Start, End }

/// How far to move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveBy { Char, Word }

/// Platform-agnostic keyboard event for a text input field.
/// The platform layer (winit, web, …) translates its native key type to this
/// before handing it to the widget — the widget never imports winit.
#[derive(Debug, Clone)]
pub enum TextInputEvent<'a> {
    /// A printable character was typed.
    Char(char),
    /// Backspace. `word = true` deletes to word boundary (Ctrl+Backspace).
    Backspace { word: bool },
    /// Delete forward. `word = true` deletes to word boundary (Ctrl+Delete).
    Delete { word: bool },
    /// Move cursor / extend selection.
    Move { dir: MoveDir, by: MoveBy, selecting: bool },
    /// Ctrl+A — select all text.
    SelectAll,
    /// Ctrl+C — copy selection. Primitive signals back via `EventOutcome::Copy`.
    Copy,
    /// Ctrl+X — cut selection. Primitive signals back via `EventOutcome::Cut`.
    Cut,
    /// Ctrl+V — paste the given text (caller already read clipboard).
    Paste(&'a str),
    /// Escape — clear selection.
    Escape,
}

/// Result of [`TextInput::handle_key`].
#[derive(Debug, Clone, PartialEq)]
pub enum EventOutcome {
    /// Nothing changed (e.g., unrecognised key).
    Nothing,
    /// State changed — caller should redraw.
    Handled,
    /// The caller should read the clipboard and pass it back as `Paste`.
    /// State has not changed yet.
    NeedsPaste,
    /// Selection was copied — value is the copied text.
    Copied(String),
    /// Selection was cut — value is the cut text.
    Cut(String),
}

impl TextInput {
    /// Handle a platform-agnostic keyboard event.
    ///
    /// `rect` and `scale` are needed to keep the cursor visible after edits.
    /// They must match the values last passed to [`view`].
    ///
    /// Returns an [`EventOutcome`] so the caller can handle clipboard I/O
    /// without the primitive importing any clipboard crate.
    pub fn handle_key(&mut self, event: TextInputEvent<'_>, rect: Rect, scale: f32) -> EventOutcome {
        let outcome = match event {
            TextInputEvent::Char(ch) => {
                self.insert(ch);
                EventOutcome::Handled
            }
            TextInputEvent::Backspace { word: true }  => { self.delete_word_left();  EventOutcome::Handled }
            TextInputEvent::Backspace { word: false } => { self.backspace();         EventOutcome::Handled }
            TextInputEvent::Delete   { word: true }   => { self.delete_word_right(); EventOutcome::Handled }
            TextInputEvent::Delete   { word: false }  => { self.delete();            EventOutcome::Handled }
            TextInputEvent::Move { dir, by, selecting } => {
                match (dir, by) {
                    (MoveDir::Left,  MoveBy::Char) => self.move_left(selecting),
                    (MoveDir::Right, MoveBy::Char) => self.move_right(selecting),
                    (MoveDir::Left,  MoveBy::Word) => self.move_word_left(selecting),
                    (MoveDir::Right, MoveBy::Word) => self.move_word_right(selecting),
                    (MoveDir::Start, _)            => self.move_to_start(selecting),
                    (MoveDir::End,   _)            => self.move_to_end(selecting),
                }
                EventOutcome::Handled
            }
            TextInputEvent::SelectAll => { self.select_all(); EventOutcome::Handled }
            TextInputEvent::Copy  => {
                match self.copy() {
                    Some(text) => EventOutcome::Copied(text),
                    None       => EventOutcome::Nothing,
                }
            }
            TextInputEvent::Cut => {
                match self.cut() {
                    Some(text) => EventOutcome::Cut(text),
                    None       => EventOutcome::Nothing,
                }
            }
            TextInputEvent::Paste(text) => { self.paste(text); EventOutcome::Handled }
            TextInputEvent::Escape => {
                if self.has_selection() { self.clear_selection(); EventOutcome::Handled }
                else                    { EventOutcome::Nothing }
            }
        };
        if outcome == EventOutcome::Handled {
            self.ensure_visible(rect, scale);
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(text: &str, cw: f32) -> TextInput {
        let mut f = TextInput::new(text);
        f.set_char_width(cw);
        f
    }

    fn rect(w: f32) -> Rect {
        Rect { x: 0.0, y: 0.0, width: w, height: 24.0 }
    }

    const SCALE: f32 = 1.0;
    const CW: f32 = 8.0;
    const PAD: f32 = 8.0; // padding(scale=1.0)

    fn x_of(char_idx: usize) -> f32 {
        PAD + char_idx as f32 * CW
    }

    #[test]
    fn single_click_positions_cursor() {
        let mut f = field("hello", CW);
        let r = rect(200.0);
        f.on_pointer_down(x_of(2), 12.0, r, SCALE, false);
        assert_eq!(f.cursor(), 2);
        assert!(!f.has_selection());
    }

    #[test]
    fn shift_click_extends_selection() {
        let mut f = field("hello world", CW);
        let r = rect(300.0);
        f.on_pointer_down(x_of(0), 12.0, r, SCALE, false);
        f.on_pointer_down(x_of(5), 12.0, r, SCALE, true);
        assert!(f.has_selection());
        assert_eq!(f.state.selected_text(), "hello");
    }

    #[test]
    fn drag_extends_selection_from_anchor() {
        let mut f = field("hello world", CW);
        let r = rect(300.0);
        f.on_pointer_down(x_of(0), 12.0, r, SCALE, false);
        f.on_drag(x_of(5), r, SCALE);
        assert!(f.has_selection());
        assert_eq!(f.state.selected_text(), "hello");
    }

    #[test]
    fn double_click_selects_word() {
        let mut f = field("hello world", CW);
        let r = rect(300.0);
        f.on_double_click(x_of(2), 12.0, r, SCALE);
        assert!(f.has_selection());
        assert_eq!(f.state.selected_text(), "hello");
    }

    #[test]
    fn double_click_selects_second_word() {
        let mut f = field("hello world", CW);
        let r = rect(300.0);
        f.on_double_click(x_of(7), 12.0, r, SCALE);
        assert!(f.has_selection());
        assert_eq!(f.state.selected_text(), "world");
    }

    #[test]
    fn triple_click_selects_all() {
        let mut f = field("hello world", CW);
        let r = rect(300.0);
        f.on_triple_click(x_of(3), 12.0, r, SCALE);
        assert!(f.has_selection());
        assert_eq!(f.state.selected_text(), "hello world");
    }

    #[test]
    fn click_outside_returns_false() {
        let mut f = field("hello", CW);
        let r = rect(100.0);
        let hit = f.on_pointer_down(200.0, 12.0, r, SCALE, false);
        assert!(!hit);
    }
}

// ── Backward-compatibility shims (components + tests still use these) ─────────

impl TextInput {
    /// Deprecated: use `TextInput::empty()` instead.
    #[inline] pub fn new_empty() -> Self { Self::empty() }

    /// Deprecated: use `view(rect, scale, props)` instead.
    pub fn render_at(&self, rect: Rect, scale: f32, placeholder: &str, cursor_visible: bool) -> Node {
        self.view(rect, scale, ViewProps { placeholder, cursor_visible, focused: cursor_visible })
    }

    /// Deprecated: `relayout` is no longer needed — geometry is computed in `view`.
    pub fn relayout(&mut self, rect: Rect, scale: f32) {
        self.ensure_visible(rect, scale);
    }

}
