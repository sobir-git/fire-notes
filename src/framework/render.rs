//! Render pass — walks a laid-out `Element` tree and produces a `Node` tree.
//!
//! After `layout_tree()` assigns rects to every element, this pass converts
//! each `ElementKind` into its corresponding `Node` representation for the
//! renderer. The component never calls render methods directly — the framework
//! does it automatically.

use crate::framework::element::{Element, ElementKind};
use crate::layout::Node;
use crate::layout::node::{BoxStyle, Color};
use crate::theme::Theme;

/// Convert a fully laid-out `Element` tree into a `Node` tree for the renderer.
/// `theme` is passed through for visual tokens.
/// `cursor_visible` drives the text cursor blink state.
pub fn render_element<Msg>(el: &Element<Msg>, theme: &Theme, scale: f32, cursor_visible: bool) -> Node {
    match &el.kind {
        ElementKind::Backdrop { .. } => {
            // Backdrop: full-screen dim + panel box + children
            let (br, bg, bb, ba) = theme.overlay_backdrop;
            let (pr, pg, pb, pa) = theme.overlay_bg;
            let (er, eg, eb, ea) = theme.overlay_border;
            let mut nodes = vec![
                // Dim layer
                Node::Box {
                    rect:  el.rect,
                    style: BoxStyle::filled(Color::rgba(br, bg, bb, ba)),
                },
            ];
            // Panel chrome: drawn around the children's bounding rect
            // For now draw the panel around the first Column child rect
            if let Some(panel_child) = el.children.first() {
                let panel = panel_child.rect.inset(-(8.0 * scale));
                nodes.push(Node::Box {
                    rect:  panel,
                    style: BoxStyle::filled(Color::rgba(pr, pg, pb, pa))
                        .with_border(Color::rgba(er, eg, eb, ea), theme.overlay_border_width)
                        .with_radius(theme.overlay_radius * scale),
                });
            }
            // Children
            for child in &el.children {
                nodes.push(render_element(child, theme, scale, cursor_visible));
            }
            Node::layer(nodes)
        }

        ElementKind::Column | ElementKind::Row => {
            let children: Vec<Node> = el.children.iter()
                .map(|c| render_element(c, theme, scale, cursor_visible))
                .collect();
            Node::layer(children)
        }

        ElementKind::TextInput { state, placeholder, auto_focus, .. } => {
            let _ = auto_focus;
            state.render_at(el.rect, scale, placeholder, cursor_visible)
        }

        ElementKind::List { items, .. } => {
            render_list(el.rect, items, theme, scale)
        }
    }
}

fn render_list(rect: crate::ui::Rect, items: &[crate::framework::element::ListItem], theme: &Theme, scale: f32) -> Node {
    use crate::config::layout as cfg;
    use crate::layout::node::{TextStyle};

    let sb_w        = cfg::SCROLLBAR_WIDTH * scale;
    let item_height = 32.0 * scale;
    let rects       = rect.split_h(&[1.0, -sb_w]);
    let list_rect   = rects[0];

    let font_size   = theme.overlay_font_size * scale;
    let (r, g, b)   = theme.list_row_fg;
    let (sr, sg, sb_) = theme.list_sel_fg;
    let (ar, ag, ab, aa) = theme.list_accent;
    let (sbr, sbg, sbb, sba) = theme.list_sel_bg;

    let row_style = TextStyle {
        font_size,
        color:      Color::rgb(r, g, b),
        baseline_y: 0.0,
        text_x:     list_rect.x + 8.0 * scale,
        scroll_x:   0.0,
    };
    let sel_style = TextStyle { color: Color::rgb(sr, sg, sb_), ..row_style };
    let sel_bg    = Color::rgba(sbr, sbg, sbb, sba);
    let dot_color = Color::rgba(ar, ag, ab, aa);

    let max_vis = (list_rect.height / item_height).floor() as usize;
    let visible = max_vis.min(items.len());

    let mut children: Vec<Node> = items.iter().take(visible).enumerate().map(|(i, item)| {
        let row_rect = crate::ui::Rect {
            x:      list_rect.x,
            y:      list_rect.y + i as f32 * item_height,
            width:  list_rect.width,
            height: item_height,
        };
        let baseline_y = row_rect.y + item_height / 2.0 + font_size * 0.35;
        let mut row_nodes = Vec::new();
        if i == 0 {
            row_nodes.push(Node::Box { rect: row_rect, style: BoxStyle::filled(sel_bg) });
        }
        row_nodes.push(Node::Text {
            text:  item.label.clone(),
            style: TextStyle { baseline_y, ..if i == 0 { sel_style } else { row_style } },
            clip:  Some(list_rect),
        });
        if item.marked {
            row_nodes.push(Node::Text {
                text:  "•".to_string(),
                style: TextStyle {
                    baseline_y,
                    color: dot_color,
                    text_x: list_rect.x + list_rect.width - 16.0 * scale,
                    ..row_style
                },
                clip: Some(list_rect),
            });
        }
        Node::layer(row_nodes)
    }).collect();

    if items.is_empty() {
        let (er, eg, eb, ea) = theme.list_empty_fg;
        children.push(Node::Text {
            text:  "No results".to_string(),
            style: TextStyle {
                baseline_y: list_rect.y + item_height / 2.0 + font_size * 0.35,
                color:      Color::rgba(er, eg, eb, ea),
                ..row_style
            },
            clip: Some(list_rect),
        });
    }

    Node::layer(children)
}
