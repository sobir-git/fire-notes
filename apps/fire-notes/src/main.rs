mod components;
mod storage;
use components::*;
use fire_ui::*;
use fire_ui_native::{run_with, WindowOptions};
use fire_ui_widgets::*;
use std::{collections::BTreeSet, path::PathBuf, sync::Arc};
use storage::{Note, Save, Session, Writer};

struct Record {
    note: Note,
    loaded: bool,
    page: Option<Child<Page>>,
    revision: u64,
    saved: u64,
}
struct Notes {
    directory: PathBuf,
    records: Vec<Record>,
    open: Vec<usize>,
    active: Option<usize>,
    brand: Child<Label>,
    new_button: Child<Button<Label>>,
    find_button: Child<Button<Label>>,
    commands_button: Child<Button<Label>>,
    tabs: Child<Tabs>,
    footer: Child<Label>,
    empty: Child<Label>,
    picker: Child<Picker>,
    anchor: Child<Label>,
    focus_pending: Option<usize>,
    pending_saves: BTreeSet<usize>,
    open_task: TaskSlot,
    wrap: bool,
    closing: bool,
    size: Size,
}
enum Message {
    New,
    Find(PickerMode),
    Tabs(TabAction),
    Page(usize, PageOutput),
    Pick(PickerOutput),
    Loaded(Option<usize>, Result<Note, String>),
    Saved(usize, u64, Result<(), String>),
}
impl Data for Message {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Page(_, o) => o.bytes(),
                Self::Pick(o) => o.bytes(),
                Self::Loaded(_, Ok(n)) => n.bytes(),
                Self::Loaded(_, Err(e)) | Self::Saved(_, _, Err(e)) => e.capacity(),
                _ => 0,
            }
    }
}
enum Output {
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
            }
    }
}
impl Notes {
    fn new(directory: PathBuf, notes: Vec<Note>, session: &Session) -> Element<Self> {
        let mut records: Vec<_> = notes
            .into_iter()
            .map(|note| Record {
                note,
                loaded: false,
                page: None,
                revision: 0,
                saved: 0,
            })
            .collect();
        let mut open = vec![];
        for path in session.tabs.iter().take(16) {
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
                    });
                    records.len() - 1
                });
            if let Ok(note) = Note::load(path) {
                records[id].note = note;
                records[id].loaded = true;
                if !open.contains(&id) {
                    open.push(id);
                }
            }
        }
        if open.is_empty() && !records.is_empty() {
            if let Ok(note) = Note::load(&records[0].note.path) {
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
                brand: c.add(label("Fire Notes", 22., Color::hex(0xf0c99e))),
                new_button: c.connect(button("New note"), |_| Message::New),
                find_button: c.connect(button("Find"), |_| Message::Find(PickerMode::Notes)),
                commands_button: c
                    .connect(button("Commands"), |_| Message::Find(PickerMode::Commands)),
                footer: c.add(label("Saved locally", 12., Color::hex(0x91a18b))),
                empty: c.add(label(
                    "A little room for your next idea.\n\nCreate a note, or find one with Ctrl P.",
                    20.,
                    Color::hex(0x9eae98),
                )),
                picker: c.connect(Picker::new(), |o| Message::Pick(o.clone())),
                anchor: c.add(Element::leaf(Label::new(""))),
                focus_pending: active,
                pending_saves: BTreeSet::new(),
                open_task: TaskSlot::new(),
                wrap: true,
                closing: false,
                size: Size::new(session.width, session.height),
            }
        })
    }
    fn status(&self, cx: &mut Update<'_, Self>, text: impl Into<String>) {
        let _ = cx.send(self.footer, text.into());
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
                content: record.note.markdown().into(),
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
            if self.open.len() >= 16 {
                self.status(
                    cx,
                    "Close a tab before opening another. Notes stay in your library.",
                );
                return;
            }
            if !self.records[id].loaded {
                self.load(cx, Some(id), self.records[id].note.path.clone());
                return;
            }
            let element = Page::new(
                self.records[id].note.title.clone(),
                self.records[id].note.body.clone(),
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
        if self.open.len() >= 16 {
            self.status(cx, "Close a tab before creating another.");
            return;
        }
        let note = storage::new_note(&self.directory);
        let id = self.records.len();
        self.records.push(Record {
            note,
            loaded: true,
            page: None,
            revision: 1,
            saved: 0,
        });
        self.pending_saves.insert(id);
        self.display(cx, id);
        self.flush(cx);
    }
    fn close_tab(&mut self, cx: &mut Update<'_, Self>, id: usize) {
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
    fn show_picker(&mut self, cx: &mut Update<'_, Self>, mode: PickerMode) {
        let items = if mode == PickerMode::Commands {
            vec![
                (
                    Choice::New,
                    Arc::from("New note                         Ctrl N"),
                ),
                (
                    Choice::Open,
                    Arc::from("Open a file                       Ctrl O"),
                ),
                (
                    Choice::Save,
                    Arc::from("Save note                         Ctrl S"),
                ),
                (Choice::Rename, Arc::from("Rename note")),
                (
                    Choice::Wrap,
                    Arc::from("Toggle word wrap                 Alt Z"),
                ),
                (
                    Choice::Close,
                    Arc::from("Close tab                         Ctrl W"),
                ),
            ]
        } else {
            self.records
                .iter()
                .enumerate()
                .map(|(id, r)| (Choice::Note(id), r.note.title.clone()))
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
    fn choose(&mut self, cx: &mut Update<'_, Self>, choice: Choice) {
        match choice {
            Choice::Note(id) => self.display(cx, id),
            Choice::New => self.create(cx),
            Choice::Open => self.show_picker(cx, PickerMode::File),
            Choice::Save => {
                if let Some(id) = self.active {
                    self.pending_saves.insert(id);
                    self.flush(cx);
                }
            }
            Choice::Rename => {
                if let Some(page) = self.active.and_then(|id| self.records[id].page) {
                    let _ = cx.send(page, PageCommand::Rename);
                }
            }
            Choice::Wrap => {
                self.wrap = !self.wrap;
                for r in &self.records {
                    if let Some(page) = r.page {
                        let _ = cx.send(page, PageCommand::Wrap(self.wrap));
                    }
                }
                self.status(
                    cx,
                    if self.wrap {
                        "Word wrap on"
                    } else {
                        "Word wrap off"
                    },
                );
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
                let _ = cx.anchor(self.picker, Some(self.anchor));
                let _ = cx.show(self.empty, self.active.is_none());
                for (id, r) in self.records.iter().enumerate() {
                    if let Some(page) = r.page {
                        let _ = cx.show(page, Some(id) == self.active);
                    }
                }
                cx.request_frame();
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
            Message::New => self.create(cx),
            Message::Find(mode) => self.show_picker(cx, mode),
            Message::Tabs(TabAction::Select(id)) => self.display(cx, id),
            Message::Tabs(TabAction::Close(id)) => {
                if let Some(page) = self.records[id].page {
                    let _ = cx.send(page, PageCommand::Close);
                }
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
                    PageOutput::Close | PageOutput::LimitReached => unreachable!(),
                };
                r.revision += 1;
                self.pending_saves.insert(id);
                self.sync_tabs(cx);
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
                    });
                } else if self.records[id].revision == self.records[id].saved {
                    self.records[id].note = note;
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
        if let Some(id) = self.focus_pending.take() {
            if let Some(page) = self.records[id].page {
                let _ = cx.send(page, PageCommand::Focus);
                let _ = cx.send(page, PageCommand::Wrap(self.wrap));
            }
        }
        self.flush(cx);
    }
    fn close_requested(&mut self, cx: &mut Update<'_, Self>) -> bool {
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
        if !matches!(phase, Phase::Preview | Phase::Target) {
            return;
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
                    Key::Character('p') => self.show_picker(cx, PickerMode::Notes),
                    Key::Character('o') => self.show_picker(cx, PickerMode::File),
                    Key::Character('/') => self.show_picker(cx, PickerMode::Commands),
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
        place(cx, self.brand, Rect::new(28., 22., 125., 32.));
        place(
            cx,
            self.new_button,
            Rect::new((w - 143.).max(0.), 16., 115., 42.),
        );
        let find_w = if w < 700. { 135. } else { 230. };
        place(
            cx,
            self.find_button,
            Rect::new(
                (w - find_w - 160.).max(165.),
                16.,
                (w - 325.).clamp(85., find_w),
                42.,
            ),
        );
        place(
            cx,
            self.commands_button,
            Rect::new(205., 16., if w >= 860. { 125. } else { 0. }, 42.),
        );
        place(cx, self.tabs, Rect::new(28., 77., (w - 56.).max(0.), 46.));
        let width = (w - 64.).clamp(0., 850.);
        let x = (w - width) / 2.;
        for r in &self.records {
            if let Some(page) = r.page {
                place(cx, page, Rect::new(x, 159., width, (h - 219.).max(0.)));
            }
        }
        place(
            cx,
            self.empty,
            Rect::new(x, 175., width, (h - 250.).max(0.)),
        );
        place(
            cx,
            self.footer,
            Rect::new(28., (h - 33.).max(0.), (w - 56.).max(0.), 22.),
        );
        place(cx, self.anchor, Rect::new(0., 0., 0., 0.));
        place(cx, self.picker, Rect::new(0., 0., w, h));
        Metrics::new(c.max)
    }
    fn paint(&self, cx: &mut Paint<'_>) {
        cx.painter.rect(cx.bounds, 0., paper().background.into());
        cx.painter.rect(
            Rect::new(0., 0., cx.bounds.width, 69.),
            0.,
            Color::hex(0x202721).into(),
        );
        cx.painter.rect(
            Rect::new(28., 135., (cx.bounds.width - 56.).max(0.), 1.),
            0.,
            Color::hex(0x354131).into(),
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
        let note=Note{path:directory.join("welcome.md"),title:Arc::from("Welcome to Fire Notes"),body:Arc::from("Your notes save automatically as Markdown files.\n\nCtrl N starts a note. Ctrl P finds one. Ctrl W closes a tab without deleting its note.\n\nEdit the title above to rename this note, or start writing here.")};
        storage::atomic_write(&note.path, note.markdown().as_bytes())?;
        library.push(note);
    }
    for note in &mut library {
        note.path = std::fs::canonicalize(&note.path).map_err(|e| e.to_string())?;
    }
    let session = storage::session(&directory);
    let size = Size::new(
        session.width.clamp(520., 2000.),
        session.height.clamp(480., 1600.),
    );
    let root = Notes::new(directory, library, &session);
    let mut writer: Option<Writer> = None;
    run_with(
        root,
        WindowOptions {
            title: "Fire Notes".into(),
            size,
            background: paper().background,
            limits: Limits {
                message_bytes: 32 * 1024 * 1024,
                ..Limits::default()
            },
        },
        move |output, wake| match output {
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
        assert!(ui.send(Message::New).is_ok());
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
        assert!(ui.send(Message::New).is_ok());
        settle(&mut ui);
        assert!(ui
            .send(Message::Page(
                0,
                PageOutput::Body(Arc::from("unsaved idea"))
            ))
            .is_ok());
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
}
