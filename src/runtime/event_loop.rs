//! winit `ApplicationHandler` implementation — hidden from app code.
//!
//! This is the only file in the runtime that imports winit/glutin.
//! Everything above this layer is platform-agnostic.

use std::collections::HashSet;
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
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowAttributes};

use crate::platform::keys::{convert_winit_key, key_event_to_text_input};
use crate::primitives::text_input::EventOutcome;
use crate::renderer::Renderer;
use crate::runtime::app_trait::App;
use crate::runtime::clipboard::{ArboardClipboard, ClipboardPort, NoopClipboard};
use crate::runtime::focus::FocusManager;
use crate::view_ctx::ViewCtx;
use crate::ui::Rect;

const BLINK_MS: u64 = 530;

// ── RuntimeState ──────────────────────────────────────────────────────────────

struct RuntimeState {
    app:        Box<dyn App>,
    renderer:   Renderer,
    focus:      FocusManager,
    clipboard:  Box<dyn ClipboardPort>,
    cursor_on:  bool,
    last_blink: Instant,
    width:      f32,
    height:     f32,
    scale:      f32,
    char_width: f32,
    /// Layout cache from the most recent `view()` call.
    /// Provides correct rects for event handlers without re-computing layout.
    last_ctx:   Option<ViewCtx>,
}

impl RuntimeState {
    fn view_ctx(&self) -> ViewCtx {
        let mut focused = HashSet::new();
        if let Some(id) = self.focus.focused_id() { focused.insert(id); }
        let s = self.scale;
        ViewCtx {
            scale:      s,
            char_width: self.char_width / s,  // logical
            cursor_on:  self.cursor_on,
            focused,
            window:  Rect { x: 0.0, y: 0.0, width: self.width / s, height: self.height / s }, // logical
            layout:  std::collections::HashMap::new(),
        }
    }

    fn rebuild_focus(&mut self) {
        let ids = self.app.focus_order();
        self.focus = FocusManager::new();
        for id in ids { self.focus.register(id); }
    }
}

// ── RuntimeHandler ────────────────────────────────────────────────────────────

pub(crate) struct RuntimeConfig {
    pub title:    String,
    pub width:    f64,
    pub height:   f64,
    pub make_app: Box<dyn FnOnce() -> Box<dyn App>>,
}

pub(crate) struct RuntimeHandler {
    config:    Option<RuntimeConfig>,
    state:     Option<RuntimeState>,
    window:    Option<Window>,
    gl_ctx:    Option<PossiblyCurrentContext>,
    gl_surf:   Option<Surface<WindowSurface>>,
    modifiers: ModifiersState,
    mouse_x:   f32,
    mouse_y:   f32,
    dirty:     bool,
}

impl RuntimeHandler {
    pub fn new(config: RuntimeConfig) -> Self {
        Self {
            config: Some(config),
            state: None, window: None, gl_ctx: None, gl_surf: None,
            modifiers: ModifiersState::default(),
            mouse_x: 0.0, mouse_y: 0.0,
            dirty: true,
        }
    }

    fn do_render(&mut self) {
        let (Some(rs), Some(gl_surf), Some(gl_ctx)) = (
            self.state.as_mut(), self.gl_surf.as_ref(), self.gl_ctx.as_ref(),
        ) else { return; };
        let mut ctx = rs.view_ctx();
        let node = rs.app.view(&mut ctx);
        rs.last_ctx = Some(ctx);
        rs.renderer.render(&node, rs.width, rs.height);
        let _ = gl_surf.swap_buffers(gl_ctx);
        self.dirty = false;
    }
}

// ── ApplicationHandler ────────────────────────────────────────────────────────

impl ApplicationHandler for RuntimeHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }
        let config = self.config.take().expect("resumed() called twice");

        let attrs = WindowAttributes::default()
            .with_title(&config.title)
            .with_inner_size(winit::dpi::LogicalSize::new(config.width, config.height));

        let config_template = ConfigTemplateBuilder::new().with_alpha_size(8);
        let display_builder = DisplayBuilder::new().with_window_attributes(Some(attrs));
        let (window, gl_config) = display_builder
            .build(event_loop, config_template, |mut c| c.next().unwrap())
            .expect("Failed to build display");
        let window     = window.unwrap();
        let gl_display = gl_config.display();

        let ctx_attrs = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(None))
            .build(Some(window.window_handle().unwrap().as_raw()));
        let gl_ctx = unsafe { gl_display.create_context(&gl_config, &ctx_attrs).unwrap() };

        let size  = window.inner_size();
        let scale = window.scale_factor() as f32;
        let w = size.width  as f32;
        let h = size.height as f32;

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

        let mut renderer = Renderer::new(gl_renderer, w, h, scale);
        let char_width   = renderer.get_text_input_char_width();

        let clipboard: Box<dyn ClipboardPort> = match ArboardClipboard::new() {
            Some(c) => Box::new(c),
            None    => Box::new(NoopClipboard),
        };

        let app = (config.make_app)();
        let mut rs = RuntimeState {
            app, renderer, clipboard,
            focus: FocusManager::new(),
            cursor_on: true, last_blink: Instant::now(),
            width: w, height: h, scale, char_width,
            last_ctx: None,
        };
        rs.rebuild_focus();

        // First render
        let mut ctx = rs.view_ctx();
        let node = rs.app.view(&mut ctx);
        rs.last_ctx = Some(ctx);
        rs.renderer.render(&node, w, h);
        let _ = gl_surf.swap_buffers(&gl_ctx);

        self.window  = Some(window);
        self.gl_ctx  = Some(gl_ctx);
        self.gl_surf = Some(gl_surf);
        self.state   = Some(rs);
        self.dirty   = false;
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: winit::window::WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::Resized(size) => {
                if size.width == 0 || size.height == 0 { return; }
                if let (Some(surf), Some(ctx)) = (self.gl_surf.as_ref(), self.gl_ctx.as_ref()) {
                    surf.resize(ctx,
                        NonZeroU32::new(size.width).unwrap(),
                        NonZeroU32::new(size.height).unwrap());
                }
                if let Some(rs) = self.state.as_mut() {
                    let scale = self.window.as_ref().map(|w| w.scale_factor() as f32).unwrap_or(1.0);
                    rs.scale  = scale;
                    rs.width  = size.width  as f32;
                    rs.height = size.height as f32;
                    rs.renderer.resize(rs.width, rs.height, scale);
                    rs.char_width = rs.renderer.get_text_input_char_width();
                }
                self.do_render();
            }

            WindowEvent::ModifiersChanged(m) => { self.modifiers = m.state(); }

            WindowEvent::KeyboardInput { event, is_synthetic, .. } => {
                if is_synthetic || event.state != ElementState::Pressed { return; }
                let shift = self.modifiers.shift_key();
                let ctrl  = self.modifiers.control_key();

                let rs = match self.state.as_mut() { Some(s) => s, None => return };

                let Some(key_ev) = convert_winit_key(&event.logical_key, &self.modifiers) else { return };

                // Tab: cycle focus — not a text event
                if key_ev.key == crate::app::Key::Tab {
                    if shift { rs.focus.focus_prev(); } else { rs.focus.focus_next(); }
                    rs.cursor_on  = true;
                    rs.last_blink = Instant::now();
                    self.dirty = true;
                    return;
                }

                // Ctrl+V: read clipboard then deliver as Paste
                if ctrl && key_ev.key == crate::app::Key::Char('v') {
                    if let Some(text) = rs.clipboard.get_text() {
                        if let Some(id) = rs.focus.focused_id() {
                            let ctx = rs.last_ctx.as_ref().map(|c| c.clone()).unwrap_or_else(|| rs.view_ctx());
                            rs.app.handle_paste(id, &text, &ctx);
                        }
                        self.dirty = true;
                    }
                    return;
                }

                // All other keys → TextInputEvent → App::handle_key
                if let Some(text_ev) = key_event_to_text_input(&key_ev) {
                    if let Some(id) = rs.focus.focused_id() {
                        let ctx = rs.last_ctx.as_ref().map(|c| c.clone()).unwrap_or_else(|| rs.view_ctx());
                        let outcome = rs.app.handle_key(id, text_ev, &ctx);
                        match outcome {
                            EventOutcome::Copied(text) => rs.clipboard.set_text(text),
                            EventOutcome::Cut(text)    => rs.clipboard.set_text(text),
                            _ => {}
                        }
                    }
                    self.dirty = true;
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_x = position.x as f32;
                self.mouse_y = position.y as f32;
            }

            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                let rs = match self.state.as_mut() { Some(s) => s, None => return };
                // Divide physical mouse coords by scale → logical coords for app.
                let x = self.mouse_x / rs.scale;
                let y = self.mouse_y / rs.scale;
                let ctx = rs.last_ctx.as_ref().cloned().unwrap_or_else(|| rs.view_ctx());
                if let Some(focused_id) = rs.app.handle_pointer_down(x, y, &ctx) {
                    rs.focus.focus(focused_id);
                }
                rs.cursor_on  = true;
                rs.last_blink = Instant::now();
                self.dirty = true;
            }

            WindowEvent::RedrawRequested => self.do_render(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(rs) = self.state.as_mut() {
            if rs.last_blink.elapsed() >= Duration::from_millis(BLINK_MS) {
                rs.cursor_on  = !rs.cursor_on;
                rs.last_blink = Instant::now();
                self.dirty = true;
            }
        }
        if self.dirty {
            if let Some(w) = self.window.as_ref() { w.request_redraw(); }
        }
        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(16),
        ));
    }
}
