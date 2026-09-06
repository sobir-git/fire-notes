//! TabRename — inline tab-title editor. One file, one component.

use crate::app::overlay_event::{InlineResult, OverlayEvent};
use crate::primitives::TextInput;
use crate::ui::Rect;

pub struct TabRename {
    pub tab_index: usize,
    pub input:     TextInput,
}

impl TabRename {
    pub fn new(tab_index: usize, title: &str, rect: Rect, scale: f32) -> Self {
        let mut input = TextInput::new(title);
        let _ = (rect, scale);
        input.select_all();
        Self { tab_index, input }
    }

    pub fn on_event(&mut self, ev: OverlayEvent<'_>) -> InlineResult {
        use InlineResult::*;
        match ev {
            OverlayEvent::Confirm                => Commit(self.input.text().trim().to_string()),
            OverlayEvent::Cancel                 => Cancel,
            OverlayEvent::PointerDown { .. } | OverlayEvent::PointerDrag { .. } => Redraw,
            OverlayEvent::Text(text_ev) => {
                let dummy = Rect { x: 0.0, y: 0.0, width: 9999.0, height: 24.0 };
                self.input.handle_key(text_ev, dummy, 1.0);
                Redraw
            }
            _ => Nothing,
        }
    }
}
