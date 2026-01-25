//! Centralized configuration constants for Fire Notes
//!
//! All magic numbers and tunable parameters should be defined here.
//! Some constants may be defined for future use or documentation purposes.

#![allow(dead_code)]

/// Layout constants (in logical pixels, will be scaled by DPI)
pub mod layout {
    /// Height of edit line (line spacing)
    pub const LINE_HEIGHT: f32 = 24.0;
    /// Height of the tab bar
    pub const TAB_HEIGHT: f32 = 40.0;
    /// General padding around content areas
    pub const PADDING: f32 = 16.0;
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
}

/// Timing constants (in milliseconds)
pub mod timing {
    /// Cursor blink interval
    pub const CURSOR_BLINK_MS: u64 = 500;
    /// Throttle for drag-scroll when selecting outside viewport
    pub const DRAG_SCROLL_THROTTLE_MS: u64 = 50;
    /// Double-click detection window
    pub const DOUBLE_CLICK_MS: u64 = 500;
    /// Double-click max distance (pixels)
    pub const DOUBLE_CLICK_DISTANCE: f64 = 5.0;
}

/// Rendering constants
pub mod rendering {
    /// Fallback monospace character width (before font measurement)
    pub const FALLBACK_CHAR_WIDTH: f32 = 9.6;
    /// Default font size for content
    pub const CONTENT_FONT_SIZE: f32 = 16.0;
    /// Default font size for tab titles
    pub const TAB_FONT_SIZE: f32 = 14.0;
    /// Approximate character width ratio for tab width calculation
    pub const TAB_CHAR_WIDTH_RATIO: f32 = 9.0;
}

/// Scroll behavior constants
pub mod scroll {
    /// Lines to scroll per wheel tick
    pub const LINES_PER_WHEEL_TICK: usize = 1;
    /// Pixels per scroll for tab bar horizontal scroll
    pub const TAB_SCROLL_PIXELS: f32 = 30.0;
}
