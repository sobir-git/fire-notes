//! Scrollbar math and page-scroll tests.
#![allow(unused_must_use)]

use crate::app::action::Action;
use super::app;

fn overflowing_app() -> crate::logic::AppLogic {
    let mut logic = app();
    for _ in 0..100 {
        logic.execute(Action::InsertChar('\n'));
    }
    logic
}

#[test]
fn scrollbar_absent_when_content_fits() {
    let mut logic = app();
    let frame = logic.render_frame();
    assert!(frame.scrollbar.is_none());
}

#[test]
fn scrollbar_present_when_content_overflows() {
    let mut logic = overflowing_app();
    let frame = logic.render_frame();
    assert!(frame.scrollbar.is_some());
}

#[test]
fn scrollbar_thumb_height_ratio_is_viewport_fraction() {
    let mut logic = overflowing_app();
    let frame = logic.render_frame();
    let sb = frame.scrollbar.as_ref().expect("scrollbar should exist");
    let total = frame.total_lines as f32;
    let visible = logic.visible_line_count() as f32;
    let expected_height = visible / total;
    assert!((sb.thumb_height_ratio - expected_height).abs() < 1e-5);
}

#[test]
fn scrollbar_thumb_top_is_zero_at_doc_start() {
    let mut logic = overflowing_app();
    logic.execute(Action::CursorDocStart { selecting: false });
    let frame = logic.render_frame();
    let sb = frame.scrollbar.as_ref().expect("scrollbar should exist");
    assert_eq!(sb.thumb_top_ratio, 0.0);
}

#[test]
fn scrollbar_thumb_top_increases_after_scroll_down() {
    let mut logic = overflowing_app();
    logic.execute(Action::PageDown { selecting: false });
    let frame = logic.render_frame();
    let sb = frame.scrollbar.as_ref().expect("scrollbar should exist");
    assert!(sb.thumb_top_ratio > 0.0, "thumb_top_ratio should move down after page-down");
}
