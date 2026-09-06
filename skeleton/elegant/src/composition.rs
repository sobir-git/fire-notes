use std::{
    any::Any,
    collections::{BTreeMap, BTreeSet, VecDeque},
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Id(u64);
fn identity() -> Id {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    Id(NEXT
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .expect("identity space exhausted"))
}

/// A typed reference created by an owner's child builder. Identity is opaque and never reused.
/// The widget type is invariant: function-pointer subtyping must not change its command type.
///
/// ```compile_fail
/// use fire_ui_elegant_model::*;
/// use std::marker::PhantomData;
/// struct Sink<T>(PhantomData<T>);
/// impl<T: 'static> Widget for Sink<T> {
///     type Command = T;
///     type Output = ();
///     fn update(&mut self, _: &mut Update<'_, Self>, _: T) {}
/// }
/// fn change(child: Child<Sink<fn(&str)>>) -> Child<Sink<fn(&'static str)>> {
///     child
/// }
/// ```
pub struct Child<W: Widget> {
    id: Id,
    marker: PhantomData<fn(W) -> W>,
}
impl<W: Widget> Copy for Child<W> {}
impl<W: Widget> Clone for Child<W> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<W: Widget> std::fmt::Debug for Child<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Child")
    }
}
impl<W: Widget> Child<W> {
    fn new(id: Id) -> Self {
        Self {
            id,
            marker: PhantomData,
        }
    }
}

/// Typed widget protocol subset. Full input, lifecycle, geometry and painting retain separate contracts.
pub trait Widget: Any + Sized {
    type Command: 'static;
    type Output: 'static;
    fn update(&mut self, cx: &mut Update<'_, Self>, command: Self::Command);
    fn frame(&mut self, _cx: &mut Update<'_, Self>, _time: FrameTime) {}
    fn timer(&mut self, _cx: &mut Update<'_, Self>, _timer: Timer) {}
}

type Payload = Box<dyn Any>;
type MapOutput = Box<dyn Fn(Payload) -> Payload>;
trait Erased {
    fn state(&self) -> &dyn Any;
    fn update(&mut self, cx: &mut RawUpdate<'_>, command: Payload);
    fn frame(&mut self, cx: &mut RawUpdate<'_>, time: FrameTime);
    fn timer(&mut self, cx: &mut RawUpdate<'_>, timer: Timer);
}
impl<W: Widget> Erased for W {
    fn state(&self) -> &dyn Any {
        self
    }
    fn update(&mut self, raw: &mut RawUpdate<'_>, payload: Payload) {
        let command = *payload
            .downcast::<W::Command>()
            .expect("private typed-command invariant");
        self.update(&mut raw.typed(), command);
    }
    fn frame(&mut self, raw: &mut RawUpdate<'_>, time: FrameTime) {
        self.frame(&mut raw.typed(), time);
    }
    fn timer(&mut self, raw: &mut RawUpdate<'_>, timer: Timer) {
        self.timer(&mut raw.typed(), timer);
    }
}
struct Prepared {
    id: Id,
    widget: Box<dyn Erased>,
    children: Vec<Prepared>,
    output: Option<MapOutput>,
}
/// A single-use detached tree. Construction itself never runs widget callbacks.
///
/// ```compile_fail
/// use fire_ui_elegant_model::*;
/// use std::marker::PhantomData;
/// struct Sink<T>(PhantomData<T>);
/// impl<T: 'static> Widget for Sink<T> {
///     type Command = T;
///     type Output = ();
///     fn update(&mut self, _: &mut Update<'_, Self>, _: T) {}
/// }
/// fn change(tree: Element<Sink<fn(&str)>>) -> Element<Sink<fn(&'static str)>> {
///     tree
/// }
/// ```
pub struct Element<W: Widget> {
    prepared: Prepared,
    marker: PhantomData<fn(W) -> W>,
}
impl<W: Widget> Element<W> {
    pub fn leaf(widget: W) -> Self {
        Self::build(|_| widget)
    }
    pub fn build(build: impl FnOnce(&mut Children<W>) -> W) -> Self {
        let mut children = Children {
            nodes: vec![],
            marker: PhantomData,
        };
        let widget = build(&mut children);
        Self {
            prepared: Prepared {
                id: identity(),
                widget: Box::new(widget),
                children: children.nodes,
                output: None,
            },
            marker: PhantomData,
        }
    }
}
pub struct Children<P: Widget> {
    nodes: Vec<Prepared>,
    marker: PhantomData<fn(P) -> P>,
}
impl<P: Widget> Children<P> {
    /// Explicitly discard a unit-output child's notifications. Use `connect` to handle them.
    pub fn add<W: Widget<Output = ()>>(&mut self, element: Element<W>) -> Child<W> {
        self.push(element.prepared)
    }
    pub fn connect<W: Widget>(
        &mut self,
        element: Element<W>,
        map: impl Fn(W::Output) -> P::Command + 'static,
    ) -> Child<W> {
        let mut node = element.prepared;
        node.output = Some(Box::new(move |output| {
            Box::new(map(*output
                .downcast::<W::Output>()
                .expect("private output invariant")))
        }));
        self.push(node)
    }
    /// An explicit choice to ignore a child's outputs, without a dynamic payload API.
    pub fn discard_output<W: Widget>(&mut self, element: Element<W>) -> Child<W> {
        self.push(element.prepared)
    }
    fn push<W: Widget>(&mut self, node: Prepared) -> Child<W> {
        let child = Child::new(node.id);
        self.nodes.push(node);
        child
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendError {
    Full,
    Stale,
    NotOwned,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lifetime {
    Visible,
    Mounted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timer(Id);
impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}
impl Timer {
    pub fn new() -> Self {
        Self(identity())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameTime {
    pub now: Duration,
    pub elapsed: Duration,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Work {
    pub ready: bool,
    pub frame: bool,
    pub deadline: Option<Duration>,
}
impl Work {
    pub fn is_idle(self) -> bool {
        !self.ready && !self.frame && self.deadline.is_none()
    }
}
#[derive(Default, Debug)]
pub struct Progress {
    pub delivered: usize,
    pub stale: usize,
}

struct Node {
    parent: Option<Id>,
    children: Vec<Id>,
    widget: Option<Box<dyn Erased>>,
    output: Option<MapOutput>,
    visible: bool,
}
enum Delivery {
    Command(Id, Payload),
    Output(Id, Payload),
}
/// The sole owner of queue capacity. A callback's batch is admitted before it is published.
struct Mailbox {
    items: VecDeque<Delivery>,
    capacity: usize,
}
impl Mailbox {
    fn free(&self) -> usize {
        self.capacity - self.items.len()
    }
    fn append(&mut self, items: Vec<Delivery>) {
        assert!(items.len() <= self.free());
        self.items.extend(items);
    }
}
struct Changes {
    free: usize,
    outgoing: Vec<Delivery>,
    remove: BTreeSet<Id>,
    visibility: BTreeMap<Id, bool>,
    frame: Option<bool>,
    timers: BTreeMap<Timer, Option<(Duration, Lifetime)>>,
}
struct RawUpdate<'a> {
    me: Id,
    nodes: &'a BTreeMap<Id, Node>,
    changes: Changes,
    now: Duration,
}
impl RawUpdate<'_> {
    fn typed<W: Widget>(&mut self) -> Update<'_, W> {
        Update {
            me: self.me,
            nodes: self.nodes,
            changes: &mut self.changes,
            now: self.now,
            marker: PhantomData,
        }
    }
}
pub struct Update<'a, W: Widget> {
    me: Id,
    nodes: &'a BTreeMap<Id, Node>,
    changes: &'a mut Changes,
    now: Duration,
    marker: PhantomData<fn(W) -> W>,
}
impl<W: Widget> Update<'_, W> {
    fn owned<C: Widget>(&self, child: Child<C>) -> Result<(), SendError> {
        match self.nodes.get(&child.id) {
            None => Err(SendError::Stale),
            Some(node) if node.parent != Some(self.me) => Err(SendError::NotOwned),
            Some(_) => Ok(()),
        }
    }
    /// Admission failure returns the command. Later removal cancels admitted delivery.
    ///
    /// ```compile_fail
    /// use fire_ui_elegant_model::*;
    /// struct Owner;
    /// impl Widget for Owner {
    ///     type Command = (); type Output = ();
    ///     fn update(&mut self, _: &mut Update<'_, Self>, _: ()) {}
    /// }
    /// struct Label;
    /// impl Widget for Label {
    ///     type Command = String; type Output = ();
    ///     fn update(&mut self, _: &mut Update<'_, Self>, _: String) {}
    /// }
    /// fn wrong(cx: &mut Update<'_, Owner>, label: Child<Label>) {
    ///     cx.send(label, 42_u32).unwrap();
    /// }
    /// ```
    pub fn send<C: Widget>(
        &mut self,
        child: Child<C>,
        command: C::Command,
    ) -> Result<(), (SendError, C::Command)> {
        if let Err(e) = self.owned(child) {
            return Err((e, command));
        }
        if self.changes.outgoing.len() == self.changes.free {
            return Err((SendError::Full, command));
        }
        self.changes
            .outgoing
            .push(Delivery::Command(child.id, Box::new(command)));
        Ok(())
    }
    pub fn emit(&mut self, output: W::Output) -> Result<(), (SendError, W::Output)> {
        if self.changes.outgoing.len() == self.changes.free {
            return Err((SendError::Full, output));
        }
        self.changes
            .outgoing
            .push(Delivery::Output(self.me, Box::new(output)));
        Ok(())
    }
    pub fn remove<C: Widget>(&mut self, child: Child<C>) -> Result<(), SendError> {
        self.owned(child)?;
        self.changes.remove.insert(child.id);
        Ok(())
    }
    pub fn show<C: Widget>(&mut self, child: Child<C>, visible: bool) -> Result<(), SendError> {
        self.owned(child)?;
        self.changes.visibility.insert(child.id, visible);
        Ok(())
    }
    pub fn request_frame(&mut self) {
        self.changes.frame = Some(true);
    }
    pub fn cancel_frame(&mut self) {
        self.changes.frame = Some(false);
    }
    pub fn after(&mut self, timer: Timer, delay: Duration) {
        self.after_with_lifetime(timer, delay, Lifetime::Visible);
    }
    pub fn after_with_lifetime(&mut self, timer: Timer, delay: Duration, lifetime: Lifetime) {
        self.changes
            .timers
            .insert(timer, Some((self.now.saturating_add(delay), lifetime)));
    }
    pub fn cancel_timer(&mut self, timer: Timer) {
        self.changes.timers.insert(timer, None);
    }
}

/// A typed root with no OS or renderer dependency. This experiment omits the C7 input/lifecycle router.
///
/// ```compile_fail
/// use fire_ui_elegant_model::*;
/// use std::marker::PhantomData;
/// struct Sink<T>(PhantomData<T>);
/// impl<T: 'static> Widget for Sink<T> {
///     type Command = T;
///     type Output = ();
///     fn update(&mut self, _: &mut Update<'_, Self>, _: T) {}
/// }
/// fn change(ui: Ui<Sink<fn(&str)>>) -> Ui<Sink<fn(&'static str)>> {
///     ui
/// }
/// ```
pub struct Ui<W: Widget> {
    root: Id,
    nodes: BTreeMap<Id, Node>,
    mailbox: Mailbox,
    frames: BTreeSet<Id>,
    timers: BTreeMap<(Id, Timer), (Duration, Lifetime, u64)>,
    epoch: u64,
    now: Duration,
    last_frame: Duration,
    marker: PhantomData<fn(W) -> W>,
}
impl<W: Widget> Ui<W> {
    pub fn new(element: Element<W>, capacity: usize) -> Self {
        assert!(capacity > 0);
        let root = element.prepared.id;
        let mut ui = Self {
            root,
            nodes: BTreeMap::new(),
            mailbox: Mailbox {
                items: VecDeque::new(),
                capacity,
            },
            frames: BTreeSet::new(),
            timers: BTreeMap::new(),
            epoch: 0,
            now: Duration::ZERO,
            last_frame: Duration::ZERO,
            marker: PhantomData,
        };
        ui.install(None, element.prepared);
        ui
    }
    fn install(&mut self, parent: Option<Id>, tree: Prepared) {
        let children = tree.children.iter().map(|n| n.id).collect();
        self.nodes.insert(
            tree.id,
            Node {
                parent,
                children,
                widget: Some(tree.widget),
                output: tree.output,
                visible: true,
            },
        );
        for child in tree.children {
            self.install(Some(tree.id), child);
        }
    }
    pub fn send(&mut self, command: W::Command) -> Result<(), (SendError, W::Command)> {
        if self.mailbox.free() == 0 {
            return Err((SendError::Full, command));
        }
        self.mailbox
            .append(vec![Delivery::Command(self.root, Box::new(command))]);
        Ok(())
    }
    pub fn root(&self) -> &W {
        self.nodes[&self.root]
            .widget
            .as_ref()
            .unwrap()
            .state()
            .downcast_ref()
            .unwrap()
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    pub fn queued(&self) -> usize {
        self.mailbox.items.len()
    }
    fn visible(&self, id: Id) -> bool {
        self.nodes
            .get(&id)
            .is_some_and(|n| n.visible && n.parent.is_none_or(|p| self.visible(p)))
    }
    fn invoke(&mut self, id: Id, call: impl FnOnce(&mut dyn Erased, &mut RawUpdate<'_>)) {
        let Some(mut widget) = self.nodes.get_mut(&id).and_then(|n| n.widget.take()) else {
            return;
        };
        let mut cx = RawUpdate {
            me: id,
            nodes: &self.nodes,
            changes: Changes {
                free: self.mailbox.free(),
                outgoing: vec![],
                remove: BTreeSet::new(),
                visibility: BTreeMap::new(),
                frame: None,
                timers: BTreeMap::new(),
            },
            now: self.now,
        };
        call(widget.as_mut(), &mut cx);
        let Changes {
            outgoing,
            remove,
            visibility,
            frame,
            timers,
            ..
        } = cx.changes;
        self.nodes.get_mut(&id).unwrap().widget = Some(widget);
        self.mailbox.append(outgoing);
        for child in remove {
            self.retire(child);
        }
        for (child, visible) in visibility {
            if let Some(n) = self.nodes.get_mut(&child) {
                n.visible = visible;
            }
        }
        if let Some(wanted) = frame {
            if wanted && self.visible(id) {
                self.frames.insert(id);
            } else {
                self.frames.remove(&id);
            }
        }
        for (timer, request) in timers {
            if let Some((at, life)) =
                request.filter(|(_, life)| *life == Lifetime::Mounted || self.visible(id))
            {
                self.epoch += 1;
                self.timers.insert((id, timer), (at, life, self.epoch));
            } else {
                self.timers.remove(&(id, timer));
            }
        }
        self.frames = self
            .frames
            .iter()
            .copied()
            .filter(|id| self.visible(*id))
            .collect();
        let hidden: Vec<_> = self
            .timers
            .iter()
            .filter(|((id, _), (_, life, _))| *life == Lifetime::Visible && !self.visible(*id))
            .map(|(k, _)| *k)
            .collect();
        for k in hidden {
            self.timers.remove(&k);
        }
    }
    fn retire(&mut self, id: Id) {
        let Some(node) = self.nodes.remove(&id) else {
            return;
        };
        for child in node.children {
            self.retire(child);
        }
        self.frames.remove(&id);
        self.timers.retain(|(owner, _), _| *owner != id);
        if let Some(parent) = node.parent.and_then(|p| self.nodes.get_mut(&p)) {
            parent.children.retain(|child| *child != id);
        }
    }
    pub fn pump(&mut self, budget: usize, mut output: impl FnMut(W::Output)) -> Progress {
        let mut progress = Progress::default();
        for _ in 0..budget {
            let Some(delivery) = self.mailbox.items.pop_front() else {
                break;
            };
            progress.delivered += 1;
            match delivery {
                Delivery::Command(id, command) => {
                    if self.nodes.contains_key(&id) {
                        self.invoke(id, |w, cx| w.update(cx, command));
                    } else {
                        progress.stale += 1;
                    }
                }
                Delivery::Output(id, payload) => {
                    let Some(node) = self.nodes.get(&id) else {
                        progress.stale += 1;
                        continue;
                    };
                    if let Some(parent) = node.parent {
                        if let Some(map) = &node.output {
                            self.mailbox
                                .append(vec![Delivery::Command(parent, map(payload))]);
                        }
                    } else {
                        output(
                            *payload
                                .downcast::<W::Output>()
                                .expect("private root-output invariant"),
                        );
                    }
                }
            }
        }
        progress
    }
    pub fn advance(&mut self, now: Duration) {
        assert!(now >= self.now);
        self.now = now;
        let due: Vec<_> = self
            .timers
            .iter()
            .filter(|(_, (at, _, _))| *at <= now)
            .map(|(k, v)| (*k, *v))
            .collect();
        for ((id, timer), expected) in due {
            if self.timers.get(&(id, timer)) != Some(&expected) {
                continue;
            }
            self.timers.remove(&(id, timer));
            self.invoke(id, |w, cx| w.timer(cx, timer));
        }
    }
    pub fn frame(&mut self, now: Duration) {
        assert!(now >= self.now);
        self.now = now;
        let due = std::mem::take(&mut self.frames);
        let time = FrameTime {
            now,
            elapsed: now.saturating_sub(self.last_frame),
        };
        self.last_frame = now;
        for id in due {
            if self.visible(id) {
                self.invoke(id, |w, cx| w.frame(cx, time));
            }
        }
    }
    pub fn next_work(&self) -> Work {
        Work {
            ready: !self.mailbox.items.is_empty(),
            frame: !self.frames.is_empty(),
            deadline: self.timers.values().map(|(at, _, _)| *at).min(),
        }
    }
}
