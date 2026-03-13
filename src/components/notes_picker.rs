#![allow(dead_code)]
//! Notes picker component — state + geometry + snapshot, one file.
//!
//! Owns the search input and result list. Produces `NotesPickerFrameData`
//! for the renderer without any GPU imports. Draw code lives in
//! `renderer/notes_picker.rs` and consumes the pure frame data.

use crate::app::focus::NoteEntry;
use crate::fw::widgets::{List, TextInput as FwTextInput};
use crate::logic::{picker_relayout, PICKER_MAX_VISIBLE};
use crate::render_frame::{FrameRect, NotesPickerFrameData, NotesPickerRowData};
use crate::ui::{CursorShape, Rect};

// ── Component ─────────────────────────────────────────────────────────────────

/// Notes picker — owns search widget, list widget, and geometry baking.
pub struct NotesPicker {
    pub search: FwTextInput,
    pub list:   List<NoteEntry>,
}

impl NotesPicker {
    /// Create and relayout immediately.
    pub fn new(entries: Vec<NoteEntry>, window: Rect, scale: f32) -> Self {
        let mut list   = List::new(entries);
        let mut search = FwTextInput::new_empty();
        list.set_max_visible(PICKER_MAX_VISIBLE);
        picker_relayout(window, scale, &mut list, &mut search);
        Self { search, list }
    }

    /// Relayout both widgets (call on resize or filter change).
    pub fn relayout(&mut self, window: Rect, scale: f32) {
        picker_relayout(window, scale, &mut self.list, &mut self.search);
    }

    // ── State passthrough ─────────────────────────────────────────────────────

    pub fn scroll_by(&mut self, lines: isize) -> bool { self.list.scroll_by(lines) }

    pub fn select_up(&mut self)   -> bool { self.list.select_up() }
    pub fn select_down(&mut self) -> bool { self.list.select_down() }

    pub fn selected_item(&self) -> Option<&NoteEntry> { self.list.selected_item() }

    pub fn is_empty(&self) -> bool { self.list.is_empty() }

    pub fn update_filter(&mut self, window: Rect, scale: f32) {
        let query = self.search.text().to_lowercase();
        if query.is_empty() {
            self.list.clear_filter();
        } else {
            self.list.filter(move |note: &NoteEntry| {
                note.title.to_lowercase().contains(&query)
            });
        }
        self.relayout(window, scale);
    }

    pub fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape {
        self.search.cursor_shape_at(x, y)
    }

    pub fn on_hover(&mut self, x: f32, y: f32) -> bool {
        self.list.on_hover(x, y)
    }

    // ── Overlay rect (for outside-click detection) ────────────────────────────

    pub fn overlay_rect(&self, window: Rect, scale: f32) -> Rect {
        let padding      = 8.0  * scale;
        let input_height = 36.0 * scale;
        let item_height  = 32.0 * scale;
        let visible      = self.list.len().min(PICKER_MAX_VISIBLE);
        let overlay_w    = (window.width * 0.6).min(500.0 * scale);
        let overlay_h    = input_height + visible as f32 * item_height + 2.0 * padding;
        let (_, below_top) = window.cut_top(60.0 * scale);
        below_top.centered_in(overlay_w, overlay_h)
    }

    // ── Snapshot ──────────────────────────────────────────────────────────────

    /// Bake all geometry into a pure data snapshot for the renderer.
    pub fn snapshot(
        &self,
        window: Rect,
        scale: f32,
        cursor_visible: bool,
    ) -> NotesPickerFrameData {
        let padding      = 8.0  * scale;
        let input_height = 36.0 * scale;
        let item_height  = 32.0 * scale;
        let font_size    = 14.0 * scale;

        let visible_count = self.list.len().min(PICKER_MAX_VISIBLE);
        let overlay_w = (window.width * 0.6).min(500.0 * scale);
        let overlay_h = input_height + visible_count as f32 * item_height + 2.0 * padding;
        let (_, below_top) = window.cut_top(60.0 * scale);
        let overlay_rect = below_top.centered_in(overlay_w, overlay_h);

        let (input_rect_ui, list_remainder) =
            overlay_rect.inset(padding).cut_top(input_height - 4.0 * scale);
        let list_rect = Rect {
            x:      list_remainder.x,
            y:      list_remainder.y + 4.0 * scale,
            width:  list_remainder.width,
            height: visible_count as f32 * item_height,
        };

        let indicator_x          = list_rect.x + list_rect.width - 2.0 * padding;
        let input_text_x         = input_rect_ui.x + padding;
        let input_text_baseline_y = input_rect_ui.y + input_rect_ui.height * 0.65;

        let input_selection = self.search.state.selection_anchor.map(|anchor| {
            let cursor = self.search.state.cursor;
            (anchor.min(cursor), anchor.max(cursor))
        });

        let scroll_offset  = self.list.scroll_offset();
        let selected_index = self.list.selected_index();
        let row_visible    = self.list.visible_count();

        let rows: Vec<NotesPickerRowData> = self.list
            .filtered_indices()
            .iter()
            .skip(scroll_offset)
            .take(row_visible)
            .enumerate()
            .filter_map(|(display_idx, &filtered_idx)| {
                let item = self.list.items().get(filtered_idx)?;
                let row_y    = list_rect.y + display_idx as f32 * item_height;
                let row_rect = FrameRect {
                    x: list_rect.x, y: row_y,
                    width: list_rect.width, height: item_height,
                };
                Some(NotesPickerRowData {
                    title:       item.title.clone(),
                    is_open:     item.is_open,
                    is_selected: scroll_offset + display_idx == selected_index,
                    row_rect,
                    baseline_y:  row_y + item_height * 0.65,
                    center_y:    row_y + item_height * 0.5,
                })
            })
            .collect();

        let scrollbar = self.list.scrollbar_rects().map(|(track, thumb)| {
            (to_frame(track), to_frame(thumb))
        });

        NotesPickerFrameData {
            backdrop_rect:         to_frame(window),
            overlay_rect:          to_frame(overlay_rect),
            input_rect:            to_frame(input_rect_ui),
            input_text_x,
            input_text_baseline_y,
            query:                 self.search.text().to_string(),
            input_scroll_offset:   self.search.scroll_offset(),
            input_cursor:          self.search.state.cursor,
            cursor_visible,
            input_selection,
            font_size,
            scale,
            indicator_x,
            rows,
            scrollbar,
            no_results:            self.list.is_empty() && !self.search.text().is_empty(),
            first_row_baseline_y:  list_rect.y + item_height * 0.65,
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn to_frame(r: Rect) -> FrameRect {
    FrameRect { x: r.x, y: r.y, width: r.width, height: r.height }
}

