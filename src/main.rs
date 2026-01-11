//! Fire Notes - A blazing-fast markdown editor
//!
//! Performance targets:
//! - Input latency: ≤5ms
//! - Frame rate: 120+ fps
//! - Memory: <10MB
//! - Binary size: <2MB

mod app;
mod config;
mod persistence;
mod renderer;
mod tab;
mod text_buffer;
mod theme;

use app::App;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, PossiblyCurrentContext};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use std::ffi::CString;
use std::num::NonZeroU32;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut handler = AppHandler::new();
    event_loop.run_app(&mut handler).expect("Event loop failed");
}

struct AppHandler {
    state: Option<AppState>,
    modifiers: ModifiersState,
    mouse_position: (f64, f64),
    mouse_pressed: bool,
    last_click_time: Option<Instant>,
    last_click_pos: Option<(f64, f64)>,
}

struct AppState {
    window: Window,
    gl_context: PossiblyCurrentContext,
    gl_surface: Surface<WindowSurface>,
    app: App,
}

impl AppHandler {
    fn new() -> Self {
        Self {
            state: None,
            modifiers: ModifiersState::default(),
            mouse_position: (0.0, 0.0),
            mouse_pressed: false,
            last_click_time: None,
            last_click_pos: None,
        }
    }
}

impl ApplicationHandler for AppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        // Window attributes - smaller for easier testing
        let window_attrs = WindowAttributes::default()
            .with_title("Fire Notes")
            .with_inner_size(LogicalSize::new(600.0, 400.0));

        // OpenGL config with 4x MSAA for smooth text and edges
        let config_template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_multisampling(4); // 4x anti-aliasing

        let display_builder = DisplayBuilder::new().with_window_attributes(Some(window_attrs));

        let (window, gl_config) = display_builder
            .build(event_loop, config_template, |configs| {
                configs
                    .reduce(|accum, config| {
                        if config.num_samples() > accum.num_samples() {
                            config
                        } else {
                            accum
                        }
                    })
                    .expect("No GL configs found")
            })
            .expect("Failed to create window");

        let window = window.expect("Window not created");
        let gl_display = gl_config.display();

        // Create OpenGL context
        let context_attrs = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::OpenGl(None))
            .build(Some(
                window
                    .window_handle()
                    .expect("Failed to get window handle")
                    .as_raw(),
            ));

        let gl_context = unsafe {
            gl_display
                .create_context(&gl_config, &context_attrs)
                .expect("Failed to create GL context")
        };

        // Create surface
        let size = window.inner_size();
        let surface_attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window
                .window_handle()
                .expect("Failed to get window handle")
                .as_raw(),
            NonZeroU32::new(size.width.max(1)).unwrap(),
            NonZeroU32::new(size.height.max(1)).unwrap(),
        );

        let gl_surface = unsafe {
            gl_display
                .create_window_surface(&gl_config, &surface_attrs)
                .expect("Failed to create surface")
        };

        let gl_context = gl_context
            .make_current(&gl_surface)
            .expect("Failed to make context current");

        // Load OpenGL functions
        let renderer = unsafe {
            femtovg::renderer::OpenGl::new_from_function_cstr(|name| {
                let cstr = CString::new(name.to_bytes()).unwrap();
                gl_display.get_proc_address(&cstr) as *const _
            })
            .expect("Failed to create renderer")
        };

        let scale = window.scale_factor() as f32;
        let app = App::new(renderer, size.width as f32, size.height as f32, scale);

        self.state = Some(AppState {
            window,
            gl_context,
            gl_surface,
            app,
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(s) => s,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    state.gl_surface.resize(
                        &state.gl_context,
                        NonZeroU32::new(size.width).unwrap(),
                        NonZeroU32::new(size.height).unwrap(),
                    );
                    let scale = state.window.scale_factor() as f32;
                    state
                        .app
                        .resize(size.width as f32, size.height as f32, scale);
                    state.window.request_redraw();
                }
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    let ctrl = self.modifiers.control_key();
                    let shift = self.modifiers.shift_key();
                    let alt = self.modifiers.alt_key();

                    let result = match &event.logical_key {
                        Key::Named(NamedKey::Escape) => {
                            event_loop.exit();
                            return;
                        }
                        Key::Character(c) if ctrl && c.as_str() == "n" => state.app.new_tab(),
                        Key::Character(c) if ctrl && c.as_str() == "w" => {
                            state.app.close_current_tab()
                        }
                        Key::Character(c) if ctrl && c.as_str() == "s" => state.app.save_current(),
                        Key::Character(c) if ctrl && c.as_str() == "o" => state.app.open_file(),
                        Key::Character(c) if ctrl && c.as_str() == "c" => state.app.handle_copy(),
                        Key::Character(c) if ctrl && c.as_str() == "x" => state.app.handle_cut(),
                        Key::Character(c) if ctrl && c.as_str() == "v" => state.app.handle_paste(),
                        Key::Character(c) if ctrl && c.as_str() == "a" => {
                            state.app.handle_select_all()
                        }
                        Key::Named(NamedKey::Tab) if ctrl => state.app.next_tab(),
                        Key::Named(NamedKey::Backspace) => state.app.handle_backspace(),
                        Key::Named(NamedKey::Delete) => state.app.handle_delete(),
                        Key::Named(NamedKey::Enter) => state.app.handle_char('\n'),
                        Key::Named(NamedKey::ArrowLeft) => {
                            if ctrl {
                                state.app.move_cursor_word_left(shift)
                            } else {
                                state.app.move_cursor_left(shift)
                            }
                        }
                        Key::Named(NamedKey::ArrowRight) => {
                            if ctrl {
                                state.app.move_cursor_word_right(shift)
                            } else {
                                state.app.move_cursor_right(shift)
                            }
                        }
                        Key::Named(NamedKey::ArrowUp) => {
                            if alt {
                                state.app.handle_move_lines_up()
                            } else {
                                state.app.move_cursor_up(shift)
                            }
                        }
                        Key::Named(NamedKey::ArrowDown) => {
                            if alt {
                                state.app.handle_move_lines_down()
                            } else {
                                state.app.move_cursor_down(shift)
                            }
                        }
                        Key::Character(c) => {
                            let char_lower = c
                                .to_lowercase()
                                .chars()
                                .next()
                                .unwrap_or(c.chars().next().unwrap_or('\0'));
                            match char_lower {
                                'a' if ctrl => state.app.handle_select_all(),
                                'c' if ctrl => state.app.handle_copy(),
                                'x' if ctrl => state.app.handle_cut(),
                                'v' if ctrl => state.app.handle_paste(),
                                'z' if ctrl => state.app.handle_undo(),
                                'z' if alt => state.app.toggle_word_wrap(),
                                'y' if ctrl => state.app.handle_redo(),
                                _ => {
                                    // Regular typing
                                    if !ctrl && !alt {
                                        state.app.handle_char(c.chars().next().unwrap_or('\0'))
                                    } else {
                                        crate::app::AppResult::Ok
                                    }
                                }
                            }
                        }
                        Key::Named(NamedKey::Space) => state.app.handle_char(' '),
                        Key::Named(NamedKey::PageUp) => state.app.scroll_up(),
                        Key::Named(NamedKey::PageDown) => state.app.scroll_down(),
                        Key::Named(NamedKey::Home) => {
                            if ctrl {
                                state.app.move_cursor_to_start(shift)
                            } else {
                                state.app.move_cursor_to_line_start(shift)
                            }
                        }
                        Key::Named(NamedKey::End) => {
                            if ctrl {
                                state.app.move_cursor_to_end(shift)
                            } else {
                                state.app.move_cursor_to_line_end(shift)
                            }
                        }
                        _ => crate::app::AppResult::Ok,
                    };

                    if result.needs_redraw() {
                        state.window.request_redraw();
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y as i32,
                    MouseScrollDelta::PixelDelta(pos) => -(pos.y / 24.0) as i32,
                };

                let mut redraw = false;
                if scroll_lines > 0 {
                    for _ in 0..scroll_lines {
                        if state.app.scroll_down().needs_redraw() {
                            redraw = true;
                        }
                    }
                } else {
                    for _ in 0..(-scroll_lines) {
                        if state.app.scroll_up().needs_redraw() {
                            redraw = true;
                        }
                    }
                }

                if redraw {
                    state.window.request_redraw();
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = (position.x, position.y);
                let needs_redraw_on_hover = state
                    .app
                    .handle_mouse_move(self.mouse_position.0 as f32, self.mouse_position.1 as f32)
                    .needs_redraw();

                if self.mouse_pressed {
                    if state
                        .app
                        .drag_at(self.mouse_position.0 as f32, self.mouse_position.1 as f32)
                        .needs_redraw()
                    {
                        state.window.request_redraw();
                    }
                } else if needs_redraw_on_hover {
                    state.window.request_redraw();
                }
            }

            WindowEvent::MouseInput {
                state: button_state,
                button,
                ..
            } => {
                if button == MouseButton::Left {
                    if button_state == ElementState::Pressed {
                        self.mouse_pressed = true;
                        let now = Instant::now();
                        let mut is_double_click = false;

                        if let Some(last_time) = self.last_click_time {
                            if now.duration_since(last_time).as_millis() < 500 {
                                if let Some((last_x, last_y)) = self.last_click_pos {
                                    let dist = ((self.mouse_position.0 - last_x).powi(2)
                                        + (self.mouse_position.1 - last_y).powi(2))
                                    .sqrt();
                                    if dist < 5.0 {
                                        is_double_click = true;
                                    }
                                }
                            }
                        }

                        let result = if is_double_click {
                            let r = state.app.handle_double_click(
                                self.mouse_position.0 as f32,
                                self.mouse_position.1 as f32,
                            );
                            self.last_click_time = None;
                            r
                        } else {
                            let shift = self.modifiers.shift_key();
                            let r = state.app.click_at(
                                self.mouse_position.0 as f32,
                                self.mouse_position.1 as f32,
                                shift,
                            );
                            self.last_click_time = Some(now);
                            self.last_click_pos = Some(self.mouse_position);
                            r
                        };

                        if result.needs_redraw() {
                            state.window.request_redraw();
                        }
                    } else {
                        self.mouse_pressed = false;
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                state.app.render();
                state
                    .gl_surface
                    .swap_buffers(&state.gl_context)
                    .expect("Failed to swap buffers");
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(state) = &mut self.state {
            if state.app.tick().needs_redraw() {
                state.window.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::Poll);
    }
}
