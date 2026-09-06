#![allow(dead_code, unused_imports)]
//! Framework runtime — the public entry point for application code.
//!
//! # Usage
//! ```ignore
//! Runtime::new()
//!     .title("My App")
//!     .size(720.0, 400.0)
//!     .run(MyApp::new());
//! ```
//!
//! The runtime owns the window, OpenGL context, event loop, renderer,
//! clipboard, cursor blink clock, and focus manager.
//! App code never touches any of those directly.

pub mod app_trait;
pub mod clipboard;
pub mod focus;
pub mod view_ctx;
pub(crate) mod event_loop;

pub use app_trait::App;
pub use crate::view_ctx::ViewCtx;

use event_loop::{RuntimeConfig, RuntimeHandler};

/// Builder for launching a framework application.
pub struct Runtime {
    title:  String,
    width:  f64,
    height: f64,
}

impl Runtime {
    pub fn new() -> Self {
        Self { title: "App".into(), width: 800.0, height: 600.0 }
    }

    pub fn title(mut self, t: impl Into<String>) -> Self { self.title = t.into(); self }
    pub fn size(mut self, w: f64, h: f64) -> Self { self.width = w; self.height = h; self }

    /// Start the event loop, transferring ownership of `app`.
    /// This function blocks until the window is closed.
    pub fn run(self, app: impl App + 'static) {
        let config = RuntimeConfig {
            title:    self.title,
            width:    self.width,
            height:   self.height,
            make_app: Box::new(move || Box::new(app)),
        };
        let mut handler = RuntimeHandler::new(config);
        let event_loop  = winit::event_loop::EventLoop::new().expect("event loop");
        event_loop.run_app(&mut handler).expect("run");
    }
}

impl Default for Runtime {
    fn default() -> Self { Self::new() }
}
