use fire_ui_contract_model::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct Noop;
impl Widget for Noop {}
struct Count(Rc<Cell<usize>>);
impl Widget for Count {
    fn action(&mut self, _: &mut Cx<'_>, _: Action) -> Option<Action> {
        self.0.set(self.0.get() + 1);
        None
    }
}
#[test]
fn queued_task_is_revalidated_at_delivery() {
    let mut ui = Ui::new(1, 10);
    let count = Rc::new(Cell::new(0));
    let k = ui.insert(None, 0, Count(count.clone())).unwrap();
    ui.pump(10);
    let a = ui.begin_task(k, 0).unwrap();
    ui.receive(a, ()).unwrap();
    ui.begin_task(k, 0).unwrap();
    ui.pump(10);
    assert_eq!(count.get(), 0);
}
struct ForwardAndEmit(Rc<Cell<bool>>);
impl Widget for ForwardAndEmit {
    fn action(&mut self, cx: &mut Cx<'_>, a: Action) -> Option<Action> {
        self.0.set(cx.emit(42).is_err());
        Some(a)
    }
}
struct Emit;
impl Widget for Emit {
    fn event(&mut self, cx: &mut Cx<'_>, p: Phase, e: &Input) {
        if p == Phase::Target && matches!(e, Input::Button { down: true, .. }) {
            cx.emit(()).unwrap();
        }
    }
}
#[test]
fn forwarding_reserves_capacity_and_reports_backpressure() {
    let mut ui = Ui::new(2, 1);
    let full = Rc::new(Cell::new(false));
    let r = ui.insert(None, 0, Noop).unwrap();
    ui.pump(1);
    let parent = ui.insert(Some(r), 0, ForwardAndEmit(full.clone())).unwrap();
    ui.pump(1);
    ui.insert(Some(parent), 0, Emit).unwrap();
    ui.pump(1);
    ui.dispatch(Input::Button {
        pointer: 1,
        button: 1,
        down: true,
        position: Point(10., 10.),
    });
    assert_eq!(ui.queued(), 1);
    ui.pump(1);
    assert!(full.get());
    assert_eq!(ui.queued(), 1);
    ui.pump(1);
    assert_eq!(ui.app_actions.len(), 1);
}
struct SpawnOnCleanup(Rc<Cell<bool>>);
impl Widget for SpawnOnCleanup {
    fn event(&mut self, cx: &mut Cx<'_>, _: Phase, e: &Input) {
        if matches!(e, Input::Hidden(true)) {
            cx.insert(0, Noop).unwrap();
        }
        if matches!(e, Input::Unmount) {
            self.0.set(
                cx.insert(0, Noop).is_err()
                    && cx
                        .schedule(0, Some(Wake::Frame), Lifetime::Mounted)
                        .is_err(),
            );
        }
    }
}
#[test]
fn retirement_rejects_work_and_does_not_emit_ordinary_hide() {
    let mut ui = Ui::new(3, 10);
    let denied = Rc::new(Cell::new(false));
    let k = ui.insert(None, 0, SpawnOnCleanup(denied.clone())).unwrap();
    ui.pump(10);
    ui.remove(k).unwrap();
    assert_eq!(ui.mounted(), 0);
    assert!(denied.get());
    assert!(ui.next_work().idle());
}
struct CancelSecond(Rc<Cell<usize>>);
impl Widget for CancelSecond {
    fn event(&mut self, cx: &mut Cx<'_>, _: Phase, e: &Input) {
        if let Input::Wake(slot) = e {
            if *slot == 0 {
                cx.schedule(1, None, Lifetime::Mounted).unwrap()
            } else {
                self.0.set(self.0.get() + 1)
            }
        }
    }
}
#[test]
fn cancellation_invalidates_already_collected_due_work() {
    let mut ui = Ui::new(4, 10);
    let count = Rc::new(Cell::new(0));
    let k = ui.insert(None, 0, CancelSecond(count.clone())).unwrap();
    ui.pump(10);
    ui.schedule(k, 0, Some(Wake::At(1)), Lifetime::Mounted)
        .unwrap();
    ui.schedule(k, 1, Some(Wake::At(1)), Lifetime::Mounted)
        .unwrap();
    ui.advance(1);
    assert_eq!(count.get(), 0);
}
struct CaptureAndStop;
impl Widget for CaptureAndStop {
    fn event(&mut self, cx: &mut Cx<'_>, p: Phase, e: &Input) {
        if p != Phase::Target {
            return;
        }
        if let Input::Button { pointer, down, .. } = e {
            if *down {
                cx.capture(*pointer, true).unwrap()
            } else {
                cx.stop = true
            }
        }
    }
}
#[test]
fn stopping_release_propagation_does_not_keep_capture() {
    let mut ui = Ui::new(5, 10);
    let k = ui.insert(None, 0, CaptureAndStop).unwrap();
    ui.pump(10);
    for button in [1, 2] {
        ui.dispatch(Input::Button {
            pointer: 1,
            button,
            down: true,
            position: Point(10., 10.),
        })
    }
    ui.dispatch(Input::Button {
        pointer: 1,
        button: 1,
        down: false,
        position: Point(10., 10.),
    });
    assert_eq!(ui.captured(1), Some(k));
    ui.dispatch(Input::Button {
        pointer: 1,
        button: 2,
        down: false,
        position: Point(10., 10.),
    });
    assert_eq!(ui.captured(1), None);
}
#[test]
fn overlay_tracks_effective_anchor_visibility_and_rejects_cycles() {
    let mut ui = Ui::new(6, 10);
    let r = ui.insert(None, 0, Noop).unwrap();
    let branch = ui.insert(Some(r), 0, Noop).unwrap();
    let anchor = ui.insert(Some(branch), 0, Noop).unwrap();
    let popup = ui.insert(Some(r), 1, Noop).unwrap();
    ui.pump(10);
    ui.place(
        popup,
        Placement {
            overlay_anchor: Some(anchor),
            ..Default::default()
        },
    )
    .unwrap();
    ui.hide(branch, true).unwrap();
    assert!(!ui.active(popup));
    ui.hide(branch, false).unwrap();
    assert!(ui.active(popup));
    assert_eq!(
        ui.place(
            anchor,
            Placement {
                overlay_anchor: Some(popup),
                ..Default::default()
            }
        ),
        Err(Error::InvalidGeometry)
    );
}
struct DuplicateChildren(Rc<RefCell<Vec<Result<Key, Error>>>>);
impl Widget for DuplicateChildren {
    fn mount(&mut self, cx: &mut Cx<'_>) {
        cx.insert(0, Noop).unwrap();
        cx.insert(0, Noop).unwrap();
    }
    fn event(&mut self, _: &mut Cx<'_>, _: Phase, e: &Input) {
        if let Input::Committed { result, .. } = e {
            self.0.borrow_mut().push(*result);
        }
    }
}
#[test]
fn structural_commands_return_individual_commit_results() {
    let mut ui = Ui::new(7, 10);
    let results = Rc::default();
    let k = ui
        .insert(None, 0, DuplicateChildren(Rc::clone(&results)))
        .unwrap();
    ui.pump(10);
    assert_eq!(results.borrow().len(), 2);
    assert_eq!(results.borrow()[0], Ok(ui.child(k, 0).unwrap()));
    assert_eq!(results.borrow()[1], Err(Error::DuplicateToken));
    assert_eq!(ui.mounted(), 2);
}
#[test]
fn host_sees_frame_request_and_earlier_timer_together() {
    let mut ui = Ui::new(8, 10);
    let k = ui.insert(None, 0, Noop).unwrap();
    ui.pump(10);
    ui.schedule(k, 0, Some(Wake::Frame), Lifetime::Visible)
        .unwrap();
    ui.schedule(k, 1, Some(Wake::At(5)), Lifetime::Mounted)
        .unwrap();
    let w = ui.next_work();
    assert!(w.frame);
    assert_eq!(w.deadline, Some(5));
    let next_frame = 16;
    assert_eq!(w.deadline.unwrap().min(next_frame), 5);
}
struct RestartFrame(Rc<Cell<usize>>);
impl Widget for RestartFrame {
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
fn child_receives_effective_visibility_change_to_restart_animation() {
    let mut ui = Ui::new(9, 10);
    let r = ui.insert(None, 0, Noop).unwrap();
    let count = Rc::new(Cell::new(0));
    let k = ui.insert(Some(r), 0, RestartFrame(count.clone())).unwrap();
    ui.pump(10);
    ui.schedule(k, 0, Some(Wake::Frame), Lifetime::Visible)
        .unwrap();
    ui.hide(r, true).unwrap();
    assert!(!ui.next_work().frame);
    ui.hide(r, false).unwrap();
    assert!(ui.next_work().frame);
    ui.frame(16);
    assert_eq!(count.get(), 1);
}
struct TextCounter(Rc<Cell<usize>>);
impl Widget for TextCounter {
    fn event(&mut self, cx: &mut Cx<'_>, p: Phase, e: &Input) {
        if p == Phase::Target && matches!(e, Input::Button { down: true, .. }) {
            cx.focus().unwrap();
        }
        if matches!(e, Input::Text { .. }) {
            self.0.set(self.0.get() + 1);
        }
    }
}
#[test]
fn text_delivery_is_target_only() {
    let mut ui = Ui::new(10, 10);
    let parent_count = Rc::new(Cell::new(0));
    let child_count = Rc::new(Cell::new(0));
    let r = ui
        .insert(None, 0, TextCounter(parent_count.clone()))
        .unwrap();
    ui.insert(Some(r), 0, TextCounter(child_count.clone()))
        .unwrap();
    ui.pump(10);
    ui.dispatch(Input::Button {
        pointer: 1,
        button: 1,
        down: true,
        position: Point(10., 10.),
    });
    ui.dispatch(Input::Text {
        session: ui.session(),
        text: "hello".into(),
    });
    assert_eq!(child_count.get(), 1);
    assert_eq!(parent_count.get(), 0);
}
