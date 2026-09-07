use crate::design::*;
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
            let _ = cx.set_environment(self.body, Rc::new(editor_theme()), true);
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
    Trash,
    ShowTrash,
    Restore(u64),
    New,
    Save,
    Rename,
    Wrap,
    Close,
}
impl Choice {
    pub fn shortcut(self) -> &'static str {
        match self {
            Self::New => "Ctrl N",
            Self::Save => "Ctrl S",
            Self::Rename => "Ctrl R",
            Self::Wrap => "Alt Z",
            Self::Close => "Ctrl W",
            Self::ShowTrash => "Ctrl Shift T",
            _ => "",
        }
    }
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
    Trash,
}
#[derive(Clone)]
pub struct PickerItem {
    pub key: Choice,
    pub title: Arc<str>,
    pub open: bool,
    pub hint: Arc<str>,
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
    hint: Option<Arc<Paragraph>>,
    shortcut: Arc<str>,
    open: bool,
}
impl PickerRow {
    fn new(item: &PickerItem) -> Element<Self> {
        Element::build(|_| Self {
            title: item.title.clone(),
            paragraph: None,
            hint: None,
            shortcut: item.hint.clone(),
            open: item.open,
        })
    }
}
impl Widget for PickerRow {
    type Command = std::convert::Infallible;
    type Output = std::convert::Infallible;
    fn layout(&mut self, cx: &mut Layout<'_>, c: Constraints) -> Metrics {
        for (text, cached) in [
            (self.title.clone(), &mut self.paragraph),
            (self.shortcut.clone(), &mut self.hint),
        ] {
            if cached
                .as_ref()
                .is_none_or(|p| p.service_revision != cx.text_revision())
            {
                *cached = Some(cx.paragraph(TextRequest {
                    text,
                    style: TextStyle {
                        size: CONTROL_TEXT,
                        font: 0,
                    },
                    width: None,
                    revision: 0,
                }));
            }
        }
        Metrics::new(c.max)
    }
    fn paint(&self, cx: &mut Paint<'_>) {
        let hint_width = self.hint.as_ref().map_or(0., |p| p.size.width);
        let trailing = if self.open {
            24.
        } else if hint_width > 0. {
            hint_width + 24.
        } else {
            8.
        };
        if let Some(p) = &self.paragraph {
            cx.painter.save();
            cx.painter.clip(Rect::new(
                8.,
                0.,
                (cx.bounds.width - trailing - 8.).max(0.),
                cx.bounds.height,
            ));
            cx.painter
                .paragraph(p, Point::new(8., (32. - p.line_height) / 2.), TEXT.into());
            cx.painter.restore();
        }
        if let Some(p) = &self.hint {
            cx.painter.paragraph(
                p,
                Point::new(
                    cx.bounds.width - p.size.width - 8.,
                    (32. - p.line_height) / 2.,
                ),
                MUTED.into(),
            );
        }
        if self.open {
            cx.painter.rect(
                Rect::new(cx.bounds.width - 14., 14., 4., 4.),
                2.,
                EMBER.into(),
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
pub struct Picker {
    search: Child<Editor>,
    list: Child<NoteList>,
    empty: Child<Label>,
    retention: Child<Label>,
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
                        .chrome(false)
                        .padding(8., 7.),
                ),
                |o| PickerCommand::Query(o.clone()),
            ),
            list: c.connect(list(&[]), |o| PickerCommand::List(o.clone())),
            empty: c.add(label("No matching notes", 14., MUTED)),
            retention: c.add(label(
                "Restore within 30 days. Older notes are deleted.",
                12.,
                MUTED,
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
            let _ = cx.set_environment(self.search, Rc::new(popup_theme()), true);
        }
    }
    fn update(&mut self, cx: &mut Update<'_, Self>, c: PickerCommand) {
        match c {
            PickerCommand::Position(r) => self.anchor = r,
            PickerCommand::Show(items, mode) => {
                self.items = items;
                self.mode = mode;
                self.query.clear();
                let _ = cx.send(
                    self.empty,
                    match mode {
                        PickerMode::Commands => "No matching commands",
                        PickerMode::Trash => "No notes in Trash",
                        _ => "No matching notes",
                    }
                    .to_string(),
                );
                let _ = cx.send(self.search, Edit::Set(String::new()));
                let _ = cx.send(
                    self.search,
                    Edit::Placeholder(Arc::from(match mode {
                        PickerMode::Notes => "Search notes...",
                        PickerMode::Commands => "Search commands...",
                        PickerMode::File => "Enter a file path...",
                        PickerMode::Trash => "Search Trash to restore...",
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
        let _ = cx.show(self.retention, self.mode == PickerMode::Trash);
        let _ = cx.show(self.list, self.mode != PickerMode::File);
        let _ = cx.show(
            self.empty,
            self.mode != PickerMode::File && self.filtered().is_empty(),
        );
        let _ = cx.set_environment(
            self.list,
            Rc::new(Theme {
                radius: 3.,
                ..popup_theme()
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
        let w = (if commands { 360_f32 } else { 420_f32 }).min((c.max.width - 16.).max(0.));
        let footer = if self.mode == PickerMode::Trash {
            28.
        } else {
            0.
        };
        let h = ((if rows == 0 {
            52.
        } else {
            50. + rows as f32 * 32. + 8.
        }) + footer)
            .min((c.max.height - 16.).max(0.));
        let (x, y) = if commands {
            let x = (self.anchor.x - 8.).clamp(8., (c.max.width - w - 8.).max(8.));
            let below = self.anchor.y + self.anchor.height + 4.;
            let y = if below + h <= c.max.height - 8. {
                below
            } else {
                self.anchor.y - h - 4.
            };
            (x, y.clamp(8., (c.max.height - h - 8.).max(8.)))
        } else {
            ((c.max.width - w) / 2., ((c.max.height - h) / 2.).max(8.))
        };
        self.panel = Rect::new(x, y, w, h);
        place(cx, self.search, Rect::new(x + 8., y + 8., w - 16., 36.));
        place(
            cx,
            self.list,
            Rect::new(x + 8., y + 50., w - 16., (h - 58. - footer).max(0.)),
        );
        place(
            cx,
            self.retention,
            Rect::new(x + 16., y + h - 26., w - 32., 20.),
        );
        place(cx, self.empty, Rect::new(x + 16., y + 56., w - 32., 21.));
        Metrics::new(c.max)
    }
    fn paint(&self, cx: &mut Paint<'_>) {
        cx.painter.rect(self.panel, POPUP_RADIUS, PANEL.into());
        cx.painter
            .stroke(self.panel.inset(0.5), POPUP_RADIUS, 1., BORDER);
        if self.mode != PickerMode::File {
            cx.painter.rect(
                Rect::new(
                    self.panel.x + 8.,
                    self.panel.y + 44.,
                    self.panel.width - 16.,
                    1.,
                ),
                0.,
                BORDER.into(),
            );
        }
    }
    fn semantics(&self) -> Semantics {
        Semantics {
            role: Role::Dialog,
            label: match self.mode {
                PickerMode::Notes => "Find a note",
                PickerMode::Commands => "Commands",
                PickerMode::File => "Open a file",
                PickerMode::Trash => "Trash",
            }
            .into(),
            ..Semantics::default()
        }
    }
}
