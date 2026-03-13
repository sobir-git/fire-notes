#![allow(unused_imports)]
//! Reusable UI primitives — one file each, state + geometry + events.
//!
//! Each primitive owns its complete state and geometry. No external state threading.

pub mod button;
pub mod label;
pub mod list;
pub mod scrollbar;
pub mod text_input;

pub use button::Button;
pub use label::Label;
pub use list::{List, ListPointerResult};
pub use scrollbar::{Scrollbar, ScrollbarAction, ThumbMetrics};
pub use text_input::TextInput;
