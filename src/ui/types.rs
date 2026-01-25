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

impl Rect {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }
}
