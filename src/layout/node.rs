//! Node — the declarative UI tree.
//!
//! A `Node` is pure data produced by `Component::render()`.
//! The renderer walks it to emit draw calls; tests assert on it without GPU.
//! No widget types, no GPU handles, no mutable state.
//!
//! Analogous to React's virtual DOM element / JSX output.

use crate::ui::Rect;

// ── Style types ───────────────────────────────────────────────────────────────

/// RGBA colour (0.0–1.0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32, pub g: f32, pub b: f32, pub a: f32,
}

impl Color {
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self { Self { r, g, b, a } }
    pub const fn rgb(r: f32, g: f32, b: f32)            -> Self { Self { r, g, b, a: 1.0 } }
    pub const fn transparent() -> Self { Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 } }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    pub font_size:   f32,
    pub color:       Color,
    pub baseline_y:  f32,
    /// X coordinate where text rendering starts (text origin, not the scissor rect).
    pub text_x:      f32,
    /// Horizontal scroll offset in pixels.
    pub scroll_x:    f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxStyle {
    pub fill:         Color,
    pub border:       Color,
    pub border_width: f32,
    pub radius:       f32,
}

impl BoxStyle {
    pub const fn filled(fill: Color) -> Self {
        Self { fill, border: Color::transparent(), border_width: 0.0, radius: 0.0 }
    }
    pub fn with_border(mut self, color: Color, width: f32) -> Self {
        self.border = color; self.border_width = width; self
    }
    pub fn with_radius(mut self, r: f32) -> Self { self.radius = r; self }
}

// ── Cursor node ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct CursorNode {
    pub rect:    Rect,
    pub color:   Color,
    pub visible: bool,
}

// ── Selection node ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SelectionNode {
    pub rect:  Rect,
    pub color: Color,
}

// ── Scrollbar node ────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ScrollbarNode {
    pub track: Rect,
    pub thumb: Rect,
    pub track_color: Color,
    pub thumb_color: Color,
}

// ── Tab bar node ─────────────────────────────────────────────────────────────

/// One tab entry in the tab bar.
#[derive(Debug, Clone)]
pub struct TabEntry {
    pub title:     String,
    pub rect:      Rect,
    pub is_active: bool,
    pub is_hovered: bool,
}

/// Window control button.
#[derive(Debug, Clone)]
pub struct WinButton {
    pub rect:      Rect,
    pub hovered:   bool,
    pub kind:      WinButtonKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WinButtonKind { Close, Maximize, Minimize }

/// Rename overlay inside a tab — drawn over the active tab's text.
#[derive(Debug, Clone)]
pub struct RenameOverlayNode {
    pub tab_index:      usize,
    pub text:           String,
    pub cursor:         usize,
    pub cursor_visible: bool,
    /// Pixel y baseline.
    pub text_y:         f32,
}

/// Full tab bar node — self-contained, no external geometry needed.
#[derive(Debug, Clone)]
pub struct TabBarNode {
    pub rect:         Rect,
    /// Clip x for the tab strip (tabs_clip_x from the old TabBar widget).
    pub tabs_clip_x:  f32,
    pub tabs:         Vec<TabEntry>,
    pub new_tab_rect: Rect,
    pub new_tab_hovered: bool,
    pub win_buttons:  Vec<WinButton>,
    pub rename:       Option<RenameOverlayNode>,
}

// ── Editor node ──────────────────────────────────────────────────────────────

/// One visible text line in the editor.
#[derive(Debug, Clone)]
pub struct TextLine {
    pub line_index: usize,
    pub text:       String,
    /// Pre-computed Y baseline for this line.
    pub y:          f32,
}

/// Cursor in the editor (different from the search-input CursorNode).
#[derive(Debug, Clone, Copy)]
pub struct EditorCursor {
    pub x:           f32,
    pub y:           f32,
    pub line_height: f32,
    pub visible:     bool,
}

/// Selection range in screen coordinates (one rect per selected line).
#[derive(Debug, Clone)]
pub struct EditorSelection {
    /// (x, y, width, height) for each selected row stripe.
    pub rects: Vec<Rect>,
}

/// Flame / typing effect positions.
#[derive(Debug, Clone)]
pub struct FlamePos {
    /// Center x, center y, bottom y, age (0=selection, >0=typing age).
    pub cx: f32, pub cy: f32, pub bottom: f32, pub age: f32,
}

/// Self-contained editor content area node.
#[derive(Debug, Clone)]
pub struct EditorNode {
    pub rect:         Rect,
    pub line_height:  f32,
    pub text_padding: f32,
    pub char_width:   f32,
    pub scroll_x:     f32,
    pub word_wrap:    bool,
    pub lines:        Vec<TextLine>,
    pub cursor:       EditorCursor,
    pub selection:    Option<EditorSelection>,
    pub scrollbar:    Option<ScrollbarNode>,
    pub flames:       Vec<FlamePos>,
}

// ── Node ──────────────────────────────────────────────────────────────────────

/// A node in the declarative UI tree.
/// Produced by `Component::render()`, consumed by the renderer.
/// Never contains GPU handles or mutable widget references.
#[derive(Debug, Clone)]
pub enum Node {
    /// Filled (and optionally bordered) rectangle.
    Box {
        rect:  Rect,
        style: BoxStyle,
    },

    /// A single text string.
    Text {
        text:  String,
        style: TextStyle,
        /// Optional clip rect (text is scissored to this).
        clip:  Option<Rect>,
    },

    /// Blinking text cursor.
    Cursor(CursorNode),

    /// Text selection highlight.
    Selection(SelectionNode),

    /// Scrollbar (track + thumb).
    Scrollbar(ScrollbarNode),

    /// Ordered list of child nodes — painted in order (back to front).
    Layer(Vec<Node>),

    /// Full tab bar — rendered natively for crisp hit-testing + renaming.
    TabBar(TabBarNode),

    /// Editor content area — text lines, cursor, scrollbar, flame effect.
    Editor(EditorNode),
}

impl Node {
    /// Convenience: wrap multiple nodes into a layer.
    pub fn layer(children: impl Into<Vec<Node>>) -> Self {
        Node::Layer(children.into())
    }
}
