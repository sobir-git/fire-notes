use fire_ui_contract_model::*;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
type Log = Rc<RefCell<Vec<String>>>;
fn down(x: f64, y: f64) -> Input {
    Input::Button {
        pointer: 7,
        button: 1,
        down: true,
        position: Point(x, y),
    }
}
fn key(physical: u32, down: bool) -> Input {
    Input::Key {
        physical,
        logical: "ArrowLeft".into(),
        down,
        repeat: false,
    }
}
fn placement(x: f64, y: f64, w: f64, h: f64) -> Placement {
    Placement {
        transform: Affine([1., 0., 0., 1., x, y]),
        bounds: Rect(0., 0., w, h),
        ..Placement::default()
    }
}
struct Empty;
impl Widget for Empty {}
struct Schedule {
    delay: u64,
    log: Log,
}
impl Widget for Schedule {
    fn event(&mut self, cx: &mut Cx<'_>, phase: Phase, e: &Input) {
        if phase == Phase::Preview {
            return;
        }
        match e {
            Input::Button { down: true, .. } => {
                cx.schedule(0, Some(Wake::At(self.delay)), Lifetime::Visible)
                    .unwrap();
            }
            Input::Wake(s) => self.log.borrow_mut().push(format!("{}:{s}", self.delay)),
            _ => {}
        }
    }
}
#[test]
fn nested_timers_and_slots_survive_each_other() {
    let log = Log::default();
    let mut ui = Ui::new(1, 64);
    let parent = ui
        .insert(
            None,
            0,
            Schedule {
                delay: 100,
                log: log.clone(),
            },
        )
        .unwrap();
    let child = ui
        .insert(
            Some(parent),
            0,
            Schedule {
                delay: 5,
                log: log.clone(),
            },
        )
        .unwrap();
    ui.pump(64);
    ui.dispatch(down(10., 10.));
    assert_eq!(
        ui.next_work(),
        Work {
            deadline: Some(5),
            ..Work::default()
        }
    );
    ui.schedule(child, 1, Some(Wake::At(20)), Lifetime::Mounted)
        .unwrap();
    ui.advance(5);
    assert_eq!(&*log.borrow(), &["5:0"]);
    ui.hide(child, true).unwrap();
    assert_eq!(
        ui.next_work(),
        Work {
            deadline: Some(20),
            ..Work::default()
        }
    );
    ui.advance(20);
    ui.advance(100);
    assert_eq!(&*log.borrow(), &["5:0", "5:1", "100:0"]);
    assert_eq!(ui.next_work(), Work::default());
}
struct Search;
struct List(Log);
struct Picker(Log);
#[derive(Debug)]
struct Query(String);
#[derive(Debug)]
struct SetQuery(String);
#[derive(Debug)]
struct Chosen(String);
#[derive(Debug)]
struct Selected(String);
impl Widget for Search {
    fn event(&mut self, cx: &mut Cx<'_>, phase: Phase, e: &Input) {
        if phase != Phase::Target {
            return;
        }
        match e {
            Input::Button { down: true, .. } => cx.focus().unwrap(),
            Input::Text { text, .. } => cx.emit(Query(text.clone())).unwrap(),
            _ => {}
        }
    }
}
impl Widget for List {
    fn action(&mut self, _: &mut Cx<'_>, a: Action) -> Option<Action> {
        if let Some(q) = a.payload.downcast_ref::<SetQuery>() {
            self.0.borrow_mut().push(q.0.clone());
            None
        } else {
            Some(a)
        }
    }
    fn event(&mut self, cx: &mut Cx<'_>, phase: Phase, e: &Input) {
        if phase == Phase::Target && matches!(e, Input::Button { down: true, .. }) {
            cx.emit(Chosen("record-42".into())).unwrap();
        }
    }
}
impl Widget for Picker {
    fn mount(&mut self, cx: &mut Cx<'_>) {
        cx.insert_at(0, Search, placement(0., 0., 100., 30.))
            .unwrap();
        cx.insert_at(1, List(self.0.clone()), placement(0., 40., 100., 50.))
            .unwrap();
    }
    fn action(&mut self, cx: &mut Cx<'_>, a: Action) -> Option<Action> {
        if let Some(q) = a.payload.downcast_ref::<Query>() {
            cx.send(cx.child(1).unwrap(), SetQuery(q.0.clone()))
                .unwrap();
            None
        } else if let Some(c) = a.payload.downcast_ref::<Chosen>() {
            cx.emit(Selected(c.0.clone())).unwrap();
            None
        } else {
            Some(a)
        }
    }
}
#[test]
fn picker_owns_children_and_consumes_internal_actions_without_app_glue() {
    let log = Log::default();
    let mut ui = Ui::new(1, 64);
    let picker = ui.insert(None, 0, Picker(log.clone())).unwrap();
    ui.pump(64);
    assert_eq!(ui.mounted(), 3);
    ui.dispatch(down(10., 10.));
    ui.dispatch(Input::Text {
        session: ui.session(),
        text: "ab".into(),
    });
    ui.pump(64);
    assert_eq!(&*log.borrow(), &["ab"]);
    assert!(ui.app_actions.is_empty());
    ui.dispatch(down(10., 50.));
    ui.pump(64);
    assert_eq!(ui.app_actions.len(), 1);
    assert_eq!(ui.app_actions[0].source, picker);
    assert_eq!(
        ui.app_actions[0]
            .payload
            .downcast_ref::<Selected>()
            .unwrap()
            .0,
        "record-42"
    );
    ui.remove(picker).unwrap();
    assert_eq!(ui.mounted(), 0);
}
struct PointerLog(Rc<RefCell<Vec<Point>>>);
impl Widget for PointerLog {
    fn event(&mut self, _: &mut Cx<'_>, phase: Phase, e: &Input) {
        if phase == Phase::Target {
            if let Some(p) = e.position() {
                self.0.borrow_mut().push(p)
            }
        }
    }
}
#[test]
fn transformed_drawing_and_input_use_the_same_coordinates_and_clip() {
    let mut ui = Ui::new(1, 64);
    let root = ui.insert(None, 0, Empty).unwrap();
    let log = Rc::default();
    let child = ui
        .insert(Some(root), 0, PointerLog(Rc::clone(&log)))
        .unwrap();
    ui.pump(64);
    ui.place(
        root,
        Placement {
            transform: Affine([2., 0., 0., 2., 10., 10.]),
            bounds: Rect(0., 0., 20., 20.),
            ..Placement::default()
        },
    )
    .unwrap();
    ui.place(child, placement(0., 0., 40., 40.)).unwrap();
    let p = Point(30., 30.);
    assert!(ui.paint_contains(child, p));
    assert_eq!(ui.hit(p), Some(child));
    ui.dispatch(down(p.0, p.1));
    assert_eq!(&*log.borrow(), &[Point(10., 10.)]);
    assert!(!ui.paint_contains(child, Point(70., 30.)));
    assert_eq!(ui.hit(Point(70., 30.)), None);
    ui.place(
        root,
        Placement {
            transform: Affine([0., 2., -2., 0., 50., 10.]),
            ..Placement::default()
        },
    )
    .unwrap();
    let p = ui.world(child).unwrap().point(Point(3., 7.));
    ui.dispatch(down(p.0, p.1));
    assert_eq!(log.borrow().last(), Some(&Point(3., 7.)));
}
struct Game(Log);
impl Widget for Game {
    fn event(&mut self, cx: &mut Cx<'_>, phase: Phase, e: &Input) {
        if phase != Phase::Target {
            return;
        }
        match e {
            Input::Button {
                down: true,
                pointer,
                ..
            } => {
                cx.focus().unwrap();
                cx.capture(*pointer, true).unwrap();
            }
            Input::Key { physical, down, .. } => {
                self.0.borrow_mut().push(format!("{physical}:{down}"))
            }
            Input::CancelKeys => self.0.borrow_mut().push("cancel".into()),
            _ => {}
        }
    }
}
#[test]
fn held_keys_releases_capture_and_focus_loss_are_explicit() {
    let mut ui = Ui::new(1, 64);
    let log = Log::default();
    let game = ui.insert(None, 0, Game(log.clone())).unwrap();
    ui.pump(64);
    ui.dispatch(down(10., 10.));
    assert_eq!(ui.captured(7), Some(game));
    assert_eq!(ui.captured(8), None);
    ui.dispatch(key(37, true));
    assert!(ui.pressed(37));
    ui.dispatch(key(37, false));
    assert!(!ui.pressed(37));
    ui.dispatch(key(37, true));
    ui.blur();
    assert!(!ui.pressed(37));
    assert_eq!(ui.captured(7), None);
    assert!(log.borrow().contains(&"37:false".into()));
    assert!(log.borrow().contains(&"cancel".into()));
}
#[test]
fn mouse_batch_keeps_latest_position_and_flushes_before_click() {
    let mut batch = InputBatch::default();
    for x in 0..1000 {
        assert!(batch
            .push(Input::Move {
                pointer: 7,
                position: Point(x as f64, 10.)
            })
            .is_empty())
    }
    let out = batch.push(down(999., 10.));
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].position(), Some(Point(999., 10.)));
    assert!(matches!(out[1], Input::Button { .. }));
    assert!(batch.flush().is_empty());
}
struct Results(Log);
impl Widget for Results {
    fn action(&mut self, _: &mut Cx<'_>, a: Action) -> Option<Action> {
        if let Some(s) = a.payload.downcast_ref::<String>() {
            self.0.borrow_mut().push(s.clone());
        }
        None
    }
}
#[test]
fn removal_reuse_and_foreign_ui_cannot_receive_stale_results() {
    let mut ui = Ui::new(1, 2);
    let log = Log::default();
    let a = ui.insert(None, 0, Results(log.clone())).unwrap();
    ui.pump(2);
    let ticket = ui.begin_task(a, 0).unwrap();
    ui.remove(a).unwrap();
    let b = ui.insert(None, 0, Results(log.clone())).unwrap();
    ui.pump(2);
    assert_eq!(a.slot, b.slot);
    assert_ne!(a, b);
    assert_eq!(
        ui.receive(ticket, "old".to_string()),
        Err((Error::Stale, "old".into()))
    );
    assert_eq!(ui.begin_task(Key { ui: 2, ..b }, 0), Err(Error::Stale));
    assert!(log.borrow().is_empty());
}
#[test]
fn mailbox_full_returns_payload_to_the_producer() {
    let mut ui = Ui::new(1, 2);
    let log = Log::default();
    let a = ui.insert(None, 0, Results(log.clone())).unwrap();
    ui.pump(2);
    let t = ui.begin_task(a, 0).unwrap();
    ui.receive(t, "one".to_string()).unwrap();
    ui.receive(t, "two".to_string()).unwrap();
    assert_eq!(
        ui.receive(t, "three".to_string()),
        Err((Error::Full, "three".into()))
    );
    assert_eq!(ui.queued(), 2);
    assert_eq!(ui.pump(1), 1);
    assert_eq!(
        ui.next_work(),
        Work {
            ready: true,
            ..Work::default()
        }
    );
    ui.pump(1);
    assert_eq!(ui.next_work(), Work::default());
}
#[test]
fn modal_overlay_keeps_ownership_and_escapes_the_anchor_clip() {
    let mut ui = Ui::new(1, 64);
    let owner = ui.insert(None, 0, Empty).unwrap();
    let anchor = ui.insert(Some(owner), 0, Empty).unwrap();
    let popup = ui.insert(Some(owner), 1, Empty).unwrap();
    ui.pump(64);
    ui.place(owner, placement(0., 0., 20., 20.)).unwrap();
    ui.place(
        popup,
        Placement {
            overlay_anchor: Some(anchor),
            ..placement(30., 30., 20., 20.)
        },
    )
    .unwrap();
    ui.set_modal(Some(owner)).unwrap();
    assert_eq!(ui.hit(Point(35., 35.)), Some(popup));
    assert_eq!(ui.owner(popup), Some(owner));
    ui.remove(owner).unwrap();
    assert!(!ui.live(popup));
    assert_eq!(ui.hit(Point(35., 35.)), None);
}
#[test]
fn virtual_list_mounts_bounded_rows_and_preserves_record_identity() {
    let mut ui = Ui::new(1, 128);
    let list = ui.insert(None, 0, Empty).unwrap();
    ui.pump(128);
    let mut rows = BTreeMap::new();
    for offset in [0., 300., 90000., 300.] {
        let range = visible_rows(100000, offset, 600., 30., 2);
        let remove: Vec<_> = rows
            .keys()
            .copied()
            .filter(|i| !range.contains(i))
            .collect();
        for i in remove {
            ui.remove(rows.remove(&i).unwrap()).unwrap();
        }
        for i in range {
            rows.entry(i)
                .or_insert_with(|| ui.insert(Some(list), (i % 65535) as u16, Empty).unwrap());
        }
        ui.pump(128);
        assert!(ui.mounted() <= 26);
    }
}
#[test]
fn stale_ime_session_does_not_insert_into_new_focus() {
    let log = Log::default();
    let mut ui = Ui::new(1, 64);
    let p = ui.insert(None, 0, Picker(log.clone())).unwrap();
    ui.pump(64);
    ui.dispatch(down(10., 10.));
    let old = ui.session();
    ui.blur();
    ui.dispatch(down(10., 10.));
    assert!(ui.session() > old);
    ui.dispatch(Input::Text {
        session: old,
        text: "stale".into(),
    });
    ui.pump(64);
    assert!(log.borrow().is_empty());
    assert!(ui.live(p));
}
