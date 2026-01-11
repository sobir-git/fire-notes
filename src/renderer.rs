//! GPU-accelerated rendering with femtovg

use crate::tab::Tab;
use crate::theme::Theme;
use femtovg::{Canvas, Color, FontId, Paint, Path, renderer::OpenGl};

pub struct Renderer {
    canvas: Canvas<OpenGl>,
    font: FontId,
    theme: Theme,
    width: f32,
    height: f32,
    scale: f32,
    new_tab_button_bounds: Option<(f32, f32, f32, f32)>, // (x, y, width, height)
}

impl Renderer {
    /// Snap a coordinate to the pixel grid to prevent blurry text rendering.
    /// femtovg uses bilinear filtering which causes blur at sub-pixel positions.
    #[inline]
    fn snap_to_pixel(coord: f32) -> f32 {
        coord.round()
    }

    pub fn new(renderer: OpenGl, width: f32, height: f32, scale: f32) -> Self {
        let mut canvas = Canvas::new(renderer).expect("Failed to create canvas");

        // Load a system monospace font - try common paths
        let font = Self::load_font(&mut canvas);

        let theme = Theme::dark();

        Self {
            canvas,
            font,
            theme,
            width,
            height,
            scale,
            new_tab_button_bounds: None,
        }
    }

    fn load_font(canvas: &mut Canvas<OpenGl>) -> FontId {
        // Try common monospace font paths on Linux
        let font_paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
            "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf",
            "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
            "/usr/share/fonts/dejavu/DejaVuSansMono.ttf",
        ];

        for path in &font_paths {
            if let Ok(font) = canvas.add_font(path) {
                return font;
            }
        }

        // Fallback: try to find any TTF font
        if let Ok(entries) = std::fs::read_dir("/usr/share/fonts/truetype") {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                        for sub_entry in sub_entries.flatten() {
                            let path = sub_entry.path();
                            if path.extension().map(|e| e == "ttf").unwrap_or(false) {
                                if let Ok(font) = canvas.add_font(path) {
                                    return font;
                                }
                            }
                        }
                    }
                }
            }
        }

        panic!("No suitable font found! Please install dejavu-fonts or liberation-fonts.");
    }

    pub fn resize(&mut self, width: f32, height: f32, scale: f32) {
        self.width = width;
        self.height = height;
        self.scale = scale;
    }

    pub fn new_tab_button_bounds(&self) -> Option<(f32, f32, f32, f32)> {
        self.new_tab_button_bounds
    }

    pub fn render(&mut self, tabs: &[(&str, bool)], current_tab: &Tab) {
        let (width, height, _) = (self.width, self.height, self.scale);

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
        self.draw_tab_bar(tabs);

        // Draw text content
        self.draw_text_content(current_tab);

        self.canvas.flush();
    }

    fn draw_tab_bar(&mut self, tabs: &[(&str, bool)]) {
        let tab_height = 36.0 * self.scale;
        let mut x = 0.0;
        let tab_padding = 16.0 * self.scale;

        for (title, is_active) in tabs {
            let tab_width =
                (title.len() as f32 * 9.0 * self.scale + tab_padding * 2.0).max(100.0 * self.scale);

            // Tab background
            let mut path = Path::new();
            path.rect(x, 0.0, tab_width, tab_height);

            let color = if *is_active {
                Color::rgbf(
                    self.theme.tab_active.0,
                    self.theme.tab_active.1,
                    self.theme.tab_active.2,
                )
            } else {
                Color::rgbf(
                    self.theme.tab_inactive.0,
                    self.theme.tab_inactive.1,
                    self.theme.tab_inactive.2,
                )
            };

            self.canvas.fill_path(&path, &Paint::color(color));

            // Tab title
            let mut text_paint = Paint::color(Color::rgbf(
                self.theme.fg.0,
                self.theme.fg.1,
                self.theme.fg.2,
            ));
            text_paint.set_font(&[self.font]);
            text_paint.set_font_size(14.0 * self.scale);

            let text_x = Self::snap_to_pixel(x + tab_padding);
            let text_y = Self::snap_to_pixel(tab_height / 2.0 + 5.0);
            let _ = self.canvas.fill_text(text_x, text_y, title, &text_paint);

            x += tab_width + 1.0;
        }

        // New Tab (+) button
        let new_tab_button_size = 28.0 * self.scale;
        let button_x = x + 8.0 * self.scale;
        let button_y = (tab_height - new_tab_button_size) / 2.0;

        let mut btn_path = Path::new();
        btn_path.rounded_rect(
            button_x,
            button_y,
            new_tab_button_size,
            new_tab_button_size,
            4.0 * self.scale,
        );
        self.canvas
            .fill_path(&btn_path, &Paint::color(Color::rgbf(0.25, 0.25, 0.25)));

        // Draw + symbol
        let mut plus_paint = Paint::color(Color::rgbf(0.7, 0.7, 0.7));
        plus_paint.set_font(&[self.font]);
        plus_paint.set_font_size(18.0 * self.scale);
        let plus_x = Self::snap_to_pixel(button_x + new_tab_button_size / 2.0 - 4.0 * self.scale);
        let plus_y = Self::snap_to_pixel(button_y + new_tab_button_size / 2.0 + 6.0 * self.scale);
        let _ = self.canvas.fill_text(plus_x, plus_y, "+", &plus_paint);

        // Store button bounds for click detection
        self.new_tab_button_bounds =
            Some((button_x, button_y, new_tab_button_size, new_tab_button_size));

        // Tab bar bottom line
        let mut line = Path::new();
        line.rect(0.0, tab_height, self.width, 1.0);
        self.canvas
            .fill_path(&line, &Paint::color(Color::rgbf(0.3, 0.3, 0.3)));
    }

    fn draw_text_content(&mut self, tab: &Tab) {
        let tab_height = 36.0 * self.scale;
        let padding = 16.0 * self.scale;
        let line_height = 24.0 * self.scale;
        let start_y = tab_height + padding;
        let scroll_offset = tab.scroll_offset();

        let text = tab.content();
        let cursor_pos = tab.cursor_position();

        // Setup text paint with font
        let mut text_paint = Paint::color(Color::rgbf(
            self.theme.fg.0,
            self.theme.fg.1,
            self.theme.fg.2,
        ));
        text_paint.set_font(&[self.font]);
        // Use physical font size (16 * scale) - scaled up for crisp rendering
        text_paint.set_font_size(16.0 * self.scale);
        let char_width = self.measure_char_width(&text_paint);

        // Draw selection
        if let Some(((start_line, start_col), (end_line, end_col))) = tab.selection_range_line_col()
        {
            let selection_color = Paint::color(Color::rgbf(
                self.theme.selection.0,
                self.theme.selection.1,
                self.theme.selection.2,
            ));

            for line_idx in start_line..=end_line {
                // Skip lines before scroll offset
                if line_idx < scroll_offset {
                    continue;
                }

                let visible_idx = line_idx - scroll_offset;
                let y = start_y + (visible_idx as f32 * line_height);

                // Stop if we're past visible area
                if y > self.height {
                    break;
                }

                let start_x = if line_idx == start_line {
                    padding + (start_col as f32 * char_width)
                } else {
                    padding
                };

                let end_x = if line_idx == end_line {
                    padding + (end_col as f32 * char_width)
                } else {
                    // For full line selection, use the matching line width plus a marker for newline
                    // We need to look up the line content
                    let lines: Vec<&str> = text.lines().collect();
                    let line_len = if line_idx < lines.len() {
                        lines[line_idx].chars().count()
                    } else {
                        0
                    };
                    // Highlight text + newline (approx 0.5 char width or at least min width)
                    padding + ((line_len as f32 + 0.5) * char_width)
                };

                if end_x > start_x {
                    let mut path = Path::new();
                    path.rect(start_x, y, end_x - start_x, line_height);
                    self.canvas.fill_path(&path, &selection_color);
                }
            }
        }

        // Calculate visible line range
        let max_visible_lines = ((self.height - start_y - padding) / line_height).ceil() as usize;

        // Draw text line by line (only visible lines)
        let lines: Vec<&str> = text.lines().collect();
        for (visible_idx, line_idx) in (scroll_offset..).enumerate() {
            if line_idx >= lines.len() {
                break;
            }
            if visible_idx >= max_visible_lines {
                break;
            }

            // Snap to pixel grid to prevent blurry text from bilinear filtering
            let text_x = Self::snap_to_pixel(padding);
            let text_y = Self::snap_to_pixel(
                start_y + (visible_idx as f32 * line_height) + line_height * 0.75,
            );
            let _ = self
                .canvas
                .fill_text(text_x, text_y, lines[line_idx], &text_paint);
        }

        // Draw cursor (adjusted for scroll)
        let (cursor_line, cursor_col) = self.get_cursor_line_col(text, cursor_pos);

        // Only draw cursor if it's in visible range
        if cursor_line >= scroll_offset && cursor_line < scroll_offset + max_visible_lines {
            let visible_cursor_line = cursor_line - scroll_offset;
            let char_width = self.measure_char_width(&text_paint);
            let cursor_x = padding + (cursor_col as f32 * char_width);
            let cursor_y = start_y + (visible_cursor_line as f32 * line_height);

            let mut cursor_path = Path::new();
            cursor_path.rect(cursor_x, cursor_y, 2.0 * self.scale, line_height);
            self.canvas.fill_path(
                &cursor_path,
                &Paint::color(Color::rgbf(
                    self.theme.cursor.0,
                    self.theme.cursor.1,
                    self.theme.cursor.2,
                )),
            );
        }

        // Draw scrollbar
        let total_lines = lines.len().max(1);
        if total_lines > max_visible_lines {
            // Scrollbar logic
            let scrollbar_width = 12.0 * self.scale;
            let scroll_area_height = self.height - tab_height;
            let start_y = tab_height;

            // Calculate thumb height proportion
            let view_ratio = max_visible_lines as f32 / total_lines as f32;
            let thumb_height = (scroll_area_height * view_ratio).max(30.0 * self.scale); // Min thumb height 30px

            // Calculate thumb position
            // Max scrollable lines
            let max_scroll = total_lines.saturating_sub(max_visible_lines);
            let scroll_ratio = if max_scroll > 0 {
                scroll_offset as f32 / max_scroll as f32
            } else {
                0.0
            };

            // Available track height for thumb movement
            let track_height = scroll_area_height - thumb_height;
            let thumb_y = start_y + (track_height * scroll_ratio);
            let thumb_x = self.width - scrollbar_width;

            // Draw track (optional)
            // Draw thumb
            let mut path = Path::new();
            path.rounded_rect(
                thumb_x - 4.0,
                thumb_y,
                scrollbar_width - 4.0,
                thumb_height,
                4.0,
            );

            // Semi-transparent thumb color based on theme FG
            let thumb_color = Paint::color(Color::rgba(
                (self.theme.fg.0 * 255.0) as u8,
                (self.theme.fg.1 * 255.0) as u8,
                (self.theme.fg.2 * 255.0) as u8,
                50, // Alpha 50/255
            ));
            self.canvas.fill_path(&path, &thumb_color);
        }
    }

    fn measure_char_width(&self, paint: &Paint) -> f32 {
        // Measure width of a single character
        if let Ok(metrics) = self.canvas.measure_text(0.0, 0.0, "M", paint) {
            metrics.width()
        } else {
            9.6 // Fallback approximate width
        }
    }

    fn get_cursor_line_col(&self, text: &str, cursor_pos: usize) -> (usize, usize) {
        let mut line = 0;
        let mut col = 0;
        let mut pos = 0;

        for ch in text.chars() {
            if pos >= cursor_pos {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
            pos += 1;
        }

        (line, col)
    }
}
