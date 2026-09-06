//! Per-node draw implementations.
//!
//! Each submodule owns the rendering logic for exactly one Node variant.
//! All GPU calls are isolated here; nothing else imports femtovg.

pub mod box_node;
pub mod cursor_node;
pub mod editor_node;
pub mod scrollbar_node;
pub mod selection_node;
pub mod tab_bar_node;
pub mod text_node;

use crate::layout::Node;
use super::draw_ctx::DrawCtx;

/// Recursively draw any Node using its own draw implementation.
pub fn draw(node: &Node, ctx: &mut DrawCtx<'_>) {
    match node {
        Node::Layer(children) => {
            for child in children {
                draw(child, ctx);
            }
        }
        Node::Box { rect, style }          => box_node::draw(rect, style, ctx),
        Node::Text { text, style, clip }   => text_node::draw(text, style, clip.as_ref(), ctx),
        Node::Cursor(c)                    => cursor_node::draw(c, ctx),
        Node::Selection(s)                 => selection_node::draw(s, ctx),
        Node::Scrollbar(s)                 => scrollbar_node::draw(s, ctx),
        Node::TabBar(tb)                   => tab_bar_node::draw(tb, ctx),
        Node::Editor(ed)                   => editor_node::draw(ed, ctx),
    }
}
