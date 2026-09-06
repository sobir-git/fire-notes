//! Focus management — tracks which widget has keyboard focus.
//!
//! The runtime owns a `FocusManager`. Widgets register IDs; Tab cycles through
//! them in order. App code never touches focus state directly.

/// Manages keyboard focus across a set of registered widget IDs.
#[derive(Debug, Default)]
pub struct FocusManager {
    /// Widget IDs in Tab-cycle order.
    order: Vec<u64>,
    /// Index into `order` of the currently focused widget (if any).
    current: Option<usize>,
}

impl FocusManager {
    pub fn new() -> Self { Self::default() }

    /// Register a widget in the Tab cycle. Call in declaration order.
    pub fn register(&mut self, id: u64) {
        if !self.order.contains(&id) {
            self.order.push(id);
        }
        if self.current.is_none() && !self.order.is_empty() {
            self.current = Some(0);
        }
    }

    /// Move focus to the next widget (Tab).
    pub fn focus_next(&mut self) {
        if self.order.is_empty() { return; }
        self.current = Some(match self.current {
            None    => 0,
            Some(i) => (i + 1) % self.order.len(),
        });
    }

    /// Move focus to the previous widget (Shift+Tab).
    pub fn focus_prev(&mut self) {
        if self.order.is_empty() { return; }
        self.current = Some(match self.current {
            None    => self.order.len() - 1,
            Some(0) => self.order.len() - 1,
            Some(i) => i - 1,
        });
    }

    /// Focus a specific widget by ID (e.g., on pointer click).
    pub fn focus(&mut self, id: u64) {
        self.current = self.order.iter().position(|&x| x == id);
    }

    /// Returns the currently focused widget ID, if any.
    pub fn focused_id(&self) -> Option<u64> {
        self.current.map(|i| self.order[i])
    }

    /// Returns `true` if the given widget ID currently has focus.
    pub fn is_focused(&self, id: u64) -> bool {
        self.focused_id() == Some(id)
    }
}
