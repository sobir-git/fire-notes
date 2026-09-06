//! Event loop handler — bridges winit events to the App.
//!
//! This file owns `AppHandler` and the `ApplicationHandler` impl.
//! Each heavy event arm is delegated to a focused sub-module:
//!
//! | Module                  | Handles                          |
//! |-------------------------|----------------------------------|
//! | `handle_resize`         | `WindowEvent::Resized`           |
//! | `handle_keyboard`       | `WindowEvent::KeyboardInput`     |
//! | `handle_mouse`          | Wheel / CursorMoved / MouseInput |
//! | `handle_dropped_file`   | `WindowEvent::DroppedFile`       |

use std::time::{Duration, Instant};

use glutin::prelude::*;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::ModifiersState;
use winit::window::WindowId;

use crate::persistence::{save_session_state, save_window_state};

use super::window::{AppState, build_app_state, capture_window_state};

pub struct AppHandler {
    pub state: Option<AppState>,
    pub(super) modifiers: ModifiersState,
    pub(super) mouse_position: (f64, f64),
    pub(super) mouse_pressed: bool,
    pub(super) last_click_time: Option<Instant>,
    pub(super) last_click_pos: Option<(f64, f64)>,
    pub(super) click_count: u32,
}

impl AppHandler {
    pub fn new() -> Self {
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

impl ApplicationHandler for AppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() { return; }
        self.state = Some(build_app_state(event_loop));
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
                if let Some(ws) = capture_window_state(&state.window) {
                    let _ = save_window_state(ws);
                }
                let _ = save_session_state(&state.app.export_session_state());
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                super::handle_resize::handle_resize(state, size);
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }

            WindowEvent::KeyboardInput { event, is_synthetic, .. } => {
                super::handle_keyboard::handle_keyboard(
                    state, &event, is_synthetic, &self.modifiers,
                );
            }

            WindowEvent::MouseWheel { delta, .. } => {
                super::handle_mouse::handle_mouse_wheel(state, delta);
            }

            WindowEvent::CursorMoved { position, .. } => {
                super::handle_mouse::handle_cursor_moved(
                    &mut self.mouse_position,
                    self.mouse_pressed,
                    state,
                    position,
                );
            }

            WindowEvent::MouseInput { state: button_state, button, .. } => {
                super::handle_mouse::handle_mouse_input(
                    self.mouse_position,
                    &mut self.mouse_pressed,
                    &mut self.last_click_time,
                    &mut self.last_click_pos,
                    &mut self.click_count,
                    self.modifiers,
                    state,
                    event_loop,
                    button_state,
                    button,
                );
            }

            WindowEvent::DroppedFile(path) => {
                super::handle_dropped_file::handle_dropped_file(state, path);
            }

            WindowEvent::RedrawRequested => {
                state.app.render();
                state.gl_surface
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
            if state.app.logic.poll_events() {
                state.window.request_redraw();
            }
            if state.app.has_active_animations() {
                event_loop.set_control_flow(ControlFlow::WaitUntil(
                    Instant::now() + Duration::from_millis(16),
                ));
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(
                    Instant::now() + Duration::from_millis(500),
                ));
            }
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}
