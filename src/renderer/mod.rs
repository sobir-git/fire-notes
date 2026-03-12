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
use tab_bar::{TabBarRenderer, TabBarInteraction};
use text_content::{TextContentRenderer, ScrollbarState};

/// All per-frame state passed to `Renderer::render`.
/// Groups the 15 previously-decomposed arguments into one struct.
pub struct RenderFrame<'a> {
    pub ui_tree:                &'a UiTree,
    pub tabs:                   &'a [(&'a str, bool)],
    pub current_tab:            &'a Tab,
    pub cursor_visible:         bool,
    pub hovered_tab_index:      Option<usize>,
    pub hovered_plus:           bool,
    pub hovered_scrollbar:      bool,
    pub dragging_scrollbar:     bool,
    pub renaming_tab:           Option<usize>,
    pub rename_input:           Option<&'a TextInput>,
    pub typing_flame_positions: &'a [(usize, usize, Instant)],
    pub hovered_window_minimize: bool,
    pub hovered_window_maximize: bool,
    pub hovered_window_close:    bool,
    pub notes_picker_state:     Option<(&'a TextInput, &'a ListWidget<NoteEntry>)>,
}

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

    pub fn render(&mut self, frame: &RenderFrame<'_>) {
        let ui_tree                 = frame.ui_tree;
        let tabs                    = frame.tabs;
        let current_tab             = frame.current_tab;
        let cursor_visible          = frame.cursor_visible;
        let hovered_scrollbar       = frame.hovered_scrollbar;
        let dragging_scrollbar      = frame.dragging_scrollbar;
        let typing_flame_positions  = frame.typing_flame_positions;
        let notes_picker_state      = frame.notes_picker_state;
        let width  = ui_tree.width();
        let height = ui_tree.height();

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
                &TabBarInteraction {
                    hovered_tab_index:  frame.hovered_tab_index,
                    hovered_plus:       frame.hovered_plus,
                    renaming_tab:       frame.renaming_tab,
                    rename_input:       frame.rename_input,
                    cursor_visible,
                    hovered_minimize:   frame.hovered_window_minimize,
                    hovered_maximize:   frame.hovered_window_maximize,
                    hovered_close:      frame.hovered_window_close,
                },
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
                &ScrollbarState {
                    widget:   &ui_tree.content_area.scrollbar,
                    hovered:  hovered_scrollbar,
                    dragging: dragging_scrollbar,
                },
                cursor_visible,
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
