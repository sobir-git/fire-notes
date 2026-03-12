//! Application state and coordination
//!
//! Architecture overview:
//! - `Focus` - determines which component receives keyboard input
//! - `UiState` - transient UI state (hover, cursor blink, mouse)
//! - `App` - coordinates between components, owns tabs and renderer

pub(crate) mod action;
mod file;
pub(crate) mod focus;
mod input;
pub(crate) mod input_handler;
pub(crate) mod keybindings;
mod mouse;
mod notes_picker;
mod scroll;
pub(crate) mod scroll_state;
pub(crate) mod state;
mod tabs;
pub(crate) mod ui_state;

use arboard::Clipboard;

use crate::logic::AppLogic;
use crate::persistence;
use crate::renderer::Renderer;
use crate::tab::Tab;

pub use focus::NoteEntry;
pub use keybindings::{Key, KeyEvent, Modifiers, resolve as resolve_keybinding};
pub use scroll_state::ScrollInput;
pub use state::AppResult;
pub use crate::ui::CursorShape;
pub use ui_state::{MouseInteraction, UiState};

pub struct App {
    /// GPU renderer — the only thing App owns that AppLogic cannot.
    renderer: Renderer,
    /// OS clipboard — requires platform access, excluded from AppLogic.
    clipboard: Option<Clipboard>,
    /// Pure logic layer — all document and UI state lives here.
    pub(crate) logic: AppLogic,
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

        let (tabs, active_tab) = if let Some(session) = persistence::load_session_state() {
            let mut loaded_tabs = Vec::new();
            let mut active_index = None;
            for (index, tab_state) in session.tabs.iter().enumerate() {
                if let Some(mut tab) = Tab::from_file(tab_state.path.clone()) {
                    tab.apply_state(tab_state);
                    if session.active_path.as_ref().map(|p| p == &tab_state.path).unwrap_or(false) {
                        active_index = Some(index);
                    }
                    loaded_tabs.push(tab);
                }
            }
            (loaded_tabs, active_index.unwrap_or(0))
        } else {
            let tabs = match persistence::list_notes() {
                Ok(note_paths) if !note_paths.is_empty() => note_paths
                    .into_iter()
                    .filter_map(Tab::from_file)
                    .collect(),
                _ => vec![Tab::new_untitled()],
            };
            (tabs, 0)
        };

        let logic = AppLogic::new_from_session(tabs, active_tab, width, height, scale);

        Self { renderer, clipboard, logic }
    }

    // =========================================================================
    // Core lifecycle
    // =========================================================================

    pub fn tick(&mut self) -> AppResult {
        let logic_result = self.logic.tick();
        // Continuously redraw when flame particles are active
        if self.renderer.has_active_flames() {
            return AppResult::Redraw;
        }
        logic_result
    }

    pub fn resize(&mut self, width: f32, height: f32, scale: f32) {
        self.logic.resize(width, height, scale);
        self.renderer.resize(width, height, scale);
    }

    pub fn render(&mut self) {
        let ui = &self.logic.ui_state;
        let renaming_tab_index = self.logic.focus.renaming_tab_index();
        let rename_input = self.logic.focus.rename_input();
        let notes_picker_state = self.logic.focus.notes_picker_state();

        let tab_info: Vec<(&str, bool)> = self.logic.tabs
            .iter()
            .enumerate()
            .map(|(i, t)| {
                if Some(i) == renaming_tab_index {
                    if let Some(input) = rename_input {
                        (input.text(), i == self.logic.active_tab)
                    } else {
                        (t.title(), i == self.logic.active_tab)
                    }
                } else {
                    (t.title(), i == self.logic.active_tab)
                }
            })
            .collect();

        // Build UiTree once per frame — TabBar flows from here to the renderer.
        let ui_tree = self.logic.build_ui_tree(&tab_info);

        let current_tab = &self.logic.tabs[self.logic.active_tab];
        let dragging_scrollbar = matches!(ui.mouse_interaction, MouseInteraction::ScrollbarDrag { .. });

        self.renderer.render(&crate::renderer::RenderFrame {
            ui_tree:                &ui_tree,
            tabs:                   &tab_info,
            current_tab,
            cursor_visible:         ui.cursor_visible,
            hovered_tab_index:      ui.hovered_tab_index,
            hovered_plus:           ui.hovered_plus,
            hovered_scrollbar:      ui.hovered_scrollbar,
            dragging_scrollbar,
            renaming_tab:           renaming_tab_index,
            rename_input,
            typing_flame_positions: &ui.typing_flame_positions,
            hovered_window_minimize: ui.hovered_window_minimize,
            hovered_window_maximize: ui.hovered_window_maximize,
            hovered_window_close:    ui.hovered_window_close,
            notes_picker_state,
        });
    }

    // =========================================================================
    // Layout helpers
    // =========================================================================

    pub(crate) fn visible_lines(&self) -> usize {
        self.logic.visible_line_count()
    }

    pub(crate) fn auto_scroll(&mut self) {
        let char_width = self.renderer.get_char_width();
        self.logic.set_char_width_hint(char_width);
        self.logic.auto_scroll();
    }

    pub(crate) fn tab_titles(&self) -> Vec<(&str, bool)> {
        self.logic.tabs
            .iter()
            .enumerate()
            .map(|(i, t)| (t.title(), i == self.logic.active_tab))
            .collect()
    }

    pub fn has_active_animations(&self) -> bool {
        self.renderer.has_active_flames() || !self.logic.ui_state.typing_flame_positions.is_empty()
    }

    pub fn handle_scroll_event(&mut self, input: ScrollInput) -> AppResult {
        self.logic.handle_scroll_event(input)
    }

    pub fn is_mouse_in_tab_bar(&self) -> bool {
        self.logic.is_mouse_in_tab_bar()
    }

    pub fn ui_state(&self) -> &UiState {
        self.logic.ui_state()
    }

    pub fn scroll_tab_bar(&mut self, delta: f32) -> AppResult {
        self.logic.scroll_tab_bar(delta)
    }

    pub fn reset_scroll_state(&mut self) {
        self.logic.reset_scroll_state();
    }

    // =========================================================================
    // Session state
    // =========================================================================

    pub fn export_session_state(&self) -> persistence::SessionState {
        self.logic.export_session_state()
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
