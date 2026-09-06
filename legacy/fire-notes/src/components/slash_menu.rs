//! SlashMenu — pure state + build() + update(). Nothing else.

use crate::app::overlay_event::OverlayResult;
use crate::framework::element::{Element, ListItem, Size, backdrop_el, column_el, list_el as list, text_input_el};
use crate::framework::runner::OverlayWidget;
use crate::framework::widget::Widget;
use crate::layout::{FloatDir, floating_rect};
use crate::primitives::TextInput;
use crate::ui::Rect;

#[derive(Debug, Clone)]
pub struct SlashCommand {
    pub name:        String,
    #[allow(dead_code)]
    pub description: String,
}

#[derive(Debug)]
pub enum Msg { QueryChanged(String), Confirm, Close }

pub struct SlashMenu {
    search:   TextInput,
    commands: Vec<SlashCommand>,
    filtered: Vec<SlashCommand>,
    pub anchor: Rect,
}

impl SlashMenu {
    pub fn new(commands: Vec<SlashCommand>, anchor: Rect, char_width: f32) -> Self {
        let mut search = TextInput::empty();
        search.set_char_width(char_width);
        let filtered = commands.clone();
        Self { search, commands, filtered, anchor }
    }

    fn refilter(&mut self) {
        let q = self.search.text().to_lowercase();
        self.filtered = if q.is_empty() {
            self.commands.clone()
        } else {
            self.commands.iter().filter(|c| c.name.to_lowercase().contains(&q)).cloned().collect()
        };
    }
}

impl Widget for SlashMenu {
    type Msg = Msg;

    fn build(&self) -> Element<Msg> {
        let items = self.filtered.iter().map(|c| ListItem { label: c.name.clone(), marked: false }).collect();
        backdrop_el(|| Msg::Close)
            .child(column_el()
                .child(text_input_el(&self.search, "Search commands...").auto_focus(true).size(Size::Fixed(36.0)).on_change(Msg::QueryChanged).on_cancel(|| Msg::Close).build())
                .child(list(items).size(Size::Fill(1.0)).on_confirm(|_| Msg::Confirm).on_cancel(|| Msg::Close).build())
                .build())
            .build()
    }

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::QueryChanged(q) => { self.search.select_all(); self.search.paste(&q); self.refilter(); }
            Msg::Confirm | Msg::Close => {}
        }
    }
}

impl OverlayWidget for SlashMenu {
    fn panel_rect(&self, window: Rect, scale: f32) -> Rect {
        floating_rect(self.anchor, window, scale, self.filtered.len(), FloatDir::Down)
    }
    fn result_for(&self, msg: &Msg) -> OverlayResult {
        match msg {
            Msg::Confirm | Msg::Close => OverlayResult::Close,
            Msg::QueryChanged(_)      => OverlayResult::Redraw,
        }
    }
}
