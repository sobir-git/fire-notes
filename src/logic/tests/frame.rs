//! RenderFrame structural and integrity tests.
#![allow(unused_must_use)]

use crate::app::action::Action;
use super::{app, type_str};

#[test]
fn initial_frame_has_one_tab() {
    let logic = app();
    let frame = logic.render_frame();
    assert_eq!(frame.tabs.len(), 1);
    assert!(frame.tabs[0].is_active);
}

#[test]
fn active_tab_flag_matches_active_tab_index() {
    let mut logic = app();
    logic.execute(Action::NewTab);
    logic.execute(Action::NewTab);
    logic.execute(Action::PreviousTab);
    let frame = logic.render_frame();
    let active_count = frame.tabs.iter().filter(|t| t.is_active).count();
    assert_eq!(active_count, 1);
    assert!(frame.tabs[frame.active_tab].is_active);
}

#[test]
fn frame_dimensions_match_headless_size() {
    let logic = app();
    let frame = logic.render_frame();
    assert_eq!(frame.width, 800.0);
    assert_eq!(frame.height, 600.0);
    assert_eq!(frame.scale, 1.0);
}

#[test]
fn frame_total_lines_matches_content() {
    let mut logic = app();
    type_str(&mut logic, "a");
    logic.execute(Action::InsertChar('\n'));
    type_str(&mut logic, "b");
    let frame = logic.render_frame();
    assert_eq!(frame.total_lines, 2);
}

#[test]
fn scroll_offset_zero_at_start() {
    let logic = app();
    let frame = logic.render_frame();
    assert_eq!(frame.scroll_offset, 0);
}
