//! GPU-accelerated rendering with femtovg.
//!
//! Architecture:
//! - `Renderer::render(node, w, h)` — public entry point; clears canvas, walks tree, flushes.
//! - `draw::draw(node, ctx)` — recursive walker; dispatches to per-node draw functions.
//! - `draw/box_node.rs`, `text_node.rs`, … — each node variant owns its GPU calls.
//! - `DrawCtx` — shared render context (canvas, fonts, theme, scale, flame).
//! - `node_renderer` — thin shell; kept for the `nc()` color helper used by draw modules.
//!
//! Invariant: `src/logic/` and `src/layout/` never import femtovg.
//! All GPU calls live inside `src/renderer/draw/`.

mod draw_ctx;
pub(crate) mod draw;
mod flame;
mod fonts;
pub(crate) mod node_renderer; // kept for nc() color helper
pub(crate) mod text_content;  // FlameHit / build_flame_lookup
#[cfg(test)]
pub mod headless;
#[cfg(test)]
mod tests;

use crate::config::rendering;
use crate::layout::Node;
use crate::theme::Theme;
use draw_ctx::DrawCtx;
use femtovg::{Canvas, Color, FontId, Paint, renderer::OpenGl};
use std::time::Instant;

use flame::FlameSystem;

pub struct Renderer {
    canvas:          Canvas<OpenGl>,
    fonts:           Vec<FontId>,
    theme:           Theme,
    scale:           f32,
    flame_system:    FlameSystem,
    animation_start: Instant,
}

impl Renderer {
    pub fn new(renderer: OpenGl, _width: f32, _height: f32, scale: f32) -> Self {
        let mut canvas = Canvas::new(renderer).expect("Failed to create canvas");
        let fonts = fonts::load_fonts(&mut canvas);
        let theme = Theme::dark();
        let now   = Instant::now();
        Self { canvas, fonts, theme, scale, flame_system: FlameSystem::new(), animation_start: now }
    }

    pub fn resize(&mut self, _width: f32, _height: f32, scale: f32) {
        self.scale = scale;
    }

    pub fn has_active_flames(&self) -> bool {
        self.flame_system.has_active_flames()
    }

    /// Render the full UI scene from a Node tree.
    pub fn render(&mut self, node: &Node, width: f32, height: f32) {
        self.canvas.set_size(width as u32, height as u32, 1.0);
        self.canvas.clear_rect(
            0, 0, width as u32, height as u32,
            Color::rgbf(self.theme.bg.0, self.theme.bg.1, self.theme.bg.2),
        );

        let mut ctx = DrawCtx {
            canvas:          &mut self.canvas,
            fonts:           &self.fonts,
            theme:           &self.theme,
            scale:           self.scale,
            animation_start: self.animation_start,
            flame:           &mut self.flame_system,
        };
        draw::draw(node, &mut ctx);

        self.canvas.flush();
    }

    pub fn get_picker_char_width(&mut self) -> f32 {
        let mut paint = Paint::color(Color::rgb(255, 255, 255));
        paint.set_font(&self.fonts);
        paint.set_font_size(14.0 * self.scale);
        fonts::measure_char_width(&mut self.canvas, &paint, self.scale)
    }

    /// Measured advance width for the TextInput font (14px × scale).
    /// Feed this back to `TextInput::set_char_width` after init and on resize.
    #[allow(dead_code)]
    pub fn get_text_input_char_width(&mut self) -> f32 {
        self.get_picker_char_width()
    }

    pub fn get_char_width(&mut self) -> f32 {
        let mut paint = Paint::color(Color::rgb(255, 255, 255));
        paint.set_font(&self.fonts);
        paint.set_font_size(rendering::CONTENT_FONT_SIZE * self.scale);
        fonts::measure_char_width(&mut self.canvas, &paint, self.scale)
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn canvas_mut(&mut self) -> &mut femtovg::Canvas<femtovg::renderer::OpenGl> {
        &mut self.canvas
    }
}
