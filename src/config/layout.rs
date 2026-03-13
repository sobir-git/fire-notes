//! Layout constants (in logical pixels, will be scaled by DPI)

#![allow(dead_code)]

/// Height of edit line (line spacing)
pub const LINE_HEIGHT: f32 = 24.0;
/// Height of the tab bar
pub const TAB_HEIGHT: f32 = 40.0;
/// General padding around content areas
pub const PADDING: f32 = 16.0;
/// Top margin before the first line of text — part of the document, scrolls away with content.
/// When at scroll offset 0 the first line is indented by this amount; scroll down and it disappears.
pub const DOC_TOP_MARGIN: f32 = 8.0;
/// Width of the scrollbar
pub const SCROLLBAR_WIDTH: f32 = 12.0;
/// Minimum scrollbar thumb height
pub const MIN_SCROLLBAR_THUMB: f32 = 30.0;
/// Tab horizontal padding
pub const TAB_PADDING: f32 = 16.0;
/// Minimum tab width
pub const MIN_TAB_WIDTH: f32 = 100.0;
/// New tab button size
pub const NEW_TAB_BUTTON_SIZE: f32 = 28.0;
/// Window control button size (minimize/maximize/close)
pub const WINDOW_BUTTON_SIZE: f32 = 28.0;
/// Margin between window edge and close button, and between buttons
pub const WINDOW_BUTTON_MARGIN: f32 = 8.0;
/// Gap between window control buttons
pub const WINDOW_BUTTON_GAP: f32 = 4.0;
/// Dead zone between the scrolling tabs area and window controls, used for window dragging
pub const TAB_DRAG_GAP: f32 = 36.0;
