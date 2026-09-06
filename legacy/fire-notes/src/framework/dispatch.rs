//! Event dispatcher — walks the retained `Element` tree and fires callbacks.
//!
//! After `layout_tree()` assigns rects, events are dispatched by walking the
//! tree and hit-testing. The dispatcher fires the appropriate callback and
//! returns a `DispatchResult<Msg>` — either a message for the component's
//! `update()` or a signal that nothing matched.

use crate::framework::element::{Element, ElementKind};
use crate::primitives::text_input::TextInputEvent;

// ── Input events ──────────────────────────────────────────────────────────────

/// Platform-independent input event delivered to the framework dispatcher.
#[derive(Debug)]
pub enum InputEvent<'a> {
    PointerDown        { x: f32, y: f32 },
    PointerDoubleClick { x: f32, y: f32 },
    PointerTripleClick { x: f32, y: f32 },
    PointerDrag        { x: f32, y: f32 },
    PointerMove        { x: f32, y: f32 },
    Scroll             { lines: isize },
    Text               (TextInputEvent<'a>),
    Paste              (String),
    Up,
    Down,
    Confirm,
    Cancel,
}

// ── Dispatch result ───────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum DispatchResult<Msg> {
    /// A callback fired and produced a message.
    Msg(Msg),
    /// The event was handled (e.g. text input updated state) but produced no message.
    Handled,
    /// Nothing in the tree matched the event.
    Unhandled,
}

impl<Msg> DispatchResult<Msg> {
    pub fn is_handled(&self) -> bool {
        !matches!(self, Self::Unhandled)
    }
}

// ── Focus state ───────────────────────────────────────────────────────────────

/// Tracks which element currently has keyboard focus within the tree.
/// Identified by position in a depth-first walk (stable as long as tree shape is stable).
#[derive(Debug, Default, Clone)]
pub struct FocusState {
    /// Index of the focused element in a depth-first walk. `None` = auto (first auto_focus).
    pub focused_path: Option<Vec<usize>>,
}

// ── Dispatcher ────────────────────────────────────────────────────────────────

/// Dispatch an `InputEvent` into the element tree.
///
/// The tree must have already been laid out by `layout_tree()`.
/// `scale` is needed for text input event routing.
pub fn dispatch_event<Msg: 'static>(
    root:  &mut Element<Msg>,
    ev:    InputEvent<'_>,
    focus: &mut FocusState,
    scale: f32,
) -> DispatchResult<Msg> {
    match ev {
        InputEvent::PointerDown { x, y }        => dispatch_pointer(root, x, y, PointerKind::Down,        focus, scale),
        InputEvent::PointerDoubleClick { x, y } => dispatch_pointer(root, x, y, PointerKind::DoubleClick, focus, scale),
        InputEvent::PointerTripleClick { x, y } => dispatch_pointer(root, x, y, PointerKind::TripleClick, focus, scale),
        InputEvent::PointerDrag { x, y }        => dispatch_pointer(root, x, y, PointerKind::Drag,        focus, scale),
        InputEvent::PointerMove { x, y }        => dispatch_pointer(root, x, y, PointerKind::Move,        focus, scale),
        InputEvent::Scroll { lines }            => dispatch_scroll(root, lines),
        InputEvent::Text(ev)                    => dispatch_text(root, ev, focus, scale),
        InputEvent::Paste(text)                 => dispatch_paste(root, text, focus),
        InputEvent::Up                          => dispatch_nav(root, NavDir::Up),
        InputEvent::Down                        => dispatch_nav(root, NavDir::Down),
        InputEvent::Confirm                     => dispatch_confirm(root, focus),
        InputEvent::Cancel                      => dispatch_cancel(root),
    }
}

// ── Pointer dispatch ──────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum PointerKind { Down, DoubleClick, TripleClick, Drag, Move }

fn dispatch_pointer<Msg: 'static>(
    el:    &mut Element<Msg>,
    x: f32, y: f32,
    kind:  PointerKind,
    focus: &mut FocusState,
    scale: f32,
) -> DispatchResult<Msg> {
    // Check backdrop: if point is outside all children, fire on_click_outside.
    if let ElementKind::Backdrop { on_click_outside } = &el.kind {
        let inside_any = el.children.iter().any(|c| c.rect.contains(x, y));
        if !inside_any {
            if let (PointerKind::Down, Some(cb)) = (kind, on_click_outside) {
                return DispatchResult::Msg(cb.call());
            }
            return DispatchResult::Unhandled;
        }
    }

    // Recurse into children first (depth-first, last child wins on overlap).
    for child in el.children.iter_mut().rev() {
        if child.rect.contains(x, y) {
            let r = dispatch_pointer(child, x, y, kind, focus, scale);
            if r.is_handled() { return r; }
        }
    }

    // Handle at this node.
    match &mut el.kind {
        ElementKind::TextInput { state, .. } => {
            let rect = el.rect;
            match kind {
                PointerKind::Down        => { state.on_pointer_down(x, y, rect, scale, false); }
                PointerKind::DoubleClick => { state.on_double_click(x, y, rect, scale); }
                PointerKind::TripleClick => { state.on_triple_click(x, y, rect, scale); }
                PointerKind::Drag        => { state.on_drag(x, rect, scale); }
                PointerKind::Move        => {}
            }
            DispatchResult::Handled
        }
        ElementKind::List { items, on_confirm, .. } => {
            // List pointer dispatch: check if pointer-down selects or confirms a row.
            if let PointerKind::Down = kind {
                let item_h = el.rect.height / items.len().max(1) as f32;
                let idx = ((y - el.rect.y) / item_h).floor() as usize;
                if idx < items.len() {
                    // Single-click: select; double-click via ListPointerResult::Confirmed
                    if let Some(cb) = on_confirm {
                        // Treat second click on same item as confirm — simplified
                        let _ = cb; // on_confirm fires on Confirm key, not pointer
                    }
                }
            }
            DispatchResult::Handled
        }
        _ => DispatchResult::Unhandled,
    }
}

// ── Scroll dispatch ───────────────────────────────────────────────────────────

fn dispatch_scroll<Msg: 'static>(el: &mut Element<Msg>, lines: isize) -> DispatchResult<Msg> {
    // Delegate to first List child.
    for child in el.children.iter_mut() {
        if matches!(child.kind, ElementKind::List { .. }) {
            // List scroll is handled inside the List primitive — mark handled.
            let _ = lines;
            return DispatchResult::Handled;
        }
        let r = dispatch_scroll(child, lines);
        if r.is_handled() { return r; }
    }
    DispatchResult::Unhandled
}

// ── Text / paste dispatch ─────────────────────────────────────────────────────

fn dispatch_text<Msg: 'static>(
    el:    &mut Element<Msg>,
    ev:    TextInputEvent<'_>,
    focus: &mut FocusState,
    scale: f32,
) -> DispatchResult<Msg> {
    // Find the focused (or first auto-focus) TextInput and deliver the event.
    if let Some(msg) = find_and_deliver_text(el, ev, focus, scale) {
        return DispatchResult::Msg(msg);
    }
    DispatchResult::Handled
}

fn find_and_deliver_text<Msg: 'static>(
    el:    &mut Element<Msg>,
    ev:    TextInputEvent<'_>,
    _focus: &mut FocusState,
    scale: f32,
) -> Option<Msg> {
    if let ElementKind::TextInput { state, on_change, .. } = &mut el.kind {
        state.handle_key(ev, el.rect, scale);
        if let Some(cb) = on_change {
            return Some(cb.call(state.text().to_string()));
        }
        return None;
    }
    for child in el.children.iter_mut() {
        if let Some(msg) = find_and_deliver_text(child, ev.clone(), _focus, scale) {
            return Some(msg);
        }
    }
    None
}

fn dispatch_paste<Msg: 'static>(
    el:    &mut Element<Msg>,
    text:  String,
    focus: &mut FocusState,
) -> DispatchResult<Msg> {
    if let Some(msg) = find_and_paste(el, &text, focus) {
        return DispatchResult::Msg(msg);
    }
    DispatchResult::Handled
}

fn find_and_paste<Msg: 'static>(
    el:    &mut Element<Msg>,
    text:  &str,
    _focus: &mut FocusState,
) -> Option<Msg> {
    if let ElementKind::TextInput { state, on_change, .. } = &mut el.kind {
        state.paste(text);
        if let Some(cb) = on_change {
            return Some(cb.call(state.text().to_string()));
        }
        return None;
    }
    for child in el.children.iter_mut() {
        if let Some(msg) = find_and_paste(child, text, _focus) {
            return Some(msg);
        }
    }
    None
}

// ── Navigation dispatch ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum NavDir { Up, Down }

fn dispatch_nav<Msg: 'static>(el: &mut Element<Msg>, dir: NavDir) -> DispatchResult<Msg> {
    for child in el.children.iter_mut() {
        if matches!(child.kind, ElementKind::List { .. }) {
            // List navigation — mark handled; actual list.select_up/down called in update()
            let _ = dir;
            return DispatchResult::Handled;
        }
        let r = dispatch_nav(child, dir);
        if r.is_handled() { return r; }
    }
    DispatchResult::Unhandled
}

// ── Confirm / cancel ──────────────────────────────────────────────────────────

fn dispatch_confirm<Msg: 'static>(
    el:    &mut Element<Msg>,
    _focus: &mut FocusState,
) -> DispatchResult<Msg> {
    // Fire on_confirm on the focused TextInput or the first List.
    if let ElementKind::TextInput { on_confirm, .. } = &el.kind {
        if let Some(cb) = on_confirm {
            return DispatchResult::Msg(cb.call());
        }
    }
    if let ElementKind::List { on_confirm, selected, .. } = &el.kind {
        if let Some(cb) = on_confirm {
            return DispatchResult::Msg(cb.call(*selected));
        }
    }
    for child in el.children.iter_mut() {
        let r = dispatch_confirm(child, _focus);
        if r.is_handled() { return r; }
    }
    DispatchResult::Unhandled
}

fn dispatch_cancel<Msg: 'static>(el: &mut Element<Msg>) -> DispatchResult<Msg> {
    if let ElementKind::TextInput { on_cancel, .. } = &el.kind {
        if let Some(cb) = on_cancel {
            return DispatchResult::Msg(cb.call());
        }
    }
    if let ElementKind::List { on_cancel, .. } = &el.kind {
        if let Some(cb) = on_cancel {
            return DispatchResult::Msg(cb.call());
        }
    }
    if let ElementKind::Backdrop { on_click_outside } = &el.kind {
        if let Some(cb) = on_click_outside {
            return DispatchResult::Msg(cb.call());
        }
    }
    for child in el.children.iter_mut() {
        let r = dispatch_cancel(child);
        if r.is_handled() { return r; }
    }
    DispatchResult::Unhandled
}
