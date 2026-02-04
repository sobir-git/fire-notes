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
mod ui;
mod visual_position;

use app::{App, Key as AppKey, KeyEvent, Modifiers, resolve_keybinding};
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, PossiblyCurrentContext};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use persistence::{WindowState, load_window_state, save_session_state, save_window_state};
use raw_window_handle::HasWindowHandle;
use std::ffi::CString;
use std::num::NonZeroU32;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
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

fn capture_window_state(window: &Window) -> Option<WindowState> {
    let position = window.outer_position().ok()?;
    let size = window.inner_size();
    if size.width == 0 || size.height == 0 {
        return None;
    }
    Some(WindowState {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    })
}

struct AppHandler {
    state: Option<AppState>,
    modifiers: ModifiersState,
    mouse_position: (f64, f64),
    mouse_pressed: bool,
    last_click_time: Option<Instant>,
    last_click_pos: Option<(f64, f64)>,
    click_count: u32,
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
            click_count: 0,
        }
    }
}

/// Convert winit Key to our KeyEvent (free function to avoid borrow issues)
fn convert_winit_key(key: &Key, modifiers: &ModifiersState) -> Option<KeyEvent> {
    let mods = Modifiers {
        ctrl: modifiers.control_key(),
        shift: modifiers.shift_key(),
        alt: modifiers.alt_key(),
    };

    let app_key = match key {
        Key::Named(NamedKey::Escape) => AppKey::Escape,
        Key::Named(NamedKey::Enter) => AppKey::Enter,
        Key::Named(NamedKey::Tab) => AppKey::Tab,
        Key::Named(NamedKey::Backspace) => AppKey::Backspace,
        Key::Named(NamedKey::Delete) => AppKey::Delete,
        Key::Named(NamedKey::ArrowLeft) => AppKey::ArrowLeft,
        Key::Named(NamedKey::ArrowRight) => AppKey::ArrowRight,
        Key::Named(NamedKey::ArrowUp) => AppKey::ArrowUp,
        Key::Named(NamedKey::ArrowDown) => AppKey::ArrowDown,
        Key::Named(NamedKey::Home) => AppKey::Home,
        Key::Named(NamedKey::End) => AppKey::End,
        Key::Named(NamedKey::PageUp) => AppKey::PageUp,
        Key::Named(NamedKey::PageDown) => AppKey::PageDown,
        Key::Named(NamedKey::Space) => AppKey::Space,
        Key::Character(c) => {
            let ch = c.chars().next()?;
            AppKey::Char(ch)
        }
        _ => return None,
    };

    Some(KeyEvent::new(app_key, mods))
}

impl ApplicationHandler for AppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        // Window attributes - borderless for custom title bar
        let mut window_attrs = WindowAttributes::default()
            .with_title("Fire Notes")
            .with_decorations(false);
            
        #[cfg(target_os = "linux")]
        {
            use winit::platform::wayland::WindowAttributesExtWayland;
            use winit::platform::x11::WindowAttributesExtX11;
            
            window_attrs = WindowAttributesExtWayland::with_name(window_attrs, "fire-notes", "fire-notes");
            window_attrs = WindowAttributesExtX11::with_name(window_attrs, "fire-notes", "fire-notes");
        }
        if let Some(saved) = load_window_state() {
            window_attrs = window_attrs
                .with_inner_size(PhysicalSize::new(saved.width, saved.height))
                .with_position(PhysicalPosition::new(saved.x, saved.y));
        } else {
            window_attrs = window_attrs.with_inner_size(LogicalSize::new(600.0, 400.0));
        }

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
                if let Some(window_state) = capture_window_state(&state.window) {
                    let _ = save_window_state(window_state);
                }
                let session_state = state.app.export_session_state();
                let _ = save_session_state(&session_state);
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

            WindowEvent::KeyboardInput { event, is_synthetic, .. } => {
                // Ignore synthetic key events generated when window gains/loses focus
                // This prevents Tab insertion from Alt+Tab window switching
                if is_synthetic {
                    return;
                }
                
                if event.state == ElementState::Pressed {
                    // Convert winit key event to our KeyEvent
                    // Extract modifiers before borrowing state
                    let key_event = convert_winit_key(&event.logical_key, &self.modifiers);
                    if let Some(key_event) = key_event {
                        // Resolve to action and execute
                        if let Some(action) = resolve_keybinding(&key_event) {
                            let result = state.app.execute(action);
                            if result.needs_redraw() {
                                state.window.request_redraw();
                            }
                        }
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                use crate::app::ScrollInput;

                // Check if scrolling in tab bar area (horizontal scroll)
                if state.app.is_mouse_in_tab_bar() {
                    let scroll_delta = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y * 30.0,
                        MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 2.0,
                    };
                    let result = state.app.scroll_tab_bar(scroll_delta);
                    if result.needs_redraw() {
                        state.window.request_redraw();
                    }
                } else {
                    // Content area scrolling - use centralized scroll abstraction
                    let scroll_input = match delta {
                        MouseScrollDelta::LineDelta(_, y) => ScrollInput::LineDelta(y),
                        MouseScrollDelta::PixelDelta(pos) => ScrollInput::PixelDelta(pos.y as f32),
                    };

                    let result = state.app.handle_scroll_event(scroll_input);
                    if result.needs_redraw() {
                        state.window.request_redraw();
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = (position.x, position.y);
                let needs_redraw_on_hover = state
                    .app
                    .handle_mouse_move(self.mouse_position.0 as f32, self.mouse_position.1 as f32)
                    .needs_redraw();

                // Update cursor based on hover position
                use winit::window::CursorIcon;
                let cursor = if let Some(edge) = state.app.hovered_resize_edge() {
                    match edge {
                        crate::ui::ResizeEdge::North | crate::ui::ResizeEdge::South => {
                            CursorIcon::NsResize
                        }
                        crate::ui::ResizeEdge::East | crate::ui::ResizeEdge::West => {
                            CursorIcon::EwResize
                        }
                        crate::ui::ResizeEdge::NorthEast
                        | crate::ui::ResizeEdge::SouthWest => CursorIcon::NeswResize,
                        crate::ui::ResizeEdge::NorthWest
                        | crate::ui::ResizeEdge::SouthEast => CursorIcon::NwseResize,
                    }
                } else if state.app.is_mouse_in_tab_bar() {
                    // Check if hovering over window controls
                    if state.app.ui_state().hovered_window_close {
                        CursorIcon::Pointer
                    } else if state.app.ui_state().hovered_window_maximize {
                        CursorIcon::Pointer
                    } else if state.app.ui_state().hovered_window_minimize {
                        CursorIcon::Pointer
                    } else if state.app.ui_state().hovered_plus {
                        CursorIcon::Pointer
                    } else if state.app.ui_state().hovered_tab_index.is_some() {
                        CursorIcon::Pointer
                    } else {
                        CursorIcon::Default
                    }
                } else {
                    // Editor area - use configurable cursor
                    // Change EDITOR_CURSOR_TYPE in config.rs to customize
                    match crate::config::cursor::EDITOR_CURSOR_TYPE {
                        "Text" => CursorIcon::Text,
                        "Help" => CursorIcon::Help,
                        "Crosshair" => CursorIcon::Crosshair,
                        "Cell" => CursorIcon::Cell,
                        "VerticalText" => CursorIcon::VerticalText,
                        "Alias" => CursorIcon::Alias,
                        "Copy" => CursorIcon::Copy,
                        "Move" => CursorIcon::Move,
                        "NoDrop" => CursorIcon::NoDrop,
                        "NotAllowed" => CursorIcon::NotAllowed,
                        "Grab" => CursorIcon::Grab,
                        "Grabbing" => CursorIcon::Grabbing,
                        "Progress" => CursorIcon::Progress,
                        "Wait" => CursorIcon::Wait,
                        "ContextMenu" => CursorIcon::ContextMenu,
                        "ZoomIn" => CursorIcon::ZoomIn,
                        "ZoomOut" => CursorIcon::ZoomOut,
                        "AllScroll" => CursorIcon::AllScroll,
                        _ => CursorIcon::Text, // Default to Text if unknown
                    }
                };
                state.window.set_cursor(cursor);

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
            } => match button {
                MouseButton::Left => {
                    if button_state == ElementState::Pressed {
                        self.mouse_pressed = true;
                        let now = Instant::now();
                        let mut is_consecutive_click = false;

                        if let Some(last_time) = self.last_click_time {
                            if now.duration_since(last_time).as_millis() < 500 {
                                if let Some((last_x, last_y)) = self.last_click_pos {
                                    let dist = ((self.mouse_position.0 - last_x).powi(2)
                                        + (self.mouse_position.1 - last_y).powi(2))
                                    .sqrt();
                                    if dist < 5.0 {
                                        is_consecutive_click = true;
                                    }
                                }
                            }
                        }

                        if is_consecutive_click {
                            self.click_count += 1;
                        } else {
                            self.click_count = 1;
                        }

                        self.last_click_time = Some(now);
                        self.last_click_pos = Some(self.mouse_position);

                        let result = match self.click_count {
                            2 => state.app.handle_double_click(
                                self.mouse_position.0 as f32,
                                self.mouse_position.1 as f32,
                            ),
                            3 => {
                                let res = state.app.handle_triple_click(
                                    self.mouse_position.0 as f32,
                                    self.mouse_position.1 as f32,
                                );
                                self.click_count = 0; // Reset after triple click
                                res
                            }
                            _ => {
                                let shift = self.modifiers.shift_key();
                                state.app.click_at(
                                    self.mouse_position.0 as f32,
                                    self.mouse_position.1 as f32,
                                    shift,
                                )
                            }
                        };

                        // Handle window control actions
                        match &result {
                            crate::app::AppResult::WindowMinimize => {
                                state.window.set_minimized(true);
                            }
                            crate::app::AppResult::WindowMaximize => {
                                let is_maximized = state.window.is_maximized();
                                state.window.set_maximized(!is_maximized);
                            }
                            crate::app::AppResult::WindowClose => {
                                if let Some(window_state) = capture_window_state(&state.window) {
                                    let _ = save_window_state(window_state);
                                }
                                let session_state = state.app.export_session_state();
                                let _ = save_session_state(&session_state);
                                event_loop.exit();
                                return;
                            }
                            crate::app::AppResult::WindowDrag => {
                                let _ = state.window.drag_window();
                                // OS takes over, reset our state
                                self.mouse_pressed = false;
                                state.app.end_drag();
                            }
                            crate::app::AppResult::WindowResize(edge) => {
                                use winit::window::ResizeDirection;
                                let direction = match edge {
                                    crate::ui::ResizeEdge::North => ResizeDirection::North,
                                    crate::ui::ResizeEdge::South => ResizeDirection::South,
                                    crate::ui::ResizeEdge::East => ResizeDirection::East,
                                    crate::ui::ResizeEdge::West => ResizeDirection::West,
                                    crate::ui::ResizeEdge::NorthEast => ResizeDirection::NorthEast,
                                    crate::ui::ResizeEdge::NorthWest => ResizeDirection::NorthWest,
                                    crate::ui::ResizeEdge::SouthEast => ResizeDirection::SouthEast,
                                    crate::ui::ResizeEdge::SouthWest => ResizeDirection::SouthWest,
                                };
                                let _ = state.window.drag_resize_window(direction);
                                // OS takes over, reset our state
                                self.mouse_pressed = false;
                                state.app.end_drag();
                            }
                            _ => {}
                        }

                        if result.needs_redraw() {
                            state.window.request_redraw();
                        }
                    } else {
                        self.mouse_pressed = false;
                        state.app.end_drag();
                        state.app.reset_scroll_state();
                    }
                }
                MouseButton::Right | MouseButton::Other(2) | MouseButton::Middle
                    if button_state == ElementState::Pressed =>
                {
                    println!("Right-click detected at {:?}", self.mouse_position);
                    let result = state
                        .app
                        .right_click_at(self.mouse_position.0 as f32, self.mouse_position.1 as f32);
                    if result.needs_redraw() {
                        state.window.request_redraw();
                    }
                }
                _ => {}
            },

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
            
            // Only poll when animations are active, otherwise wait efficiently
            if state.app.has_active_animations() {
                // Rate-limit animation polling to ~60 FPS
                event_loop.set_control_flow(ControlFlow::WaitUntil(
                    Instant::now() + Duration::from_millis(16)
                ));
            } else {
                // Wait until next cursor blink (500ms) or event
                let next_blink = Instant::now() + Duration::from_millis(500);
                event_loop.set_control_flow(ControlFlow::WaitUntil(next_blink));
            }
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}
