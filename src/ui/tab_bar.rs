//! Tab bar layout and hit-testing

use crate::config::{layout, rendering};
use super::types::{Rect, UiNode};

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
