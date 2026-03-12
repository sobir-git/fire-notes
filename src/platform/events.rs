//! Event loop handler — bridges winit events to the App.

use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use glutin::prelude::*;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::ModifiersState;
use winit::window::WindowId;

use crate::app::{AppResult, ScrollInput};
use crate::config::scroll;
use crate::persistence::{save_session_state, save_window_state};

use super::keys::convert_winit_key;
use super::window::{AppState, build_app_state, capture_window_state};

pub struct AppHandler {
    pub state: Option<AppState>,
    modifiers: ModifiersState,
    mouse_position: (f64, f64),
    mouse_pressed: bool,
    last_click_time: Option<Instant>,
    last_click_pos: Option<(f64, f64)>,
    click_count: u32,
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
                if size.width > 0 && size.height > 0 {
                    state.gl_surface.resize(
                        &state.gl_context,
                        NonZeroU32::new(size.width).unwrap(),
                        NonZeroU32::new(size.height).unwrap(),
                    );
                    let scale = state.window.scale_factor() as f32;
                    state.app.resize(size.width as f32, size.height as f32, scale);
                    state.window.request_redraw();
                }
            }

            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }

            WindowEvent::KeyboardInput { event, is_synthetic, .. } => {
                if is_synthetic { return; }
                if event.state == ElementState::Pressed {
                    let key_event = convert_winit_key(&event.logical_key, &self.modifiers);
                    if let Some(key_event) = key_event {
                        if let Some(action) = crate::app::resolve_keybinding(&key_event) {
                            let result = state.app.execute(action);
                            if result.needs_redraw() { state.window.request_redraw(); }
                        }
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                if state.app.is_mouse_in_tab_bar() {
                    let scroll_delta = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y * scroll::TAB_SCROLL_PIXELS,
                        MouseScrollDelta::PixelDelta(pos) => pos.y as f32 / 2.0,
                    };
                    if state.app.scroll_tab_bar(scroll_delta).needs_redraw() {
                        state.window.request_redraw();
                    }
                } else {
                    let scroll_input = match delta {
                        MouseScrollDelta::LineDelta(_, y) => ScrollInput::LineDelta(y),
                        MouseScrollDelta::PixelDelta(pos) => ScrollInput::PixelDelta(pos.y as f32),
                    };
                    if state.app.handle_scroll_event(scroll_input).needs_redraw() {
                        state.window.request_redraw();
                    }
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = (position.x, position.y);
                let x = self.mouse_position.0 as f32;
                let y = self.mouse_position.1 as f32;
                let needs_redraw_on_hover = state.app.handle_mouse_move(x, y).needs_redraw();

                use winit::window::CursorIcon;
                let cursor = if let Some(edge) = state.app.hovered_resize_edge() {
                    match edge {
                        crate::ui::ResizeEdge::North | crate::ui::ResizeEdge::South => CursorIcon::NsResize,
                        crate::ui::ResizeEdge::East | crate::ui::ResizeEdge::West => CursorIcon::EwResize,
                        crate::ui::ResizeEdge::NorthEast | crate::ui::ResizeEdge::SouthWest => CursorIcon::NeswResize,
                        crate::ui::ResizeEdge::NorthWest | crate::ui::ResizeEdge::SouthEast => CursorIcon::NwseResize,
                    }
                } else if state.app.is_mouse_in_tab_bar() {
                    let ui = state.app.ui_state();
                    if ui.hovered_window_close || ui.hovered_window_maximize || ui.hovered_window_minimize
                        || ui.hovered_plus || ui.hovered_tab_index.is_some()
                    {
                        CursorIcon::Pointer
                    } else {
                        CursorIcon::Default
                    }
                } else {
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
                        _ => CursorIcon::Text,
                    }
                };
                state.window.set_cursor(cursor);

                if self.mouse_pressed {
                    if state.app.drag_at(x, y).needs_redraw() {
                        state.window.request_redraw();
                    }
                } else if needs_redraw_on_hover {
                    state.window.request_redraw();
                }
            }

            WindowEvent::MouseInput { state: button_state, button, .. } => {
                match button {
                    MouseButton::Left => {
                        if button_state == ElementState::Pressed {
                            self.mouse_pressed = true;
                            let now = Instant::now();
                            let mut consecutive = false;

                            if let Some(last_time) = self.last_click_time {
                                if now.duration_since(last_time).as_millis() < 500 {
                                    if let Some((lx, ly)) = self.last_click_pos {
                                        let dist = ((self.mouse_position.0 - lx).powi(2)
                                            + (self.mouse_position.1 - ly).powi(2))
                                        .sqrt();
                                        if dist < 5.0 { consecutive = true; }
                                    }
                                }
                            }

                            if consecutive { self.click_count += 1; } else { self.click_count = 1; }
                            self.last_click_time = Some(now);
                            self.last_click_pos = Some(self.mouse_position);

                            let x = self.mouse_position.0 as f32;
                            let y = self.mouse_position.1 as f32;

                            let result = match self.click_count {
                                2 => state.app.handle_double_click(x, y),
                                3 => {
                                    let r = state.app.handle_triple_click(x, y);
                                    self.click_count = 0;
                                    r
                                }
                                _ => state.app.click_at(x, y, self.modifiers.shift_key()),
                            };

                            match &result {
                                AppResult::WindowMinimize => {
                                    state.window.set_minimized(true);
                                }
                                AppResult::WindowMaximize => {
                                    let is_maximized = state.window.is_maximized();
                                    state.window.set_maximized(!is_maximized);
                                }
                                AppResult::WindowClose => {
                                    if let Some(ws) = capture_window_state(&state.window) {
                                        let _ = save_window_state(ws);
                                    }
                                    let _ = save_session_state(&state.app.export_session_state());
                                    event_loop.exit();
                                    return;
                                }
                                AppResult::WindowDrag => {
                                    let _ = state.window.drag_window();
                                    self.mouse_pressed = false;
                                    state.app.end_drag();
                                }
                                AppResult::WindowResize(edge) => {
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
                                    self.mouse_pressed = false;
                                    state.app.end_drag();
                                }
                                _ => {}
                            }

                            if result.needs_redraw() { state.window.request_redraw(); }
                        } else {
                            self.mouse_pressed = false;
                            state.app.end_drag();
                            state.app.reset_scroll_state();
                        }
                    }
                    MouseButton::Right | MouseButton::Other(2) | MouseButton::Middle
                        if button_state == ElementState::Pressed =>
                    {
                        let result = state.app.right_click_at(
                            self.mouse_position.0 as f32,
                            self.mouse_position.1 as f32,
                        );
                        if result.needs_redraw() { state.window.request_redraw(); }
                    }
                    _ => {}
                }
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
