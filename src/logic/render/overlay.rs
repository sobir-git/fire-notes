//! Renders the active overlay (picker, dialog, …) into a `Node`, if any.

use crate::layout::Node;
use crate::logic::AppLogic;

pub(super) fn build(logic: &mut AppLogic) -> Option<Node> {
    let theme          = &logic.theme;
    let window         = crate::ui::Rect { x: 0.0, y: 0.0, width: logic.width, height: logic.height };
    let scale          = logic.scale;
    let cursor_visible = logic.cursor_visible;
    logic.overlay.as_mut().map(|o| o.render(theme, window, scale, cursor_visible))
}
