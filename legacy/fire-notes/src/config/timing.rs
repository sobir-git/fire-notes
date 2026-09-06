//! Timing constants (in milliseconds)

#![allow(dead_code)]

/// Cursor blink interval
pub const CURSOR_BLINK_MS: u64 = 500;
/// Throttle for drag-scroll when selecting outside viewport
pub const DRAG_SCROLL_THROTTLE_MS: u64 = 50;
/// Double-click detection window
pub const DOUBLE_CLICK_MS: u64 = 500;
/// Double-click max distance (pixels)
pub const DOUBLE_CLICK_DISTANCE: f64 = 5.0;
