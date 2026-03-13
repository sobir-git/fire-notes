//! AppLogic state integrity tests.
#![allow(unused_must_use)]

use crate::app::action::Action;
use super::{app, type_str, total_lines, scroll_offset};

#[test]
fn initial_state_has_one_tab() {
    let logic = app();
    assert_eq!(logic.tabs.len(), 1);
    assert_eq!(logic.active_tab, 0);
}

#[test]
fn active_tab_index_is_consistent() {
    let mut logic = app();
    logic.execute(Action::NewTab);
    logic.execute(Action::NewTab);
    logic.execute(Action::PreviousTab);
    assert_eq!(logic.active_tab, 1);
    assert!(logic.active_tab < logic.tabs.len());
}

#[test]
fn headless_dimensions_are_correct() {
    let logic = app();
    assert_eq!(logic.width, 800.0);
    assert_eq!(logic.height, 600.0);
    assert_eq!(logic.scale, 1.0);
}

#[test]
fn total_lines_matches_content() {
    let mut logic = app();
    type_str(&mut logic, "a");
    logic.execute(Action::InsertChar('\n'));
    type_str(&mut logic, "b");
    assert_eq!(total_lines(&logic), 2);
}

#[test]
fn scroll_offset_zero_at_start() {
    let logic = app();
    assert_eq!(scroll_offset(&logic), 0);
}
