//! Executable ownership/routing experiment, not the production framework or a renderer.
mod types;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
pub use types::*;

pub trait Widget {
    fn mount(&mut self, _cx: &mut Cx<'_>) {}
    fn event(&mut self, _cx: &mut Cx<'_>, _phase: Phase, _input: &Input) {}
    fn action(&mut self, _cx: &mut Cx<'_>, action: Action) -> Option<Action> {
        Some(action)
    }
}
struct Node {
    key: Key,
    parent: Option<Key>,
    token: u16,
    children: Vec<Key>,
    visible: bool,
    placement: Placement,
    retiring: bool,
    mounted: bool,
    published: Option<VisibilityState>,
    widget: Option<Box<dyn Widget>>,
}
#[derive(Clone, Copy)]
struct AnchorState {
    key: Key,
    live: bool,
    available: bool,
}
#[derive(Clone, Copy)]
struct VisibilityState {
    visible: bool,
    anchor: Option<AnchorState>,
}
struct Slot {
    generation: u64,
    node: Option<Node>,
}
enum Request {
    Insert(u16, Box<dyn Widget>, Placement),
    Remove(Key),
    Send(Key, Box<dyn std::any::Any>),
    Emit(Box<dyn std::any::Any>),
    Schedule(u16, Option<Wake>, Lifetime),
    Capture(u32, bool),
    Focus,
}
impl Request {
    fn cost(&self) -> usize {
        match self {
            Self::Insert(..) => 2,
            Self::Schedule(..) | Self::Capture(..) | Self::Focus => 0,
            _ => 1,
        }
    }
}
enum Delivery {
    Mount(Key),
    Command(Key, Request),
    Input(Key, Input),
    Action(Key, Action, Option<Ticket>, bool),
}
pub struct Cx<'a> {
    pub me: Key,
    nodes: &'a [Slot],
    pending: Vec<Request>,
    limit: usize,
    cleanup: bool,
    deferred: bool,
    pub stop: bool,
}
impl Cx<'_> {
    fn push(&mut self, r: Request) -> Result<(), Error> {
        if self.cleanup {
            return Err(Error::Stale);
        }
        if self.pending.len() >= 64 {
            return Err(Error::Full);
        }
        let cost = if self.deferred {
            r.cost().max(1)
        } else {
            r.cost()
        };
        if cost > self.limit {
            return Err(Error::Full);
        }
        self.limit -= cost;
        self.pending.push(r);
        Ok(())
    }
    pub fn child(&self, token: u16) -> Option<Key> {
        self.nodes[self.me.slot]
            .node
            .as_ref()?
            .children
            .iter()
            .copied()
            .find(|k| {
                self.nodes[k.slot]
                    .node
                    .as_ref()
                    .is_some_and(|n| n.token == token)
            })
    }
    pub fn insert(&mut self, token: u16, widget: impl Widget + 'static) -> Result<(), Error> {
        self.insert_at(token, widget, Placement::default())
    }
    pub fn insert_at(
        &mut self,
        token: u16,
        widget: impl Widget + 'static,
        placement: Placement,
    ) -> Result<(), Error> {
        self.push(Request::Insert(token, Box::new(widget), placement))
    }
    pub fn remove(&mut self, key: Key) -> Result<(), Error> {
        self.push(Request::Remove(key))
    }
    pub fn send<T: 'static>(&mut self, key: Key, value: T) -> Result<(), (Error, T)> {
        if self.cleanup {
            return Err((Error::Stale, value));
        }
        if self.limit == 0 || self.pending.len() >= 64 {
            return Err((Error::Full, value));
        }
        self.limit -= 1;
        self.pending.push(Request::Send(key, Box::new(value)));
        Ok(())
    }
    pub fn emit<T: 'static>(&mut self, value: T) -> Result<(), (Error, T)> {
        if self.cleanup {
            return Err((Error::Stale, value));
        }
        if self.limit == 0 || self.pending.len() >= 64 {
            return Err((Error::Full, value));
        }
        self.limit -= 1;
        self.pending.push(Request::Emit(Box::new(value)));
        Ok(())
    }
    pub fn schedule(&mut self, slot: u16, wake: Option<Wake>, life: Lifetime) -> Result<(), Error> {
        self.push(Request::Schedule(slot, wake, life))
    }
    pub fn capture(&mut self, pointer: u32, yes: bool) -> Result<(), Error> {
        self.push(Request::Capture(pointer, yes))
    }
    pub fn focus(&mut self) -> Result<(), Error> {
        self.push(Request::Focus)
    }
}
pub struct Ui {
    id: u64,
    slots: Vec<Slot>,
    roots: Vec<Key>,
    queue: VecDeque<Delivery>,
    capacity: usize,
    wakes: BTreeMap<(Key, u16), (Wake, Lifetime, u64)>,
    tasks: BTreeMap<(Key, u16), u64>,
    captures: BTreeMap<u32, Key>,
    focus: Option<Key>,
    modal: Option<Key>,
    modal_history: Vec<(Key, Option<Key>)>,
    pressed: BTreeSet<u32>,
    buttons: BTreeSet<(u32, u16)>,
    session: u64,
    last_frame: u64,
    wake_epoch: u64,
    reserved: usize,
    notifying: bool,
    window_focused: bool,
    // Test observation sinks; production calls the application rather than retaining this log.
    pub app_actions: Vec<Action>,
    pub errors: Vec<Error>,
}
impl Ui {
    pub fn new(id: u64, capacity: usize) -> Self {
        assert!(capacity > 0);
        Self {
            id,
            slots: vec![],
            roots: vec![],
            queue: VecDeque::new(),
            capacity,
            wakes: BTreeMap::new(),
            tasks: BTreeMap::new(),
            captures: BTreeMap::new(),
            focus: None,
            modal: None,
            modal_history: vec![],
            pressed: BTreeSet::new(),
            buttons: BTreeSet::new(),
            session: 0,
            last_frame: 0,
            wake_epoch: 0,
            reserved: 0,
            notifying: false,
            window_focused: true,
            app_actions: vec![],
            errors: vec![],
        }
    }
    fn node(&self, key: Key) -> Option<&Node> {
        if key.ui != self.id {
            return None;
        }
        self.slots
            .get(key.slot)?
            .node
            .as_ref()
            .filter(|n| n.key == key)
    }
    fn node_mut(&mut self, key: Key) -> Option<&mut Node> {
        if key.ui != self.id {
            return None;
        }
        self.slots
            .get_mut(key.slot)?
            .node
            .as_mut()
            .filter(|n| n.key == key)
    }
    pub fn live(&self, key: Key) -> bool {
        self.node(key).is_some_and(|n| !n.retiring)
    }
    pub fn owner(&self, key: Key) -> Option<Key> {
        self.node(key)?.parent
    }
    pub fn child(&self, key: Key, token: u16) -> Option<Key> {
        self.node(key)?
            .children
            .iter()
            .copied()
            .find(|k| self.node(*k).is_some_and(|n| n.token == token))
    }
    pub fn insert(
        &mut self,
        parent: Option<Key>,
        token: u16,
        widget: impl Widget + 'static,
    ) -> Result<Key, Error> {
        self.insert_box(parent, token, Box::new(widget))
    }
    fn insert_box(
        &mut self,
        parent: Option<Key>,
        token: u16,
        widget: Box<dyn Widget>,
    ) -> Result<Key, Error> {
        if self.queue.len() + self.reserved >= self.capacity {
            return Err(Error::Full);
        }
        if let Some(p) = parent {
            if !self.live(p) {
                return Err(Error::Stale);
            }
            if self.child(p, token).is_some() {
                return Err(Error::DuplicateToken);
            }
        }
        let index = self
            .slots
            .iter()
            .position(|s| s.node.is_none())
            .unwrap_or(self.slots.len());
        if index == self.slots.len() {
            self.slots.push(Slot {
                generation: 0,
                node: None,
            })
        }
        let key = Key {
            ui: self.id,
            slot: index,
            generation: self.slots[index].generation,
        };
        self.slots[index].node = Some(Node {
            key,
            parent,
            token,
            children: vec![],
            visible: true,
            placement: Placement::default(),
            retiring: false,
            mounted: false,
            published: None,
            widget: Some(widget),
        });
        if let Some(p) = parent {
            self.node_mut(p).unwrap().children.push(key)
        } else {
            self.roots.push(key)
        }
        self.queue.push_back(Delivery::Mount(key));
        Ok(key)
    }
    pub fn descendant(&self, key: Key, owner: Key) -> bool {
        let mut k = Some(key);
        while let Some(at) = k {
            if at == owner {
                return self.live(at);
            }
            k = self.owner(at)
        }
        false
    }
    pub fn active(&self, key: Key) -> bool {
        let Some(n) = self.node(key) else {
            return false;
        };
        !n.retiring
            && n.mounted
            && n.visible
            && n.parent.is_none_or(|p| self.active(p))
            && n.placement
                .overlay_anchor
                .is_none_or(|a| self.world(a).is_some())
    }
    pub fn eligible(&self, key: Key) -> bool {
        self.world(key).is_some() && self.modal.is_none_or(|m| self.descendant(key, m))
    }
    pub fn world(&self, key: Key) -> Option<Affine> {
        if !self.active(key) {
            return None;
        }
        let n = self.node(key)?;
        let m = if n.placement.overlay_anchor.is_some() {
            n.placement.transform
        } else {
            n.placement.transform.then(
                n.parent
                    .map(|p| self.world(p))
                    .unwrap_or(Some(Affine::IDENTITY))?,
            )
        };
        m.inverse()?;
        Some(m)
    }
    pub fn paint_contains(&self, key: Key, p: Point) -> bool {
        let Some(n) = self.node(key) else {
            return false;
        };
        let Some(inv) = self.world(key).and_then(Affine::inverse) else {
            return false;
        };
        if !n.placement.bounds.contains(inv.point(p)) {
            return false;
        }
        if n.placement.overlay_anchor.is_some() {
            return true;
        }
        let mut parent = n.parent;
        while let Some(k) = parent {
            let n = self.node(k).unwrap();
            if n.placement.clip
                && !self
                    .world(k)
                    .and_then(Affine::inverse)
                    .is_some_and(|i| n.placement.bounds.contains(i.point(p)))
            {
                return false;
            }
            if n.placement.overlay_anchor.is_some() {
                break;
            }
            parent = n.parent;
        }
        true
    }
    fn order(&self, key: Key, flow: &mut Vec<Key>, overlays: &mut Vec<Key>) {
        let Some(n) = self.node(key) else { return };
        if n.placement.overlay_anchor.is_some() {
            overlays.push(key);
            return;
        }
        flow.push(key);
        for c in &n.children {
            self.order(*c, flow, overlays)
        }
    }
    pub fn hit(&self, p: Point) -> Option<Key> {
        let mut order = vec![];
        let mut overlays = vec![];
        for r in &self.roots {
            self.order(*r, &mut order, &mut overlays)
        }
        let mut i = 0;
        while i < overlays.len() {
            let k = overlays[i];
            order.push(k);
            if let Some(n) = self.node(k) {
                for c in &n.children {
                    self.order(*c, &mut order, &mut overlays)
                }
            }
            i += 1
        }
        order
            .into_iter()
            .rev()
            .find(|k| self.eligible(*k) && self.paint_contains(*k, p))
    }
    pub fn place(&mut self, key: Key, p: Placement) -> Result<(), Error> {
        if let Some(a) = p.overlay_anchor {
            if !self.live(a) || self.depends(a, key) {
                return Err(Error::InvalidGeometry);
            }
        }
        self.node_mut(key).ok_or(Error::Stale)?.placement = p;
        self.reconcile();
        self.publish_visibility();
        Ok(())
    }
    fn depends(&self, key: Key, target: Key) -> bool {
        key == target
            || self.node(key).is_some_and(|n| {
                n.parent.is_some_and(|p| self.depends(p, target))
                    || n.placement
                        .overlay_anchor
                        .is_some_and(|a| self.depends(a, target))
            })
    }
    pub fn hide(&mut self, key: Key, hidden: bool) -> Result<(), Error> {
        self.node_mut(key).ok_or(Error::Stale)?.visible = !hidden;
        self.reconcile();
        self.publish_visibility();
        Ok(())
    }
    fn anchor_state(&self, key: Key) -> Option<AnchorState> {
        let anchor = self.node(key)?.placement.overlay_anchor?;
        Some(AnchorState {
            key: anchor,
            live: self.live(anchor),
            available: self.world(anchor).is_some(),
        })
    }
    fn visibility(&self, key: Key) -> VisibilityState {
        VisibilityState {
            visible: self.world(key).is_some(),
            anchor: self.anchor_state(key),
        }
    }
    fn publish_visibility(&mut self) {
        let keys: Vec<_> = self
            .slots
            .iter()
            .filter_map(|s| {
                s.node
                    .as_ref()
                    .filter(|n| n.mounted && !n.retiring)
                    .map(|n| n.key)
            })
            .collect();
        for k in keys {
            if !self.live(k) {
                continue;
            }
            let now = self.visibility(k);
            // Record before callbacks so a nested publication cannot repeat this transition.
            let old = self.node_mut(k).unwrap().published.replace(now);
            let Some(old) = old else {
                continue;
            };
            if old.visible != now.visible {
                self.call_event(k, Phase::Target, &Input::Hidden(!now.visible), false);
            }
            if let (Some(was), Some(anchor)) = (old.anchor, now.anchor) {
                if was.key == anchor.key
                    && ((was.available && !anchor.available) || (was.live && !anchor.live))
                {
                    if let Some(owner) = self.owner(k) {
                        self.notice(
                            owner,
                            &Input::AnchorUnavailable {
                                overlay: k,
                                anchor: anchor.key,
                            },
                        );
                    }
                }
            }
        }
    }
    fn reconcile(&mut self) {
        let mut restore = None;
        let mut closed_scope = false;
        while self
            .modal_history
            .last()
            .is_some_and(|(k, _)| self.world(*k).is_none())
        {
            restore = self.modal_history.pop().unwrap().1;
            closed_scope = true;
        }
        self.modal = self.modal_history.last().map(|(k, _)| *k);
        if closed_scope {
            let next = restore.filter(|k| self.eligible(*k)).or(self.modal);
            self.change_focus(next.filter(|_| self.window_focused));
        }
        let invalid: Vec<_> = self
            .captures
            .iter()
            .filter(|(_, k)| !self.eligible(**k) || self.world(**k).is_none())
            .map(|(p, k)| (*p, *k))
            .collect();
        for (p, k) in invalid {
            self.captures.remove(&p);
            self.notice(k, &Input::CaptureLost { pointer: p });
        }
        if self
            .focus
            .is_some_and(|k| !self.eligible(k) || self.world(k).is_none())
        {
            self.change_focus(None)
        }
        let invalid: Vec<_> = self
            .wakes
            .iter()
            .filter(|((k, _), (_, l, _))| {
                !self.live(*k) || (*l == Lifetime::Visible && self.world(*k).is_none())
            })
            .map(|(k, _)| *k)
            .collect();
        for k in invalid {
            self.wakes.remove(&k);
        }
    }
    pub fn remove(&mut self, key: Key) -> Result<(), Error> {
        if !self.live(key) {
            return Err(Error::Stale);
        }
        fn collect(ui: &Ui, k: Key, out: &mut Vec<Key>) {
            for c in &ui.node(k).unwrap().children {
                collect(ui, *c, out)
            }
            out.push(k)
        }
        let mut retiring = vec![];
        collect(self, key, &mut retiring);
        for k in &retiring {
            self.node_mut(*k).unwrap().retiring = true;
        }
        self.tasks.retain(|(k, _), _| !retiring.contains(k));
        self.wakes.retain(|(k, _), _| !retiring.contains(k));
        let captures: Vec<_> = self
            .captures
            .iter()
            .filter(|(_, k)| retiring.contains(k))
            .map(|(p, k)| (*p, *k))
            .collect();
        for (p, k) in captures {
            self.captures.remove(&p);
            self.call_event(k, Phase::Target, &Input::CaptureLost { pointer: p }, true);
        }
        if self.focus.is_some_and(|k| retiring.contains(&k)) {
            let k = self.focus.take().unwrap();
            self.pressed.clear();
            self.session += 1;
            self.call_event(k, Phase::Target, &Input::CancelKeys, true);
            self.call_event(k, Phase::Target, &Input::Focus(false), true);
        }
        for k in retiring {
            if self.node(k).is_some_and(|n| n.mounted) {
                self.call_event(k, Phase::Target, &Input::Unmount, true);
            }
            if let Some(p) = self.owner(k) {
                self.node_mut(p).unwrap().children.retain(|c| *c != k)
            } else {
                self.roots.retain(|c| *c != k)
            }
            self.slots[k.slot].node = None;
            self.slots[k.slot].generation += 1;
            if self.modal == Some(k) {
                self.modal = None
            }
        }
        self.reconcile();
        self.publish_visibility();
        Ok(())
    }
    pub fn set_modal(&mut self, key: Option<Key>) -> Result<(), Error> {
        if let Some(k) = key {
            if self.world(k).is_none() {
                return Err(Error::Stale);
            }
            if self.modal == Some(k) {
                return Ok(());
            }
            if self.modal.is_some_and(|m| !self.descendant(k, m)) {
                return Err(Error::InvalidOwner);
            }
            self.modal_history.push((k, self.focus));
            self.modal = Some(k);
            self.reconcile();
            if self.focus.is_none() && self.window_focused {
                self.change_focus(Some(k));
            }
        } else if let Some((_, saved)) = self.modal_history.pop() {
            self.modal = self.modal_history.last().map(|(k, _)| *k);
            self.reconcile();
            let next = saved.filter(|k| self.eligible(*k)).or(self.modal);
            self.change_focus(next.filter(|_| self.window_focused));
        }
        Ok(())
    }
    fn change_focus(&mut self, next: Option<Key>) {
        if self.focus == next {
            return;
        }
        self.pressed.clear();
        self.session += 1;
        let previous = std::mem::replace(&mut self.focus, next);
        if let Some(k) = previous {
            self.notice(k, &Input::CancelKeys);
            self.notice(k, &Input::Focus(false));
        }
        if let Some(k) = next {
            self.notice(k, &Input::Focus(true));
        }
    }
    pub fn activate(&mut self) {
        self.window_focused = true;
    }
    pub fn blur(&mut self) {
        self.window_focused = false;
        self.change_focus(None);
        self.pressed.clear();
        self.buttons.clear();
        self.session += 1;
        let captures = std::mem::take(&mut self.captures);
        for (pointer, k) in captures {
            self.notice(k, &Input::CaptureLost { pointer });
        }
    }
    pub fn focused(&self) -> Option<Key> {
        self.focus
    }
    pub fn session(&self) -> u64 {
        self.session
    }
    pub fn pressed(&self, key: u32) -> bool {
        self.pressed.contains(&key)
    }
    pub fn captured(&self, pointer: u32) -> Option<Key> {
        self.captures.get(&pointer).copied()
    }
    fn callback(
        &mut self,
        key: Key,
        f: impl FnOnce(&mut dyn Widget, &mut Cx<'_>),
        cleanup: bool,
    ) -> bool {
        if !cleanup && !self.live(key) {
            return false;
        }
        let Some(mut widget) = self.node_mut(key).and_then(|n| n.widget.take()) else {
            return false;
        };
        let mut cx = Cx {
            me: key,
            nodes: &self.slots,
            pending: vec![],
            limit: if cleanup {
                0
            } else {
                self.capacity
                    .saturating_sub(self.queue.len() + self.reserved)
            },
            cleanup,
            deferred: self.notifying,
            stop: false,
        };
        f(widget.as_mut(), &mut cx);
        let stop = cx.stop;
        let pending = cx.pending;
        self.node_mut(key).unwrap().widget = Some(widget);
        if self.notifying {
            for r in pending {
                self.reserved += r.cost().saturating_sub(1);
                self.queue.push_back(Delivery::Command(key, r));
            }
        } else {
            self.reserved += pending.iter().map(Request::cost).sum::<usize>();
            for r in pending {
                self.reserved -= r.cost();
                self.commit(key, r);
            }
        }
        stop
    }
    fn commit(&mut self, key: Key, r: Request) {
        if let Err(e) = self.apply(key, r) {
            self.errors.push(e);
            if self.live(key) && self.queue.len() + self.reserved < self.capacity {
                self.queue
                    .push_back(Delivery::Input(key, Input::Rejected(e)));
            }
        }
    }
    // State transitions finish their notifications before any notification-emitted work runs.
    fn notice(&mut self, key: Key, event: &Input) {
        let previous = self.notifying;
        self.notifying = true;
        self.call_event(key, Phase::Target, event, false);
        self.notifying = previous;
    }
    fn call_event(&mut self, key: Key, phase: Phase, event: &Input, cleanup: bool) -> bool {
        self.callback(key, |w, cx| w.event(cx, phase, event), cleanup)
    }
    fn apply(&mut self, me: Key, r: Request) -> Result<(), Error> {
        if !self.live(me) {
            return Err(Error::Stale);
        }
        match r {
            Request::Insert(t, w, p) => {
                let result = if p.overlay_anchor.is_some_and(|a| !self.live(a)) {
                    Err(Error::InvalidGeometry)
                } else {
                    self.insert_box(Some(me), t, w).inspect(|&key| {
                        self.node_mut(key).unwrap().placement = p;
                    })
                };
                self.queue
                    .push_back(Delivery::Input(me, Input::Committed { token: t, result }));
            }
            Request::Remove(k) => {
                if self.owner(k) != Some(me) {
                    return Err(Error::InvalidOwner);
                }
                self.remove(k)?;
            }
            Request::Send(k, p) => {
                if self.owner(k) != Some(me) {
                    return Err(Error::InvalidOwner);
                }
                self.queue.push_back(Delivery::Action(
                    k,
                    Action {
                        source: me,
                        payload: p,
                    },
                    None,
                    false,
                ));
            }
            Request::Emit(p) => {
                let a = Action {
                    source: me,
                    payload: p,
                };
                if let Some(p) = self.owner(me) {
                    self.queue.push_back(Delivery::Action(p, a, None, true))
                } else {
                    self.app_actions.push(a)
                }
            }
            Request::Schedule(s, w, l) => {
                if let Some(w) = w {
                    if l == Lifetime::Mounted || self.world(me).is_some() {
                        self.wake_epoch += 1;
                        self.wakes.insert((me, s), (w, l, self.wake_epoch));
                    }
                } else {
                    self.wakes.remove(&(me, s));
                }
            }
            Request::Capture(p, yes) => {
                if yes && self.window_focused && self.eligible(me) {
                    if let Some(old) = self.captures.insert(p, me).filter(|old| *old != me) {
                        self.notice(old, &Input::CaptureLost { pointer: p });
                    }
                } else if self.captures.get(&p) == Some(&me) {
                    self.captures.remove(&p);
                    self.notice(me, &Input::CaptureLost { pointer: p });
                }
            }
            Request::Focus => {
                if self.window_focused && self.eligible(me) && self.focus != Some(me) {
                    self.change_focus(Some(me));
                }
            }
        }
        Ok(())
    }
    pub fn pump(&mut self, budget: usize) -> usize {
        let mut done = 0;
        while done < budget {
            let Some(d) = self.queue.pop_front() else {
                break;
            };
            done += 1;
            match d {
                Delivery::Mount(k) => {
                    if let Some(n) = self.node_mut(k) {
                        n.mounted = true;
                    }
                    let initial = self.visibility(k);
                    if let Some(n) = self.node_mut(k) {
                        n.published = Some(initial);
                    }
                    self.callback(k, |w, cx| w.mount(cx), false);
                    self.publish_visibility();
                }
                Delivery::Command(k, r) => {
                    self.reserved -= r.cost().saturating_sub(1);
                    self.commit(k, r);
                }
                Delivery::Input(k, e) => {
                    self.call_event(k, Phase::Target, &e, false);
                }
                Delivery::Action(k, a, ticket, bubble) => {
                    if !self.live(a.source) || !self.live(k) {
                        continue;
                    }
                    if ticket.is_some_and(|t| self.tasks.get(&(t.owner, t.slot)) != Some(&t.epoch))
                    {
                        continue;
                    }
                    if bubble {
                        self.reserved += 1;
                    }
                    let mut forwarded = None;
                    self.callback(k, |w, cx| forwarded = w.action(cx, a), false);
                    if bubble {
                        self.reserved -= 1;
                    }
                    if !bubble {
                        if forwarded.is_some() {
                            self.errors.push(Error::Unhandled);
                        }
                        continue;
                    }
                    if let Some(a) = forwarded {
                        if let Some(p) = self.owner(k) {
                            self.queue.push_back(Delivery::Action(p, a, None, true))
                        } else {
                            self.app_actions.push(a)
                        }
                    }
                }
            }
        }
        done
    }
    pub fn dispatch(&mut self, input: Input) {
        if let Input::Button {
            pointer,
            button,
            down: true,
            ..
        } = input
        {
            self.buttons.insert((pointer, button));
        }
        self.route_input(&input);
        if let Input::Button {
            pointer,
            button,
            down: false,
            ..
        } = input
        {
            self.buttons.remove(&(pointer, button));
            if !self.buttons.iter().any(|(p, _)| *p == pointer) {
                if let Some(k) = self.captures.remove(&pointer) {
                    self.notice(k, &Input::CaptureLost { pointer });
                }
            }
        }
    }
    fn route_input(&mut self, input: &Input) {
        if let Input::Key { physical, down, .. } = input {
            if *down {
                self.pressed.insert(*physical);
            } else {
                self.pressed.remove(physical);
            }
        }
        if let Input::Text { session, .. } = input {
            if *session != self.session {
                return;
            }
        }
        let target = if let Some(p) = input.position() {
            input
                .pointer()
                .and_then(|p| self.captured(p))
                .or_else(|| self.hit(p))
                .or(self.modal)
        } else {
            self.focus
        };
        let Some(target) = target.filter(|k| self.eligible(*k)) else {
            return;
        };
        if matches!(input, Input::Text { .. }) {
            self.call_event(target, Phase::Target, input, false);
            return;
        }
        let mut path = vec![target];
        let mut parent = self.owner(target);
        while let Some(p) = parent {
            path.push(p);
            parent = self.owner(p)
        }
        let input_session = self.session;
        for phase in [Phase::Preview, Phase::Target, Phase::Bubble] {
            let keys: Vec<Key> = match phase {
                Phase::Preview => path
                    .iter()
                    .rev()
                    .copied()
                    .filter(|k| *k != target)
                    .collect(),
                Phase::Target => vec![target],
                Phase::Bubble => path.iter().skip(1).copied().collect(),
            };
            for k in keys {
                if matches!(input, Input::Key { .. }) && self.session != input_session {
                    return;
                }
                if !self.eligible(k) {
                    continue;
                }
                let Some(inv) = self.world(k).and_then(Affine::inverse) else {
                    continue;
                };
                if self.call_event(k, phase, &input.local(inv), false) {
                    return;
                }
            }
        }
    }
    pub fn schedule(
        &mut self,
        key: Key,
        slot: u16,
        wake: Option<Wake>,
        life: Lifetime,
    ) -> Result<(), Error> {
        self.apply(key, Request::Schedule(slot, wake, life))
    }
    pub fn advance(&mut self, now: u64) {
        let due: Vec<_> = self
            .wakes
            .iter()
            .filter(|((k, _), (w, _, _))| {
                self.node(*k).is_some_and(|n| n.mounted) && matches!(w,Wake::At(t)if *t<=now)
            })
            .map(|(k, v)| (*k, *v))
            .collect();
        for ((k, s), expected) in due {
            if self.wakes.get(&(k, s)) != Some(&expected) {
                continue;
            }
            self.wakes.remove(&(k, s));
            self.call_event(k, Phase::Target, &Input::Wake(s), false);
        }
    }
    pub fn frame(&mut self, now: u64) {
        let due: Vec<_> = self
            .wakes
            .iter()
            .filter(|((k, _), (w, _, _))| {
                self.node(*k).is_some_and(|n| n.mounted) && *w == Wake::Frame
            })
            .map(|(k, v)| (*k, *v))
            .collect();
        let elapsed = now.saturating_sub(self.last_frame);
        self.last_frame = now;
        for ((k, s), expected) in due {
            if self.wakes.get(&(k, s)) != Some(&expected) {
                continue;
            }
            self.wakes.remove(&(k, s));
            self.call_event(k, Phase::Target, &Input::Frame { now, elapsed }, false);
        }
    }
    pub fn next_work(&self) -> Work {
        Work {
            ready: !self.queue.is_empty(),
            frame: self.wakes.values().any(|(w, _, _)| *w == Wake::Frame),
            deadline: self
                .wakes
                .values()
                .filter_map(|(w, _, _)| if let Wake::At(t) = w { Some(*t) } else { None })
                .min(),
        }
    }
    pub fn begin_task(&mut self, key: Key, slot: u16) -> Result<Ticket, Error> {
        if !self.live(key) {
            return Err(Error::Stale);
        }
        let e = self.tasks.entry((key, slot)).or_default();
        *e += 1;
        Ok(Ticket {
            owner: key,
            slot,
            epoch: *e,
        })
    }
    pub fn receive<T: 'static>(&mut self, t: Ticket, payload: T) -> Result<(), (Error, T)> {
        if !self.live(t.owner) || self.tasks.get(&(t.owner, t.slot)) != Some(&t.epoch) {
            return Err((Error::Stale, payload));
        }
        if self.queue.len() + self.reserved >= self.capacity {
            return Err((Error::Full, payload));
        }
        self.queue.push_back(Delivery::Action(
            t.owner,
            Action {
                source: t.owner,
                payload: Box::new(payload),
            },
            Some(t),
            false,
        ));
        Ok(())
    }
    pub fn queued(&self) -> usize {
        self.queue.len()
    }
    pub fn mounted(&self) -> usize {
        self.slots.iter().filter(|s| s.node.is_some()).count()
    }
}

/// Model of host move batching. All non-move input is an ordering barrier.
#[derive(Default)]
pub struct InputBatch {
    moves: BTreeMap<u32, Input>,
}
impl InputBatch {
    pub fn push(&mut self, input: Input) -> Vec<Input> {
        if let Input::Move { pointer, .. } = input {
            self.moves.insert(pointer, input);
            vec![]
        } else {
            let mut out = self.flush();
            out.push(input);
            out
        }
    }
    pub fn flush(&mut self) -> Vec<Input> {
        std::mem::take(&mut self.moves).into_values().collect()
    }
}
/// Fixed-height list baseline. A variable-height controller can replace this index.
pub fn visible_rows(
    total: usize,
    offset: f64,
    height: f64,
    row: f64,
    overscan: usize,
) -> std::ops::Range<usize> {
    assert!(row.is_finite() && row > 0.);
    let first = (offset.max(0.) / row).floor() as usize;
    first.saturating_sub(overscan).min(total)
        ..(first
            .saturating_add((height.max(0.) / row).ceil() as usize)
            .saturating_add(1)
            .saturating_add(overscan))
        .min(total)
}
