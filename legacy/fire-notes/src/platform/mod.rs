//! Platform layer — windowing, OpenGL context, event loop.

pub mod events;
pub mod keys;
#[cfg(target_os = "linux")]
pub mod linux;
pub mod window;

pub(crate) mod handle_dropped_file;
pub(crate) mod handle_keyboard;
pub(crate) mod handle_mouse;
pub(crate) mod handle_resize;
