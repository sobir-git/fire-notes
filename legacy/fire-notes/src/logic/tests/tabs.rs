//! Tab management tests.
#![allow(unused_must_use)]

use crate::app::action::Action;
use super::{app, type_str, line};

#[test]
fn new_tab_increases_tab_count() {
    let mut logic = app();
    logic.execute(Action::NewTab);
    assert_eq!(logic.tabs.len(), 2);
}

#[test]
fn close_tab_decreases_count() {
    let mut logic = app();
    logic.execute(Action::NewTab);
    logic.execute(Action::CloseTab);
    assert_eq!(logic.tabs.len(), 1);
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
    assert_eq!(line(&logic, 0), "tab1");
    logic.execute(Action::PreviousTab);
    assert_eq!(line(&logic, 0), "tab0");
}

#[test]
fn two_tabs_second_is_active() {
    let mut logic = app();
    type_str(&mut logic, "first");
    logic.execute(Action::NewTab);
    type_str(&mut logic, "second");
    assert_eq!(logic.tabs.len(), 2);
    assert_eq!(logic.active_tab, 1);
}
