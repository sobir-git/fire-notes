//! Render sub-modules — each builds one part of the `Node` scene.
//!
//! `AppLogic::render()` calls into these helpers; no rendering logic lives
//! in `logic/mod.rs` itself.

mod editor;
mod flames;
mod overlay;
mod tab_bar;

use crate::layout::Node;
use crate::layout::node::EditorNode;
use crate::logic::AppLogic;

impl AppLogic {
    /// Produce the full UI scene as a `Node` tree.
    ///
    /// This is the **only** render path. The renderer walks this tree and emits
    /// draw calls. No `RenderFrame`, no snapshot bridging, no per-subsystem
    /// baking methods.
    pub fn render(&mut self) -> Node {
        self.prepare_ui_tree();

        let tab_bar_node = tab_bar::build(self);

        let (mut editor_node, sel_rects) = editor::build(self);

        // Inject flames into the already-built EditorNode
        if let Node::Editor(ref mut en) = editor_node {
            let ca           = &self.ui_tree.content_area;
            let current_tab  = &self.tabs[self.active_tab];
            let scroll_offset = current_tab.scroll_offset();
            let line_height   = ca.text.line_height;
            let text_padding  = ca.text.text_padding;
            let doc_top_margin = ca.text.doc_top_margin;
            let scroll_x      = current_tab.scroll_offset_x();
            en.flames = flames::build(
                self,
                &sel_rects,
                scroll_offset,
                line_height,
                text_padding,
                doc_top_margin,
                scroll_x,
            );
        }

        let overlay = overlay::build(self);

        let mut scene = vec![tab_bar_node, editor_node];
        if let Some(o) = overlay { scene.push(o); }
        Node::layer(scene)
    }
}

// Silence the unused import lint that Rust emits when EditorNode is only used
// via pattern matching inside the render() body above.
const _: fn() = || { let _: EditorNode; };
