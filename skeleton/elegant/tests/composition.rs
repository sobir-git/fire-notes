use fire_ui_elegant_model::*;
use std::{cell::RefCell, rc::Rc, time::Duration};

#[derive(Debug)]
struct Activate;
struct Button;
impl Widget for Button {
    type Command = Activate;
    type Output = ();
    fn update(&mut self, cx: &mut Update<'_, Self>, _: Activate) {
        cx.emit(()).unwrap();
    }
}
struct Label(Rc<RefCell<String>>);
impl Widget for Label {
    type Command = String;
    type Output = ();
    fn update(&mut self, _: &mut Update<'_, Self>, text: String) {
        *self.0.borrow_mut() = text;
    }
}
#[derive(Debug)]
enum CounterCommand {
    Activate,
    Increment,
}
struct Counter {
    button: Child<Button>,
    label: Child<Label>,
    count: usize,
}
fn counter(label: Rc<RefCell<String>>) -> Element<Counter> {
    Element::build(|children| Counter {
        button: children.connect(Element::leaf(Button), |()| CounterCommand::Increment),
        label: children.add(Element::leaf(Label(label))),
        count: 0,
    })
}
impl Widget for Counter {
    type Command = CounterCommand;
    type Output = usize;
    fn update(&mut self, cx: &mut Update<'_, Self>, command: CounterCommand) {
        match command {
            CounterCommand::Activate => cx.send(self.button, Activate).unwrap(),
            CounterCommand::Increment => {
                self.count += 1;
                cx.send(self.label, self.count.to_string()).unwrap();
                cx.emit(self.count).unwrap();
            }
        }
    }
}
#[derive(Debug)]
enum BoardCommand {
    Left,
    Right,
    Count(&'static str, usize),
    RetireLeft,
    StaleLeft,
}
struct Board {
    left: Child<Counter>,
    right: Child<Counter>,
}
fn board(left: Rc<RefCell<String>>, right: Rc<RefCell<String>>) -> Element<Board> {
    Element::build(|children| Board {
        left: children.connect(counter(left), |n| BoardCommand::Count("left", n)),
        right: children.connect(counter(right), |n| BoardCommand::Count("right", n)),
    })
}
impl Widget for Board {
    type Command = BoardCommand;
    type Output = (&'static str, usize);
    fn update(&mut self, cx: &mut Update<'_, Self>, command: BoardCommand) {
        match command {
            BoardCommand::Left => cx.send(self.left, CounterCommand::Activate).unwrap(),
            BoardCommand::Right => cx.send(self.right, CounterCommand::Activate).unwrap(),
            BoardCommand::Count(side, n) => cx.emit((side, n)).unwrap(),
            BoardCommand::RetireLeft => {
                cx.send(self.left, CounterCommand::Activate).unwrap();
                cx.remove(self.left).unwrap();
            }
            BoardCommand::StaleLeft => assert!(matches!(
                cx.send(self.left, CounterCommand::Activate),
                Err((SendError::Stale, _))
            )),
        }
    }
}
#[test]
fn two_reusable_counters_keep_private_wiring_and_independent_state() {
    let left = Rc::default();
    let right = Rc::default();
    let mut ui = Ui::new(board(Rc::clone(&left), Rc::clone(&right)), 32);
    for command in [BoardCommand::Left, BoardCommand::Right, BoardCommand::Left] {
        ui.send(command).unwrap();
    }
    let mut outputs = vec![];
    ui.pump(100, |out| outputs.push(out));
    assert_eq!(&*left.borrow(), "2");
    assert_eq!(&*right.borrow(), "1");
    assert_eq!(outputs, [("left", 1), ("right", 1), ("left", 2)]);
    assert_eq!(ui.len(), 7);
    assert!(ui.next_work().is_idle());
}
#[test]
fn queued_work_is_cancelled_when_its_owner_subtree_is_removed() {
    let mut ui = Ui::new(board(Rc::default(), Rc::default()), 16);
    ui.send(BoardCommand::RetireLeft).unwrap();
    let result = ui.pump(100, |_| panic!("removed control emitted"));
    assert_eq!(result.stale, 1);
    assert_eq!(ui.len(), 4);
    ui.send(BoardCommand::StaleLeft).unwrap();
    ui.pump(100, |_| {});
    ui.send(BoardCommand::Right).unwrap();
    let mut outputs = vec![];
    ui.pump(100, |out| outputs.push(out));
    assert_eq!(outputs, [("right", 1)]);
}
#[derive(Debug)]
enum SearchCommand {
    Set(String),
    Edit(String), // A semantic stand-in for input delivery, not a native key event.
}
struct Search {
    text: String,
}
impl Widget for Search {
    type Command = SearchCommand;
    type Output = String;
    fn update(&mut self, cx: &mut Update<'_, Self>, command: SearchCommand) {
        match command {
            SearchCommand::Set(text) => self.text = text,
            SearchCommand::Edit(text) => {
                self.text = text.clone();
                cx.emit(text).unwrap();
            }
        }
    }
}
#[derive(Debug)]
enum ResultsCommand {
    Filter(String),
    Choose,
}
struct Results {
    query: String,
}
impl Widget for Results {
    type Command = ResultsCommand;
    type Output = String;
    fn update(&mut self, cx: &mut Update<'_, Self>, command: ResultsCommand) {
        match command {
            ResultsCommand::Filter(query) => self.query = query,
            ResultsCommand::Choose => cx.emit(format!("note:{}", self.query)).unwrap(),
        }
    }
}
#[derive(Debug)]
enum PickerCommand {
    Type(String),
    EditSearch(String),
    Query(String),
    Choose,
    Selected(String),
}
struct Picker {
    search: Child<Search>,
    results: Child<Results>,
}
fn picker() -> Element<Picker> {
    Element::build(|children| Picker {
        search: children.connect(
            Element::leaf(Search {
                text: String::new(),
            }),
            PickerCommand::Query,
        ),
        results: children.connect(
            Element::leaf(Results {
                query: String::new(),
            }),
            PickerCommand::Selected,
        ),
    })
}
impl Widget for Picker {
    type Command = PickerCommand;
    type Output = String;
    fn update(&mut self, cx: &mut Update<'_, Self>, command: PickerCommand) {
        match command {
            // Owner commands update both views in order. A programmatic Set is silent:
            // echoing it through Search would let a later Choose overtake the filter.
            PickerCommand::Type(text) => {
                cx.send(self.search, SearchCommand::Set(text.clone()))
                    .unwrap();
                cx.send(self.results, ResultsCommand::Filter(text)).unwrap();
            }
            PickerCommand::EditSearch(text) => {
                cx.send(self.search, SearchCommand::Edit(text)).unwrap()
            }
            PickerCommand::Query(text) => {
                cx.send(self.results, ResultsCommand::Filter(text)).unwrap()
            }
            PickerCommand::Choose => cx.send(self.results, ResultsCommand::Choose).unwrap(),
            PickerCommand::Selected(note) => cx.emit(note).unwrap(),
        }
    }
}
#[test]
fn picker_maps_typed_outputs_inside_its_owner() {
    let mut ui = Ui::new(picker(), 8);
    ui.send(PickerCommand::EditSearch("fire".into())).unwrap();
    ui.pump(32, |_| panic!("query escaped the picker"));
    ui.send(PickerCommand::Choose).unwrap();
    let mut selected = vec![];
    ui.pump(32, |note| selected.push(note));
    assert_eq!(selected, ["note:fire"]);
    assert!(ui.next_work().is_idle());
}
#[test]
fn queued_picker_updates_and_selections_preserve_owner_command_order() {
    let mut ui = Ui::new(picker(), 16);
    for command in [
        PickerCommand::Type("fire".into()),
        PickerCommand::Choose,
        PickerCommand::Type("ice".into()),
        PickerCommand::Choose,
    ] {
        ui.send(command).unwrap();
    }
    let mut selected = vec![];
    // Deliberately yield after every envelope. No drain-to-quiescence assumption.
    while ui.next_work().ready {
        ui.pump(1, |note| selected.push(note));
    }
    assert_eq!(selected, ["note:fire", "note:ice"]);
    assert!(ui.next_work().is_idle());
}
struct Foreign {
    other: Child<Counter>,
}
impl Widget for Foreign {
    type Command = ();
    type Output = ();
    fn update(&mut self, cx: &mut Update<'_, Self>, _: ()) {
        assert!(matches!(
            cx.send(self.other, CounterCommand::Activate),
            Err((SendError::Stale, _))
        ));
    }
}
#[test]
fn handles_cannot_address_another_ui() {
    let first = Ui::new(board(Rc::default(), Rc::default()), 8);
    let mut second = Ui::new(
        Element::leaf(Foreign {
            other: first.root().left,
        }),
        8,
    );
    second.send(()).unwrap();
    second.pump(8, |_| {});
}

#[derive(Debug)]
enum ClockCommand {
    Start,
    Pause,
}
struct Clock {
    caret: Timer,
    debounce: Timer,
    frames: usize,
    fired: Vec<Timer>,
}
impl Widget for Clock {
    type Command = ClockCommand;
    type Output = ();
    fn update(&mut self, cx: &mut Update<'_, Self>, command: ClockCommand) {
        match command {
            ClockCommand::Start => {
                cx.request_frame();
                cx.request_frame();
                cx.after(self.caret, Duration::from_millis(5));
                cx.after(self.debounce, Duration::from_millis(9));
            }
            ClockCommand::Pause => {
                cx.cancel_frame();
                cx.cancel_timer(self.caret);
                cx.cancel_timer(self.debounce);
            }
        }
    }
    fn frame(&mut self, _: &mut Update<'_, Self>, _: FrameTime) {
        self.frames += 1;
    }
    fn timer(&mut self, _: &mut Update<'_, Self>, timer: Timer) {
        self.fired.push(timer);
    }
}
#[test]
fn helper_frame_requests_coalesce_and_timers_remain_independent() {
    let caret = Timer::new();
    let debounce = Timer::new();
    let mut ui = Ui::new(
        Element::leaf(Clock {
            caret,
            debounce,
            frames: 0,
            fired: vec![],
        }),
        8,
    );
    ui.send(ClockCommand::Start).unwrap();
    ui.pump(8, |_| {});
    assert!(ui.next_work().frame);
    assert_eq!(ui.next_work().deadline, Some(Duration::from_millis(5)));
    ui.advance(Duration::from_millis(5));
    assert_eq!(ui.root().fired, [caret]);
    ui.frame(Duration::from_millis(8));
    assert_eq!(ui.root().frames, 1);
    ui.advance(Duration::from_millis(9));
    assert_eq!(ui.root().fired, [caret, debounce]);
    assert!(ui.next_work().is_idle());
    ui.send(ClockCommand::Start).unwrap();
    ui.pump(8, |_| {});
    ui.send(ClockCommand::Pause).unwrap();
    ui.pump(8, |_| {});
    assert!(ui.next_work().is_idle());
}
struct Echo;
impl Widget for Echo {
    type Command = String;
    type Output = String;
    fn update(&mut self, cx: &mut Update<'_, Self>, value: String) {
        cx.emit(value).unwrap();
    }
}
#[test]
fn capacity_one_preserves_payload_and_yields_between_deliveries() {
    let mut ui = Ui::new(Element::leaf(Echo), 1);
    ui.send("first".into()).unwrap();
    assert_eq!(
        ui.send("retry".into()),
        Err((SendError::Full, "retry".into()))
    );
    assert_eq!(ui.pump(1, |_| panic!("budget exceeded")).delivered, 1);
    assert_eq!(ui.queued(), 1);
    let mut got = vec![];
    ui.pump(1, |out| got.push(out));
    assert_eq!(got, ["first"]);
    assert!(ui.next_work().is_idle());
}

struct Branch;
impl Widget for Branch {
    type Command = ();
    type Output = ();
    fn update(&mut self, _: &mut Update<'_, Self>, _: ()) {}
}
struct Intruder {
    grandchild: Child<Label>,
}
impl Widget for Intruder {
    type Command = ();
    type Output = ();
    fn update(&mut self, cx: &mut Update<'_, Self>, _: ()) {
        assert_eq!(
            cx.send(self.grandchild, "forbidden".into()),
            Err((SendError::NotOwned, "forbidden".into()))
        );
    }
}
#[test]
fn correct_type_does_not_grant_authority_over_grandchildren() {
    let mut target = None;
    let branch = Element::build(|children| {
        target = Some(children.add(Element::leaf(Label(Rc::default()))));
        Branch
    });
    let root = Element::build(|children| {
        children.add(branch);
        Intruder {
            grandchild: target.unwrap(),
        }
    });
    let mut ui = Ui::new(root, 2);
    ui.send(()).unwrap();
    ui.pump(2, |_| {});
}

struct Rearm {
    first: Timer,
    second: Timer,
    fired: Vec<Timer>,
}
impl Widget for Rearm {
    type Command = ();
    type Output = ();
    fn update(&mut self, cx: &mut Update<'_, Self>, _: ()) {
        cx.after(self.first, Duration::ZERO);
        cx.after(self.second, Duration::ZERO);
    }
    fn timer(&mut self, cx: &mut Update<'_, Self>, timer: Timer) {
        self.fired.push(timer);
        if timer == self.first {
            cx.cancel_timer(self.second);
            cx.after(self.second, Duration::ZERO);
        }
    }
}
#[test]
fn replacing_an_already_selected_timer_does_not_deliver_its_old_epoch() {
    let first = Timer::new();
    let second = Timer::new();
    let mut ui = Ui::new(
        Element::leaf(Rearm {
            first,
            second,
            fired: vec![],
        }),
        2,
    );
    ui.send(()).unwrap();
    ui.pump(2, |_| {});
    ui.advance(Duration::ZERO);
    assert_eq!(ui.root().fired, [first]);
    ui.advance(Duration::ZERO);
    assert_eq!(ui.root().fired, [first, second]);
    assert!(ui.next_work().is_idle());
}

struct Animator {
    frames: Rc<RefCell<usize>>,
    visible_timer: Timer,
    mounted_timer: Timer,
    fired: Rc<RefCell<Vec<Timer>>>,
}
impl Widget for Animator {
    type Command = ();
    type Output = ();
    fn update(&mut self, cx: &mut Update<'_, Self>, _: ()) {
        cx.request_frame();
        cx.request_frame();
        cx.after(self.visible_timer, Duration::from_millis(1));
        cx.after_with_lifetime(
            self.mounted_timer,
            Duration::from_millis(2),
            Lifetime::Mounted,
        );
    }
    fn frame(&mut self, cx: &mut Update<'_, Self>, _: FrameTime) {
        *self.frames.borrow_mut() += 1;
        cx.request_frame();
        cx.request_frame();
    }
    fn timer(&mut self, _: &mut Update<'_, Self>, timer: Timer) {
        self.fired.borrow_mut().push(timer);
    }
}
#[derive(Debug)]
enum AnimationCommand {
    Start,
    Hide,
}
struct AnimationOwner {
    child: Child<Animator>,
    frames: usize,
}
impl Widget for AnimationOwner {
    type Command = AnimationCommand;
    type Output = ();
    fn update(&mut self, cx: &mut Update<'_, Self>, command: AnimationCommand) {
        match command {
            AnimationCommand::Start => {
                cx.request_frame();
                cx.send(self.child, ()).unwrap();
            }
            AnimationCommand::Hide => cx.show(self.child, false).unwrap(),
        }
    }
    fn frame(&mut self, _: &mut Update<'_, Self>, _: FrameTime) {
        self.frames += 1;
    }
}
#[test]
fn child_frames_are_independent_and_hide_cancels_only_visual_work() {
    let frames = Rc::default();
    let fired = Rc::default();
    let mounted = Timer::new();
    let root = Element::build(|children| AnimationOwner {
        child: children.add(Element::leaf(Animator {
            frames: Rc::clone(&frames),
            visible_timer: Timer::new(),
            mounted_timer: mounted,
            fired: Rc::clone(&fired),
        })),
        frames: 0,
    });
    let mut ui = Ui::new(root, 4);
    ui.send(AnimationCommand::Start).unwrap();
    ui.pump(4, |_| {});
    ui.frame(Duration::ZERO);
    assert_eq!(ui.root().frames, 1);
    assert_eq!(*frames.borrow(), 1);
    ui.frame(Duration::ZERO);
    assert_eq!(ui.root().frames, 1);
    assert_eq!(*frames.borrow(), 2);
    ui.send(AnimationCommand::Hide).unwrap();
    ui.pump(4, |_| {});
    assert!(!ui.next_work().frame);
    assert_eq!(ui.next_work().deadline, Some(Duration::from_millis(2)));
    ui.advance(Duration::from_millis(2));
    assert_eq!(*fired.borrow(), [mounted]);
    assert!(ui.next_work().is_idle());
}

struct RetryOutput {
    pending: Option<String>,
    retry: Timer,
}
impl RetryOutput {
    fn flush(&mut self, cx: &mut Update<'_, Self>) {
        if let Some(value) = self.pending.take() {
            if let Err((SendError::Full, value)) = cx.emit(value) {
                self.pending = Some(value);
                cx.after_with_lifetime(self.retry, Duration::ZERO, Lifetime::Mounted);
            }
        }
    }
}
impl Widget for RetryOutput {
    type Command = ();
    type Output = String;
    fn update(&mut self, cx: &mut Update<'_, Self>, _: ()) {
        cx.emit("first".into()).unwrap();
        self.pending = Some("second".into());
        self.flush(cx);
    }
    fn timer(&mut self, cx: &mut Update<'_, Self>, _: Timer) {
        self.flush(cx);
    }
}
#[test]
fn callback_backpressure_can_retain_one_payload_and_retry_without_reentry() {
    let mut ui = Ui::new(
        Element::leaf(RetryOutput {
            pending: None,
            retry: Timer::new(),
        }),
        1,
    );
    ui.send(()).unwrap();
    ui.pump(1, |_| {});
    assert_eq!(ui.root().pending.as_deref(), Some("second"));
    let mut got = vec![];
    // A timer firing before the host drains output remains bounded and retries next turn.
    ui.advance(Duration::ZERO);
    assert_eq!(ui.queued(), 1);
    ui.pump(1, |out| got.push(out));
    ui.advance(Duration::ZERO);
    ui.pump(1, |out| got.push(out));
    assert_eq!(got, ["first", "second"]);
    assert!(ui.root().pending.is_none());
    assert!(ui.next_work().is_idle());
}
