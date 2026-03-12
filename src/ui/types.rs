//! Core UI types and enums

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UiNode {
    None,
    Tab(usize),
    NewTabButton,
    Scrollbar,
    TextArea,
    TabBar,
    WindowMinimize,
    WindowMaximize,
    WindowClose,
    WindowResizeEdge(ResizeEdge),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UiHover {
    pub tab_index: Option<usize>,
    pub plus: bool,
    pub scrollbar: bool,
    pub window_minimize: bool,
    pub window_maximize: bool,
    pub window_close: bool,
    pub resize_edge: Option<ResizeEdge>,
}

#[derive(Debug, Clone, Copy)]
pub enum UiAction {
    None,
    ActivateTab(usize),
    NewTab,
    StartScrollbarDrag { drag_offset: f32 },
    ScrollbarJump { ratio: f32 },
    TextClick,
    WindowMinimize,
    WindowMaximize,
    WindowClose,
    WindowDrag,
    WindowResize(ResizeEdge),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeEdge {
    North,
    South,
    East,
    West,
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

#[derive(Debug, Clone, Copy)]
pub enum UiDragAction {
    None,
    ScrollbarDrag { ratio: f32 },
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// A `Rect` that represents the full OS window surface.
///
/// The constructor is `pub(crate)` — only the platform layer (`App`, `AppLogic`)
/// may create one.  This is a compile-time constraint: child widgets receive `Rect`
/// from their parent; they can never construct a `WindowRect`, so they can never
/// accidentally encode parent-level geometry knowledge.
///
/// ```text
/// WindowRect::new(w, h)   ← only App / AppLogic
///   └─ UiTree::layout(window, scale)
///         └─ TabBar::layout(rect, scale)       ← Rect, not WindowRect
///         └─ ContentArea::layout(rect, scale)  ← Rect, not WindowRect
/// ```
#[derive(Debug, Clone, Copy)]
pub struct WindowRect(Rect);

impl WindowRect {
    /// Create the root window rect.  Only callable inside `crate` — external
    /// code (including child widgets) cannot construct a `WindowRect`.
    pub(crate) fn new(width: f32, height: f32) -> Self {
        Self(Rect { x: 0.0, y: 0.0, width, height })
    }

    /// Width of the window in physical pixels (at scale 1.0 logical).
    pub fn width(self) -> f32 { self.0.width }

    /// Height of the window in physical pixels.
    pub fn height(self) -> f32 { self.0.height }
}

impl From<WindowRect> for Rect {
    fn from(w: WindowRect) -> Rect { w.0 }
}

impl Rect {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, width: 0.0, height: 0.0 };

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    /// Returns the intersection of two rects, or `Rect::ZERO` if they do not overlap.
    pub fn intersect(&self, other: &Rect) -> Rect {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        if right > x && bottom > y {
            Rect { x, y, width: right - x, height: bottom - y }
        } else {
            Rect::ZERO
        }
    }

    // ── Layout primitives ────────────────────────────────────────────────
    //
    // `cut_*` carves a fixed strip from one edge → (strip, remainder).
    // `split_h` / `split_v` subdivide proportionally + fixed in one call.
    // These are the ONLY places size arithmetic should appear.

    /// Carve `px` pixels from the **top** edge → `(strip, remainder)`.
    pub fn cut_top(self, px: f32) -> (Rect, Rect) {
        let px = px.min(self.height);
        let strip = Rect { x: self.x, y: self.y,      width: self.width, height: px };
        let rest  = Rect { x: self.x, y: self.y + px, width: self.width, height: (self.height - px).max(0.0) };
        (strip, rest)
    }

    /// Carve `px` pixels from the **bottom** edge → `(strip, remainder)`.
    #[allow(dead_code)]
    pub fn cut_bottom(self, px: f32) -> (Rect, Rect) {
        let px = px.min(self.height);
        let strip = Rect { x: self.x, y: self.y + self.height - px, width: self.width, height: px };
        let rest  = Rect { x: self.x, y: self.y, width: self.width, height: (self.height - px).max(0.0) };
        (strip, rest)
    }

    /// Carve `px` pixels from the **right** edge → `(strip, remainder)`.
    #[allow(dead_code)]
    pub fn cut_right(self, px: f32) -> (Rect, Rect) {
        let px = px.min(self.width);
        let strip = Rect { x: self.x + self.width - px, y: self.y, width: px,                          height: self.height };
        let rest  = Rect { x: self.x,                   y: self.y, width: (self.width - px).max(0.0), height: self.height };
        (strip, rest)
    }

    /// Carve `px` pixels from the **left** edge → `(strip, remainder)`.
    #[allow(dead_code)]
    pub fn cut_left(self, px: f32) -> (Rect, Rect) {
        let px = px.min(self.width);
        let strip = Rect { x: self.x,      y: self.y, width: px,                          height: self.height };
        let rest  = Rect { x: self.x + px, y: self.y, width: (self.width - px).max(0.0), height: self.height };
        (strip, rest)
    }

    /// Shrink all four edges inward by `px`.
    #[allow(dead_code)]
    pub fn inset(self, px: f32) -> Rect {
        Rect {
            x:      self.x + px,
            y:      self.y + px,
            width:  (self.width  - 2.0 * px).max(0.0),
            height: (self.height - 2.0 * px).max(0.0),
        }
    }

    /// Return a copy with a different height (top-anchored).
    #[allow(dead_code)]
    pub fn with_height(self, height: f32) -> Rect {
        Rect { height, ..self }
    }

    /// Return local coordinates of `(x, y)` relative to this rect's origin.
    #[allow(dead_code)]
    pub fn relative(self, x: f32, y: f32) -> (f32, f32) {
        (x - self.x, y - self.y)
    }

    /// Split horizontally into N columns.
    ///
    /// Each weight value:
    ///   - **positive** → proportional share of the remaining space (flex)
    ///   - **negative** → fixed pixel width (abs value)
    ///
    /// Example: `rect.split_h(&[1.0, -12.0])` → `[fill_col, 12px_right_col]`
    pub fn split_h(self, weights: &[f32]) -> Vec<Rect> {
        Self::split_axis(self.x, self.width, self.y, self.height, weights, true)
    }

    /// Split vertically into N rows.
    ///
    /// Same weight convention as `split_h`.
    ///
    /// Example: `rect.split_v(&[-40.0, 1.0])` → `[40px_top_row, fill_rest]`
    #[allow(dead_code)]
    pub fn split_v(self, weights: &[f32]) -> Vec<Rect> {
        Self::split_axis(self.y, self.height, self.x, self.width, weights, false)
    }

    fn split_axis(origin: f32, total: f32, cross_origin: f32, cross_size: f32, weights: &[f32], horizontal: bool) -> Vec<Rect> {
        // First pass: allocate fixed slots, sum fill weights.
        let fixed_total: f32 = weights.iter().map(|w| if *w < 0.0 { w.abs() } else { 0.0 }).sum();
        let fill_total:  f32 = weights.iter().map(|w| if *w > 0.0 { *w } else { 0.0 }).sum();
        let fill_space = (total - fixed_total).max(0.0);

        let mut cursor = origin;
        weights.iter().map(|w| {
            let size = if *w < 0.0 {
                w.abs().min(total)
            } else if fill_total > 0.0 {
                fill_space * (w / fill_total)
            } else {
                0.0
            };
            let rect = if horizontal {
                Rect { x: cursor, y: cross_origin, width: size, height: cross_size }
            } else {
                Rect { x: cross_origin, y: cursor, width: cross_size, height: size }
            };
            cursor += size;
            rect
        }).collect()
    }

    /// Return a rect of `(w, h)` centered within `self`.
    #[allow(dead_code)]
    pub fn centered_in(self, w: f32, h: f32) -> Rect {
        Rect {
            x:      self.x + (self.width  - w).max(0.0) / 2.0,
            y:      self.y + (self.height - h).max(0.0) / 2.0,
            width:  w.min(self.width),
            height: h.min(self.height),
        }
    }

    // ── Backward-compat aliases ───────────────────────────────────────────
    #[allow(dead_code)] #[inline] pub fn split_off_top(self, px: f32)   -> (Rect, Rect) { self.cut_top(px) }
    #[allow(dead_code)] #[inline] pub fn split_off_right(self, px: f32) -> (Rect, Rect) { self.cut_right(px) }
}
