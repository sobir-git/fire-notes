use fire_ui::*;
use fire_ui_widgets::*;
use std::{collections::HashMap, rc::Rc, sync::Arc};

pub fn label(text: impl Into<Arc<str>>, size: f32, color: Color) -> Element<Label> {
    Element::leaf(Label::new(text).appearance(Appearance {
        font_size: Some(size),
        foreground: Some(color),
    }))
}
pub fn button(text: &str) -> Element<Button<Label>> {
    Button::new(label(text, 14., Color::hex(0xeee9df)), text)
}
pub fn place<W: Widget>(cx: &mut Layout<'_>, child: Child<W>, rect: Rect) {
    cx.measure(child, Constraints::tight(rect.size()));
    cx.place(child, Point::new(rect.x, rect.y));
}
pub fn paper() -> Theme {
    Theme {
        background: Color::hex(0x191d1a),
        font_size: 17.,
        inset: 0.,
        ..Theme::default()
    }
}
#[derive(Clone, Debug)]
pub enum PageOutput {
    Title(Arc<str>),
    Body(Arc<str>),
    LimitReached,
    Close,
}
impl Data for PageOutput {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Title(s) | Self::Body(s) => s.len(),
                Self::Close | Self::LimitReached => 0,
            }
    }
}
#[derive(Clone, Debug)]
pub enum PageCommand {
    Title(EditorOutput),
    Body(EditorOutput),
    Focus,
    Rename,
    Wrap(bool),
    Close,
}
impl Data for PageCommand {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Title(o) | Self::Body(o) => o.bytes(),
                _ => 0,
            }
    }
}
pub struct Page {
    title: Child<Editor>,
    body: Child<Editor>,
}
impl Page {
    pub fn new(title: Arc<str>, body: Arc<str>) -> Element<Self> {
        Element::build(|c| Self {
            title: c.connect(
                Element::leaf(
                    Editor::field(title.to_string())
                        .max_bytes(crate::storage::MAX_TITLE_BYTES)
                        .chrome(false)
                        .caret_blink(false)
                        .placeholder("Untitled"),
                ),
                |o| PageCommand::Title(o.clone()),
            ),
            body: c.connect(
                Element::leaf(
                    Editor::new(body.to_string())
                        .max_bytes(crate::storage::MAX_BODY_BYTES)
                        .chrome(false)
                        .caret_blink(false)
                        .placeholder("Start writing…"),
                ),
                |o| PageCommand::Body(o.clone()),
            ),
        })
    }
}
impl Widget for Page {
    type Command = PageCommand;
    type Output = PageOutput;
    fn lifecycle(&mut self, cx: &mut Update<'_, Self>, e: Lifecycle) {
        if e == Lifecycle::Mount {
            let _ = cx.set_environment(
                self.title,
                Rc::new(Theme {
                    font_size: 30.,
                    ..paper()
                }),
                true,
            );
            let _ = cx.set_environment(self.body, Rc::new(paper()), true);
        }
    }
    fn update(&mut self, cx: &mut Update<'_, Self>, command: PageCommand) {
        match command {
            PageCommand::Title(EditorOutput::LimitReached)
            | PageCommand::Body(EditorOutput::LimitReached) => {
                let _ = cx.emit(PageOutput::LimitReached);
            }
            PageCommand::Title(EditorOutput::Changed { text, .. }) => {
                let _ = cx.emit(PageOutput::Title(text));
            }
            PageCommand::Body(EditorOutput::Changed { text, .. }) => {
                let _ = cx.emit(PageOutput::Body(text));
            }
            PageCommand::Focus => {
                let _ = cx.focus_child(self.body);
            }
            PageCommand::Rename => {
                let _ = cx.focus_child(self.title);
                let _ = cx.send(
                    self.title,
                    Edit::Select {
                        anchor: 0,
                        caret: usize::MAX,
                    },
                );
            }
            PageCommand::Wrap(w) => {
                let _ = cx.send(self.body, Edit::Wrap(w));
            }
            PageCommand::Close => {
                let _ = cx.emit(PageOutput::Close);
            }
            _ => {}
        }
    }
    fn layout(&mut self, cx: &mut Layout<'_>, c: Constraints) -> Metrics {
        place(cx, self.title, Rect::new(0., 0., c.max.width, 52.));
        place(
            cx,
            self.body,
            Rect::new(0., 78., c.max.width, (c.max.height - 78.).max(0.)),
        );
        Metrics::new(c.max)
    }
}
#[derive(Clone, Debug)]
pub enum TabAction {
    Select(usize),
    Close(usize),
}
impl Data for TabAction {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}
#[derive(Clone, Debug)]
pub struct TabState {
    pub title: Arc<str>,
    pub active: bool,
}
impl Data for TabState {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>() + self.title.len()
    }
}
struct Tab {
    id: usize,
    title: Child<Label>,
    state: TabState,
    pressed: Option<u32>,
}
impl Tab {
    fn new(id: usize, state: TabState) -> Element<Self> {
        Element::build(|c| Self {
            id,
            title: c.add(label(state.title.clone(), 14., Color::hex(0xd9dfd3))),
            state,
            pressed: None,
        })
    }
}
impl Widget for Tab {
    type Command = TabState;
    type Output = TabAction;
    fn update(&mut self, cx: &mut Update<'_, Self>, s: TabState) {
        let _ = cx.send(self.title, s.title.to_string());
        self.state = s;
        cx.repaint();
    }
    fn focusable(&self) -> bool {
        true
    }
    fn layout(&mut self, cx: &mut Layout<'_>, c: Constraints) -> Metrics {
        place(
            cx,
            self.title,
            Rect::new(15., 13., (c.max.width - 50.).max(0.), 22.),
        );
        Metrics::new(c.max)
    }
    fn lifecycle(&mut self, _: &mut Update<'_, Self>, e: Lifecycle) {
        if matches!(e, Lifecycle::CaptureLost(_) | Lifecycle::Focus(false)) {
            self.pressed = None;
        }
    }
    fn input(&mut self, cx: &mut Update<'_, Self>, phase: Phase, input: &Input) {
        if phase == Phase::Preview {
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
                let _ = cx.focus();
                cx.stop();
            }
            Input::Button {
                pointer,
                button: 1,
                down: false,
                position,
            } if self.pressed == Some(*pointer) => {
                self.pressed = None;
                let _ = cx.release(*pointer);
                if cx.bounds().contains(*position) {
                    let _ = cx.emit(if position.x > cx.bounds().width - 31. {
                        TabAction::Close(self.id)
                    } else {
                        TabAction::Select(self.id)
                    });
                }
                cx.stop();
            }
            Input::Key {
                key: Key::Enter | Key::Character(' '),
                down: true,
                repeat: false,
                ..
            } => {
                let _ = cx.emit(TabAction::Select(self.id));
                cx.stop();
            }
            _ => {}
        }
    }
    fn paint(&self, cx: &mut Paint<'_>) {
        if self.state.active || cx.hovered {
            cx.painter.rect(
                cx.bounds,
                6.,
                Color::hex(if self.state.active {
                    0x30392f
                } else {
                    0x242c25
                })
                .into(),
            );
        }
        if self.state.active {
            cx.painter.rect(
                Rect::new(12., cx.bounds.height - 2., cx.bounds.width - 24., 2.),
                1.,
                Color::hex(0xe9ae73).into(),
            );
        }
        let x = cx.bounds.width - 19.;
        let y = cx.bounds.height / 2.;
        cx.painter.path(
            &[
                Path::Move(Point::new(x - 3., y - 3.)),
                Path::Line(Point::new(x + 3., y + 3.)),
                Path::Move(Point::new(x + 3., y - 3.)),
                Path::Line(Point::new(x - 3., y + 3.)),
            ],
            Color::hex(0x91a28f).into(),
            Some(1.2),
        );
        if cx.focused {
            cx.painter
                .stroke(cx.bounds.inset(0.5), 6., 1., Color::hex(0x788c71));
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
    fn accessibility(&mut self, cx: &mut Update<'_, Self>, action: SemanticAction) {
        match action {
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
}
impl Data for TabsCommand {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Sync(items, _) => items.iter().map(|(_, t)| t.len() + 24).sum(),
                _ => 0,
            }
    }
}
pub struct Tabs {
    children: Vec<(usize, Arc<str>, Child<Tab>)>,
    active: Option<usize>,
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
        })
    }
}
impl Widget for Tabs {
    type Command = TabsCommand;
    type Output = TabAction;
    fn update(&mut self, cx: &mut Update<'_, Self>, command: TabsCommand) {
        match command {
            TabsCommand::Action(a) => {
                let _ = cx.emit(a);
            }
            TabsCommand::Sync(items, active) => {
                self.children.retain(|(id, _, child)| {
                    if items.iter().any(|(key, _)| key == id) {
                        true
                    } else {
                        cx.remove(*child).is_err()
                    }
                });
                let mut next = vec![];
                for (id, title) in items {
                    let state = TabState {
                        title: title.clone(),
                        active: Some(id) == active,
                    };
                    if let Some((_, _, child)) = self.children.iter().find(|(key, _, _)| *key == id)
                    {
                        let _ = cx.send(*child, state);
                        next.push((id, title, *child));
                    } else if let Ok(child) =
                        cx.insert(Tab::new(id, state), |a| TabsCommand::Action(a.clone()))
                    {
                        next.push((id, title, child));
                    }
                }
                self.children = next;
                self.active = active;
                cx.relayout();
            }
        }
    }
    fn layout(&mut self, cx: &mut Layout<'_>, c: Constraints) -> Metrics {
        let widths: Vec<_> = self
            .children
            .iter()
            .map(|(_, t, _)| (t.chars().count() as f32 * 8. + 55.).clamp(125., 220.))
            .collect();
        let right = self
            .children
            .iter()
            .position(|(id, _, _)| Some(*id) == self.active)
            .map(|i| widths[..=i].iter().sum::<f32>() + i as f32 * 6.)
            .unwrap_or(0.);
        let mut x = -(right - c.max.width).max(0.);
        for ((_, _, child), w) in self.children.iter().zip(widths) {
            place(cx, *child, Rect::new(x, 0., w, c.max.height));
            x += w + 6.;
        }
        Metrics::new(c.max)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Choice {
    Note(usize),
    New,
    Save,
    Rename,
    Wrap,
    Close,
    Open,
}
impl Data for Choice {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickerMode {
    Notes,
    Commands,
    File,
}
type RowFactory = Box<dyn Fn(&Choice) -> Element<Label>>;
type NoteList = VirtualList<Choice, Label, RowFactory>;
#[derive(Clone, Debug)]
pub enum PickerOutput {
    Selected(Choice),
    Open(String),
    Close,
}
impl Data for PickerOutput {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Open(s) => s.capacity(),
                _ => 0,
            }
    }
}
pub enum PickerCommand {
    Show(Vec<(Choice, Arc<str>)>, PickerMode),
    Query(EditorOutput),
    List(ListOutput<Choice, std::convert::Infallible>),
}
impl Data for PickerCommand {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Show(v, _) => v.iter().map(|(_, t)| t.len() + 24).sum(),
                Self::Query(o) => o.bytes(),
                Self::List(o) => o.bytes(),
            }
    }
}
pub struct Picker {
    search: Child<Editor>,
    list: Child<NoteList>,
    heading: Child<Label>,
    help: Child<Label>,
    items: Vec<(Choice, Arc<str>)>,
    query: String,
    mode: PickerMode,
    panel: Rect,
}
fn list(items: &[(Choice, Arc<str>)]) -> Element<NoteList> {
    let titles: HashMap<_, _> = items.iter().cloned().collect();
    let keys = items.iter().map(|(id, _)| *id).collect();
    let factory: RowFactory = Box::new(move |id| {
        label(
            format!("  {}", titles.get(id).map_or("Untitled", |s| s.as_ref())),
            16.,
            Color::hex(0xe5e9df),
        )
    });
    Element::leaf(VirtualList::new(keys, 42., factory))
}
impl Picker {
    pub fn new() -> Element<Self> {
        Element::build(|c| Self {
            search: c.connect(
                Element::leaf(Editor::field("").caret_blink(false).placeholder("Search…")),
                |o| PickerCommand::Query(o.clone()),
            ),
            list: c.connect(list(&[]), |o| PickerCommand::List(o.clone())),
            heading: c.add(label("Find a note", 24., Color::hex(0xf0eee4))),
            help: c.add(label(
                "Enter to open  ·  Escape to return",
                12.,
                Color::hex(0x9aa893),
            )),
            items: vec![],
            query: String::new(),
            mode: PickerMode::Notes,
            panel: Rect::default(),
        })
    }
    fn filtered(&self) -> Vec<Choice> {
        let query = self.query.to_lowercase();
        self.items
            .iter()
            .filter(|(_, title)| title.to_lowercase().contains(&query))
            .map(|(id, _)| *id)
            .collect()
    }
}
impl Widget for Picker {
    type Command = PickerCommand;
    type Output = PickerOutput;
    fn update(&mut self, cx: &mut Update<'_, Self>, command: PickerCommand) {
        match command {
            PickerCommand::Show(items, file) => {
                self.items = items;
                self.mode = file;
                self.query.clear();
                let _ = cx.send(self.search, Edit::Set(String::new()));
                let _ = cx.send(
                    self.heading,
                    match file {
                        PickerMode::File => "Open a file",
                        PickerMode::Notes => "Find a note",
                        PickerMode::Commands => "Commands",
                    }
                    .into(),
                );
                let _ = cx.send(
                    self.help,
                    if file == PickerMode::File {
                        "Enter a path to a Markdown or text file"
                    } else {
                        "Enter to open  ·  Escape to return"
                    }
                    .into(),
                );
                if cx.remove(self.list).is_ok() {
                    if let Ok(child) =
                        cx.insert(list(&self.items), |o| PickerCommand::List(o.clone()))
                    {
                        self.list = child;
                    }
                }
                // Visibility of the newly inserted list is applied in the next layout/mount turn.
                cx.request_frame();
                cx.relayout();
                let _ = cx.focus_child(self.search);
            }
            PickerCommand::Query(EditorOutput::Changed { text, .. }) => {
                self.query = text.to_string();
                if self.mode != PickerMode::File {
                    let _ = cx.send(self.list, ListCommand::Keys(self.filtered()));
                }
            }
            PickerCommand::Query(EditorOutput::Submitted) => {
                if self.mode == PickerMode::File {
                    let _ = cx.emit(PickerOutput::Open(self.query.clone()));
                } else if let Some(id) = self.filtered().first() {
                    let _ = cx.emit(PickerOutput::Selected(*id));
                }
            }
            PickerCommand::List(ListOutput::Selected(id)) => {
                let _ = cx.emit(PickerOutput::Selected(id));
            }
            _ => {}
        }
    }
    fn frame(&mut self, cx: &mut Update<'_, Self>, _: FrameTime) {
        let _ = cx.show(self.list, self.mode != PickerMode::File);
    }
    fn input(&mut self, cx: &mut Update<'_, Self>, phase: Phase, input: &Input) {
        match input {
            Input::Key {
                key: Key::Escape,
                down: true,
                ..
            } if phase == Phase::Preview => {
                let _ = cx.emit(PickerOutput::Close);
                cx.stop();
            }
            Input::Key {
                key: Key::Down,
                down: true,
                ..
            } if phase == Phase::Preview && self.mode != PickerMode::File => {
                let _ = cx.focus_child(self.list);
            }
            Input::Button {
                button: 1,
                down: true,
                position,
                ..
            } if phase == Phase::Target && !self.panel.contains(*position) => {
                let _ = cx.emit(PickerOutput::Close);
                cx.stop();
            }
            _ => {}
        }
    }
    fn layout(&mut self, cx: &mut Layout<'_>, c: Constraints) -> Metrics {
        let w = (c.max.width - 40.).clamp(0., 600.);
        let h = if self.mode == PickerMode::File {
            225.
        } else {
            (c.max.height - 80.).clamp(225., 510.)
        };
        let x = (c.max.width - w) / 2.;
        let y = ((c.max.height - h) / 2.).max(20.);
        self.panel = Rect::new(x, y, w, h);
        place(cx, self.heading, Rect::new(x + 26., y + 24., w - 52., 34.));
        place(cx, self.search, Rect::new(x + 26., y + 80., w - 52., 46.));
        place(
            cx,
            self.list,
            Rect::new(x + 26., y + 146., w - 52., (h - 202.).max(0.)),
        );
        place(cx, self.help, Rect::new(x + 26., y + h - 36., w - 52., 22.));
        Metrics::new(c.max)
    }
    fn paint(&self, cx: &mut Paint<'_>) {
        cx.painter
            .rect(cx.bounds, 0., Color::hex(0x050905).alpha(0.78).into());
        cx.painter
            .rect(self.panel, 14., Color::hex(0x242b24).into());
        cx.painter
            .stroke(self.panel.inset(0.5), 14., 1., Color::hex(0x54634f));
    }
    fn semantics(&self) -> Semantics {
        Semantics {
            role: Role::Dialog,
            label: if self.mode == PickerMode::File {
                "Open a file"
            } else {
                "Find a note"
            }
            .into(),
            ..Semantics::default()
        }
    }
}
