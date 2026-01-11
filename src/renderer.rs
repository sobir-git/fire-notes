//! GPU-accelerated rendering with femtovg

use crate::tab::Tab;
use crate::theme::Theme;
use femtovg::{Canvas, Color, FontId, Paint, Path, renderer::OpenGl};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HitTestResult {
    Tab(usize),
    NewTabButton,
}

pub struct Renderer {
    canvas: Canvas<OpenGl>,
    fonts: Vec<FontId>,
    theme: Theme,
    width: f32,
    height: f32,
    scale: f32,
    tab_scroll_x: f32,
    // (x, y, width, height)
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

        // Load fonts with fallbacks
        let fonts = Self::load_fonts(&mut canvas);

        let theme = Theme::dark();

        Self {
            canvas,
            fonts,
            theme,
            width,
            height,
            scale,
            tab_scroll_x: 0.0,
        }
    }

    fn load_fonts(canvas: &mut Canvas<OpenGl>) -> Vec<FontId> {
        let mut fonts = Vec::new();

        // 1. Try common monospace font paths on Linux
        let mono_paths = [
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
            "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf",
            "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
            "/usr/share/fonts/dejavu/DejaVuSansMono.ttf",
        ];

        for path in &mono_paths {
            if let Ok(font) = canvas.add_font(path) {
                fonts.push(font);
                break; // Use the first available monospace font
            }
        }

        // 2. Add fallback fonts for extended coverage (Cyrillic, CJK, etc.)
        // These might not be monospace, but better than a box.
        let fallback_paths = [
            "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf", // Excellent fallback
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",           // Good generic coverage
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/TTF/DejaVuSans.ttf",
        ];

        for path in &fallback_paths {
            if let Ok(font) = canvas.add_font(path) {
                // Avoid adding duplicates if we somehow loaded the same file?
                // FontId is unique per add_font call usually.
                fonts.push(font);
            }
        }

        // 3. Fallback: if no fonts loaded at all, try to find any TTF
        if fonts.is_empty() {
            if let Ok(entries) = std::fs::read_dir("/usr/share/fonts/truetype") {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        if let Ok(sub_entries) = std::fs::read_dir(entry.path()) {
                            for sub_entry in sub_entries.flatten() {
                                let path = sub_entry.path();
                                if path.extension().map(|e| e == "ttf").unwrap_or(false) {
                                    if let Ok(font) = canvas.add_font(path) {
                                        fonts.push(font);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if fonts.is_empty() {
            panic!(
                "No suitable font found! Please install dejavu-fonts, liberation-fonts, or fonts-droid-fallback."
            );
        }

        fonts
    }

    pub fn resize(&mut self, width: f32, height: f32, scale: f32) {
        self.width = width;
        self.height = height;
        self.scale = scale;
    }

    pub fn set_tab_scroll_x(&mut self, scroll: f32) {
        self.tab_scroll_x = scroll;
    }

    pub fn render(
        &mut self,
        tabs: &[(&str, bool)],
        current_tab: &Tab,
        cursor_visible: bool,
        hovered_tab_index: Option<usize>,
        hovered_plus: bool,
    ) {
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
        self.draw_tab_bar(tabs, hovered_tab_index, hovered_plus);

        // Draw text content
        self.draw_text_content(current_tab, cursor_visible);

        self.canvas.flush();
    }

    pub fn hit_test(&self, x: f32, y: f32, tabs: &[(&str, bool)]) -> Option<HitTestResult> {
        let tab_height = 40.0 * self.scale;

        if y > tab_height {
            return None;
        }

        let mut current_x = -self.tab_scroll_x;
        let tab_padding = 16.0 * self.scale;

        // Check tabs
        for (i, (title, _)) in tabs.iter().enumerate() {
            let tab_width =
                (title.len() as f32 * 9.0 * self.scale + tab_padding * 2.0).max(100.0 * self.scale);

            // Optimization: check if relevant area
            if current_x + tab_width > 0.0 && current_x < self.width {
                if x >= current_x && x < current_x + tab_width {
                    return Some(HitTestResult::Tab(i));
                }
            }

            current_x += tab_width + 1.0;
        }

        // Check new tab button - flows with tabs
        let new_tab_button_size = 28.0 * self.scale;
        let button_x = current_x + 8.0 * self.scale;
        let button_y = (tab_height - new_tab_button_size) / 2.0;

        if x >= button_x
            && x <= button_x + new_tab_button_size
            && y >= button_y
            && y <= button_y + new_tab_button_size
        {
            return Some(HitTestResult::NewTabButton);
        }

        None
    }

    fn draw_tab_bar(
        &mut self,
        tabs: &[(&str, bool)],
        hovered_tab_index: Option<usize>,
        hovered_plus: bool,
    ) {
        let tab_height = 40.0 * self.scale;
        let tab_padding = 16.0 * self.scale;

        // Save state for clipping
        self.canvas.save();

        // Clip to tab bar area to prevent drawing outside when scrolling
        self.canvas
            .intersect_scissor(0.0, 0.0, self.width, tab_height);

        let mut x = -self.tab_scroll_x;

        for (i, (title, is_active)) in tabs.iter().enumerate() {
            let tab_width =
                (title.len() as f32 * 9.0 * self.scale + tab_padding * 2.0).max(100.0 * self.scale);

            // Optimization: skip drawing off-screen tabs
            if x + tab_width < 0.0 {
                x += tab_width + 1.0;
                continue;
            }
            if x > self.width {
                // If we're past the width, we still need to calculate X for the plus button,
                // but we can break the loop if we assume plus button is always at end.
                // However, we need to know the final X. So we can't just break unless we calculate remaining width.
                // For simplicity, let's just continue loop but not draw?
                // Actually, let's just draw properly. Canvas handles clipping.
            }

            // Tab background
            let mut path = Path::new();
            path.rect(x, 0.0, tab_width, tab_height);

            let color = if *is_active {
                Color::rgbf(
                    self.theme.tab_active.0,
                    self.theme.tab_active.1,
                    self.theme.tab_active.2,
                )
            } else if Some(i) == hovered_tab_index {
                // Slightly lighter than inactive for hover
                let c = self.theme.tab_inactive;
                Color::rgbf(c.0 + 0.05, c.1 + 0.05, c.2 + 0.05)
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
            text_paint.set_font(&self.fonts);
            text_paint.set_font_size(14.0 * self.scale);

            let text_x = Self::snap_to_pixel(x + tab_padding);
            let text_y = Self::snap_to_pixel(tab_height / 2.0 + 5.0 * self.scale);
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

        let btn_color = if hovered_plus {
            Color::rgbf(0.35, 0.35, 0.35)
        } else {
            Color::rgbf(0.25, 0.25, 0.25)
        };
        self.canvas.fill_path(&btn_path, &Paint::color(btn_color));

        // Draw + symbol
        let mut plus_paint = Paint::color(Color::rgbf(0.7, 0.7, 0.7));
        plus_paint.set_font(&self.fonts);
        plus_paint.set_font_size(20.0 * self.scale); // Slightly larger

        // Measure to center perfectly
        let mut plus_width = 0.0;
        if let Ok(metrics) = self.canvas.measure_text(0.0, 0.0, "+", &plus_paint) {
            plus_width = metrics.width();
        }

        // Center X: button_x + (button_width - text_width) / 2
        // Center Y: button_y + button_height / 2 + text_height_adjustment
        // heuristic for vertical center with this font: + 7.0 * scale
        let plus_x = Self::snap_to_pixel(button_x + (new_tab_button_size - plus_width) / 2.0);
        let plus_y = Self::snap_to_pixel(button_y + new_tab_button_size / 2.0 + 7.0 * self.scale);
        let _ = self.canvas.fill_text(plus_x, plus_y, "+", &plus_paint);

        // Restore state (clear clipping)
        self.canvas.restore();

        // Tab bar bottom line
        let mut line = Path::new();
        line.rect(0.0, tab_height, self.width, 1.0);
        self.canvas
            .fill_path(&line, &Paint::color(Color::rgbf(0.3, 0.3, 0.3)));
    }

    fn draw_text_content(&mut self, tab: &Tab, cursor_visible: bool) {
        let tab_height = 40.0 * self.scale;
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
        text_paint.set_font(&self.fonts);
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

            let line = lines[line_idx];
            let line_y = start_y + (visible_idx as f32 * line_height) + line_height * 0.75;
            let line_y_snapped = Self::snap_to_pixel(line_y);

            // let mut col = 0;
            let mut x_offset = padding;

            let mut buf = [0u8; 4];

            for ch in line.chars() {
                // Determine width of this character in grid cells
                let advance = if ch == '\t' {
                    4 // minimal tab handling
                } else {
                    1
                };

                // Draw non-whitespace characters
                if !ch.is_control() && ch != ' ' {
                    // Force grid alignment: always draw at computed grid position
                    let text_x = Self::snap_to_pixel(x_offset);

                    let s = ch.encode_utf8(&mut buf);
                    let _ = self
                        .canvas
                        .fill_text(text_x, line_y_snapped, s, &text_paint);
                }

                // Advance X by grid size
                x_offset += char_width * advance as f32;
                // col += advance; // This line is removed as per instruction
            }
        }

        // Draw cursor (adjusted for scroll)
        if cursor_visible {
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
