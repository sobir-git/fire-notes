# Framework Architecture Plan — Closing the React Gap

## Goal
The widget demo should be ~50 lines of app code, not ~400.
Every line of boilerplate that exists in the demo today is a framework gap.

---

## Target demo (what we're building toward)

```rust
struct Demo {
    inputs: [TextInput; 3],
}

impl App for Demo {
    fn view(&self, ctx: &ViewCtx) -> Node {
        let labels = ["Empty input:", "Pre-filled:", "Long / scroll:"];
        let placeholders = ["Type here...", "", ""];
        let rects = ctx.layout.column(3);
        Node::layer(rects.iter().enumerate().map(|(i, &rect)| {
            self.inputs[i].view(rect, ctx, placeholders[i])
        }).collect())
    }
}

fn main() {
    Runtime::new()
        .title("Widget Demo")
        .run(Demo { inputs: [TextInput::new(""), TextInput::new("Pre-filled…"), TextInput::new("Long…")] });
}
```

---

## Gaps and fixes (top-down)

### Gap 1 — No `Runtime` / app entry point
Every demo re-implements: OpenGL setup, window creation, event loop, resize
handling, redraw scheduling.

**Fix:** `src/runtime/mod.rs` — a `Runtime` struct that owns the window, GL
context, event loop, renderer, clock, clipboard, and focus manager. App code
calls `Runtime::new().run(app)`.

### Gap 2 — No focus system
App code tracks focused widget index manually; Tab switching is manual;
keyboard events are manually routed to the focused field.

**Fix:** `src/runtime/focus.rs` — `FocusManager` owns the list of focusable
widget IDs. The runtime routes keyboard events to the currently focused widget.
Tab cycles focus automatically. No app code needed.

### Gap 3 — Clipboard owned by caller
Every `Ctrl+C/X/V` branches through caller-owned clipboard handle.

**Fix:** `src/runtime/clipboard.rs` — `ClipboardPort` trait + arboard impl,
stored in `Runtime`. `TextInput::handle_key` returns `EventOutcome::Copy(text)`
/ `EventOutcome::NeedsPaste`; the runtime handles the clipboard I/O before
dispatching the result back.

### Gap 4 — Cursor blink owned by caller
`cursor_on: bool`, `last_blink: Instant`, `tick()` all live in every app.

**Fix:** `src/runtime/clock.rs` — runtime owns a 530ms blink timer, tracks
`cursor_on: bool`, passes it via `ViewCtx`. App never touches it.

### Gap 5 — `set_char_width` must be called manually
Every consumer must remember to call `input.set_char_width(cw)` after font load
and on resize.

**Fix:** `ViewCtx` carries `char_width: f32` measured by the renderer. `TextInput::view`
reads it from ctx — no setter needed.

### Gap 6 — Rect duplication: `view(rect)` and event handler must agree
`build_node` calls `view(rect_a)` but the keyboard handler recomputes `rect_b`
independently. Any mismatch breaks scroll/click.

**Fix:** Layout is computed once per frame into a shared `LayoutCache` inside
`ViewCtx`. Both `view()` and event routing read from the same cache entry by
widget ID.

### Gap 7 — No `App` trait
Every demo implements `ApplicationHandler` (a winit detail) manually.

**Fix:** `pub trait App { fn view(&self, ctx: &ViewCtx) -> Node; }` — that's
the entire contract. Runtime implements `ApplicationHandler` internally.

---

## File layout after refactor

```
src/runtime/
  mod.rs          — Runtime struct, builder, run()
  app_trait.rs    — pub trait App
  clock.rs        — blink timer, frame clock
  clipboard.rs    — ClipboardPort trait + arboard impl
  focus.rs        — FocusManager, focus cycling
  view_ctx.rs     — ViewCtx passed to App::view()
  event_loop.rs   — ApplicationHandler impl (winit detail, hidden from app)
```

---

## Execution order

1. `src/runtime/view_ctx.rs` — ViewCtx (carries char_width, cursor_on, focused set, scale)
2. `src/runtime/app_trait.rs` — App trait
3. `src/runtime/clock.rs` — blink timer
4. `src/runtime/clipboard.rs` — ClipboardPort + arboard impl
5. `src/runtime/focus.rs` — FocusManager
6. `src/runtime/event_loop.rs` — ApplicationHandler over App
7. `src/runtime/mod.rs` — Runtime builder + run()
8. Update `TextInput::view` to accept `&ViewCtx` instead of raw `(rect, scale)`
9. Rewrite `src/bin/widget_demo.rs` — target ≤60 lines
10. All tests green
