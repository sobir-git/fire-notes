//! Application state and coordination

use crate::config::{layout, timing};
use crate::persistence;
use crate::renderer::{HitTestResult, Renderer};
use crate::tab::Tab;
use std::time::{Duration, Instant};

use arboard::Clipboard;

/// Result type for application actions that may trigger UI updates
#[must_use = "Handle the AppResult to ensure the UI updates correctly"]
pub enum AppResult {
    /// No action needed
    Ok,
    /// UI needs to be redrawn
    Redraw,
}

impl AppResult {
    pub fn needs_redraw(&self) -> bool {
        matches!(self, AppResult::Redraw)
    }
}

/// Transient editor state for UI interactions (cursor blink, hover, drag, etc.)
struct EditorState {
    cursor_visible: bool,
    last_cursor_blink: Instant,
    hovered_tab_index: Option<usize>,
    hovered_plus: bool,
    is_dragging_scrollbar: bool,
    dragging_tab_index: Option<usize>,
    last_drag_scroll: Instant,
    last_mouse_x: f32,
    last_mouse_y: f32,
    tab_scroll_x: f32,
    renaming_tab: Option<usize>,
    rename_buffer: String,
}

impl EditorState {
    fn new() -> Self {
        Self {
            cursor_visible: true,
            last_cursor_blink: Instant::now(),
            hovered_tab_index: None,
            hovered_plus: false,
            is_dragging_scrollbar: false,
            dragging_tab_index: None,
            last_drag_scroll: Instant::now(),
            last_mouse_x: 0.0,
            last_mouse_y: 0.0,
            tab_scroll_x: 0.0,
            renaming_tab: None,
            rename_buffer: String::new(),
        }
    }

    /// Reset cursor blink (call after user action)
    fn reset_cursor_blink(&mut self) {
        self.cursor_visible = true;
        self.last_cursor_blink = Instant::now();
    }
}

pub struct App {
    renderer: Renderer,
    tabs: Vec<Tab>,
    active_tab: usize,
    width: f32,
    height: f32,
    scale: f32,
    clipboard: Option<Clipboard>,
    state: EditorState,
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
            state: EditorState::new(),
        }
    }

    // =========================================================================
    // Core lifecycle
    // =========================================================================

    pub fn tick(&mut self) -> AppResult {
        if self.state.last_cursor_blink.elapsed() >= Duration::from_millis(timing::CURSOR_BLINK_MS)
        {
            self.state.cursor_visible = !self.state.cursor_visible;
            self.state.last_cursor_blink = Instant::now();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn resize(&mut self, width: f32, height: f32, scale: f32) {
        self.width = width;
        self.height = height;
        self.scale = scale;
        self.renderer.resize(width, height, scale);
    }

    pub fn render(&mut self) {
        let tab_info: Vec<(&str, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| {
                if Some(i) == self.state.renaming_tab {
                    (self.state.rename_buffer.as_str(), i == self.active_tab)
                } else {
                    (t.title(), i == self.active_tab)
                }
            })
            .collect();

        let current_tab = &self.tabs[self.active_tab];

        self.renderer.render(
            &tab_info,
            current_tab,
            self.state.cursor_visible,
            self.state.hovered_tab_index,
            self.state.hovered_plus,
            self.state.renaming_tab,
        );
    }

    // =========================================================================
    // Layout helpers
    // =========================================================================

    fn visible_lines(&self) -> usize {
        let content_height =
            self.height - layout::TAB_HEIGHT * self.scale - layout::PADDING * 2.0 * self.scale;
        (content_height / (layout::LINE_HEIGHT * self.scale))
            .floor()
            .max(1.0) as usize
    }

    fn content_start_y(&self) -> f32 {
        layout::TAB_HEIGHT * self.scale + layout::PADDING * self.scale
    }

    /// Convert screen coordinates to text coordinates (line, col).
    /// Returns None if the click is outside the text area.
    #[allow(dead_code)]
    fn screen_to_text_coords(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        let content_start_y = self.content_start_y();
        if y < content_start_y {
            return None;
        }

        let relative_y = y - content_start_y;
        let visual_line = (relative_y / (layout::LINE_HEIGHT * self.scale)).floor() as isize;
        let scroll_offset = self.tabs[self.active_tab].scroll_offset();
        let line = (scroll_offset as isize + visual_line).max(0) as usize;

        let char_width = self.renderer.get_char_width();
        let scroll_offset_x = self.tabs[self.active_tab].scroll_offset_x();
        let relative_x = (x - layout::PADDING * self.scale + scroll_offset_x).max(0.0);
        let col = (relative_x / char_width).round() as usize;

        Some((line, col))
    }

    fn auto_scroll(&mut self) {
        let visible = self.visible_lines();
        let visible_width = self.width - layout::PADDING * 2.0 * self.scale;
        let char_width = self.renderer.get_char_width();
        self.tabs[self.active_tab].ensure_cursor_visible(visible, visible_width, char_width);
        self.state.reset_cursor_blink();
    }

    // =========================================================================
    // Mouse handling
    // =========================================================================

    pub fn handle_mouse_move(&mut self, x: f32, y: f32) -> AppResult {
        self.state.last_mouse_x = x;
        self.state.last_mouse_y = y;

        let tab_info: Vec<(&str, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| (t.title(), i == self.active_tab))
            .collect();

        let prev_hovered_tab_index = self.state.hovered_tab_index;
        let prev_hovered_plus = self.state.hovered_plus;

        match self.renderer.hit_test(x, y, &tab_info) {
            Some(HitTestResult::Tab(i)) => {
                self.state.hovered_tab_index = Some(i);
                self.state.hovered_plus = false;
            }
            Some(HitTestResult::NewTabButton) => {
                self.state.hovered_tab_index = None;
                self.state.hovered_plus = true;
            }
            None => {
                self.state.hovered_tab_index = None;
                self.state.hovered_plus = false;
            }
        }

        if prev_hovered_tab_index != self.state.hovered_tab_index
            || prev_hovered_plus != self.state.hovered_plus
        {
            AppResult::Redraw
        } else {
            AppResult::Ok
        }
    }

    pub fn click_at(&mut self, x: f32, y: f32, selecting: bool) -> AppResult {
        let scrollbar_width = layout::SCROLLBAR_WIDTH * self.scale;
        let tab_info: Vec<(&str, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| (t.title(), i == self.active_tab))
            .collect();

        // Check for tab interactions first (only if not selecting)
        if !selecting && y < layout::TAB_HEIGHT * self.scale {
            match self.renderer.hit_test(x, y, &tab_info) {
                Some(HitTestResult::Tab(i)) => {
                    self.active_tab = i;
                    self.auto_scroll();
                    self.state.dragging_tab_index = Some(i);
                    return AppResult::Redraw;
                }
                Some(HitTestResult::NewTabButton) => {
                    return self.new_tab();
                }
                None => {
                    return AppResult::Ok;
                }
            }
        }

        // Check if clicked on scrollbar (only if not already selecting text)
        if !selecting {
            if x > self.width - scrollbar_width {
                let content_start_y = layout::TAB_HEIGHT * self.scale;
                if y >= content_start_y {
                    self.state.is_dragging_scrollbar = true;
                    return self.drag_at(x, y);
                }
            } else {
                self.state.is_dragging_scrollbar = false;
            }
        }

        // Calculate which line was clicked
        let content_start_y = self.content_start_y();

        if !selecting && y < content_start_y {
            return AppResult::Ok;
        }

        let height = self.visible_lines() as isize;
        let relative_y = y - content_start_y;
        let mut clicked_visual_line =
            (relative_y / (layout::LINE_HEIGHT * self.scale)).floor() as isize;

        if selecting {
            if clicked_visual_line < 0 || clicked_visual_line >= height {
                if self.state.last_drag_scroll.elapsed()
                    < Duration::from_millis(timing::DRAG_SCROLL_THROTTLE_MS)
                {
                    return AppResult::Ok;
                }
                self.state.last_drag_scroll = Instant::now();

                if clicked_visual_line < 0 {
                    clicked_visual_line = -1;
                } else {
                    clicked_visual_line = height;
                }
            }
        }

        let scroll_offset = self.tabs[self.active_tab].scroll_offset();
        let clicked_line = (scroll_offset as isize + clicked_visual_line).max(0) as usize;

        let char_width = self.renderer.get_char_width();
        let scroll_offset_x = self.tabs[self.active_tab].scroll_offset_x();
        let relative_x = (x - layout::PADDING * self.scale + scroll_offset_x).max(0.0);
        let clicked_col = (relative_x / char_width).round() as usize;

        self.tabs[self.active_tab].set_cursor_position(clicked_line, clicked_col, selecting);

        if selecting {
            self.auto_scroll();
        }

        self.state.reset_cursor_blink();
        AppResult::Redraw
    }

    pub fn handle_double_click(&mut self, x: f32, y: f32) -> AppResult {
        let _ = self.click_at(x, y, false);
        self.tabs[self.active_tab].select_word_at_cursor();
        AppResult::Redraw
    }

    pub fn handle_triple_click(&mut self, x: f32, y: f32) -> AppResult {
        let _ = self.click_at(x, y, false);
        self.tabs[self.active_tab].select_line_at_cursor();
        AppResult::Redraw
    }

    pub fn right_click_at(&mut self, x: f32, y: f32) -> AppResult {
        println!("right_click_at: ({}, {}) scale={}", x, y, self.scale);
        if y >= layout::TAB_HEIGHT * self.scale {
            println!("Click below tab bar");
            return AppResult::Ok;
        }

        let tab_info: Vec<(&str, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| (t.title(), i == self.active_tab))
            .collect();

        match self.renderer.hit_test(x, y, &tab_info) {
            Some(HitTestResult::Tab(i)) => {
                println!("Hit tab {}", i);
                self.start_rename(i);
                AppResult::Redraw
            }
            r => {
                println!("Hit test result: {:?}", r);
                AppResult::Ok
            }
        }
    }

    pub fn drag_at(&mut self, x: f32, y: f32) -> AppResult {
        if y < layout::TAB_HEIGHT * self.scale {
            if let Some(drag_index) = self.state.dragging_tab_index {
                return self.reorder_tab_at(x, y, drag_index);
            }
        }
        if self.state.is_dragging_scrollbar {
            let start_y = layout::TAB_HEIGHT * self.scale;
            let scroll_area_height = self.height - layout::TAB_HEIGHT * self.scale;

            let relative_y = (y - start_y).clamp(0.0, scroll_area_height);
            let scroll_ratio = relative_y / scroll_area_height;

            let total_lines = self.tabs[self.active_tab].total_lines();
            let visible_lines = self.visible_lines();

            let max_scroll = total_lines.saturating_sub(visible_lines);
            let scroll_offset = (scroll_ratio * max_scroll as f32).round() as usize;

            if self.tabs[self.active_tab].set_scroll_offset(scroll_offset) {
                return AppResult::Redraw;
            }
            return AppResult::Ok;
        }

        self.click_at(x, y, true)
    }

    pub fn end_drag(&mut self) {
        self.state.dragging_tab_index = None;
        self.state.is_dragging_scrollbar = false;
    }

    fn reorder_tab_at(&mut self, x: f32, y: f32, from_index: usize) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }

        let tab_info: Vec<(&str, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| (t.title(), i == self.active_tab))
            .collect();

        if let Some(HitTestResult::Tab(to_index)) = self.renderer.hit_test(x, y, &tab_info) {
            if to_index != from_index && from_index < self.tabs.len() && to_index < self.tabs.len()
            {
                let tab = self.tabs.remove(from_index);
                self.tabs.insert(to_index, tab);

                if self.active_tab == from_index {
                    self.active_tab = to_index;
                } else if from_index < self.active_tab && to_index >= self.active_tab {
                    self.active_tab = self.active_tab.saturating_sub(1);
                } else if from_index > self.active_tab && to_index <= self.active_tab {
                    self.active_tab = (self.active_tab + 1).min(self.tabs.len() - 1);
                }

                if let Some(rename_index) = self.state.renaming_tab {
                    self.state.renaming_tab = if rename_index == from_index {
                        Some(to_index)
                    } else if from_index < rename_index && to_index >= rename_index {
                        Some(rename_index - 1)
                    } else if from_index > rename_index && to_index <= rename_index {
                        Some(rename_index + 1)
                    } else {
                        Some(rename_index)
                    };
                }

                self.state.dragging_tab_index = Some(to_index);
                return AppResult::Redraw;
            }
        }

        AppResult::Ok
    }

    // =========================================================================
    // Tab management
    // =========================================================================

    pub fn new_tab(&mut self) -> AppResult {
        self.tabs.push(Tab::new_untitled());
        self.active_tab = self.tabs.len() - 1;
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn close_current_tab(&mut self) -> AppResult {
        if self.tabs.len() <= 1 {
            return AppResult::Ok;
        }
        self.tabs.remove(self.active_tab);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn next_tab(&mut self) -> AppResult {
        if self.tabs.is_empty() {
            return AppResult::Ok;
        }
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn previous_tab(&mut self) -> AppResult {
        if self.tabs.is_empty() {
            return AppResult::Ok;
        }
        if self.active_tab == 0 {
            self.active_tab = self.tabs.len() - 1;
        } else {
            self.active_tab -= 1;
        }
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn go_to_tab(&mut self, index: usize) -> AppResult {
        if index >= self.tabs.len() {
            return AppResult::Ok;
        }
        self.active_tab = index;
        self.auto_scroll();
        AppResult::Redraw
    }

    // =========================================================================
    // Text editing
    // =========================================================================

    pub fn handle_char(&mut self, ch: char) -> AppResult {
        if self.state.renaming_tab.is_some() {
            if !ch.is_control() {
                self.state.rename_buffer.push(ch);
                return AppResult::Redraw;
            }
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].insert_char(ch);
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_backspace(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            self.state.rename_buffer.pop();
            return AppResult::Redraw;
        }
        self.tabs[self.active_tab].backspace();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_delete_word_left(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            self.state.rename_buffer.clear();
            return AppResult::Redraw;
        }
        self.tabs[self.active_tab].delete_word_left();
        self.tabs[self.active_tab].auto_save();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn handle_delete(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            self.state.rename_buffer.clear();
            return AppResult::Redraw;
        }
        self.tabs[self.active_tab].delete();
        self.auto_scroll();
        AppResult::Redraw
    }

    // =========================================================================
    // Cursor movement
    // =========================================================================

    pub fn move_cursor_left(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_left(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_right(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_right(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_word_left(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_word_left(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_word_right(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_word_right(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_up(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_up(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_down(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_down(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_line_start(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_to_line_start(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_line_end(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_to_line_end(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_start(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_to_start(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn move_cursor_to_end(&mut self, selecting: bool) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].move_to_end(selecting);
        self.auto_scroll();
        AppResult::Redraw
    }

    // =========================================================================
    // Scrolling
    // =========================================================================

    pub fn scroll_up(&mut self) -> AppResult {
        if self.state.last_mouse_y < layout::TAB_HEIGHT * self.scale {
            self.state.tab_scroll_x =
                (self.state.tab_scroll_x - layout::TAB_PADDING * 2.0 * self.scale).max(0.0);
            self.renderer.set_tab_scroll_x(self.state.tab_scroll_x);
            return AppResult::Redraw;
        }
        self.tabs[self.active_tab].scroll_up(crate::config::scroll::LINES_PER_WHEEL_TICK);
        AppResult::Redraw
    }

    pub fn scroll_down(&mut self) -> AppResult {
        if self.state.last_mouse_y < layout::TAB_HEIGHT * self.scale {
            let max_scroll = 1000.0; // TODO: Calculate based on tabs width
            self.state.tab_scroll_x =
                (self.state.tab_scroll_x + layout::TAB_PADDING * 2.0 * self.scale).min(max_scroll);
            self.renderer.set_tab_scroll_x(self.state.tab_scroll_x);
            return AppResult::Redraw;
        }
        let visible = self.visible_lines();
        self.tabs[self.active_tab]
            .scroll_down(crate::config::scroll::LINES_PER_WHEEL_TICK, visible);
        AppResult::Redraw
    }

    // =========================================================================
    // File operations
    // =========================================================================

    pub fn save_current(&mut self) -> AppResult {
        self.tabs[self.active_tab].save();
        AppResult::Redraw
    }

    pub fn open_file(&mut self) -> AppResult {
        if let Some(tab) = Tab::open() {
            self.tabs.push(tab);
            self.active_tab = self.tabs.len() - 1;
            self.auto_scroll();
            AppResult::Redraw
        } else {
            AppResult::Ok
        }
    }

    pub fn rename_current(&mut self) -> AppResult {
        self.start_rename(self.active_tab);
        AppResult::Redraw
    }

    // =========================================================================
    // Clipboard operations
    // =========================================================================

    pub fn handle_copy(&mut self) -> AppResult {
        if let Some(text) = self.tabs[self.active_tab].copy_selection() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
        }
        AppResult::Ok
    }

    pub fn handle_cut(&mut self) -> AppResult {
        if let Some(text) = self.tabs[self.active_tab].cut_selection() {
            if let Some(clipboard) = &mut self.clipboard {
                let _ = clipboard.set_text(text);
            }
            self.tabs[self.active_tab].auto_save();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn handle_paste(&mut self) -> AppResult {
        if let Some(clipboard) = &mut self.clipboard {
            if let Ok(text) = clipboard.get_text() {
                self.tabs[self.active_tab].paste_text(&text);
                self.tabs[self.active_tab].auto_save();
                self.auto_scroll();
                return AppResult::Redraw;
            }
        }
        AppResult::Ok
    }

    pub fn handle_select_all(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].select_all();
        AppResult::Redraw
    }

    // =========================================================================
    // Line operations
    // =========================================================================

    pub fn handle_move_lines_up(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        if self.tabs[self.active_tab].move_lines_up() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn handle_move_lines_down(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        if self.tabs[self.active_tab].move_lines_down() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    // =========================================================================
    // Undo/Redo
    // =========================================================================

    pub fn handle_undo(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        if self.tabs[self.active_tab].undo() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn handle_redo(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        if self.tabs[self.active_tab].redo() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    // =========================================================================
    // View settings
    // =========================================================================

    pub fn toggle_word_wrap(&mut self) -> AppResult {
        if self.state.renaming_tab.is_some() {
            return AppResult::Ok;
        }
        self.tabs[self.active_tab].toggle_word_wrap();
        self.auto_scroll();
        AppResult::Redraw
    }

    pub fn start_rename(&mut self, tab_index: usize) {
        println!("start_rename: tab_index={}", tab_index);
        if let Some(tab) = self.tabs.get(tab_index) {
            self.state.renaming_tab = Some(tab_index);
            self.state.rename_buffer = tab.title().to_string();
            println!(
                "renaming_tab set to {:?}, buffer='{}'",
                self.state.renaming_tab, self.state.rename_buffer
            );
        }
    }

    pub fn confirm_rename(&mut self) -> AppResult {
        if let Some(tab_index) = self.state.renaming_tab.take() {
            let title = self.state.rename_buffer.trim();
            if !title.is_empty() {
                if let Some(tab) = self.tabs.get_mut(tab_index) {
                    tab.set_title(title.to_string());
                }
            }
            self.state.rename_buffer.clear();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    pub fn cancel_rename(&mut self) -> AppResult {
        if self.state.renaming_tab.take().is_some() {
            self.state.rename_buffer.clear();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

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
