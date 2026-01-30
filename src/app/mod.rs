//! Application state and coordination
//!
//! Architecture overview:
//! - `Focus` - determines which component receives keyboard input
//! - `UiState` - transient UI state (hover, cursor blink, mouse)
//! - `App` - coordinates between components, owns tabs and renderer

mod action;
mod file;
mod focus;
mod input;
mod input_handler;
mod keybindings;
mod mouse;
mod notes_picker;
mod scroll;
mod scroll_state;
mod state;
mod tabs;
mod ui_state;

use arboard::Clipboard;

use crate::config::{self, layout, timing};
use crate::persistence;
use crate::renderer::Renderer;
use crate::tab::Tab;

pub use focus::{Focus, NoteEntry};
pub use keybindings::{Key, KeyEvent, Modifiers, resolve as resolve_keybinding};
pub use scroll_state::{ScrollDirection, ScrollInput, ScrollState};
pub use state::AppResult;
pub use ui_state::{MouseInteraction, UiState};

pub struct App {
    // Core components
    renderer: Renderer,
    tabs: Vec<Tab>,
    active_tab: usize,

    // Window state
    width: f32,
    height: f32,
    scale: f32,

    // Input/clipboard
    clipboard: Option<Clipboard>,

    // State management (new architecture)
    focus: Focus,
    ui_state: UiState,
    scroll_state: ScrollState,
}

impl App {
    pub fn new(
        gl_renderer: femtovg::renderer::OpenGl,
        width: f32,
        height: f32,
        scale: f32,
    ) -> Self {
        let renderer = Renderer::new(gl_renderer, width, height, scale);
        let clipboard = Clipboard::new().ok();

        let (mut tabs, active_tab) = if let Some(session) = persistence::load_session_state() {
            let mut loaded_tabs = Vec::new();
            let mut active_index = None;

            for (index, tab_state) in session.tabs.iter().enumerate() {
                if let Some(mut tab) = Tab::from_file(tab_state.path.clone()) {
                    tab.apply_state(tab_state);
                    if session
                        .active_path
                        .as_ref()
                        .map(|path| path == &tab_state.path)
                        .unwrap_or(false)
                    {
                        active_index = Some(index);
                    }
                    loaded_tabs.push(tab);
                }
            }

            let active_tab = active_index.unwrap_or(0);
            (loaded_tabs, active_tab)
        } else {
            let tabs = match persistence::list_notes() {
                Ok(note_paths) if !note_paths.is_empty() => note_paths
                    .into_iter()
                    .filter_map(|path| Tab::from_file(path))
                    .collect(),
                _ => vec![Tab::new_untitled()],
            };
            (tabs, 0)
        };

        if tabs.is_empty() {
            tabs.push(Tab::new_untitled());
        }

        Self {
            renderer,
            tabs,
            active_tab,
            width,
            height,
            scale,
            clipboard,
            focus: Focus::default(),
            ui_state: UiState::new(),
            scroll_state: ScrollState::new(),
        }
    }

    // =========================================================================
    // Core lifecycle
    // =========================================================================

    pub fn tick(&mut self) -> AppResult {
        let mut needs_redraw = false;

        // Cursor blink
        if self.ui_state.tick_cursor_blink(timing::CURSOR_BLINK_MS) {
            needs_redraw = true;
        }

        // Clean up expired typing flame positions
        if self.ui_state.cleanup_typing_flames(config::flame::TYPING_FLAME_EXPIRY) {
            needs_redraw = true;
        }

        // Continuously redraw when flame particles are active
        if self.renderer.has_active_flames() {
            needs_redraw = true;
        }

        if needs_redraw {
            AppResult::Redraw
        } else {
            AppResult::Ok
        }
    }

    pub fn resize(&mut self, width: f32, height: f32, scale: f32) {
        self.width = width;
        self.height = height;
        self.scale = scale;
        self.renderer.resize(width, height, scale);
    }

    pub fn render(&mut self) {
        let renaming_tab_index = self.focus.renaming_tab_index();
        let rename_input = self.focus.rename_input();
        let notes_picker_state = self.focus.notes_picker_state();

        let tab_info: Vec<(&str, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| {
                if Some(i) == renaming_tab_index {
                    if let Some(input) = rename_input {
                        (input.text(), i == self.active_tab)
                    } else {
                        (t.title(), i == self.active_tab)
                    }
                } else {
                    (t.title(), i == self.active_tab)
                }
            })
            .collect();

        let current_tab = &self.tabs[self.active_tab];

        self.renderer.render(
            &tab_info,
            current_tab,
            self.ui_state.cursor_visible,
            self.ui_state.hovered_tab_index,
            self.ui_state.hovered_plus,
            self.ui_state.hovered_scrollbar,
            matches!(self.ui_state.mouse_interaction, MouseInteraction::ScrollbarDrag { .. }),
            renaming_tab_index,
            rename_input,
            &self.ui_state.typing_flame_positions,
            self.ui_state.hovered_window_minimize,
            self.ui_state.hovered_window_maximize,
            self.ui_state.hovered_window_close,
            notes_picker_state,
        );
    }

    // =========================================================================
    // Layout helpers
    // =========================================================================

    pub(crate) fn visible_lines(&self) -> usize {
        let content_height =
            self.height - layout::TAB_HEIGHT * self.scale - layout::PADDING * 2.0 * self.scale;
        (content_height / (layout::LINE_HEIGHT * self.scale))
            .floor()
            .max(1.0) as usize
    }

    pub(crate) fn content_start_y(&self) -> f32 {
        layout::TAB_HEIGHT * self.scale + layout::PADDING * self.scale
    }

    pub(crate) fn auto_scroll(&mut self) {
        let visible = self.visible_lines();
        let visible_width = self.width - layout::PADDING * 2.0 * self.scale;
        let char_width = self.renderer.get_char_width();
        self.tabs[self.active_tab].ensure_cursor_visible(visible, visible_width, char_width);
        self.ui_state.reset_cursor_blink();
    }

    pub(crate) fn tab_titles(&self) -> Vec<(&str, bool)> {
        self.tabs
            .iter()
            .enumerate()
            .map(|(i, t)| (t.title(), i == self.active_tab))
            .collect()
    }

    pub fn hovered_resize_edge(&self) -> Option<crate::ui::ResizeEdge> {
        self.ui_state.hovered_resize_edge
    }

    /// Check if any animations are currently active (flames, typing effects)
    pub fn has_active_animations(&self) -> bool {
        self.renderer.has_active_flames() || !self.ui_state.typing_flame_positions.is_empty()
    }

    /// Process a scroll event and apply it to the active tab
    pub fn handle_scroll_event(&mut self, input: ScrollInput) -> AppResult {
        let Some((direction, lines)) = self.scroll_state.process_scroll(input) else {
            return AppResult::Ok;
        };

        match direction {
            ScrollDirection::Up => {
                for _ in 0..lines {
                    self.tabs[self.active_tab].scroll_up(1);
                }
            }
            ScrollDirection::Down => {
                let visible = self.visible_lines();
                for _ in 0..lines {
                    self.tabs[self.active_tab].scroll_down(1, visible);
                }
            }
        }

        AppResult::Redraw
    }

    /// Check if mouse is in tab bar area
    pub fn is_mouse_in_tab_bar(&self) -> bool {
        self.ui_state.last_mouse_y < layout::TAB_HEIGHT * self.scale
    }

    /// Get UI state reference for hover checks
    pub fn ui_state(&self) -> &UiState {
        &self.ui_state
    }

    /// Scroll the tab bar horizontally
    pub fn scroll_tab_bar(&mut self, delta: f32) -> AppResult {
        if delta > 0.0 {
            self.ui_state.tab_scroll_x = (self.ui_state.tab_scroll_x - delta.abs()).max(0.0);
        } else {
            let max_scroll = 1000.0;
            self.ui_state.tab_scroll_x = (self.ui_state.tab_scroll_x + delta.abs()).min(max_scroll);
        }
        self.renderer.set_tab_scroll_x(self.ui_state.tab_scroll_x);
        AppResult::Redraw
    }

    /// Reset scroll state (call when scroll interaction ends)
    pub fn reset_scroll_state(&mut self) {
        self.scroll_state.reset();
    }

    // =========================================================================
    // Session state
    // =========================================================================

    pub fn export_session_state(&self) -> persistence::SessionState {
        let active_path = self
            .tabs
            .get(self.active_tab)
            .and_then(|tab| tab.path().cloned());
        let tabs = self
            .tabs
            .iter()
            .filter_map(|tab| tab.export_state())
            .collect();
        persistence::SessionState { active_path, tabs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_management_logic() {
        let tab = Tab::new_untitled();
        assert!(tab.title().starts_with("Untitled"));
    }
}
