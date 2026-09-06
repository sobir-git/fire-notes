//! Handles mouse events: wheel, cursor movement, and button clicks.

use std::time::Instant;

use winit::event::{ElementState, MouseButton, MouseScrollDelta};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::ModifiersState; // needed for ModifiersState param type in handle_mouse_input

use crate::app::{AppResult, CursorShape, ScrollInput};
use crate::config::scroll;
use crate::persistence::{save_session_state, save_window_state};

use super::window::{AppState, capture_window_state};

pub(super) fn handle_mouse_wheel(
    state: &mut AppState,
    delta: MouseScrollDelta,
) {
    if state.app.is_notes_picker_open() {
        let lines = match delta {
            MouseScrollDelta::LineDelta(_, y) => if y > 0.0 { -1isize } else { 1 },
            MouseScrollDelta::PixelDelta(pos) => if pos.y > 0.0 { -1 } else { 1 },
        };
        if state.app.scroll_notes_picker(lines).needs_redraw() {
            state.window.request_redraw();
        }
    } else if state.app.is_mouse_in_tab_bar() {
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

/// Mouse position is updated in-place via the `mouse_position` out-parameter.
pub(super) fn handle_cursor_moved(
    mouse_position: &mut (f64, f64),
    mouse_pressed: bool,
    state: &mut AppState,
    position: winit::dpi::PhysicalPosition<f64>,
) {
    *mouse_position = (position.x, position.y);
    let x = mouse_position.0 as f32;
    let y = mouse_position.1 as f32;

    let needs_redraw_on_hover = if state.app.is_notes_picker_open() {
        state.app.hover_notes_picker(x, y).needs_redraw()
    } else {
        state.app.handle_mouse_move(x, y).needs_redraw()
    };

    use winit::window::CursorIcon;
    let cursor = match state.app.logic.cursor_shape {
        CursorShape::Default    => CursorIcon::Default,
        CursorShape::Text       => CursorIcon::Text,
        CursorShape::Pointer    => CursorIcon::Pointer,
        CursorShape::NsResize   => CursorIcon::NsResize,
        CursorShape::EwResize   => CursorIcon::EwResize,
        CursorShape::NeswResize => CursorIcon::NeswResize,
        CursorShape::NwseResize => CursorIcon::NwseResize,
    };
    state.window.set_cursor(cursor);

    if mouse_pressed {
        if state.app.drag_at(x, y).needs_redraw() {
            state.window.request_redraw();
        }
    } else if needs_redraw_on_hover {
        state.window.request_redraw();
    }
}

pub(super) fn handle_mouse_input(
    mouse_position: (f64, f64),
    mouse_pressed: &mut bool,
    last_click_time: &mut Option<Instant>,
    last_click_pos: &mut Option<(f64, f64)>,
    click_count: &mut u32,
    modifiers: ModifiersState,
    state: &mut AppState,
    event_loop: &ActiveEventLoop,
    button_state: ElementState,
    button: MouseButton,
) {
    match button {
        MouseButton::Left => {
            if button_state == ElementState::Pressed {
                *mouse_pressed = true;
                let now = Instant::now();
                let mut consecutive = false;

                if let Some(last_time) = *last_click_time {
                    if now.duration_since(last_time).as_millis() < 500 {
                        if let Some((lx, ly)) = *last_click_pos {
                            let dist = ((mouse_position.0 - lx).powi(2)
                                + (mouse_position.1 - ly).powi(2))
                            .sqrt();
                            if dist < 5.0 { consecutive = true; }
                        }
                    }
                }

                if consecutive { *click_count += 1; } else { *click_count = 1; }
                *last_click_time = Some(now);
                *last_click_pos  = Some(mouse_position);

                let x = mouse_position.0 as f32;
                let y = mouse_position.1 as f32;

                let result = match *click_count {
                    2 => state.app.handle_double_click(x, y),
                    3 => {
                        let r = state.app.handle_triple_click(x, y);
                        *click_count = 0;
                        r
                    }
                    _ => state.app.click_at(x, y, modifiers.shift_key()),
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
                        *mouse_pressed = false;
                        state.app.end_drag();
                    }
                    AppResult::WindowResize(edge) => {
                        use winit::window::ResizeDirection;
                        let direction = match edge {
                            crate::ui::ResizeEdge::North     => ResizeDirection::North,
                            crate::ui::ResizeEdge::South     => ResizeDirection::South,
                            crate::ui::ResizeEdge::East      => ResizeDirection::East,
                            crate::ui::ResizeEdge::West      => ResizeDirection::West,
                            crate::ui::ResizeEdge::NorthEast => ResizeDirection::NorthEast,
                            crate::ui::ResizeEdge::NorthWest => ResizeDirection::NorthWest,
                            crate::ui::ResizeEdge::SouthEast => ResizeDirection::SouthEast,
                            crate::ui::ResizeEdge::SouthWest => ResizeDirection::SouthWest,
                        };
                        let _ = state.window.drag_resize_window(direction);
                        *mouse_pressed = false;
                        state.app.end_drag();
                    }
                    _ => {}
                }

                if result.needs_redraw() { state.window.request_redraw(); }
            } else {
                *mouse_pressed = false;
                state.app.end_drag();
                state.app.reset_scroll_state();
            }
        }
        MouseButton::Right | MouseButton::Other(2) | MouseButton::Middle
            if button_state == ElementState::Pressed =>
        {
            let result = state.app.right_click_at(
                mouse_position.0 as f32,
                mouse_position.1 as f32,
            );
            if result.needs_redraw() { state.window.request_redraw(); }
        }
        _ => {}
    }
}
