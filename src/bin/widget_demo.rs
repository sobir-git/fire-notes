//! Widget Demo — interactive testbed for fire-notes primitives.
//!
//! Run with: `cargo run --bin widget-demo`
//!
//! Shows each primitive in a live window. Use keyboard and mouse to test
//! text input, navigation, selection, and rendering in isolation — no app
//! logic, no file loading, no persistence.
//!
//! Layout (top to bottom):
//!   ┌─ Label ──────────────────────────────────┐
//!   ├─ TextInput (normal)  ────────────────────┤
//!   ├─ TextInput (pre-filled) ─────────────────┤
//!   ├─ Scrolled (long text) ───────────────────┤
//!   └─ Status bar ─────────────────────────────┘
//!
//! Keys:
//!   Left/Right      — move cursor
//!   Ctrl+Left/Right — move word
//!   Home/End        — line start/end
//!   Shift+*         — extend selection
//!   Backspace       — delete left
//!   Delete          — delete right
//!   Tab             — switch focused field (cycles through all inputs)
//!   Escape          — clear selection in focused field

use fire_notes::layout::Node;
use fire_notes::layout::node::{BoxStyle, Color as NColor, CursorNode, SelectionNode, TextStyle};
use fire_notes::primitives::TextInput;
use fire_notes::renderer::Renderer;
use fire_notes::theme::Theme;
use fire_notes::ui::Rect;

use std::ffi::CString;
use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, PossiblyCurrentContext};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowAttributes};

// ── Demo state ────────────────────────────────────────────────────────────────

const FIELDS: usize = 3;
const PAD: f32 = 20.0;
const FIELD_H: f32 = 36.0;
const GAP: f32 = 16.0;
const CHAR_W: f32 = 8.4; // approximate; good enough without font measurement

struct DemoState {
    inputs:    [TextInput; FIELDS],
    focused:   usize,
    cursor_on: bool,
    last_blink: Instant,
    width:     f32,
    height:    f32,
    scale:     f32,
}

impl DemoState {
    fn new(width: f32, height: f32, scale: f32) -> Self {
        let mut inputs = [
            TextInput::new(String::new()),
            TextInput::new("Pre-filled text — try editing me".to_string()),
            TextInput::new("A very long line that will scroll: abcdefghijklmnopqrstuvwxyz ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789".to_string()),
        ];
        for inp in &mut inputs {
            inp.set_char_width(CHAR_W * scale);
            inp.state.move_to_end(false);
        }
        Self {
            inputs,
            focused: 0,
            cursor_on: true,
            last_blink: Instant::now(),
            width,
            height,
            scale,
        }
    }

    fn resize(&mut self, width: f32, height: f32, scale: f32) {
        self.width = width; self.height = height; self.scale = scale;
        let cw = CHAR_W * scale;
        for inp in &mut self.inputs { inp.set_char_width(cw); }
        self.relayout();
    }

    fn relayout(&mut self) {
        let s = self.scale;
        let w = self.width - PAD * 2.0 * s;
        for (i, inp) in self.inputs.iter_mut().enumerate() {
            let y = PAD * s + (FIELD_H * s + GAP * s) * (i as f32 + 1.5);
            inp.relayout(Rect { x: PAD * s, y, width: w, height: FIELD_H * s }, s);
        }
    }

    fn tick(&mut self) -> bool {
        if self.last_blink.elapsed() >= Duration::from_millis(530) {
            self.cursor_on = !self.cursor_on;
            self.last_blink = Instant::now();
            return true;
        }
        false
    }

    fn focused(&mut self) -> &mut TextInput { &mut self.inputs[self.focused] }

    fn build_node(&self) -> Node {
        let s = self.scale;
        let w = self.width;
        let h = self.height;
        let theme = Theme::dark();

        let mut children: Vec<Node> = vec![
            // Background
            Node::Box { rect: Rect { x: 0.0, y: 0.0, width: w, height: h },
                style: BoxStyle::filled(NColor::rgb(theme.bg.0, theme.bg.1, theme.bg.2)) },
        ];

        // Title
        children.push(Node::Text {
            text: "Widget Demo — TextInput testbed (Tab to switch focus, Esc to clear selection)".to_string(),
            style: TextStyle {
                font_size:  13.0 * s,
                color:      NColor::rgba(0.7, 0.7, 0.7, 1.0),
                baseline_y: PAD * s + 13.0 * s,
                clip_x:     0.0,
                scroll_x:   0.0,
            },
            clip: None,
        });

        let labels = ["Empty input:", "Pre-filled:", "Long / scroll:"];
        for (i, inp) in self.inputs.iter().enumerate() {
            let focused = i == self.focused;
            let label_y = inp.rect.y - 4.0 * s;

            // Label
            children.push(Node::Text {
                text: labels[i].to_string(),
                style: TextStyle {
                    font_size:  12.0 * s,
                    color:      if focused { NColor::rgba(0.4, 0.7, 1.0, 1.0) }
                                else       { NColor::rgba(0.5, 0.5, 0.5, 1.0) },
                    baseline_y: label_y,
                    clip_x:     0.0,
                    scroll_x:   0.0,
                },
                clip: None,
            });

            // Input box
            let border_color = if focused { NColor::rgba(0.4, 0.6, 1.0, 0.9) }
                               else       { NColor::rgba(0.3, 0.3, 0.3, 0.8) };
            children.push(Node::Box {
                rect:  inp.rect,
                style: BoxStyle::filled(NColor::rgba(0.08, 0.08, 0.10, 1.0))
                    .with_border(border_color, if focused { 1.5 } else { 1.0 })
                    .with_radius(5.0),
            });

            // Selection
            if let Some(anchor) = inp.state.selection_anchor {
                let cursor = inp.state.cursor;
                let (start, end) = (anchor.min(cursor), anchor.max(cursor));
                let sc = inp.state.text()[..start].chars().count();
                let ec = inp.state.text()[..end].chars().count();
                let cw = inp.char_width;
                let sel_x = inp.text_x + sc as f32 * cw - inp.state.scroll_offset;
                let sel_w = (ec - sc) as f32 * cw;
                let vp = 3.0 * s;
                children.push(Node::Selection(SelectionNode {
                    rect:  Rect { x: sel_x, y: inp.rect.y + vp, width: sel_w, height: inp.rect.height - vp * 2.0 },
                    color: NColor::rgba(0.39, 0.55, 0.82, 0.45),
                }));
            }

            // Text (or placeholder)
            let display = if inp.state.text().is_empty() {
                match i {
                    0 => "Type here...",
                    _ => "",
                }.to_string()
            } else {
                inp.state.text().to_string()
            };
            let text_color = if inp.state.text().is_empty() {
                NColor::rgba(0.4, 0.4, 0.4, 1.0)
            } else {
                NColor::rgb(1.0, 1.0, 1.0)
            };
            children.push(Node::Text {
                text: display,
                style: TextStyle {
                    font_size:  inp.font_size,
                    color:      text_color,
                    baseline_y: inp.text_baseline_y,
                    clip_x:     inp.rect.x,
                    scroll_x:   inp.state.scroll_offset,
                },
                clip: Some(inp.rect),
            });

            // Cursor
            if focused {
                let byte_pos = inp.state.cursor();
                let chars = inp.state.text()[..byte_pos].chars().count();
                let cx = inp.text_x + chars as f32 * inp.char_width - inp.state.scroll_offset;
                let vp = 4.0 * s;
                children.push(Node::Cursor(CursorNode {
                    rect: Rect { x: cx, y: inp.rect.y + vp, width: 2.0, height: inp.rect.height - vp * 2.0 },
                    color: NColor::rgba(0.4, 0.7, 1.0, 1.0),
                    visible: self.cursor_on,
                }));
            }
        }

        // Status bar
        let focused_inp = &self.inputs[self.focused];
        let cursor_byte = focused_inp.state.cursor();
        let cursor_char = focused_inp.state.text()[..cursor_byte].chars().count();
        let sel_info = if let Some(anchor) = focused_inp.state.selection_anchor {
            let (s_byte, e_byte) = if anchor < cursor_byte { (anchor, cursor_byte) } else { (cursor_byte, anchor) };
            let sc = focused_inp.state.text()[..s_byte].chars().count();
            let ec = focused_inp.state.text()[..e_byte].chars().count();
            format!("  |  sel: {}–{} ({} chars)", sc, ec, ec - sc)
        } else { String::new() };

        let status = format!(
            "Field {} / {}   col: {}   len: {}{}",
            self.focused + 1, FIELDS,
            cursor_char,
            focused_inp.state.text().chars().count(),
            sel_info,
        );
        children.push(Node::Text {
            text: status,
            style: TextStyle {
                font_size:  12.0 * s,
                color:      NColor::rgba(0.5, 0.5, 0.5, 1.0),
                baseline_y: h - PAD * s,
                clip_x:     0.0,
                scroll_x:   0.0,
            },
            clip: None,
        });

        Node::layer(children)
    }
}

// ── Event handler ─────────────────────────────────────────────────────────────

struct DemoHandler {
    state:     Option<DemoState>,
    window:    Option<Window>,
    gl_ctx:    Option<PossiblyCurrentContext>,
    gl_surf:   Option<Surface<WindowSurface>>,
    renderer:  Option<Renderer>,
    modifiers: ModifiersState,
    dirty:     bool,
}

impl DemoHandler {
    fn new() -> Self {
        Self {
            state: None, window: None, gl_ctx: None,
            gl_surf: None, renderer: None,
            modifiers: ModifiersState::default(),
            dirty: true,
        }
    }

    fn render(&mut self) {
        let (Some(state), Some(renderer), Some(gl_surf), Some(gl_ctx), Some(window)) = (
            self.state.as_mut(), self.renderer.as_mut(),
            self.gl_surf.as_ref(), self.gl_ctx.as_ref(), self.window.as_ref(),
        ) else { return; };
        let _ = window;
        let node = state.build_node();
        renderer.render(&node, state.width, state.height);
        let _ = gl_surf.swap_buffers(gl_ctx);
        self.dirty = false;
    }
}

impl ApplicationHandler for DemoHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }

        let attrs = WindowAttributes::default()
            .with_title("fire-notes widget demo")
            .with_inner_size(winit::dpi::LogicalSize::new(720.0, 400.0));

        let config_template = ConfigTemplateBuilder::new().with_alpha_size(8);
        let display_builder = DisplayBuilder::new().with_window_attributes(Some(attrs));
        let (window, gl_config) = display_builder
            .build(event_loop, config_template, |mut configs| configs.next().unwrap())
            .expect("Failed to build display");
        let window = window.unwrap();
        let gl_display = gl_config.display();

        let ctx_attrs = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(None))
            .build(Some(window.window_handle().unwrap().as_raw()));
        let gl_ctx = unsafe {
            gl_display.create_context(&gl_config, &ctx_attrs).unwrap()
        };

        let size = window.inner_size();
        let surf_attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window.window_handle().unwrap().as_raw(),
            NonZeroU32::new(size.width.max(1)).unwrap(),
            NonZeroU32::new(size.height.max(1)).unwrap(),
        );
        let gl_surf = unsafe { gl_display.create_window_surface(&gl_config, &surf_attrs).unwrap() };
        let gl_ctx  = gl_ctx.make_current(&gl_surf).unwrap();

        let gl_renderer = unsafe {
            femtovg::renderer::OpenGl::new_from_function_cstr(|name| {
                let c = CString::new(name.to_bytes()).unwrap();
                gl_display.get_proc_address(&c) as *const _
            }).unwrap()
        };

        let scale  = window.scale_factor() as f32;
        let width  = size.width  as f32;
        let height = size.height as f32;

        let renderer = Renderer::new(gl_renderer, width, height, scale);
        let mut state = DemoState::new(width, height, scale);
        state.relayout();

        self.window   = Some(window);
        self.gl_ctx   = Some(gl_ctx);
        self.gl_surf  = Some(gl_surf);
        self.renderer = Some(renderer);
        self.state    = Some(state);
        self.dirty    = true;
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _id: winit::window::WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                _event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                if size.width == 0 || size.height == 0 { return; }
                if let (Some(gl_surf), Some(gl_ctx)) = (self.gl_surf.as_ref(), self.gl_ctx.as_ref()) {
                    gl_surf.resize(gl_ctx,
                        NonZeroU32::new(size.width).unwrap(),
                        NonZeroU32::new(size.height).unwrap());
                }
                let scale = self.window.as_ref().map(|w| w.scale_factor() as f32).unwrap_or(1.0);
                if let Some(r) = self.renderer.as_mut() { r.resize(size.width as f32, size.height as f32, scale); }
                if let Some(s) = self.state.as_mut()    { s.resize(size.width as f32, size.height as f32, scale); }
                self.render();
            }

            WindowEvent::ModifiersChanged(m) => {
                self.modifiers = m.state();
            }

            WindowEvent::KeyboardInput { event, is_synthetic, .. } => {
                if is_synthetic || event.state != ElementState::Pressed { return; }
                let ctrl  = self.modifiers.control_key();
                let shift = self.modifiers.shift_key();

                let state = match self.state.as_mut() { Some(s) => s, None => return };

                match &event.logical_key {
                    Key::Named(NamedKey::Tab) => {
                        state.focused = (state.focused + 1) % FIELDS;
                        state.cursor_on = true;
                        state.last_blink = Instant::now();
                    }
                    Key::Named(NamedKey::Escape) => {
                        state.focused().state.selection_anchor = None;
                    }
                    Key::Named(NamedKey::Backspace) => {
                        let cw = CHAR_W * state.scale;
                        if ctrl { state.focused().state.delete_word_left(); }
                        else    { state.focused().state.backspace(); }
                        state.focused().ensure_cursor_visible(cw);
                    }
                    Key::Named(NamedKey::Delete) => {
                        let cw = CHAR_W * state.scale;
                        if ctrl { state.focused().state.delete_word_right(); }
                        else    { state.focused().state.delete(); }
                        state.focused().ensure_cursor_visible(cw);
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        if ctrl { state.focused().move_word_left(shift); }
                        else    { state.focused().move_left(shift); }
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        if ctrl { state.focused().move_word_right(shift); }
                        else    { state.focused().move_right(shift); }
                    }
                    Key::Named(NamedKey::Home) => { state.focused().move_to_start(shift); }
                    Key::Named(NamedKey::End)  => { state.focused().move_to_end(shift); }
                    Key::Character(c) if !ctrl => {
                        for ch in c.chars() { state.focused().insert(ch); }
                    }
                    _ => return,
                }
                self.dirty = true;
            }

            WindowEvent::RedrawRequested => { self.render(); }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_mut() {
            if state.tick() { self.dirty = true; }
        }
        if self.dirty {
            if let Some(w) = self.window.as_ref() { w.request_redraw(); }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    let mut handler = DemoHandler::new();
    event_loop.run_app(&mut handler).expect("run");
}
