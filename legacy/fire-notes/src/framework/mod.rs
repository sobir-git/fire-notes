//! Framework core — Element tree, layout engine, event dispatcher.
//!
//! This module contains the retained-tree framework that eliminates the
//! render()/on_event() split from all overlay components.
//!
//! # Usage
//!
//! ```ignore
//! // A component is pure state + one build() function.
//! impl NotesPicker {
//!     fn build(&self) -> Element<Msg> {
//!         column_el()
//!             .child(text_input_el(&self.search, "Search...").auto_focus(true).on_change(Msg::QueryChanged).build())
//!             .child(make_list_el(items).on_confirm(|| Msg::Open).on_cancel(|| Msg::Close).build())
//!             .build()
//!     }
//!     fn update(&mut self, msg: Msg) { ... }
//! }
//! ```

pub mod element;
pub mod widget;
pub mod layout;
pub mod dispatch;
pub mod render;
pub mod runner;

pub use runner::{run_render, run_event};
