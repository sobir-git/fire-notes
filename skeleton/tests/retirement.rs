use fire_ui_contract_model::*;
use std::{cell::RefCell, rc::Rc};
type Log = Rc<RefCell<Vec<&'static str>>>;
struct Control(Log);
impl Widget for Control {
    fn mount(&mut self, _: &mut Cx<'_>) {
        self.0.borrow_mut().push("mount");
    }
    fn event(&mut self, cx: &mut Cx<'_>, phase: Phase, e: &Input) {
        if phase != Phase::Target {
            return;
        }
        match e {
            Input::Button { down: true, .. } => cx.focus().unwrap(),
            Input::Wake(_) => self.0.borrow_mut().push("wake"),
            Input::Frame { .. } => self.0.borrow_mut().push("frame"),
            Input::Unmount => self.0.borrow_mut().push("unmount"),
            _ => {}
        }
    }
}
fn click(x: f64) -> Input {
    Input::Button {
        pointer: 1,
        button: 1,
        down: true,
        position: Point(x, 5.),
    }
}
#[test]
fn unmounted_nodes_do_not_receive_wakes_or_unmount() {
    let mut ui = Ui::new(30, 10);
    let log = Log::default();
    let k = ui.insert(None, 0, Control(log.clone())).unwrap();
    ui.schedule(k, 0, Some(Wake::At(1)), Lifetime::Mounted)
        .unwrap();
    ui.schedule(k, 1, Some(Wake::Frame), Lifetime::Mounted)
        .unwrap();
    ui.advance(2);
    ui.frame(2);
    assert!(log.borrow().is_empty());
    ui.pump(10);
    ui.advance(3);
    ui.frame(3);
    assert_eq!(&*log.borrow(), &["mount", "wake", "frame"]);
    let untouched = Log::default();
    let k = ui.insert(None, 1, Control(untouched.clone())).unwrap();
    ui.remove(k).unwrap();
    ui.pump(10);
    assert!(untouched.borrow().is_empty());
}
#[test]
fn disappearing_nested_modal_uses_same_focus_fallback_as_explicit_close() {
    for mode in 0..3 {
        let mut ui = Ui::new(31 + mode, 20);
        let outer = ui.insert(None, 0, Control(Log::default())).unwrap();
        let saved = ui.insert(Some(outer), 0, Control(Log::default())).unwrap();
        let inner = ui.insert(Some(outer), 1, Control(Log::default())).unwrap();
        ui.pump(20);
        ui.place(
            inner,
            Placement {
                transform: Affine([1., 0., 0., 1., 200., 0.]),
                ..Default::default()
            },
        )
        .unwrap();
        ui.set_modal(Some(outer)).unwrap();
        ui.dispatch(click(5.));
        assert_eq!(ui.focused(), Some(saved));
        ui.set_modal(Some(inner)).unwrap();
        ui.remove(saved).unwrap();
        match mode {
            0 => ui.set_modal(None).unwrap(),
            1 => ui.hide(inner, true).unwrap(),
            _ => ui.remove(inner).unwrap(),
        }
        assert_eq!(ui.focused(), Some(outer), "mode {mode}");
    }
}
