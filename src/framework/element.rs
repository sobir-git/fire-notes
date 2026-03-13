#![allow(dead_code)]
//! `Element` — the declarative description of a widget with callbacks attached.
//!
//! An `Element<Msg>` is pure data. It describes:
//!   - what kind of widget this is (`ElementKind`)
//!   - what children it has
//!   - what callbacks fire on user interaction (each returns `Msg`)
//!
//! `Msg` is a component-defined enum. The framework calls `build()` to get the
//! tree, runs layout to assign rects, then on events fires callbacks and returns
//! `Msg` values. The component's `update(msg)` applies the mutation.
//!
//! Elements carry no `Rect` — geometry is assigned by the layout engine.

use crate::primitives::text_input::TextInput;
use crate::ui::Rect;

// ── Sizing ─────────────────────────────────────────────────────────────────────

/// How an element sizes itself in its parent's layout pass.
#[derive(Debug, Clone, Copy)]
pub enum Size {
    /// Fixed logical pixels (before scale). Corresponds to negative Column weights.
    Fixed(f32),
    /// Fills remaining space proportionally. Corresponds to positive Column weights.
    Fill(f32),
}

impl Default for Size { fn default() -> Self { Size::Fill(1.0) } }

// ── Callbacks ─────────────────────────────────────────────────────────────────

/// A callback that fires when the element produces a specific interaction.
/// Returns a `Msg` that the component's `update()` will handle.
pub struct Callback<Msg>(pub Box<dyn Fn() -> Msg + 'static>);

impl<Msg> Callback<Msg> {
    pub fn new(f: impl Fn() -> Msg + 'static) -> Self { Self(Box::new(f)) }
    pub fn call(&self) -> Msg { (self.0)() }
}

impl<Msg> std::fmt::Debug for Callback<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Callback<_>")
    }
}

/// A callback that carries a string argument (text change, paste).
pub struct StringCallback<Msg>(pub Box<dyn Fn(String) -> Msg + 'static>);

impl<Msg> StringCallback<Msg> {
    pub fn new(f: impl Fn(String) -> Msg + 'static) -> Self { Self(Box::new(f)) }
    pub fn call(&self, s: String) -> Msg { (self.0)(s) }
}

impl<Msg> std::fmt::Debug for StringCallback<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StringCallback<_>")
    }
}

/// A callback that carries a `usize` index argument (list confirm with selected index).
pub struct IndexCallback<Msg>(pub Box<dyn Fn(usize) -> Msg + 'static>);

impl<Msg> IndexCallback<Msg> {
    pub fn new(f: impl Fn(usize) -> Msg + 'static) -> Self { Self(Box::new(f)) }
    pub fn call(&self, i: usize) -> Msg { (self.0)(i) }
}

impl<Msg> std::fmt::Debug for IndexCallback<Msg> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IndexCallback<_>")
    }
}

// ── ElementKind ───────────────────────────────────────────────────────────────

/// What kind of widget an `Element` represents.
#[derive(Debug)]
pub enum ElementKind<Msg> {
    /// A single-line text editor. Carries snapshot of `TextInput` state for rendering.
    TextInput {
        state:       TextInput,
        placeholder: String,
        auto_focus:  bool,
        on_change:   Option<StringCallback<Msg>>,
        on_confirm:  Option<Callback<Msg>>,
        on_cancel:   Option<Callback<Msg>>,
    },

    /// A scrollable filterable list.
    List {
        items:      Vec<ListItem>,
        selected:   usize,
        on_confirm: Option<IndexCallback<Msg>>,
        on_cancel:  Option<Callback<Msg>>,
    },

    /// A backdrop: closes on click-outside, contains children.
    Backdrop {
        on_click_outside: Option<Callback<Msg>>,
    },

    /// A vertical stack of children with weighted sizing.
    Column,

    /// A horizontal stack of children with weighted sizing.
    Row,
}

// ── ListItem ──────────────────────────────────────────────────────────────────

/// A single item in a `List` element.
#[derive(Debug, Clone)]
pub struct ListItem {
    pub label:    String,
    pub marked:   bool,  // e.g. "currently open" indicator
}

// ── Element ───────────────────────────────────────────────────────────────────

/// A node in the declarative element tree.
///
/// Created by component `build()` methods. The framework assigns `rect` during
/// the layout pass — elements carry no rect at construction time.
#[derive(Debug)]
pub struct Element<Msg> {
    pub kind:     ElementKind<Msg>,
    pub children: Vec<Element<Msg>>,
    pub size:     Size,

    // ── Assigned by layout pass ───────────────────────────────────────────────
    /// Physical rect in window coordinates. Zero until `layout_tree()` is called.
    pub rect:     Rect,
}

impl<Msg: 'static> Element<Msg> {
    pub fn new(kind: ElementKind<Msg>) -> Self {
        Self { kind, children: Vec::new(), size: Size::default(), rect: Rect::ZERO }
    }

    pub fn with_size(mut self, size: Size) -> Self { self.size = size; self }

    pub fn child(mut self, child: Element<Msg>) -> Self {
        self.children.push(child);
        self
    }
}

// ── Builder helpers ───────────────────────────────────────────────────────────

/// Build a `TextInput` element from a `TextInput` state snapshot.
pub fn text_input_el<Msg: 'static>(state: &TextInput, placeholder: impl Into<String>) -> TextInputBuilder<Msg> {
    TextInputBuilder {
        state:      state.clone(),
        placeholder: placeholder.into(),
        auto_focus: false,
        on_change:  None,
        on_confirm: None,
        on_cancel:  None,
        size:       Size::default(),
    }
}

pub struct TextInputBuilder<Msg> {
    state:       TextInput,
    placeholder: String,
    auto_focus:  bool,
    on_change:   Option<StringCallback<Msg>>,
    on_confirm:  Option<Callback<Msg>>,
    on_cancel:   Option<Callback<Msg>>,
    size:        Size,
}

impl<Msg: 'static> TextInputBuilder<Msg> {
    pub fn auto_focus(mut self, v: bool) -> Self { self.auto_focus = v; self }
    pub fn size(mut self, s: Size) -> Self { self.size = s; self }

    pub fn on_change(mut self, f: impl Fn(String) -> Msg + 'static) -> Self {
        self.on_change = Some(StringCallback::new(f)); self
    }
    pub fn on_confirm(mut self, f: impl Fn() -> Msg + 'static) -> Self {
        self.on_confirm = Some(Callback::new(f)); self
    }
    pub fn on_cancel(mut self, f: impl Fn() -> Msg + 'static) -> Self {
        self.on_cancel = Some(Callback::new(f)); self
    }

    pub fn build(self) -> Element<Msg> {
        Element::new(ElementKind::TextInput {
            state:       self.state,
            placeholder: self.placeholder,
            auto_focus:  self.auto_focus,
            on_change:   self.on_change,
            on_confirm:  self.on_confirm,
            on_cancel:   self.on_cancel,
        }).with_size(self.size)
    }
}

/// Build a `List` element.
pub fn list_el<Msg: 'static>(items: Vec<ListItem>) -> ListBuilder<Msg> {
    ListBuilder { items, on_confirm: None, on_cancel: None, size: Size::default() }
}

pub struct ListBuilder<Msg> {
    items:      Vec<ListItem>,
    on_confirm: Option<IndexCallback<Msg>>,
    on_cancel:  Option<Callback<Msg>>,
    size:       Size,
}

impl<Msg: 'static> ListBuilder<Msg> {
    pub fn size(mut self, s: Size) -> Self { self.size = s; self }

    pub fn on_confirm(mut self, f: impl Fn(usize) -> Msg + 'static) -> Self {
        self.on_confirm = Some(IndexCallback::new(f)); self
    }
    pub fn on_cancel(mut self, f: impl Fn() -> Msg + 'static) -> Self {
        self.on_cancel = Some(Callback::new(f)); self
    }

    pub fn build(self) -> Element<Msg> {
        Element::new(ElementKind::List {
            items:      self.items,
            selected:   0,
            on_confirm: self.on_confirm,
            on_cancel:  self.on_cancel,
        }).with_size(self.size)
    }
}

/// Build a `Backdrop` element with optional children.
pub fn backdrop_el<Msg: 'static>(on_click_outside: impl Fn() -> Msg + 'static) -> BackdropBuilder<Msg> {
    BackdropBuilder {
        on_click_outside: Some(Callback::new(on_click_outside)),
        children: Vec::new(),
    }
}

pub struct BackdropBuilder<Msg> {
    on_click_outside: Option<Callback<Msg>>,
    children:         Vec<Element<Msg>>,
}

impl<Msg: 'static> BackdropBuilder<Msg> {
    pub fn child(mut self, child: Element<Msg>) -> Self {
        self.children.push(child); self
    }

    pub fn build(self) -> Element<Msg> {
        let mut el = Element::new(ElementKind::Backdrop {
            on_click_outside: self.on_click_outside,
        });
        el.children = self.children;
        el
    }
}

/// Build a `Column` element with children.
pub fn column_el<Msg: 'static>() -> ColumnBuilder<Msg> {
    ColumnBuilder { children: Vec::new(), size: Size::default() }
}

pub struct ColumnBuilder<Msg> {
    children: Vec<Element<Msg>>,
    size:     Size,
}

impl<Msg: 'static> ColumnBuilder<Msg> {
    pub fn size(mut self, s: Size) -> Self { self.size = s; self }
    pub fn child(mut self, child: Element<Msg>) -> Self {
        self.children.push(child); self
    }
    pub fn build(self) -> Element<Msg> {
        let mut el = Element::new(ElementKind::Column).with_size(self.size);
        el.children = self.children;
        el
    }
}
