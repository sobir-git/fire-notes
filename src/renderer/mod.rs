//! GPU-accelerated rendering with femtovg

mod flame;
mod fonts;
#[cfg(test)]
pub mod headless;
#[cfg(test)]
mod tests;
mod notes_picker;
mod tab_bar;
mod text_content;

use crate::theme::Theme;
use crate::config::rendering;
use crate::render_frame::RenderFrame;
use crate::ui::TextInput;
use femtovg::{Canvas, Color, FontId, Paint, renderer::OpenGl};
use std::time::Instant;

use flame::FlameSystem;
use notes_picker::NotesPickerRenderer;
use tab_bar::{TabBarRenderer, RenameOverlay};
use text_content::TextContentRenderer;

pub struct Renderer {
    canvas: Canvas<OpenGl>,
    fonts: Vec<FontId>,
    theme: Theme,
    scale: f32,
    flame_system: FlameSystem,
    animation_start: Instant,
}

impl Renderer {
    pub fn new(renderer: OpenGl, _width: f32, _height: f32, scale: f32) -> Self {
        let mut canvas = Canvas::new(renderer).expect("Failed to create canvas");

        // Load fonts with fallbacks
        let fonts = fonts::load_fonts(&mut canvas);

        let theme = Theme::dark();

        let now = Instant::now();
        Self {
            canvas,
            fonts,
            theme,
            scale,
            flame_system: FlameSystem::new(),
            animation_start: now,
        }
    }

    pub fn resize(&mut self, _width: f32, _height: f32, scale: f32) {
        self.scale = scale;
    }

    pub fn has_active_flames(&self) -> bool {
        self.flame_system.has_active_flames()
    }

    pub fn render(
        &mut self,
        frame: &RenderFrame,
        rename_input: Option<&TextInput>,
    ) {
        let width  = frame.width;
        let height = frame.height;
        let cursor_visible = frame.cursor.visible;

        // Build a &[(&str, bool)] slice for the tab bar renderer.
        let tab_titles_owned: Vec<(&str, bool)> = frame.tabs.iter()
            .map(|t| (t.title.as_str(), t.is_active))
            .collect();

        self.canvas.set_size(width as u32, height as u32, 1.0);
        self.canvas.clear_rect(
            0, 0, width as u32, height as u32,
            Color::rgbf(self.theme.bg.0, self.theme.bg.1, self.theme.bg.2),
        );

        // Draw tab bar
        {
            let rename_overlay = frame.rename.as_ref().and_then(|ren| {
                rename_input.map(|inp| RenameOverlay {
                    tab_index:      ren.tab_index,
                    input:          inp,
                    cursor_visible,
                })
            });
            let mut tab_bar_renderer = TabBarRenderer::new(
                &mut self.canvas, &self.fonts, &self.theme, self.scale,
            );
            tab_bar_renderer.draw(&frame.tab_bar, &tab_titles_owned, rename_overlay.as_ref());
        }

        // Draw text content
        {
            let mut text_content = TextContentRenderer::new(
                &mut self.canvas, &self.fonts, &self.theme, self.scale, self.animation_start,
            );
            text_content.draw(frame, &mut self.flame_system);
        }

        // Draw notes picker overlay if active
        if let Some(picker) = &frame.notes_picker {
            let mut picker_renderer = NotesPickerRenderer::new(
                &mut self.canvas, &self.fonts, &self.theme, self.scale,
            );
            picker_renderer.draw(picker);
        }

        self.canvas.flush();
    }

    pub fn get_picker_char_width(&mut self) -> f32 {
        let mut paint = Paint::color(Color::rgb(255, 255, 255));
        paint.set_font(&self.fonts);
        paint.set_font_size(14.0 * self.scale);
        fonts::measure_char_width(&mut self.canvas, &paint, self.scale)
    }

    pub fn get_char_width(&mut self) -> f32 {
        let mut text_paint = Paint::color(Color::rgb(255, 255, 255));
        text_paint.set_font(&self.fonts);
        text_paint.set_font_size(rendering::CONTENT_FONT_SIZE * self.scale);
        fonts::measure_char_width(&mut self.canvas, &text_paint, self.scale)
    }

    /// Expose the canvas for test-only pixel readback.
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn canvas_mut(&mut self) -> &mut femtovg::Canvas<femtovg::renderer::OpenGl> {
        &mut self.canvas
    }

}
