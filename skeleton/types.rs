use std::any::Any;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key {
    pub ui: u64,
    pub slot: usize,
    pub generation: u64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point(pub f64, pub f64);
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect(pub f64, pub f64, pub f64, pub f64);
impl Rect {
    pub fn contains(self, p: Point) -> bool {
        p.0 >= self.0 && p.1 >= self.1 && p.0 < self.0 + self.2 && p.1 < self.1 + self.3
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine(pub [f64; 6]);
impl Affine {
    pub const IDENTITY: Self = Self([1., 0., 0., 1., 0., 0.]);
    pub fn point(self, p: Point) -> Point {
        let m = self.0;
        Point(
            m[0] * p.0 + m[2] * p.1 + m[4],
            m[1] * p.0 + m[3] * p.1 + m[5],
        )
    }
    pub fn then(self, parent: Self) -> Self {
        let a = parent.0;
        let b = self.0;
        Self([
            a[0] * b[0] + a[2] * b[1],
            a[1] * b[0] + a[3] * b[1],
            a[0] * b[2] + a[2] * b[3],
            a[1] * b[2] + a[3] * b[3],
            a[0] * b[4] + a[2] * b[5] + a[4],
            a[1] * b[4] + a[3] * b[5] + a[5],
        ])
    }
    pub fn inverse(self) -> Option<Self> {
        let m = self.0;
        let det = m[0] * m[3] - m[1] * m[2];
        if !det.is_finite() || det.abs() < 1e-12 || m.iter().any(|x| !x.is_finite()) {
            return None;
        }
        let a = m[3] / det;
        let b = -m[1] / det;
        let c = -m[2] / det;
        let d = m[0] / det;
        Some(Self([
            a,
            b,
            c,
            d,
            -a * m[4] - c * m[5],
            -b * m[4] - d * m[5],
        ]))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Preview,
    Target,
    Bubble,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Input {
    Move {
        pointer: u32,
        position: Point,
    },
    Button {
        pointer: u32,
        button: u16,
        down: bool,
        position: Point,
    },
    Key {
        physical: u32,
        logical: String,
        down: bool,
        repeat: bool,
    },
    Text {
        session: u64,
        text: String,
    },
    AnchorUnavailable {
        overlay: Key,
        anchor: Key,
    },
    CaptureLost {
        pointer: u32,
    },
    CancelKeys,
    Focus(bool),
    Hidden(bool),
    Unmount,
    Committed {
        token: u16,
        result: Result<Key, Error>,
    },
    Rejected(Error),
    Wake(u16),
    Frame {
        now: u64,
        elapsed: u64,
    },
}
impl Input {
    pub fn position(&self) -> Option<Point> {
        match self {
            Self::Move { position, .. } | Self::Button { position, .. } => Some(*position),
            _ => None,
        }
    }
    pub fn pointer(&self) -> Option<u32> {
        match self {
            Self::Move { pointer, .. } | Self::Button { pointer, .. } => Some(*pointer),
            _ => None,
        }
    }
    pub fn local(&self, inverse: Affine) -> Self {
        match self {
            Self::Move { pointer, position } => Self::Move {
                pointer: *pointer,
                position: inverse.point(*position),
            },
            Self::Button {
                pointer,
                button,
                down,
                position,
            } => Self::Button {
                pointer: *pointer,
                button: *button,
                down: *down,
                position: inverse.point(*position),
            },
            _ => self.clone(),
        }
    }
}
pub struct Action {
    pub source: Key,
    pub payload: Box<dyn Any>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lifetime {
    Visible,
    Mounted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wake {
    At(u64),
    Frame,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub owner: Key,
    pub slot: u16,
    pub epoch: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Stale,
    Full,
    InvalidOwner,
    DuplicateToken,
    InvalidGeometry,
    Unhandled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Work {
    pub ready: bool,
    pub frame: bool,
    pub deadline: Option<u64>,
}
impl Work {
    pub fn idle(self) -> bool {
        !self.ready && !self.frame && self.deadline.is_none()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Affinity {
    Upstream,
    Downstream,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextPosition {
    pub byte: usize,
    pub affinity: Affinity,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayoutVersion {
    pub document: u64,
    pub font: u64,
    pub width_bits: u64,
}
pub struct Paragraph {
    pub version: LayoutVersion,
    pub visual_stops: Vec<(TextPosition, Point)>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub transform: Affine,
    pub bounds: Rect,
    pub clip: bool,
    pub overlay_anchor: Option<Key>,
}
impl Default for Placement {
    fn default() -> Self {
        Self {
            transform: Affine::IDENTITY,
            bounds: Rect(0., 0., 100., 100.),
            clip: true,
            overlay_anchor: None,
        }
    }
}
