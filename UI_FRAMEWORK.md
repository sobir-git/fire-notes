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

## Event Return Type

```rust
pub enum EventResponse<Msg = ()> {
    Pass,        // not this widget's concern — propagate upward
    Handled,     // consumed, no redraw needed
    Redraw,      // consumed, redraw needed
    Emit(Msg),   // produced a business message for the parent (implies redraw)
}
```

`Pass` is the only non-consuming variant. Widgets that don't need polymorphic dispatch
skip this type and return plain enums — whatever is clearest for the caller.

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
// A search dialog — ~12 lines, zero manual geometry
pub struct SearchDialog {
    input:  TextInput,
    list:   List<Item>,
}

impl SearchDialog {
    pub fn new(rect: Rect, scale: f32, items: Vec<Item>) -> Self {
        let col = Column::new(rect, scale, &[-40.0, 1.0]);  // input 40px, list fills rest
        Self {
            input: TextInput::new(col.slot(0), scale),
            list:  List::new(col.slot(1), scale, items),
        }
    }
}
```

No manual geometry. The combinator computes all rects.

---

## Adding a New Component

One file. No registration. No macro. No codegen.

```rust
// src/components/my_widget.rs
use crate::ui::Rect;

pub struct MyWidget { label: String, rect: Rect, hovered: bool }
pub struct MyWidgetView { rect: Rect, label: String, hovered: bool }
pub enum MyWidgetMsg { Clicked }

impl MyWidget {
    pub fn new(rect: Rect, label: impl Into<String>) -> Self {
        Self { label: label.into(), rect, hovered: false }
    }
    pub fn on_pointer_down(&mut self, x: f32, y: f32) -> Option<MyWidgetMsg> {
        if self.rect.contains(x, y) { Some(MyWidgetMsg::Clicked) } else { None }
    }
    pub fn on_hover(&mut self, x: f32, y: f32) -> bool {
        let prev = self.hovered;
        self.hovered = self.rect.contains(x, y);
        prev != self.hovered
    }
    pub fn snapshot(&self) -> MyWidgetView {
        MyWidgetView { rect: self.rect, label: self.label.clone(), hovered: self.hovered }
    }
}
```

Then in `AppLogic`: add `my_widget: MyWidget` and one line in `render_frame()`.
That is the entire integration cost.

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
