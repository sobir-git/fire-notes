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
        }
    }
}
