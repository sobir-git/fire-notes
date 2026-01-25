//! Core UI types and enums

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UiNode {
    None,
    Tab(usize),
    NewTabButton,
    Scrollbar,
    TextArea,
    TabBar,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UiHover {
    pub tab_index: Option<usize>,
    pub plus: bool,
    pub scrollbar: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum UiAction {
    None,
    ActivateTab(usize),
    NewTab,
    StartScrollbarDrag { drag_offset: f32 },
    ScrollbarJump { ratio: f32 },
    TextClick,
    TabBarClick,
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
