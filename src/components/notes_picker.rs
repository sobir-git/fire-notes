#![allow(dead_code)]
//! Notes picker component — a pure composition of primitives.
//!
//! Structure:
//!   overlay_rect
//!     └── Column([-INPUT_H, 1.0])
//!           ├── slot 0 → TextInput  (search query)
//!           └── slot 1 → List<NoteEntry>  (filtered results)
//!
//! Event routing: Column::hit_slot dispatches to the correct child.
//! NotesPicker emits typed PickerEvent — caller just matches the enum.
//! All hover/cursor/scroll logic lives here, not in the caller.

use crate::app::focus::NoteEntry;
use crate::layout::Column;
use crate::primitives::{List, TextInput};
use crate::primitives::list::ListPointerResult;
use crate::render_frame::{FrameRect, NotesPickerFrameData, NotesPickerRowData};
use crate::ui::{CursorShape, Rect};

const MAX_VISIBLE: usize = 8;
const INPUT_H:    f32    = 36.0;
const ITEM_H:     f32    = 32.0;
const PADDING:    f32    = 8.0;
const FONT_SIZE:  f32    = 14.0;

// ── Domain events ─────────────────────────────────────────────────────────────

/// Everything a caller needs to know from a picker interaction.
#[derive(Debug, Clone, PartialEq)]
pub enum PickerEvent {
    /// Nothing happened.
    None,
    /// State changed — redraw.
    Redraw,
    /// User confirmed a selection — open this entry.
    Confirmed(NoteEntry),
    /// User cancelled (outside click or Escape).
    Cancelled,
}

// ── Component ─────────────────────────────────────────────────────────────────

/// Notes picker — TextInput + List composed via Column.
/// ~50 lines of logic; the framework handles routing.
pub struct NotesPicker {
    pub search: TextInput,
    pub list:   List<NoteEntry>,
    /// Cached layout — recomputed on relayout.
    layout:     Column,
    overlay:    Rect,
    scale:      f32,
}

impl NotesPicker {
    pub fn new(entries: Vec<NoteEntry>, window: Rect, scale: f32) -> Self {
        let mut list = List::new(entries);
        list.set_max_visible(MAX_VISIBLE);
        let search  = TextInput::new_empty();
        let overlay = overlay_rect(window, scale, list.len());
        let layout  = Column::new(overlay.inset(PADDING * scale), scale, &[-INPUT_H, 1.0]);
        let mut s   = Self { search, list, layout, overlay, scale };
        s.search.relayout(s.layout.slot(0), scale);
        s.list.relayout(s.layout.slot(1), scale);
        s
    }

    pub fn relayout(&mut self, window: Rect, scale: f32) {
        self.scale   = scale;
        self.overlay = overlay_rect(window, scale, self.list.len());
        self.layout  = Column::new(self.overlay.inset(PADDING * scale), scale, &[-INPUT_H, 1.0]);
        self.search.relayout(self.layout.slot(0), scale);
        self.list.relayout(self.layout.slot(1), scale);
    }

    // ── Events — Column dispatches, NotesPicker maps to PickerEvent ───────────

    pub fn on_pointer_down(&mut self, x: f32, y: f32, char_width: f32) -> PickerEvent {
        if !self.overlay.contains(x, y) {
            return PickerEvent::Cancelled;
        }
        match self.layout.hit_slot(x, y) {
            Some(0) => {
                self.search.on_pointer_down_with(x, y, char_width);
                PickerEvent::Redraw
            }
            Some(1) => match self.list.on_pointer_down(x, y) {
                ListPointerResult::Confirmed(_) => {
                    self.list.selected_item().cloned()
                        .map(PickerEvent::Confirmed)
                        .unwrap_or(PickerEvent::None)
                }
                ListPointerResult::None => PickerEvent::None,
                _                       => PickerEvent::Redraw,
            },
            _ => PickerEvent::None,
        }
    }

    /// Returns `(cursor_shape, needs_redraw)`.
    pub fn on_hover(&mut self, x: f32, y: f32) -> (CursorShape, bool) {
        let cursor       = self.cursor_shape_at(x, y);
        let list_changed = self.list.on_hover(x, y);
        (cursor, list_changed)
    }

    pub fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape {
        match self.layout.hit_slot(x, y) {
            Some(0) => self.search.cursor_shape_at(x, y),
            _       => CursorShape::Default,
        }
    }

    pub fn on_drag(&mut self, x: f32, char_width: f32) {
        self.search.on_drag(x, char_width);
    }

    pub fn on_scroll(&mut self, lines: isize) -> bool { self.list.scroll_by(lines) }
    pub fn select_up(&mut self)   -> bool { self.list.select_up() }
    pub fn select_down(&mut self) -> bool { self.list.select_down() }
    pub fn selected_item(&self)   -> Option<&NoteEntry> { self.list.selected_item() }
    pub fn is_empty(&self)        -> bool { self.list.is_empty() }
    pub fn overlay_rect(&self)    -> Rect { self.overlay }

    pub fn update_filter(&mut self, window: Rect, scale: f32) {
        let query = self.search.text().to_lowercase();
        if query.is_empty() { self.list.clear_filter(); }
        else { self.list.filter(move |n: &NoteEntry| n.title.to_lowercase().contains(&query)); }
        self.relayout(window, scale);
    }

    // ── Snapshot ──────────────────────────────────────────────────────────────

    pub fn snapshot(&self, window: Rect, cursor_visible: bool) -> NotesPickerFrameData {
        let scale      = self.scale;
        let font_size  = FONT_SIZE * scale;
        let item_h     = ITEM_H * scale;
        let padding    = PADDING * scale;
        let list_rect  = self.list.list_rect();
        let input_rect = self.search.rect;

        let input_selection = self.search.state.selection_anchor.map(|anchor| {
            let cursor = self.search.state.cursor;
            (anchor.min(cursor), anchor.max(cursor))
        });

        let scroll_offset  = self.list.scroll_offset();
        let selected_index = self.list.selected_index();
        let row_visible    = self.list.visible_count();

        let rows: Vec<NotesPickerRowData> = self.list
            .filtered_indices().iter()
            .skip(scroll_offset).take(row_visible).enumerate()
            .filter_map(|(di, &fi)| {
                let item  = self.list.items().get(fi)?;
                let row_y = list_rect.y + di as f32 * item_h;
                Some(NotesPickerRowData {
                    title:       item.title.clone(),
                    is_open:     item.is_open,
                    is_selected: scroll_offset + di == selected_index,
                    row_rect:    FrameRect { x: list_rect.x, y: row_y, width: list_rect.width, height: item_h },
                    baseline_y:  row_y + item_h * 0.65,
                    center_y:    row_y + item_h * 0.5,
                })
            })
            .collect();

        NotesPickerFrameData {
            backdrop_rect:        to_frame(window),
            overlay_rect:         to_frame(self.overlay),
            input_rect:           to_frame(input_rect),
            input_text_x:         self.search.text_x,
            input_text_baseline_y: self.search.text_baseline_y,
            query:                self.search.text().to_string(),
            input_scroll_offset:  self.search.scroll_offset(),
            input_cursor:         self.search.state.cursor,
            cursor_visible,
            input_selection,
            font_size,
            scale,
            indicator_x:          list_rect.x + list_rect.width - 2.0 * padding,
            rows,
            scrollbar:            self.list.scrollbar_rects()
                                      .map(|(t, th)| (to_frame(t), to_frame(th))),
            no_results:           self.list.is_empty() && !self.search.text().is_empty(),
            first_row_baseline_y: list_rect.y + item_h * 0.65,
        }
    }
}

// ── Free helpers ──────────────────────────────────────────────────────────────

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
