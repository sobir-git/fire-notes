# fire-notes UI Framework — Design Specification

## Philosophy

Five rules:

1. **Local logic** — one file tells you everything about one widget. No widget reaches outside its own file for layout, state, or event handling.
2. **Rects flow down** — a parent subdivides its space and hands sub-rects to children. Children never reach upward.
3. **Token-efficient** — layout reads like a description, not arithmetic.
4. **Widgets are retained** — a widget owns its state AND its geometry. `relayout()` updates geometry without losing state. Geometry is not rebuilt on every event.
5. **Typed message bubbling** — internal events (scroll, selection, cursor movement) stay inside the widget. Only business-level events bubble up as typed `Msg` values.

---

## Module Structure

```
src/fw/                         ← Framework layer. Zero app-specific knowledge.
├── mod.rs
├── response.rs                 ← EventResponse<Msg> — universal event return type
├── layout.rs                   ← VStack / HStack (thin named wrappers over Rect::split_v/h)
└── widgets/
    ├── mod.rs
    ├── text_input.rs           ← Merged TextInput: state + geometry + events
    └── list.rs                 ← Merged List<T>: state + geometry + scrollbar events

src/ui/                         ← App-specific composed widgets. Uses fw, knows app types.
├── types.rs                    ← Rect, WindowRect, CursorShape, UiAction, etc.
├── tab_bar.rs                  ← TabBar (unchanged structure)
├── content_area.rs             ← ContentArea (unchanged)
├── notes_picker.rs             ← Self-contained: owns fw::TextInput + fw::List<NoteEntry>
├── scrollbar.rs                ← ScrollbarWidget (unchanged — used by ContentArea)
├── list_widget.rs              ← ListWidget<T> (state-only backing — used by fw::List internally)
├── list_view.rs                ← ListViewWidget (geometry-only backing — used by fw::List internally)
└── tree.rs                     ← UiTree: TabBar + ContentArea compositor

src/app/                        ← Platform shell. Routes messages to AppLogic.
└── notes_picker.rs             ← Thin: routes NotesPickerMsg to business logic (~40 lines)

src/logic/                      ← Pure logic. Has notes_picker: Option<NotesPicker> field.
```

---

## Core Abstractions

### `Rect` — unchanged, lives in `src/ui/types.rs`

All coordinate arithmetic. `cut_top/bottom/left/right`, `split_h/v`, `inset`, `centered_in`.

### `EventResponse<Msg>` — universal event return type

```rust
pub enum EventResponse<Msg = ()> {
    Pass,         // not this widget's concern — propagate upward
    Handled,      // consumed internally, no redraw
    Redraw,       // consumed internally, redraw needed
    Emit(Msg),    // produced a business message for the parent
}
```

`Pass` is the only non-consuming variant. Any other value means the widget consumed the event.  
`Emit` implies redraw (the parent will act on the message and redraw).

### `VStack` / `HStack` — named layout wrappers

Thin wrappers over `Rect::split_v` / `Rect::split_h`. Negative slot = fixed px × scale, positive = flex.

```rust
let v = VStack::new(overlay_rect.inset(padding), scale, &[-input_h, 1.0]);
search.relayout(v.slot(0), scale);
list.relayout(v.slot(1), scale);
```

---

## Widget Anatomy

Every framework widget follows this pattern:

```rust
pub struct MyWidget {
    // ── State (persists across frames) ─────────────────
    some_value: String,
    is_hovered: bool,

    // ── Geometry (updated via relayout) ────────────────
    pub rect: Rect,
    font_size: f32,
    scale: f32,
}

impl MyWidget {
    pub fn new(initial: String) -> Self { ... }

    /// Update geometry, preserve state. Called on window resize only.
    pub fn relayout(&mut self, rect: Rect, scale: f32) { ... }

    /// Handle pointer events. Returns typed outcome for the parent.
    pub fn on_pointer_down(&mut self, x: f32, y: f32) -> EventResponse<MyMsg> { ... }

    /// Cursor shape at (x, y). Called during hover.
    pub fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape { ... }
}

pub enum MyMsg { Confirmed(String), Cancelled }
```

**No trait required for basic widgets.** The `EventResponse<Msg>` type is the only contract.  
Widgets that want polymorphic dispatch can implement the optional `Widget` trait (future).

---

## Standard Widget Inventory

| Widget | File | Owns |
|---|---|---|
| `fw::TextInput` | `fw/widgets/text_input.rs` | Text state + editing + geometry + pointer/drag |
| `fw::List<T>` | `fw/widgets/list.rs` | Items + filter + selection + scroll + geometry + **scrollbar drag (fully internal)** |
| `VStack` / `HStack` | `fw/layout.rs` | Named layout containers (thin Rect wrappers) |
| `Rect` | `ui/types.rs` | Space primitive — all arithmetic |
| `WindowRect` | `ui/types.rs` | Compile-time root guard |
| `UiTree` | `ui/tree.rs` | Root compositor: TabBar + ContentArea |
| `TabBar` | `ui/tab_bar.rs` | Tab strip + window chrome |
| `ContentArea` | `ui/content_area.rs` | Text area + scrollbar (retained existing design) |
| `NotesPicker` | `ui/notes_picker.rs` | Self-contained: fw::TextInput + fw::List<NoteEntry> + all interaction |

---

## Creating a Custom Widget

No framework registration. No trait required. Just:

```rust
// ui/my_widget.rs
use crate::ui::types::Rect;
use crate::fw::EventResponse;

pub struct MyWidget {
    label: String,          // state
    pub rect: Rect,         // geometry
    pub is_hovered: bool,   // interaction state
}

pub enum MyWidgetMsg { Clicked }

impl MyWidget {
    pub fn new(label: String) -> Self {
        Self { label, rect: Rect::ZERO, is_hovered: false }
    }

    pub fn relayout(&mut self, rect: Rect, _scale: f32) {
        self.rect = rect;
    }

    pub fn on_pointer_down(&mut self, x: f32, y: f32) -> EventResponse<MyWidgetMsg> {
        if self.rect.contains(x, y) {
            EventResponse::Emit(MyWidgetMsg::Clicked)
        } else {
            EventResponse::Pass
        }
    }

    pub fn on_hover(&mut self, x: f32, y: f32) -> bool {
        let was = self.is_hovered;
        self.is_hovered = self.rect.contains(x, y);
        was != self.is_hovered
    }
}
```

Wire it in the parent:
```rust
match self.my_widget.on_pointer_down(x, y) {
    EventResponse::Emit(MyWidgetMsg::Clicked) => self.do_something(),
    _ => {}
}
```

That's the entire framework surface for custom widgets. Renderer reads `my_widget.rect` and
`my_widget.is_hovered` directly — no render trait, no draw calls in the widget.

---

## Focus Model

`Focus` is a lightweight discriminant — it identifies *what* has keyboard focus, not *where the state lives*.

```rust
pub enum Focus {
    Editor,
    TabRename(usize),            // state: AppLogic.rename_input: Option<(usize, TextInput)>
    NotesPicker,                 // state: AppLogic.notes_picker: Option<NotesPicker>
}
```

Widget state lives in dedicated `AppLogic` fields, not inside the `Focus` enum.  
This eliminates borrow-fighting when accessing multiple fields of the same widget.

Keyboard routing:
```
keyboard → Action → AppLogic::execute()
  ├── focus == NotesPicker → self.notes_picker?.handle_key(action) → Option<NotesPickerMsg>
  ├── focus == TabRename   → self.rename_input?.handle_key(action)
  └── focus == Editor      → tab editing logic
```

---

## NotesPicker: Before → After

### Before (split across two files, geometry rebuilt per-event)
- `ui/notes_picker.rs` — 258 lines: geometry only, takes `&mut TextInput` + `&mut ListWidget<T>` params
- `app/notes_picker.rs` — 194 lines: rebuilds `NotesPicker::new(w, h, scale, len)` on EVERY event call

### After (self-contained widget)
- `ui/notes_picker.rs` — ~180 lines: owns `fw::TextInput` + `fw::List<NoteEntry>`, all interaction methods
- `app/notes_picker.rs` — ~50 lines: routes `NotesPickerMsg` to business logic

**App routing after migration:**
```rust
if let Some(picker) = &mut self.logic.notes_picker {
    match picker.on_pointer_down(x, y, char_width) {
        PickerPointerResult::Confirmed  => { ... open note ... }
        PickerPointerResult::Cancel     => { self.logic.notes_picker = None; ... }
        _                               => {}
    }
}
```

**Scrollbar drag is fully internal to `fw::List`.** `ScrollbarDragStart` is never surfaced to
the caller — `List.on_pointer_down` records the drag offset internally, and the app calls
`picker.continue_scrollbar_drag(y)` + `picker.is_scrollbar_dragging()` on every mouse-move
without any extra `MouseInteraction` variant. `picker.end_scrollbar_drag()` is called on mouse-up.
No `PickerScrollbarDrag` variant exists in `MouseInteraction`.

---

## Render Path

Renderer reads widget geometry directly — no marshal step, no extra structs.

```
App::render()
  → build UiTree (tab_bar + content_area layout)
  → pass to Renderer::render(&RenderFrame)

RenderFrame contains:
  - &UiTree                   ← tab bar + content area geometry
  - &NotesPicker (Option)     ← self-contained picker (replaces separate input+list+layout)
  - misc interaction flags    ← cursor_visible, hovered_*, dragging_scrollbar
```

The picker renderer receives one `&NotesPicker` and reads `picker.search.*` and `picker.list.*` directly.

---

## Migration Phases

| Phase | What changes | Lines removed |
|---|---|---|
| **1** (current) | `fw/` module created; `NotesPicker` self-contained; `Focus::NotesPicker` payload-free | ~200 |
| **2** | `Focus::TabRename` payload-free; `rename_input: Option<(usize, fw::TextInput)>` moves to `AppLogic` | ~50 |
| **3** | `UiState` hover fields collapsed; `TabBar` + `ContentArea` own their hover state | ~60 |
| **4** | `ContentArea` uses `fw::Scrollbar` (merged geometry + drag state) | ~40 |

### Phase 2 detail — TabRename

```rust
// Focus becomes:
pub enum Focus {
    Editor,
    TabRename,      // was: TabRename { tab_index: usize, input: TextInput }
    NotesPicker,
}

// AppLogic gains:
pub rename_input: Option<(usize, fw::TextInput)>  // (tab_index, input)
```

All callers that pattern-match `Focus::TabRename { tab_index, input }` are updated to read
`self.rename_input` instead. Same pattern as the NotesPicker migration.

### Phase 3 detail — Hover ownership

Today `UiState` has 6 separate hover booleans manually synchronized in `handle_mouse_move`:
```rust
hovered_tab_index, hovered_plus, hovered_scrollbar,
hovered_window_minimize, hovered_window_maximize, hovered_window_close
```

After Phase 3, each widget owns its own hover state:
- `TabBar.hovered_tab_index: Option<usize>`, `TabBar.hovered_plus: bool`, window chrome bools
- `ContentArea.scrollbar_hovered: bool`
- `handle_mouse_move` calls `ui_tree.on_hover(x, y)` → returns `bool` (changed?)
- `RenderFrame` no longer carries individual hover flags — renderer reads from widget fields directly

This removes ~60 lines of manual hover sync and makes hover state impossible to get out of sync.

Each phase is independently shippable and non-breaking to the outside-facing API.

---

## What Stays Unchanged

- `Rect` layout primitives — `cut_*`, `split_h/v`, `inset`, `centered_in` — excellent, untouched
- `Layout` trait — `fn layout(rect, scale) -> Self` — kept for purely-geometric structs
- `WindowRect` compile guard — kept
- `AppLogic` purity — no GPU/OS deps, headless testable
- `Action` enum + `App::execute()` keyboard dispatch path
- Renderer layer — reads geometry, never computes layout
