//! Fire Notes - A blazing-fast markdown editor
//!
//! Performance targets:
//! - Input latency: ≤5ms
//! - Frame rate: 120+ fps
//! - Memory: <10MB
//! - Binary size: <2MB

mod app;
mod config;
mod logic;
mod persistence;
mod platform;
mod render_frame;
mod renderer;
mod tab;
mod text_buffer;
mod theme;
mod ui;
mod visual_position;

use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut handler = platform::events::AppHandler::new();
    event_loop.run_app(&mut handler).expect("Event loop failed");
}
