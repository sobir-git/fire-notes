//! Layout algebra only. Borrowed measurements do not own a second widget tree.
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}
impl Size {
    pub fn new(width: f32, height: f32) -> Self {
        assert!(width.is_finite() && width >= 0.0 && height.is_finite() && height >= 0.0);
        Self { width, height }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub size: Size,
    pub baseline: Option<f32>,
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub width: f32,
    pub height: Option<f32>,
}
/// Open measurement protocol. Native integration will expose this through LayoutCx.
pub trait Measure {
    fn measure(&self, limits: Limits) -> Metrics;
}
impl<F: Fn(Limits) -> Metrics> Measure for F {
    fn measure(&self, limits: Limits) -> Metrics {
        self(limits)
    }
}
/// Width is a property of this placement, not of the reusable child widget.
#[derive(Clone, Copy, Debug)]
pub enum Width {
    Natural,
    Fixed(f32),
    Fill,
}
pub struct ChildLayout<'a> {
    pub content: &'a dyn Measure,
    pub width: Width,
}
#[derive(Debug)]
pub struct Placement {
    pub x: f32,
    pub y: f32,
    pub metrics: Metrics,
}
#[derive(Debug)]
pub struct Layout {
    pub metrics: Metrics,
    pub children: Vec<Placement>,
}

/// Intrinsic column. Overflow is retained; a viewport decides what to clip.
pub fn column(children: &[ChildLayout<'_>], width: f32, gap: f32) -> Layout {
    Size::new(width, gap);
    let mut y: f32 = 0.0;
    let mut widest: f32 = 0.0;
    let mut placements = Vec::with_capacity(children.len());
    for child in children {
        let available = match child.width {
            Width::Fixed(w) => {
                Size::new(w, 0.0);
                w.min(width)
            }
            _ => width,
        };
        let mut metrics = child.content.measure(Limits {
            width: available,
            height: None,
        });
        if !matches!(child.width, Width::Natural) {
            metrics.size.width = available;
        }
        widest = widest.max(metrics.size.width);
        placements.push(Placement { x: 0.0, y, metrics });
        y += metrics.size.height + gap;
    }
    let baseline = placements.first().and_then(|p| p.metrics.baseline);
    Layout {
        metrics: Metrics {
            size: Size::new(widest, if children.is_empty() { 0.0 } else { y - gap }),
            baseline,
        },
        children: placements,
    }
}

/// Align already measured children on their text baseline; baseline-free content uses its bottom.
pub fn baseline_row(children: &[Metrics], gap: f32) -> Layout {
    Size::new(gap, 0.0);
    let above = children
        .iter()
        .map(|m| m.baseline.unwrap_or(m.size.height))
        .fold(0.0, f32::max);
    let below = children
        .iter()
        .map(|m| m.size.height - m.baseline.unwrap_or(m.size.height))
        .fold(0.0, f32::max);
    let mut x = 0.0;
    let placements = children
        .iter()
        .map(|&metrics| {
            let placement = Placement {
                x,
                y: above - metrics.baseline.unwrap_or(metrics.size.height),
                metrics,
            };
            x += metrics.size.width + gap;
            placement
        })
        .collect();
    Layout {
        metrics: Metrics {
            size: Size::new(
                if children.is_empty() { 0.0 } else { x - gap },
                above + below,
            ),
            baseline: (!children.is_empty()).then_some(above),
        },
        children: placements,
    }
}

pub fn padded(content: &dyn Measure, limits: Limits, inset: f32) -> Metrics {
    Size::new(inset, 0.0);
    let inner = content.measure(Limits {
        width: (limits.width - inset * 2.0).max(0.0),
        height: limits.height.map(|h| (h - inset * 2.0).max(0.0)),
    });
    // The actual inset shrinks under tight constraints, so intrinsic decoration respects max size.
    let horizontal = inset.min(limits.width / 2.0);
    let vertical = limits.height.map_or(inset, |h| inset.min(h / 2.0));
    Metrics {
        size: Size::new(
            inner.size.width + horizontal * 2.0,
            inner.size.height + vertical * 2.0,
        ),
        baseline: inner.baseline.map(|b| b + vertical),
    }
}
#[derive(Debug)]
pub struct ScrollLayout {
    pub viewport: Size,
    pub content: Metrics,
    pub max_offset: f32,
}
/// A normal vertical scroller measures its content at viewport width with unbounded height.
pub fn vertical_scroll(content: &dyn Measure, viewport: Size) -> ScrollLayout {
    let content = content.measure(Limits {
        width: viewport.width,
        height: None,
    });
    ScrollLayout {
        viewport,
        max_offset: (content.size.height - viewport.height).max(0.0),
        content,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color(pub u8, pub u8, pub u8);
#[derive(Clone, Debug)]
pub struct Theme {
    pub text_size: f32,
    pub control_inset: f32,
    pub foreground: Color,
    pub accent: Color,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Overrides {
    pub text_size: Option<f32>,
    pub foreground: Option<Color>,
}
/// One shared immutable theme plus narrow local values. No per-node copy of a theme table.
#[derive(Clone, Debug)]
pub struct Appearance {
    pub theme: Rc<Theme>,
    pub local: Overrides,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedAppearance {
    pub text_size: f32,
    pub control_inset: f32,
    pub foreground: Color,
    pub accent: Color,
}
impl Appearance {
    pub fn resolve(&self) -> ResolvedAppearance {
        ResolvedAppearance {
            text_size: self.local.text_size.unwrap_or(self.theme.text_size),
            control_inset: self.theme.control_inset,
            foreground: self.local.foreground.unwrap_or(self.theme.foreground),
            accent: self.theme.accent,
        }
    }
}
impl ResolvedAppearance {
    pub fn requires_layout(self, previous: Self) -> bool {
        self.text_size != previous.text_size || self.control_inset != previous.control_inset
    }
}
