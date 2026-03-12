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
pub mod viewport;

use crate::app::NoteEntry;
use crate::tab::Tab;
use crate::theme::Theme;
use crate::config::rendering;
use crate::ui::{ListWidget, UiTree, TextInput};
use femtovg::{Canvas, Color, FontId, Paint, renderer::OpenGl};
use std::time::Instant;

use flame::FlameSystem;
use notes_picker::NotesPickerRenderer;
use tab_bar::TabBarRenderer;
use text_content::TextContentRenderer;

pub struct Renderer {
    canvas: Canvas<OpenGl>,
    fonts: Vec<FontId>,
    theme: Theme,
    width: f32,
    height: f32,
    scale: f32,
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
            flame_system: FlameSystem::new(),
            animation_start: now,
        }
    }

    pub fn resize(&mut self, width: f32, height: f32, scale: f32) {
        self.width = width;
        self.height = height;
        self.scale = scale;
    }

    pub fn has_active_flames(&self) -> bool {
        self.flame_system.has_active_flames()
    }

    pub fn render(
        &mut self,
        ui_tree: &UiTree,
        tabs: &[(&str, bool)],
        current_tab: &Tab,
        cursor_visible: bool,
        hovered_tab_index: Option<usize>,
        hovered_plus: bool,
        hovered_scrollbar: bool,
        dragging_scrollbar: bool,
        renaming_tab: Option<usize>,
        rename_input: Option<&TextInput>,
        typing_flame_positions: &[(usize, usize, Instant)],
        hovered_window_minimize: bool,
        hovered_window_maximize: bool,
        hovered_window_close: bool,
        notes_picker_state: Option<(&TextInput, &ListWidget<NoteEntry>)>,
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
            let mut tab_bar_renderer = TabBarRenderer::new(
                &mut self.canvas,
                &self.fonts,
                &self.theme,
                self.scale,
            );
            tab_bar_renderer.draw(
                &ui_tree.tab_bar,
                tabs,
                hovered_tab_index,
                hovered_plus,
                renaming_tab,
                rename_input,
                cursor_visible,
                hovered_window_minimize,
                hovered_window_maximize,
                hovered_window_close,
            );
        }

        // Draw text content
        {
            let mut text_content = TextContentRenderer::new(
                &mut self.canvas,
                &self.fonts,
                &self.theme,
                self.scale,
                self.animation_start,
            );
            text_content.draw(
                current_tab,
                &ui_tree.content_area,
                &ui_tree.content_area.scrollbar,
                cursor_visible,
                hovered_scrollbar,
                dragging_scrollbar,
                &mut self.flame_system,
                typing_flame_positions,
            );
        }

        // Draw notes picker overlay if active
        if let (Some((input, list)), Some(picker_layout)) = (notes_picker_state, &ui_tree.notes_picker) {
            let mut picker = NotesPickerRenderer::new(
                &mut self.canvas,
                &self.fonts,
                &self.theme,
                self.width,
                self.height,
                self.scale,
            );
            picker.draw(input, list, cursor_visible, picker_layout);
        }

        self.canvas.flush();
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
