//! Widget Demo — interactive testbed for fire-notes primitives.
//!
//! All coordinates are in **logical pixels**. Scale is handled by the framework.
//!
//! Run with: `cargo run --bin widget-demo`

use fire_notes::layout::Node;
use fire_notes::layout::node::Color as NColor;
use fire_notes::primitives::{TextInput, text_input::{EventOutcome, TextInputEvent}};
use fire_notes::runtime::{App, Runtime, ViewCtx};
use fire_notes::theme::Theme;
use fire_notes::ui::Rect;

const FIELDS:  usize = 3;
const PAD:     f32   = 20.0;
const FIELD_H: f32   = 36.0;
const GAP:     f32   = 16.0;

const ID: [u64; FIELDS] = [1, 2, 3];

struct Demo { inputs: [TextInput; FIELDS] }

impl Demo {
    fn new() -> Self {
        Self { inputs: [
            TextInput::new(""),
            TextInput::new("Pre-filled text — try editing me"),
            TextInput::new("A very long line that will scroll: abcdefghijklmnopqrstuvwxyz ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789"),
        ]}
    }

    fn input_rect(i: usize, w: f32) -> Rect {
        let y = PAD + (FIELD_H + GAP) * (i as f32 + 1.5);
        Rect { x: PAD, y, width: w - PAD * 2.0, height: FIELD_H }
    }

    fn focused_index(&self, ctx: &ViewCtx) -> usize {
        ID.iter().position(|id| ctx.is_focused(*id)).unwrap_or(0)
    }
}

impl App for Demo {
    fn view(&mut self, ctx: &mut ViewCtx) -> Node {
        let w = ctx.window.width;
        let h = ctx.window.height;
        let theme = Theme::dark();
        let bg = NColor::rgb(theme.bg.0, theme.bg.1, theme.bg.2);

        let mut children: Vec<Node> = vec![
            ctx.background(bg),
            ctx.text("Widget Demo  (Tab = next field  ·  Ctrl+A/C/X/V  ·  Esc = clear selection)",
                13.0, NColor::rgba(0.7, 0.7, 0.7, 1.0), PAD + 13.0, 0.0),
        ];

        let labels       = ["Empty input:", "Pre-filled:", "Long / scroll:"];
        let placeholders = ["Type here...", "", ""];
        for i in 0..FIELDS {
            let rect    = Self::input_rect(i, w);
            let focused = ctx.is_focused(ID[i]);
            let lc = if focused { NColor::rgba(0.4, 0.7, 1.0, 1.0) } else { NColor::rgba(0.5, 0.5, 0.5, 1.0) };
            children.push(ctx.text(labels[i], 12.0, lc, rect.y - 4.0, 0.0));
            children.push(self.inputs[i].view_with_ctx(rect, ID[i], ctx, placeholders[i]));
        }

        let fi  = &self.inputs[self.focused_index(ctx)];
        let col = fi.text()[..fi.cursor()].chars().count();
        let sel = if fi.has_selection() {
            format!("  |  sel {} chars", fi.text().chars().count().saturating_sub(col))
        } else { String::new() };
        children.push(ctx.text(
            format!("col: {col}   len: {}{sel}", fi.text().chars().count()),
            12.0, NColor::rgba(0.5, 0.5, 0.5, 1.0), h - PAD, 0.0,
        ));

        Node::layer(children)
    }

    fn handle_key(&mut self, id: u64, event: TextInputEvent<'_>, ctx: &ViewCtx) -> EventOutcome {
        let i    = ID.iter().position(|&x| x == id).unwrap_or(0);
        let rect = ctx.rect_for(id).unwrap_or(Rect { x: 0.0, y: 0.0, width: 9999.0, height: FIELD_H });
        self.inputs[i].handle_key_ctx(event, rect, ctx)
    }

    fn handle_paste(&mut self, id: u64, text: &str, ctx: &ViewCtx) {
        let i    = ID.iter().position(|&x| x == id).unwrap_or(0);
        let rect = ctx.rect_for(id).unwrap_or(Rect { x: 0.0, y: 0.0, width: 9999.0, height: FIELD_H });
        self.inputs[i].paste(text);
        self.inputs[i].ensure_visible_ctx(rect, ctx);
    }

    fn handle_pointer_down(&mut self, x: f32, y: f32, ctx: &ViewCtx) -> Option<u64> {
        for i in 0..FIELDS {
            if let Some(rect) = ctx.rect_for(ID[i]) {
                if self.inputs[i].on_pointer_down_ctx(x, y, rect, ctx) {
                    return Some(ID[i]);
                }
            }
        }
        None
    }

    fn focus_order(&self) -> Vec<u64> { ID.to_vec() }
}

fn main() {
    Runtime::new()
        .title("fire-notes widget demo")
        .size(720.0, 400.0)
        .run(Demo::new());
}
