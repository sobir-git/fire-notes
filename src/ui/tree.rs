//! UI tree coordinator for hit-testing and actions

use super::tab_bar::TabBar;
use super::scrollbar::{ScrollbarAction, ScrollbarWidget};
use super::text_area::TextArea;
use super::types::{UiAction, UiDragAction, UiHover, UiNode};

#[derive(Debug, Clone)]
pub struct UiTree {
    pub tab_bar: TabBar,
    pub scrollbar: ScrollbarWidget,
    pub text_area: TextArea,
}

impl UiTree {
    pub fn new(width: f32, height: f32, scale: f32, tab_scroll_x: f32, tabs: &[(&str, bool)]) -> Self {
        Self {
            tab_bar: TabBar::new(width, scale, tab_scroll_x, tabs),
            scrollbar: ScrollbarWidget::new(width, height, scale),
            text_area: TextArea::new(width, height, scale),
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
        match self.tab_bar.hit_test(x, y) {
            UiNode::Tab(i) => hover.tab_index = Some(i),
            UiNode::NewTabButton => hover.plus = true,
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
            UiNode::TabBar if !selecting => UiAction::TabBarClick,
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
            UiNode::Tab(_) | UiNode::NewTabButton | UiNode::TabBar => {
                return self.click(x, y, total_lines, visible_lines, scroll_offset, false);
            }
            UiNode::Scrollbar => return UiAction::None,
            UiNode::TextArea => return UiAction::TextClick,
            _ => return UiAction::None,
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
            UiNode::Tab(_) | UiNode::NewTabButton | UiNode::TabBar => {
                return self.click(x, y, total_lines, visible_lines, scroll_offset, false);
            }
            UiNode::Scrollbar => return UiAction::None,
            UiNode::TextArea => return UiAction::TextClick,
            _ => return UiAction::None,
        }
    }

    pub fn hit_test(&self, x: f32, y: f32) -> UiNode {
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
