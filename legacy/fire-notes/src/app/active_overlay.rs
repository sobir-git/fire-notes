//! Concrete overlay and inline widget holders.
//! All pipeline logic lives in `framework::runner`. This file only matches on
//! the enum variant and calls the framework functions.

use crate::app::overlay_event::{InlineResult, OverlayEvent, OverlayResult};
use crate::components::notes_picker::NotesPicker;
use crate::components::slash_menu::SlashMenu;
use crate::components::tab_rename::TabRename;
use crate::framework::{run_event, run_render};
use crate::layout::Node;
use crate::theme::Theme;
use crate::ui::{CursorShape, Rect};

pub enum ActiveOverlay {
    NotesPicker(NotesPicker),
    SlashMenu(SlashMenu),
}

impl ActiveOverlay {
    pub fn render(&mut self, theme: &Theme, window: Rect, scale: f32, cursor_visible: bool) -> Node {
        match self {
            Self::NotesPicker(p) => run_render(p, window, scale, theme, cursor_visible),
            Self::SlashMenu(m)   => run_render(m, window, scale, theme, cursor_visible),
        }
    }

    pub fn on_event(&mut self, ev: OverlayEvent<'_>, window: Rect, scale: f32) -> OverlayResult {
        match self {
            Self::NotesPicker(p) => run_event(p, ev, window, scale),
            Self::SlashMenu(m)   => run_event(m, ev, window, scale),
        }
    }

    pub fn on_drag(&mut self, _x: f32, _y: f32) -> bool { false }
    pub fn end_drag(&mut self) {}

    pub fn cursor_shape_at(&self, _x: f32, _y: f32) -> CursorShape {
        CursorShape::Default
    }
}

// ── Active inline widget ──────────────────────────────────────────────────────

pub enum ActiveInline {
    TabRename(TabRename),
}

impl ActiveInline {
    pub fn on_event(&mut self, ev: OverlayEvent<'_>) -> InlineResult {
        match self {
            Self::TabRename(r) => r.on_event(ev),
        }
    }

    pub fn tab_index(&self) -> usize {
        match self {
            Self::TabRename(r) => r.tab_index,
        }
    }
}
