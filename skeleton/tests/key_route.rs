#[cfg(test)]
mod tests {
    use fire_ui_contract_model::*;
    use std::{cell::RefCell, rc::Rc};
    type Log = Rc<RefCell<Vec<&'static str>>>;
    struct Parent;
    impl Widget for Parent {
        fn event(&mut self, cx: &mut Cx<'_>, phase: Phase, input: &Input) {
            if phase == Phase::Preview && matches!(input, Input::Key { down: true, .. }) {
                cx.focus().unwrap();
            }
        }
    }
    struct Game {
        held: Rc<RefCell<bool>>,
        log: Log,
    }
    impl Widget for Game {
        fn event(&mut self, cx: &mut Cx<'_>, phase: Phase, input: &Input) {
            if phase != Phase::Target {
                return;
            }
            match input {
                Input::Button { down: true, .. } => cx.focus().unwrap(),
                Input::Key { down, .. } => {
                    *self.held.borrow_mut() = *down;
                    self.log
                        .borrow_mut()
                        .push(if *down { "down" } else { "up" });
                }
                Input::CancelKeys => {
                    *self.held.borrow_mut() = false;
                    self.log.borrow_mut().push("cancel");
                }
                _ => {}
            }
        }
    }
    #[test]
    fn preview_focus_transfer_must_not_leave_old_target_holding_key() {
        let mut ui = Ui::new(73, 16);
        let held = Rc::new(RefCell::new(false));
        let log = Log::default();
        let parent = ui.insert(None, 0, Parent).unwrap();
        let child = ui
            .insert(
                Some(parent),
                0,
                Game {
                    held: held.clone(),
                    log: log.clone(),
                },
            )
            .unwrap();
        ui.pump(16);
        ui.dispatch(Input::Button {
            pointer: 1,
            button: 1,
            down: true,
            position: Point(5., 5.),
        });
        assert_eq!(ui.focused(), Some(child));
        for down in [true, false] {
            ui.dispatch(Input::Key {
                physical: 37,
                logical: "ArrowLeft".into(),
                down,
                repeat: false,
            });
        }
        assert_eq!(ui.focused(), Some(parent));
        assert!(
            !*held.borrow(),
            "unfocused game keeps key held; delivery log: {:?}",
            log.borrow()
        );
    }
}
