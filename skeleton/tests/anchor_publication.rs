#[cfg(test)]
mod tests {
    use fire_ui_contract_model::*;
    use std::{cell::Cell, rc::Rc};
    struct Empty;
    impl Widget for Empty {}
    struct Notices(Rc<Cell<usize>>);
    impl Widget for Notices {
        fn event(&mut self, _: &mut Cx<'_>, _: Phase, e: &Input) {
            if matches!(e, Input::AnchorUnavailable { .. }) {
                self.0.set(self.0.get() + 1)
            }
        }
    }
    struct Animated(Rc<Cell<usize>>);
    impl Widget for Animated {
        fn event(&mut self, cx: &mut Cx<'_>, _: Phase, e: &Input) {
            match e {
                Input::Hidden(false) => {
                    cx.schedule(0, Some(Wake::Frame), Lifetime::Visible)
                        .unwrap();
                }
                Input::Frame { .. } => self.0.set(self.0.get() + 1),
                _ => {}
            }
        }
    }
    #[test]
    fn mount_of_pending_anchor_restarts_existing_overlay() {
        let mut ui = Ui::new(1000, 16);
        let owner = ui.insert(None, 0, Empty).unwrap();
        let frames = Rc::new(Cell::new(0));
        let popup = ui.insert(Some(owner), 0, Animated(frames.clone())).unwrap();
        ui.pump(16);
        let anchor = ui.insert(Some(owner), 1, Empty).unwrap();
        ui.place(
            popup,
            Placement {
                overlay_anchor: Some(anchor),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(ui.world(popup).is_none());
        ui.pump(16);
        assert!(ui.world(popup).is_some());
        ui.frame(1);
        assert_eq!(
            frames.get(),
            1,
            "effective visibility recovered but no Hidden(false) restarted the animation"
        );
    }
    #[test]
    fn hidden_overlay_owner_still_learns_anchor_removal() {
        let mut ui = Ui::new(1001, 16);
        let notices = Rc::new(Cell::new(0));
        let owner = ui.insert(None, 0, Notices(notices.clone())).unwrap();
        let anchor = ui.insert(Some(owner), 0, Empty).unwrap();
        let popup = ui.insert(Some(owner), 1, Empty).unwrap();
        ui.pump(16);
        ui.place(
            popup,
            Placement {
                overlay_anchor: Some(anchor),
                ..Default::default()
            },
        )
        .unwrap();
        ui.hide(popup, true).unwrap();
        ui.remove(anchor).unwrap();
        assert_eq!(notices.get(), 1);
    }
}
#[cfg(test)]
mod c6_tests {
    use fire_ui_contract_model::*;
    use std::{cell::Cell, rc::Rc};
    struct Empty;
    impl Widget for Empty {}
    struct Notices(Rc<Cell<usize>>);
    impl Widget for Notices {
        fn event(&mut self, _: &mut Cx<'_>, _: Phase, e: &Input) {
            if matches!(e, Input::AnchorUnavailable { .. }) {
                self.0.set(self.0.get() + 1)
            }
        }
    }
    struct RemovePreallocatedChild;
    impl Widget for RemovePreallocatedChild {
        fn mount(&mut self, cx: &mut Cx<'_>) {
            cx.remove(cx.child(0).unwrap()).unwrap();
        }
    }
    #[test]
    fn anchor_removal_during_mount_is_published_once() {
        let mut ui = Ui::new(2000, 16);
        let notices = Rc::new(Cell::new(0));
        let root = ui.insert(None, 0, Notices(notices.clone())).unwrap();
        let popup = ui.insert(Some(root), 0, Empty).unwrap();
        ui.pump(16);
        let parent = ui.insert(Some(root), 1, RemovePreallocatedChild).unwrap();
        let anchor = ui.insert(Some(parent), 0, Empty).unwrap();
        ui.place(
            popup,
            Placement {
                overlay_anchor: Some(anchor),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(notices.get(), 0);
        ui.pump(1);
        assert!(!ui.live(anchor));
        assert_eq!(
            notices.get(),
            1,
            "one removal must produce one anchor-loss notification"
        );
    }
    #[test]
    fn hidden_anchor_repeated_publication_and_reuse_do_not_duplicate_loss() {
        let mut ui = Ui::new(2001, 16);
        let notices = Rc::new(Cell::new(0));
        let root = ui.insert(None, 0, Notices(notices.clone())).unwrap();
        let popup = ui.insert(Some(root), 0, Empty).unwrap();
        let anchor = ui.insert(Some(root), 1, Empty).unwrap();
        ui.pump(16);
        ui.place(
            popup,
            Placement {
                overlay_anchor: Some(anchor),
                ..Default::default()
            },
        )
        .unwrap();
        ui.hide(popup, true).unwrap();
        ui.hide(anchor, true).unwrap();
        assert_eq!(notices.get(), 1);
        ui.hide(anchor, true).unwrap();
        ui.place(root, Placement::default()).unwrap();
        assert_eq!(notices.get(), 1);
        ui.hide(anchor, false).unwrap();
        ui.hide(anchor, true).unwrap();
        assert_eq!(notices.get(), 2);
        ui.remove(anchor).unwrap();
        assert_eq!(notices.get(), 3);
        let replacement = ui.insert(Some(root), 1, Empty).unwrap();
        ui.pump(16);
        assert_eq!(replacement.slot, anchor.slot);
        assert!(ui.world(popup).is_none());
        assert_eq!(notices.get(), 3);
    }
}
#[cfg(test)]
mod c7_tests {
    use fire_ui_contract_model::*;
    use std::{cell::RefCell, rc::Rc};
    type Log = Rc<RefCell<Vec<&'static str>>>;
    struct Empty;
    impl Widget for Empty {}
    struct Watch {
        log: Log,
        name: &'static str,
        remove_child_on_show: bool,
    }
    impl Widget for Watch {
        fn mount(&mut self, _: &mut Cx<'_>) {
            self.log.borrow_mut().push("mount");
        }
        fn event(&mut self, cx: &mut Cx<'_>, _: Phase, e: &Input) {
            match e {
                Input::Hidden(false) => {
                    self.log.borrow_mut().push(self.name);
                    if self.remove_child_on_show {
                        cx.remove(cx.child(0).unwrap()).unwrap();
                    }
                }
                Input::AnchorUnavailable { .. } => self.log.borrow_mut().push("loss"),
                _ => {}
            }
        }
    }
    #[test]
    fn nested_removal_during_anchor_recovery_does_not_publish_stale_show() {
        let log = Log::default();
        let mut ui = Ui::new(3000, 16);
        let root = ui
            .insert(
                None,
                0,
                Watch {
                    log: log.clone(),
                    name: "root",
                    remove_child_on_show: false,
                },
            )
            .unwrap();
        let popup = ui
            .insert(
                Some(root),
                0,
                Watch {
                    log: log.clone(),
                    name: "popup",
                    remove_child_on_show: true,
                },
            )
            .unwrap();
        let child_anchor = ui.insert(Some(popup), 0, Empty).unwrap();
        let dependent = ui
            .insert(
                Some(root),
                1,
                Watch {
                    log: log.clone(),
                    name: "dependent",
                    remove_child_on_show: false,
                },
            )
            .unwrap();
        ui.pump(16);
        ui.place(
            dependent,
            Placement {
                overlay_anchor: Some(child_anchor),
                ..Default::default()
            },
        )
        .unwrap();
        let pending_anchor = ui
            .insert(
                Some(root),
                2,
                Watch {
                    log: log.clone(),
                    name: "anchor",
                    remove_child_on_show: false,
                },
            )
            .unwrap();
        ui.place(
            popup,
            Placement {
                overlay_anchor: Some(pending_anchor),
                ..Default::default()
            },
        )
        .unwrap();
        log.borrow_mut().clear();
        ui.pump(1);
        assert_eq!(&*log.borrow(), &["mount", "popup", "loss"]);
        assert!(!ui.live(child_anchor));
        assert!(ui.world(dependent).is_none());
        ui.place(root, Placement::default()).unwrap();
        assert_eq!(&*log.borrow(), &["mount", "popup", "loss"]);
    }
    struct RemovePopupOnLoss;
    impl Widget for RemovePopupOnLoss {
        fn event(&mut self, cx: &mut Cx<'_>, _: Phase, e: &Input) {
            if let Input::AnchorUnavailable { overlay, .. } = e {
                cx.remove(*overlay).unwrap();
            }
        }
    }
    #[test]
    fn anchor_owner_cleanup_is_queued_and_bounded() {
        let mut ui = Ui::new(3001, 1);
        let root = ui.insert(None, 0, RemovePopupOnLoss).unwrap();
        ui.pump(1);
        let anchor = ui.insert(Some(root), 0, Empty).unwrap();
        ui.pump(1);
        let popup = ui.insert(Some(root), 1, Empty).unwrap();
        ui.pump(1);
        ui.place(
            popup,
            Placement {
                overlay_anchor: Some(anchor),
                ..Default::default()
            },
        )
        .unwrap();
        ui.remove(anchor).unwrap();
        assert!(ui.live(popup));
        assert_eq!(ui.queued(), 1);
        assert!(ui.next_work().ready);
        ui.pump(1);
        assert!(!ui.live(popup));
        assert!(ui.next_work().idle());
    }
    #[test]
    fn mount_initializes_silently_then_publishes_real_show_once() {
        let log = Log::default();
        let mut ui = Ui::new(3002, 2);
        let k = ui
            .insert(
                None,
                0,
                Watch {
                    log: log.clone(),
                    name: "show",
                    remove_child_on_show: false,
                },
            )
            .unwrap();
        ui.hide(k, true).unwrap();
        ui.pump(1);
        assert_eq!(&*log.borrow(), &["mount"]);
        ui.hide(k, false).unwrap();
        ui.hide(k, false).unwrap();
        assert_eq!(&*log.borrow(), &["mount", "show"]);
    }
}
