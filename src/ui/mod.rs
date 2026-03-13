//! UI layout and hit-testing

mod types;
mod content_area;
mod layout;
mod tab_bar;
mod text_area;
mod text_input;
mod tree;

// Public API — used across the codebase
pub use types::{CursorShape, Rect, WindowRect, ResizeEdge, UiAction, UiDragAction, UiNode};
pub use text_input::TextInput;
pub use tree::UiTree;
