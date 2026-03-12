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

---

## Widget inventory

| Widget | File | Purpose |
|---|---|---|
| `Rect` | `ui/types.rs` | Space primitive — all arithmetic |
| `UiTree` | `ui/tree.rs` | Root compositor, hover/hit dispatch |
| `TabBar` | `ui/tab_bar.rs` | Tab strip, window controls, new-tab button |
| `ContentArea` | `ui/content_area.rs` | Text area + scrollbar |
| `TextArea` | `ui/text_area.rs` | Character geometry, hit-to-position |
| `ScrollbarWidget` | `ui/scrollbar.rs` | Thumb geometry, drag/click actions |
| `NotesPicker` | `ui/notes_picker.rs` | Search overlay, item rows |
| `TextInputWidget` | `ui/text_input/` | Single-line text field with geometry |
| `ListWidget<T>` | `ui/list_widget.rs` | Selection + filter state (no geometry) |
