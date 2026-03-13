//! Handles `WindowEvent::Resized` — resizes the GL surface and re-renders immediately.

use std::num::NonZeroU32;

use glutin::prelude::*;

use super::window::AppState;

pub(super) fn handle_resize(state: &mut AppState, size: winit::dpi::PhysicalSize<u32>) {
    if size.width == 0 || size.height == 0 { return; }
    state.gl_surface.resize(
        &state.gl_context,
        NonZeroU32::new(size.width).unwrap(),
        NonZeroU32::new(size.height).unwrap(),
    );
    let scale = state.window.scale_factor() as f32;
    state.app.resize(size.width as f32, size.height as f32, scale);
    // Render immediately — avoids one-frame lag on Wayland where the
    // compositor resizes the surface before the next RedrawRequested.
    state.app.render();
    state.gl_surface
        .swap_buffers(&state.gl_context)
        .expect("Failed to swap buffers");
}
