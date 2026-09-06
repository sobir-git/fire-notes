# fire-notes UI Framework

## The Prime Directive

> **We are not building a good app. We are perfecting the framework that builds the app.**
>
> The app is a proving ground. Every component written reveals a gap in the framework.
> When a gap is found, we do not paper over it with a local fix — we fix the framework.
> We do not make pragmatic patches, transitional compromises, or "minimal correct fixes".
> We identify the ideal architecture and implement it **radically and completely**.

---

## Status: The Framework Is Shipped

The Element tree framework is **implemented and live**. Both `NotesPicker` and `SlashMenu`
are rewritten as pure `Widget::build()` components. `OverlayCtx` is deleted.

**Before** (OverlayCtx era):
- `NotesPicker`: 5 struct fields (including `window`, `scale`, `ctx`) · `render()` · `on_event()` · ~110 lines
- `SlashMenu`: same pattern, ~118 lines
- `OverlayCtx`: ~250 lines of child-registration and manual dispatch glue

**After** (framework era):
- `NotesPicker`: 4 pure state fields · `build()` · `update()` · zero `on_event` · zero `OverlayCtx`
- `SlashMenu`: same pattern
- `OverlayCtx`: **deleted** (file removed from codebase)

This document describes the shipped framework architecture and the history of how we got here.

---

## Philosophy

1. **One file per component** — one file tells you everything about one widget. No widget reaches outside its own file for layout, state, or event handling.
2. **Rects flow down** — a parent subdivides its space and hands sub-rects to children. Children never reach upward for geometry.
3. **The Node tree is the firewall** — the only boundary between logic and GPU is a declarative `Node` tree. `AppLogic::render()` produces it; `NodeRenderer` consumes it. No intermediate snapshot structs.
4. **State is retained, geometry is computed** — a component owns its mutable state. Geometry is computed from window/scale on demand; it is never stored in the component.
5. **`AppLogic` is always headless** — `src/logic/` never imports `femtovg` or any platform dep. `cargo test` runs the full logic suite with zero GPU.

---

## Layer Map (Current)

```
┌─────────────────────────────────────────────────────────┐
│  Platform  (src/platform/)                              │
│  winit OS events → typed KeyEvent / MouseEvent          │
├─────────────────────────────────────────────────────────┤
│  App shell  (src/app/)                                  │
│  Routes events → AppLogic; owns clipboard, file I/O     │
├─────────────────────────────────────────────────────────┤
│  AppLogic  (src/logic/)                                 │
│  All state: tabs, focus, ActiveOverlay, ActiveInline    │
│  render() → Node tree (pure data, no GPU)               │
│  Zero GPU imports. Fully headless-testable.             │
├─────────────────────────────────────────────────────────┤
│  Framework  (src/framework/)                            │
│  Element tree, layout engine, event dispatcher          │
│  Widget trait: build() + update(Msg)                    │
├─────────────────────────────────────────────────────────┤
│  Components  (src/components/)                          │
│  Widget::build() → Element<Msg>; Widget::update(Msg)    │
│  NotesPicker, SlashMenu — zero on_event, zero geometry  │
├─────────────────────────────────────────────────────────┤
│  Primitives  (src/primitives/)                          │
│  TextInput, List<T>, Scrollbar — state + render + events│
├─────────────────────────────────────────────────────────┤
│  Layout  (src/layout/)                                  │
│  Column, Row, overlay_panel, floating_rect              │
├─────────────────────────────────────────────────────────┤
│  NodeRenderer  (src/renderer/)                          │
│  Node tree → femtovg. The ONLY GPU caller.              │
└─────────────────────────────────────────────────────────┘
```

---

## Current Event System

`AppLogic` holds the active overlay as a concrete enum:

```rust
pub enum ActiveOverlay { NotesPicker(NotesPicker), SlashMenu(SlashMenu) }
pub enum ActiveInline  { TabRename(TabRename) }
```

All platform events flow through a single typed enum before reaching components:

```rust
pub enum OverlayEvent<'a> {
    Up, Down, Confirm, Cancel,
    Text(TextInputEvent<'a>),
    Paste(String),
    PointerDown { x: f32, y: f32 },
    PointerDoubleClick { x: f32, y: f32 },
    PointerTripleClick { x: f32, y: f32 },
    PointerDrag { x: f32 },
    PointerMove { x: f32, y: f32 },
    Scroll { lines: isize },
}
```

`AppLogic::dispatch_overlay(ev)` calls `ActiveOverlay::on_event(ev)` which
delegates to the concrete component.

---

## The Framework (`src/framework/`)

### The Widget contract

Every overlay component implements one trait:

```rust
pub trait Widget {
    type Msg;
    fn build(&self) -> Element<Self::Msg>;  // declare UI — pure, no mutation
    fn update(&mut self, msg: Self::Msg);   // apply a message — the only mutation site
}
```

No `render()`. No `on_event()`. No stored `window` or `scale`.
The framework calls `build()`, runs layout, renders, dispatches events,
calls `update()` with any produced messages. The component only knows about
its own state and what it wants to show.

---

### `NotesPicker` — the proof

```rust
// Complete component state — zero geometry fields
pub struct NotesPicker {
    search:   TextInput,
    entries:  Vec<NoteEntry>,
    filtered: Vec<NoteEntry>,
    selected: usize,
}

impl Widget for NotesPicker {
    type Msg = Msg;

    fn build(&self) -> Element<Msg> {
        let items = self.filtered.iter().map(|e| ListItem {
            label: e.title.clone(), marked: e.is_open,
        }).collect();

        backdrop_el(|| Msg::Close)
            .child(column_el()
                .child(text_input_el(&self.search, "Search notes...")
                    .auto_focus(true)
                    .size(Size::Fixed(36.0))
                    .on_change(Msg::QueryChanged)
                    .on_cancel(|| Msg::Close)
                    .build())
                .child(make_list_el(items)
                    .on_confirm(|| Msg::Open)
                    .on_cancel(|| Msg::Close)
                    .build())
                .build())
            .build()
    }

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::QueryChanged(q) => { self.search.select_all(); self.search.paste(&q); self.refilter(); }
            Msg::Up   => { if self.selected > 0 { self.selected -= 1; } }
            Msg::Down => { if self.selected + 1 < self.filtered.len() { self.selected += 1; } }
            Msg::Open | Msg::Close => {}  // handled at overlay level
        }
    }
}
```

Compare to the React baseline:

```jsx
function NotesPicker({ notes, onOpen, onClose }) {
  const [query, setQuery] = useState('');
  const filtered = notes.filter(n => n.title.toLowerCase().includes(query));
  return (
    <Backdrop onClickOutside={onClose}>
      <TextInput value={query} onChange={setQuery} autoFocus />
      <List items={filtered} onConfirm={onOpen} />
    </Backdrop>
  );
}
```

Same structure. Same level of abstraction. Different syntax. The framework gap is closed.

---

### Framework pipeline

For every frame and every event the framework runs this pipeline:

```
build() → Element<Msg> tree  (pure, no rects)
       ↓
layout_tree(root_rect, scale) → assigns Rect to every node top-down
       ↓
render_element(theme, scale, cursor_visible) → Node tree → renderer
       ↓  (on input event)
dispatch_event(InputEvent) → DispatchResult<Msg>
       ↓
update(msg)                 → component mutation
```

The component participates only at the top (`build`) and bottom (`update`).
Everything in between is framework machinery.

---

### `Element<Msg>` — the declarative tree node

```rust
pub enum ElementKind<Msg> {
    TextInput { state, placeholder, auto_focus, on_change, on_confirm, on_cancel },
    List      { items, on_confirm, on_cancel },
    Backdrop  { on_click_outside },
    Column,
    Row,
}

pub struct Element<Msg> {
    pub kind:     ElementKind<Msg>,
    pub children: Vec<Element<Msg>>,
    pub size:     Size,      // Fixed(px) or Fill(weight)
    pub rect:     Rect,      // zero at build() time; assigned by layout_tree()
}
```

Callbacks are `Box<dyn Fn() -> Msg>` stored inside the element — they capture
nothing mutable, only produce a `Msg` value. This is the Elm/Redux pattern:
callbacks don't mutate, they describe intent. `update()` is the only mutation site.

---

### Layout engine (`src/framework/layout.rs`)

`layout_tree(root: &mut Element<Msg>, available: Rect, scale: f32)` walks the
tree top-down:

- `Backdrop` / `Column` — split available rect among children by `Size`
  - `Size::Fixed(px)` → `px * scale` pixels
  - `Size::Fill(w)` → proportional share of remaining space after fixed children
- `TextInput` / `List` — leaves; receive the rect the parent assigned
- No component stores geometry; it flows in fresh each frame

---

### Event dispatcher (`src/framework/dispatch.rs`)

`dispatch_event(root, InputEvent, focus, scale) -> DispatchResult<Msg>` walks
the laid-out tree and fires callbacks:

- **Pointer events**: depth-first hit-test; innermost matching element wins
- **Backdrop**: `on_click_outside` fires if pointer is outside all children
- **Text/keyboard**: delivered to the first `auto_focus` TextInput
- **Confirm/Cancel**: walks tree, fires first matching callback found
- **Navigation (Up/Down)**: delivered to the first List element

No component ever writes a single line of event routing.

---

## History: The Four Gaps We Closed

### Gap 1 — Stored geometry → eliminated

Old: `window: Rect`, `scale: f32` stored in every component.  
Now: `window`/`scale` flow into `run_render()`/`run_event()` at call time.
The component struct holds zero geometry.

### Gap 2 — `render()` + `on_event()` split → collapsed into `build()`

Old: Two methods, identical structural knowledge, manually kept in sync.  
Now: One `build()` method declares UI and attaches callbacks simultaneously.
`on_event()` does not exist in the framework.

### Gap 3 — `OverlayCtx` borrow split → eliminated by owned snapshots

Old: `ctx` held metadata; component held objects; every dispatch reunited them.  
Now: `build()` produces a snapshot of `TextInput` state into the Element tree.
The dispatcher mutates the snapshot; `sync_state_from_element()` writes it back.
No borrow split. No `ctx`. No re-passing children on every call.

### Gap 4 — Flat event enum requiring manual forwarding → tree dispatch

Old: `OverlayEvent::Up => { self.list.select_up(); }` — component acted as relay.  
Now: `dispatch_event` walks the tree and delivers `InputEvent::Up` to the first
List element. The component never sees navigation events it doesn't own.

---

## Current Architecture in Detail

### Focus model

```rust
pub enum Focus { Editor, TabRename, Overlay }
```

`Focus` is a discriminant tag — not one variant per component, just three broad states.
State lives in `AppLogic` fields (not behind the tag), eliminating borrow conflicts.

### Overlay / inline variant holders

```rust
pub enum ActiveOverlay { NotesPicker(NotesPicker), SlashMenu(SlashMenu) }
pub enum ActiveInline  { TabRename(TabRename) }
```

`AppLogic` holds `overlay: Option<ActiveOverlay>` and `inline: Option<ActiveInline>`.
All dispatch is a plain `match` — no trait objects, no vtable, no `Box<dyn _>`.

### Layout combinators (legacy)

`Column` / `Row` in `src/layout/` remain as geometry utilities used by the
editor and tab bar. Overlay components no longer use them — they use
`framework::layout_tree()` instead, which walks the `Element` tree directly.

---

## Invariants That Must Never Break

- `src/logic/` never imports `femtovg` or any platform dep — enforced by headless tests
- `AppLogic::render()` is the only path that produces draw data
- `NodeRenderer` is the only caller of femtovg draw primitives — one file per `Node` variant
- `DrawCtx` is the only struct that holds `&mut Canvas`
- Components never hardcode colors — all visual values flow from `Theme`
- `src/ui/types.rs` (`Rect`) has zero widget imports — pure coordinate math only

---

## File Map

```
src/
├── app/                  — OS event routing, clipboard, file I/O
│   ├── active_overlay.rs — ActiveOverlay / ActiveInline enum holders + dispatch
│   ├── overlay_event.rs  — OverlayEvent enum (platform-agnostic)
│   ├── focus/            — Focus enum + NoteEntry
│   └── mouse/            — click.rs, drag.rs, hover.rs, scroll.rs
│
├── framework/            — Element tree framework
│   ├── element.rs        — Element<Msg>, ElementKind, builder helpers
│   ├── widget.rs         — Widget trait: build() + update()
│   ├── layout.rs         — layout_tree(): assigns Rects top-down
│   ├── dispatch.rs       — dispatch_event(): hit-test + fire callbacks
│   └── render.rs         — render_element(): Element tree → Node tree
│
├── components/           — one file = one component; all use Widget::build()
│   ├── notes_picker.rs   — search input + filterable note list
│   ├── slash_menu.rs     — command palette anchored to cursor
│   └── tab_rename.rs     — inline tab-title text editor (uses OverlayEvent)
│
├── primitives/           — one file = one primitive widget
│   ├── text_input.rs     — full keyboard + pointer + selection + scroll
│   ├── list.rs           — filterable scrollable list with selection
│   └── scrollbar.rs
│
├── layout/               — pure geometry, no GPU
│   ├── node.rs           — Node enum (the declarative UI tree)
│   ├── column.rs         — Column: weight-based vertical rect splitting (editor/tabbar)
│   └── row.rs            — Row: weight-based horizontal rect splitting
│
├── logic/
│   ├── mod.rs            — AppLogic struct, dispatch_overlay, dispatch_inline
│   ├── actions/          — cursor_ops.rs, edit_ops.rs, tab_ops.rs
│   └── render/           — tab_bar.rs, editor.rs, flames.rs, overlay.rs
│
├── renderer/
│   ├── draw/             — one file per Node variant; the only GPU callers
│   └── draw_ctx.rs       — DrawCtx holds &mut Canvas
│
├── platform/             — winit event bridging, one file per event type
├── runtime/              — lightweight framework for simple apps (widget_demo)
│   ├── app_trait.rs      — App trait + ViewCtx-based routing
│   ├── focus.rs          — FocusManager (Tab cycle)
│   └── event_loop.rs
│
└── ui/                   — Rect, WindowRect, UiTree (hit-testing + hover)
```
