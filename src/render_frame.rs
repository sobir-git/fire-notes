#![allow(dead_code)]
//! Pure data snapshot of the UI state for rendering.
//!
//! `RenderFrame` is produced by `AppLogic::render_frame()` and consumed by
//! `Renderer::render`.  It contains **no GPU types and no widget references** —
//! just plain Rust structs — so it can be constructed and asserted on in
//! headless unit tests without a window or OpenGL context.

/// A complete description of what the UI should show in one frame.
#[derive(Debug, Clone)]
pub struct RenderFrame {
    /// All open tabs.
    pub tabs: Vec<TabFrameData>,
    /// Index of the currently active tab.
    pub active_tab: usize,
    /// Lines of text visible in the editor (already clipped to scroll window).
    pub visible_lines: Vec<LineData>,
    /// Total number of lines in the active tab's buffer.
    pub total_lines: usize,
    /// First visible line index (scroll offset).
    pub scroll_offset: usize,
    /// Maximum scroll offset for the active tab.
    pub max_scroll_offset: usize,
    /// Horizontal scroll offset in pixels.
    pub scroll_offset_x: f32,
    /// Whether word-wrap is on for the active tab.
    pub word_wrap: bool,
    /// Cursor position and visibility.
    pub cursor: CursorData,
    /// Text selection, if any.
    pub selection: Option<SelectionData>,
    /// Notes picker overlay (None when closed).
    pub notes_picker: Option<NotesPickerFrameData>,
    /// Whether a tab rename is in progress.
    pub rename: Option<RenameData>,
    /// Typing flame positions (line, col, timestamp).
    pub flame_positions: Vec<(usize, usize, std::time::Instant)>,
    /// Window dimensions.
    pub width: f32,
    pub height: f32,
    pub scale: f32,
    // ── Geometry (baked from UiTree at snapshot time) ─────────────────────
    /// Tab bar layout geometry and hover state.
    pub tab_bar: TabBarFrameData,
    /// Content area layout geometry.
    pub content: ContentFrameData,
    /// Scrollbar ratios for logic/test use (None when not scrollable).
    pub scrollbar: Option<ScrollbarData>,
    /// Scrollbar pixel geometry for the renderer (None when not scrollable).
    pub scrollbar_geom: Option<ScrollbarFrameData>,
}

// ── Geometry structs (baked from UiTree) ──────────────────────────────────

/// A plain axis-aligned rectangle in screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Per-tab rendering geometry (screen rect + index).
#[derive(Debug, Clone)]
pub struct TabRectData {
    pub index: usize,
    pub rect: FrameRect,
}

/// Tab bar geometry and hover state, fully baked for the renderer.
#[derive(Debug, Clone)]
pub struct TabBarFrameData {
    pub rect: FrameRect,
    /// Screen x of the right clip boundary (tabs are scissored here).
    pub tabs_clip_x: f32,
    /// All tab rects in scroll-space (already offset by scroll_x).
    pub tabs: Vec<TabRectData>,
    pub new_tab_rect: FrameRect,
    pub minimize_rect: FrameRect,
    pub maximize_rect: FrameRect,
    pub close_rect: FrameRect,
    /// Hover state.
    pub hovered_tab_index: Option<usize>,
    pub hovered_plus: bool,
    pub hovered_minimize: bool,
    pub hovered_maximize: bool,
    pub hovered_close: bool,
}

/// Content area geometry baked for the renderer.
#[derive(Debug, Clone)]
pub struct ContentFrameData {
    pub rect: FrameRect,
    pub text_rect: FrameRect,
    pub line_height: f32,
    pub text_padding: f32,
    pub doc_top_margin: f32,
    pub scrollbar_rect: FrameRect,
}

/// Scrollbar render state: thumb geometry + interaction flags.
#[derive(Debug, Clone)]
pub struct ScrollbarFrameData {
    /// Thumb rect in screen coordinates.
    pub thumb_rect: FrameRect,
    pub hovered: bool,
    pub dragging: bool,
}

// ── Data structs ──────────────────────────────────────────────────────────

/// Data for a single tab in the tab bar.
#[derive(Debug, Clone, PartialEq)]
pub struct TabFrameData {
    pub title: String,
    pub is_active: bool,
}

/// A single line of text in the editor.
#[derive(Debug, Clone, PartialEq)]
pub struct LineData {
    /// Absolute line number in the document.
    pub line_index: usize,
    /// Text content of the line (without trailing newline).
    pub text: String,
}

/// Cursor position and visibility.
#[derive(Debug, Clone, PartialEq)]
pub struct CursorData {
    /// Line index in the document.
    pub line: usize,
    /// Column index within the line (character units).
    pub col: usize,
    /// Whether the cursor blink state is currently visible.
    pub visible: bool,
}

/// Text selection range in document coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectionData {
    /// Start position (line, col) — always ≤ end.
    pub start: (usize, usize),
    /// End position (line, col).
    pub end: (usize, usize),
}

/// Scrollbar thumb position and size (normalised 0..1 ratios of the track).
#[derive(Debug, Clone, PartialEq)]
pub struct ScrollbarData {
    /// Thumb top ratio (0 = top of track, 1 = bottom).
    pub thumb_top_ratio: f32,
    /// Thumb height ratio (0..1).
    pub thumb_height_ratio: f32,
}

/// Notes picker overlay — all geometry and state baked for the renderer.
#[derive(Debug, Clone)]
pub struct NotesPickerFrameData {
    /// Full-window backdrop rect.
    pub backdrop_rect: FrameRect,
    /// Floating panel rect.
    pub overlay_rect: FrameRect,
    /// Search input box rect.
    pub input_rect: FrameRect,
    /// X position for the text baseline inside the input.
    pub input_text_x: f32,
    /// Y position for the text baseline inside the input.
    pub input_text_baseline_y: f32,
    /// Current search query.
    pub query: String,
    /// Horizontal scroll offset of the input text (pixels).
    pub input_scroll_offset: f32,
    /// Cursor position within the query (character index).
    pub input_cursor: usize,
    /// Whether the cursor blink is visible.
    pub cursor_visible: bool,
    /// Text selection within the input (start_char, end_char), if any.
    pub input_selection: Option<(usize, usize)>,
    /// Font size for all picker text.
    pub font_size: f32,
    /// Scale factor.
    pub scale: f32,
    /// X position for the "open" indicator dot.
    pub indicator_x: f32,
    /// Visible list rows, pre-computed.
    pub rows: Vec<NotesPickerRowData>,
    /// Scrollbar track + thumb rects (None when list fits without scrolling).
    pub scrollbar: Option<(FrameRect, FrameRect)>,
    /// True if the list is empty but a query is active (show "no results").
    pub no_results: bool,
    /// Y baseline of the first list row (used for no-results message).
    pub first_row_baseline_y: f32,
}

/// One rendered row in the notes picker list.
#[derive(Debug, Clone)]
pub struct NotesPickerRowData {
    pub title: String,
    pub is_open: bool,
    pub is_selected: bool,
    /// Bounding rect of the row (for highlight).
    pub row_rect: FrameRect,
    /// Y of the text baseline.
    pub baseline_y: f32,
    /// Y of the center (for the indicator dot).
    pub center_y: f32,
}

/// Tab rename overlay data.
#[derive(Debug, Clone, PartialEq)]
pub struct RenameData {
    /// Index of the tab being renamed.
    pub tab_index: usize,
    /// Current text in the rename input.
    pub text: String,
    /// Cursor position within the rename input.
    pub cursor: usize,
}
