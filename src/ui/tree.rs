//! UI tree — top-level layout compositor.
//!
//! `UiTree::layout(window_rect, scale)` cuts the window into tab bar and
//! content using `Rect` primitives, then delegates to each child's `Layout`
//! impl.  No coordinate arithmetic lives anywhere else.

use crate::config::layout as cfg_layout;
use super::content_area::ContentArea;
use super::layout::Layout;
use super::tab_bar::TabBar;
use crate::primitives::scrollbar::ScrollbarAction;
use super::types::{CursorShape, Rect, WindowRect, ResizeEdge, UiAction, UiDragAction, UiHover, UiNode};

const RESIZE_BORDER: f32 = 5.0;

pub struct UiTree {
    pub tab_bar: TabBar,
    /// Content area owns both `text: TextArea` and `scrollbar: ScrollbarWidget`.
    pub content_area: ContentArea,
    width: f32,
    height: f32,
    scale: f32,
}

impl UiTree {
    /// Lay out the full UI tree from the OS window dimensions.
    ///
    /// `window` can only be constructed by the platform layer — this is a
    /// compile-time guarantee that no child widget accidentally encodes
    /// parent-level geometry (e.g. knowing the tab bar height).
    pub fn layout(window: WindowRect, scale: f32) -> Self {
        let rect: Rect = window.into();
        let (tab_rect, content_rect) = rect.cut_top(cfg_layout::TAB_HEIGHT * scale);
        Self {
            tab_bar:      TabBar::layout(tab_rect, scale),
            content_area: ContentArea::layout(content_rect, scale),
            width:  rect.width,
            height: rect.height,
            scale,
        }
    }

    /// Rebuild tab geometry in-place, preserving all hover state owned by widgets.
    ///
    /// Prefer this over creating a new `UiTree` + manually copying hover fields.
    pub fn relayout_tabs(&mut self, tab_scroll_x: f32, tabs: &[(&str, bool)]) {
        self.tab_bar.relayout_in_place(self.width, self.scale, tab_scroll_x, tabs);
    }

    /// Window width in physical pixels (as laid out).
    #[allow(dead_code)]
    pub fn width(&self)  -> f32 { self.width }
    /// Window height in physical pixels (as laid out).
    #[allow(dead_code)]
    pub fn height(&self) -> f32 { self.height }

    // ── Resize edge detection ─────────────────────────────────────────────

    fn detect_resize_edge(&self, x: f32, y: f32) -> Option<ResizeEdge> {
        let border = RESIZE_BORDER * self.scale;
        let near_left = x < border;
        let near_right = x > self.width - border;
        let near_top = y < border;
        let near_bottom = y > self.height - border;

        match (near_left, near_right, near_top, near_bottom) {
            (true, _, true, _) => Some(ResizeEdge::NorthWest),
            (true, _, _, true) => Some(ResizeEdge::SouthWest),
            (_, true, true, _) => Some(ResizeEdge::NorthEast),
            (_, true, _, true) => Some(ResizeEdge::SouthEast),
            (true, _, _, _) => Some(ResizeEdge::West),
            (_, true, _, _) => Some(ResizeEdge::East),
            (_, _, true, _) => Some(ResizeEdge::North),
            (_, _, _, true) => Some(ResizeEdge::South),
            _ => None,
        }
    }

    // ── Public API ────────────────────────────────────────────────────────

    /// Update hover state on widget fields. Returns `(changed, cursor_shape, resize_edge)`.
    /// Call this when `UiTree` is retained across frames — widgets own their hover state.
    pub fn on_hover(
        &mut self,
        x: f32,
        y: f32,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
    ) -> (bool, CursorShape, Option<ResizeEdge>) {
        let tab_changed = self.tab_bar.on_hover(x, y);
        let sb_changed  = self.content_area.on_hover(x, y, total_lines, visible_lines);
        let changed = tab_changed || sb_changed;

        // Compute cursor shape and resize edge from the immutable hover result.
        let result = self.hover(x, y, total_lines, visible_lines, scroll_offset);
        (changed, result.cursor_shape, result.resize_edge)
    }

    pub fn hover(
        &self,
        x: f32,
        y: f32,
        total_lines: usize,
        visible_lines: usize,
        _scroll_offset: usize,
    ) -> UiHover {
        let mut hover = UiHover::default();

        if let Some(edge) = self.detect_resize_edge(x, y) {
            hover.resize_edge = Some(edge);
            hover.cursor_shape = match edge {
                ResizeEdge::North | ResizeEdge::South => CursorShape::NsResize,
                ResizeEdge::East  | ResizeEdge::West  => CursorShape::EwResize,
                ResizeEdge::NorthEast | ResizeEdge::SouthWest => CursorShape::NeswResize,
                ResizeEdge::NorthWest | ResizeEdge::SouthEast => CursorShape::NwseResize,
            };
            return hover;
        }

        match self.tab_bar.hit_test(x, y) {
            UiNode::Tab(i) => { hover.tab_index = Some(i); hover.cursor_shape = CursorShape::Pointer; }
            UiNode::NewTabButton => { hover.plus = true; hover.cursor_shape = CursorShape::Pointer; }
            UiNode::WindowMinimize => { hover.window_minimize = true; hover.cursor_shape = CursorShape::Default; }
            UiNode::WindowMaximize => { hover.window_maximize = true; hover.cursor_shape = CursorShape::Default; }
            UiNode::WindowClose => { hover.window_close = true; hover.cursor_shape = CursorShape::Default; }
            _ => { hover.cursor_shape = CursorShape::Default; }
        }

        let on_scrollbar = self.content_area.scrollbar.hit_test(x, y)
            && self.content_area.scrollbar.is_scrollable(total_lines, visible_lines);
        hover.scrollbar = on_scrollbar;
        if on_scrollbar {
            hover.cursor_shape = CursorShape::Default;
        } else if self.content_area.text.rect.contains(x, y) {
            hover.cursor_shape = CursorShape::Text;
        }
        hover
    }

    pub fn click(
        &mut self,
        x: f32,
        y: f32,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
        selecting: bool,
    ) -> UiAction {
        match self.hit_test(x, y) {
            UiNode::Tab(i) if !selecting => UiAction::ActivateTab(i),
            UiNode::NewTabButton if !selecting => UiAction::NewTab,
            UiNode::TabBar if !selecting => UiAction::WindowDrag,
            UiNode::WindowMinimize if !selecting => UiAction::WindowMinimize,
            UiNode::WindowMaximize if !selecting => UiAction::WindowMaximize,
            UiNode::WindowClose if !selecting => UiAction::WindowClose,
            UiNode::WindowResizeEdge(edge) if !selecting => UiAction::WindowResize(edge),
            UiNode::Scrollbar if !selecting => {
                match self.content_area.scrollbar.on_click(x, y, total_lines, visible_lines, scroll_offset) {
                    ScrollbarAction::StartDrag { drag_offset } =>
                        UiAction::StartScrollbarDrag { drag_offset },
                    ScrollbarAction::JumpTo { ratio } => UiAction::ScrollbarJump { ratio },
                    ScrollbarAction::None => UiAction::None,
                }
            }
            UiNode::TextArea => UiAction::TextClick,
            _ => UiAction::None,
        }
    }

    pub fn drag_scrollbar(
        &mut self,
        y: f32,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
    ) -> UiDragAction {
        if let Some(ratio) = self.content_area.scrollbar.continue_drag(y, total_lines, visible_lines, scroll_offset) {
            return UiDragAction::ScrollbarDrag { ratio };
        }
        UiDragAction::None
    }

    pub fn double_click(
        &mut self,
        x: f32,
        y: f32,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
    ) -> UiAction {
        self.multi_click(x, y, total_lines, visible_lines, scroll_offset)
    }

    pub fn triple_click(
        &mut self,
        x: f32,
        y: f32,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
    ) -> UiAction {
        self.multi_click(x, y, total_lines, visible_lines, scroll_offset)
    }

    fn multi_click(
        &mut self,
        x: f32,
        y: f32,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
    ) -> UiAction {
        match self.hit_test(x, y) {
            UiNode::Tab(_) | UiNode::NewTabButton | UiNode::TabBar
            | UiNode::WindowMinimize | UiNode::WindowMaximize | UiNode::WindowClose
            | UiNode::WindowResizeEdge(_) => {
                self.click(x, y, total_lines, visible_lines, scroll_offset, false)
            }
            UiNode::Scrollbar => UiAction::None,
            UiNode::TextArea => UiAction::TextClick,
            UiNode::None => UiAction::None,
        }
    }

    pub fn hit_test(&self, x: f32, y: f32) -> UiNode {
        if let Some(edge) = self.detect_resize_edge(x, y) {
            return UiNode::WindowResizeEdge(edge);
        }

        let tab_node = self.tab_bar.hit_test(x, y);
        if tab_node != UiNode::None {
            return tab_node;
        }

        if self.content_area.scrollbar.hit_test(x, y) {
            return UiNode::Scrollbar;
        }

        if self.content_area.text.hit_test(x, y) {
            return UiNode::TextArea;
        }

        UiNode::None
    }
}
