//! Notes picker overlay rendering

use crate::app::NoteEntry;
use crate::theme::Theme;
use crate::ui::{ListWidget, TextInput};
use femtovg::{Canvas, Color, Paint, Path, FontId, renderer::OpenGl};

pub struct NotesPickerRenderer<'a> {
    canvas: &'a mut Canvas<OpenGl>,
    fonts: &'a [FontId],
    theme: &'a Theme,
    width: f32,
    height: f32,
    scale: f32,
}

impl<'a> NotesPickerRenderer<'a> {
    pub fn new(
        canvas: &'a mut Canvas<OpenGl>,
        fonts: &'a [FontId],
        theme: &'a Theme,
        width: f32,
        height: f32,
        scale: f32,
    ) -> Self {
        Self {
            canvas,
            fonts,
            theme,
            width,
            height,
            scale,
        }
    }

    pub fn draw(
        &mut self,
        input: &TextInput,
        list: &ListWidget<NoteEntry>,
        cursor_visible: bool,
    ) {
        let scale = self.scale;
        
        // Overlay dimensions
        let overlay_width = (self.width * 0.6).min(500.0 * scale);
        let overlay_x = (self.width - overlay_width) / 2.0;
        let overlay_y = 60.0 * scale;
        
        let input_height = 36.0 * scale;
        let item_height = 32.0 * scale;
        let max_visible_items = 8;
        let visible_items = list.len().min(max_visible_items);
        let list_height = visible_items as f32 * item_height;
        let overlay_height = input_height + list_height + 16.0 * scale;
        
        // Draw semi-transparent backdrop
        let mut backdrop = Path::new();
        backdrop.rect(0.0, 0.0, self.width, self.height);
        self.canvas.fill_path(
            &backdrop,
            &Paint::color(Color::rgba(0, 0, 0, 120)),
        );
        
        // Draw overlay background with rounded corners
        let mut bg = Path::new();
        bg.rounded_rect(
            overlay_x,
            overlay_y,
            overlay_width,
            overlay_height,
            8.0 * scale,
        );
        self.canvas.fill_path(
            &bg,
            &Paint::color(Color::rgbf(
                self.theme.tab_inactive.0,
                self.theme.tab_inactive.1,
                self.theme.tab_inactive.2,
            )),
        );
        
        // Draw border
        let mut border = Path::new();
        border.rounded_rect(
            overlay_x,
            overlay_y,
            overlay_width,
            overlay_height,
            8.0 * scale,
        );
        self.canvas.stroke_path(
            &border,
            &Paint::color(Color::rgbf(
                self.theme.tab_active_border.0,
                self.theme.tab_active_border.1,
                self.theme.tab_active_border.2,
            )).with_line_width(2.0),
        );
        
        // Draw search input
        let input_x = overlay_x + 8.0 * scale;
        let input_y = overlay_y + 8.0 * scale;
        let input_width = overlay_width - 16.0 * scale;
        
        let mut input_bg = Path::new();
        input_bg.rounded_rect(input_x, input_y, input_width, input_height - 4.0 * scale, 4.0 * scale);
        self.canvas.fill_path(
            &input_bg,
            &Paint::color(Color::rgbf(
                self.theme.bg.0,
                self.theme.bg.1,
                self.theme.bg.2,
            )),
        );
        
        // Draw search text or placeholder
        let font_size = 14.0 * scale;
        let mut text_paint = Paint::color(Color::rgbf(
            self.theme.fg.0,
            self.theme.fg.1,
            self.theme.fg.2,
        ));
        text_paint.set_font(&self.fonts);
        text_paint.set_font_size(font_size);
        
        let text_x = input_x + 8.0 * scale;
        let text_y = input_y + (input_height - 4.0 * scale) / 2.0 + font_size * 0.35;
        
        if input.text().is_empty() {
            // Draw placeholder
            let mut placeholder_paint = Paint::color(Color::rgba(150, 150, 150, 180));
            placeholder_paint.set_font(&self.fonts);
            placeholder_paint.set_font_size(font_size);
            let _ = self.canvas.fill_text(text_x, text_y, "Search notes...", &placeholder_paint);
        } else {
            let _ = self.canvas.fill_text(text_x, text_y, input.text(), &text_paint);
        }
        
        // Draw cursor
        if cursor_visible {
            let cursor_char_idx = input.text()[..input.cursor()].chars().count();
            let char_width = self.measure_char_width(&text_paint);
            let cursor_x = text_x + cursor_char_idx as f32 * char_width;
            
            let mut cursor_path = Path::new();
            cursor_path.rect(cursor_x, input_y + 4.0 * scale, 2.0, input_height - 12.0 * scale);
            self.canvas.fill_path(
                &cursor_path,
                &Paint::color(Color::rgbf(
                    self.theme.tab_active_border.0,
                    self.theme.tab_active_border.1,
                    self.theme.tab_active_border.2,
                )),
            );
        }
        
        // Draw list items using ListWidget's visible_items iterator
        let list_y = input_y + input_height + 4.0 * scale;
        
        // Use scroll offset from the list widget
        let scroll_offset = list.scroll_offset();
        let selected_index = list.selected_index();
        
        for (display_idx, filtered_idx) in list.filtered_indices().iter().skip(scroll_offset).take(max_visible_items).enumerate() {
            let item_y = list_y + display_idx as f32 * item_height;
            let is_selected = scroll_offset + display_idx == selected_index;
            
            // Draw selection highlight
            if is_selected {
                let mut highlight = Path::new();
                highlight.rounded_rect(
                    input_x,
                    item_y,
                    input_width,
                    item_height - 2.0 * scale,
                    4.0 * scale,
                );
                self.canvas.fill_path(
                    &highlight,
                    &Paint::color(Color::rgbf(
                        self.theme.tab_active_border.0 * 0.3,
                        self.theme.tab_active_border.1 * 0.3,
                        self.theme.tab_active_border.2 * 0.3,
                    )),
                );
            }
            
            if let Some(note) = list.items().get(*filtered_idx) {
                // Draw note title
                let title_color = if is_selected {
                    Color::rgbf(self.theme.fg.0, self.theme.fg.1, self.theme.fg.2)
                } else {
                    Color::rgba(200, 200, 200, 220)
                };
                
                let mut title_paint = Paint::color(title_color);
                title_paint.set_font(&self.fonts);
                title_paint.set_font_size(font_size);
                
                let title_y = item_y + item_height / 2.0 + font_size * 0.35;
                let _ = self.canvas.fill_text(text_x, title_y, &note.title, &title_paint);
                
                // Draw "open" indicator if the note is already open
                if note.is_open {
                    let indicator_text = "●";
                    let mut indicator_paint = Paint::color(Color::rgbf(
                        self.theme.tab_active_border.0,
                        self.theme.tab_active_border.1,
                        self.theme.tab_active_border.2,
                    ));
                    indicator_paint.set_font(&self.fonts);
                    indicator_paint.set_font_size(font_size * 0.8);
                    
                    let indicator_x = input_x + input_width - 20.0 * scale;
                    let _ = self.canvas.fill_text(indicator_x, title_y, indicator_text, &indicator_paint);
                }
            }
        }
        
        // Draw "no results" message if empty
        if list.is_empty() && !input.text().is_empty() {
            let mut no_results_paint = Paint::color(Color::rgba(150, 150, 150, 180));
            no_results_paint.set_font(&self.fonts);
            no_results_paint.set_font_size(font_size);
            
            let msg_y = list_y + item_height / 2.0 + font_size * 0.35;
            let _ = self.canvas.fill_text(text_x, msg_y, "No matching notes", &no_results_paint);
        }
    }
    
    fn measure_char_width(&self, paint: &Paint) -> f32 {
        if let Ok(metrics) = self.canvas.measure_text(0.0, 0.0, "M", paint) {
            metrics.width()
        } else {
            9.6 * self.scale
        }
    }
}
