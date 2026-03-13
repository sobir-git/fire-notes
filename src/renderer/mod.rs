//! GPU-accelerated rendering with femtovg
//!
//! Single entry point: `Renderer::render(node, width, height)` walks the
//! `Node` tree produced by `AppLogic::render()` and emits draw calls.
//! No `RenderFrame`, no per-subsystem renderers.

mod flame;
mod fonts;
mod node_renderer;
pub(crate) mod text_content; // kept only for FlameHit/build_flame_lookup used by node_renderer
#[cfg(test)]
pub mod headless;
#[cfg(test)]
mod tests;

use crate::config::rendering;
use crate::layout::Node;
use crate::theme::Theme;
use femtovg::{Canvas, Color, FontId, Paint, renderer::OpenGl};
use std::time::Instant;

use flame::FlameSystem;
use node_renderer::NodeRenderer;

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
    ///
    /// `node` is produced by `AppLogic::render()` — no RenderFrame, no
    /// per-subsystem baking, no translate structs.
    pub fn render(&mut self, node: &Node, width: f32, height: f32) {
        self.canvas.set_size(width as u32, height as u32, 1.0);
        self.canvas.clear_rect(
            0, 0, width as u32, height as u32,
            Color::rgbf(self.theme.bg.0, self.theme.bg.1, self.theme.bg.2),
        );

        let mut nr = NodeRenderer {
            canvas:          &mut self.canvas,
            fonts:           &self.fonts,
            theme:           &self.theme,
            scale:           self.scale,
            animation_start: self.animation_start,
            flame:           &mut self.flame_system,
        };
        nr.draw(node);

        self.canvas.flush();
    }

    pub fn get_picker_char_width(&mut self) -> f32 {
        let mut paint = Paint::color(Color::rgb(255, 255, 255));
        paint.set_font(&self.fonts);
        paint.set_font_size(14.0 * self.scale);
        fonts::measure_char_width(&mut self.canvas, &paint, self.scale)
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
