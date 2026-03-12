//! Notes picker overlay — layout, hit-testing, and interaction.
//!
//! Implements `Layout`: given the window rect, computes all geometry via
//! `Rect` primitives. No inline arithmetic in renderers.
//!
//! # Interaction ownership
//! All picker interaction decisions (click, hover, drag, scroll, filter, cursor-visible)
//! are methods on `NotesPicker` itself. The app layer calls these methods and acts on
//! the returned `PickerAction` — it never reaches into widget internals directly.

use super::layout::Layout;
use super::list_view::ListViewWidget;
use super::list_widget::ListWidget;
use super::scrollbar::ScrollbarAction;
use super::text_input::{TextInput, TextInputWidget};
use super::types::{CursorShape, Rect};

// ── Interaction return types ───────────────────────────────────────────────

/// Result of a click on the notes picker.
#[derive(Debug)]
pub enum PickerClickOutcome {
    /// Click landed outside the overlay — picker should close.
    Cancel,
    /// Click placed cursor in the input field; caller owns the drag state.
    InputFocus,
    /// User started dragging the scrollbar thumb.
    StartScrollbarDrag { drag_offset: f32 },
    /// Scrollbar track jump — scroll offset was updated inside `list`.
    Scrolled,
    /// User clicked on a list item.
    Selected,
    /// User double-clicked an already-selected item — confirm it.
    Confirmed,
    /// Click was inside the overlay but hit no interactive element.
    None,
}

pub const MAX_VISIBLE_ITEMS: usize = 8;

/// Complete layout for the notes picker overlay.
#[derive(Debug, Clone)]
pub struct NotesPicker {
    /// Full window — semi-transparent backdrop.
    pub backdrop_rect: Rect,
    /// Floating panel rect.
    pub overlay_rect: Rect,
    /// Search input field widget — owns rect, text geometry, and cursor shape.
    pub input: TextInputWidget,
    /// Scrollable list view — owns row rects, scrollbar geometry.
    pub list: ListViewWidget,
    /// Font size for all picker text.
    pub font_size: f32,
    /// Scale factor.
    pub scale: f32,
    /// X of the right-side "open" indicator dot.
    pub indicator_x: f32,
}

impl NotesPicker {
    /// Build from window dimensions, scale, and current list length.
    pub fn new(window_width: f32, window_height: f32, scale: f32, list_len: usize) -> Self {
        let window = Rect { x: 0.0, y: 0.0, width: window_width, height: window_height };
        let mut picker = Self::layout(window, scale);
        picker.resize_for_list(list_len);
        picker
    }

    fn resize_for_list(&mut self, list_len: usize) {
        let visible = list_len.min(MAX_VISIBLE_ITEMS);
        let list_height = visible as f32 * self.list.item_height;
        let padding      = self.scale * 8.0;
        let input_height = self.input.rect.height;
        self.overlay_rect.height = input_height + list_height + 2.0 * padding;
        // Recompute list rect to match trimmed overlay height.
        self.list = ListViewWidget::layout(
            Rect {
                x:      self.list.rect.x,
                y:      self.list.rect.y,
                width:  self.list.rect.width,
                height: list_height,
            },
            self.scale,
        );
    }

    /// Hit-test: returns display index of item under (x, y), if any.
    pub fn item_hit_test(&self, x: f32, y: f32, total_items: usize) -> Option<usize> {
        if !self.overlay_rect.contains(x, y) { return None; }
        self.list.hit_test_item(x, y, total_items)
    }

    /// Hit-test the scrollbar and return the appropriate action.
    /// Returns `None` if the click is not on the scrollbar.
    pub fn scrollbar_action(&self, x: f32, y: f32, total_items: usize, scroll_offset: usize) -> Option<ScrollbarAction> {
        if !self.list.scrollbar.hit_test(x, y) { return None; }
        Some(self.list.scrollbar_click(x, y, total_items, scroll_offset))
    }

    /// Cursor shape appropriate for the area under (x, y).
    /// Delegates to the input widget for the Text-cursor region.
    pub fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape {
        self.input.cursor_shape_at(x, y)
    }

    // ── Interaction ───────────────────────────────────────────────────────

    /// Handle a mouse click. Updates `input` and `list` state as needed.
    /// Returns a `PickerClickOutcome` describing what happened so the caller
    /// can act (e.g. close the picker, start a drag, confirm a selection).
    pub fn handle_click<T>(
        &self,
        x: f32,
        y: f32,
        input: &mut TextInput,
        list: &mut ListWidget<T>,
        char_width: f32,
    ) -> PickerClickOutcome {
        if !self.overlay_rect.contains(x, y) {
            return PickerClickOutcome::Cancel;
        }

        // Click in input field → place cursor.
        if self.input.rect.contains(x, y) {
            let relative_x = x - self.input.text_x;
            input.set_cursor_from_x(relative_x, char_width, false);
            return PickerClickOutcome::InputFocus;
        }

        // Click on scrollbar.
        let list_len = list.len();
        let scroll_offset = list.scroll_offset();
        if let Some(action) = self.scrollbar_action(x, y, list_len, scroll_offset) {
            match action {
                ScrollbarAction::StartDrag { drag_offset } => {
                    return PickerClickOutcome::StartScrollbarDrag { drag_offset };
                }
                ScrollbarAction::JumpTo { ratio } => {
                    let target = self.list.scroll_offset_from_ratio(ratio, list_len);
                    list.scroll_to(target);
                    return PickerClickOutcome::Scrolled;
                }
                ScrollbarAction::None => {}
            }
        }

        // Click in list area → select or confirm.
        if let Some(display_idx) = self.item_hit_test(x, y, list_len.min(MAX_VISIBLE_ITEMS)) {
            let clicked_idx = list.scroll_offset() + display_idx;
            let was_already_selected = list.selected_index() == clicked_idx;
            if list.select_index(clicked_idx) {
                if was_already_selected {
                    return PickerClickOutcome::Confirmed;
                }
                return PickerClickOutcome::Selected;
            }
        }

        PickerClickOutcome::None
    }

    /// Handle a drag on the input field (text selection drag).
    /// Always updates cursor — caller is responsible for only calling this
    /// when a TextSelection drag is in progress on the picker input.
    pub fn handle_input_drag(
        &self,
        x: f32,
        input: &mut TextInput,
        char_width: f32,
    ) {
        let relative_x = x - self.input.text_x;
        input.set_cursor_from_x(relative_x, char_width, true);
    }

    /// Handle scrollbar thumb drag. Returns true if scroll offset changed.
    pub fn handle_scrollbar_drag<T>(
        &self,
        y: f32,
        drag_offset: f32,
        list: &mut ListWidget<T>,
    ) -> bool {
        let list_len = list.len();
        let scroll_offset = list.scroll_offset();
        if let Some(ratio) = self.list.scrollbar_drag_ratio(y, list_len, scroll_offset, drag_offset) {
            let target = self.list.scroll_offset_from_ratio(ratio, list_len);
            list.scroll_to(target);
            return true;
        }
        false
    }

    /// Handle mouse hover. Returns the cursor shape for the hovered area.
    pub fn cursor_shape_at_hover(&self, x: f32, y: f32) -> CursorShape {
        self.cursor_shape_at(x, y)
    }

    /// Scroll the list by `lines` (positive = down, negative = up).
    /// Returns true if the selection changed (redraw needed).
    pub fn scroll_list<T>(&self, lines: isize, list: &mut ListWidget<T>) -> bool {
        if lines > 0 {
            (0..lines as usize).fold(false, |acc, _| list.select_down() || acc)
        } else {
            (0..(-lines) as usize).fold(false, |acc, _| list.select_up() || acc)
        }
    }

    /// Ensure the input cursor stays visible after editing.
    /// `char_width` must come from the renderer (font measurement).
    pub fn ensure_input_cursor_visible(&self, input: &mut TextInput, char_width: f32) {
        self.input.ensure_cursor_visible(input, char_width);
    }

}

impl Layout for NotesPicker {
    /// Layout from the full window rect.
    /// List height defaults to `MAX_VISIBLE_ITEMS`; call `new()` for the correct
    /// height based on actual list length.
    fn layout(window: Rect, scale: f32) -> Self {
        let padding      = 8.0  * scale;
        let input_height = 36.0 * scale;
        let item_height  = 32.0 * scale;
        let font_size    = 14.0 * scale;

        // Overlay: 60% of window width, capped at 500 logical px, dropped 60px from top.
        let overlay_w = (window.width * 0.6).min(500.0 * scale);
        let overlay_h = input_height + MAX_VISIBLE_ITEMS as f32 * item_height + 2.0 * padding;
        let (_, below_top) = window.cut_top(60.0 * scale);
        let overlay_rect   = below_top.centered_in(overlay_w, overlay_h);

        // Input box: overlay inset by padding, input_height tall.
        let (input_rect, list_remainder) = overlay_rect.inset(padding).cut_top(input_height - 4.0 * scale);
        let input = TextInputWidget::layout(input_rect, scale);

        // List area: remainder of inset overlay after input, capped to MAX_VISIBLE_ITEMS rows.
        let list_rect = Rect {
            x:      list_remainder.x,
            y:      list_remainder.y + 4.0 * scale,
            width:  list_remainder.width,
            height: MAX_VISIBLE_ITEMS as f32 * item_height,
        };
        let list = ListViewWidget::layout(list_rect, scale);

        // Indicator dot: right edge of the list content area (left of scrollbar), inset by padding.
        let indicator_x = list.list_rect.x + list.list_rect.width - 2.0 * padding;

        Self {
            backdrop_rect: window,
            overlay_rect,
            input,
            list,
            font_size,
            scale,
            indicator_x,
        }
    }
}
