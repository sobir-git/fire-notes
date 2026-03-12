//! Tab bar layout and hit-testing.
//!
//! Each sub-component (WindowControls, NewTabButton) is a self-contained
//! widget that owns its own layout and exposes a clean API.
//! `TabBar` assembles them — no inline pixel arithmetic here.

use crate::config::{layout, rendering};
use super::layout::Layout;
use super::types::{Rect, UiNode};

// ── WindowControls widget ──────────────────────────────────────────────────

/// Three window-chrome buttons (minimize / maximize / close), right-aligned.
#[derive(Debug, Clone)]
pub struct WindowControls {
    pub minimize: Rect,
    pub maximize: Rect,
    pub close: Rect,
}

impl WindowControls {
    /// Build right-aligned within `bar_rect`.
    pub fn new(bar_rect: Rect, scale: f32) -> Self {
        let size   = layout::WINDOW_BUTTON_SIZE   * scale;
        let margin = layout::WINDOW_BUTTON_MARGIN * scale;
        let gap    = layout::WINDOW_BUTTON_GAP    * scale;
        let y = bar_rect.y + (bar_rect.height - size) / 2.0;

        let close    = Rect { x: bar_rect.x + bar_rect.width - size - margin,  y, width: size, height: size };
        let maximize = Rect { x: close.x - size - gap,                         y, width: size, height: size };
        let minimize = Rect { x: maximize.x - size - gap,                      y, width: size, height: size };

        Self { minimize, maximize, close }
    }

    /// Left edge of the leftmost button.
    pub fn left_edge(&self) -> f32 { self.minimize.x }

    pub fn hit_test(&self, x: f32, y: f32) -> UiNode {
        if self.close.contains(x, y)    { return UiNode::WindowClose; }
        if self.maximize.contains(x, y) { return UiNode::WindowMaximize; }
        if self.minimize.contains(x, y) { return UiNode::WindowMinimize; }
        UiNode::None
    }
}

// ── NewTabButton widget ────────────────────────────────────────────────────

/// The [+] button that creates a new tab.
///
/// Follows the last tab when there is room; clamps leftward once tabs
/// are scrolled so far right that the ideal position would enter the
/// drag gap. Always visible — never hidden.
#[derive(Debug, Clone)]
pub struct NewTabButton {
    pub rect: Rect,
}

impl NewTabButton {
    /// Build from the current end-of-tabs position, the clip boundary, and the bar rect.
    ///
    /// `tabs_end_x`  — screen x right after the last visible tab
    /// `max_right_x` — rightmost allowed right edge (left of drag gap)
    pub fn new(tabs_end_x: f32, bar_rect: Rect, max_right_x: f32, scale: f32) -> Self {
        let size   = layout::NEW_TAB_BUTTON_SIZE   * scale;
        let margin = layout::WINDOW_BUTTON_MARGIN  * scale;
        let x = (tabs_end_x + margin).min(max_right_x - size - margin);
        Self {
            rect: Rect {
                x,
                y: bar_rect.y + (bar_rect.height - size) / 2.0,
                width:  size,
                height: size,
            },
        }
    }

    pub fn hit_test(&self, x: f32, y: f32) -> bool {
        self.rect.contains(x, y)
    }
}

// ── TabScrollArea ──────────────────────────────────────────────────────────

/// A clipped, horizontally-scrollable container for tab widgets.
///
/// Children are positioned in scroll-space (origin = left edge of all tabs).
/// Hit-testing automatically clips through `clip_rect` — children outside the
/// visible area are invisible to pointer events without any per-child logic.
#[derive(Debug, Clone)]
pub struct TabScrollArea {
    /// The visible rect on screen (scissor boundary).
    pub clip_rect: Rect,
    /// Horizontal scroll offset applied to all children.
    #[allow(dead_code)]
    pub scroll_x: f32,
    pub tabs: Vec<TabMetrics>,
}

impl TabScrollArea {
    pub fn new(clip_rect: Rect, scroll_x: f32, tabs: Vec<TabMetrics>) -> Self {
        Self { clip_rect, scroll_x, tabs }
    }

    /// Hit-test through the clip: only children whose scroll-space rect intersects
    /// the clip rect and contains (x, y) are reachable.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<usize> {
        if !self.clip_rect.contains(x, y) {
            return None;
        }
        for tab in &self.tabs {
            // Intersect child rect with the clip — this is the visible portion.
            let visible = tab.rect.intersect(&self.clip_rect);
            if !visible.is_empty() && visible.contains(x, y) {
                return Some(tab.index);
            }
        }
        None
    }
}

// ── TabMetrics ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TabMetrics {
    pub index: usize,
    /// Rect in scroll-space (x accounts for scroll offset). Used by renderer and hit-testing.
    pub rect: Rect,
}

impl TabMetrics {
    pub fn new(index: usize, rect: Rect) -> Self {
        Self { index, rect }
    }
}

// ── TabBar ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TabBar {
    pub rect: Rect,
    /// Scrollable tab container — owns clip rect, scroll offset, and all tab children.
    /// Use `tab_bar.scroll_area.tabs` to iterate tabs in the renderer.
    pub scroll_area: TabScrollArea,
    pub new_tab_button: NewTabButton,
    pub controls: WindowControls,
    // Flat convenience accessors for the renderer (mirror values from sub-widgets).
    pub tabs_clip_x: f32,
    pub minimize_rect: Rect,
    pub maximize_rect: Rect,
    pub close_rect: Rect,
    pub new_tab_rect: Rect,
}

impl Layout for TabBar {
    /// Layout from the parent-supplied rect (the tab bar strip).
    /// Tabs default to no scroll and no entries — call `with_tabs` afterwards.
    fn layout(rect: Rect, scale: f32) -> Self {
        Self::build(rect, scale, 0.0, &[])
    }
}

impl TabBar {
    /// Convenience: build from window width + tab state.
    pub fn new(width: f32, scale: f32, tab_scroll_x: f32, tabs: &[(&str, bool)]) -> Self {
        let rect = Rect { x: 0.0, y: 0.0, width, height: layout::TAB_HEIGHT * scale };
        Self::build(rect, scale, tab_scroll_x, tabs)
    }

    fn build(rect: Rect, scale: f32, tab_scroll_x: f32, tabs: &[(&str, bool)]) -> Self {
        let tab_padding = layout::TAB_PADDING * scale;
        let drag_gap    = layout::TAB_DRAG_GAP * scale;

        let controls    = WindowControls::new(rect, scale);
        let tabs_clip_x = controls.left_edge() - drag_gap;

        // ── Scrolling tabs ─────────────────────────────────────────────────
        let mut current_x = rect.x - tab_scroll_x;
        let mut tab_metrics = Vec::with_capacity(tabs.len());

        for (i, (title, _)) in tabs.iter().enumerate() {
            let tab_width =
                (title.len() as f32 * rendering::TAB_CHAR_WIDTH_RATIO * scale + tab_padding * 2.0)
                    .max(layout::MIN_TAB_WIDTH * scale);
            tab_metrics.push(TabMetrics::new(i, Rect { x: current_x, y: rect.y, width: tab_width, height: rect.height }));
            current_x += tab_width + 1.0;
        }

        let new_tab_button = NewTabButton::new(current_x, rect, tabs_clip_x, scale);
        let clip_rect = Rect { x: rect.x, y: rect.y, width: tabs_clip_x - rect.x, height: rect.height };
        let scroll_area = TabScrollArea::new(clip_rect, tab_scroll_x, tab_metrics);

        Self {
            rect,
            scroll_area,
            new_tab_rect: new_tab_button.rect,
            tabs_clip_x,
            minimize_rect: controls.minimize,
            maximize_rect: controls.maximize,
            close_rect: controls.close,
            new_tab_button,
            controls,
        }
    }

    /// Return the `tab_scroll_x` value that ensures tab `tab_index` is fully
    /// visible — not clipped by the left edge or the plus-button boundary.
    /// Returns the current scroll unchanged if the tab is already fully visible.
    pub fn scroll_x_to_reveal(&self, tab_index: usize, current_scroll_x: f32) -> f32 {
        let Some(tab) = self.scroll_area.tabs.iter().find(|t| t.index == tab_index) else {
            return current_scroll_x;
        };
        let left_bound = self.rect.x;
        // When tabs overflow the button pins at tabs_clip_x - size - margin.
        // When they don't, button.x is larger. min() always picks the tightest
        // (leftmost) position, giving a scroll-independent right bound.
        let size        = self.new_tab_button.rect.width;
        let right_bound = self.new_tab_button.rect.x.min(self.tabs_clip_x - size);

        if tab.rect.x < left_bound {
            return (current_scroll_x - (left_bound - tab.rect.x)).max(0.0);
        }
        if tab.rect.x + tab.rect.width > right_bound {
            return current_scroll_x + (tab.rect.x + tab.rect.width - right_bound);
        }
        current_scroll_x
    }

    pub fn hit_test(&self, x: f32, y: f32) -> UiNode {
        if !self.rect.contains(x, y) {
            return UiNode::None;
        }

        // Priority (highest → lowest):
        //   1. Window controls
        //   2. New-tab button (pinned; wins over any tab scrolled under it)
        //   3. Tabs
        //   4. Tab bar background (drag target)

        if let node @ (UiNode::WindowClose | UiNode::WindowMaximize | UiNode::WindowMinimize)
            = self.controls.hit_test(x, y)
        {
            return node;
        }

        if self.new_tab_button.hit_test(x, y) {
            return UiNode::NewTabButton;
        }

        if let Some(tab_index) = self.scroll_area.hit_test(x, y) {
            return UiNode::Tab(tab_index);
        }

        UiNode::TabBar
    }
}
