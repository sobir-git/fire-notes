//! UI tree coordinator for hit-testing and actions

use super::tab_bar::TabBar;
use super::scrollbar::{ScrollbarAction, ScrollbarWidget};
use super::text_area::TextArea;
use super::types::{ResizeEdge, UiAction, UiDragAction, UiHover, UiNode};

const RESIZE_BORDER: f32 = 5.0;

#[derive(Debug, Clone)]
pub struct UiTree {
    pub tab_bar: TabBar,
    pub scrollbar: ScrollbarWidget,
    pub text_area: TextArea,
    width: f32,
    height: f32,
    scale: f32,
}

impl UiTree {
    pub fn new(width: f32, height: f32, scale: f32, tab_scroll_x: f32, tabs: &[(&str, bool)]) -> Self {
        Self {
            tab_bar: TabBar::new(width, scale, tab_scroll_x, tabs),
            scrollbar: ScrollbarWidget::new(width, height, scale),
            text_area: TextArea::new(width, height, scale),
            width,
            height,
            scale,
        }
    }

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

    pub fn hover(
        &self,
        x: f32,
        y: f32,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
    ) -> UiHover {
        let mut hover = UiHover::default();

        // Check resize edges first
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

        hover.scrollbar = self
            .scrollbar
            .metrics(total_lines, visible_lines, scroll_offset)
            .map(|metrics| metrics.track.contains(x, y))
            .unwrap_or(false);
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
            UiNode::Scrollbar => {
                if selecting {
                    return UiAction::None;
                }
                match self
                    .scrollbar
                    .on_click(x, y, total_lines, visible_lines, scroll_offset)
                {
                    ScrollbarAction::StartDrag { drag_offset } => {
                        UiAction::StartScrollbarDrag { drag_offset }
                    }
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
        if let Some(ratio) = self
            .scrollbar
            .drag_ratio(y, total_lines, visible_lines, drag_offset, scroll_offset)
        {
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
        match self.hit_test(x, y) {
            UiNode::Tab(_) | UiNode::NewTabButton | UiNode::TabBar
            | UiNode::WindowMinimize | UiNode::WindowMaximize | UiNode::WindowClose
            | UiNode::WindowResizeEdge(_) => {
                return self.click(x, y, total_lines, visible_lines, scroll_offset, false);
            }
            UiNode::Scrollbar => return UiAction::None,
            UiNode::TextArea => return UiAction::TextClick,
            UiNode::None => return UiAction::None,
        }
    }

    pub fn triple_click(
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
                return self.click(x, y, total_lines, visible_lines, scroll_offset, false);
            }
            UiNode::Scrollbar => return UiAction::None,
            UiNode::TextArea => return UiAction::TextClick,
            UiNode::None => return UiAction::None,
        }
    }

    pub fn hit_test(&self, x: f32, y: f32) -> UiNode {
        // Check resize edges first (highest priority for borderless window)
        if let Some(edge) = self.detect_resize_edge(x, y) {
            return UiNode::WindowResizeEdge(edge);
        }

        if self.tab_bar.rect.contains(x, y) {
            return self.tab_bar.hit_test(x, y);
        }

        if self.scrollbar.hit_test(x, y) {
            return UiNode::Scrollbar;
        }

        if self.text_area.hit_test(x, y) {
            return UiNode::TextArea;
        }

        UiNode::None
    }
}
