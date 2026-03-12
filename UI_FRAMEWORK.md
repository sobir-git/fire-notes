# fire-notes UI Framework

## Philosophy

Three rules:

1. **Local logic** — one file tells you everything about one widget. No widget reaches outside its own file for layout information.
2. **Rects flow down** — a parent subdivides its space and hands sub-rects to children at construction. Children never reach upward.
3. **Token-efficient** — layout reads like a description, not arithmetic.

---

## Core contracts

### `Rect` — the only place size arithmetic lives

`Rect` knows how to subdivide itself: `cut_top/bottom/left/right`, `split_h/v`, `inset`, `centered_in`, `contains`. Every layout operation composes these primitives — never raw `+`/`*` elsewhere.

### `Layout` trait — "given space, produce yourself"

```rust
trait Layout: Sized {
    fn layout(rect: Rect, scale: f32) -> Self;
}
```

Every child widget implements this. The parent hands a `Rect` down; the child owns everything inside it. No widget knows its parent's dimensions.

### `WindowRect` — compile-time root guard

`WindowRect::new` is `pub(crate)`. Only the app entry point can create one. Widgets receive `Rect`, never `WindowRect` — so a child physically cannot encode assumptions about the window size or tab bar height. This turns an architectural rule into a compiler error.

### No `Paint` trait

The `ui` module must not import `femtovg`. Renderers read widget geometry and draw. This is the correct boundary. Visual constants (radii, font sizes) live in the renderer layer, not in `src/ui/`.

---

## Rules

**Layout**
- All coordinate arithmetic inside `Rect` methods only.
- Parents subdivide their rect and pass sub-rects to children via `Widget::layout(rect, scale)`.
- No widget constructs its own rect from raw dimensions.

**Renderers**
- Zero layout arithmetic in renderers.
- Zero `layout::*` constants in renderers.
- Only visual constants (`corner_radius`, `border_width`, `font_size`) may use `* scale`.

**Cursor shape**
- Each widget owns its cursor shape decision locally (inside `UiTree::hover()` or a widget's `cursor_shape_at()`).
- `events.rs` is a pure translation table: `CursorShape → winit::CursorIcon`. No cursor decisions there.

**Text fields**
- Any text input field in the UI uses `TextInputWidget` — never a bare `Rect` + manual geometry.

**Interaction ownership**
- A widget owns *all* of its interaction logic (click, hover, drag, scroll, cursor-visible) as methods on the widget struct in its own file.
- The app layer calls e.g. `picker.handle_click(x, y, input, list, char_width)` and receives a typed outcome (`PickerClickOutcome`). It never inspects widget internals directly.
- `App::execute(action)` is the single keyboard dispatch path. It delegates to `AppLogic::execute` for all pure logic. Only three categories stay at the App layer: clipboard (OS access), `OpenNotesPicker` (persistence read), and `Save`/`OpenFile` (filesystem).

---

## Widget inventory

| Widget | File | Owns |
|---|---|---|
| `Rect` | `ui/types.rs` | Space primitive — all arithmetic |
| `UiTree` | `ui/tree.rs` | Root compositor, hover/hit dispatch |
| `TabBar` | `ui/tab_bar.rs` | Tab strip, window controls, new-tab button |
| `ContentArea` | `ui/content_area.rs` | Text area + scrollbar |
| `TextArea` | `ui/text_area.rs` | Character geometry, hit-to-position |
| `ScrollbarWidget` | `ui/scrollbar.rs` | Thumb geometry, drag/click actions |
| `NotesPicker` | `ui/notes_picker.rs` | Search overlay layout **and all interaction**: click (`handle_click`), drag (`handle_input_drag`, `handle_scrollbar_drag`), scroll (`scroll_list`), cursor-visible (`ensure_input_cursor_visible`) |
| `TextInputWidget` | `ui/text_input/` | Single-line text field with geometry |
| `ListWidget<T>` | `ui/list_widget.rs` | Selection + filter state (no geometry) |
| `ListViewWidget` | `ui/list_view.rs` | Scrollable list geometry paired with `ListWidget` |

---

## Action dispatch model

```
keyboard event
  └── platform/events.rs          translate to KeyEvent
        └── keybindings::resolve  KeyEvent → Action
              └── App::execute(action)
                    ├── Copy/Cut/Paste   → OS clipboard + logic.cut_selection() / copy_selection() / insert_paste_text()
                    ├── OpenNotesPicker  → app/notes_picker.rs (reads persistence)
                    ├── Save/OpenFile    → filesystem (Tab::save, Tab::open)
                    └── everything else  → AppLogic::execute(action)
                                               └── logic/actions/{edit_ops,cursor_ops,tab_ops}.rs
```

## Deleted / consolidated

The following files were removed as part of the local-logic refactoring:

- `src/app/input/` (entire directory) — was a dead duplicate of `src/logic/actions/`. All keyboard actions now go through `AppLogic::execute`.
- `src/app/file.rs` — `save_current`, `open_file`, `rename_current` shims moved inline into `App::execute`.
