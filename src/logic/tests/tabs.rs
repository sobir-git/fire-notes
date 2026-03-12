//! Tab management tests.
#![allow(unused_must_use)]

use crate::app::action::Action;
use super::{app, type_str};

#[test]
fn new_tab_increases_tab_count() {
    let mut logic = app();
    logic.execute(Action::NewTab);
    let frame = logic.render_frame();
    assert_eq!(frame.tabs.len(), 2);
}

#[test]
fn close_tab_decreases_count() {
    let mut logic = app();
    logic.execute(Action::NewTab);
    logic.execute(Action::CloseTab);
    let frame = logic.render_frame();
    assert_eq!(frame.tabs.len(), 1);
}

#[test]
fn next_and_previous_tab_switches_active() {
    let mut logic = app();
    logic.execute(Action::NewTab);
    assert_eq!(logic.active_tab, 1);
    logic.execute(Action::PreviousTab);
    assert_eq!(logic.active_tab, 0);
    logic.execute(Action::NextTab);
    assert_eq!(logic.active_tab, 1);
}

#[test]
fn tabs_are_independent_buffers() {
    let mut logic = app();
    type_str(&mut logic, "tab0");
    logic.execute(Action::NewTab);
    type_str(&mut logic, "tab1");

    let frame = logic.render_frame();
    assert_eq!(frame.visible_lines[0].text, "tab1");

    logic.execute(Action::PreviousTab);
    let frame = logic.render_frame();
    assert_eq!(frame.visible_lines[0].text, "tab0");
}

#[test]
fn snapshot_two_tabs_bar() {
    let mut logic = app();
    type_str(&mut logic, "first");
    logic.execute(Action::NewTab);
    type_str(&mut logic, "second");
    let frame = logic.render_frame();
    assert_eq!(frame.tabs.len(), 2);
    assert!(!frame.tabs[0].is_active);
    assert!(frame.tabs[1].is_active);
}
