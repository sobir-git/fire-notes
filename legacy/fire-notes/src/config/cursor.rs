//! Cursor configuration

#![allow(dead_code)]

/// Editor area cursor type
/// Options from winit::window::CursorIcon:
/// - Text: I-beam cursor (default for text editors)
/// - Help: Arrow with question mark
/// - Crosshair: Crosshair cursor
/// - Cell: Cell selection cursor
/// - VerticalText: Vertical I-beam
/// - Alias: Alias cursor (curved arrow)
/// - Copy: Copy cursor (arrow with plus)
/// - Move: Move cursor (crossed arrows)
/// - NoDrop: No drop cursor (slashed circle)
/// - NotAllowed: Not allowed cursor (slashed circle)
/// - Grab: Grab cursor (open hand)
/// - Grabbing: Grabbing cursor (closed hand)
/// - Progress: Progress cursor (arrow with spinning circle)
/// - Wait: Wait cursor (spinning circle)
/// - ContextMenu: Context menu cursor
/// - ZoomIn: Zoom in cursor (magnifying glass with plus)
/// - ZoomOut: Zoom out cursor (magnifying glass with minus)
/// - Cell: Cell selection cursor
/// - AllScroll: Scroll in all directions cursor
pub const EDITOR_CURSOR_TYPE: &str = "Text";
