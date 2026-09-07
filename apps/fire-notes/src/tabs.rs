use crate::components::{label, paper, place};
use fire_ui::*;
use fire_ui_widgets::*;
use std::{rc::Rc, sync::Arc};

pub fn tab_width(title: &str) -> f32 {
    (title.len() as f32 * 9. + 32.).max(100.)
}
#[derive(Clone, Debug)]
pub enum TabAction {
    Window(WindowAction),
    Select(usize),
    BeginRename(usize),
    Context(usize, Point),
    Rename(usize, Arc<str>),
    RenameDone(usize),
    Reorder(usize, usize),
}
impl Data for TabAction {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + if let Self::Rename(_, s) = self {
                s.len()
            } else {
                0
            }
    }
}
#[derive(Clone, Debug)]
struct TabState {
    title: Arc<str>,
    active: bool,
}
impl Data for TabState {
    fn bytes(&self) -> usize {
        self.title.len() + std::mem::size_of::<Self>()
    }
}
enum TabCommand {
    State(TabState),
    Rename,
    Edit(EditorOutput),
}
impl Data for TabCommand {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::State(s) => s.bytes(),
                Self::Edit(o) => o.bytes(),
                _ => 0,
            }
    }
}
struct Tab {
    id: usize,
    state: TabState,
    title: Child<Label>,
    editor: Child<Editor>,
    renaming: bool,
    draft: Arc<str>,
    pressed: Option<u32>,
}
impl Tab {
    fn new(id: usize, state: TabState) -> Element<Self> {
        Element::build(|c| Self {
            id,
            title: c.add(label(state.title.clone(), 14., paper().foreground)),
            editor: c.connect(
                Element::leaf(
                    Editor::field("")
                        .chrome(false)
                        .caret_blink(false)
                        .max_bytes(crate::storage::MAX_TITLE_BYTES),
                ),
                |o| TabCommand::Edit(o.clone()),
            ),
            draft: state.title.clone(),
            state,
            renaming: false,
            pressed: None,
        })
    }
    fn finish(&mut self, cx: &mut Update<'_, Self>, commit: bool) {
        self.renaming = false;
        let _ = cx.show(self.editor, false);
        let _ = cx.show(self.title, true);
        if commit {
            let _ = cx.emit(TabAction::Rename(self.id, self.draft.clone()));
        }
        let _ = cx.emit(TabAction::RenameDone(self.id));
        cx.relayout();
    }
}
impl Widget for Tab {
    type Command = TabCommand;
    type Output = TabAction;
    fn lifecycle(&mut self, cx: &mut Update<'_, Self>, e: Lifecycle) {
        if e == Lifecycle::Mount {
            let _ = cx.show(self.editor, false);
            let _ = cx.set_environment(
                self.editor,
                Rc::new(Theme {
                    font_size: 14.,
                    background: Color(0.15, 0.05, 0.05, 1.),
                    ..paper()
                }),
                true,
            );
        }
        if matches!(e, Lifecycle::CaptureLost(_)) {
            self.pressed = None;
        }
    }
    fn update(&mut self, cx: &mut Update<'_, Self>, c: TabCommand) {
        match c {
            TabCommand::State(state) => {
                let _ = cx.send(self.title, state.title.to_string());
                self.state = state;
                cx.relayout();
            }
            TabCommand::Rename => {
                self.renaming = true;
                self.draft = self.state.title.clone();
                let _ = cx.send(self.editor, Edit::Set(self.draft.to_string()));
                let _ = cx.show(self.title, false);
                let _ = cx.show(self.editor, true);
                let _ = cx.focus_child(self.editor);
                let _ = cx.send(
                    self.editor,
                    Edit::Select {
                        anchor: 0,
                        caret: usize::MAX,
                    },
                );
                cx.relayout();
            }
            TabCommand::Edit(EditorOutput::Changed { text, .. }) => {
                self.draft = text;
                cx.relayout();
            }
            TabCommand::Edit(EditorOutput::Submitted) => self.finish(cx, true),
            TabCommand::Edit(EditorOutput::FocusChanged(false)) if self.renaming => {
                self.renaming = false;
                let _ = cx.show(self.editor, false);
                let _ = cx.show(self.title, true);
                let _ = cx.emit(TabAction::Rename(self.id, self.draft.clone()));
                cx.relayout();
            }
            _ => {}
        }
    }
    fn cursor(&self, _position: Point) -> Option<CursorIcon> {
        Some(CursorIcon::Pointer)
    }
    fn focusable(&self) -> bool {
        true
    }
    fn input(&mut self, cx: &mut Update<'_, Self>, phase: Phase, input: &Input) {
        if let Input::Button {
            button: 2,
            down: true,
            position,
            ..
        } = input
        {
            let _ = cx.emit(TabAction::Context(self.id, *position));
            cx.stop();
            return;
        }
        if self.renaming {
            if phase == Phase::Preview
                && matches!(
                    input,
                    Input::Key {
                        key: Key::Escape,
                        down: true,
                        ..
                    }
                )
            {
                self.finish(cx, false);
                cx.stop();
            }
            return;
        }
        if phase != Phase::Target && phase != Phase::Bubble {
            return;
        }
        match input {
            Input::Button {
                button: 3,
                down: true,
                ..
            } => {
                let _ = cx.emit(TabAction::BeginRename(self.id));
                cx.stop();
            }
            Input::Button {
                pointer,
                button: 1,
                down: true,
                ..
            } => {
                self.pressed = Some(*pointer);
                let _ = cx.capture(*pointer);
                let _ = cx.emit(TabAction::Select(self.id));
                cx.stop();
            }
            Input::Button {
                pointer,
                button: 1,
                down: false,
                ..
            } => {
                self.pressed = None;
                let _ = cx.release(*pointer);
                cx.stop();
            }
            Input::Key {
                key: Key::Enter,
                down: true,
                ..
            } => {
                let _ = cx.emit(TabAction::Select(self.id));
                cx.stop();
            }
            _ => {}
        }
    }
    fn layout(&mut self, cx: &mut Layout<'_>, c: Constraints) -> Metrics {
        let m = cx.measure(
            self.title,
            Constraints {
                min: Size::ZERO,
                max: Size::new(c.max.width, 40.),
            },
        );
        cx.place(
            self.title,
            Point::new(((c.max.width - m.size.width) / 2.).round(), 10.),
        );
        let width =
            (self.draft.chars().count() as f32 * 8.43 + 4.).clamp(8., (c.max.width - 12.).max(8.));
        place(
            cx,
            self.editor,
            Rect::new(((c.max.width - width) / 2.).round(), 10., width, 18.),
        );
        Metrics::new(c.max)
    }
    fn paint(&self, cx: &mut Paint<'_>) {
        let bg = if self.state.active {
            Color(0.15, 0.05, 0.05, 1.)
        } else if cx.hovered {
            Color(0.25, 0.1, 0.05, 1.)
        } else {
            Color(0.05, 0.02, 0.02, 1.)
        };
        cx.painter.rect(cx.bounds, 0., bg.into());
        if self.state.active {
            cx.painter.rect(
                Rect::new(0., 0., cx.bounds.width, 2.),
                0.,
                Color(1., 0.4, 0., 1.).into(),
            );
        }
        if self.renaming {
            let width = (self.draft.chars().count() as f32 * 8.43).min(cx.bounds.width - 12.);
            cx.painter.rect(
                Rect::new((cx.bounds.width - width) / 2., 28., width, 2.),
                0.,
                paper().foreground.into(),
            );
        }
    }
    fn semantics(&self) -> Semantics {
        Semantics {
            role: Role::Button,
            label: self.state.title.to_string(),
            selected: self.state.active,
            ..Semantics::default()
        }
    }
    fn accessibility(&mut self, cx: &mut Update<'_, Self>, a: SemanticAction) {
        match a {
            SemanticAction::Activate => {
                let _ = cx.emit(TabAction::Select(self.id));
            }
            SemanticAction::Focus => {
                let _ = cx.focus();
            }
        }
    }
}
pub enum TabsCommand {
    Sync(Vec<(usize, Arc<str>)>, Option<usize>),
    Action(TabAction),
    Rename(usize),
}
impl Data for TabsCommand {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Sync(v, _) => v.iter().map(|(_, s)| s.len() + 24).sum(),
                Self::Action(a) => a.bytes(),
                _ => 0,
            }
    }
}
pub struct Tabs {
    children: Vec<(usize, Arc<str>, Child<Tab>)>,
    active: Option<usize>,
    offset: f32,
    extent: f32,
    rects: Vec<(usize, Rect)>,
    drag: Option<(usize, f32)>,
    reveal: bool,
}
impl Tabs {
    pub fn new(items: Vec<(usize, Arc<str>)>, active: Option<usize>) -> Element<Self> {
        Element::build(|c| Self {
            children: items
                .into_iter()
                .map(|(id, title)| {
                    let child = c.connect(
                        Tab::new(
                            id,
                            TabState {
                                title: title.clone(),
                                active: Some(id) == active,
                            },
                        ),
                        |a| TabsCommand::Action(a.clone()),
                    );
                    (id, title, child)
                })
                .collect(),
            active,
            offset: 0.,
            extent: 0.,
            rects: vec![],
            drag: None,
            reveal: true,
        })
    }
}
impl Widget for Tabs {
    type Command = TabsCommand;
    type Output = TabAction;
    fn update(&mut self, cx: &mut Update<'_, Self>, c: TabsCommand) {
        match c {
            TabsCommand::Action(TabAction::Context(id, point)) => {
                if let Some((_, rect)) = self.rects.iter().find(|(key, _)| *key == id) {
                    let _ = cx.emit(TabAction::Context(
                        id,
                        Point::new(rect.x + point.x, rect.y + point.y),
                    ));
                }
            }
            TabsCommand::Action(a) => {
                let _ = cx.emit(a);
            }
            TabsCommand::Rename(id) => {
                if let Some((_, _, child)) = self.children.iter().find(|(key, _, _)| *key == id) {
                    let _ = cx.send(*child, TabCommand::Rename);
                }
            }
            TabsCommand::Sync(items, active) => {
                self.children.retain(|(id, _, child)| {
                    items.iter().any(|(key, _)| key == id) || cx.remove(*child).is_err()
                });
                let mut next = vec![];
                for (id, title) in items {
                    let state = TabState {
                        title: title.clone(),
                        active: Some(id) == active,
                    };
                    if let Some((_, _, child)) = self.children.iter().find(|(key, _, _)| *key == id)
                    {
                        let _ = cx.send(*child, TabCommand::State(state));
                        next.push((id, title, *child));
                    } else if let Ok(child) =
                        cx.insert(Tab::new(id, state), |a| TabsCommand::Action(a.clone()))
                    {
                        next.push((id, title, child));
                    }
                }
                self.children = next;
                self.reveal |= self.active != active;
                self.active = active;
                cx.relayout();
            }
        }
    }
    fn input(&mut self, cx: &mut Update<'_, Self>, phase: Phase, input: &Input) {
        match input {
            Input::Button {
                button: 1,
                down: true,
                ..
            } if phase == Phase::Target => {
                let _ = cx.emit(TabAction::Window(WindowAction::Drag));
                cx.stop();
            }
            Input::Scroll { delta, .. } => {
                self.offset = (self.offset - delta.y - delta.x)
                    .clamp(0., (self.extent - cx.bounds().width).max(0.));
                self.reveal = false;
                cx.relayout();
                cx.stop();
            }
            Input::Button {
                button: 1,
                down: true,
                position,
                ..
            } if phase == Phase::Preview => {
                self.drag = self
                    .rects
                    .iter()
                    .find(|(_, r)| r.contains(*position))
                    .map(|(id, _)| (*id, position.x));
            }
            Input::Pointer { position, .. } if phase == Phase::Preview => {
                if let Some((id, start)) = self.drag {
                    if (position.x - start).abs() > 5. {
                        if let Some((other, _)) = self
                            .rects
                            .iter()
                            .find(|(key, r)| *key != id && r.contains(*position))
                        {
                            let _ = cx.emit(TabAction::Reorder(id, *other));
                            self.drag = Some((id, position.x));
                            cx.stop();
                        }
                    }
                }
            }
            Input::Button {
                button: 1,
                down: false,
                ..
            } => self.drag = None,
            _ => {}
        }
    }
    fn layout(&mut self, cx: &mut Layout<'_>, c: Constraints) -> Metrics {
        self.extent = self
            .children
            .iter()
            .map(|(_, t, _)| tab_width(t) + 1.)
            .sum();
        self.offset = self.offset.clamp(0., (self.extent - c.max.width).max(0.));
        if self.reveal {
            let mut x = 0.;
            for (id, t, _) in &self.children {
                let w = tab_width(t);
                if Some(*id) == self.active {
                    if x < self.offset {
                        self.offset = x;
                    } else if x + w > self.offset + c.max.width {
                        self.offset = x + w - c.max.width;
                    }
                }
                x += w + 1.;
            }
            self.reveal = false;
        }
        self.rects.clear();
        let mut x = -self.offset;
        for (id, t, child) in &self.children {
            let w = tab_width(t);
            let rect = Rect::new(x, 0., w, 40.);
            self.rects.push((*id, rect));
            place(cx, *child, rect);
            x += w + 1.;
        }
        Metrics::new(c.max)
    }
}
#[derive(Clone, Copy)]
pub enum ChromeAction {
    New,
    Minimize,
    Maximize,
    Close,
}
impl Data for ChromeAction {
    fn bytes(&self) -> usize {
        1
    }
}
pub struct ChromeButton {
    action: ChromeAction,
    pressed: Option<u32>,
}
impl ChromeButton {
    pub fn new(action: ChromeAction) -> Element<Self> {
        Element::leaf(Self {
            action,
            pressed: None,
        })
    }
}
impl Widget for ChromeButton {
    type Command = ();
    type Output = ChromeAction;
    fn cursor(&self, _position: Point) -> Option<CursorIcon> {
        Some(CursorIcon::Pointer)
    }
    fn focusable(&self) -> bool {
        true
    }
    fn input(&mut self, cx: &mut Update<'_, Self>, phase: Phase, input: &Input) {
        if phase != Phase::Target {
            return;
        }
        match input {
            Input::Button {
                pointer,
                button: 1,
                down: true,
                ..
            } => {
                self.pressed = Some(*pointer);
                let _ = cx.capture(*pointer);
                cx.stop();
            }
            Input::Button {
                pointer,
                button: 1,
                down: false,
                position,
            } => {
                if self.pressed.take() == Some(*pointer) && cx.bounds().contains(*position) {
                    let _ = cx.emit(self.action);
                }
                let _ = cx.release(*pointer);
                cx.stop();
            }
            Input::Key {
                key: Key::Enter | Key::Character(' '),
                down: true,
                ..
            } => {
                let _ = cx.emit(self.action);
                cx.stop();
            }
            _ => {}
        }
    }
    fn lifecycle(&mut self, _: &mut Update<'_, Self>, e: Lifecycle) {
        if matches!(e, Lifecycle::CaptureLost(_)) {
            self.pressed = None;
        }
    }
    fn layout(&mut self, _: &mut Layout<'_>, c: Constraints) -> Metrics {
        Metrics::new(c.constrain(Size::new(28., 28.)))
    }
    fn paint(&self, cx: &mut Paint<'_>) {
        let bg = if cx.hovered && matches!(self.action, ChromeAction::Close) {
            Color(0.9, 0.2, 0.2, 1.)
        } else if cx.hovered {
            Color(0.3, 0.1, 0.05, 1.)
        } else {
            Color(0.1, 0.03, 0.03, 1.)
        };
        cx.painter.rect(cx.bounds, 4., bg.into());
        let color = Color(1., 0.6, 0., 1.);
        let x = cx.bounds.width / 2.;
        let y = cx.bounds.height / 2.;
        let line = |p: &mut dyn Painter, a: Point, b: Point| {
            p.path(&[Path::Move(a), Path::Line(b)], color.into(), Some(1.5))
        };
        match self.action {
            ChromeAction::Close => {
                line(
                    cx.painter,
                    Point::new(x - 5., y - 5.),
                    Point::new(x + 5., y + 5.),
                );
                line(
                    cx.painter,
                    Point::new(x + 5., y - 5.),
                    Point::new(x - 5., y + 5.),
                );
            }
            ChromeAction::Maximize => {
                cx.painter
                    .stroke(Rect::new(x - 5., y - 5., 10., 10.), 0., 1.5, color)
            }
            ChromeAction::Minimize => {
                line(cx.painter, Point::new(x - 5., y), Point::new(x + 5., y))
            }
            ChromeAction::New => {
                line(cx.painter, Point::new(x - 5., y), Point::new(x + 5., y));
                line(cx.painter, Point::new(x, y - 5.), Point::new(x, y + 5.));
            }
        }
    }
    fn semantics(&self) -> Semantics {
        Semantics {
            role: Role::Button,
            label: match self.action {
                ChromeAction::New => "New note",
                ChromeAction::Close => "Close window",
                ChromeAction::Minimize => "Minimize",
                ChromeAction::Maximize => "Maximize or restore",
            }
            .into(),
            ..Semantics::default()
        }
    }
    fn accessibility(&mut self, cx: &mut Update<'_, Self>, a: SemanticAction) {
        match a {
            SemanticAction::Activate => {
                let _ = cx.emit(self.action);
            }
            SemanticAction::Focus => {
                let _ = cx.focus();
            }
        }
    }
}
