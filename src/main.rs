//! Fire Notes - A blazing-fast markdown editor
//!
//! Performance targets:
//! - Input latency: ≤5ms
//! - Frame rate: 120+ fps
//! - Memory: <10MB
//! - Binary size: <2MB

mod app;
mod components;
mod config;
mod layout;
mod logic;
mod persistence;
mod platform;
mod primitives;
mod renderer;
mod tab;
mod text_buffer;
mod theme;
mod ui;
mod visual_position;

use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    // On GNOME Wayland, XCURSOR_THEME and XCURSOR_SIZE are not exported as env
    // vars (Plasma 6+ also stopped doing this). winit/SCTK falls back to theme
    // "default" at size 24 when they are absent, ignoring the user's settings.
    // Read from gsettings and populate the vars before the event loop starts so
    // the compositor-loaded cursor matches the system cursor in theme and size.
    #[cfg(target_os = "linux")]
    platform::linux::init_xcursor_env();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut handler = platform::events::AppHandler::new();
    event_loop.run_app(&mut handler).expect("Event loop failed");
}
