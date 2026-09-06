//! Theme colors for the editor

pub struct Theme {
    /// Background color (RGB 0.0-1.0)
    pub bg: (f32, f32, f32),
    /// Foreground/text color
    pub fg: (f32, f32, f32),
    /// Active tab background
    pub tab_active: (f32, f32, f32),
    /// Inactive tab background
    pub tab_inactive: (f32, f32, f32),
    /// Hovered tab background
    pub tab_hover: (f32, f32, f32),
    /// Active tab accent border/line
    pub tab_active_border: (f32, f32, f32),
    /// General UI button background
    pub button_bg: (f32, f32, f32),
    /// General UI button hover background
    pub button_hover: (f32, f32, f32),
    /// General UI button foreground (text/icon)
    pub button_fg: (f32, f32, f32),
    /// Border color for UI elements
    pub border: (f32, f32, f32),
    /// Cursor color
    pub cursor: (f32, f32, f32),

    // ── List tokens ───────────────────────────────────────────────────────
    /// Row text color
    pub list_row_fg: (f32, f32, f32),
    /// Selected row text color
    pub list_sel_fg: (f32, f32, f32),
    /// Selected row background tint
    pub list_sel_bg: (f32, f32, f32, f32),
    /// Accent color (open-file dot, scrollbar thumb)
    pub list_accent: (f32, f32, f32, f32),
    /// Empty list placeholder text color
    pub list_empty_fg: (f32, f32, f32, f32),
    /// Scrollbar track color
    pub list_scrollbar_track: (f32, f32, f32, f32),

    // ── Overlay tokens ────────────────────────────────────────────────────
    /// Panel background
    pub overlay_bg: (f32, f32, f32, f32),
    /// Panel border color
    pub overlay_border: (f32, f32, f32, f32),
    /// Panel border width in logical pixels
    pub overlay_border_width: f32,
    /// Panel corner radius in logical pixels
    pub overlay_radius: f32,
    /// Dimmed backdrop behind the panel
    pub overlay_backdrop: (f32, f32, f32, f32),
    /// Default font size inside overlays in logical pixels
    pub overlay_font_size: f32,
}

impl Theme {
    /// Dark theme (default)
    pub fn dark() -> Self {
        Self {
            bg: (0.0, 0.0, 0.0),                 // Pure black
            fg: (1.0, 0.9, 0.8),                // Warm off-white
            tab_active: (0.15, 0.05, 0.05),     // Dark deep red
            tab_inactive: (0.05, 0.02, 0.02),   // Very dark red/black
            tab_hover: (0.25, 0.1, 0.05),       // Fire orange-red
            tab_active_border: (1.0, 0.4, 0.0), // Bright fire orange
            button_bg: (0.1, 0.03, 0.03),      // Dark ember
            button_hover: (0.3, 0.1, 0.05),     // Glowing coal
            button_fg: (1.0, 0.6, 0.0),         // Flame yellow-orange
            border: (0.2, 0.05, 0.05),          // Deep ember border
            cursor: (1.0, 0.8, 0.0),            // Bright yellow flame

            list_row_fg:          (0.78, 0.78, 0.78),
            list_sel_fg:          (1.0,  1.0,  1.0),
            list_sel_bg:          (0.4,  0.7,  1.0,  0.12),
            list_accent:          (0.4,  0.7,  1.0,  1.0),
            list_empty_fg:        (0.59, 0.59, 0.59, 0.71),
            list_scrollbar_track: (0.24, 0.24, 0.24, 0.47),

            overlay_bg:           (0.13, 0.13, 0.15, 1.0),
            overlay_border:       (0.4,  0.7,  1.0,  0.55),
            overlay_border_width: 2.0,
            overlay_radius:       8.0,
            overlay_backdrop:     (0.0,  0.0,  0.0,  0.47),
            overlay_font_size:    14.0,
        }
    }

    /// Light theme
    #[allow(dead_code)]
    pub fn light() -> Self {
        Self {
            bg: (0.98, 0.98, 0.98),             // #fafafa
            fg: (0.1, 0.1, 0.1),                // #1a1a1a
            tab_active: (1.0, 1.0, 1.0),        // White
            tab_inactive: (0.92, 0.92, 0.92),   // #ebebeb
            tab_hover: (0.95, 0.95, 0.95),      // Slight grey
            tab_active_border: (0.2, 0.4, 0.8), // Blue accent
            button_bg: (0.95, 0.95, 0.95),
            button_hover: (0.9, 0.9, 0.9),
            button_fg: (0.2, 0.4, 0.8), // Blue accent
            border: (0.85, 0.85, 0.85),
            cursor: (0.2, 0.4, 0.8),         // Blue

            list_row_fg:          (0.2,  0.2,  0.2),
            list_sel_fg:          (0.0,  0.0,  0.0),
            list_sel_bg:          (0.2,  0.4,  0.8,  0.12),
            list_accent:          (0.2,  0.4,  0.8,  1.0),
            list_empty_fg:        (0.5,  0.5,  0.5,  0.7),
            list_scrollbar_track: (0.8,  0.8,  0.8,  0.5),

            overlay_bg:           (0.98, 0.98, 0.98, 1.0),
            overlay_border:       (0.2,  0.4,  0.8,  0.8),
            overlay_border_width: 1.5,
            overlay_radius:       8.0,
            overlay_backdrop:     (0.0,  0.0,  0.0,  0.3),
            overlay_font_size:    14.0,
        }
    }
}
