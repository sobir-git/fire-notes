//! TabRename — inline tab-title editor. One file, one component.

use crate::app::Key;
use crate::layout::{FrameworkEvent, InlineResult, InlineWidget};
use crate::primitives::TextInput;
use crate::ui::Rect;

pub struct TabRename {
    #[allow(dead_code)]
    pub tab_index: usize,
    pub input:     TextInput,
}

impl TabRename {
    pub fn new(tab_index: usize, title: &str, rect: Rect, scale: f32) -> Self {
        let mut input = TextInput::new(title.to_string());
        input.relayout(rect, scale);
        input.state.select_all();
        Self { tab_index, input }
    }
}

impl InlineWidget for TabRename {
    fn as_any(&self) -> &dyn std::any::Any { self }

    fn on_event(&mut self, ev: FrameworkEvent) -> InlineResult {
        use InlineResult::*;
        match ev {
            FrameworkEvent::Char(c)              => { self.input.insert(c);     Redraw }
            FrameworkEvent::Backspace            => { self.input.backspace();   Redraw }
            FrameworkEvent::Key(Key::Enter)      => Commit(self.input.text().trim().to_string()),
            FrameworkEvent::Key(Key::Escape)     => Cancel,
            FrameworkEvent::PointerDown { x, y, char_width } => {
                self.input.on_pointer_down_with(x, y, char_width); Redraw
            }
            FrameworkEvent::PointerDrag { x, char_width } => {
                self.input.on_drag(x, char_width); Redraw
            }
            _ => Nothing,
        }
    }
}
