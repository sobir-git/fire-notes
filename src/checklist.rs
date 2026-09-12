use crate::design::*;
use fire_ui::*;
use fire_ui_widgets::{revealed, EditorExtension, EditorLayer, EditorView, Target, Transaction};

/// One parsed task-list line, as byte positions into the paragraph text.
struct Item {
    id: u64,
    line_start: usize,
    /// The bullet character, after any indentation.
    marker_start: usize,
    line_end: usize,
    state: usize,
    content: usize,
    checked: bool,
}

/// `- [ ] `, `- [x] `, `* [X] `, `+ [ ] `, after any indent, per GFM task lists.
fn parse_line(line: &str, offset: usize) -> Option<Item> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while matches!(bytes.get(i), Some(b' ' | b'\t')) {
        i += 1;
    }
    if !matches!(bytes.get(i), Some(b'-' | b'*' | b'+')) {
        return None;
    }
    let marker_start = offset + i;
    i += 1;
    let mut spaces = 0;
    while spaces < 3 && matches!(bytes.get(i), Some(b' ' | b'\t')) {
        i += 1;
        spaces += 1;
    }
    if spaces == 0 || bytes.get(i) != Some(&b'[') {
        return None;
    }
    let state = offset + i + 1;
    let checked = match bytes.get(i + 1) {
        Some(b'x' | b'X') => true,
        Some(b' ') => false,
        _ => return None,
    };
    if bytes.get(i + 2) != Some(&b']') {
        return None;
    }
    // Content follows one blank; text glued to the box is plain text.
    let mut content = offset + i + 3;
    match bytes.get(i + 3) {
        Some(b' ' | b'\t') => content += 1,
        Some(_) => return None,
        None => {}
    }
    Some(Item {
        id: 0,
        line_start: offset,
        marker_start,
        line_end: offset + line.len(),
        state,
        content,
        checked,
    })
}

/// The fence character and run length of a line that opens a code fence, per
/// CommonMark: up to three spaces of indent, then at least three of the same
/// fence character. An opening fence may carry an info string.
fn fence(line: &str) -> Option<(usize, u8)> {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    let ch = *trimmed.as_bytes().first()?;
    if ch != b'`' && ch != b'~' {
        return None;
    }
    let run = trimmed.bytes().take_while(|b| *b == ch).count();
    if run < 3 {
        return None;
    }
    let rest = &trimmed[run..];
    if ch == b'`' && rest.contains('`') {
        return None;
    }
    Some((run, ch))
}

/// Whether a line closes a fence that opened with `run` characters of `ch`:
/// at least as many fence characters and nothing else but whitespace.
fn closes(line: &str, ch: u8, run: usize) -> bool {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return false;
    }
    let taken = trimmed.bytes().take_while(|b| *b == ch).count();
    taken >= run && trimmed[taken..].trim().is_empty()
}

fn parse(text: &str) -> Vec<Item> {
    let mut items = vec![];
    let mut open: Option<(usize, u8)> = None;
    let mut start = 0;
    for line in text.split('\n') {
        match open {
            Some((run, ch)) => {
                // Only a closing fence of the same character ends the block.
                if closes(line, ch, run) {
                    open = None;
                }
            }
            None => {
                if let Some((run, ch)) = fence(line) {
                    open = Some((run, ch));
                } else if let Some(item) = parse_line(line, start) {
                    items.push(item);
                }
            }
        }
        start += line.len() + 1;
    }
    items
}

/// How long the check mark takes to draw itself after a toggle, in seconds.
const POP: f32 = 0.16;
/// Extra pixels around the box that still count as a click on it.
const MARGIN: f32 = 3.;

/// Markdown task lists rendered and toggled in place, as an editor extension.
///
/// The editor suppresses the glyphs of each presented marker and reveals the
/// raw characters while the caret or selection is inside it; this extension
/// draws the checkbox over the reserved space, toggles the state character
/// through a transaction, and publishes each box as a semantic child.
pub struct Checklist {
    items: Vec<Item>,
    revision: u64,
    pop: Option<(usize, f32)>,
    source: std::sync::Arc<str>,
    next_id: u64,
}

impl Default for Checklist {
    fn default() -> Self {
        Self {
            items: vec![],
            revision: u64::MAX,
            pop: None,
            source: "".into(),
            next_id: 1,
        }
    }
}

impl Checklist {
    /// Re-parse the task list and start the pop when a state byte flipped.
    fn refresh(&mut self, view: &EditorView<'_>) {
        if self.revision == view.paragraph.revision {
            return;
        }
        let previous: std::collections::HashMap<usize, bool> = self
            .items
            .iter()
            .map(|item| (item.state, item.checked))
            .collect();
        let fresh = &view.paragraph.text;
        let mut prefix = self
            .source
            .bytes()
            .zip(fresh.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        let common_suffix = |prefix| {
            self.source.as_bytes()[prefix..]
                .iter()
                .rev()
                .zip(fresh.as_bytes()[prefix..].iter().rev())
                .take_while(|(a, b)| a == b)
                .count()
        };
        let mut suffix = common_suffix(prefix);
        if self.source.as_bytes()[prefix..self.source.len() - suffix].contains(&b'\n')
            || fresh.as_bytes()[prefix..fresh.len() - suffix].contains(&b'\n')
        {
            // Identical task prefixes must not transfer a removed row's id to
            // its successor. Compare whole affected rows when lines change.
            prefix = self.source.as_bytes()[..prefix]
                .iter()
                .rposition(|b| *b == b'\n')
                .map_or(0, |i| i + 1);
            suffix = common_suffix(prefix);
        }
        let old_end = self.source.len() - suffix;
        let new_end = fresh.len() - suffix;
        let identities: std::collections::HashMap<usize, u64> = self
            .items
            .iter()
            .filter_map(|item| {
                let state = if item.state < prefix {
                    item.state
                } else if item.state >= old_end {
                    item.state - old_end + new_end
                } else if item.state == prefix && old_end == prefix + 1 && new_end == old_end {
                    item.state // Checking or unchecking keeps the same task identity.
                } else {
                    return None;
                };
                Some((state, item.id))
            })
            .collect();
        self.items = parse(&view.paragraph.text);
        for item in &mut self.items {
            item.id = identities.get(&item.state).copied().unwrap_or_else(|| {
                let id = self.next_id;
                self.next_id += 1;
                id
            });
        }
        self.source = fresh.clone();
        self.revision = view.paragraph.revision;
        if let Some(fresh) = self
            .items
            .iter()
            .find(|item| previous.get(&item.state) == Some(&!item.checked))
        {
            self.pop = Some((fresh.state, 0.));
        }
        // A pop for an item that no longer exists is stale; drop it.
        if self
            .pop
            .as_ref()
            .is_some_and(|(state, _)| !self.items.iter().any(|item| item.state == *state))
        {
            self.pop = None;
        }
    }
    /// The box geometry, centered over the presented marker's first visual
    /// fragment so indented and wrapped items place it correctly.
    fn square(paragraph: &Paragraph, item: &Item) -> Option<Rect> {
        let marker = paragraph
            .range_fragments(item.marker_start..item.state + 2)
            .into_iter()
            .next()?;
        let side = paragraph.line_height * 0.55;
        let center = (marker.x.start + marker.x.end) / 2.;
        Some(Rect::new(
            center - side / 2.,
            marker.y + (paragraph.line_height - side) / 2.,
            side,
            side,
        ))
    }
    /// Strikethrough rectangles, one per visual row the checked content
    /// occupies, so wrapped items are struck on every row. Trailing
    /// whitespace is not struck.
    fn strikes(paragraph: &Paragraph, item: &Item) -> Vec<Rect> {
        let end = item.content + paragraph.text[item.content..item.line_end].trim_end().len();
        paragraph
            .range_fragments(item.content..end)
            .into_iter()
            .map(|f| {
                Rect::new(
                    f.x.start,
                    f.y + paragraph.line_height * 0.56,
                    (f.x.end - f.x.start).max(0.),
                    1.5,
                )
            })
            .collect()
    }
    /// The state-character replacement that flips this item.
    fn toggle(item: &Item) -> Transaction {
        Transaction::replace(
            item.state..item.state + 1,
            if item.checked { " " } else { "x" },
        )
    }
    /// Enter on a task line continues the list: a fresh unchecked marker
    /// follows the caret, which lands ready for typing. Splitting mid-item
    /// carries the rest of the line into the new item, and Enter on an empty
    /// item removes it instead — the standard way out of a list.
    fn continue_list(&self, view: &EditorView<'_>) -> Option<Transaction> {
        let text = &view.paragraph.text;
        let selection = view.selection.clone();
        let at = selection.as_ref().map_or(view.caret.byte, |r| r.start);
        let item = self
            .items
            .iter()
            .find(|item| at >= item.line_start && at <= item.line_end)?;
        let indent = &text[item.line_start..item.marker_start];
        if let Some(range) = selection {
            // Removing source syntax must behave like ordinary text editing.
            if range.start < item.content {
                return None;
            }
            let insert = format!("\n{indent}- [ ] ");
            let caret = range.start + insert.len();
            return Some(Transaction::replace(range, insert).caret(caret).reveal());
        }
        if text[item.content..item.line_end].trim().is_empty() {
            return Some(
                Transaction::replace(item.line_start..item.line_end, "")
                    .caret(item.line_start)
                    .reveal(),
            );
        }
        // A caret inside the marker acts at its end; the marker itself never
        // splits.
        let at = at.clamp(item.content, item.line_end);
        let insert = format!("\n{indent}- [ ] ");
        let caret = at + insert.len();
        Some(Transaction::replace(at..at, insert).caret(caret).reveal())
    }
    fn pop_progress(&self, state: usize) -> f32 {
        self.pop
            .as_ref()
            .filter(|(byte, _)| *byte == state)
            .map_or(1., |(_, progress)| *progress)
    }
    fn visible_items(&self, view: &EditorView<'_>) -> &[Item] {
        let lines = &view.paragraph.lines;
        let start = lines.partition_point(|l| l.y + view.paragraph.line_height < view.viewport.y);
        let end = lines.partition_point(|l| l.y <= view.viewport.y + view.viewport.height);
        let Some(first) = lines.get(start).filter(|_| start < end) else {
            return &[];
        };
        let last = &lines[end - 1];
        let begin = self
            .items
            .partition_point(|item| item.line_end < first.range.start);
        let finish = self
            .items
            .partition_point(|item| item.marker_start <= last.range.end);
        &self.items[begin..finish]
    }
}

impl EditorExtension for Checklist {
    fn sync(&mut self, view: &EditorView<'_>) {
        self.refresh(view);
    }
    fn presentation(
        &self,
        _view: &EditorView<'_>,
        present: &mut dyn FnMut(std::ops::Range<usize>),
    ) {
        for item in &self.items {
            present(item.marker_start..item.state + 2);
        }
    }
    fn targets(&self, view: &EditorView<'_>) -> Vec<Target> {
        self.visible_items(view)
            .iter()
            // While the raw syntax is revealed for editing, clicks belong to
            // the text: the box steps aside entirely.
            .filter(|item| {
                !revealed(
                    item.marker_start..item.state + 2,
                    view.caret,
                    view.selection.clone(),
                )
            })
            .filter_map(|item| {
                Some(Target {
                    id: item.id,
                    bounds: Self::square(view.paragraph, item)?.inset(-MARGIN),
                })
            })
            .collect()
    }
    fn activate(&mut self, _view: &EditorView<'_>, target: u64) -> Option<Transaction> {
        let item = self.items.iter().find(|item| item.id == target)?;
        Some(Self::toggle(item))
    }
    fn key(
        &mut self,
        view: &EditorView<'_>,
        key: Key,
        modifiers: Modifiers,
    ) -> Option<Transaction> {
        if key != Key::Enter {
            return None;
        }
        if modifiers.control {
            let at = view.caret.byte;
            let item = self
                .items
                .iter()
                .find(|item| at >= item.line_start && at <= item.line_end)?;
            return Some(Self::toggle(item));
        }
        if modifiers.shift || modifiers.alt || modifiers.meta {
            return None;
        }
        self.continue_list(view)
    }
    fn frame(&mut self, _view: &EditorView<'_>, time: FrameTime, _inserted: bool) -> bool {
        if let Some((_, progress)) = &mut self.pop {
            *progress += time.elapsed.as_secs_f32() / POP;
            if *progress >= 1. {
                self.pop = None;
            }
        }
        self.pop.is_some()
    }
    fn damage(&self, view: &EditorView<'_>) -> Option<Rect> {
        let mut total: Option<Rect> = None;
        for item in self.visible_items(view) {
            let mut union = |rect: Rect| {
                total = Some(total.map_or(rect, |t| t.union(rect)));
            };
            if let Some(square) = Self::square(view.paragraph, item) {
                union(square.inset(-1.));
            }
            if item.checked {
                for strike in Self::strikes(view.paragraph, item) {
                    union(strike);
                }
            }
        }
        total
    }
    fn paint(&self, view: &EditorView<'_>, layer: EditorLayer, p: &mut dyn Painter) {
        if layer != EditorLayer::AboveText {
            return;
        }
        for item in self.visible_items(view) {
            let Some(square) = Self::square(view.paragraph, item) else {
                continue;
            };
            // While the raw syntax is revealed for editing, the box steps
            // aside so the characters being edited are unobstructed.
            if revealed(
                item.marker_start..item.state + 2,
                view.caret,
                view.selection.clone(),
            ) {
                continue;
            }
            if item.checked {
                p.rect(square, square.width * 0.28, EMBER.into());
                let t = self.pop_progress(item.state);
                let center =
                    Point::new(square.x + square.width / 2., square.y + square.height / 2.);
                let at = |fx: f32, fy: f32| {
                    let scale = 0.6 + 0.4 * t;
                    Point::new(
                        center.x + (square.x + fx * square.width - center.x) * scale,
                        center.y + (square.y + fy * square.height - center.y) * scale,
                    )
                };
                p.path(
                    &[
                        Path::Move(at(0.26, 0.52)),
                        Path::Line(at(0.44, 0.72)),
                        Path::Line(at(0.78, 0.28)),
                    ],
                    CANVAS.alpha(t).into(),
                    Some(2.2),
                );
                for strike in Self::strikes(view.paragraph, item) {
                    p.rect(strike, 0., EMBER.alpha(0.85).into());
                }
            } else {
                p.stroke(square, square.width * 0.28, 1.5, MUTED);
            }
        }
    }
    fn semantics(&self, view: &EditorView<'_>, target: u64) -> Option<Semantics> {
        let item = self.items.iter().find(|item| item.id == target)?;
        Some(Semantics {
            role: Role::CheckBox,
            label: view.paragraph.text[item.content..item.line_end]
                .trim()
                .to_string(),
            checked: Some(item.checked),
            actions: vec![SemanticActionKind::Activate],
            ..Semantics::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Page, PageOutput};
    use fire_ui_widgets::{Edit, Editor, EditorState};
    use std::sync::Arc;

    fn states(text: &str) -> Vec<(usize, usize, bool)> {
        parse(text)
            .into_iter()
            .map(|item| (item.state, item.content, item.checked))
            .collect()
    }

    #[test]
    fn parses_task_list_lines() {
        let text = "- [ ] one\n* [x] two\n+ [X] three\nnot a task\n  - [ ] indented\n- [ ]";
        let items = parse(text);
        assert_eq!(items.len(), 5);
        assert_eq!(items[0].marker_start, 0);
        assert_eq!(items[0].state, 3);
        assert_eq!(items[0].content, 6);
        assert!(!items[0].checked);
        assert_eq!(items[1].state, 13);
        assert!(items[1].checked);
        assert!(items[2].checked);
        assert_eq!(items[3].line_start, 43);
        assert_eq!(items[4].marker_start, 60);
        assert_eq!(items[4].state, text.len() - 2);
        assert_eq!(items[4].content, text.len());
        // Up to three spaces may follow the bullet, per CommonMark.
        assert_eq!(states("-  [ ] two"), vec![(4, 7, false)]);
        assert_eq!(states("-   [ ] three"), vec![(5, 8, false)]);
    }

    #[test]
    fn rejects_lookalikes() {
        assert!(parse("- [ ]nospace").is_empty());
        assert!(parse("- []").is_empty());
        assert!(parse("- [y] no").is_empty());
        assert!(parse("-[ ] no space").is_empty());
        assert!(parse("- [ ]no space").is_empty());
        assert!(parse("-    [ ] four spaces is code").is_empty());
        assert_eq!(
            states("- [ ] a\n- [x] b"),
            vec![(3, 6, false), (11, 14, true)]
        );
    }

    #[test]
    fn skips_task_syntax_inside_fenced_code_blocks() {
        let text = "- [ ] real\n```text\n- [ ] not a task\n```\n- [x] after";
        let items = parse(text);
        assert_eq!(items.len(), 2, "the fenced example is not a task");
        assert_eq!(items[0].state, 3);
        assert_eq!(items[1].state, 43);
        assert!(items[1].checked);
        // Tilde fences count too, and the fence state survives blank lines.
        assert_eq!(parse("~~~\n\n- [ ] no\n~~~\n- [ ] yes").len(), 1);
        // A closing fence cannot carry an info string: this line is code
        // content, so the block stays open across the lookalike task.
        let tricky = "```text\n```not-a-closer\n- [ ] still code\n```\n- [ ] yes";
        assert_eq!(parse(tricky).len(), 1, "only the line after the block");
        assert_eq!(parse(tricky)[0].state, 48);
    }

    /// A page with the checklist composed alongside flames, at editor scale.
    fn page(body: &str) -> Ui<Page> {
        let mut ui = Ui::new(
            Page::new(Arc::from("Test"), Arc::from(body), EditorState::default()),
            Size::new(600., 400.),
            Limits::default(),
        )
        .unwrap();
        for _ in 0..10 {
            ui.frame(std::time::Duration::ZERO, 100);
            ui.pump(1000, |_| {}, |_| {});
            ui.layout(&mut TestText);
        }
        ui
    }
    fn settle(ui: &mut Ui<Page>) -> Vec<PageOutput> {
        let mut outputs = vec![];
        for _ in 0..10 {
            ui.frame(std::time::Duration::ZERO, 100);
            ui.pump(1000, |o| outputs.push(o), |_| {});
            ui.layout(&mut TestText);
        }
        outputs
    }
    /// The box center for the first task line, in window coordinates.
    /// The editor theme is 16px; TestText advances 9.6 per cell, 24 per line.
    fn box_center() -> Point {
        let (pad_x, pad_y) = (16., 8.);
        let line_height = 24.;
        let side = line_height * 0.55;
        let end = 5. * 9.6;
        Point::new(
            pad_x + (end - side) / 2. + side / 2.,
            pad_y + line_height / 2.,
        )
    }
    fn press(ui: &mut Ui<Page>, at: Point) -> Vec<PageOutput> {
        let mut outputs = vec![];
        ui.dispatch(
            Input::Button {
                pointer: 0,
                button: 1,
                down: true,
                position: at,
            },
            &mut TestText,
        );
        outputs.extend(settle(ui));
        ui.dispatch(
            Input::Button {
                pointer: 0,
                button: 1,
                down: false,
                position: at,
            },
            &mut TestText,
        );
        outputs.extend(settle(ui));
        outputs
    }
    /// Pointer press/release against a bare `Ui<Editor>` (no Page wrapper).
    fn press_editor(ui: &mut Ui<Editor>, at: Point) {
        ui.dispatch(
            Input::Button {
                pointer: 0,
                button: 1,
                down: true,
                position: at,
            },
            &mut TestText,
        );
        for _ in 0..10 {
            ui.frame(std::time::Duration::ZERO, 100);
            ui.pump(1000, |_| {}, |_| {});
            ui.layout(&mut TestText);
        }
        ui.dispatch(
            Input::Button {
                pointer: 0,
                button: 1,
                down: false,
                position: at,
            },
            &mut TestText,
        );
        for _ in 0..10 {
            ui.frame(std::time::Duration::ZERO, 100);
            ui.pump(1000, |_| {}, |_| {});
            ui.layout(&mut TestText);
        }
    }

    #[test]
    fn toggles_through_a_real_editor() {
        let mut ui = page("- [ ] buy milk");
        let at = box_center();
        let outputs = press(&mut ui, at);
        assert!(outputs
            .iter()
            .any(|o| matches!(o, PageOutput::Body(text) if &**text == "- [x] buy milk")));
        let outputs = press(&mut ui, at);
        assert!(outputs
            .iter()
            .any(|o| matches!(o, PageOutput::Body(text) if &**text == "- [ ] buy milk")));
    }

    #[test]
    fn plain_clicks_still_place_the_caret() {
        let mut ui = page("- [ ] buy milk");
        // Click well right of the box, on the text itself.
        let at = Point::new(16. + 9.6 * 10., 8. + 12.);
        let outputs = press(&mut ui, at);
        assert!(!outputs
            .iter()
            .any(|o| matches!(o, PageOutput::Body(text) if &**text == "- [x] buy milk")));
    }

    #[test]
    fn typing_markdown_stays_verbatim_and_each_box_toggles() {
        let body = "- [ ] buy milk\n- [x] ship the release\nplain text";
        let mut ui = page(body);
        // The stored Markdown is untouched by rendering.
        let at = box_center();
        let outputs = press(&mut ui, at);
        assert!(outputs
            .iter()
            .any(|o| matches!(o, PageOutput::Body(text) if &**text == "- [x] buy milk\n- [x] ship the release\nplain text")));
        let outputs = press(&mut ui, at);
        assert!(outputs
            .iter()
            .any(|o| matches!(o, PageOutput::Body(text) if &**text == body)));
        // The second box toggles independently.
        let pitch = 24.;
        let at = Point::new(at.x, at.y + pitch);
        let outputs = press(&mut ui, at);
        assert!(outputs
            .iter()
            .any(|o| matches!(o, PageOutput::Body(text) if &**text ==
                "- [ ] buy milk\n- [ ] ship the release\nplain text")));
    }

    /// A bare editor with the checklist, settled and ready.
    fn bare(body: &str) -> Ui<Editor> {
        let mut ui = Ui::new(
            Element::leaf(
                Editor::new(body)
                    .extension(crate::checklist::Checklist::default())
                    .padding(16., 8.)
                    .chrome(false),
            ),
            Size::new(600., 400.),
            Limits::default(),
        )
        .unwrap();
        settle_editor(&mut ui);
        ui
    }
    fn settle_editor(ui: &mut Ui<Editor>) {
        for _ in 0..10 {
            ui.frame(std::time::Duration::ZERO, 100);
            ui.pump(1000, |_| {}, |_| {});
            ui.layout(&mut TestText);
        }
    }
    fn press_key(ui: &mut Ui<Editor>, key: Key, modifiers: Modifiers) {
        ui.dispatch(
            Input::Key {
                key,
                physical: 1,
                down: true,
                repeat: false,
                modifiers,
            },
            &mut TestText,
        );
        settle_editor(ui);
    }

    #[test]
    fn toggle_is_one_undo_step_and_redo_restores_it() {
        let mut ui = bare("- [ ] buy milk");
        // The box for a 16px editor theme: side = 24*0.55 = 13.2, centered
        // over the marker fragment 0..48, so its center sits at (24, 12)
        // in paragraph coordinates; the editor is the whole window here.
        let side = 24. * 0.55;
        let at = Point::new(16. + (48. - side) / 2. + side / 2., 8. + 12.);
        press_editor(&mut ui, at);
        assert_eq!(ui.root().text(), "- [x] buy milk", "toggled on");
        ui.send(Edit::Undo).unwrap();
        settle_editor(&mut ui);
        assert_eq!(ui.root().text(), "- [ ] buy milk", "one undo reverts it");
        ui.send(Edit::Redo).unwrap();
        settle_editor(&mut ui);
        assert_eq!(ui.root().text(), "- [x] buy milk", "one redo reapplies it");
    }

    #[test]
    fn ctrl_enter_toggles_the_item_at_the_caret() {
        let mut ui = bare("- [ ] buy milk\n- [x] ship it");
        let key = |control: bool| Input::Key {
            key: Key::Enter,
            physical: 36,
            down: true,
            repeat: false,
            modifiers: Modifiers {
                control,
                ..Modifiers::default()
            },
        };
        // The caret starts on the first item.
        ui.dispatch(key(true), &mut TestText);
        settle_editor(&mut ui);
        assert_eq!(
            ui.root().text(),
            "- [x] buy milk\n- [x] ship it",
            "Ctrl+Enter toggled the item under the caret"
        );
        // Plain Enter is not a toggle: it continues the list instead.
        ui.dispatch(key(false), &mut TestText);
        settle_editor(&mut ui);
        assert_eq!(
            ui.root().text(),
            "- [x] \n- [ ] buy milk\n- [x] ship it",
            "plain Enter continues the list; only Ctrl+Enter toggles"
        );
    }

    #[test]
    fn enter_replaces_selection_in_either_direction_and_undo_restores_it() {
        for (anchor, caret) in [(10, 14), (14, 10)] {
            let mut ui = bare("- [ ] buy milk");
            ui.send(Edit::Select { anchor, caret }).unwrap();
            settle_editor(&mut ui);
            press_key(&mut ui, Key::Enter, Modifiers::default());
            assert_eq!(ui.root().text(), "- [ ] buy \n- [ ] ");
            ui.send(Edit::Undo).unwrap();
            settle_editor(&mut ui);
            assert_eq!(ui.root().text(), "- [ ] buy milk");
            assert_eq!(ui.root().selection(), Some(10..14));
        }
        let mut ui = bare("- [ ] buy milk\n- [ ] bread");
        ui.send(Edit::Select {
            anchor: 10,
            caret: 26,
        })
        .unwrap();
        settle_editor(&mut ui);
        press_key(&mut ui, Key::Enter, Modifiers::default());
        assert_eq!(ui.root().text(), "- [ ] buy \n- [ ] ");
        ui.send(Edit::Select {
            anchor: 0,
            caret: ui.root().text().len(),
        })
        .unwrap();
        settle_editor(&mut ui);
        press_key(&mut ui, Key::Enter, Modifiers::default());
        assert_eq!(ui.root().text(), "\n");
    }

    #[test]
    fn semantic_identity_survives_another_checkbox_being_revealed() {
        let mut ui = bare("- [ ] first\n- [ ] second");
        let before: Vec<_> = ui
            .semantics()
            .into_iter()
            .filter(|n| n.semantics.role == Role::CheckBox)
            .collect();
        assert_eq!(before.len(), 2);
        ui.send(Edit::Select {
            anchor: 3,
            caret: 3,
        })
        .unwrap();
        settle_editor(&mut ui);
        let after: Vec<_> = ui
            .semantics()
            .into_iter()
            .filter(|n| n.semantics.role == Role::CheckBox)
            .collect();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].id, before[1].id);
        assert_eq!(
            ui.accessibility(before[0].id, SemanticAction::Activate),
            Err(SemanticError::Unavailable)
        );
        ui.accessibility(before[1].id, SemanticAction::Activate)
            .unwrap();
        settle_editor(&mut ui);
        assert_eq!(ui.root().text(), "- [ ] first\n- [x] second");
    }

    #[test]
    fn task_identity_follows_inserted_text_and_deleted_tasks_do_not_retarget() {
        let mut ui = bare("- [ ] first\n- [ ] second");
        let before: Vec<_> = ui
            .semantics()
            .into_iter()
            .filter(|n| n.semantics.role == Role::CheckBox)
            .collect();
        ui.send(Edit::Select {
            anchor: 0,
            caret: 0,
        })
        .unwrap();
        ui.send(Edit::Insert("plain\n".into())).unwrap();
        settle_editor(&mut ui);
        let after: Vec<_> = ui
            .semantics()
            .into_iter()
            .filter(|n| n.semantics.role == Role::CheckBox)
            .collect();
        assert_eq!(
            after.iter().map(|n| n.id).collect::<Vec<_>>(),
            before.iter().map(|n| n.id).collect::<Vec<_>>()
        );
        ui.send(Edit::Select {
            anchor: 6,
            caret: 18,
        })
        .unwrap();
        ui.send(Edit::Insert("".into())).unwrap();
        settle_editor(&mut ui);
        assert_eq!(
            ui.accessibility(before[0].id, SemanticAction::Activate),
            Err(SemanticError::Unavailable)
        );
        ui.accessibility(before[1].id, SemanticAction::Activate)
            .unwrap();
        settle_editor(&mut ui);
        assert_eq!(ui.root().text(), "plain\n- [x] second");
    }

    #[test]
    #[ignore = "manual geometry timing"]
    fn measure_visible_checklist_geometry() {
        for count in [100, 1_000, 10_000] {
            let text: Arc<str> = "- [x] task\n".repeat(count).into();
            let paragraph = TestText.layout(TextRequest {
                text,
                style: TextStyle::default(),
                width: None,
                revision: 1,
                previous: None,
            });
            let view = EditorView {
                paragraph: &paragraph,
                caret: Caret::at(0),
                selection: None,
                viewport: Rect::new(0., 0., 600., 400.),
                focused: true,
            };
            let mut checklist = Checklist::default();
            checklist.sync(&view);
            assert!(checklist.visible_items(&view).len() <= 20);
            let start = std::time::Instant::now();
            for _ in 0..1_000 {
                std::hint::black_box(checklist.targets(&view));
                std::hint::black_box(checklist.damage(&view));
            }
            eprintln!(
                "{count} tasks: targets + damage {:.2} us/iteration",
                start.elapsed().as_secs_f64() * 1_000.
            );
        }
    }

    #[test]
    fn enter_continues_the_list_at_the_caret() {
        let mut ui = bare("- [ ] buy milk");
        // A plain click past the text end focuses the editor (text input
        // routes to the focused widget only) and parks the caret there.
        press_editor(&mut ui, Point::new(150., 20.));
        ui.send(Edit::Select {
            anchor: 14,
            caret: 14,
        })
        .unwrap();
        settle_editor(&mut ui);
        press_key(&mut ui, Key::Enter, Modifiers::default());
        assert_eq!(ui.root().text(), "- [ ] buy milk\n- [ ] ");
        // Typing lands after the fresh marker, ready to write the item.
        // The IME session advanced when the click focused the editor.
        ui.dispatch(
            Input::Text {
                session: ui.session(),
                text: "second".into(),
            },
            &mut TestText,
        );
        settle_editor(&mut ui);
        assert_eq!(ui.root().text(), "- [ ] buy milk\n- [ ] second");
    }

    #[test]
    fn enter_mid_item_splits_it() {
        let mut ui = bare("- [ ] buy milk");
        ui.send(Edit::Select {
            anchor: 10,
            caret: 10,
        })
        .unwrap();
        settle_editor(&mut ui);
        press_key(&mut ui, Key::Enter, Modifiers::default());
        // The space before the caret stays on the first line, as in any
        // plain text split.
        assert_eq!(ui.root().text(), "- [ ] buy \n- [ ] milk");
    }

    #[test]
    fn enter_on_an_empty_item_exits_the_list() {
        let mut ui = bare("- [ ] buy milk\n- [ ] ");
        ui.send(Edit::Select {
            anchor: 21,
            caret: 21,
        })
        .unwrap();
        settle_editor(&mut ui);
        press_key(&mut ui, Key::Enter, Modifiers::default());
        assert_eq!(
            ui.root().text(),
            "- [ ] buy milk\n",
            "the empty item made way for plain text"
        );
    }

    #[test]
    fn enter_on_plain_text_is_not_intercepted() {
        let mut ui = bare("hello\n- [ ] task");
        ui.send(Edit::Select {
            anchor: 2,
            caret: 2,
        })
        .unwrap();
        settle_editor(&mut ui);
        press_key(&mut ui, Key::Enter, Modifiers::default());
        assert_eq!(ui.root().text(), "he\nllo\n- [ ] task");
    }

    #[test]
    fn continuation_preserves_indent_and_is_one_undo_step() {
        let mut ui = bare("  - [ ] nested");
        ui.send(Edit::Select {
            anchor: 14,
            caret: 14,
        })
        .unwrap();
        settle_editor(&mut ui);
        press_key(&mut ui, Key::Enter, Modifiers::default());
        assert_eq!(ui.root().text(), "  - [ ] nested\n  - [ ] ");
        ui.send(Edit::Undo).unwrap();
        settle_editor(&mut ui);
        assert_eq!(ui.root().text(), "  - [ ] nested", "one undo reverts it");
    }

    #[test]
    fn pop_starts_on_a_toggle_and_dies_with_its_item() {
        let make = |text: &str, revision: u64| Paragraph {
            text: text.into(),
            style: TextStyle::default(),
            revision,
            service_revision: fire_ui::TextRevision::new(),
            width: None,
            size: Size::new(600., 400.),
            baseline: 19.,
            line_height: 24.,
            lines: vec![],
        };
        let mut checklist = Checklist::default();
        let paragraph = make("- [ ] buy milk", 1);
        let view = EditorView {
            paragraph: &paragraph,
            caret: Caret::at(0),
            selection: None,
            viewport: Rect::new(0., 0., 600., 400.),
            focused: false,
        };
        checklist.sync(&view);
        assert!(checklist.pop.is_none());
        // The editor applies the toggle; the next sync sees the flip.
        let _transaction = checklist.activate(&view, checklist.items[0].id).unwrap();
        let toggled = make("- [x] buy milk", 2);
        let view = EditorView {
            paragraph: &toggled,
            ..view
        };
        checklist.sync(&view);
        assert_eq!(checklist.pop.map(|(state, _)| state), Some(3));
        // The item disappears; the stale pop must go with it.
        let gone = make("plain", 3);
        let view = EditorView {
            paragraph: &gone,
            ..view
        };
        checklist.sync(&view);
        assert!(checklist.pop.is_none(), "stale pop must be dropped");
    }
}
