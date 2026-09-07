mod components;
mod flames;
mod storage;
mod tabs;
use components::*;
use fire_ui::*;
use fire_ui_native::{run_with, WindowOptions};
use fire_ui_widgets::*;
use std::{collections::BTreeSet, path::PathBuf, sync::Arc};
use storage::{Note, Save, Session, Writer};
use tabs::*;

struct Record {
    note: Note,
    loaded: bool,
    page: Option<Child<Page>>,
    revision: u64,
    saved: u64,
    view: EditorState,
}
struct Notes {
    directory: PathBuf,
    records: Vec<Record>,
    open: Vec<usize>,
    active: Option<usize>,
    new_button: Child<ChromeButton>,
    minimize: Child<ChromeButton>,
    maximize: Child<ChromeButton>,
    close: Child<ChromeButton>,
    tabs: Child<Tabs>,
    footer: Child<Label>,
    empty: Child<Label>,
    picker: Child<Picker>,
    menu: Option<(usize, Child<Menu<Choice>>)>,
    menu_pending: bool,
    anchor: Child<Label>,
    focus_pending: Option<usize>,
    rename_pending: Option<usize>,
    pending_saves: BTreeSet<usize>,
    open_task: TaskSlot,
    closing: bool,
    size: Size,
    position: Option<(i32, i32)>,
}
enum Message {
    Chrome(ChromeAction),
    Tabs(TabAction),
    Page(usize, PageOutput),
    Pick(PickerOutput),
    Menu(MenuOutput<Choice>),
    Loaded(Option<usize>, Result<Note, String>),
    Saved(usize, u64, Result<(), String>),
    FileSelected(Option<usize>, Result<Option<PathBuf>, String>),
}
impl Data for Message {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Page(_, o) => o.bytes(),
                Self::Pick(o) => o.bytes(),
                Self::Loaded(_, Ok(n)) => n.bytes(),
                Self::Loaded(_, Err(e)) | Self::Saved(_, _, Err(e)) => e.capacity(),
                Self::FileSelected(_, Ok(Some(path))) => path.as_os_str().len(),
                Self::FileSelected(_, Err(e)) => e.len(),
                _ => 0,
            }
    }
}
enum Output {
    PickFile(Ticket<Notes>, Option<(usize, Arc<str>)>),
    Save(Save),
    Load {
        ticket: Ticket<Notes>,
        id: Option<usize>,
        path: PathBuf,
    },
}
impl Data for Output {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Save(s) => s.content.len() + s.path.as_os_str().len(),
                Self::Load { path, .. } => path.as_os_str().len(),
                Self::PickFile(_, save) => save.as_ref().map_or(0, |(_, title)| title.len()),
            }
    }
}
impl Notes {
    fn new(directory: PathBuf, notes: Vec<Note>, session: &Session) -> Element<Self> {
        let mut records: Vec<_> = notes
            .into_iter()
            .map(|note| Record {
                view: session
                    .views
                    .get(&note.path)
                    .map(EditorState::from)
                    .unwrap_or(EditorState {
                        wrap: false,
                        ..EditorState::default()
                    }),
                note,
                loaded: false,
                page: None,
                revision: 0,
                saved: 0,
            })
            .collect();
        let mut open = vec![];
        for path in &session.tabs {
            let id = records
                .iter()
                .position(|r| r.note.path == *path)
                .unwrap_or_else(|| {
                    records.push(Record {
                        note: Note {
                            path: path.clone(),
                            title: Arc::from("Note"),
                            body: Arc::from(""),
                        },
                        loaded: false,
                        page: None,
                        revision: 0,
                        saved: 0,
                        view: session.views.get(path).map(EditorState::from).unwrap_or(
                            EditorState {
                                wrap: false,
                                ..EditorState::default()
                            },
                        ),
                    });
                    records.len() - 1
                });
            if let Ok(mut note) = Note::load(path) {
                note.title = session
                    .titles
                    .get(path)
                    .map(|s| Arc::from(s.as_str()))
                    .unwrap_or(records[id].note.title.clone());
                records[id].note = note;
                records[id].loaded = true;
                if !open.contains(&id) {
                    open.push(id);
                }
            }
        }
        if open.is_empty() && !records.is_empty() {
            if let Ok(mut note) = Note::load(&records[0].note.path) {
                note.title = records[0].note.title.clone();
                records[0].note = note;
                records[0].loaded = true;
                open.push(0);
            }
        }
        let active = session
            .active
            .as_ref()
            .and_then(|path| open.iter().find(|id| records[**id].note.path == *path))
            .copied()
            .or(open.first().copied());
        Element::build(|c| {
            for id in &open {
                let id = *id;
                records[id].page = Some(c.connect(
                    Page::new(
                        records[id].note.title.clone(),
                        records[id].note.body.clone(),
                        records[id].view.clone(),
                    ),
                    move |o| Message::Page(id, o.clone()),
                ));
            }
            let tabs = c.connect(
                Tabs::new(
                    open.iter()
                        .map(|id| (*id, records[*id].note.title.clone()))
                        .collect(),
                    active,
                ),
                |a| Message::Tabs(a.clone()),
            );
            Self {
                directory,
                records,
                open,
                active,
                tabs,
                new_button: c.connect(ChromeButton::new(ChromeAction::New), |a| {
                    Message::Chrome(*a)
                }),
                minimize: c.connect(ChromeButton::new(ChromeAction::Minimize), |a| {
                    Message::Chrome(*a)
                }),
                maximize: c.connect(ChromeButton::new(ChromeAction::Maximize), |a| {
                    Message::Chrome(*a)
                }),
                close: c.connect(ChromeButton::new(ChromeAction::Close), |a| {
                    Message::Chrome(*a)
                }),
                footer: c.add(label("Saved locally", 12., Color::hex(0x91a18b))),
                empty: c.add(label(
                    "A little room for your next idea.\n\nCreate a note, or find one with Ctrl P.",
                    20.,
                    Color::hex(0x9eae98),
                )),
                picker: c.connect(Picker::new(), |o| Message::Pick(o.clone())),
                menu: None,
                menu_pending: false,
                anchor: c.add(Element::leaf(Label::new(""))),
                focus_pending: active,
                rename_pending: None,
                pending_saves: BTreeSet::new(),
                open_task: TaskSlot::new(),
                closing: false,
                size: Size::new(session.width, session.height),
                position: session.position,
            }
        })
    }
    fn status(&self, cx: &mut Update<'_, Self>, text: impl Into<String>) {
        let text = text.into();
        let visible = text.starts_with("Could")
            || text.contains("failed")
            || text.contains("busy")
            || text.contains("limit")
            || text.starts_with("Close a tab");
        let _ = cx.show(self.footer, visible);
        let _ = cx.send(self.footer, text);
    }
    fn sync_tabs(&self, cx: &mut Update<'_, Self>) {
        let _ = cx.send(
            self.tabs,
            TabsCommand::Sync(
                self.open
                    .iter()
                    .map(|id| (*id, self.records[*id].note.title.clone()))
                    .collect(),
                self.active,
            ),
        );
    }
    fn save_session(&self, cx: &mut Update<'_, Self>) {
        let session = Session {
            tabs: self
                .open
                .iter()
                .map(|id| self.records[*id].note.path.clone())
                .collect(),
            active: self.active.map(|id| self.records[id].note.path.clone()),
            titles: self
                .records
                .iter()
                .map(|r| (r.note.path.clone(), r.note.title.to_string()))
                .collect(),
            views: self
                .records
                .iter()
                .map(|r| (r.note.path.clone(), storage::NoteView::from(&r.view)))
                .collect(),
            position: self.position,
            width: self.size.width,
            height: self.size.height,
        };
        if let Ok(content) = serde_json::to_string(&session) {
            let _ = cx.emit(Output::Save(Save {
                id: usize::MAX,
                revision: 0,
                path: self.directory.join("session.json"),
                content: content.into(),
            }));
        }
    }
    fn flush(&mut self, cx: &mut Update<'_, Self>) {
        for id in self.pending_saves.clone() {
            let record = &self.records[id];
            let job = Save {
                id,
                revision: record.revision,
                path: record.note.path.clone(),
                content: record.note.body.clone(),
            };
            if cx.emit(Output::Save(job)).is_err() {
                cx.request_frame();
                break;
            }
            self.pending_saves.remove(&id);
        }
    }
    fn display(&mut self, cx: &mut Update<'_, Self>, id: usize) {
        if id >= self.records.len() {
            return;
        }
        if self.records[id].page.is_none() {
            if !self.records[id].loaded {
                self.load(cx, Some(id), self.records[id].note.path.clone());
                return;
            }
            let element = Page::new(
                self.records[id].note.title.clone(),
                self.records[id].note.body.clone(),
                self.records[id].view.clone(),
            );
            match cx.insert(element, move |o| Message::Page(id, o.clone())) {
                Ok(page) => self.records[id].page = Some(page),
                Err(_) => {
                    self.status(cx, "The editor is busy. Try opening this note again.");
                    return;
                }
            }
            self.open.push(id);
        }
        self.active = Some(id);
        for (key, r) in self.records.iter().enumerate() {
            if let Some(page) = r.page {
                let _ = cx.show(page, key == id);
            }
        }
        let _ = cx.show(self.empty, false);
        self.focus_pending = Some(id);
        cx.request_frame();
        self.sync_tabs(cx);
        self.save_session(cx);
        cx.relayout();
    }
    fn load(&mut self, cx: &mut Update<'_, Self>, id: Option<usize>, path: PathBuf) {
        match cx.replace_task(self.open_task) {
            Ok(ticket) => {
                if cx.emit(Output::Load { ticket, id, path }).is_ok() {
                    self.status(cx, "Opening note…");
                } else {
                    self.status(cx, "The editor is busy. Try opening again.");
                }
            }
            Err(_) => self.status(cx, "The editor is busy. Try opening again."),
        }
    }
    fn create(&mut self, cx: &mut Update<'_, Self>) {
        let mut note = storage::new_note(&self.directory);
        note.title = Arc::from(format!("Untitled-{}", self.records.len() + 1));
        let id = self.records.len();
        self.records.push(Record {
            note,
            loaded: true,
            page: None,
            revision: 1,
            saved: 0,
            view: EditorState {
                wrap: false,
                ..EditorState::default()
            },
        });
        self.pending_saves.insert(id);
        self.display(cx, id);
        self.flush(cx);
    }
    fn close_tab(&mut self, cx: &mut Update<'_, Self>, id: usize) {
        if self.open.len() <= 1 {
            return;
        }
        if let Some(page) = self.records.get_mut(id).and_then(|r| r.page.take()) {
            if cx.remove(page).is_err() {
                self.records[id].page = Some(page);
                self.status(cx, "The editor is busy. Try closing this tab again.");
                return;
            }
        }
        let position = self.open.iter().position(|key| *key == id).unwrap_or(0);
        self.open.retain(|key| *key != id);
        if self.records[id].saved == self.records[id].revision {
            self.records[id].note.body = Arc::from("");
            self.records[id].loaded = false;
        }
        if self.active == Some(id) {
            self.active = self
                .open
                .get(position.min(self.open.len().saturating_sub(1)))
                .copied();
        }
        if let Some(active) = self.active {
            self.display(cx, active);
        } else {
            let _ = cx.show(self.empty, true);
            self.sync_tabs(cx);
            self.save_session(cx);
            cx.relayout();
        }
    }
    fn hide_menu(&mut self, cx: &mut Update<'_, Self>) -> Option<usize> {
        let (id, menu) = self.menu.take()?;
        self.menu_pending = false;
        let _ = cx.close_modal();
        let _ = cx.remove(menu);
        Some(id)
    }
    fn show_menu(&mut self, cx: &mut Update<'_, Self>, id: usize, at: Point) {
        self.hide_menu(cx);
        let items = vec![
            MenuItem::new(Choice::Rename, "Rename").hint("Ctrl R"),
            MenuItem::new(Choice::Save, "Save").hint("Ctrl S"),
            MenuItem::new(Choice::Wrap, "Word wrap")
                .hint("Alt Z")
                .checked(self.records[id].view.wrap),
            MenuItem::new(Choice::Close, "Close tab")
                .hint("Ctrl W")
                .enabled(self.open.len() > 1),
        ];
        if let Ok(menu) = cx.insert(Menu::new(items, at), |o| Message::Menu(o.clone())) {
            self.menu = Some((id, menu));
            self.menu_pending = true;
            cx.request_frame();
            cx.relayout();
        }
    }
    fn show_picker(&mut self, cx: &mut Update<'_, Self>, mode: PickerMode) {
        let items = if mode == PickerMode::Commands {
            [
                (Choice::New, "New Tab"),
                (Choice::Save, "Save"),
                (Choice::Wrap, "Toggle Word Wrap"),
                (Choice::Close, "Close Tab"),
            ]
            .into_iter()
            .map(|(key, title)| PickerItem {
                key,
                title: Arc::from(title),
                open: false,
            })
            .collect()
        } else {
            self.records
                .iter()
                .enumerate()
                .map(|(id, r)| PickerItem {
                    key: Choice::Note(id),
                    title: r.note.title.clone(),
                    open: self.open.contains(&id),
                })
                .collect()
        };
        let _ = cx.send(self.picker, PickerCommand::Show(items, mode));
        let _ = cx.show(self.picker, true);
        let _ = cx.open_modal(self.picker);
        cx.repaint();
    }
    fn hide_picker(&self, cx: &mut Update<'_, Self>) {
        let _ = cx.close_modal();
        let _ = cx.show(self.picker, false);
    }
    fn save_as(&self, cx: &mut Update<'_, Self>) {
        if let (Some(id), Ok(ticket)) = (self.active, cx.replace_task(self.open_task)) {
            let _ = cx.emit(Output::PickFile(
                ticket,
                Some((id, self.records[id].note.title.clone())),
            ));
        }
    }
    fn choose(&mut self, cx: &mut Update<'_, Self>, choice: Choice) {
        match choice {
            Choice::Note(id) => self.display(cx, id),
            Choice::New => self.create(cx),
            Choice::Save => {
                if let Some(id) = self.active {
                    self.pending_saves.insert(id);
                    self.flush(cx);
                }
            }
            Choice::Rename => {
                if let Some(id) = self.active {
                    let _ = cx.send(self.tabs, TabsCommand::Rename(id));
                }
            }
            Choice::Wrap => {
                if let Some(id) = self.active {
                    let state = &mut self.records[id];
                    state.view.wrap = !state.view.wrap;
                    if let Some(page) = state.page {
                        let _ = cx.send(page, PageCommand::Wrap(state.view.wrap));
                    }
                    self.save_session(cx);
                }
            }
            Choice::Close => {
                if let Some(page) = self.active.and_then(|id| self.records[id].page) {
                    let _ = cx.send(page, PageCommand::Close);
                }
            }
        }
    }
}
impl Widget for Notes {
    type Command = Message;
    type Output = Output;
    fn lifecycle(&mut self, cx: &mut Update<'_, Self>, event: Lifecycle) {
        match event {
            Lifecycle::Mount => {
                let _ = cx.show(self.picker, false);
                let _ = cx.show(self.footer, false);
                let _ = cx.anchor(self.picker, Some(self.anchor));
                let _ = cx.show(self.empty, self.active.is_none());
                for (id, r) in self.records.iter().enumerate() {
                    if let Some(page) = r.page {
                        let _ = cx.show(page, Some(id) == self.active);
                    }
                }
                cx.request_frame();
            }
            Lifecycle::Moved { x, y } => {
                self.position = Some((x, y));
                self.save_session(cx);
            }
            Lifecycle::Resized => {
                self.size = cx.bounds().size();
                self.save_session(cx);
            }
            _ => {}
        }
    }
    fn update(&mut self, cx: &mut Update<'_, Self>, message: Message) {
        match message {
            Message::Tabs(TabAction::Context(id, at)) => self.show_menu(cx, id, at),
            Message::Menu(MenuOutput::Dismissed) => {
                self.hide_menu(cx);
            }
            Message::Menu(MenuOutput::Selected(choice)) => {
                if let Some(id) = self.hide_menu(cx) {
                    match choice {
                        Choice::Rename => {
                            self.display(cx, id);
                            self.rename_pending = Some(id);
                        }
                        Choice::Close => self.close_tab(cx, id),
                        Choice::Save => {
                            self.pending_saves.insert(id);
                            self.flush(cx);
                        }
                        Choice::Wrap => {
                            let r = &mut self.records[id];
                            r.view.wrap = !r.view.wrap;
                            if let Some(page) = r.page {
                                let _ = cx.send(page, PageCommand::Wrap(r.view.wrap));
                            }
                            self.save_session(cx);
                        }
                        _ => {}
                    }
                }
            }
            Message::FileSelected(None, Ok(Some(path))) => self.load(cx, None, path),
            Message::FileSelected(Some(id), Ok(Some(path))) => {
                if self
                    .records
                    .iter()
                    .enumerate()
                    .any(|(other, r)| other != id && r.note.path == path)
                {
                    self.status(cx, "Could not save: this file belongs to another note");
                    return;
                }
                let r = &mut self.records[id];
                r.note.path = path;
                r.note.title = Arc::from(
                    r.note
                        .path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .as_ref(),
                );
                r.revision += 1;
                self.pending_saves.insert(id);
                self.sync_tabs(cx);
                self.save_session(cx);
                self.flush(cx);
            }
            Message::FileSelected(_, Ok(None)) => {}
            Message::FileSelected(_, Err(e)) => {
                self.status(cx, format!("Could not open file dialog: {e}"))
            }
            Message::Chrome(action) => match action {
                ChromeAction::New => self.create(cx),
                ChromeAction::Close => {
                    let _ = cx.close_window();
                }
                ChromeAction::Minimize => {
                    let _ = cx.window(WindowAction::Minimize);
                }
                ChromeAction::Maximize => {
                    let _ = cx.window(WindowAction::ToggleMaximized);
                }
            },
            Message::Tabs(TabAction::Window(action)) => {
                let _ = cx.window(action);
            }
            Message::Tabs(TabAction::BeginRename(id)) => {
                self.display(cx, id);
                self.rename_pending = Some(id);
            }
            Message::Tabs(TabAction::Rename(id, title)) => {
                self.update(cx, Message::Page(id, PageOutput::Title(title)))
            }
            Message::Tabs(TabAction::RenameDone(id)) => self.display(cx, id),
            Message::Tabs(TabAction::Reorder(id, other)) => {
                if let (Some(a), Some(b)) = (
                    self.open.iter().position(|x| *x == id),
                    self.open.iter().position(|x| *x == other),
                ) {
                    self.open.swap(a, b);
                    self.sync_tabs(cx);
                    self.save_session(cx);
                    cx.relayout();
                }
            }
            Message::Tabs(TabAction::Select(id)) => self.display(cx, id),
            Message::Page(id, PageOutput::State(state)) => {
                self.records[id].view = state;
            }
            Message::Page(_, PageOutput::Palette(rect)) => {
                let _ = cx.send(
                    self.picker,
                    PickerCommand::Position(Rect::new(
                        rect.x,
                        rect.y + 40.,
                        rect.width,
                        rect.height,
                    )),
                );
                self.show_picker(cx, PickerMode::Commands);
            }
            Message::Page(id, PageOutput::Close) => self.close_tab(cx, id),
            Message::Page(_, PageOutput::LimitReached) => {
                self.status(
                    cx,
                    "Note size limit reached. Start another note to keep writing.",
                );
            }
            Message::Page(id, change) => {
                let r = &mut self.records[id];
                match change {
                    PageOutput::Title(title) => {
                        r.note.title = if title.trim().is_empty() {
                            Arc::from("Untitled")
                        } else {
                            title
                        }
                    }
                    PageOutput::Body(body) => r.note.body = body,
                    PageOutput::Close
                    | PageOutput::LimitReached
                    | PageOutput::Palette(_)
                    | PageOutput::State(_) => {
                        unreachable!()
                    }
                };
                r.revision += 1;
                self.pending_saves.insert(id);
                self.sync_tabs(cx);
                self.save_session(cx);
                self.status(cx, "Saving…");
                self.flush(cx);
            }
            Message::Pick(PickerOutput::Close) => self.hide_picker(cx),
            Message::Pick(PickerOutput::Selected(choice)) => {
                self.hide_picker(cx);
                self.choose(cx, choice);
            }
            Message::Pick(PickerOutput::Open(path)) => {
                self.hide_picker(cx);
                let path = PathBuf::from(path.trim());
                if !path.as_os_str().is_empty() {
                    self.load(cx, None, path);
                }
            }
            Message::Loaded(id, Ok(note)) => {
                let id = id
                    .or_else(|| self.records.iter().position(|r| r.note.path == note.path))
                    .unwrap_or(self.records.len());
                if id == self.records.len() {
                    self.records.push(Record {
                        note,
                        loaded: true,
                        page: None,
                        revision: 0,
                        saved: 0,
                        view: EditorState {
                            wrap: false,
                            ..EditorState::default()
                        },
                    });
                } else if self.records[id].revision == self.records[id].saved {
                    let title = self.records[id].note.title.clone();
                    self.records[id].note = note;
                    self.records[id].note.title = title;
                    self.records[id].loaded = true;
                }
                self.display(cx, id);
                self.status(cx, "Saved locally");
            }
            Message::Loaded(_, Err(error)) => {
                self.status(cx, format!("Could not open note: {error}"))
            }
            Message::Saved(id, revision, result) => {
                if id == usize::MAX {
                    if let Err(e) = result {
                        self.status(cx, format!("Could not remember tabs: {e}"));
                    }
                    return;
                }
                match result {
                    Ok(()) => {
                        let r = &mut self.records[id];
                        if r.revision == revision {
                            r.saved = revision;
                            if r.page.is_none() {
                                r.note.body = Arc::from("");
                                r.loaded = false;
                            }
                            if self.active == Some(id) {
                                self.status(cx, "Saved locally");
                            }
                        }
                        if self.closing && self.records.iter().all(|r| r.revision == r.saved) {
                            let _ = cx.close_window();
                        }
                    }
                    Err(error) => {
                        self.closing = false;
                        self.status(cx, format!("Save failed: {error}. Ctrl S to retry."));
                    }
                }
            }
        }
    }
    fn frame(&mut self, cx: &mut Update<'_, Self>, _: FrameTime) {
        // Insertions commit after their callback; configure the mounted overlay here.
        if std::mem::take(&mut self.menu_pending) {
            if let Some((_, menu)) = self.menu {
                let _ = cx.set_environment(
                    menu,
                    std::rc::Rc::new(Theme {
                        panel: Color::hex(0x211719),
                        raised: Color::hex(0x402522),
                        border: Color::hex(0x6b3c2c),
                        muted: Color::hex(0xa78070),
                        font_size: 14.,
                        radius: 6.,
                        ..paper()
                    }),
                    true,
                );
                let _ = cx.anchor(menu, Some(self.anchor));
                let _ = cx.open_modal(menu);
            }
        }
        if let Some(id) = self.focus_pending.take() {
            if let Some(page) = self.records[id].page {
                if self.rename_pending.take() == Some(id) {
                    let _ = cx.send(self.tabs, TabsCommand::Rename(id));
                } else {
                    let _ = cx.send(page, PageCommand::Focus);
                }
            }
        }
        self.flush(cx);
    }
    fn close_requested(&mut self, cx: &mut Update<'_, Self>) -> bool {
        self.save_session(cx);
        let dirty: Vec<_> = self
            .records
            .iter()
            .enumerate()
            .filter(|(_, r)| r.revision != r.saved)
            .map(|(id, _)| id)
            .collect();
        if dirty.is_empty() {
            return true;
        }
        self.closing = true;
        self.pending_saves.extend(dirty);
        self.flush(cx);
        self.status(cx, "Saving before closing…");
        false
    }
    fn input(&mut self, cx: &mut Update<'_, Self>, phase: Phase, input: &Input) {
        if phase == Phase::Target {
            if let Input::FileDropped(path) = input {
                self.hide_menu(cx);
                self.hide_picker(cx);
                self.load(cx, None, path.clone());
                cx.stop();
                return;
            }
        }
        if phase == Phase::Bubble
            && matches!(
                input,
                Input::Key {
                    key: Key::Escape,
                    down: true,
                    ..
                }
            )
        {
            let _ = cx.close_window();
            cx.stop();
            return;
        }
        if !matches!(phase, Phase::Preview | Phase::Target) {
            return;
        }
        if let Input::Button {
            button: 1,
            down: true,
            position,
            ..
        } = input
        {
            let b = cx.bounds();
            let left = position.x < 4.;
            let right = position.x > b.width - 4.;
            let top = position.y < 4.;
            let bottom = position.y > b.height - 4.;
            let edge = match (left, right, top, bottom) {
                (true, _, true, _) => Some(ResizeEdge::NorthWest),
                (_, true, true, _) => Some(ResizeEdge::NorthEast),
                (true, _, _, true) => Some(ResizeEdge::SouthWest),
                (_, true, _, true) => Some(ResizeEdge::SouthEast),
                (true, _, _, _) => Some(ResizeEdge::West),
                (_, true, _, _) => Some(ResizeEdge::East),
                (_, _, true, _) => Some(ResizeEdge::North),
                (_, _, _, true) => Some(ResizeEdge::South),
                _ => None,
            };
            if let Some(edge) = edge {
                let _ = cx.window(WindowAction::Resize(edge));
                cx.stop();
                return;
            }
            if phase == Phase::Target && position.y < 40. {
                let _ = cx.window(WindowAction::Drag);
                cx.stop();
                return;
            }
        }

        if let Input::Key {
            key,
            down: true,
            repeat: false,
            modifiers,
            ..
        } = input
        {
            if modifiers.command() {
                match key {
                    Key::Character('n') => self.create(cx),
                    Key::Character('r') => self.choose(cx, Choice::Rename),
                    Key::Character(c @ '1'..='9') => {
                        if let Some(id) = self.open.get((*c as u8 - b'1') as usize).copied() {
                            self.display(cx, id);
                        }
                    }
                    Key::Character('p') => self.show_picker(cx, PickerMode::Notes),
                    Key::Character('o') => {
                        if modifiers.shift {
                            self.show_picker(cx, PickerMode::File);
                        } else if let Ok(ticket) = cx.replace_task(self.open_task) {
                            let _ = cx.emit(Output::PickFile(ticket, None));
                        }
                    }
                    Key::Character('/') => {
                        if let Some(page) = self.active.and_then(|id| self.records[id].page) {
                            let _ = cx.send(page, PageCommand::Palette);
                        }
                    }
                    Key::Character('s') if modifiers.shift => self.save_as(cx),
                    Key::Character('s') => self.choose(cx, Choice::Save),
                    Key::Character('w') => self.choose(cx, Choice::Close),
                    Key::Character('q') => {
                        let _ = cx.close_window();
                    }
                    Key::Tab => {
                        if !self.open.is_empty() {
                            let i = self
                                .active
                                .and_then(|id| self.open.iter().position(|key| *key == id))
                                .unwrap_or(0);
                            let next = if modifiers.shift {
                                (i + self.open.len() - 1) % self.open.len()
                            } else {
                                (i + 1) % self.open.len()
                            };
                            self.display(cx, self.open[next]);
                        }
                    }
                    _ => return,
                }
            } else if modifiers.alt && *key == Key::Character('z') {
                self.choose(cx, Choice::Wrap);
            } else {
                return;
            }
            cx.stop();
        }
    }
    fn layout(&mut self, cx: &mut Layout<'_>, c: Constraints) -> Metrics {
        let w = c.max.width;
        let h = c.max.height;
        let available = (w - 172.).max(0.);
        let tabs_width = self
            .open
            .iter()
            .map(|id| tab_width(&self.records[*id].note.title) + 1.)
            .sum::<f32>();
        place(cx, self.tabs, Rect::new(0., 0., available, 40.));
        place(
            cx,
            self.new_button,
            Rect::new((tabs_width + 8.).min(available + 8.), 6., 28., 28.),
        );
        place(cx, self.minimize, Rect::new(w - 100., 6., 28., 28.));
        place(cx, self.maximize, Rect::new(w - 68., 6., 28., 28.));
        place(cx, self.close, Rect::new(w - 36., 6., 28., 28.));
        for r in &self.records {
            if let Some(page) = r.page {
                place(cx, page, Rect::new(0., 40., w, (h - 40.).max(0.)));
            }
        }
        place(
            cx,
            self.empty,
            Rect::new(16., 56., (w - 32.).max(0.), (h - 56.).max(0.)),
        );
        place(
            cx,
            self.footer,
            Rect::new(16., (h - 26.).max(40.), (w - 32.).max(0.), 22.),
        );
        place(cx, self.anchor, Rect::default());
        place(cx, self.picker, Rect::from_size(c.max));
        if let Some((_, menu)) = self.menu {
            place(cx, menu, Rect::from_size(c.max));
        }
        Metrics::new(c.max)
    }
    fn paint(&self, cx: &mut Paint<'_>) {
        cx.painter.rect(cx.bounds, 0., Color::hex(0).into());
        cx.painter.rect(
            Rect::new(0., 0., cx.bounds.width, 40.),
            0.,
            Color(0.05, 0.02, 0.02, 1.).into(),
        );
        cx.painter.rect(
            Rect::new(0., 40., cx.bounds.width, 1.),
            0.,
            Color(0.2, 0.05, 0.05, 1.).into(),
        );
    }
}
fn main() {
    if let Err(error) = start() {
        eprintln!("Fire Notes: {error}");
        std::process::exit(1);
    }
}
fn start() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    let mut directory = std::env::var_os("FIRE_NOTES_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("tmp/fire-notes"));
    while let Some(arg) = args.next() {
        if arg == "--data-dir" {
            directory = PathBuf::from(args.next().ok_or("--data-dir requires a path")?);
        } else if arg == "--help" {
            println!("fire-notes [--data-dir PATH]\nCtrl N new · Ctrl P find · Ctrl O open · Ctrl / commands · Ctrl S save · Ctrl W close · Ctrl Tab switch · Alt Z wrap · Ctrl Q quit");
            return Ok(());
        } else {
            return Err(format!("Unknown argument: {}", arg.to_string_lossy()));
        }
    }
    let mut library = storage::scan(&directory)?;
    directory = std::fs::canonicalize(&directory).map_err(|e| e.to_string())?;
    if library.is_empty() {
        let note = Note {
            path: directory.join("note-1.md"),
            title: Arc::from("Untitled-1"),
            body: Arc::from(""),
        };
        storage::atomic_write(&note.path, note.body.as_bytes())?;
        library.push(note);
    }
    for note in &mut library {
        note.path = std::fs::canonicalize(&note.path).map_err(|e| e.to_string())?;
    }
    let session = storage::session(&directory);
    let size = Size::new(
        session.width.clamp(420., 2000.),
        session.height.clamp(360., 1600.),
    );
    for note in &mut library {
        if let Some(title) = session.titles.get(&note.path) {
            note.title = Arc::from(title.as_str());
        }
    }
    let root = Notes::new(directory, library, &session);
    let mut writer: Option<Writer> = None;
    run_with(
        root,
        WindowOptions {
            title: "Fire Notes".into(),
            decorations: false,
            position: session.position,
            font: Some(PathBuf::from(
                "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            )),
            size,
            background: paper().background,
            limits: Limits {
                message_bytes: 32 * 1024 * 1024,
                ..Limits::default()
            },
        },
        move |output, wake| match output {
            Output::PickFile(ticket, save) => {
                let wake = wake.clone();
                std::thread::spawn(move || {
                    let result = if let Some((_, title)) = &save {
                        fire_ui_native::save_file(title, &[("Markdown", &["md"])])
                    } else {
                        fire_ui_native::open_file(&[("Markdown", &["md", "markdown", "txt"])])
                    };
                    let mut message = Message::FileSelected(save.map(|(id, _)| id), result);
                    loop {
                        match wake.complete(ticket, message) {
                            Ok(()) => break,
                            Err((Error::Full, returned)) => {
                                message = returned;
                                std::thread::sleep(std::time::Duration::from_millis(2));
                            }
                            Err(_) => break,
                        }
                    }
                });
            }
            Output::Save(save) => {
                let writer = writer.get_or_insert_with(|| {
                    let wake = wake.clone();
                    Writer::new(move |id, revision, result| {
                        let mut message = Message::Saved(id, revision, result);
                        loop {
                            match wake.post(message) {
                                Ok(()) => break,
                                Err((Error::Full, returned)) => {
                                    message = returned;
                                    std::thread::sleep(std::time::Duration::from_millis(2));
                                }
                                Err(_) => break,
                            }
                        }
                    })
                });
                writer.submit(save);
            }
            Output::Load { ticket, id, path } => {
                let wake = wake.clone();
                std::thread::spawn(move || {
                    let result = std::fs::canonicalize(&path)
                        .map_err(|e| e.to_string())
                        .and_then(|p| Note::load(&p));
                    let mut message = Message::Loaded(id, result);
                    loop {
                        match wake.complete(ticket, message) {
                            Ok(()) => break,
                            Err((Error::Full, returned)) => {
                                message = returned;
                                std::thread::sleep(std::time::Duration::from_millis(2));
                            }
                            Err(_) => break,
                        }
                    }
                });
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> Ui<Notes> {
        Ui::new(
            Notes::new(
                PathBuf::from("/unused-test-directory"),
                vec![],
                &Session::default(),
            ),
            Size::new(1100., 820.),
            Limits::default(),
        )
        .unwrap()
    }
    fn settle(ui: &mut Ui<Notes>) -> Vec<Output> {
        let mut outputs = vec![];
        for _ in 0..10 {
            ui.frame(std::time::Duration::ZERO, 100);
            ui.pump(1000, |o| outputs.push(o), |_| {});
            ui.layout(&mut TestText);
        }
        outputs
    }
    #[test]
    fn failed_save_keeps_window_open_until_latest_revision_is_acknowledged() {
        let mut ui = app();
        settle(&mut ui);
        assert!(ui.send(Message::Chrome(ChromeAction::New)).is_ok());
        settle(&mut ui);
        assert!(ui
            .send(Message::Page(
                0,
                PageOutput::Body(Arc::from("last keystroke"))
            ))
            .is_ok());
        let outputs = settle(&mut ui);
        let revision = ui.root().records[0].revision;
        assert!(outputs.iter().any(
            |o| matches!(o, Output::Save(s) if s.id == 0 && s.content.contains("last keystroke"))
        ));
        assert!(!ui.request_close());
        assert!(ui
            .send(Message::Saved(0, revision, Err("disk full".into())))
            .is_ok());
        settle(&mut ui);
        assert!(!ui.root().closing);
        assert!(!ui.request_close());
        assert!(ui.send(Message::Saved(0, revision - 1, Ok(()))).is_ok());
        settle(&mut ui);
        assert!(!ui.request_close());
        assert!(ui.send(Message::Saved(0, revision, Ok(()))).is_ok());
        settle(&mut ui);
        assert!(ui.request_close());
    }
    #[test]
    fn closing_and_reopening_before_save_completion_keeps_latest_body() {
        let mut ui = app();
        settle(&mut ui);
        assert!(ui.send(Message::Chrome(ChromeAction::New)).is_ok());
        settle(&mut ui);
        assert!(ui
            .send(Message::Page(
                0,
                PageOutput::Body(Arc::from("unsaved idea"))
            ))
            .is_ok());
        settle(&mut ui);
        assert!(ui.send(Message::Chrome(ChromeAction::New)).is_ok());
        settle(&mut ui);
        assert!(ui.send(Message::Page(0, PageOutput::Close)).is_ok());
        settle(&mut ui);
        assert!(ui.root().records[0].page.is_none());
        assert_eq!(&*ui.root().records[0].note.body, "unsaved idea");
        assert!(ui.send(Message::Tabs(TabAction::Select(0))).is_ok());
        let outputs = settle(&mut ui);
        assert!(!outputs.iter().any(|o| matches!(o, Output::Load { .. })));
        assert!(ui.root().records[0].page.is_some());
        assert_eq!(&*ui.root().records[0].note.body, "unsaved idea");
    }
    #[test]
    fn file_drop_reaches_the_app_when_unfocused_and_modal_is_open() {
        let mut ui = app();
        settle(&mut ui);
        assert!(ui.send(Message::Chrome(ChromeAction::New)).is_ok());
        settle(&mut ui);
        ui.dispatch(
            Input::Key {
                key: Key::Character('p'),
                physical: 1,
                down: true,
                repeat: false,
                modifiers: Modifiers {
                    control: true,
                    ..Modifiers::default()
                },
            },
            &mut TestText,
        );
        settle(&mut ui);
        ui.window_focus(false);
        let path = PathBuf::from("/test/dropped.md");
        ui.dispatch(Input::FileDropped(path.clone()), &mut TestText);
        let outputs = settle(&mut ui);
        assert!(outputs
            .iter()
            .any(|o| matches!(o,Output::Load{path:p,..} if *p==path)));
    }
}
