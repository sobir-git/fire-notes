//! The find strip under a note: a query field, the occurrence count, and steps
//! through the note. The note's editor owns the occurrences; this strip only
//! asks for them and reports what it heard.
use crate::{components::*, design::*};
use fire_ui::*;
use fire_ui_widgets::*;
use std::{rc::Rc, sync::Arc};

pub const FIND_HEIGHT: f32 = 36.;

#[derive(Clone, Debug)]
pub enum FindCommand {
    /// Focus the query with its text selected and ask for its occurrences again.
    Open,
    /// How many occurrences the note has, and which one is selected.
    Found {
        selected: Option<usize>,
        count: usize,
    },
    Query(EditorOutput),
    Next,
    Previous,
    Close,
}
impl Data for FindCommand {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Query(o) => o.bytes(),
                _ => 0,
            }
    }
}
#[derive(Clone, Debug)]
pub enum FindOutput {
    Query(Arc<str>),
    Next,
    Previous,
    Close,
}
impl Data for FindOutput {
    fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Query(q) => q.len(),
                _ => 0,
            }
    }
}
pub struct FindBar {
    query: Child<Editor>,
    count: Child<Label>,
    previous: Child<IconButton>,
    next: Child<IconButton>,
    close: Child<IconButton>,
    text: Arc<str>,
}
impl FindBar {
    pub fn new() -> Element<Self> {
        Element::build(|c| Self {
            query: c.connect(
                Editor::new("")
                    .single_line()
                    .label("Find in note")
                    .key("find-query")
                    .placeholder("Find in note...")
                    .caret_blink(false)
                    .padding(8., 5.),
                |o| FindCommand::Query(o.clone()),
            ),
            count: c.add(label("", 13., MUTED)),
            previous: c.connect(icon_button(Icon::Up, "Previous match", false), |_| {
                FindCommand::Previous
            }),
            next: c.connect(icon_button(Icon::Down, "Next match", false), |_| {
                FindCommand::Next
            }),
            close: c.connect(icon_button(Icon::Cross, "Close find", false), |_| {
                FindCommand::Close
            }),
            text: Arc::from(""),
        })
    }
}
impl Widget for FindBar {
    type Command = FindCommand;
    type Output = FindOutput;
    fn lifecycle(&mut self, cx: &mut Update<'_, Self>, event: Lifecycle) {
        if event == Lifecycle::Mount {
            let _ = cx.set_environment(self.query, Rc::new(popup_theme()), true);
        }
    }
    fn update(&mut self, cx: &mut Update<'_, Self>, command: FindCommand) {
        match command {
            FindCommand::Open => {
                let _ = cx.focus_child(self.query);
                let _ = cx.send(
                    self.query,
                    Edit::Select {
                        anchor: 0,
                        caret: self.text.len(),
                    },
                );
                if !self.text.is_empty() {
                    let _ = cx.emit(FindOutput::Query(self.text.clone()));
                }
            }
            FindCommand::Found { selected, count } => {
                let status = match (selected, count) {
                    _ if self.text.is_empty() => String::new(),
                    (_, 0) => "No results".into(),
                    (Some(i), n) => format!("{} of {n}", i + 1),
                    (None, 1) => "1 match".into(),
                    (None, n) => format!("{n} matches"),
                };
                let _ = cx.send(self.count, status);
            }
            FindCommand::Query(EditorOutput::Changed { text, .. }) => {
                self.text = text.clone();
                let _ = cx.emit(FindOutput::Query(text));
            }
            FindCommand::Query(EditorOutput::Submitted) => {
                let _ = cx.emit(FindOutput::Next);
            }
            // A step button keeps typing in the query rather than taking focus.
            FindCommand::Next | FindCommand::Previous => {
                let _ = cx.emit(if matches!(command, FindCommand::Next) {
                    FindOutput::Next
                } else {
                    FindOutput::Previous
                });
                let _ = cx.focus_child(self.query);
            }
            FindCommand::Close => {
                let _ = cx.emit(FindOutput::Close);
            }
            FindCommand::Query(_) => {}
        }
    }
    fn input(&mut self, cx: &mut Update<'_, Self>, phase: Phase, input: &Input) {
        // The field submits on Enter whatever the modifiers; Shift steps back.
        if phase == Phase::Preview
            && matches!(input, Input::Key { key: Key::Enter, down: true, modifiers, .. } if modifiers.shift)
        {
            let _ = cx.emit(FindOutput::Previous);
            cx.stop();
        }
    }
    fn layout(&mut self, cx: &mut Layout<'_>, c: Constraints) -> Metrics {
        let w = c.max.width;
        let field = (w - 236.).clamp(0., 320.);
        let x = 8. + field + 12.;
        place(cx, self.query, Rect::new(8., 4., field, 28.));
        place(cx, self.count, Rect::new(x, 9., 92., 18.));
        place(cx, self.previous, Rect::new(x + 96., 4., 28., 28.));
        place(cx, self.next, Rect::new(x + 128., 4., 28., 28.));
        place(cx, self.close, Rect::new(w - 36., 4., 28., 28.));
        Metrics::new(c.max)
    }
    fn paint(&self, cx: &mut Paint<'_>) {
        cx.painter.rect(cx.bounds, 0., CHROME.into());
        cx.painter.rect(
            Rect::new(0., 0., cx.bounds.width, 1.),
            0.,
            ACTIVE_TAB.into(),
        );
    }
    fn semantics(&self) -> Semantics {
        Semantics {
            role: Role::Group,
            label: "Find in note".into(),
            key: Some("find-bar".into()),
            ..Semantics::default()
        }
    }
}
