//! UI layout and hit-testing

use crate::config::{layout, rendering};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UiNode {
    None,
    Tab(usize),
    NewTabButton,
    Scrollbar,
    TextArea,
    TabBar,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UiHover {
    pub tab_index: Option<usize>,
    pub plus: bool,
    pub scrollbar: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum UiAction {
    None,
    ActivateTab(usize),
    NewTab,
    StartScrollbarDrag { drag_offset: f32 },
    ScrollbarJump { ratio: f32 },
    TextClick,
    TabBarClick,
}

#[derive(Debug, Clone, Copy)]
pub enum UiDragAction {
    None,
    ScrollbarDrag { ratio: f32 },
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}

#[derive(Debug, Clone)]
pub struct TabMetrics {
    pub index: usize,
    pub rect: Rect,
}

#[derive(Debug, Clone)]
pub struct TabBar {
    pub rect: Rect,
    pub tabs: Vec<TabMetrics>,
    pub new_tab_rect: Rect,
}

impl TabBar {
    pub fn new(width: f32, scale: f32, tab_scroll_x: f32, tabs: &[(&str, bool)]) -> Self {
        let tab_height = layout::TAB_HEIGHT * scale;
        let tab_padding = layout::TAB_PADDING * scale;

        let mut current_x = -tab_scroll_x;
        let mut tab_metrics = Vec::with_capacity(tabs.len());

        for (i, (title, _)) in tabs.iter().enumerate() {
            let tab_width =
                (title.len() as f32 * rendering::TAB_CHAR_WIDTH_RATIO * scale + tab_padding * 2.0)
                    .max(layout::MIN_TAB_WIDTH * scale);
            let rect = Rect {
                x: current_x,
                y: 0.0,
                width: tab_width,
                height: tab_height,
            };
            tab_metrics.push(TabMetrics { index: i, rect });
            current_x += tab_width + 1.0;
        }

        let new_tab_button_size = layout::NEW_TAB_BUTTON_SIZE * scale;
        let new_tab_rect = Rect {
            x: current_x + 8.0 * scale,
            y: (tab_height - new_tab_button_size) / 2.0,
            width: new_tab_button_size,
            height: new_tab_button_size,
        };

        Self {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width,
                height: tab_height,
            },
            tabs: tab_metrics,
            new_tab_rect,
        }
    }

    pub fn hit_test(&self, x: f32, y: f32) -> UiNode {
        if !self.rect.contains(x, y) {
            return UiNode::None;
        }

        for tab in &self.tabs {
            if tab.rect.contains(x, y) {
                return UiNode::Tab(tab.index);
            }
        }

        if self.new_tab_rect.contains(x, y) {
            return UiNode::NewTabButton;
        }

        UiNode::TabBar
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ScrollbarAction {
    None,
    StartDrag { drag_offset: f32 },
    JumpTo { ratio: f32 },
}

#[derive(Debug, Clone, Copy)]
pub struct ScrollbarMetrics {
    pub track: Rect,
    pub thumb: Rect,
}

#[derive(Debug, Clone)]
pub struct ScrollbarWidget {
    pub rect: Rect,
    scale: f32,
}

impl ScrollbarWidget {
    pub fn new(width: f32, height: f32, scale: f32) -> Self {
        let tab_height = layout::TAB_HEIGHT * scale;
        let padding = layout::PADDING * scale;
        let scrollbar_width = layout::SCROLLBAR_WIDTH * scale;
        Self {
            rect: Rect {
                x: width - scrollbar_width,
                y: tab_height,
                width: scrollbar_width,
                height: (height - tab_height - padding).max(0.0),
            },
            scale,
        }
    }

    pub fn hit_test(&self, x: f32, y: f32) -> bool {
        self.rect.contains(x, y)
    }

    pub fn is_scrollable(&self, total_lines: usize, visible_lines: usize) -> bool {
        total_lines > visible_lines && visible_lines > 0
    }

    pub fn thumb_rect(
        &self,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
    ) -> Option<Rect> {
        self.metrics(total_lines, visible_lines, scroll_offset)
            .map(|metrics| metrics.thumb)
    }

    pub fn metrics(
        &self,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
    ) -> Option<ScrollbarMetrics> {
        if !self.is_scrollable(total_lines, visible_lines) {
            return None;
        }

        let track_height = self.rect.height;
        let track = Rect {
            x: self.rect.x,
            y: self.rect.y,
            width: self.rect.width,
            height: track_height,
        };

        let view_ratio = visible_lines as f32 / total_lines as f32;
        let min_thumb = layout::MIN_SCROLLBAR_THUMB * self.scale;
        let thumb_height = (track_height * view_ratio).max(min_thumb);

        let max_scroll = total_lines.saturating_sub(visible_lines);
        let scroll_ratio = if max_scroll > 0 {
            scroll_offset as f32 / max_scroll as f32
        } else {
            0.0
        };

        let track_space = (track_height - thumb_height).max(0.0);
        let thumb_y = self.rect.y + track_space * scroll_ratio.clamp(0.0, 1.0);
        let thumb = Rect {
            x: track.x,
            y: thumb_y,
            width: track.width,
            height: thumb_height,
        };

        Some(ScrollbarMetrics { track, thumb })
    }

    pub fn on_click(
        &self,
        x: f32,
        y: f32,
        total_lines: usize,
        visible_lines: usize,
        scroll_offset: usize,
    ) -> ScrollbarAction {
        if !self.is_scrollable(total_lines, visible_lines) {
            return ScrollbarAction::None;
        }

        if let Some(thumb) = self.thumb_rect(total_lines, visible_lines, scroll_offset) {
            if thumb.contains(x, y) {
                return ScrollbarAction::StartDrag {
                    drag_offset: y - thumb.y,
                };
            }
        }

        ScrollbarAction::JumpTo {
            ratio: self.jump_ratio(y),
        }
    }

    pub fn drag_ratio(
        &self,
        y: f32,
        total_lines: usize,
        visible_lines: usize,
        drag_offset: f32,
        scroll_offset: usize,
    ) -> Option<f32> {
        if !self.is_scrollable(total_lines, visible_lines) {
            return None;
        }

        let thumb = self.thumb_rect(total_lines, visible_lines, scroll_offset)?;
        let track_space = (self.rect.height - thumb.height).max(0.0);
        let relative_y = (y - self.rect.y - drag_offset).clamp(0.0, track_space);
        Some(if track_space > 0.0 {
            relative_y / track_space
        } else {
            0.0
        })
    }

    fn jump_ratio(&self, y: f32) -> f32 {
        let track_height = self.rect.height.max(1.0);
        let relative_y = (y - self.rect.y).clamp(0.0, track_height);
        (relative_y / track_height).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone)]
pub struct TextArea {
    pub rect: Rect,
}

impl TextArea {
    pub fn new(width: f32, height: f32, scale: f32) -> Self {
        let tab_height = layout::TAB_HEIGHT * scale;
        let padding = layout::PADDING * scale;
        let y = tab_height + padding;
        let height = (height - y - padding).max(0.0);
        Self {
            rect: Rect {
                x: 0.0,
                y,
                width,
                height,
            },
        }
    }

    pub fn hit_test(&self, x: f32, y: f32) -> bool {
        self.rect.contains(x, y)
    }
}

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
