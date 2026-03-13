# fire-notes UI Framework

## Philosophy

Five rules:

1. **One file per component** — one file tells you everything about one widget. No widget reaches outside its own file for layout, state, or event handling.
2. **Rects flow down** — a parent subdivides its space and hands sub-rects to children. Children never reach upward for geometry.
3. **The Node tree is the firewall** — the only boundary between logic and GPU is a declarative `Node` tree. `AppLogic::render()` produces it; `NodeRenderer` consumes it. No intermediate snapshot structs.
4. **Widgets are retained** — a widget owns its state AND its geometry. `relayout()` updates geometry without losing state. Geometry is never rebuilt on every event.
5. **`AppLogic` is always headless** — `src/logic/` never imports `femtovg` or any platform dep. `cargo test` runs the full logic suite with zero GPU.

---

## Layer Map

```
┌─────────────────────────────────────────────────────────┐
│  App shell  (src/app/)                                  │
│  Routes OS events → AppLogic methods                   │
│  Calls renderer.render(&node, width, height)            │
├─────────────────────────────────────────────────────────┤
│  AppLogic  (src/logic/)                                 │
│  Owns all state: tabs, focus, overlay, inline widget.   │
│  render() → Node tree (pure data, no GPU)               │
│  Zero GPU imports. Fully headless-testable.             │
├─────────────────────────────────────────────────────────┤
│  Components  (src/components/)                          │
│  One file each. Implements Overlay or InlineWidget.     │
│  NotesPicker, SlashMenu — Overlay trait                 │
│  TabRename — InlineWidget trait                         │
├─────────────────────────────────────────────────────────┤
│  Primitives  (src/primitives/)                          │
│  Button, TextInput, Label, Scrollbar, List              │
│  One file each. render_at(rect, scale) → Node.          │
├─────────────────────────────────────────────────────────┤
│  Layout  (src/layout/)                                  │
│  Column, Row — pure geometry, no GPU.                   │
│  floating_rect() — anchored overlay positioning.        │
│  Overlay / InlineWidget / Component traits.             │
├─────────────────────────────────────────────────────────┤
│  NodeRenderer  (src/renderer/node_renderer.rs)          │
│  Single walker — reads Node tree, calls femtovg.        │
│  The only file that touches the GPU API.                │
├─────────────────────────────────────────────────────────┤
│  Rect  (src/ui/types.rs)                                │
│  The only place coordinate arithmetic lives.            │
└─────────────────────────────────────────────────────────┘
```

---

## Component Contract

Regular components (tab bar, editor, etc.) own their own state, layout, and
rendering snapshot:

```rust
// src/components/my_component.rs

pub struct MyComponent { ... }

impl MyComponent {
    pub fn on_pointer_down(&mut self, x: f32, y: f32) -> MyEvent { ... }
    pub fn snapshot(&self) -> MyView { ... }   // pure data; renderer reads this
}
```

**Full-screen overlays** (pickers, dialogs) implement the `Overlay` trait instead.
See _Overlay Trait_ section below.

---

## Widget Trait

Every interactive primitive implements `Widget` (`src/layout/mod.rs`):

```rust
pub trait Widget {
    type Event;
    fn on_pointer_down(&mut self, x: f32, y: f32) -> Self::Event;
    fn on_hover(&mut self, x: f32, y: f32) -> bool;          // returns needs_redraw
    fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape;
}
```

- `TextInput::Event = InputEvent` (`Focused` | `Miss`)
- `List<T>::Event = ListPointerResult` (`Selected` | `Confirmed(usize)` | `Scrolled` | …)

Components hold primitives directly and use `Column::hit_slot` to dispatch:

```rust
fn on_pointer_down(&mut self, x: f32, y: f32, char_width: f32) -> PickerEvent {
    match self.layout.hit_slot(x, y) {
        Some(0) => { self.search.on_pointer_down_with(x, y, char_width); PickerEvent::Redraw }
        Some(1) => self.list.on_pointer_down(x, y).into_picker_event(&self.list),
        _       => PickerEvent::None,
    }
}
```

No manual rect-contains checks. No if-else chains. The combinator routes; the component maps.

---

## Event Return Type

```rust
pub enum EventResponse<Msg = ()> {
    Pass,        // not this widget's concern — propagate upward
    Handled,     // consumed, no redraw needed
    Redraw,      // consumed, redraw needed
    Emit(Msg),   // produced a business message for the parent (implies redraw)
}
```

`Pass` is the only non-consuming variant. When a component's callers only need a
plain bool or a typed result, skip `EventResponse` and return the natural type.

---

## Overlay Trait

Full-screen overlays (pickers, dialogs, …) implement `Overlay` (`src/layout/mod.rs`):

```rust
pub trait Overlay {
    fn render(&self, theme: &Theme) -> Node;               // declare UI as a Node tree
    fn on_event(&mut self, ev: FrameworkEvent) -> OverlayResult;
    fn on_drag(&mut self, x: f32, y: f32) -> bool;        // scrollbar drag
    fn end_drag(&mut self);
    fn contains(&self, x: f32, y: f32) -> bool;
    fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape;
}
```

`OverlayResult` is what every event method returns — no knowledge of `AppResult`:

```rust
pub enum OverlayResult {
    Nothing,
    Redraw,
    Close,
    OpenPath(PathBuf),   // overlay completed: open this file
}
```

`AppLogic` holds `overlay: Option<Box<dyn Overlay>>`. It never imports the concrete type.
All routing goes through `dispatch_overlay(FrameworkEvent)` → `handle_overlay_result(OverlayResult)`.

**The only file that names `NotesPicker` by type is `app/notes_picker.rs` (the instantiation site).**
Everything else is generic:

| File | What it does |
|---|---|
| `logic/mod.rs` | `overlay: Option<Box<dyn Overlay>>`, `dispatch_overlay()` |
| `app/focus/mod.rs` | `Focus::Overlay` variant |
| `logic/actions/edit_ops.rs` | `is_overlay()` guard → `dispatch_overlay(Backspace/Char)` |
| `logic/actions/cursor_ops.rs` | `is_overlay()` guard → `dispatch_overlay(ArrowUp/Down)` |
| `app/mouse/drag.rs` | `o.on_drag(x, y)` / `o.end_drag()` |
| `app/mouse/click.rs` | `dispatch_overlay(PointerDown)` |
| `logic/mod.rs` (render) | `overlay.as_ref().map(\|o\| o.render(&self.theme))` |

Adding a second overlay (e.g. Find dialog) = **one new file** implementing `Overlay`.
Zero other files change.

---

## FrameworkEvent

The typed event enum delivered to overlays and components:

```rust
pub enum FrameworkEvent {
    Char(char),
    Backspace,
    Delete,
    ArrowUp,
    ArrowDown,
    Key(Key),                                    // escape, enter, etc.
    PointerDown { x: f32, y: f32, char_width: f32 },
    PointerMove { x: f32, y: f32 },
    PointerDrag { x: f32, char_width: f32 },
    Scroll { lines: isize },
}
```

Dispatched from the OS layer through `AppLogic::dispatch_overlay()`. The overlay handles
what it cares about; everything else returns `OverlayResult::Nothing`.

---

## Focus Model

`Focus` is a lightweight discriminant — it identifies *what* has keyboard focus, not *where the state lives*:

```rust
pub enum Focus {
    Editor,
    TabRename,  // state lives in: AppLogic.rename_input: Option<(usize, TextInput)>
    Overlay,    // state lives in: AppLogic.overlay: Option<Box<dyn Overlay>>
}
```

Widget state lives in dedicated `AppLogic` fields. `Focus` is just a tag.
This eliminates borrow-fighting when accessing multiple fields of the same widget.

---

## Composability: Layout Combinators

`Column` / `Row` are thin wrappers over `Rect::split_v` / `Rect::split_h`.
Negative weight = fixed pixels × scale; positive = flex fill.

```rust
// NotesPicker layout — 4 lines, zero manual geometry
fn apply_layout(&mut self, overlay: Rect, scale: f32) {
    let col = Column::new(overlay.inset(PADDING * scale), scale, &[-INPUT_H, 1.0]);
    self.search.relayout(col.slot(0), scale);
    self.list.relayout(col.slot(1), scale);
}
```

`Column::hit_slot(x, y)` returns the slot index containing the point — components
use this for event dispatch without writing any `rect.contains` checks.

```rust
match self.layout.hit_slot(x, y) {
    Some(0) => /* input was hit */,
    Some(1) => /* list was hit */,
    _       => /* outside overlay */,
}
```

---

## Adding a New Overlay

One file. No registration. No macro. No codegen.

```rust
// src/components/find_dialog.rs
use crate::layout::{Column, FrameworkEvent, Node, Overlay, OverlayResult};
use crate::primitives::TextInput;
use crate::ui::{CursorShape, Rect};

pub struct FindDialog { input: TextInput, overlay: Rect, scale: f32 }

impl FindDialog {
    pub fn new(window: Rect, scale: f32) -> Self { ... }
}

impl Overlay for FindDialog {
    fn render(&self) -> Node { /* Node tree */ }

    fn on_event(&mut self, ev: FrameworkEvent) -> OverlayResult {
        match ev {
            FrameworkEvent::Char(c)   => { self.input.state.insert_char(c); OverlayResult::Redraw }
            FrameworkEvent::Backspace => { self.input.state.handle_backspace(); OverlayResult::Redraw }
            FrameworkEvent::Key(Key::Escape) => OverlayResult::Close,
            _ => OverlayResult::Nothing,
        }
    }

    fn on_drag(&mut self, _x: f32, _y: f32) -> bool { false }
    fn end_drag(&mut self) {}
    fn contains(&self, x: f32, y: f32) -> bool { self.overlay.contains(x, y) }
    fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape {
        if self.input.rect.contains(x, y) { CursorShape::Text } else { CursorShape::Default }
    }
}
```

Open it from anywhere with one call:

```rust
self.logic.open_overlay_with(Box::new(FindDialog::new(window, scale)))
```

Zero other files change.

---

## Animations

Animations are plain data inside the component — no scheduler, no thread, no callback.

```rust
// src/animation.rs
pub struct Animated<T: Lerp> {
    from: T, to: T, started: Instant, duration: Duration, easing: Easing,
}

impl<T: Lerp + Copy> Animated<T> {
    pub fn value(&self) -> T { T::lerp(self.from, self.to, self.easing.apply(t)) }
    pub fn is_done(&self) -> bool { self.started.elapsed() >= self.duration }
    pub fn transition_to(&mut self, target: T, duration: Duration, easing: Easing) {
        self.from = self.value();   // animate from current, not original
        self.to = target;
        self.started = Instant::now();
        // ...
    }
}
```

Components expose `has_active_animation() -> bool`. `AppLogic` aggregates this.
The platform shell keeps vsync alive while any animation is running — zero CPU at idle.

---

## Theming

`Theme` (currently a plain color struct) evolves into a `StyleSheet` — a typed token map:

```rust
pub struct VisualTokens {
    pub background: Color, pub foreground: Color,
    pub border: Color,     pub border_width: f32,
    pub corner_radius: f32, pub font_size: f32,
}
```

Components never hardcode colors. They call `style.get(ComponentId::Button, state_flags)`.
A new theme = a new file filling the same tokens with different values. Zero component changes.

---

## File Map (current state)

```
src/
├── components/           ← one file = one component
│   ├── notes_picker.rs   — Overlay: search input + result list
│   ├── slash_menu.rs     — Overlay: command palette anchored to cursor
│   └── tab_rename.rs     — InlineWidget: tab title editor
│
├── primitives/           ← one file = one primitive widget
│   ├── scrollbar.rs
│   ├── text_input.rs
│   ├── list.rs
│   ├── button.rs
│   └── label.rs
│
├── layout/               ← pure geometry + traits, no GPU
│   ├── node.rs           — Node enum (the declarative UI tree)
│   ├── column.rs
│   └── row.rs
│
├── logic/
│   ├── mod.rs            — AppLogic; render() → Node tree
│   └── actions/          — keyboard/mouse handlers
│
├── renderer/
│   ├── node_renderer.rs  — single NodeRenderer walks Node tree → femtovg
│   └── text_content/     — layout helpers (flame lookup, cursor geometry)
│
├── ui/                   — Rect, WindowRect, UiTree (hit-testing + hover)
└── app/                  — OS event routing, clipboard, file I/O
```

---

## Invariants That Must Never Break

- `src/logic/` never imports `femtovg` — enforced by headless tests.
- `src/ui/types.rs` (`Rect`, `WindowRect`) has zero widget imports.
- Layout arithmetic exists only inside `Rect` methods and `Column`/`Row` constructors.
- `AppLogic::render()` is the only function that calls `.render(theme)` on overlays.
- `AppLogic` never imports a concrete overlay type — only `Box<dyn Overlay>`.
- `NodeRenderer` is the only file that calls femtovg draw primitives — no per-subsystem renderers.
- Components never hardcode colors — all visual values flow from `Theme`.
