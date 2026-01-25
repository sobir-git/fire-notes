//! GPU-accelerated rendering with femtovg

use crate::tab::Tab;
use crate::theme::Theme;
use crate::ui::ScrollbarWidget;
use femtovg::{Canvas, Color, FontId, Paint, Path, renderer::OpenGl};

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
        hovered_scrollbar: bool,
        dragging_scrollbar: bool,
        renaming_tab: Option<usize>,
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
        self.draw_tab_bar(tabs, hovered_tab_index, hovered_plus, renaming_tab);

        // Draw text content
        self.draw_text_content(current_tab, cursor_visible, hovered_scrollbar, dragging_scrollbar);

        self.canvas.flush();
    }

    fn draw_tab_bar(
        &mut self,
        tabs: &[(&str, bool)],
        hovered_tab_index: Option<usize>,
        hovered_plus: bool,
        renaming_tab: Option<usize>,
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
                Color::rgbf(
                    self.theme.tab_hover.0,
                    self.theme.tab_hover.1,
                    self.theme.tab_hover.2,
                )
            } else {
                Color::rgbf(
                    self.theme.tab_inactive.0,
                    self.theme.tab_inactive.1,
                    self.theme.tab_inactive.2,
                )
            };

            self.canvas.fill_path(&path, &Paint::color(color));

            // Active tab indicator (top line)
            if *is_active {
                let mut indicator = Path::new();
                indicator.rect(x, 0.0, tab_width, 2.0 * self.scale);
                self.canvas.fill_path(
                    &indicator,
                    &Paint::color(Color::rgbf(
                        self.theme.tab_active_border.0,
                        self.theme.tab_active_border.1,
                        self.theme.tab_active_border.2,
                    )),
                );
            }

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

            // Draw underline if this tab is being renamed
            if Some(i) == renaming_tab {
                let metrics = self
                    .canvas
                    .measure_text(text_x, text_y, title, &text_paint)
                    .unwrap_or_default();
                let text_width = metrics.width();
                let underline_y = text_y + 12.0 * self.scale;
                let mut underline_path = Path::new();
                underline_path.move_to(text_x, underline_y);
                underline_path.line_to(text_x + text_width, underline_y);
                let mut underline_paint = Paint::color(Color::rgbf(
                    self.theme.fg.0,
                    self.theme.fg.1,
                    self.theme.fg.2,
                ));
                underline_paint.set_line_width(2.0 * self.scale);
                self.canvas.stroke_path(&underline_path, &underline_paint);
            }

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
            Color::rgbf(
                self.theme.button_hover.0,
                self.theme.button_hover.1,
                self.theme.button_hover.2,
            )
        } else {
            Color::rgbf(
                self.theme.button_bg.0,
                self.theme.button_bg.1,
                self.theme.button_bg.2,
            )
        };
        self.canvas.fill_path(&btn_path, &Paint::color(btn_color));

        // Draw + symbol
        let mut plus_paint = Paint::color(Color::rgbf(
            self.theme.button_fg.0,
            self.theme.button_fg.1,
            self.theme.button_fg.2,
        ));
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
        self.canvas.fill_path(
            &line,
            &Paint::color(Color::rgbf(
                self.theme.border.0,
                self.theme.border.1,
                self.theme.border.2,
            )),
        );
    }

    fn draw_text_content(
        &mut self,
        tab: &Tab,
        cursor_visible: bool,
        hovered_scrollbar: bool,
        dragging_scrollbar: bool,
    ) {
        let tab_height = 40.0 * self.scale;
        let padding = 16.0 * self.scale;
        let line_height = 24.0 * self.scale;
        let start_y = tab_height + padding;
        let scroll_offset = tab.scroll_offset();
        let scroll_x = tab.scroll_offset_x();
        let do_wrap = tab.word_wrap();

        let text = tab.content();
        let cursor_pos = tab.cursor_position();

        // Setup text paint
        let mut text_paint = Paint::color(Color::rgbf(
            self.theme.fg.0,
            self.theme.fg.1,
            self.theme.fg.2,
        ));
        text_paint.set_font(&self.fonts);
        text_paint.set_font_size(16.0 * self.scale);
        let char_width = self.measure_char_width(&text_paint);

        // Draw selection (Limited support for wrapping currently)
        if !do_wrap {
            if let Some(((start_line, start_col), (end_line, end_col))) =
                tab.selection_range_line_col()
            {
                let selection_color = Paint::color(Color::rgbf(
                    self.theme.selection.0,
                    self.theme.selection.1,
                    self.theme.selection.2,
                ));

                for line_idx in start_line..=end_line {
                    if line_idx < scroll_offset {
                        continue;
                    }
                    let visible_idx = line_idx - scroll_offset;
                    let y = start_y + (visible_idx as f32 * line_height);
                    if y > self.height {
                        break;
                    }

                    let start_x = if line_idx == start_line {
                        padding - scroll_x + (start_col as f32 * char_width)
                    } else {
                        padding - scroll_x
                    };

                    let end_x = if line_idx == end_line {
                        padding - scroll_x + (end_col as f32 * char_width)
                    } else {
                        // For full line selection, approximate width
                        let lines: Vec<&str> = text.lines().collect();
                        let line_len = if line_idx < lines.len() {
                            lines[line_idx].chars().count()
                        } else {
                            0
                        };
                        padding - scroll_x + ((line_len as f32 + 0.5) * char_width)
                    };

                    if end_x > start_x {
                        let mut path = Path::new();
                        path.rect(start_x, y, end_x - start_x, line_height);
                        self.canvas.fill_path(&path, &selection_color);
                    }
                }
            }
        }

        // Draw text and cursor
        let lines: Vec<&str> = text.lines().skip(scroll_offset).collect();
        let mut current_y = start_y;
        let (cursor_line_idx, cursor_col_idx) = self.get_cursor_line_col(text, cursor_pos);
        let mut cursor_rect = None;

        for (idx, line) in lines.iter().enumerate() {
            let logical_line_idx = scroll_offset + idx;
            if current_y > self.height {
                break;
            }

            let mut x_offset = if do_wrap { padding } else { padding - scroll_x };
            let line_has_cursor = logical_line_idx == cursor_line_idx;

            // Check cursor at start of line (col 0)
            if line_has_cursor && cursor_col_idx == 0 {
                cursor_rect = Some((x_offset, current_y));
            }

            let mut current_col = 0;
            let mut line_chars = line.chars(); // Use iterator

            while let Some(ch) = line_chars.next() {
                let advance = if ch == '\t' { 4 } else { 1 };
                let char_w = char_width * advance as f32;

                // Wrap check
                if do_wrap && x_offset + char_w > self.width - padding {
                    current_y += line_height;
                    x_offset = padding;
                    if current_y > self.height {
                        break;
                    }
                }

                if current_y + line_height > 0.0 && current_y < self.height {
                    if !ch.is_control() && ch != ' ' {
                        let text_x = Self::snap_to_pixel(x_offset);
                        let text_y_snapped = Self::snap_to_pixel(current_y + line_height * 0.75);
                        let mut buf = [0u8; 4];
                        let s = ch.encode_utf8(&mut buf);
                        let _ = self
                            .canvas
                            .fill_text(text_x, text_y_snapped, s, &text_paint);
                    }
                }

                x_offset += char_w;
                current_col += 1;

                if line_has_cursor && current_col == cursor_col_idx {
                    cursor_rect = Some((x_offset, current_y));
                }
            }

            // Check if cursor is at end of line (after last character)
            if line_has_cursor && cursor_col_idx == current_col && cursor_rect.is_none() {
                cursor_rect = Some((x_offset, current_y));
            }

            // Move to next line
            current_y += line_height;
        }

        // Handle cursor if it's past the last line (e.g. empty file or cursor at very end)
        if cursor_rect.is_none() && scroll_offset <= cursor_line_idx {
            // If cursor is beyond the last line, place it at the beginning of a new line
            if cursor_line_idx >= lines.len() {
                // Calculate y position for the cursor line
                let cursor_y = start_y + (cursor_line_idx as f32 * line_height);
                cursor_rect = Some((padding - scroll_x, cursor_y));
            }
            // Exception: Empty file
            else if text.is_empty() {
                cursor_rect = Some((padding - scroll_x, start_y));
            }
        }

        // Draw Cursor
        if cursor_visible {
            if let Some((cx, cy)) = cursor_rect {
                let mut cursor_path = Path::new();
                cursor_path.rect(cx, cy, 2.0 * self.scale, line_height);
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
        let max_visible_lines = ((self.height - start_y - padding) / line_height).ceil() as usize;
        let total_lines = tab.total_lines().max(1);
        if total_lines > max_visible_lines {
            let scrollbar = ScrollbarWidget::new(self.width, self.height, self.scale);
            if let Some(metrics) = scrollbar.metrics(total_lines, max_visible_lines, scroll_offset) {
                let mut path = Path::new();
                path.rounded_rect(
                    metrics.thumb.x,
                    metrics.thumb.y,
                    metrics.thumb.width,
                    metrics.thumb.height,
                    4.0,
                );

                let thumb_alpha = if dragging_scrollbar {
                    140
                } else if hovered_scrollbar {
                    90
                } else {
                    50
                };
                // Semi-transparent thumb color based on theme FG
                let thumb_color = Paint::color(Color::rgba(
                    (self.theme.fg.0 * 255.0) as u8,
                    (self.theme.fg.1 * 255.0) as u8,
                    (self.theme.fg.2 * 255.0) as u8,
                    thumb_alpha,
                ));
                self.canvas.fill_path(&path, &thumb_color);
            }
        }
    }

    pub fn get_char_width(&self) -> f32 {
        let mut text_paint = Paint::color(Color::rgb(255, 255, 255));
        text_paint.set_font(&self.fonts);
        text_paint.set_font_size(16.0 * self.scale);
        self.measure_char_width(&text_paint)
    }

    fn measure_char_width(&self, paint: &Paint) -> f32 {
        // Measure width of a single character
        if let Ok(metrics) = self.canvas.measure_text(0.0, 0.0, "M", paint) {
            metrics.width()
        } else {
            9.6 * self.scale // Fallback approximate width
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
