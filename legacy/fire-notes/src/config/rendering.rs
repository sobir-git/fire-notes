//! Rendering constants

#![allow(dead_code)]

/// Fallback monospace character width (before font measurement)
pub const FALLBACK_CHAR_WIDTH: f32 = 9.6;
/// Default font size for content
pub const CONTENT_FONT_SIZE: f32 = 16.0;
/// Default font size for tab titles
pub const TAB_FONT_SIZE: f32 = 14.0;
/// Approximate character width ratio for tab width calculation
pub const TAB_CHAR_WIDTH_RATIO: f32 = 9.0;
/// Font size for the new-tab (+) button
pub const NEW_TAB_BUTTON_FONT_SIZE: f32 = 20.0;
/// Font size for the notes picker overlay
pub const NOTES_PICKER_FONT_SIZE: f32 = 14.0;
/// Fallback monospace character width as a raw pixel value (before scaling)
pub const FALLBACK_CHAR_WIDTH_PX: f32 = 9.6;
