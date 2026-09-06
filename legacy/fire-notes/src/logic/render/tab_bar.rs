//! Builds the `Node::TabBar` subtree from retained `UiTree` state.

use crate::layout::Node;
use crate::layout::node::{
    RenameOverlayNode, TabBarNode, TabEntry, WinButton, WinButtonKind,
};
use crate::logic::AppLogic;

pub(super) fn build(logic: &AppLogic) -> Node {
    let tb    = &logic.ui_tree.tab_bar;
    let scale = logic.scale;

    let renaming_tab = logic.inline.as_ref().map(|w| w.tab_index());

    let tab_bar_font_size = crate::config::rendering::TAB_FONT_SIZE * scale;
    let tab_bar_baseline  = tb.rect.y + tb.rect.height / 2.0 + tab_bar_font_size * 0.35;

    let tabs: Vec<TabEntry> = tb.scroll_area.tabs.iter().map(|tm| {
        let t = &logic.tabs[tm.index];
        let title = if Some(tm.index) == renaming_tab {
            if let Some(crate::app::active_overlay::ActiveInline::TabRename(tr)) = &logic.inline {
                tr.input.text().to_string()
            } else {
                t.title().to_string()
            }
        } else {
            t.title().to_string()
        };
        TabEntry {
            title,
            rect: tm.rect,
            is_active:  tm.index == logic.active_tab,
            is_hovered: tb.hovered_tab_index == Some(tm.index),
        }
    }).collect();

    let rename = logic.inline.as_ref().and_then(|w| {
        let crate::app::active_overlay::ActiveInline::TabRename(tr) = w;
        let tab_index = tr.tab_index;
        let _ = tb.scroll_area.tabs.iter().find(|t| t.index == tab_index)?;
        Some(RenameOverlayNode {
            tab_index,
            text:           tr.input.text().to_string(),
            cursor:         tr.input.cursor(),
            cursor_visible: logic.cursor_visible,
            text_y:         tab_bar_baseline,
        })
    });

    let win_buttons = vec![
        WinButton { rect: tb.close_rect,    hovered: tb.hovered_close,    kind: WinButtonKind::Close },
        WinButton { rect: tb.maximize_rect, hovered: tb.hovered_maximize, kind: WinButtonKind::Maximize },
        WinButton { rect: tb.minimize_rect, hovered: tb.hovered_minimize, kind: WinButtonKind::Minimize },
    ];

    Node::TabBar(TabBarNode {
        rect:            tb.rect,
        tabs_clip_x:     tb.tabs_clip_x,
        tabs,
        new_tab_rect:    tb.new_tab_rect,
        new_tab_hovered: tb.hovered_plus,
        win_buttons,
        rename,
    })
}
