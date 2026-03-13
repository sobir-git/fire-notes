//! Pure application logic — no GPU, no window, no file dialogs.
//!
//! `AppLogic` owns all document state and can be instantiated in tests with
//! `AppLogic::new_headless()`.  The platform shell (`App` in `app/mod.rs`)
//! wraps this and adds the renderer and clipboard.
//!
//! # Testability contract
//! - No `femtovg` / `glutin` / `winit` imports anywhere in this module tree.
//! - `AppLogic::new_headless()` must never touch the filesystem or OS.
//! - `AppLogic::render()` returns a pure `Node` tree — assert on Tab state
//!   directly in tests (no rendering required).

mod actions;   // src/logic/actions/mod.rs

#[cfg(test)]
mod tests;

use std::sync::mpsc;

use crate::app::action::Action;
use crate::app::focus::Focus;
use crate::app::scroll_state::{ScrollDirection, ScrollInput, ScrollState};
use crate::app::state::AppResult;
use crate::config::{self, layout, timing};
use crate::layout::{FrameworkEvent, InlineResult, InlineWidget, Overlay, OverlayResult};
#[allow(unused_imports)] use InlineResult as _;
use crate::tab::Tab;
use crate::theme::Theme;
use crate::ui::{UiTree, WindowRect};

/// Events that background tasks can inject into the UI event loop.
#[derive(Debug)]
#[allow(dead_code)]
pub enum AppEvent {
    /// Force a redraw (e.g. after an animation step or external data change).
    Redraw,
    /// An AI-generated token to append to the active ghost text buffer.
    AiToken(String),
    /// Ghost text suggestion ready — replaces any existing ghost text.
    GhostText(String),
    /// Clear ghost text.
    ClearGhostText,
}

pub struct AppLogic {
    pub(crate) tabs: Vec<Tab>,
    pub(crate) active_tab: usize,

    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) scale: f32,

    /// Char width hint supplied by the renderer after font load.
    /// Falls back to a reasonable default in headless/test mode.
    pub(crate) char_width_hint: f32,

    pub(crate) focus: Focus,
    /// Active theme — passed into overlay.render().
    pub(crate) theme: Theme,
    /// Active overlay (picker, dialog…) — `Some` while open.
    pub(crate) overlay: Option<Box<dyn Overlay>>,
    /// Active inline widget (tab rename, …) — `(tab_index, widget)`.
    pub(crate) inline: Option<(usize, Box<dyn InlineWidget>)>,
    /// Retained UI tree — persists across frames so hover state survives.
    pub(crate) ui_tree: UiTree,
    pub(crate) scroll_state: ScrollState,

    // ── Cursor blink ──────────────────────────────────────────────────────────
    pub(crate) cursor_visible: bool,
    pub(crate) last_cursor_blink: std::time::Instant,

    // ── Hover / cursor shape ──────────────────────────────────────────────────
    pub(crate) hovered_resize_edge: Option<crate::ui::ResizeEdge>,
    pub(crate) cursor_shape: crate::ui::CursorShape,

    // ── Mouse interaction ─────────────────────────────────────────────────────
    pub(crate) last_drag_scroll: std::time::Instant,
    pub(crate) last_mouse_x: f32,
    pub(crate) last_mouse_y: f32,

    // ── Tab bar scroll ────────────────────────────────────────────────────────
    pub(crate) tab_scroll_x: f32,

    // ── Flame effect ─────────────────────────────────────────────────────────
    pub(crate) typing_flame_positions: Vec<(usize, usize, std::time::Instant)>,

    // ── Async event channel ───────────────────────────────────────────────────
    /// Sender half — clone and pass to background tasks.
    #[allow(dead_code)]
    pub(crate) event_tx: mpsc::Sender<AppEvent>,
    /// Receiver half — polled in the platform event loop's `about_to_wait`.
    pub(crate) event_rx: mpsc::Receiver<AppEvent>,

    // ── Ghost text (AI inline suggestion) ────────────────────────────────────
    pub(crate) ghost_text: String,
}

impl AppLogic {
    /// Create a fully isolated, headless instance for testing.
    /// Never touches the filesystem or OS.
    #[allow(dead_code)]
    pub fn new_headless(width: f32, height: f32, scale: f32) -> Self {
        let now = std::time::Instant::now();
        let (event_tx, event_rx) = mpsc::channel();
        Self {
            tabs: vec![Tab::new_untitled()],
            active_tab: 0,
            width,
            height,
            scale,
            char_width_hint: crate::config::rendering::FALLBACK_CHAR_WIDTH,
            focus: Focus::default(),
            theme: Theme::dark(),
            overlay: None,
            inline: None,
            ui_tree: UiTree::layout(WindowRect::new(width, height), scale),
            scroll_state: ScrollState::new(),
            cursor_visible: true,
            last_cursor_blink: now,
            hovered_resize_edge: None,
            cursor_shape: crate::ui::CursorShape::Default,
            last_drag_scroll: now,
            last_mouse_x: 0.0,
            last_mouse_y: 0.0,
            tab_scroll_x: 0.0,
            typing_flame_positions: Vec::new(),
            event_tx,
            event_rx,
            ghost_text: String::new(),
        }
    }

    /// Create from persisted session (used by the platform shell on startup).
    pub fn new_from_session(
        tabs: Vec<Tab>,
        active_tab: usize,
        width: f32,
        height: f32,
        scale: f32,
    ) -> Self {
        let now = std::time::Instant::now();
        let (event_tx, event_rx) = mpsc::channel();
        let mut logic = Self {
            tabs,
            active_tab,
            width,
            height,
            scale,
            char_width_hint: crate::config::rendering::FALLBACK_CHAR_WIDTH,
            focus: Focus::default(),
            theme: Theme::dark(),
            overlay: None,
            inline: None,
            ui_tree: UiTree::layout(WindowRect::new(width, height), scale),
            scroll_state: ScrollState::new(),
            cursor_visible: true,
            last_cursor_blink: now,
            hovered_resize_edge: None,
            cursor_shape: crate::ui::CursorShape::Default,
            last_drag_scroll: now,
            last_mouse_x: 0.0,
            last_mouse_y: 0.0,
            tab_scroll_x: 0.0,
            typing_flame_positions: Vec::new(),
            event_tx,
            event_rx,
            ghost_text: String::new(),
        };
        if logic.tabs.is_empty() {
            logic.tabs.push(Tab::new_untitled());
        }
        logic
    }

    // =========================================================================
    // Core lifecycle
    // =========================================================================

    /// Returns a `Sender` that background tasks can use to inject events.
    #[allow(dead_code)]
    pub fn event_sender(&self) -> mpsc::Sender<AppEvent> {
        self.event_tx.clone()
    }

    /// Drain all pending async events. Returns `true` if a redraw is needed.
    pub fn poll_events(&mut self) -> bool {
        let mut redraw = false;
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                AppEvent::Redraw => { redraw = true; }
                AppEvent::AiToken(token) => { self.ghost_text.push_str(&token); redraw = true; }
                AppEvent::GhostText(text) => { self.ghost_text = text; redraw = true; }
                AppEvent::ClearGhostText  => { self.ghost_text.clear(); redraw = true; }
            }
        }
        redraw
    }

    /// Reset cursor blink (call after user action)
    pub(crate) fn reset_cursor_blink(&mut self) {
        self.cursor_visible = true;
        self.last_cursor_blink = std::time::Instant::now();
    }

    /// Advance internal timers. Returns whether a redraw is needed.
    pub fn tick(&mut self) -> AppResult {
        let mut needs_redraw = false;
        if self.last_cursor_blink.elapsed().as_millis() >= timing::CURSOR_BLINK_MS as u128 {
            self.cursor_visible = !self.cursor_visible;
            self.last_cursor_blink = std::time::Instant::now();
            needs_redraw = true;
        }
        let had_flames = !self.typing_flame_positions.is_empty();
        let now = std::time::Instant::now();
        self.typing_flame_positions.retain(|(_, _, ts)| {
            now.duration_since(*ts).as_secs_f32() < config::flame::TYPING_FLAME_EXPIRY
        });
        if had_flames { needs_redraw = true; }
        if needs_redraw { AppResult::Redraw } else { AppResult::Ok }
    }

    pub fn resize(&mut self, width: f32, height: f32, scale: f32) {
        self.width = width;
        self.height = height;
        self.scale = scale;
        self.ui_tree = UiTree::layout(WindowRect::new(width, height), scale);
    }

    // =========================================================================
    // Execute actions
    // =========================================================================

    /// Execute any `Action` and return whether a redraw is needed.
    pub fn execute(&mut self, action: Action) -> AppResult {
        match action {
            Action::NewTab => self.new_tab(),
            Action::CloseTab => self.close_current_tab(),
            Action::NextTab => self.next_tab(),
            Action::PreviousTab => self.previous_tab(),
            Action::GoToTab(index) => self.go_to_tab(index),

            Action::Save => AppResult::Ok,
            Action::OpenFile => AppResult::Ok,
            Action::RenameTab => self.rename_current(),

            Action::OpenNotesPicker => AppResult::Ok,
            Action::ConfirmNotesPicker => self.confirm_overlay(),
            Action::CancelNotesPicker => self.cancel_overlay(),
            Action::OpenSlashMenu => self.open_slash_menu(),

            Action::Undo => self.handle_undo(),
            Action::Redo => self.handle_redo(),
            Action::Copy => AppResult::Ok,
            Action::Cut => AppResult::Ok,
            Action::Paste => AppResult::Ok,
            Action::SelectAll => self.handle_select_all(),
            Action::DeleteWordLeft => self.handle_delete_word_left(),
            Action::DeleteWordRight => self.handle_delete_word_right(),
            Action::Delete => self.handle_delete(),
            Action::Backspace => self.handle_backspace(),

            Action::CursorLeft { selecting } => self.move_cursor_left(selecting),
            Action::CursorRight { selecting } => self.move_cursor_right(selecting),
            Action::CursorUp { selecting } => self.move_cursor_up(selecting),
            Action::CursorDown { selecting } => self.move_cursor_down(selecting),
            Action::CursorWordLeft { selecting } => self.move_cursor_word_left(selecting),
            Action::CursorWordRight { selecting } => self.move_cursor_word_right(selecting),
            Action::CursorLineStart { selecting } => self.move_cursor_to_line_start(selecting),
            Action::CursorLineEnd { selecting } => self.move_cursor_to_line_end(selecting),
            Action::CursorDocStart { selecting } => self.move_cursor_to_start(selecting),
            Action::CursorDocEnd { selecting } => self.move_cursor_to_end(selecting),
            Action::PageUp { selecting } => self.page_up(selecting),
            Action::PageDown { selecting } => self.page_down(selecting),

            Action::MoveLinesUp => self.handle_move_lines_up(),
            Action::MoveLinesDown => self.handle_move_lines_down(),

            Action::ToggleWordWrap => self.toggle_word_wrap(),

            Action::Cancel => {
                let r = self.cancel_overlay();
                if r.needs_redraw() { return r; }
                if matches!(self.focus, Focus::TabRename) {
                    return self.dispatch_inline(FrameworkEvent::Key(crate::app::Key::Escape));
                }
                AppResult::Ok
            }
            Action::Confirm => {
                let r = self.confirm_overlay();
                if r.needs_redraw() { return r; }
                if matches!(self.focus, Focus::TabRename) {
                    return self.dispatch_inline(FrameworkEvent::Key(crate::app::Key::Enter));
                }
                self.handle_char('\n')
            }

            Action::InsertChar(ch) => self.handle_char(ch),
        }
    }

    /// Open an overlay. The caller constructs the concrete type and boxes it.
    pub fn open_overlay_with(&mut self, overlay: Box<dyn Overlay>) -> AppResult {
        self.overlay = Some(overlay);
        self.focus   = Focus::open_overlay();
        AppResult::Redraw
    }

    pub fn cancel_overlay(&mut self) -> AppResult {
        if self.focus.close_overlay() {
            self.overlay = None;
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    /// Dispatch a FrameworkEvent to the active overlay and translate OverlayResult → AppResult.
    pub fn dispatch_overlay(&mut self, event: FrameworkEvent) -> AppResult {
        let result = if let Some(o) = &mut self.overlay {
            o.on_event(event)
        } else {
            return AppResult::Ok;
        };
        self.handle_overlay_result(result)
    }

    pub(crate) fn handle_overlay_result(&mut self, result: OverlayResult) -> AppResult {
        match result {
            OverlayResult::Nothing  => AppResult::Ok,
            OverlayResult::Redraw   => AppResult::Redraw,
            OverlayResult::Close    => self.cancel_overlay(),
            OverlayResult::OpenPath(path) => {
                let _ = self.cancel_overlay();
                self.open_or_switch_to(path)
            }
        }
    }

    fn confirm_overlay(&mut self) -> AppResult {
        let result = if let Some(o) = &mut self.overlay {
            o.on_event(FrameworkEvent::Key(crate::app::Key::Enter))
        } else {
            return AppResult::Ok;
        };
        self.handle_overlay_result(result)
    }

    /// Dispatch a FrameworkEvent to the active inline widget and handle the result.
    pub(crate) fn dispatch_inline(&mut self, event: FrameworkEvent) -> AppResult {
        let result = if let Some((_, w)) = &mut self.inline {
            w.on_event(event)
        } else {
            return AppResult::Ok;
        };
        self.handle_inline_result(result)
    }

    pub(crate) fn handle_inline_result(&mut self, result: InlineResult) -> AppResult {
        match result {
            InlineResult::Nothing => AppResult::Ok,
            InlineResult::Redraw  => { self.reset_cursor_blink(); AppResult::Redraw }
            InlineResult::Cancel  => {
                self.inline = None;
                self.focus  = Focus::Editor;
                AppResult::Redraw
            }
            InlineResult::Commit(title) => {
                if let Some((tab_index, _)) = self.inline.take() {
                    if !title.is_empty() {
                        if let Some(tab) = self.tabs.get_mut(tab_index) {
                            tab.set_title(title);
                        }
                    }
                }
                self.focus = Focus::Editor;
                AppResult::Redraw
            }
        }
    }

    /// Insert text from clipboard (called by platform shell after reading clipboard).
    pub fn insert_paste_text(&mut self, text: &str) -> AppResult {
        if matches!(self.focus, Focus::TabRename) {
            return AppResult::Ok;
        }
        if !text.is_empty() {
            self.tabs[self.active_tab].paste_text(text);
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
            return AppResult::Redraw;
        }
        AppResult::Ok
    }

    /// Copy current selection. Returns text for the platform shell to write to clipboard.
    pub fn copy_selection(&self) -> Option<String> {
        if matches!(self.focus, Focus::TabRename) {
            return None; // inline widget doesn't expose copy yet
        }
        self.tabs[self.active_tab].copy_selection()
    }

    /// Cut current selection. Returns text for the clipboard.
    pub fn cut_selection(&mut self) -> Option<String> {
        if matches!(self.focus, Focus::TabRename) {
            return None; // inline widget doesn't expose cut yet
        }
        let text = self.tabs[self.active_tab].cut_selection();
        if text.is_some() {
            self.tabs[self.active_tab].auto_save();
            self.auto_scroll();
        }
        text
    }

    // =========================================================================
    // Layout helpers
    // =========================================================================

    pub fn visible_line_count(&self) -> usize {
        use crate::ui::{WindowRect, UiTree};
        let window = WindowRect::new(self.width, self.height);
        UiTree::layout(window, self.scale).content_area.visible_line_count()
    }

    pub fn auto_scroll(&mut self) {
        let visible = self.visible_line_count();
        let visible_width = self.width - layout::PADDING * 2.0 * self.scale;
        self.tabs[self.active_tab].ensure_cursor_visible(visible, visible_width, self.char_width_hint);
        self.reset_cursor_blink();
    }

    /// Update the char width hint (called by platform shell after font load).
    pub fn set_char_width_hint(&mut self, width: f32) {
        self.char_width_hint = width;
    }

    // =========================================================================
    // Command palette
    // =========================================================================

    fn open_slash_menu(&mut self) -> AppResult {
        use crate::components::slash_menu::{SlashCommand, SlashMenu};
        use crate::ui::{Rect, WindowRect};

        let commands = vec![
            SlashCommand { name: "New Tab".into(),      description: "Open a new empty tab".into() },
            SlashCommand { name: "Save".into(),         description: "Save current file".into() },
            SlashCommand { name: "Toggle Word Wrap".into(), description: "Toggle word wrap mode".into() },
            SlashCommand { name: "Close Tab".into(),    description: "Close the active tab".into() },
        ];

        let window: Rect = WindowRect::new(self.width, self.height).into();
        let content = self.ui_tree.slot("content");
        let tab = &self.tabs[self.active_tab];
        let cursor_line = tab.cursor_line();
        let cursor_col  = tab.cursor_col();
        let scroll      = tab.scroll_offset();
        let lh          = crate::config::layout::LINE_HEIGHT * self.scale;
        let cw          = self.char_width_hint.max(8.0);
        let padding     = crate::config::layout::PADDING * self.scale;
        let anchor = Rect {
            x:      content.x + padding + cursor_col as f32 * cw,
            y:      content.y + (cursor_line.saturating_sub(scroll)) as f32 * lh,
            width:  cw,
            height: lh,
        };

        let menu = SlashMenu::new(commands, anchor, window, self.scale);
        self.open_overlay_with(Box::new(menu))
    }

    // =========================================================================
    // Scroll
    // =========================================================================

    pub fn handle_scroll_event(&mut self, input: ScrollInput) -> AppResult {
        let Some((direction, lines)) = self.scroll_state.process_scroll(input) else {
            return AppResult::Ok;
        };
        match direction {
            ScrollDirection::Up => {
                for _ in 0..lines { self.tabs[self.active_tab].scroll_up(1); }
            }
            ScrollDirection::Down => {
                let visible = self.visible_line_count();
                for _ in 0..lines { self.tabs[self.active_tab].scroll_down(1, visible); }
            }
        }
        AppResult::Redraw
    }

    pub fn scroll_tab_bar(&mut self, delta: f32) -> AppResult {
        if delta > 0.0 {
            self.tab_scroll_x = (self.tab_scroll_x - delta.abs()).max(0.0);
        } else {
            self.tab_scroll_x = (self.tab_scroll_x + delta.abs()).min(1000.0);
        }
        AppResult::Redraw
    }

    pub fn reset_scroll_state(&mut self) {
        self.scroll_state.reset();
    }

    // =========================================================================
    // Mouse helpers
    // =========================================================================

    pub fn is_mouse_in_tab_bar(&self) -> bool {
        self.last_mouse_y < layout::TAB_HEIGHT * self.scale
    }

    /// Sync the retained `ui_tree` from the current tab state.
    ///
    /// Call before any hit-test or render that needs up-to-date geometry.
    /// Preserves all hover and drag state owned by widgets.
    pub fn prepare_ui_tree(&mut self) {
        let tab_scroll_x = self.tab_scroll_x;
        let titles: Vec<(String, bool)> = self.tabs.iter().enumerate()
            .map(|(i, t)| (t.title().to_string(), i == self.active_tab))
            .collect();
        let tab_info: Vec<(&str, bool)> = titles.iter()
            .map(|(s, a)| (s.as_str(), *a))
            .collect();
        self.ui_tree.relayout_tabs(tab_scroll_x, &tab_info);
    }

    /// Sync the retained ui_tree's tab layout from caller-supplied titles.
    ///
    /// Prefer `prepare_ui_tree` when no external title vec is needed.
    #[allow(dead_code)]
    pub fn sync_ui_tree(&mut self, tab_titles: &[(&str, bool)]) {
        self.ui_tree.relayout_tabs(self.tab_scroll_x, tab_titles);
    }

    /// Update hover state on the retained ui_tree. Returns `(changed, cursor, resize_edge)`.
    pub fn handle_hover(
        &mut self,
        x: f32,
        y: f32,
    ) -> (bool, crate::ui::CursorShape, Option<crate::ui::ResizeEdge>) {
        self.prepare_ui_tree();
        let total_lines   = self.tabs[self.active_tab].total_lines();
        let visible_lines = self.visible_line_count();
        let scroll_offset = self.tabs[self.active_tab].scroll_offset();
        self.ui_tree.on_hover(x, y, total_lines, visible_lines, scroll_offset)
    }

    #[allow(dead_code)]
    pub fn tab_scroll_x(&self) -> f32 { self.tab_scroll_x }

    // =========================================================================
    // Session export
    // =========================================================================

    pub fn export_session_state(&self) -> crate::persistence::SessionState {
        let active_path = self.tabs.get(self.active_tab).and_then(|t| t.path().cloned());
        let tabs = self.tabs.iter().filter_map(|t| t.export_state()).collect();
        crate::persistence::SessionState { active_path, tabs }
    }

    // =========================================================================
    // Render — single Node tree, replaces render_frame() + snapshot.rs
    // =========================================================================

    /// Produce the full UI scene as a `Node` tree.
    ///
    /// This is the **only** render path. The renderer walks this tree and emits
    /// draw calls. No `RenderFrame`, no snapshot bridging, no per-subsystem
    /// baking methods.
    pub fn render(&mut self) -> crate::layout::Node {
        use crate::layout::Node;
        use crate::layout::node::{
            EditorCursor, EditorNode, EditorSelection, FlamePos,
            RenameOverlayNode, ScrollbarNode, TabBarNode, TabEntry,
            TextLine, WinButton, WinButtonKind,
        };
        use crate::layout::node::Color as NColor;

        self.prepare_ui_tree();

        // ── Geometry from ui_tree ─────────────────────────────────────────
        let tb      = &self.ui_tree.tab_bar;
        let ca      = &self.ui_tree.content_area;
        let scale   = self.scale;

        // ── Tab bar node ──────────────────────────────────────────────────
        let renaming_tab = self.inline.as_ref().map(|(idx, _)| *idx);

        let tab_bar_font_size = crate::config::rendering::TAB_FONT_SIZE * scale;
        let tab_bar_baseline  = tb.rect.y + tb.rect.height / 2.0 + tab_bar_font_size * 0.35;

        let tabs: Vec<TabEntry> = tb.scroll_area.tabs.iter().map(|tm| {
            let t = &self.tabs[tm.index];
            let title = if Some(tm.index) == renaming_tab {
                self.inline.as_ref()
                    .and_then(|(_, w)| w.as_any()
                        .downcast_ref::<crate::components::tab_rename::TabRename>())
                    .map(|tr| tr.input.text().to_string())
                    .unwrap_or_else(|| t.title().to_string())
            } else {
                t.title().to_string()
            };
            TabEntry {
                title,
                rect: tm.rect,
                is_active:  tm.index == self.active_tab,
                is_hovered: tb.hovered_tab_index == Some(tm.index),
            }
        }).collect();

        let rename = self.inline.as_ref().and_then(|(tab_index, w)| {
            let tr = w.as_any().downcast_ref::<crate::components::tab_rename::TabRename>()?;
            // Find the tab rect for text_x/text_y
            let tm = tb.scroll_area.tabs.iter().find(|t| t.index == *tab_index)?;
            Some(RenameOverlayNode {
                tab_index:      *tab_index,
                text:           tr.input.text().to_string(),
                cursor:         tr.input.state.cursor,
                cursor_visible: self.cursor_visible,
                text_x:         tm.rect.x + tm.rect.width / 2.0, // centering; renderer measures
                text_y:         tab_bar_baseline,
            })
        });

        let win_buttons = vec![
            WinButton { rect: tb.close_rect,    hovered: tb.hovered_close,    kind: WinButtonKind::Close },
            WinButton { rect: tb.maximize_rect, hovered: tb.hovered_maximize, kind: WinButtonKind::Maximize },
            WinButton { rect: tb.minimize_rect, hovered: tb.hovered_minimize, kind: WinButtonKind::Minimize },
        ];

        let tab_bar_node = Node::TabBar(TabBarNode {
            rect:            tb.rect,
            tabs_clip_x:     tb.tabs_clip_x,
            tabs,
            new_tab_rect:    tb.new_tab_rect,
            new_tab_hovered: tb.hovered_plus,
            win_buttons,
            rename,
        });

        // ── Editor node ───────────────────────────────────────────────────
        let current_tab   = &self.tabs[self.active_tab];
        let scroll_offset = current_tab.scroll_offset();
        let total_lines   = current_tab.total_lines();
        let visible_count = ca.visible_line_count();
        let scroll_x      = current_tab.scroll_offset_x();
        let line_height   = ca.text.line_height;
        let text_padding  = ca.text.text_padding;
        let doc_top_margin = ca.text.doc_top_margin;
        let char_width    = self.char_width_hint;

        let lines: Vec<TextLine> = current_tab.content().lines()
            .enumerate()
            .skip(scroll_offset)
            .take(visible_count)
            .map(|(idx, text)| {
                let visual = idx - scroll_offset;
                let y = ca.text.rect.y + doc_top_margin + visual as f32 * line_height;
                TextLine { line_index: idx, text: text.to_string(), y }
            })
            .collect();

        // Cursor
        let cursor = {
            let cl = current_tab.cursor_line();
            let cc = current_tab.cursor_col();
            let (cx, cy) = if cl >= scroll_offset {
                let visual = cl - scroll_offset;
                let y = ca.text.rect.y + doc_top_margin + visual as f32 * line_height;
                let line_text = current_tab.content().lines().nth(cl).unwrap_or("");
                let vl = crate::visual_position::VisualLine::new(line_text);
                let x  = vl.char_col_to_visual_x(cc, text_padding - scroll_x, char_width);
                (x, y)
            } else { (0.0, -999.0) };
            EditorCursor { x: cx, y: cy, line_height, visible: self.cursor_visible }
        };

        // Selection
        let selection = current_tab.selection_range_line_col().map(|(start, end)| {
            let (start_line, start_col) = start;
            let (end_line, end_col)     = end;
            let mut rects = Vec::new();
            for ld in lines.iter() {
                let li = ld.line_index;
                if li < start_line || li > end_line { continue; }
                let sc = if li == start_line { start_col } else { 0 };
                let line_text = &ld.text;
                let ec = if li == end_line { end_col.min(line_text.chars().count()) }
                         else              { line_text.chars().count() };
                if sc >= ec { continue; }
                let vl = crate::visual_position::VisualLine::new(line_text);
                let x1 = vl.char_col_to_visual_x(sc, text_padding - scroll_x, char_width);
                let x2 = vl.char_col_to_visual_x(ec, text_padding - scroll_x, char_width);
                let sel_rect = crate::ui::Rect { x: x1, y: ld.y, width: (x2 - x1).max(char_width), height: line_height };
                rects.push(sel_rect);
            }
            EditorSelection { rects }
        });

        // Scrollbar
        let scrollbar = if total_lines > visible_count && visible_count > 0 {
            let s = &ca.scrollbar;
            s.thumb(total_lines, visible_count, scroll_offset).map(|thumb| ScrollbarNode {
                track:       s.rect,
                thumb:       thumb.rect,
                track_color: NColor::rgba(0.0, 0.0, 0.0, 0.0),
                thumb_color: NColor::rgba(1.0, 1.0, 1.0,
                    if ca.scrollbar_hovered { 0.35 } else { 0.20 }),
            })
        } else { None };

        // Flames
        let now = std::time::Instant::now();
        let mut flames: Vec<FlamePos> = Vec::new();
        // Selection flames — one spawn point per character column across each selection rect
        let cw = char_width.max(1.0);
        for r in selection.as_ref().map(|s| s.rects.iter()).into_iter().flatten() {
            let bottom = r.y + r.height;
            let cy     = r.y + r.height * 0.5;
            let n_chars = (r.width / cw).ceil() as usize;
            for i in 0..n_chars.max(1) {
                let cx = r.x + (i as f32 + 0.5) * cw;
                flames.push(FlamePos { cx, cy, bottom, age: 0.0 });
            }
        }
        // Typing flames
        for &(line, col, timestamp) in &self.typing_flame_positions {
            if line < scroll_offset { continue; }
            let visual = line - scroll_offset;
            let y = ca.text.rect.y + doc_top_margin + visual as f32 * line_height;
            if y > ca.rect.y + ca.rect.height { continue; }
            let line_text = current_tab.content().lines().nth(line).unwrap_or("");
            let vl = crate::visual_position::VisualLine::new(line_text);
            let cx = vl.char_col_to_visual_center_x(col, text_padding - scroll_x, char_width);
            let age = now.duration_since(timestamp).as_secs_f32().min(1.0);
            flames.push(FlamePos { cx, cy: y + line_height * 0.5, bottom: y + line_height, age });
        }

        let editor_node = Node::Editor(EditorNode {
            rect:         ca.rect,
            text_rect:    ca.text.rect,
            line_height,
            text_padding,
            char_width,
            scroll_x,
            word_wrap:    current_tab.word_wrap(),
            lines,
            cursor,
            selection,
            scrollbar,
            flames,
        });

        // ── Overlay ───────────────────────────────────────────────────────
        let overlay = self.overlay.as_ref().map(|o| o.render(&self.theme));

        // ── Full scene ────────────────────────────────────────────────────
        let mut scene = vec![tab_bar_node, editor_node];
        if let Some(o) = overlay { scene.push(o); }
        Node::layer(scene)
    }
}

