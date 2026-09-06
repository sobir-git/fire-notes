//! `App` trait — the only thing app code implements.

use crate::layout::Node;
use crate::primitives::text_input::{EventOutcome, TextInputEvent};
use crate::runtime::view_ctx::ViewCtx;

/// Implement this to make a runnable application.
///
/// The runtime calls `view()` every time it needs to render, passing a
/// [`ViewCtx`] with all framework-managed state (focus, blink, scale, …).
/// App code never touches winit, OpenGL, clipboard, or timers directly.
pub trait App {
    /// Declare the current UI as a pure node tree.
    /// `ctx` is `&mut` so widgets can register their rects via [`ViewCtx::register_rect`]
    /// during rendering, making those rects available to [`App::handle_key`].
    fn view(&mut self, ctx: &mut ViewCtx) -> Node;

    /// Deliver a keyboard event to the widget identified by `id`.
    /// The runtime routes this to whichever of the app's `TextInput` fields
    /// owns that ID. Return the outcome so the runtime can handle clipboard.
    ///
    /// Default: no-op (apps without text inputs can omit this).
    /// Deliver a keyboard event to the widget identified by `id`.
    /// `ctx` carries the layout cache from the last `view()` call so the widget
    /// can find its rendered rect via `ctx.rect_for(id)`.
    fn handle_key(&mut self, id: u64, event: TextInputEvent<'_>, ctx: &ViewCtx) -> EventOutcome {
        let _ = (id, event, ctx);
        EventOutcome::Nothing
    }

    /// Deliver clipboard paste text to the widget identified by `id`.
    fn handle_paste(&mut self, id: u64, text: &str, ctx: &ViewCtx) {
        let _ = (id, text, ctx);
    }

    /// Deliver a pointer-down event at window coordinates `(x, y)`.
    ///
    /// The app should:
    /// 1. Find which widget (if any) was hit.
    /// 2. Call `TextInput::on_pointer_down` on it to set the caret.
    /// 3. Return the ID of the focused widget, or `None` if the click missed all inputs.
    ///
    /// The runtime updates focus to that ID automatically.
    fn handle_pointer_down(&mut self, x: f32, y: f32, ctx: &ViewCtx) -> Option<u64> {
        let _ = (x, y, ctx);
        None
    }

    /// Return the ordered list of focusable widget IDs.
    /// The runtime uses this to build the Tab cycle.
    /// Called once at startup and again when the list may have changed.
    fn focus_order(&self) -> Vec<u64> { vec![] }
}
