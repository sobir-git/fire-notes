use fire_ui_contract_model::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
type Log = Rc<RefCell<Vec<String>>>;
fn click(x: f64) -> Input {
    Input::Button {
        pointer: 1,
        button: 1,
        down: true,
        position: Point(x, 5.),
    }
}
struct MountLog(Log);
impl Widget for MountLog {
    fn mount(&mut self, _: &mut Cx<'_>) {
        self.0.borrow_mut().push("mount".into());
    }
    fn event(&mut self, _: &mut Cx<'_>, phase: Phase, e: &Input) {
        if phase == Phase::Target && matches!(e, Input::Button { .. }) {
            self.0.borrow_mut().push("input".into());
        }
    }
}
struct SpawnMount(Log);
impl Widget for SpawnMount {
    fn mount(&mut self, cx: &mut Cx<'_>) {
        cx.insert(0, MountLog(self.0.clone())).unwrap();
    }
}
#[test]
fn input_waits_until_initial_mount() {
    let mut ui = Ui::new(1, 10);
    let log = Log::default();
    ui.insert(None, 0, SpawnMount(log.clone())).unwrap();
    ui.pump(1); // The delivery budget expires after the owner's mount inserts a child.
    ui.dispatch(click(5.));
    ui.pump(1);
    assert_eq!(&*log.borrow(), &["mount"]);
}
struct FocusLog {
    name: &'static str,
    reclaim: bool,
    log: Log,
}
impl Widget for FocusLog {
    fn event(&mut self, cx: &mut Cx<'_>, phase: Phase, e: &Input) {
        if phase != Phase::Target {
            return;
        }
        match e {
            Input::Button { down: true, .. } => {
                cx.focus().unwrap();
            }
            Input::Focus(value) => {
                self.log.borrow_mut().push(format!("{}:{value}", self.name));
                if !value && self.reclaim {
                    self.reclaim = false;
                    cx.focus().unwrap();
                }
            }
            _ => {}
        }
    }
}
#[test]
fn focus_notifications_complete_before_reentrant_requests() {
    let mut ui = Ui::new(1, 10);
    let log = Log::default();
    let a = ui
        .insert(
            None,
            0,
            FocusLog {
                name: "a",
                reclaim: true,
                log: log.clone(),
            },
        )
        .unwrap();
    let b = ui
        .insert(
            None,
            1,
            FocusLog {
                name: "b",
                reclaim: false,
                log: log.clone(),
            },
        )
        .unwrap();
    ui.pump(10);
    ui.place(
        b,
        Placement {
            transform: Affine([1., 0., 0., 1., 200., 0.]),
            ..Default::default()
        },
    )
    .unwrap();
    ui.dispatch(click(5.));
    ui.dispatch(click(205.));
    assert_eq!(ui.focused(), Some(b));
    ui.pump(10);
    assert_eq!(ui.focused(), Some(a));
    assert_eq!(
        &*log.borrow(),
        &["a:true", "a:false", "b:true", "b:false", "a:true"]
    );
}
struct CaptureLog {
    cancelled: Rc<Cell<usize>>,
}
impl Widget for CaptureLog {
    fn event(&mut self, cx: &mut Cx<'_>, phase: Phase, e: &Input) {
        if let Input::CaptureLost { pointer: 1 } = e {
            self.cancelled.set(self.cancelled.get() + 1);
        }
        if phase != Phase::Bubble && matches!(e, Input::Button { down: true, .. }) {
            cx.capture(1, true).unwrap();
        }
    }
}
#[test]
fn capture_replacement_notifies_the_old_owner() {
    let mut ui = Ui::new(1, 10);
    let cancelled = Rc::new(Cell::new(0));
    let parent = ui
        .insert(
            None,
            0,
            CaptureLog {
                cancelled: cancelled.clone(),
            },
        )
        .unwrap();
    let child = ui
        .insert(
            Some(parent),
            0,
            CaptureLog {
                cancelled: Rc::default(),
            },
        )
        .unwrap();
    ui.pump(10);
    ui.dispatch(click(5.));
    assert_eq!(ui.captured(1), Some(child));
    assert_eq!(cancelled.get(), 1);
}
struct CaptureOnly;
impl Widget for CaptureOnly {
    fn event(&mut self, cx: &mut Cx<'_>, phase: Phase, e: &Input) {
        if phase == Phase::Target {
            if let Input::Button {
                pointer,
                down: true,
                ..
            } = e
            {
                cx.capture(*pointer, true).unwrap();
            }
        }
    }
}
#[test]
fn removing_focus_preserves_unrelated_captured_pointer_buttons() {
    let mut ui = Ui::new(1, 10);
    let a = ui
        .insert(
            None,
            0,
            FocusLog {
                name: "a",
                reclaim: false,
                log: Log::default(),
            },
        )
        .unwrap();
    let b = ui.insert(None, 1, CaptureOnly).unwrap();
    ui.pump(10);
    ui.place(
        b,
        Placement {
            transform: Affine([1., 0., 0., 1., 200., 0.]),
            ..Default::default()
        },
    )
    .unwrap();
    ui.dispatch(click(5.));
    for button in [1, 2] {
        ui.dispatch(Input::Button {
            pointer: 2,
            button,
            down: true,
            position: Point(205., 5.),
        });
    }
    assert_eq!(ui.captured(2), Some(b));
    ui.remove(a).unwrap();
    ui.dispatch(Input::Button {
        pointer: 2,
        button: 1,
        down: false,
        position: Point(205., 5.),
    });
    assert_eq!(ui.captured(2), Some(b)); // Button 2 has never been released.
}

struct Reclaim;
impl Widget for Reclaim {
    fn event(&mut self, cx: &mut Cx<'_>, phase: Phase, e: &Input) {
        if phase == Phase::Target
            && matches!(e, Input::Button { down: true, .. } | Input::Focus(false))
        {
            let _ = cx.focus();
        }
    }
}
#[test]
fn adversarial_focus_chain_yields_and_blur_prevents_reacquisition() {
    let mut ui = Ui::new(12, 2);
    ui.insert(None, 0, Reclaim).unwrap();
    ui.pump(2);
    let b = ui.insert(None, 1, Reclaim).unwrap();
    ui.pump(2);
    ui.place(
        b,
        Placement {
            transform: Affine([1., 0., 0., 1., 200., 0.]),
            ..Default::default()
        },
    )
    .unwrap();
    ui.dispatch(click(5.));
    ui.dispatch(click(205.));
    assert_eq!(ui.pump(5), 5);
    assert_eq!(ui.queued(), 1);
    ui.blur();
    ui.pump(10);
    assert_eq!(ui.focused(), None);
    assert!(ui.next_work().idle());
}
#[test]
fn nested_modal_close_restores_eligible_focus_and_rejects_reused_slot() {
    let mut ui = Ui::new(13, 20);
    let base = ui
        .insert(
            None,
            0,
            FocusLog {
                name: "base",
                reclaim: false,
                log: Log::default(),
            },
        )
        .unwrap();
    let modal = ui.insert(None, 1, Reclaim).unwrap();
    let inner = ui.insert(Some(modal), 0, Reclaim).unwrap();
    ui.pump(20);
    ui.place(
        modal,
        Placement {
            transform: Affine([1., 0., 0., 1., 200., 0.]),
            ..Default::default()
        },
    )
    .unwrap();
    ui.dispatch(click(5.));
    assert_eq!(ui.focused(), Some(base));
    ui.set_modal(Some(modal)).unwrap();
    assert_eq!(ui.focused(), Some(modal));
    ui.set_modal(Some(inner)).unwrap();
    assert_eq!(ui.focused(), Some(inner));
    ui.set_modal(None).unwrap();
    assert_eq!(ui.focused(), Some(modal));
    ui.set_modal(None).unwrap();
    assert_eq!(ui.focused(), Some(base));
    ui.pump(20); // Reclaim callbacks are application policy; isolate saved-key check below.
    ui.dispatch(click(5.));
    ui.set_modal(Some(modal)).unwrap();
    ui.remove(base).unwrap();
    let replacement = ui.insert(None, 2, Reclaim).unwrap();
    ui.pump(20);
    ui.set_modal(None).unwrap();
    assert_ne!(ui.focused(), Some(replacement));
}
struct VisibilityLog(Log);
impl Widget for VisibilityLog {
    fn event(&mut self, cx: &mut Cx<'_>, _: Phase, e: &Input) {
        match e {
            Input::Hidden(false) => {
                cx.schedule(0, Some(Wake::Frame), Lifetime::Visible)
                    .unwrap();
            }
            Input::AnchorUnavailable { .. } => {
                self.0.borrow_mut().push("anchor unavailable".into())
            }
            _ => {}
        }
    }
}
#[test]
fn singular_anchor_suppresses_overlay_and_recovery_restarts_frames() {
    let mut ui = Ui::new(14, 20);
    let log = Log::default();
    let owner = ui.insert(None, 0, VisibilityLog(log.clone())).unwrap();
    let anchor = ui.insert(Some(owner), 0, Reclaim).unwrap();
    let overlay = ui
        .insert(Some(owner), 1, VisibilityLog(Log::default()))
        .unwrap();
    ui.pump(20);
    ui.place(
        overlay,
        Placement {
            overlay_anchor: Some(anchor),
            ..Default::default()
        },
    )
    .unwrap();
    ui.schedule(overlay, 0, Some(Wake::Frame), Lifetime::Visible)
        .unwrap();
    ui.place(
        anchor,
        Placement {
            transform: Affine([0.; 6]),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(!ui.next_work().frame);
    assert_eq!(log.borrow().len(), 1);
    ui.place(anchor, Placement::default()).unwrap();
    assert!(ui.next_work().frame);
    ui.remove(anchor).unwrap();
    assert_eq!(log.borrow().len(), 2);
    assert!(ui.world(overlay).is_none());
}
