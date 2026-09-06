//! Consumer experiment: versioned text-layout ownership, not a native shaping implementation.
use fire_ui_contract_model::{Affinity, LayoutVersion, Paragraph, Point, TextPosition};
use std::{cell::Cell, rc::Rc};

struct Document {
    text: String,
    revision: u64,
}
impl Document {
    fn replace(&mut self, range: std::ops::Range<usize>, value: &str) -> Result<(), ()> {
        if range.start > range.end
            || range.end > self.text.len()
            || !self.text.is_char_boundary(range.start)
            || !self.text.is_char_boundary(range.end)
        {
            return Err(());
        }
        self.text.replace_range(range, value);
        self.revision += 1;
        Ok(())
    }
}
struct Editor {
    document: Document,
    font: u64,
    width: f64,
    layout: Option<Rc<Paragraph>>,
    cursor: TextPosition,
    session: u64,
    preedit: String,
}
impl Editor {
    fn new(text: &str) -> Self {
        Self {
            document: Document {
                text: text.into(),
                revision: 0,
            },
            font: 4,
            width: 400.,
            layout: None,
            cursor: TextPosition {
                byte: 0,
                affinity: Affinity::Downstream,
            },
            session: 1,
            preedit: String::new(),
        }
    }
    fn version(&self) -> LayoutVersion {
        LayoutVersion {
            document: self.document.revision,
            font: self.font,
            width_bits: self.width.to_bits(),
        }
    }
    fn install(&mut self, layout: Rc<Paragraph>) -> Result<(), Rc<Paragraph>> {
        if layout.version != self.version() {
            return Err(layout);
        }
        self.layout = Some(layout);
        Ok(())
    }
    fn current(&self) -> Option<&Rc<Paragraph>> {
        self.layout.as_ref().filter(|l| l.version == self.version())
    }
    fn hit(&mut self, x: f64) -> bool {
        let Some(layout) = self.current() else {
            return false;
        };
        let Some((cursor, _)) = layout
            .visual_stops
            .iter()
            .min_by(|(_, a), (_, b)| (a.0 - x).abs().total_cmp(&(b.0 - x).abs()))
        else {
            return false;
        };
        self.cursor = *cursor;
        true
    }
    fn move_visual(&mut self, forward: bool) -> bool {
        let Some(layout) = self.current() else {
            return false;
        };
        let Some(at) = layout
            .visual_stops
            .iter()
            .position(|(c, _)| *c == self.cursor)
        else {
            return false;
        };
        let next = if forward {
            (at + 1).min(layout.visual_stops.len() - 1)
        } else {
            at.saturating_sub(1)
        };
        self.cursor = layout.visual_stops[next].0;
        true
    }
    fn paint(&self, sink: &mut impl FnMut(&Rc<Paragraph>)) -> bool {
        if let Some(layout) = self.current() {
            sink(layout);
            true
        } else {
            false
        }
    }
    fn compose(&mut self, session: u64, preedit: &str) {
        if session == self.session {
            self.preedit = preedit.into();
        }
    }
    fn commit(&mut self, session: u64, text: &str) -> bool {
        if session != self.session {
            return false;
        }
        if self
            .document
            .replace(self.cursor.byte..self.cursor.byte, text)
            .is_err()
        {
            return false;
        }
        self.cursor.byte += text.len();
        self.preedit.clear();
        true
    }
    fn blur(&mut self) {
        self.session += 1;
        self.preedit.clear();
    }
}
fn fixture(version: LayoutVersion) -> Rc<Paragraph> {
    // A service-supplied fixture with two visual affinities at byte 1.
    // Tests exercise its consumers; they make no claim about native bidi shaping.
    Rc::new(Paragraph {
        version,
        visual_stops: vec![
            (
                TextPosition {
                    byte: 0,
                    affinity: Affinity::Downstream,
                },
                Point(0., 0.),
            ),
            (
                TextPosition {
                    byte: 1,
                    affinity: Affinity::Upstream,
                },
                Point(10., 0.),
            ),
            (
                TextPosition {
                    byte: 3,
                    affinity: Affinity::Downstream,
                },
                Point(20., 0.),
            ),
            (
                TextPosition {
                    byte: 1,
                    affinity: Affinity::Downstream,
                },
                Point(30., 0.),
            ),
            (
                TextPosition {
                    byte: 5,
                    affinity: Affinity::Upstream,
                },
                Point(40., 0.),
            ),
        ],
    })
}
#[test]
fn paint_hit_and_visual_navigation_share_one_layout_lease() {
    let mut editor = Editor::new("aאב");
    let lease = fixture(editor.version());
    assert!(editor.install(lease.clone()).is_ok());
    assert!(editor.hit(30.));
    assert_eq!(
        editor.cursor,
        TextPosition {
            byte: 1,
            affinity: Affinity::Downstream
        }
    );
    assert!(editor.move_visual(false));
    assert_eq!(editor.cursor.byte, 3);
    let paints = Cell::new(0);
    assert!(editor.paint(&mut |drawn| {
        assert!(Rc::ptr_eq(drawn, &lease));
        paints.set(paints.get() + 1);
    }));
    assert_eq!(paints.get(), 1);
}
#[test]
fn edits_fonts_and_width_reject_stale_layout_before_paint_or_input() {
    let mut editor = Editor::new("abc");
    let lease = fixture(editor.version());
    assert!(editor.install(lease.clone()).is_ok());
    editor.document.replace(0..1, "long").unwrap();
    assert!(!editor.paint(&mut |_| panic!("stale paint")));
    assert!(!editor.hit(10.));
    assert!(editor.install(lease).is_err());
    let font_stale = fixture(editor.version());
    editor.font += 1;
    assert!(editor.install(font_stale).is_err());
    let width_stale = fixture(editor.version());
    editor.width = 200.;
    assert!(editor.install(width_stale).is_err());
}
#[test]
fn composition_is_uncommitted_and_blur_invalidates_queued_commit() {
    let mut editor = Editor::new("abc");
    let session = editor.session;
    editor.compose(session, "candidate");
    assert_eq!(editor.document.text, "abc");
    assert_eq!(editor.document.revision, 0);
    editor.blur();
    assert!(editor.preedit.is_empty());
    assert!(!editor.commit(session, "old"));
    assert_eq!(editor.document.text, "abc");
    assert!(editor.commit(editor.session, "x"));
    assert_eq!(editor.document.text, "xabc");
    assert_eq!(editor.document.revision, 1);
    assert!(editor.document.replace(100..101, "bad").is_err());
    assert_eq!(editor.document.revision, 1);
}
