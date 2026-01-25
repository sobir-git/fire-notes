//! GPU-accelerated rendering with femtovg

mod flame;
mod fonts;
mod tab_bar;
mod text_content;

use crate::tab::Tab;
use crate::theme::Theme;
use femtovg::{Canvas, Color, FontId, Paint, renderer::OpenGl};
use std::time::Instant;

use flame::FlameSystem;
use tab_bar::TabBarRenderer;
use text_content::TextContentRenderer;

pub struct Renderer {
    canvas: Canvas<OpenGl>,
    fonts: Vec<FontId>,
    theme: Theme,
    width: f32,
    height: f32,
    scale: f32,
    tab_scroll_x: f32,
    flame_system: FlameSystem,
    animation_start: Instant,
}

impl Renderer {
    pub fn new(renderer: OpenGl, width: f32, height: f32, scale: f32) -> Self {
        let mut canvas = Canvas::new(renderer).expect("Failed to create canvas");

        // Load fonts with fallbacks
        let fonts = fonts::load_fonts(&mut canvas);

        let theme = Theme::dark();

        let now = Instant::now();
        Self {
            canvas,
            fonts,
            theme,
            width,
            height,
            scale,
            tab_scroll_x: 0.0,
            flame_system: FlameSystem::new(),
            animation_start: now,
        }
    }

    pub fn resize(&mut self, width: f32, height: f32, scale: f32) {
        self.width = width;
        self.height = height;
        self.scale = scale;
    }

    pub fn set_tab_scroll_x(&mut self, scroll: f32) {
        self.tab_scroll_x = scroll;
    }

    pub fn has_active_flames(&self) -> bool {
        self.flame_system.has_active_flames()
    }

    pub fn render(
        &mut self,
        tabs: &[(&str, bool)],
        current_tab: &Tab,
        cursor_visible: bool,
        hovered_tab_index: Option<usize>,
        hovered_plus: bool,
        hovered_scrollbar: bool,
        dragging_scrollbar: bool,
        renaming_tab: Option<usize>,
        typing_flame_positions: &[(usize, usize, Instant)],
    ) {
        let (width, height) = (self.width, self.height);

        // Use DPI=1.0, but we compensate by using larger font sizes in physical pixels
        // This forces femtovg to rasterize glyphs at higher resolution
        self.canvas.set_size(width as u32, height as u32, 1.0);
        self.canvas.clear_rect(
            0,
            0,
            width as u32,
            height as u32,
            Color::rgbf(self.theme.bg.0, self.theme.bg.1, self.theme.bg.2),
        );

        // Draw tab bar
        {
            let mut tab_bar = TabBarRenderer::new(
                &mut self.canvas,
                &self.fonts,
                &self.theme,
                self.width,
                self.scale,
                self.tab_scroll_x,
            );
            tab_bar.draw(tabs, hovered_tab_index, hovered_plus, renaming_tab);
        }

        // Draw text content
        {
            let mut text_content = TextContentRenderer::new(
                &mut self.canvas,
                &self.fonts,
                &self.theme,
                self.width,
                self.height,
                self.scale,
                self.animation_start,
            );
            text_content.draw(
                current_tab,
                cursor_visible,
                hovered_scrollbar,
                dragging_scrollbar,
                &mut self.flame_system,
                typing_flame_positions,
            );
        }

        self.canvas.flush();
    }

    pub fn get_char_width(&self) -> f32 {
        let mut text_paint = Paint::color(Color::rgb(255, 255, 255));
        text_paint.set_font(&self.fonts);
        text_paint.set_font_size(16.0 * self.scale);
        self.measure_char_width(&text_paint)
    }

    fn measure_char_width(&self, paint: &Paint) -> f32 {
        if let Ok(metrics) = self.canvas.measure_text(0.0, 0.0, "M", paint) {
            metrics.width()
        } else {
            9.6 * self.scale // Fallback approximate width
        }
    }

}
