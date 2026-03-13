# fire-notes UI Framework

## Philosophy

Five rules:

1. **One file per component** — one file tells you everything about one widget. No widget reaches outside its own file for layout, state, or event handling.
2. **Rects flow down** — a parent subdivides its space and hands sub-rects to children. Children never reach upward for geometry.
3. **The snapshot is the firewall** — the only boundary between logic and GPU is a plain data struct. Components produce it; the renderer consumes it. Nothing else crosses.
4. **Widgets are retained** — a widget owns its state AND its geometry. `relayout()` updates geometry without losing state. Geometry is never rebuilt on every event.
5. **`AppLogic` is always headless** — `src/logic/` never imports `femtovg` or any platform dep. `cargo test` runs the full logic suite with zero GPU.

---

## Layer Map

```
┌─────────────────────────────────────────────────────────┐
│  App shell  (src/app/)                                  │
│  Routes OS events → component methods                   │
│  Calls renderer.render(frame)                           │
├─────────────────────────────────────────────────────────┤
│  AppLogic  (src/logic/)                                 │
│  Owns component instances as fields.                    │
│  render_frame() delegates to component.snapshot()       │
│  Zero GPU imports. Fully headless-testable.             │
├─────────────────────────────────────────────────────────┤
│  Components  (src/components/)                          │
│  One file each. Owns state + layout + hit-test.         │
│  Exposes snapshot() → pure ViewData struct.             │
│  #[cfg(feature="gpu")] impl ViewData { fn draw() }      │
├─────────────────────────────────────────────────────────┤
│  Primitives  (src/primitives/)                          │
│  Button, TextInput, Label, Scrollbar, List              │
│  One file each. Same contract as components.            │
├─────────────────────────────────────────────────────────┤
│  Layout  (src/layout/)                                  │
│  Column, Row — pure geometry, no GPU.                   │
│  Compose primitives. Auto-dispatch hit-testing.         │
├─────────────────────────────────────────────────────────┤
│  Rect  (src/ui/types.rs)                                │
│  The only place coordinate arithmetic lives.            │
└─────────────────────────────────────────────────────────┘
```

---

## Component Contract

Every component follows this pattern (convention, not a Rust trait):

```rust
// src/components/my_component.rs

// 1. State
pub struct MyComponent {
    value:    String,
    hovered:  bool,
    dirty:    bool,
}

// 2. Logic (no GPU)
impl MyComponent {
    pub fn on_pointer_down(&mut self, x: f32, y: f32) -> MyEvent { ... }
    pub fn on_hover(&mut self, x: f32, y: f32) -> bool { ... }   // returns changed?
    pub fn snapshot(&self, rect: Rect, scale: f32) -> MyView { ... }
}

// 3. Draw (GPU, feature-gated — future)
#[cfg(feature = "gpu")]
impl MyView {
    pub fn draw(&self, canvas: &mut Canvas<OpenGl>, theme: &Theme) { ... }
}
```

`MyView` is a plain data struct — `Clone`, `Debug`, no GPU types. Tests call
`.snapshot()` and assert on the struct directly.

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

## Typed Component Events

Components emit domain events — not raw booleans, not `AppResult`. Callers just match:

```rust
// PickerEvent defined in components/notes_picker.rs
pub enum PickerEvent {
    None,
    Redraw,
    Confirmed(NoteEntry),   // carries the selected item — no extra lookups
    Cancelled,
}

// Entire click handler in app/notes_picker.rs:
match picker.on_pointer_down(x, y, char_width) {
    PickerEvent::None      => AppResult::Ok,
    PickerEvent::Redraw    => AppResult::Redraw,
    PickerEvent::Cancelled => self.logic.cancel_notes_picker(),
    PickerEvent::Confirmed(entry) => self.logic.open_or_switch_to(entry.path),
}
```

The caller has zero knowledge of which child was hit. All routing lives in the component.

---

## Focus Model

`Focus` is a lightweight discriminant — it identifies *what* has keyboard focus, not *where the state lives*:

```rust
pub enum Focus {
    Editor,
    TabRename,    // state lives in: AppLogic.rename_input: Option<(usize, FwTextInput)>
    NotesPicker,  // state lives in: AppLogic.notes_picker: Option<NotesPicker>
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

## Adding a New Component

One file. No registration. No macro. No codegen.

```rust
// src/components/my_dialog.rs
use crate::layout::Column;
use crate::primitives::{TextInput, Button};
use crate::ui::Rect;

pub enum DialogEvent { Submitted(String), Cancelled }

pub struct MyDialog {
    input:  TextInput,
    ok:     Button,
    cancel: Button,
    layout: Column,
}

impl MyDialog {
    pub fn new(rect: Rect, scale: f32) -> Self {
        let col = Column::new(rect, scale, &[-40.0, -36.0]);
        let row = crate::layout::Row::new(col.slot(1), scale, &[1.0, 1.0]);
        Self {
            input:  TextInput::new_empty().with_rect(col.slot(0), scale),
            ok:     Button::new(row.slot(0), scale, "OK"),
            cancel: Button::new(row.slot(1), scale, "Cancel"),
            layout: col,
        }
    }
    pub fn on_pointer_down(&mut self, x: f32, y: f32) -> DialogEvent {
        match self.layout.hit_slot(x, y) {
            Some(0) => { self.input.on_pointer_down_with(x, y, self.input.char_width); DialogEvent::None }
            Some(1) => /* row dispatch */ DialogEvent::None,
            _       => DialogEvent::Cancelled,
        }
    }
    pub fn snapshot(&self) -> MyDialogView { /* read fields off children */ }
}
```

Then in `AppLogic`: add one field, one line in `render_frame()`. No other changes.

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
│   ├── notes_picker.rs   — owns search input + list + geometry baking
│   ├── tab_bar.rs        — snapshot baking for the tab bar
│   └── text_editor.rs    — snapshot baking for the content area
│
├── primitives/           ← one file = one primitive widget
│   ├── scrollbar.rs
│   ├── text_input.rs
│   ├── list.rs
│   ├── button.rs
│   └── label.rs
│
├── layout/               ← pure geometry, no GPU
│   ├── column.rs
│   └── row.rs
│
├── logic/
│   ├── mod.rs            — AppLogic owns component fields; render_frame() delegates
│   └── actions/          — keyboard/mouse handlers; call component methods
│
├── renderer/             — walks RenderFrame, calls draw; zero layout logic
├── render_frame.rs       — pure data snapshot; fields are component ViewData structs
├── ui/                   — Rect, WindowRect, UiTree, TabBar, ContentArea geometry
└── app/                  — OS event routing, clipboard, file I/O
```

---

## Invariants That Must Never Break

- `src/logic/` never imports `femtovg` — enforced by headless tests.
- `src/ui/types.rs` (`Rect`, `WindowRect`) has zero widget imports.
- Layout arithmetic exists only inside `Rect` methods and `Column`/`Row` constructors.
- `render_frame()` is the only function that calls `.snapshot()`.
- Components never hardcode colors, durations, or easings — all visual values come from `StyleSheet` and animation presets.
