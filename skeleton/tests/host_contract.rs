//! Host-side contract experiments. No GPU, OS event source, or shaping engine is simulated as proven.
use fire_ui_contract_model::*;
use std::{cell::Cell, rc::Rc};
#[derive(Clone, Copy, Debug, PartialEq)]
struct MeasureKey {
    width: u32,
    content: u64,
    font: u64,
}
struct ParagraphCache {
    key: Option<MeasureKey>,
    measurements: usize,
    lines: usize,
}
impl ParagraphCache {
    fn measure(&mut self, key: MeasureKey, text: &str) -> usize {
        if self.key != Some(key) {
            // Monospace service stand-in. The experiment concerns ownership/invalidation only.
            self.lines = text.chars().count().div_ceil(key.width.max(1) as usize);
            self.key = Some(key);
            self.measurements += 1;
        }
        self.lines
    }
}
#[derive(Default)]
struct Host {
    pending_size: Option<(u32, u32)>,
    size: (u32, u32),
    dirty: bool,
    exposed: bool,
}
impl Host {
    fn resize(&mut self, size: (u32, u32)) {
        self.pending_size = Some(size);
    }
    fn prepare(&mut self, cache: &mut ParagraphCache, content: u64, font: u64, text: &str) -> bool {
        if let Some(size) = self.pending_size.take() {
            self.dirty |= size != self.size;
            self.size = size;
        }
        if self.size.0 == 0 || self.size.1 == 0 {
            return false;
        }
        if !self.dirty && !self.exposed {
            return false;
        }
        cache.measure(
            MeasureKey {
                width: self.size.0,
                content,
                font,
            },
            text,
        );
        self.dirty = false;
        self.exposed = false;
        true
    }
}
#[test]
fn resize_burst_uses_final_width_and_height_only_changes_reuse_text() {
    let mut cache = ParagraphCache {
        key: None,
        measurements: 0,
        lines: 0,
    };
    let mut host = Host::default();
    for width in 1..=100 {
        host.resize((width, 50));
    }
    assert!(host.prepare(&mut cache, 1, 1, &"a".repeat(200)));
    assert_eq!(cache.measurements, 1);
    assert_eq!(cache.lines, 2);
    host.resize((100, 10));
    assert!(host.prepare(&mut cache, 1, 1, &"a".repeat(200)));
    assert_eq!(cache.measurements, 1);
    host.resize((25, 10));
    assert!(host.prepare(&mut cache, 1, 1, &"a".repeat(200)));
    assert_eq!(cache.measurements, 2);
    assert_eq!(cache.lines, 8);
    assert!(!host.prepare(&mut cache, 1, 1, "unused"));
    host.exposed = true;
    assert!(host.prepare(&mut cache, 1, 1, &"a".repeat(200)));
    assert_eq!(cache.measurements, 2);
    host.resize((0, 0));
    assert!(!host.prepare(&mut cache, 1, 1, "unused"));
    host.resize((25, 10));
    assert!(host.prepare(&mut cache, 1, 1, &"a".repeat(200)));
}
struct Resource(Rc<Cell<usize>>);
impl Drop for Resource {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1)
    }
}
struct ResourceWidget {
    lease: Rc<Resource>,
}
impl Widget for ResourceWidget {
    fn mount(&mut self, _: &mut Cx<'_>) {
        assert_eq!(self.lease.0.get(), 0)
    }
}
struct Frame {
    leases: Vec<Rc<Resource>>,
}
impl Frame {
    fn submit(leases: Vec<Rc<Resource>>) -> Self {
        Self { leases }
    }
    fn complete(self) {
        drop(self.leases)
    }
}
#[test]
fn embedding_host_retains_resource_until_frame_completion_after_widget_removal() {
    let drops = Rc::new(Cell::new(0));
    let lease = Rc::new(Resource(drops.clone()));
    let mut ui = Ui::new(20, 4);
    let k = ui
        .insert(
            None,
            0,
            ResourceWidget {
                lease: lease.clone(),
            },
        )
        .unwrap();
    ui.pump(4);
    let frame = Frame::submit(vec![lease.clone()]);
    drop(lease);
    ui.remove(k).unwrap();
    assert_eq!(drops.get(), 0);
    frame.complete();
    assert_eq!(drops.get(), 1);
    assert!(ui.next_work().idle());
}
