#![allow(dead_code)]
//! List primitive — selection state + scrollable geometry + events, one file.
//!
//! Replaces `ui::ListWidget<T>` (state) + `ui::ListViewWidget` (geometry) + `fw::List<T>` (merged).

use crate::config::layout as cfg;
use crate::layout::Widget;
use crate::layout::node::{BoxStyle, Color, Node, ScrollbarNode, TextStyle};
use crate::ui::{CursorShape, Rect};
use super::scrollbar::{Scrollbar, ScrollbarAction};

// ── Row geometry ────────────────────────────────────────────────────────────

/// Geometry for one visible row, passed to snapshot closures.
#[derive(Debug, Clone, Copy)]
pub struct RowGeometry {
    /// Row bounding rect.
    pub rect:       crate::ui::Rect,
    /// Text baseline Y (rect.y + height * 0.65).
    pub baseline_y: f32,
    /// Center Y (rect.y + height * 0.5).
    pub center_y:   f32,
    /// True if this row is the selected one.
    pub is_selected: bool,
    /// 0-based display index within the visible window.
    pub display_index: usize,
}

// ── Event type ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ListPointerResult {
    None,
    Selected,
    Confirmed(usize),
    Scrolled,
    ScrollbarDragStart { drag_offset: f32 },
}

// ── Primitive ─────────────────────────────────────────────────────────────────

/// Scrollable list — owns items, selection, filter, geometry, and scrollbar drag.
#[derive(Debug, Clone)]
pub struct List<T> {
    items:            Vec<T>,
    filtered_indices: Vec<usize>,
    selected_index:   usize,
    scroll_offset:    usize,
    max_visible:      usize,

    // ── Geometry ──────────────────────────────────────────────────────────────
    rect:        Rect,
    list_rect:   Rect,
    scrollbar:   Scrollbar,
    item_height: f32,
    scale:       f32,
}

impl<T> List<T> {
    /// Create with items. Geometry is zeroed until `relayout()` is called.
    pub fn new(items: Vec<T>) -> Self {
        let n = items.len();
        Self {
            items,
            filtered_indices: (0..n).collect(),
            selected_index:   0,
            scroll_offset:    0,
            max_visible:      10,
            rect:        Rect::ZERO,
            list_rect:   Rect::ZERO,
            scrollbar:   Scrollbar::new(Rect::ZERO, 1.0),
            item_height: 32.0,
            scale:       1.0,
        }
    }

    /// Update geometry, preserving all state.
    pub fn relayout(&mut self, rect: Rect, scale: f32) {
        let sb_w        = cfg::SCROLLBAR_WIDTH * scale;
        let item_height = 32.0 * scale;
        let rects       = rect.split_h(&[1.0, -sb_w]);
        let list_rect   = rects[0];
        let sb_rect     = rects[1];
        self.rect        = rect;
        self.list_rect   = list_rect;
        self.scrollbar.relayout(sb_rect, scale);
        self.item_height = item_height;
        self.scale       = scale;
    }

    pub fn set_max_visible(&mut self, max: usize) { self.max_visible = max.max(1); }

    // ── State accessors ───────────────────────────────────────────────────────

    pub fn items(&self)            -> &[T]    { &self.items }
    pub fn filtered_indices(&self) -> &[usize] { &self.filtered_indices }
    pub fn selected_index(&self)   -> usize   { self.selected_index }
    pub fn scroll_offset(&self)    -> usize   { self.scroll_offset }
    pub fn len(&self)              -> usize   { self.filtered_indices.len() }
    pub fn is_empty(&self)         -> bool    { self.filtered_indices.is_empty() }

    pub fn selected_item(&self) -> Option<&T> {
        self.filtered_indices.get(self.selected_index).and_then(|&i| self.items.get(i))
    }

    pub fn selected_original_index(&self) -> Option<usize> {
        self.filtered_indices.get(self.selected_index).copied()
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    pub fn select_up(&mut self) -> bool {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.ensure_visible();
            true
        } else { false }
    }

    pub fn select_down(&mut self) -> bool {
        if !self.filtered_indices.is_empty() && self.selected_index < self.filtered_indices.len() - 1 {
            self.selected_index += 1;
            self.ensure_visible();
            true
        } else { false }
    }

    pub fn select_index(&mut self, index: usize) -> bool {
        if index < self.filtered_indices.len() {
            self.selected_index = index;
            self.ensure_visible();
            true
        } else { false }
    }

    pub fn scroll_to(&mut self, offset: usize) {
        let max = self.filtered_indices.len().saturating_sub(1);
        self.scroll_offset = offset.min(max);
    }

    pub fn scroll_by(&mut self, lines: isize) -> bool {
        if lines > 0 {
            (0..lines as usize).fold(false, |c, _| self.select_down() || c)
        } else {
            (0..(-lines) as usize).fold(false, |c, _| self.select_up() || c)
        }
    }

    // ── Filter ────────────────────────────────────────────────────────────────

    pub fn filter<F: Fn(&T) -> bool>(&mut self, pred: F) {
        self.filtered_indices = self.items.iter().enumerate()
            .filter(|(_, item)| pred(item))
            .map(|(i, _)| i)
            .collect();
        self.selected_index = 0;
        self.scroll_offset  = 0;
    }

    pub fn clear_filter(&mut self) {
        self.filtered_indices = (0..self.items.len()).collect();
        self.selected_index   = 0;
        self.scroll_offset    = 0;
    }

    // ── Geometry accessors ────────────────────────────────────────────────────

    pub fn visible_count(&self) -> usize {
        let max = (self.list_rect.height / self.item_height).floor() as usize;
        max.min(self.filtered_indices.len()).min(self.max_visible)
    }

    pub fn item_rect(&self, display_idx: usize) -> Rect {
        Rect {
            x:      self.list_rect.x,
            y:      self.list_rect.y + display_idx as f32 * self.item_height,
            width:  self.list_rect.width,
            height: self.item_height,
        }
    }

    pub fn item_baseline_y(&self, display_idx: usize, font_size: f32) -> f32 {
        self.list_rect.y + display_idx as f32 * self.item_height + self.item_height / 2.0 + font_size * 0.35
    }

    pub fn item_center_y(&self, display_idx: usize) -> f32 {
        self.list_rect.y + (display_idx as f32 + 0.5) * self.item_height
    }

    pub fn scrollbar_rect(&self) -> Rect { self.scrollbar.rect }

    /// Returns `(track_rect, thumb_rect)` when scrollbar is visible.
    pub fn scrollbar_rects(&self) -> Option<(Rect, Rect)> {
        let total   = self.filtered_indices.len();
        let visible = self.visible_count();
        let thumb   = self.scrollbar.list_thumb(total, visible, self.scroll_offset)?;
        Some((self.scrollbar.rect, thumb.rect))
    }

    pub fn list_rect(&self) -> Rect { self.list_rect }
    pub fn rect(&self) -> Rect { self.rect }

    /// Iterate visible rows, calling `f(item, geometry)` for each.
    /// Returns the mapped results as a `Vec<R>`.
    /// This is the canonical way to build row snapshot data without
    /// duplicating the scroll/select/index arithmetic in callers.
    pub fn visible_rows_snapshot<R, F>(&self, f: F) -> Vec<R>
    where
        F: Fn(&T, RowGeometry) -> R,
    {
        let scroll   = self.scroll_offset;
        let selected = self.selected_index;
        let visible  = self.visible_count();
        let item_h   = self.item_height;

        self.filtered_indices.iter()
            .skip(scroll)
            .take(visible)
            .enumerate()
            .filter_map(|(di, &fi)| {
                let item = self.items.get(fi)?;
                let y    = self.list_rect.y + di as f32 * item_h;
                let geo  = RowGeometry {
                    rect:          Rect { x: self.list_rect.x, y, width: self.list_rect.width, height: item_h },
                    baseline_y:    y + item_h * 0.65,
                    center_y:      y + item_h * 0.5,
                    is_selected:   scroll + di == selected,
                    display_index: di,
                };
                Some(f(item, geo))
            })
            .collect()
    }

    // ── Events ────────────────────────────────────────────────────────────────

    pub fn on_pointer_down(&mut self, x: f32, y: f32) -> ListPointerResult {
        let total   = self.filtered_indices.len();
        let visible = self.visible_count();
        let offset  = self.scroll_offset;

        if self.scrollbar.hit_test(x, y) {
            match self.scrollbar.on_click(x, y, total, visible, offset) {
                ScrollbarAction::StartDrag { drag_offset } => {
                    return ListPointerResult::ScrollbarDragStart { drag_offset };
                }
                ScrollbarAction::JumpTo { ratio } => {
                    let max_scroll = total.saturating_sub(visible);
                    let target = (ratio.clamp(0.0, 1.0) * max_scroll as f32).round() as usize;
                    self.scroll_to(target);
                    return ListPointerResult::Scrolled;
                }
                ScrollbarAction::None => {}
            }
        }

        if self.list_rect.contains(x, y) {
            let display_idx = ((y - self.list_rect.y) / self.item_height).floor() as usize;
            if display_idx < visible {
                let abs_idx    = offset + display_idx;
                let was_selected = self.selected_index == abs_idx;
                self.select_index(abs_idx);
                return if was_selected {
                    ListPointerResult::Confirmed(abs_idx)
                } else {
                    ListPointerResult::Selected
                };
            }
        }

        ListPointerResult::None
    }

    pub fn continue_scrollbar_drag(&mut self, y: f32) -> bool {
        let Some(drag_offset) = self.scrollbar.drag_offset_value() else { return false; };
        let total   = self.filtered_indices.len();
        let visible = self.visible_count();
        let offset  = self.scroll_offset;
        if let Some(ratio) = self.scrollbar.list_drag_ratio(y, total, visible, offset, drag_offset) {
            let max_scroll = total.saturating_sub(visible);
            let target = (ratio.clamp(0.0, 1.0) * max_scroll as f32).round() as usize;
            self.scroll_to(target);
            return true;
        }
        false
    }

    pub fn end_drag(&mut self) { self.scrollbar.end_drag(); }

    pub fn is_scrollbar_dragging(&self) -> bool { self.scrollbar.is_dragging() }

    pub fn on_hover(&mut self, x: f32, y: f32) -> bool {
        let total   = self.filtered_indices.len();
        let visible = self.visible_count();
        let offset  = self.scroll_offset;
        if self.list_rect.contains(x, y) {
            let display_idx = ((y - self.list_rect.y) / self.item_height).floor() as usize;
            if display_idx < visible {
                let target = offset + display_idx;
                if total > 0 && target < total && self.selected_index != target {
                    self.select_index(target);
                    return true;
                }
            }
        }
        false
    }

    // ── Private ───────────────────────────────────────────────────────────────

    fn ensure_visible(&mut self) {
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + self.max_visible {
            self.scroll_offset = self.selected_index - self.max_visible + 1;
        }
    }
}

// ── Node / Component ──────────────────────────────────────────────────────────

impl<T: Clone> List<T> {
    /// Declare this list as a `Node` subtree.
    /// `row_label` — closure mapping an item to `(title, has_indicator)` for display.
    /// `font_size` — physical pixels.
    /// `indicator_x` — X of the right-side dot (passed in from the parent layout).
    pub fn render<F>(&self, font_size: f32, indicator_x: f32, row_label: F) -> Node
    where
        F: Fn(&T) -> (String, bool),
    {
        let item_h = self.item_height;
        let accent = Color::rgba(0.4, 0.7, 1.0, 1.0);

        let row_style = TextStyle {
            font_size,
            color:      Color::rgb(0.78, 0.78, 0.78),
            baseline_y: 0.0,
            clip_x:     self.list_rect.x,
            scroll_x:   0.0,
        };
        let sel_style  = TextStyle { color: Color::rgb(1.0, 1.0, 1.0), ..row_style };
        let dot_style  = TextStyle { font_size: font_size * 0.8, color: accent, ..row_style };

        self.render_inner(font_size, indicator_x, row_label,
            row_style, sel_style, dot_style,
            Color::rgba(0.4, 0.7, 1.0, 0.12),
            Color::rgba(0.59, 0.59, 0.59, 0.71),
            Color::rgba(0.24, 0.24, 0.24, 0.47),
            Color::rgba(0.4, 0.7, 1.0, 0.6),
            item_h,
        )
    }

    /// Theme-aware render — derives all colors from `theme` tokens.
    pub fn render_themed<F>(&self, theme: &crate::theme::Theme, scale: f32, row_label: F) -> Node
    where
        F: Fn(&T) -> (String, bool),
    {
        let item_h      = self.item_height;
        let font_size   = theme.overlay_font_size * scale;
        let indicator_x = self.list_rect.x + self.list_rect.width - 16.0 * scale;

        let (r, g, b)   = theme.list_row_fg;
        let (sr, sg, sb) = theme.list_sel_fg;
        let (ar, ag, ab, aa) = theme.list_accent;

        let row_style = TextStyle {
            font_size,
            color:      Color::rgb(r, g, b),
            baseline_y: 0.0,
            clip_x:     self.list_rect.x,
            scroll_x:   0.0,
        };
        let sel_style = TextStyle { color: Color::rgb(sr, sg, sb), ..row_style };
        let dot_style = TextStyle { font_size: font_size * 0.8, color: Color::rgba(ar, ag, ab, aa), ..row_style };

        let (sbr, sbg, sbb, sba) = theme.list_sel_bg;
        let (er, eg, eb, ea)     = theme.list_empty_fg;
        let (tr, tg, tb, ta)     = theme.list_scrollbar_track;
        let thumb_color          = Color::rgba(ar, ag, ab, aa * 0.6);

        self.render_inner(font_size, indicator_x, row_label,
            row_style, sel_style, dot_style,
            Color::rgba(sbr, sbg, sbb, sba),
            Color::rgba(er, eg, eb, ea),
            Color::rgba(tr, tg, tb, ta),
            thumb_color,
            item_h,
        )
    }

    /// Render at `rect` with geometry computed on-the-fly — no prior `relayout` needed.
    /// Uses the same theme tokens as `render_themed`. The stored `list_rect` / `item_height`
    /// are ignored; everything is derived from `rect` and `scale`.
    pub fn render_themed_at<F>(&self, rect: Rect, theme: &crate::theme::Theme, scale: f32, row_label: F) -> Node
    where
        F: Fn(&T) -> (String, bool),
    {
        use crate::config::layout as cfg;
        let sb_w        = cfg::SCROLLBAR_WIDTH * scale;
        let item_height = 32.0 * scale;
        let rects       = rect.split_h(&[1.0, -sb_w]);
        let list_rect   = rects[0];
        let sb_rect     = rects[1];

        let font_size   = theme.overlay_font_size * scale;
        let indicator_x = list_rect.x + list_rect.width - 16.0 * scale;

        let (r, g, b)        = theme.list_row_fg;
        let (sr, sg, sb_)    = theme.list_sel_fg;
        let (ar, ag, ab, aa) = theme.list_accent;

        let row_style = TextStyle {
            font_size,
            color:      Color::rgb(r, g, b),
            baseline_y: 0.0,
            clip_x:     list_rect.x,
            scroll_x:   0.0,
        };
        let sel_style = TextStyle { color: Color::rgb(sr, sg, sb_), ..row_style };
        let dot_style = TextStyle { font_size: font_size * 0.8, color: Color::rgba(ar, ag, ab, aa), ..row_style };

        let (sbr, sbg, sbb, sba) = theme.list_sel_bg;
        let (er, eg, eb, ea)     = theme.list_empty_fg;
        let (tr, tg, tb, ta)     = theme.list_scrollbar_track;
        let thumb_color          = Color::rgba(ar, ag, ab, aa * 0.6);
        let sel_bg               = Color::rgba(sbr, sbg, sbb, sba);

        let scroll   = self.scroll_offset;
        let selected = self.selected_index;
        let max_vis  = (list_rect.height / item_height).floor() as usize;
        let visible  = max_vis.min(self.filtered_indices.len()).min(self.max_visible);

        let mut children: Vec<Node> = self.filtered_indices.iter()
            .skip(scroll)
            .take(visible)
            .enumerate()
            .map(|(display_idx, &item_idx)| {
                let item = &self.items[item_idx];
                let (title, has_indicator) = row_label(item);
                let row_rect = Rect {
                    x: list_rect.x,
                    y: list_rect.y + display_idx as f32 * item_height,
                    width: list_rect.width,
                    height: item_height,
                };
                let baseline_y = row_rect.y + item_height / 2.0 + font_size * 0.35;
                let center_y   = row_rect.y + item_height * 0.5;
                let is_selected = display_idx + scroll == selected;

                let mut row_nodes = vec![];
                if is_selected {
                    row_nodes.push(Node::Box { rect: row_rect, style: BoxStyle::filled(sel_bg).with_radius(4.0) });
                }
                let style = if is_selected { sel_style } else { row_style };
                row_nodes.push(Node::Text {
                    text:  title,
                    style: TextStyle { baseline_y, ..style },
                    clip:  Some(list_rect),
                });
                if has_indicator {
                    row_nodes.push(Node::Text {
                        text:  "●".to_string(),
                        style: TextStyle { baseline_y: center_y, clip_x: indicator_x, ..dot_style },
                        clip:  None,
                    });
                }
                Node::layer(row_nodes)
            })
            .collect();

        if self.filtered_indices.is_empty() {
            children.push(Node::Text {
                text:  "No matching notes".to_string(),
                style: TextStyle {
                    baseline_y: list_rect.y + item_height * 0.65,
                    color: Color::rgba(er, eg, eb, ea),
                    ..row_style
                },
                clip: None,
            });
        }

        // Scrollbar
        let total = self.filtered_indices.len();
        if total > visible && visible > 0 {
            use crate::primitives::scrollbar::Scrollbar;
            let mut sb = Scrollbar::new(sb_rect, scale);
            sb.relayout(sb_rect, scale);
            if let Some(thumb) = sb.list_thumb(total, visible, scroll) {
                children.push(Node::Scrollbar(ScrollbarNode {
                    track: sb_rect,
                    thumb: thumb.rect,
                    track_color: Color::rgba(tr, tg, tb, ta),
                    thumb_color,
                }));
            }
        }

        Node::layer(children)
    }

    #[allow(clippy::too_many_arguments)]
    fn render_inner<F>(
        &self,
        _font_size: f32,
        indicator_x: f32,
        row_label: F,
        row_style: TextStyle,
        sel_style: TextStyle,
        dot_style: TextStyle,
        sel_bg: Color,
        empty_fg: Color,
        track_color: Color,
        thumb_color: Color,
        item_h: f32,
    ) -> Node
    where
        F: Fn(&T) -> (String, bool),
    {
        let mut children: Vec<Node> = self.visible_rows_snapshot(|item, geo| {
            let (title, has_indicator) = row_label(item);
            let mut row_nodes = vec![];

            if geo.is_selected {
                row_nodes.push(Node::Box {
                    rect:  geo.rect,
                    style: BoxStyle::filled(sel_bg).with_radius(4.0),
                });
            }

            let style = if geo.is_selected { sel_style } else { row_style };
            row_nodes.push(Node::Text {
                text:  title,
                style: TextStyle { baseline_y: geo.baseline_y, ..style },
                clip:  Some(self.list_rect),
            });

            if has_indicator {
                row_nodes.push(Node::Text {
                    text:  "●".to_string(),
                    style: TextStyle { baseline_y: geo.center_y, clip_x: indicator_x, ..dot_style },
                    clip:  None,
                });
            }

            Node::layer(row_nodes)
        });

        if self.filtered_indices.is_empty() {
            children.push(Node::Text {
                text:  "No matching notes".to_string(),
                style: TextStyle {
                    baseline_y: self.list_rect.y + item_h * 0.65,
                    color: empty_fg,
                    ..row_style
                },
                clip:  None,
            });
        }

        if let Some((track, thumb)) = self.scrollbar_rects() {
            children.push(Node::Scrollbar(ScrollbarNode { track, thumb, track_color, thumb_color }));
        }

        Node::layer(children)
    }
}

// ── Widget impl ───────────────────────────────────────────────────────────────

impl<T: Clone> Widget for List<T> {
    type Event = ListPointerResult;
    fn on_pointer_down(&mut self, x: f32, y: f32) -> ListPointerResult {
        self.on_pointer_down(x, y)
    }
    fn on_hover(&mut self, x: f32, y: f32) -> bool {
        self.on_hover(x, y)
    }
    fn cursor_shape_at(&self, _x: f32, _y: f32) -> CursorShape {
        CursorShape::Default
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection() {
        let mut list: List<&str> = List::new(vec!["a", "b", "c", "d"]);
        assert_eq!(list.selected_index(), 0);
        assert!(list.select_down());
        assert_eq!(list.selected_index(), 1);
        assert!(list.select_down());
        assert_eq!(list.selected_index(), 2);
        assert!(list.select_up());
        assert_eq!(list.selected_index(), 1);
        list.select_index(3);
        assert_eq!(list.selected_index(), 3);
        assert!(!list.select_down()); // already at end
        assert_eq!(list.selected_index(), 3);
    }

    #[test]
    fn test_filter() {
        let mut list: List<&str> = List::new(vec!["apple", "banana", "apricot", "cherry"]);
        assert_eq!(list.len(), 4);
        list.filter(|s| s.starts_with('a'));
        assert_eq!(list.len(), 2);
        assert_eq!(list.selected_index(), 0);
        assert_eq!(list.selected_item(), Some(&"apple"));
        assert!(list.select_down());
        assert_eq!(list.selected_item(), Some(&"apricot"));
        list.clear_filter();
        assert_eq!(list.len(), 4);
        assert_eq!(list.selected_index(), 0);
    }
}
