//! Pure data snapshot of the UI state for rendering.
//!
//! `RenderFrame` is produced by `AppLogic::render_frame()` and consumed by
//! `Renderer::render_frame()`.  It contains **no GPU types** — just plain Rust
//! structs — so it can be constructed and asserted on in headless unit tests
//! without a window or OpenGL context.

#![allow(dead_code)]

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
    /// Cursor state.
    pub cursor: CursorData,
    /// Text selection, if any.
    pub selection: Option<SelectionData>,
    /// Scrollbar metrics (None when no scrollbar is needed).
    pub scrollbar: Option<ScrollbarData>,
    /// Notes picker overlay (None when closed).
    pub notes_picker: Option<NotesPickerData>,
    /// Whether a tab rename is in progress.
    pub rename: Option<RenameData>,
    /// Typing flame positions (line, col).
    pub flame_positions: Vec<(usize, usize)>,
    /// Window dimensions.
    pub width: f32,
    pub height: f32,
    pub scale: f32,
}

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

/// Notes picker overlay data.
#[derive(Debug, Clone)]
pub struct NotesPickerData {
    /// Current search query text.
    pub query: String,
    /// Filtered list of note entries (title, path string, is_open).
    pub entries: Vec<NotesPickerEntry>,
    /// Index of the highlighted entry.
    pub selected_index: usize,
    /// Scroll offset within the list.
    pub scroll_offset: usize,
}

/// A single entry in the notes picker list.
#[derive(Debug, Clone, PartialEq)]
pub struct NotesPickerEntry {
    pub title: String,
    pub path: String,
    pub is_open: bool,
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
