# Migration Plan

Strategy: one module at a time, build stays green at every step.
When changing many things in a file → delete and recreate from scratch.
When changing one line → edit in place.

---

## Milestones

### M1 — `primitives/scrollbar.rs` [ ]
Consolidate `ui::ScrollbarWidget` + `fw::Scrollbar` + renderer scrollbar draw into one file.
Delete `src/fw/widgets/scrollbar.rs`, remove scrollbar from `src/ui/scrollbar.rs` (keep only `ScrollbarAction`, `ThumbMetrics`).

### M2 — `primitives/text_input.rs` [ ]
Consolidate `ui::TextInput` + `ui::TextInputWidget` + `fw::TextInput` + picker input draw into one file.
Delete `src/ui/text_input/`, `src/fw/widgets/text_input.rs`.

### M3 — `primitives/list.rs` [ ]
Consolidate `ui::ListWidget` + `ui::ListViewWidget` + `fw::List` + list draw into one file.
Delete `src/ui/list_widget.rs`, `src/ui/list_view.rs`, `src/fw/widgets/list.rs`.

### M4 — `primitives/button.rs` + `primitives/label.rs` [ ]
New files. ~40–80 lines each. No old code to delete.

### M5 — `layout/column.rs` + `layout/row.rs` [ ]
New files. Replace dead `fw/layout.rs`. Delete `src/fw/layout.rs`.

### M6 — `components/notes_picker.rs` [ ]
Consolidate `app/notes_picker.rs` + `renderer/notes_picker.rs` + geometry baking from `snapshot.rs`
into one file. Delete all three source files.

### M7 — `components/tab_bar.rs` [ ]
Consolidate `ui/tab_bar.rs` + `renderer/tab_bar/` + tab bar baking from `snapshot.rs`
into one file. Delete source files.

### M8 — `components/text_editor.rs` [ ]
Consolidate `ui/content_area.rs` + `ui/text_area.rs` + `renderer/text_content/` + content baking
into one file. Delete source files.

### M9 — Cleanup [ ]
Delete `src/fw/`, `src/ui/` (except `types.rs`), update `Cargo.toml` gpu feature gate.
`cargo test --no-default-features` must pass.

---

## Progress Log

- M1: pending
- M2: pending
- M3: pending
- M4: pending
- M5: pending
- M6: pending
- M7: pending
- M8: pending
- M9: pending
