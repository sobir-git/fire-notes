use fire_ui::*;
use fire_ui_widgets::{EditorDecoration, EditorLayer, EditorView};
use std::time::Duration;
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    size: f32,
    phase: f32,
    behind: bool,
}
/// The app's fire treatment is an ordinary editor decoration, with a bounded particle budget.
pub struct Flames {
    particles: Vec<Particle>,
    recent: Vec<(Rect, Duration)>,
    seed: u64,
    time: f32,
}
impl Default for Flames {
    fn default() -> Self {
        Self {
            particles: Vec::with_capacity(500),
            recent: Vec::with_capacity(64),
            seed: 0xfeed1234,
            time: 0.,
        }
    }
}
impl Flames {
    fn random(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 7;
        self.seed ^= self.seed << 17;
        (self.seed as u32) as f32 / u32::MAX as f32
    }
    fn rects(view: &EditorView<'_>) -> Vec<Rect> {
        view.selection
            .clone()
            .map(|r| {
                view.paragraph
                    .selection_in(r, view.viewport)
                    .into_iter()
                    .filter(|r| {
                        r.intersect(view.viewport).width > 0.
                            && r.intersect(view.viewport).height > 0.
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    fn recolor(view: &EditorView<'_>, rect: Rect, color: Color, p: &mut dyn Painter) {
        p.save();
        p.clip(rect);
        p.paragraph(view.paragraph, Point::default(), color.into());
        p.restore();
    }
}
impl EditorDecoration for Flames {
    fn damage(&self, view: &EditorView<'_>) -> Option<Rect> {
        let mut rects = Self::rects(view);
        rects.extend(self.recent.iter().map(|(r, _)| *r));
        rects.extend(self.particles.iter().map(|p| {
            Rect::new(
                p.x - p.size - 2.,
                p.y - p.size - 2.,
                p.size * 2. + 4.,
                p.size * 2. + 4.,
            )
        }));
        Some(rects.into_iter().reduce(Rect::union).unwrap_or_default())
    }
    fn frame(&mut self, view: &EditorView<'_>, time: FrameTime, edited: bool) -> bool {
        self.time = time.now.as_secs_f32();
        let dt = time.elapsed.as_secs_f32().min(0.05);
        if !view.focused {
            self.particles.clear();
            self.recent.clear();
            return false;
        }
        if edited {
            let point = view.paragraph.caret_point(view.caret);
            if point.x > 0. {
                self.recent.push((
                    Rect::new(
                        (point.x - 9.63).max(0.),
                        point.y,
                        9.63,
                        view.paragraph.line_height,
                    ),
                    time.now,
                ));
                if self.recent.len() > 64 {
                    self.recent.remove(0);
                }
            }
        }
        self.recent
            .retain(|(_, at)| time.now.saturating_sub(*at) < Duration::from_secs(1));
        self.particles.retain_mut(|p| {
            p.life -= dt;
            p.x += (p.vx + (self.time * 3. + p.phase).sin() * 8.) * dt;
            p.y -= p.vy * dt;
            p.vy += 15. * dt;
            p.life > 0.
        });
        let mut sources = Self::rects(view);
        sources.extend(self.recent.iter().map(|(r, _)| *r));
        if !sources.is_empty() {
            for _ in 0..15 {
                if self.particles.len() >= 500 {
                    break;
                }
                let index = (self.random() * sources.len() as f32) as usize % sources.len();
                let r = sources[index];
                if self.random() > 0.4 {
                    continue;
                }
                let x = r.x + self.random() * r.width;
                let y = r.y + r.height * 0.5 + self.random() * 5.;
                let vx = self.random() * 24. - 12.;
                let vy = 30. + self.random() * 25.;
                let life = 0.4 + self.random() * 0.3;
                let size = 2.5 + self.random() * 2.;
                let phase = self.random() * std::f32::consts::TAU;
                let behind = self.random() < 0.7;
                self.particles.push(Particle {
                    x,
                    y,
                    vx,
                    vy,
                    life,
                    size,
                    phase,
                    behind,
                });
            }
        }
        !sources.is_empty() || !self.particles.is_empty()
    }
    fn paint(&self, view: &EditorView<'_>, layer: EditorLayer, p: &mut dyn Painter) {
        if !view.focused {
            return;
        }
        if layer == EditorLayer::AboveText {
            for r in Self::rects(view) {
                let cycle =
                    ((self.time * 2.5 + r.x * 0.1 + r.y * 0.07).sin() * 0.5 + 0.5).clamp(0., 1.);
                Self::recolor(
                    view,
                    r,
                    Color(0.9 + cycle * 0.1, 0.15 + cycle * 0.25, cycle * 0.05, 1.),
                    p,
                );
            }
            for (r, at) in &self.recent {
                let age = (self.time - at.as_secs_f32()).clamp(0., 1.);
                Self::recolor(view, *r, Color(1., 0.3 + age * 0.7, age, 1.), p);
            }
        }
        for particle in &self.particles {
            if particle.behind != (layer == EditorLayer::BehindText) {
                continue;
            }
            let life = (particle.life / 0.7).clamp(0., 1.);
            let (r, g, b) = if life > 0.7 {
                (1., 0.75, 0.15)
            } else if life > 0.4 {
                (0.95, 0.45, 0.05)
            } else if life > 0.15 {
                (0.7, 0.15, 0.)
            } else {
                (0.3, 0.05, 0.)
            };
            let radius = particle.size * (0.6 + life * 0.4);
            p.rect(
                Rect::new(
                    particle.x - radius,
                    particle.y - radius,
                    radius * 2.,
                    radius * 2.,
                ),
                radius,
                Color(r, g, b, life * 0.18).into(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fire_is_bounded_and_stops_after_typing_or_focus_loss() {
        let paragraph = TestText.layout(TextRequest {
            text: "Fire Notes".into(),
            style: TextStyle::default(),
            width: None,
            revision: 0,
        });
        let mut view = EditorView {
            paragraph: &paragraph,
            caret: Caret::at(10),
            selection: Some(0..10),
            viewport: Rect::new(0., 0., 500., 300.),
            focused: true,
        };
        let mut fire = Flames::default();
        for i in 0..1000 {
            assert!(fire.frame(
                &view,
                FrameTime {
                    now: Duration::from_millis(i * 16),
                    elapsed: Duration::from_millis(16)
                },
                false
            ));
            assert!(fire.particles.len() <= 500);
        }
        assert!(!fire.particles.is_empty());
        view.selection = None;
        for i in 1000..1100 {
            fire.frame(
                &view,
                FrameTime {
                    now: Duration::from_millis(i * 16),
                    elapsed: Duration::from_millis(16),
                },
                false,
            );
        }
        assert!(fire.particles.is_empty());
        assert!(!fire.frame(
            &view,
            FrameTime {
                now: Duration::from_secs(20),
                elapsed: Duration::from_millis(16)
            },
            false
        ));
        assert!(fire.frame(
            &view,
            FrameTime {
                now: Duration::from_secs(21),
                elapsed: Duration::from_millis(16)
            },
            true
        ));
        view.focused = false;
        assert!(!fire.frame(
            &view,
            FrameTime {
                now: Duration::from_secs(22),
                elapsed: Duration::from_millis(16)
            },
            false
        ));
        assert!(fire.particles.is_empty() && fire.recent.is_empty());
    }
}
