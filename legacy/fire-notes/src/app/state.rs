//! Application result types

use crate::ui::ResizeEdge;

/// Result type for application actions that may trigger UI updates
#[must_use = "Handle the AppResult to ensure the UI updates correctly"]
pub enum AppResult {
    /// No action needed
    Ok,
    /// UI needs to be redrawn
    Redraw,
    /// Minimize window
    WindowMinimize,
    /// Maximize/restore window
    WindowMaximize,
    /// Close window
    WindowClose,
    /// Start window drag
    WindowDrag,
    /// Start window resize from edge
    WindowResize(ResizeEdge),
}

impl AppResult {
    pub fn needs_redraw(&self) -> bool {
        matches!(self, AppResult::Redraw)
    }
}
