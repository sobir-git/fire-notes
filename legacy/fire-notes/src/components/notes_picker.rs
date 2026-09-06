//! NotesPicker — pure state + build() + update(). Nothing else.

use crate::app::focus::NoteEntry;
use crate::app::overlay_event::OverlayResult;
use crate::framework::element::{Element, ListItem, Size, backdrop_el, column_el, list_el as list, text_input_el};
use crate::framework::runner::OverlayWidget;
use crate::framework::widget::Widget;
use crate::layout::centered_overlay_rect;
use crate::primitives::TextInput;
use crate::ui::Rect;

#[derive(Debug)]
pub enum Msg { QueryChanged(String), Open(usize), Close }

pub struct NotesPicker {
    search:   TextInput,
    entries:  Vec<NoteEntry>,
    filtered: Vec<NoteEntry>,
}

impl NotesPicker {
    pub fn new(entries: Vec<NoteEntry>, char_width: f32) -> Self {
        let mut search = TextInput::empty();
        search.set_char_width(char_width);
        let filtered = entries.clone();
        Self { search, entries, filtered }
    }

    fn refilter(&mut self) {
        let q = self.search.text().to_lowercase();
        self.filtered = if q.is_empty() {
            self.entries.clone()
        } else {
            self.entries.iter().filter(|e| e.title.to_lowercase().contains(&q)).cloned().collect()
        };
    }
}

impl Widget for NotesPicker {
    type Msg = Msg;

    fn build(&self) -> Element<Msg> {
        let items = self.filtered.iter().map(|e| ListItem { label: e.title.clone(), marked: e.is_open }).collect();
        backdrop_el(|| Msg::Close)
            .child(column_el()
                .child(text_input_el(&self.search, "Search notes...").auto_focus(true).size(Size::Fixed(36.0)).on_change(Msg::QueryChanged).on_cancel(|| Msg::Close).build())
                .child(list(items).size(Size::Fill(1.0)).on_confirm(|i| Msg::Open(i)).on_cancel(|| Msg::Close).build())
                .build())
            .build()
    }

    fn update(&mut self, msg: Msg) {
        if let Msg::QueryChanged(q) = msg {
            self.search.select_all();
            self.search.paste(&q);
            self.refilter();
        }
    }
}

impl OverlayWidget for NotesPicker {
    fn panel_rect(&self, window: Rect, scale: f32) -> Rect {
        centered_overlay_rect(window, scale, self.filtered.len())
    }
    fn result_for(&self, msg: &Msg) -> OverlayResult {
        match msg {
            Msg::Open(i) => self.filtered.get(*i).map(|e| OverlayResult::OpenPath(e.path.clone())).unwrap_or(OverlayResult::Nothing),
            Msg::Close   => OverlayResult::Close,
            _            => OverlayResult::Redraw,
        }
    }
}
