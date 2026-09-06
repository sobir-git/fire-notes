//! Reproduce an API-design cost in C7; this is not a native performance benchmark.
use fire_ui_contract_model::*;
use std::{cell::Cell, rc::Rc};

struct Animated(Rc<Cell<usize>>);
impl Widget for Animated {
    fn event(&mut self, _: &mut Cx<'_>, _: Phase, input: &Input) {
        if matches!(input, Input::Frame { .. }) {
            self.0.set(self.0.get() + 1);
        }
    }
}

fn main() {
    let mut ui = Ui::new(1, 8);
    let callbacks = Rc::new(Cell::new(0));
    let widget = ui.insert(None, 0, Animated(callbacks.clone())).unwrap();
    ui.pump(8);

    // Two independent helpers in one widget each need its next frame.
    for slot in [0, 1] {
        ui.schedule(widget, slot, Some(Wake::Frame), Lifetime::Visible)
            .unwrap();
    }
    ui.frame(16);
    println!(
        "C7: two frame slots in one widget produce {} callbacks at one presentation opportunity",
        callbacks.get()
    );

    // Contrast independent timer slots, which do represent distinct callbacks.
    ui.schedule(widget, 0, Some(Wake::At(20)), Lifetime::Mounted)
        .unwrap();
    ui.schedule(widget, 1, Some(Wake::At(30)), Lifetime::Mounted)
        .unwrap();
    println!(
        "Independent timers still report {:?}",
        ui.next_work().deadline
    );
}
