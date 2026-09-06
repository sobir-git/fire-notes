//! Framework runner — build → layout → render/dispatch pipeline in one place.
//!
//! Components implement `OverlayWidget` (four methods total).
//! `ActiveOverlay` calls `run_render` / `run_event`. Nothing else needed.

use crate::app::overlay_event::{OverlayEvent, OverlayResult};
use crate::framework::dispatch::{dispatch_event, DispatchResult, FocusState, InputEvent};
use crate::framework::layout::layout_tree;
use crate::framework::render::render_element;
use crate::framework::widget::Widget;
use crate::layout::Node;
use crate::theme::Theme;
use crate::ui::Rect;

// ── OverlayWidget — the only trait a component implements ─────────────────────

pub trait OverlayWidget: Widget {
    fn panel_rect(&self, window: Rect, scale: f32) -> Rect;
    fn result_for(&self, msg: &Self::Msg) -> OverlayResult;
}

// ── Render pipeline ───────────────────────────────────────────────────────────

pub fn run_render<W>(widget: &W, window: Rect, scale: f32, theme: &Theme, cursor_visible: bool) -> Node
where
    W: OverlayWidget,
    W::Msg: 'static,
{
    let mut el = widget.build();
    layout_tree(&mut el, widget.panel_rect(window, scale), scale);
    render_element(&el, theme, scale, cursor_visible)
}

// ── Event pipeline ────────────────────────────────────────────────────────────

pub fn run_event<W>(widget: &mut W, ev: OverlayEvent<'_>, window: Rect, scale: f32) -> OverlayResult
where
    W: OverlayWidget,
    W::Msg: 'static,
{
    let mut el = widget.build();
    layout_tree(&mut el, widget.panel_rect(window, scale), scale);

    let mut focus = FocusState::default();
    match dispatch_event(&mut el, overlay_to_input(ev), &mut focus, scale) {
        DispatchResult::Msg(msg) => {
            let result = widget.result_for(&msg);
            widget.update(msg);
            result
        }
        DispatchResult::Handled   => OverlayResult::Redraw,
        DispatchResult::Unhandled => OverlayResult::Nothing,
    }
}

// ── OverlayEvent → InputEvent (one translation for all components) ────────────

fn overlay_to_input(ev: OverlayEvent<'_>) -> InputEvent<'_> {
    match ev {
        OverlayEvent::PointerDown { x, y }        => InputEvent::PointerDown { x, y },
        OverlayEvent::PointerDoubleClick { x, y } => InputEvent::PointerDoubleClick { x, y },
        OverlayEvent::PointerTripleClick { x, y } => InputEvent::PointerTripleClick { x, y },
        OverlayEvent::PointerDrag { x }           => InputEvent::PointerDrag { x, y: 0.0 },
        OverlayEvent::PointerMove { x, y }        => InputEvent::PointerMove { x, y },
        OverlayEvent::Scroll { lines }            => InputEvent::Scroll { lines },
        OverlayEvent::Text(t)                     => InputEvent::Text(t),
        OverlayEvent::Paste(s)                    => InputEvent::Paste(s),
        OverlayEvent::Up                          => InputEvent::Up,
        OverlayEvent::Down                        => InputEvent::Down,
        OverlayEvent::Confirm                     => InputEvent::Confirm,
        OverlayEvent::Cancel                      => InputEvent::Cancel,
    }
}
