# fire-notes UI Architecture

## Goal

One file per UI component. No compromises on:
- **Headless testability** — `AppLogic` never imports `femtovg`.
- **Performance** — no unnecessary re-layout or re-render.
- **Composability** — a form is a declarative composition of primitives, under 20 lines.
- **Developer experience** — adding a new component means adding one file.

---

## The Core Insight

The separation between "logic" and "GPU draw" does **not** require spreading a component across multiple files or layers. It requires a single boundary: the **view snapshot**.

```
Component (state + layout + hit-test)
    │
    │  .snapshot(rect, scale) → ViewData   ← pure data, no GPU, testable
    │
    └── ViewData.draw(canvas, theme)        ← GPU, feature-gated
```

Each component owns all three phases in one file. The snapshot is the firewall.

---

## Layer Map

```
┌─────────────────────────────────────────────────────────┐
│  App shell  (src/app/)                                  │
│  Routes OS events → component.handle_*(...)             │
│  Calls renderer.render(frame)                           │
├─────────────────────────────────────────────────────────┤
│  AppLogic  (src/logic/)                                 │
│  Owns component instances.                              │
│  render_frame() delegates to component.snapshot()       │
│  Zero GPU imports. Fully headless-testable.             │
├─────────────────────────────────────────────────────────┤
│  Components  (src/components/)          ← NEW           │
│  One file each. Owns state + layout + hit-test.         │
│  Exposes .snapshot() → pure ViewData.                   │
│  #[cfg(feature="gpu")] impl ViewData { fn draw() }      │
├─────────────────────────────────────────────────────────┤
│  Primitives  (src/primitives/)          ← NEW           │
│  Button, TextInput, Label, Scrollbar, List              │
│  One file each. Same contract as components.            │
├─────────────────────────────────────────────────────────┤
│  Layout  (src/layout/)                  ← NEW           │
│  Column, Row, Stack — pure geometry, no GPU.            │
│  Compose primitives. Auto-dispatch hit-testing.         │
├─────────────────────────────────────────────────────────┤
│  Rect  (src/ui/types.rs)                                │
│  The only place coordinate arithmetic lives.            │
└─────────────────────────────────────────────────────────┘
```

---

## Component Contract

Every component implements this pattern (not a Rust trait — just a convention):

```rust
// src/components/notes_picker.rs

// ── 1. State ──────────────────────────────────────────────────────────────
pub struct NotesPicker {
    entries: Vec<NoteEntry>,
    query:   String,
    list:    ListState,       // selection, scroll, filter
    input:   TextInputState,  // cursor, selection_anchor, scroll_x
}

// ── 2. Logic (no GPU) ─────────────────────────────────────────────────────
impl NotesPicker {
    pub fn on_click(&mut self, x: f32, y: f32, char_w: f32) -> PickerEvent { ... }
    pub fn on_key(&mut self, key: Key) -> PickerEvent { ... }
    pub fn on_hover(&mut self, x: f32, y: f32) { ... }
    pub fn on_scroll(&mut self, delta: isize) { ... }
    pub fn snapshot(&self, rect: Rect, scale: f32) -> NotesPickerView { ... }
}

// ── 3. Draw (GPU, feature-gated) ──────────────────────────────────────────
#[cfg(feature = "gpu")]
impl NotesPickerView {
    pub fn draw(&self, canvas: &mut Canvas<OpenGl>, theme: &Theme) { ... }
}
```

`NotesPickerView` is a plain data struct — `Clone`, `Debug`, no GPU types. Tests call
`.snapshot()` and assert on the view data directly. The `draw` method compiles only when
the `gpu` feature is active (which it is in the real app, not in `cargo test`).

---

## Composability: Layout Primitives

A `Column` is a thin wrapper over `Rect::split_v`. It owns slot rects and auto-dispatches
pointer events to whichever child's rect contains the point:

```rust
// src/layout/column.rs  (~60 lines total)

pub struct Column<T> {
    slots: Vec<(Rect, T)>,
}

impl<T: Widget> Column<T> {
    pub fn new(rect: Rect, scale: f32, weights: &[f32], children: Vec<T>) -> Self { ... }

    pub fn on_pointer(&mut self, x: f32, y: f32, ev: PointerEv) -> bool {
        for (rect, child) in &mut self.slots {
            if rect.contains(x, y) { return child.on_pointer(x, y, ev); }
        }
        false
    }
}
```

The `Widget` bound is a minimal local trait:

```rust
pub trait Widget {
    fn on_pointer(&mut self, x: f32, y: f32, ev: PointerEv) -> bool;
    fn snapshot_node(&self) -> Box<dyn ViewNode>;
}

#[cfg(feature = "gpu")]
pub trait ViewNode {
    fn draw(&self, canvas: &mut Canvas<OpenGl>, theme: &Theme);
}
```

A **form** then becomes fully declarative with zero custom hit-test code:

```rust
// A rename dialog — ~15 lines including blank lines
pub struct RenameDialog {
    label:  Label,
    input:  TextInput,
    ok:     Button,
    cancel: Button,
}

impl RenameDialog {
    pub fn new(rect: Rect, scale: f32, current_name: &str) -> Self {
        let [label_r, input_r, btn_row] = rect.split_v_3([-24.0, -40.0, -36.0], scale);
        let [ok_r, cancel_r] = btn_row.split_h_2([1.0, 1.0], scale);
        Self {
            label:  Label::new(label_r, "Rename tab"),
            input:  TextInput::new(input_r, scale, current_name),
            ok:     Button::new(ok_r,     scale, "OK"),
            cancel: Button::new(cancel_r, scale, "Cancel"),
        }
    }
}
```

No manual geometry. No manual hit-testing. The `Column`/`Row` dispatchers handle it.

---

## Performance: Dirty Flags + Snapshot Caching

Unnecessary re-layout and re-render are prevented at two levels:

**1. Component-level dirty flag**

Every component tracks whether its state changed since the last snapshot:

```rust
pub struct NotesPicker {
    // ...state...
    dirty: bool,   // set true on any mutation
}

impl NotesPicker {
    pub fn snapshot(&mut self, rect: Rect, scale: f32) -> Option<NotesPickerView> {
        if !self.dirty { return None; }   // renderer reuses last frame's view
        self.dirty = false;
        Some(self.bake(rect, scale))
    }
}
```

**2. Frame-level change detection in `AppLogic::render_frame()`**

`render_frame()` collects `Option<ViewData>` from each component. The renderer keeps the
previous frame's `RenderFrame` and only redraws components that produced `Some(new_view)`:

```rust
// renderer/mod.rs
pub fn render(&mut self, frame: &RenderFrame) {
    if let Some(v) = &frame.tab_bar      { v.draw(&mut self.canvas, &self.theme); }
    if let Some(v) = &frame.text_editor  { v.draw(&mut self.canvas, &self.theme); }
    if let Some(v) = &frame.notes_picker { v.draw(&mut self.canvas, &self.theme); }
}
```

When nothing changed the renderer flushes an empty frame in microseconds.

---

## `AppLogic::render_frame()` — becomes trivial

With widget-owned snapshots, the central baking function shrinks to a delegation:

```rust
pub fn render_frame(&mut self) -> RenderFrame {
    let window = Rect { x: 0.0, y: 0.0, width: self.width, height: self.height };
    RenderFrame {
        tab_bar:      self.tab_bar.snapshot(window, self.scale),
        text_editor:  self.text_editor.snapshot(window, self.scale),
        notes_picker: self.notes_picker.as_mut()
                          .and_then(|p| p.snapshot(window, self.scale)),
    }
}
```

No geometry extraction. No baking loops. Each component is the authority on its own
geometry.

---

## `Cargo.toml` Feature Gate

```toml
[features]
default = ["gpu"]
gpu     = ["dep:femtovg"]

[dependencies]
femtovg = { version = "...", optional = true }
```

Tests run with:
```
cargo test --no-default-features
```

All component logic, layout, hit-testing, and snapshot assertions compile and run with
zero GPU dependency. The `draw()` methods simply do not exist in that build.

---

## File Map (target state)

```
src/
├── layout/
│   ├── mod.rs       — re-exports
│   ├── column.rs    — Column<T>, auto hit-dispatch, ~60 lines
│   └── row.rs       — Row<T>, ~60 lines
│
├── primitives/
│   ├── mod.rs
│   ├── button.rs    — Button state + snapshot + draw, ~80 lines
│   ├── label.rs     — Label state + snapshot + draw, ~40 lines
│   ├── text_input.rs— TextInput state + snapshot + draw, ~120 lines
│   ├── scrollbar.rs — Scrollbar state + snapshot + draw, ~80 lines
│   └── list.rs      — List<T> state + snapshot + draw, ~120 lines
│
├── components/
│   ├── mod.rs
│   ├── tab_bar.rs      — ~180 lines
│   ├── text_editor.rs  — ~200 lines
│   └── notes_picker.rs — ~150 lines  (currently spread across 3 files + PickerLayout)
│
├── logic/
│   ├── mod.rs       — AppLogic owns component instances, render_frame() delegates
│   └── actions/     — keyboard/mouse action handlers, call component methods
│
├── renderer/
│   └── mod.rs       — walks RenderFrame, calls view.draw(); no layout logic
│
├── render_frame.rs  — RenderFrame struct; fields are Option<ComponentView>
├── ui/              — Rect, WindowRect, coordinate arithmetic only
└── app/             — OS event routing, clipboard, file I/O
```

---

## Migration Order

Migrate one component at a time. Each step leaves the build passing.

1. **`primitives/scrollbar.rs`** — simplest, already mostly right in `fw::Scrollbar`
2. **`primitives/text_input.rs`** — consolidates `ui::TextInput` + `fw::TextInput` + renderer draw
3. **`primitives/list.rs`** — consolidates `ui::ListWidget` + `ui::ListViewWidget` + `fw::List`
4. **`primitives/button.rs`** + **`primitives/label.rs`** — new, ~40 lines each
5. **`layout/column.rs`** + **`layout/row.rs`** — layout combinators with auto hit-dispatch
6. **`components/notes_picker.rs`** — first full component; replaces the current 3-file spread
7. **`components/tab_bar.rs`** — second full component
8. **`components/text_editor.rs`** — third full component
9. **Delete** `src/fw/`, `src/ui/` (except `Rect`/`WindowRect`), `src/renderer/tab_bar/`,
   `src/renderer/text_content/`, `src/renderer/notes_picker.rs`

At step 9 the entire old layer hierarchy is gone.

---

## Custom Components

Adding a new component requires exactly one file. The framework imposes no registration,
no macro, no codegen. The full recipe:

```rust
// src/components/my_widget.rs

// 1. State
pub struct MyWidget { value: f32, hovered: bool, dirty: bool }

// 2. Logic
impl MyWidget {
    pub fn on_pointer(&mut self, x: f32, y: f32, ev: PointerEv) -> MyEvent {
        self.hovered = self.rect.contains(x, y);
        self.dirty = true;
        MyEvent::None
    }
    pub fn snapshot(&mut self, rect: Rect, scale: f32) -> Option<MyView> {
        if !self.dirty { return None; }
        self.dirty = false;
        Some(MyView { rect, value: self.value, hovered: self.hovered, scale })
    }
}

// 3. Draw (GPU-gated)
#[cfg(feature = "gpu")]
impl MyView {
    pub fn draw(&self, canvas: &mut Canvas<OpenGl>, style: &StyleSheet) { ... }
}
```

Then in `AppLogic`: add a `my_widget: MyWidget` field and one line in `render_frame()`.
That is the entire integration cost.

---

## Custom Layouts

A custom layout is just a struct that owns a set of `(Rect, T)` slots and dispatches
pointer events. It implements the same `Widget` trait as primitives. Two built-in layouts
(`Column`, `Row`) cover most cases. A custom layout adds one file:

```rust
// src/layout/tab_strip.rs  — horizontal scrollable strip of equal-width tabs
pub struct TabStrip<T> {
    slots:    Vec<(Rect, T)>,
    scroll_x: f32,
}

impl<T: Widget> TabStrip<T> {
    pub fn new(rect: Rect, scale: f32, scroll_x: f32, children: Vec<T>) -> Self { ... }

    pub fn on_pointer(&mut self, x: f32, y: f32, ev: PointerEv) -> bool {
        let shifted_x = x + self.scroll_x;
        for (rect, child) in &mut self.slots {
            if rect.contains(shifted_x, y) { return child.on_pointer(x, y, ev); }
        }
        false
    }
}
```

No base class. No registration. It composes with every other primitive automatically
because it implements `Widget`.

---

## Animations

### Design principles

- **Data-driven, not callback-driven.** An animation is a value that changes over time,
  stored inside the component as plain data. There is no animation scheduler, no timeline
  object, no separate thread.
- **Pull, not push.** The renderer pulls the current animation value from the snapshot on
  every vsync frame. Components never push to the renderer.
- **Composable.** `Animated<T>` is a generic wrapper that adds interpolation to any value.
  It composes with theming, with state machines, with anything.

### `Animated<T>` — the animation primitive

```rust
// src/animation.rs  (~60 lines, no GPU imports)

pub struct Animated<T: Lerp> {
    from:     T,
    to:       T,
    started:  Instant,
    duration: Duration,
    easing:   Easing,
}

impl<T: Lerp + Copy> Animated<T> {
    pub fn value(&self) -> T {
        let t = self.started.elapsed().as_secs_f32() / self.duration.as_secs_f32();
        let t = self.easing.apply(t.min(1.0));
        T::lerp(self.from, self.to, t)
    }
    pub fn is_done(&self) -> bool {
        self.started.elapsed() >= self.duration
    }
    pub fn transition_to(&mut self, target: T, duration: Duration, easing: Easing) {
        self.from     = self.value(); // start from current position, not original
        self.to       = target;
        self.started  = Instant::now();
        self.duration = duration;
        self.easing   = easing;
    }
}
```

`Lerp` is a one-method trait implemented for `f32`, `Color`, `Rect`, and any other
interpolatable type.

### Using animations inside a component

```rust
pub struct Button {
    label:      String,
    bg_color:   Animated<Color>,   // animates on hover in/out
    scale_anim: Animated<f32>,     // animates on press
    hovered:    bool,
    dirty:      bool,
}

impl Button {
    pub fn on_hover_enter(&mut self) {
        self.hovered = true;
        self.bg_color.transition_to(HOVER_COLOR, Duration::from_millis(120), Easing::EaseOut);
        self.dirty = true;
    }
    pub fn on_hover_exit(&mut self) {
        self.hovered = false;
        self.bg_color.transition_to(IDLE_COLOR, Duration::from_millis(80), Easing::EaseIn);
        self.dirty = true;
    }
    pub fn has_active_animation(&self) -> bool {
        !self.bg_color.is_done() || !self.scale_anim.is_done()
    }
}
```

### The render loop

`AppLogic` exposes:

```rust
pub fn has_active_animations(&self) -> bool {
    self.tab_bar.has_active_animation()
    || self.text_editor.has_active_animation()
    || self.notes_picker.as_ref().map_or(false, |p| p.has_active_animation())
    // ...
}
```

The platform shell keeps the vsync loop alive while this returns `true`:

```rust
// app/mod.rs
fn on_vsync(&mut self) {
    if self.logic.has_active_animations() || self.needs_redraw {
        let frame = self.logic.render_frame();
        self.renderer.render(&frame);
        self.needs_redraw = false;
    }
}
```

When all animations are done and no input has arrived, the vsync callback is a no-op.
Zero CPU usage at idle.

### Reusable animation presets

```rust
// src/animation/presets.rs
pub fn fade_in()       -> AnimParams { AnimParams { duration: ms(150), easing: Easing::EaseOut } }
pub fn fade_out()      -> AnimParams { AnimParams { duration: ms(100), easing: Easing::EaseIn  } }
pub fn spring_in()     -> AnimParams { AnimParams { duration: ms(200), easing: Easing::Spring  } }
pub fn slide_up()      -> AnimParams { AnimParams { duration: ms(180), easing: Easing::EaseOutCubic } }
```

A component that wants a standard hover animation calls `fade_in()` — it never hardcodes
durations or easings.

---

## Theming and Styling

### `StyleSheet` — typed token lookup

`Theme` (current: a plain struct of color tuples) evolves into a `StyleSheet` — a map
from `(ComponentId, StateFlags) → VisualTokens`. Visual tokens are typed:

```rust
pub struct VisualTokens {
    pub background:    Color,
    pub foreground:    Color,
    pub border:        Color,
    pub border_width:  f32,
    pub corner_radius: f32,
    pub font_size:     f32,       // relative to scale; StyleSheet multiplies by scale
    pub shadow_blur:   f32,
}
```

Components never hardcode a color. They call:

```rust
let tokens = style.get(ComponentId::Button, state_flags);
canvas.fill_path(&path, &Paint::color(tokens.background));
```

### State flags

```rust
bitflags! {
    pub struct StateFlags: u8 {
        const HOVERED  = 0b0001;
        const PRESSED  = 0b0010;
        const FOCUSED  = 0b0100;
        const DISABLED = 0b1000;
    }
}
```

A button that is hovered + focused looks up `StateFlags::HOVERED | StateFlags::FOCUSED`.
The `StyleSheet` resolves the most specific match (both flags), then falls back to
single-flag, then to the base style.

### Defining a theme

```rust
// src/theme/dark.rs
pub fn dark() -> StyleSheet {
    let mut s = StyleSheet::new();
    s.set(ComponentId::Button, StateFlags::empty(),  VisualTokens { background: rgb(40,40,40),  .. });
    s.set(ComponentId::Button, StateFlags::HOVERED,  VisualTokens { background: rgb(60,60,60),  .. });
    s.set(ComponentId::Button, StateFlags::PRESSED,  VisualTokens { background: rgb(30,100,200), .. });
    s.set(ComponentId::TabBar, StateFlags::empty(),  VisualTokens { background: rgb(25,25,25),  .. });
    // ...
    s
}
```

Adding a new theme is adding a new file that fills the same tokens with different values.
No component changes.

### Animated theming

Because tokens are just values, they compose directly with `Animated<T>`:

```rust
// smooth theme transition on toggle
pub fn switch_theme(&mut self, new_sheet: StyleSheet) {
    self.bg_color.transition_to(
        new_sheet.get(ComponentId::Button, StateFlags::empty()).background,
        Duration::from_millis(300),
        Easing::EaseInOut,
    );
    self.active_sheet = new_sheet;
}
```

Every component that holds `Animated<Color>` fields smoothly transitions when the theme
changes — with zero extra code per component.

---

## Invariants That Must Never Break

- `src/logic/` never imports `femtovg` (enforced: `cargo test --no-default-features` passes).
- `src/ui/types.rs` (`Rect`, `WindowRect`) has zero widget imports.
- Layout arithmetic (`+`, `*`, `/` on coordinates) exists only inside `Rect` methods and
  `Column`/`Row` constructors — nowhere else.
- Every component method that mutates state sets `self.dirty = true`.
- Every component that has active animations returns `true` from `has_active_animation()`.
- `render_frame()` is the only function that calls `.snapshot()`.
- No component hardcodes a color, duration, or easing — all visual values come from
  `StyleSheet` and animation presets.

