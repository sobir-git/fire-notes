//! SlashMenu — floating command palette anchored to cursor position.
//! Proves that floating_rect() works and Overlay::position() is overridable.

use crate::app::Key;
use crate::layout::{FloatDir, FrameworkEvent, Node, Overlay, OverlayResult, floating_rect, overlay_panel};
use crate::primitives::{List, TextInput, list::ListPointerResult};
use crate::theme::Theme;
use crate::ui::{CursorShape, Rect};

#[derive(Debug, Clone)]
pub struct SlashCommand {
    pub name:               String,
    #[allow(dead_code)]
    pub description:        String,
}

pub struct SlashMenu {
    search: TextInput,
    list:   List<SlashCommand>,
    anchor: Rect,
    window: Rect,
    scale:  f32,
}

impl SlashMenu {
    pub fn new(commands: Vec<SlashCommand>, anchor: Rect, window: Rect, scale: f32) -> Self {
        Self { search: TextInput::new_empty(), list: List::new(commands), anchor, window, scale }
    }
    fn panel_rect(&self) -> Rect {
        floating_rect(self.anchor, self.window, self.scale, self.list.len(), FloatDir::Down)
    }
    fn slots(&self) -> [Rect; 2] {
        use crate::layout::Column;
        let col = Column::new(self.panel_rect().inset(8.0 * self.scale), self.scale, &[-36.0, 1.0]);
        [col.slot(0), col.slot(1)]
    }
    fn filter(&mut self) {
        let q = self.search.text().to_lowercase();
        if q.is_empty() { self.list.clear_filter(); }
        else { self.list.filter(move |c: &SlashCommand| c.name.to_lowercase().contains(&q)); }
    }
}

impl Overlay for SlashMenu {
    fn render(&self, theme: &Theme) -> Node {
        let [sr, lr] = self.slots();
        overlay_panel(self.window, self.scale, self.list.len(), theme, vec![
            self.search.render_at(sr, self.scale),
            self.list.render_themed_at(lr, theme, self.scale, |c: &SlashCommand| (c.name.clone(), false)),
        ])
    }

    fn on_event(&mut self, ev: FrameworkEvent) -> OverlayResult {
        use OverlayResult::*;
        match ev {
            FrameworkEvent::Char(c)              => { self.search.insert(c);   self.filter(); Redraw }
            FrameworkEvent::Backspace            => { self.search.backspace(); self.filter(); Redraw }
            FrameworkEvent::ArrowUp              => { self.list.select_up();   Redraw }
            FrameworkEvent::ArrowDown            => { self.list.select_down(); Redraw }
            FrameworkEvent::Key(Key::Escape)     => Close,
            FrameworkEvent::Key(Key::Enter)      => Close, // command execution is handled by the caller via OverlayResult::OpenPath or a future OverlayResult::Action variant
            FrameworkEvent::PointerDown { x, y, .. } => if !self.contains(x, y) { Close } else { Redraw }
            _ => Nothing,
        }
    }

    fn on_drag(&mut self, _x: f32, _y: f32) -> bool { false }
    fn end_drag(&mut self) {}
    fn contains(&self, x: f32, y: f32) -> bool {
        let [sr, lr] = self.slots();
        sr.union(&lr).contains(x, y)
    }
    fn cursor_shape_at(&self, x: f32, y: f32) -> CursorShape {
        let [sr, _] = self.slots();
        if sr.contains(x, y) { CursorShape::Text } else { CursorShape::Default }
    }
}
