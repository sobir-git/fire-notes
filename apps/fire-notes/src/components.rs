use fire_ui::*;
use fire_ui_widgets::*;
use std::{collections::HashMap, rc::Rc, sync::Arc};

pub fn label(text: impl Into<Arc<str>>, size: f32, color: Color) -> Element<Label> {
    Element::leaf(Label::new(text).appearance(Appearance {
        font_size: Some(size),
        foreground: Some(color),
    }))
}
pub fn place<W: Widget>(cx: &mut Layout<'_>, child: Child<W>, rect: Rect) {
    cx.measure(child, Constraints::tight(rect.size()));
    cx.place(child, Point::new(rect.x, rect.y));
}
pub fn paper() -> Theme {
    Theme {
        background: Color::hex(0),
        foreground: Color(1., 0.9, 0.8, 1.),
        accent: Color(1., 0.8, 0., 1.),
        selection: Color(0.39, 0.55, 0.82, 0.35),
        radius: 0.,
        font_size: 16.,
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
    Palette(Rect),
    State(EditorState),
}
impl Data for PageOutput {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Title(s) | Self::Body(s) => s.len(),
                Self::Close | Self::LimitReached | Self::Palette(_) | Self::State(_) => 0,
            }
    }
}
#[derive(Clone, Debug)]
pub enum PageCommand {
    Body(EditorOutput),
    Focus,
    Palette,
    Wrap(bool),
    Close,
}
impl Data for PageCommand {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Body(o) => o.bytes(),
                _ => 0,
            }
    }
}
pub struct Page {
    body: Child<Editor>,
}
impl Page {
    pub fn new(_title: Arc<str>, body: Arc<str>, state: EditorState) -> Element<Self> {
        Element::build(|c| Self {
            body: c.connect(
                Element::leaf(
                    Editor::new(body.to_string())
                        .restore(state)
                        .padding(16., 8.)
                        .decoration(crate::flames::Flames::default())
                        .chrome(false)
                        .caret_blink(false)
                        .max_bytes(crate::storage::MAX_BODY_BYTES),
                ),
                |o| PageCommand::Body(o.clone()),
            ),
        })
    }
}
impl Widget for Page {
    type Command = PageCommand;
    type Output = PageOutput;
    fn lifecycle(&mut self, cx: &mut Update<'_, Self>, event: Lifecycle) {
        if event == Lifecycle::Mount {
            let _ = cx.set_environment(self.body, Rc::new(paper()), true);
        }
    }
    fn update(&mut self, cx: &mut Update<'_, Self>, command: PageCommand) {
        match command {
            PageCommand::Body(EditorOutput::StateChanged(state)) => {
                let _ = cx.emit(PageOutput::State(state));
            }
            PageCommand::Body(EditorOutput::Changed { text, .. }) => {
                let _ = cx.emit(PageOutput::Body(text));
            }
            PageCommand::Body(EditorOutput::LimitReached) => {
                let _ = cx.emit(PageOutput::LimitReached);
            }
            PageCommand::Palette => {
                let _ = cx.send(self.body, Edit::ReportCursor);
            }
            PageCommand::Body(EditorOutput::Cursor(rect)) => {
                let _ = cx.emit(PageOutput::Palette(rect));
            }
            PageCommand::Focus => {
                let _ = cx.focus_child(self.body);
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
        place(cx, self.body, Rect::from_size(c.max));
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
#[derive(Clone)]
pub struct PickerItem {
    pub key: Choice,
    pub title: Arc<str>,
    pub open: bool,
}
pub enum PickerCommand {
    Show(Vec<PickerItem>, PickerMode),
    Position(Rect),
    Query(EditorOutput),
    List(ListOutput<Choice, std::convert::Infallible>),
}
impl Data for PickerCommand {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Show(v, _) => v.iter().map(|i| i.title.len() + 32).sum(),
                Self::Query(o) => o.bytes(),
                Self::List(o) => o.bytes(),
                _ => 0,
            }
    }
}
#[derive(Clone, Debug)]
pub enum PickerOutput {
    Selected(Choice),
    Open(String),
    Close,
}
impl Data for PickerOutput {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + if let Self::Open(s) = self {
                s.capacity()
            } else {
                0
            }
    }
}
struct PickerRow {
    title: Arc<str>,
    paragraph: Option<Arc<Paragraph>>,
    open: bool,
}
impl PickerRow {
    fn new(item: &PickerItem) -> Element<Self> {
        Element::build(|_| Self {
            title: item.title.clone(),
            paragraph: None,
            open: item.open,
        })
    }
}
impl Widget for PickerRow {
    type Command = std::convert::Infallible;
    type Output = std::convert::Infallible;
    fn layout(&mut self, cx: &mut Layout<'_>, c: Constraints) -> Metrics {
        if self.paragraph.is_none() {
            self.paragraph = Some(cx.paragraph(TextRequest {
                text: self.title.clone(),
                style: TextStyle { size: 14., font: 0 },
                width: None,
                revision: 0,
            }));
        }
        Metrics::new(c.max)
    }
    fn paint(&self, cx: &mut Paint<'_>) {
        if let Some(p) = &self.paragraph {
            let selected = cx.environment::<ListRowState>().is_some_and(|s| s.selected);
            cx.painter.paragraph(
                p,
                Point::new(8., 6.),
                if selected {
                    Color::hex(0xffffff)
                } else {
                    Color(0.78, 0.78, 0.78, 1.)
                }
                .into(),
            );
        }
        if self.open {
            cx.painter.rect(
                Rect::new(cx.bounds.width - 14., 14., 4., 4.),
                2.,
                Color(0.4, 0.7, 1., 1.).into(),
            );
        }
    }
}
type RowFactory = Box<dyn Fn(&Choice) -> Element<PickerRow>>;
type NoteList = VirtualList<Choice, PickerRow, RowFactory>;
fn list(items: &[PickerItem]) -> Element<NoteList> {
    let entries: HashMap<_, _> = items.iter().cloned().map(|i| (i.key, i)).collect();
    let keys = items.iter().map(|i| i.key).collect();
    let factory: RowFactory = Box::new(move |key| PickerRow::new(&entries[key]));
    Element::leaf(VirtualList::new(keys, 32., factory).select_on_hover(true))
}
fn overlay_theme() -> Theme {
    Theme {
        background: Color(0.07, 0.07, 0.09, 1.),
        panel: Color(0.13, 0.13, 0.15, 1.),
        border: Color(0.4, 0.7, 1., 0.55),
        foreground: Color::hex(0xffffff),
        muted: Color(0.4, 0.4, 0.4, 1.),
        accent: Color(0.4, 0.7, 1., 1.),
        selection: Color(0.4, 0.7, 1., 0.12),
        font_size: 14.,
        inset: 8.,
        radius: 4.,
        ..Theme::default()
    }
}
pub struct Picker {
    search: Child<Editor>,
    list: Child<NoteList>,
    empty: Child<Label>,
    items: Vec<PickerItem>,
    query: String,
    mode: PickerMode,
    panel: Rect,
    anchor: Rect,
}
impl Picker {
    pub fn new() -> Element<Self> {
        Element::build(|c| Self {
            search: c.connect(
                Element::leaf(
                    Editor::field("")
                        .caret_blink(false)
                        .placeholder("Search notes...")
                        .padding(8., 7.),
                ),
                |o| PickerCommand::Query(o.clone()),
            ),
            list: c.connect(list(&[]), |o| PickerCommand::List(o.clone())),
            empty: c.add(label(
                "No matching notes",
                14.,
                Color(0.59, 0.59, 0.59, 0.71),
            )),
            items: vec![],
            query: String::new(),
            mode: PickerMode::Notes,
            panel: Rect::default(),
            anchor: Rect::default(),
        })
    }
    fn filtered(&self) -> Vec<Choice> {
        let q = self.query.to_lowercase();
        self.items
            .iter()
            .filter(|i| i.title.to_lowercase().contains(&q))
            .map(|i| i.key)
            .collect()
    }
    fn select_first(&self, cx: &mut Update<'_, Self>) {
        if let Some(first) = self.filtered().first() {
            let _ = cx.send(self.list, ListCommand::ScrollTo(*first));
        }
    }
}
impl Widget for Picker {
    type Command = PickerCommand;
    type Output = PickerOutput;
    fn lifecycle(&mut self, cx: &mut Update<'_, Self>, e: Lifecycle) {
        if e == Lifecycle::Mount {
            let _ = cx.set_environment(self.search, Rc::new(overlay_theme()), true);
        }
    }
    fn update(&mut self, cx: &mut Update<'_, Self>, c: PickerCommand) {
        match c {
            PickerCommand::Position(r) => self.anchor = r,
            PickerCommand::Show(items, mode) => {
                self.items = items;
                self.mode = mode;
                self.query.clear();
                let _ = cx.send(self.search, Edit::Set(String::new()));
                let _ = cx.send(
                    self.search,
                    Edit::Placeholder(Arc::from(match mode {
                        PickerMode::Notes => "Search notes...",
                        PickerMode::Commands => "Search commands...",
                        PickerMode::File => "Enter a file path...",
                    })),
                );
                if cx.remove(self.list).is_ok() {
                    if let Ok(child) =
                        cx.insert(list(&self.items), |o| PickerCommand::List(o.clone()))
                    {
                        self.list = child;
                    }
                }
                let _ = cx.focus_child(self.search);
                cx.request_frame();
                cx.relayout();
            }
            PickerCommand::Query(EditorOutput::Changed { text, .. }) => {
                self.query = text.to_string();
                if self.mode != PickerMode::File {
                    let keys = self.filtered();
                    let _ = cx.send(self.list, ListCommand::Keys(keys));
                    self.select_first(cx);
                }
                let _ = cx.show(
                    self.empty,
                    self.mode != PickerMode::File && self.filtered().is_empty(),
                );
                cx.relayout();
            }
            PickerCommand::Query(EditorOutput::Submitted) => {
                if self.mode == PickerMode::File {
                    let _ = cx.emit(PickerOutput::Open(self.query.clone()));
                } else {
                    let _ = cx.send(self.list, ListCommand::Activate);
                }
            }
            PickerCommand::List(ListOutput::Selected(key)) => {
                let _ = cx.emit(PickerOutput::Selected(key));
            }
            _ => {}
        }
    }
    fn frame(&mut self, cx: &mut Update<'_, Self>, _: FrameTime) {
        let _ = cx.show(self.list, self.mode != PickerMode::File);
        let _ = cx.show(
            self.empty,
            self.mode != PickerMode::File && self.filtered().is_empty(),
        );
        let _ = cx.set_environment(
            self.list,
            Rc::new(Theme {
                radius: 0.,
                ..overlay_theme()
            }),
            false,
        );
        self.select_first(cx);
    }
    fn input(&mut self, cx: &mut Update<'_, Self>, phase: Phase, input: &Input) {
        match input {
            Input::Key {
                key: Key::Escape,
                down: true,
                ..
            } if matches!(phase, Phase::Preview | Phase::Target) => {
                let _ = cx.emit(PickerOutput::Close);
                cx.stop();
            }
            Input::Key {
                key: Key::Up | Key::Down,
                down: true,
                ..
            } if phase == Phase::Preview && self.mode != PickerMode::File => {
                let delta = if matches!(input, Input::Key { key: Key::Up, .. }) {
                    -1
                } else {
                    1
                };
                let _ = cx.send(self.list, ListCommand::Navigate(delta));
                cx.stop();
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
        let rows = if self.mode == PickerMode::File {
            0
        } else {
            self.filtered().len().clamp(1, 8)
        };
        let commands = self.mode == PickerMode::Commands;
        let w = ((c.max.width * if commands { 0.5 } else { 0.6 }).min(if commands {
            400.
        } else {
            500.
        }) + 16.)
            .min(c.max.width - 16.);
        let h = (36. + rows as f32 * 32. + 32.).min(c.max.height - 32.);
        let (x, y) = if commands {
            let x = (self.anchor.x - 8.).clamp(8., (c.max.width - w - 8.).max(8.));
            let below = self.anchor.y + self.anchor.height - 12.;
            let y = if below + h <= c.max.height - 8. {
                below
            } else {
                self.anchor.y - h - 4.
            };
            (x, y.clamp(8., (c.max.height - h - 8.).max(8.)))
        } else {
            (
                (c.max.width - w) / 2.,
                ((c.max.height - h) / 2. + 30.).min(c.max.height - h - 8.),
            )
        };
        self.panel = Rect::new(x, y, w, h);
        place(cx, self.search, Rect::new(x + 8., y + 8., w - 16., 36.));
        place(
            cx,
            self.list,
            Rect::new(x + 8., y + 44., w - 28., (h - 68.).max(0.)),
        );
        place(cx, self.empty, Rect::new(x + 16., y + 52., w - 32., 21.));
        Metrics::new(c.max)
    }
    fn paint(&self, cx: &mut Paint<'_>) {
        cx.painter
            .rect(self.panel, 8., Color(0.13, 0.13, 0.15, 1.).into());
        cx.painter
            .stroke(self.panel, 8., 2., Color(0.4, 0.7, 1., 0.55));
    }
    fn semantics(&self) -> Semantics {
        Semantics {
            role: Role::Dialog,
            label: match self.mode {
                PickerMode::Notes => "Find a note",
                PickerMode::Commands => "Commands",
                PickerMode::File => "Open a file",
            }
            .into(),
            ..Semantics::default()
        }
    }
}
