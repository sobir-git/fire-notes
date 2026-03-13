#![allow(dead_code)]
//! Notes picker component — a pure composition of primitives.
//!
//! Structure:
//!   overlay_rect
//!     └── Column([-input_h, 1.0])
//!           ├── slot 0 → TextInput  (search query)
//!           └── slot 1 → List<NoteEntry>  (filtered results)
//!
//! No manual geometry. The component holds two primitives; layout is entirely
//! delegated to Column. Snapshot reads fields off the primitives directly.

use crate::app::focus::NoteEntry;
use crate::layout::Column;
use crate::primitives::{List, TextInput};
use crate::render_frame::{FrameRect, NotesPickerFrameData, NotesPickerRowData};
use crate::ui::{CursorShape, Rect};

const MAX_VISIBLE: usize = 8;
const INPUT_H:    f32    = 36.0;   // dp
const ITEM_H:     f32    = 32.0;   // dp
const PADDING:    f32    = 8.0;    // dp
const FONT_SIZE:  f32    = 14.0;   // dp

// ── Component ─────────────────────────────────────────────────────────────────

/// Notes picker — a composition of TextInput + List laid out by Column.
pub struct NotesPicker {
    pub search: TextInput,
    pub list:   List<NoteEntry>,
    /// Cached overlay rect (recomputed on relayout).
    overlay:    Rect,
    scale:      f32,
}

impl NotesPicker {
    pub fn new(entries: Vec<NoteEntry>, window: Rect, scale: f32) -> Self {
        let mut list = List::new(entries);
        list.set_max_visible(MAX_VISIBLE);
        let search = TextInput::new_empty();
        let overlay = overlay_rect(window, scale, list.len());
        let mut s = Self { search, list, overlay, scale };
        s.apply_layout(overlay, scale);
        s
    }

    pub fn relayout(&mut self, window: Rect, scale: f32) {
        self.scale   = scale;
        self.overlay = overlay_rect(window, scale, self.list.len());
        self.apply_layout(self.overlay, scale);
    }

    // ── State passthrough ─────────────────────────────────────────────────────

    pub fn scroll_by(&mut self, lines: isize) -> bool { self.list.scroll_by(lines) }
    pub fn select_up(&mut self)   -> bool { self.list.select_up() }
    pub fn select_down(&mut self) -> bool { self.list.select_down() }
    pub fn selected_item(&self)   -> Option<&NoteEntry> { self.list.selected_item() }
    pub fn is_empty(&self)        -> bool { self.list.is_empty() }

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

    pub fn overlay_rect(&self) -> Rect { self.overlay }

    // ── Snapshot ──────────────────────────────────────────────────────────────

    /// Bake into a pure data snapshot. Reads geometry directly off the
    /// two primitives — no coordinate arithmetic here.
    pub fn snapshot(&self, window: Rect, cursor_visible: bool) -> NotesPickerFrameData {
        let scale      = self.scale;
        let font_size  = FONT_SIZE * scale;
        let item_h     = ITEM_H * scale;
        let padding    = PADDING * scale;
        let list_rect  = self.list.list_rect();
        let input_rect = self.search.rect;

        let indicator_x           = list_rect.x + list_rect.width - 2.0 * padding;
        let input_text_x          = self.search.text_x;
        let input_text_baseline_y = self.search.text_baseline_y;

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
                let item  = self.list.items().get(filtered_idx)?;
                let row_y = list_rect.y + display_idx as f32 * item_h;
                Some(NotesPickerRowData {
                    title:       item.title.clone(),
                    is_open:     item.is_open,
                    is_selected: scroll_offset + display_idx == selected_index,
                    row_rect:    FrameRect { x: list_rect.x, y: row_y, width: list_rect.width, height: item_h },
                    baseline_y:  row_y + item_h * 0.65,
                    center_y:    row_y + item_h * 0.5,
                })
            })
            .collect();

        let scrollbar = self.list.scrollbar_rects()
            .map(|(track, thumb)| (to_frame(track), to_frame(thumb)));

        NotesPickerFrameData {
            backdrop_rect:        to_frame(window),
            overlay_rect:         to_frame(self.overlay),
            input_rect:           to_frame(input_rect),
            input_text_x,
            input_text_baseline_y,
            query:                self.search.text().to_string(),
            input_scroll_offset:  self.search.scroll_offset(),
            input_cursor:         self.search.state.cursor,
            cursor_visible,
            input_selection,
            font_size,
            scale,
            indicator_x,
            rows,
            scrollbar,
            no_results:           self.list.is_empty() && !self.search.text().is_empty(),
            first_row_baseline_y: list_rect.y + item_h * 0.65,
        }
    }

    // ── Private ───────────────────────────────────────────────────────────────

    /// Apply Column layout to the two children. Called on construction and relayout.
    fn apply_layout(&mut self, overlay: Rect, scale: f32) {
        let padding = PADDING * scale;
        let inner   = overlay.inset(padding);
        // Column: fixed input row, list fills the rest
        let col = Column::new(inner, scale, &[-INPUT_H, 1.0]);
        self.search.relayout(col.slot(0), scale);
        self.list.relayout(col.slot(1), scale);
    }
}

// ── Free helpers ──────────────────────────────────────────────────────────────

/// Compute the centered overlay rect for the picker.
/// Depends on item count (drives height).
fn overlay_rect(window: Rect, scale: f32, item_count: usize) -> Rect {
    let padding   = PADDING * scale;
    let input_h   = INPUT_H * scale;
    let item_h    = ITEM_H  * scale;
    let visible   = item_count.min(MAX_VISIBLE);
    let overlay_w = (window.width * 0.6).min(500.0 * scale);
    let overlay_h = input_h + visible as f32 * item_h + 2.0 * padding;
    let (_, below_top) = window.cut_top(60.0 * scale);
    below_top.centered_in(overlay_w, overlay_h)
}

fn to_frame(r: Rect) -> FrameRect {
    FrameRect { x: r.x, y: r.y, width: r.width, height: r.height }
}
