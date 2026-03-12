//! Platform layer — windowing, OpenGL context, event loop.

pub mod events;
pub mod keys;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod window;
