//! NotesPicker — one file, one component.

use crate::app::focus::NoteEntry;
use crate::app::Key;
use crate::layout::{Column, FrameworkEvent, Node, Overlay, OverlayResult, centered_overlay_rect, overlay_panel};
use crate::primitives::{List, TextInput, list::ListPointerResult};
use crate::theme::Theme;
use crate::ui::{CursorShape, Rect};

pub struct NotesPicker { search: TextInput, list: List<NoteEntry>, window: Rect, scale: f32 }

impl NotesPicker {
    pub fn new(entries: Vec<NoteEntry>, window: Rect, scale: f32) -> Self {
        Self { search: TextInput::new_empty(), list: List::new(entries), window, scale }
    }
    fn slots(&self) -> [Rect; 2] {
        let panel = centered_overlay_rect(self.window, self.scale, self.list.len());
        let col   = Column::new(panel.inset(8.0 * self.scale), self.scale, &[-36.0, 1.0]);
        [col.slot(0), col.slot(1)]
    }
    fn filter(&mut self) {
        let q = self.search.text().to_lowercase();
        if q.is_empty() { self.list.clear_filter(); } else { self.list.filter(move |n: &NoteEntry| n.title.to_lowercase().contains(&q)); }
    }
}

impl Overlay for NotesPicker {
    fn render(&self, theme: &Theme) -> Node {
        let [search_rect, list_rect] = self.slots();
        overlay_panel(self.window, self.scale, self.list.len(), theme, vec![
            self.search.render_at(search_rect, self.scale),
            self.list.render_themed_at(list_rect, theme, self.scale, |e: &NoteEntry| (e.title.clone(), e.is_open)),
        ])
    }

    fn on_event(&mut self, ev: FrameworkEvent) -> OverlayResult {
        use OverlayResult::*;
        match ev {
            FrameworkEvent::Char(c)     => { self.search.insert(c);   self.filter(); Redraw }
            FrameworkEvent::Backspace   => { self.search.backspace(); self.filter(); Redraw }
            FrameworkEvent::ArrowUp     => { self.list.select_up();   Redraw }
            FrameworkEvent::ArrowDown   => { self.list.select_down(); Redraw }
            FrameworkEvent::Key(Key::Enter)  => self.list.selected_item().map(|e| OpenPath(e.path.clone())).unwrap_or(Nothing),
            FrameworkEvent::Key(Key::Escape) => Close,
            FrameworkEvent::PointerDown { x, y, char_width } => {
                if !self.contains(x, y) { return Close; }
                self.search.on_pointer_down_with(x, y, char_width);
                if let ListPointerResult::Confirmed(_) = self.list.on_pointer_down(x, y) {
                    self.list.selected_item().map(|e| OpenPath(e.path.clone())).unwrap_or(Nothing)
                } else { Redraw }
            }
            FrameworkEvent::PointerMove { x, y }          => if self.list.on_hover(x, y) { Redraw } else { Nothing }
            FrameworkEvent::PointerDrag { x, char_width } => { self.search.on_drag(x, char_width); Redraw }
            FrameworkEvent::Scroll { lines }               => if self.list.scroll_by(lines) { Redraw } else { Nothing }
            _ => Nothing,
        }
    }

    fn on_drag(&mut self, _x: f32, y: f32) -> bool   { self.list.continue_scrollbar_drag(y) }
    fn end_drag(&mut self)                            { self.list.end_drag(); }
    fn contains(&self, x: f32, y: f32) -> bool {
        let [sr, lr] = self.slots();
        sr.union(&lr).contains(x, y)
    }
    fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape {
        let [sr, _] = self.slots();
        if sr.contains(x, y) { CursorShape::Text } else { CursorShape::Default }
    }
}
