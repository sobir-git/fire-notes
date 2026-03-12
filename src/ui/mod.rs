//! UI layout and hit-testing

mod types;
mod content_area;
mod layout;
mod notes_picker;
mod tab_bar;
mod list_widget;
mod scrollbar;
mod text_area;
mod text_input;
mod tree;

// Re-export public types used by other modules
#[allow(unused_imports)]
pub use types::{CursorShape, Rect, WindowRect, ResizeEdge, UiAction, UiDragAction, UiHover, UiNode};
#[allow(unused_imports)]
pub use layout::Layout;
pub use content_area::ContentArea;
pub use notes_picker::{NotesPicker, MAX_VISIBLE_ITEMS};
pub use list_widget::ListWidget;
pub use scrollbar::ScrollbarWidget;
pub use tab_bar::TabBar;
#[allow(unused_imports)]
pub use text_area::TextArea;
#[allow(unused_imports)]
pub use text_input::{TextInput, TextInputWidget};
pub use tree::UiTree;
