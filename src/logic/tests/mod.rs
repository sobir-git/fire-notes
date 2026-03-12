//! Headless AppLogic integration tests — no GPU or platform deps.

mod typing;
mod tabs;
mod cursor;
mod selection;
mod scroll;
mod clipboard;
mod frame;

use crate::logic::AppLogic;
use crate::app::action::Action;

pub(super) fn app() -> AppLogic {
    AppLogic::new_headless(800.0, 600.0, 1.0)
}

pub(super) fn type_str(logic: &mut AppLogic, s: &str) {
    for ch in s.chars() {
        let _ = logic.execute(Action::InsertChar(ch));
    }
}
