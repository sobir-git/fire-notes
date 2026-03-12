# fire-notes UI Framework

## Philosophy

Three rules:

1. **Local logic** — one file tells you everything about one widget. No widget reaches outside its own file for layout information.
2. **Rects flow down** — a parent subdivides its space and hands sub-rects to children at construction. Children never ask the parent where they are.
3. **Token-efficient** — layout reads like a description, not arithmetic.

---

## Three primitives

### `Rect` — space

```rust
struct Rect { x: f32, y: f32, width: f32, height: f32 }
```

Knows how to subdivide itself. The **only** place size arithmetic lives.

```rust
impl Rect {
    // Fixed-size carving — returns (strip, remainder)
    fn cut_top(self, px: f32)    -> (Rect, Rect)
    fn cut_bottom(self, px: f32) -> (Rect, Rect)
    fn cut_left(self, px: f32)   -> (Rect, Rect)
    fn cut_right(self, px: f32)  -> (Rect, Rect)
    fn inset(self, px: f32)      -> Rect

    // Proportional subdivision — CSS flexbox equivalent
    // weights: positive float = proportional share, negative = fixed px
    fn split_h(self, weights: &[f32]) -> SmallVec<Rect>
    fn split_v(self, weights: &[f32]) -> SmallVec<Rect>

    // Hit-testing
    fn contains(self, x: f32, y: f32) -> bool
    fn relative(self, x: f32, y: f32) -> (f32, f32)  // screen → local
}
```

`split_h` weight convention:
- Positive value → proportional share of remaining space (like `flex-grow`)
- Negative value → fixed pixel size (absolute)

Example: `rect.split_h(&[1.0, -12.0])` → `[fill_rect, 12px_right_strip]`

---

### `Layout` trait — "given space, produce yourself"

```rust
trait Layout: Sized {
    fn layout(rect: Rect, scale: f32) -> Self;
}
```

Every widget implements this. The widget file owns 100% of its geometry. `UiTree` just calls `Widget::layout(rect, scale)` top-down. No layout code anywhere else.

---

### `Paint` trait — "given yourself, draw"

```rust
trait Paint {
    fn paint(&self, canvas: &mut Canvas, theme: &Theme);
}
```

Renderers become: `widget.paint(canvas, theme)`. All paint logic lives in the widget file alongside its layout.

---

## The widget hierarchy

```
Window rect (width × height)
│
└── UiTree::layout(window, scale)
    │
    ├── cut_top(TAB_HEIGHT)  →  TabBar::layout(tab_rect, scale)
    │                              cut_right(controls_w)  →  WindowControls
    │                              fill                   →  TabScrollArea
    │
    └── remainder after padding  →  ContentArea::layout(content_rect, scale)
        │
        ├── split_h([1.0, -SCROLLBAR_W])
        │   ├── fill  →  TextArea::layout(text_rect, scale)
        │   └── fixed →  ScrollbarWidget::layout(sb_rect, scale)
        │
        └── (line_height cached here for visible_line_count)
```

---

## What each widget looks like

### ScrollbarWidget

```rust
pub struct ScrollbarWidget { rect: Rect, scale: f32 }

impl Layout for ScrollbarWidget {
    fn layout(rect: Rect, scale: f32) -> Self { Self { rect, scale } }
}

impl ScrollbarWidget {
    pub fn thumb(&self, total: usize, visible: usize, offset: usize) -> Option<Rect>;
    pub fn on_click(&self, x: f32, y: f32, ...) -> ScrollbarAction;
    pub fn drag_ratio(&self, y: f32, ...) -> Option<f32>;
}

impl Paint for ScrollbarWidget {
    fn paint(&self, canvas: &mut Canvas, theme: &Theme) { ... }
}
```

One file. Complete truth. `thumb()` returns a `Rect` in screen coords — the renderer just draws it.

### TextArea

```rust
pub struct TextArea { rect: Rect, line_height: f32, char_width: f32 }

impl Layout for TextArea {
    fn layout(rect: Rect, scale: f32) -> Self { ... }
}

impl TextArea {
    // Character geometry — feeds the flame system
    pub fn char_rect(&self, line: usize, col: usize, scroll: usize) -> Rect;
    pub fn visible_line_count(&self) -> usize;
    pub fn hit_to_position(&self, x: f32, y: f32, scroll: usize) -> (usize, usize);
}
```

`char_rect` is the key: the flame system asks `text_area.char_rect(line, col)` — no renderer-level re-measurement.

### ContentArea

```rust
pub struct ContentArea {
    pub rect: Rect,
    pub text: TextArea,
    pub scrollbar: ScrollbarWidget,
}

impl Layout for ContentArea {
    fn layout(rect: Rect, scale: f32) -> Self {
        let [text_rect, sb_rect] = rect.split_h(&[1.0, -(SCROLLBAR_W * scale)]);
        ContentArea {
            rect,
            text:      TextArea::layout(text_rect, scale),
            scrollbar: ScrollbarWidget::layout(sb_rect, scale),
        }
    }
}
```

### UiTree

```rust
impl Layout for UiTree {
    fn layout(rect: Rect, scale: f32) -> Self {
        let (tab_rect,  rest) = rect.cut_top(TAB_HEIGHT * scale);
        let (_pad_rect, rest) = rest.cut_top(PADDING * scale);
        UiTree {
            tab_bar: TabBar::layout(tab_rect, scale),
            content: ContentArea::layout(rest, scale),
        }
    }
}
```

---

## Flame-around-letters (stress test)

The flame system needs character positions. With `char_rect`:

```rust
// In flame update logic:
let rect = ui_tree.content.text.char_rect(line, col, scroll_offset);
flame_system.emit_at(rect.x, rect.y, rect.width, rect.height);
```

No renderer-level text measurement. No global constants. Pure local geometry from the widget that owns its space.

---

## What this eliminates

| Before | After |
|---|---|
| `layout::TAB_HEIGHT * scale` scattered in 8 files | Only in `UiTree::layout` |
| `layout::PADDING * scale` in renderers | Only in `ContentArea::layout` |
| Coordinate translation in `UiTree` | Gone — widgets own their screen rect |
| Re-measuring characters in renderer | `text_area.char_rect(line, col)` |
| Renderer knows scrollbar position | Renderer calls `scrollbar.thumb()` → draws the rect |
| Inline pixel math in `NotesPickerRenderer` | `NotesPicker::layout(rect, scale)` |

---

## Current status

All layout items are fully implemented. The framework is complete.

### Implemented ✅
1. **`Rect` primitives** — `cut_top/bottom/left/right`, `split_h`, `split_v`, `inset`, `centered_in`, `relative`
2. **`Layout` trait** in `src/ui/layout.rs`
3. **All widgets implement `Layout`** — `TabBar`, `ContentArea`, `ScrollbarWidget`, `TextArea`, `NotesPicker`, `UiTree`
4. **`ContentArea` owns children** — `content_area.text: TextArea`, `content_area.scrollbar: ScrollbarWidget`
5. **`UiTree` owns `notes_picker: Option<NotesPicker>`** — built in `logic::build_ui_tree`, read by renderer. Renderer never constructs layout objects.
6. **`TabBar` implements `Layout`** — `TabBar::layout(tab_rect, scale)`. `WindowControls` and `NewTabButton` use full `Rect`. Layout hierarchy unbroken from window to leaves.
7. **`TextArea::char_rect(line, col, scroll)`** — character positions for flame system, no renderer re-measurement.
8. **`NotesPicker` migrated** — uses `cut_top`, `inset`, `centered_in`, `split_h`.
9. **`renderer/text_content/draw.rs` split** — orchestrator (140 lines) + `text.rs` (text/cursor/selection, ~190 lines).
10. **`renderer/fonts.rs`** — shared `measure_char_width` + `snap_to_pixel`. Single source of truth used by all renderers.

### Paint trait — deliberate decision
`Paint` trait was not implemented. Reason: the `ui` module must not depend on `femtovg`.
Renderers read widget geometry (e.g. `scrollbar.thumb()`) and draw. This is the correct boundary.
If a `Paint` trait is ever added, it belongs in the renderer layer, not in `src/ui/`.

### Renderer rule (enforced)
- Zero layout coordinate arithmetic in renderers
- Zero `layout::*` constants in renderers
- Only visual constants (corner radii, border widths, font sizes) may use `* scale` in renderers

### Remaining (low priority, leave as-is)
- **`renderer/flame.rs`** (273 lines) — coherent particle system
- **`renderer/headless.rs`** (351 lines) — coherent FFI block
