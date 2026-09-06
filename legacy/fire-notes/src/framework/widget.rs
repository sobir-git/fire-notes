#![allow(dead_code)]
//! Widget trait — implemented by components that use the Element tree framework.
//!
//! A component implementing `Widget` only needs:
//!   1. `build(&self)` — returns a pure `Element<Msg>` tree (no rects, no events)
//!   2. `update(&mut self, msg: Msg)` — applies a message produced by a callback
//!
//! The framework calls `build()` each frame, runs `layout_tree()` to assign rects,
//! and calls `update()` with any messages produced by user interactions.

use crate::framework::element::Element;

/// A component that participates in the Element tree framework.
pub trait Widget {
    /// The message type produced by this component's callbacks.
    type Msg;

    /// Declare the component's UI as a pure Element tree.
    /// Called once per frame. Must not mutate self.
    fn build(&self) -> Element<Self::Msg>;

    /// Apply a message produced by a callback during event dispatch.
    /// This is the only place the component mutates its state.
    fn update(&mut self, msg: Self::Msg);
}
