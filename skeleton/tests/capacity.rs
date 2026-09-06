#[cfg(test)]
mod capacity_tests {
    use fire_ui_contract_model::*;
    struct Empty;
    impl Widget for Empty {}
    struct Owner;
    impl Widget for Owner {
        fn action(&mut self, cx: &mut Cx<'_>, a: Action) -> Option<Action> {
            let _ = cx.focus();
            Some(a)
        }
    }
    struct Child;
    impl Widget for Child {
        fn event(&mut self, cx: &mut Cx<'_>, phase: Phase, e: &Input) {
            if phase != Phase::Target {
                return;
            }
            match e {
                Input::Button { down: true, .. } => {
                    cx.focus().unwrap();
                    cx.emit(1u32).unwrap();
                }
                Input::Focus(false) => {
                    let _ = cx.insert(0, Empty);
                    let _ = cx.emit(2u32);
                    let _ = cx.schedule(0, Some(Wake::At(10)), Lifetime::Mounted);
                }
                _ => {}
            }
        }
    }
    #[test]
    fn forwarding_and_focus_notice_requests_stay_bounded_at_small_capacities() {
        for capacity in 1..=8 {
            let mut ui = Ui::new(capacity as u64 + 100, capacity);
            let root = ui.insert(None, 0, Empty).unwrap();
            ui.pump(1);
            let owner = ui.insert(Some(root), 0, Owner).unwrap();
            ui.pump(1);
            ui.insert(Some(owner), 0, Child).unwrap();
            ui.pump(1);
            ui.dispatch(Input::Button {
                pointer: 1,
                button: 1,
                down: true,
                position: Point(5., 5.),
            });
            for _ in 0..40 {
                assert!(ui.queued() <= capacity, "capacity {capacity}");
                if ui.pump(1) == 0 {
                    break;
                }
            }
            assert_eq!(ui.queued(), 0);
            assert!(ui
                .app_actions
                .iter()
                .any(|a| a.payload.downcast_ref::<u32>() == Some(&1)));
        }
    }
}
