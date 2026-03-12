//! UI tree — top-level layout compositor.
//!
//! `UiTree::layout(window_rect, scale)` cuts the window into tab bar and
//! content using `Rect` primitives, then delegates to each child's `Layout`
//! impl.  No coordinate arithmetic lives anywhere else.

use crate::config::layout as cfg_layout;
use super::content_area::ContentArea;
use super::layout::Layout;
use super::notes_picker::NotesPicker;
use super::tab_bar::TabBar;
use super::scrollbar::ScrollbarAction;
use super::types::{Rect, ResizeEdge, UiAction, UiDragAction, UiHover, UiNode};

const RESIZE_BORDER: f32 = 5.0;

#[derive(Debug, Clone)]
pub struct UiTree {
    pub tab_bar: TabBar,
    /// Content area owns both `text: TextArea` and `scrollbar: ScrollbarWidget`.
    pub content_area: ContentArea,
    /// Notes picker overlay layout — `Some` when the picker is open.
    pub notes_picker: Option<NotesPicker>,
    width: f32,
    height: f32,
    scale: f32,
}

impl Layout for UiTree {
    fn layout(rect: Rect, scale: f32) -> Self {
        let (tab_rect, content_rect) = rect.cut_top(cfg_layout::TAB_HEIGHT * scale);
        let (_, content_rect)        = content_rect.cut_top(cfg_layout::PADDING * scale);
        Self {
            tab_bar:      TabBar::layout(tab_rect, scale),
            content_area: ContentArea::layout(content_rect, scale),
            notes_picker: None,
            width:  rect.width,
            height: rect.height,
            scale,
        }
    }
}

impl UiTree {
    /// Build from raw window dimensions + tab state.
    /// `picker_list_len` — `Some(n)` when the notes picker is open with `n` items.
    pub fn new(
        width: f32,
        height: f32,
        scale: f32,
        tab_scroll_x: f32,
        tabs: &[(&str, bool)],
        picker_list_len: Option<usize>,
    ) -> Self {
        let notes_picker = picker_list_len
            .map(|len| NotesPicker::new(width, height, scale, len));
        Self {
            tab_bar:      TabBar::new(width, scale, tab_scroll_x, tabs),
            content_area: ContentArea::new(width, height, scale),
            notes_picker,
            width,
            height,
            scale,
        }
    }

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
            return hover;
        }

        match self.tab_bar.hit_test(x, y) {
            UiNode::Tab(i) => hover.tab_index = Some(i),
            UiNode::NewTabButton => hover.plus = true,
            UiNode::WindowMinimize => hover.window_minimize = true,
            UiNode::WindowMaximize => hover.window_maximize = true,
            UiNode::WindowClose => hover.window_close = true,
            _ => {}
        }

        hover.scrollbar = self.content_area.scrollbar.hit_test(x, y)
            && self.content_area.scrollbar.is_scrollable(total_lines, visible_lines);
        hover
    }

    pub fn click(
        &self,
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
        &self,
        y: f32,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
        drag_offset: f32,
    ) -> UiDragAction {
        if let Some(ratio) = self.content_area.scrollbar.drag_ratio(y, total_lines, visible_lines, drag_offset, scroll_offset) {
            return UiDragAction::ScrollbarDrag { ratio };
        }
        UiDragAction::None
    }

    pub fn double_click(
        &self,
        x: f32,
        y: f32,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
    ) -> UiAction {
        self.multi_click(x, y, total_lines, visible_lines, scroll_offset)
    }

    pub fn triple_click(
        &self,
        x: f32,
        y: f32,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
    ) -> UiAction {
        self.multi_click(x, y, total_lines, visible_lines, scroll_offset)
    }

    fn multi_click(
        &self,
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
