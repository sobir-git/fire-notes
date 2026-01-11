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
    /// Cursor color
    pub cursor: (f32, f32, f32),
    /// Selection color
    pub selection: (f32, f32, f32, f32), // RGBA
}

impl Theme {
    /// Dark theme (default)
    pub fn dark() -> Self {
        Self {
            bg: (0.11, 0.11, 0.13),           // #1c1c21
            fg: (0.9, 0.9, 0.9),              // #e6e6e6
            tab_active: (0.18, 0.18, 0.22),   // #2e2e38
            tab_inactive: (0.13, 0.13, 0.16), // #212126
            cursor: (0.4, 0.7, 1.0),          // Light blue
            selection: (0.3, 0.5, 0.8, 0.4),  // Blue with alpha
        }
    }

    /// Light theme
    #[allow(dead_code)]
    pub fn light() -> Self {
        Self {
            bg: (0.98, 0.98, 0.98),           // #fafafa
            fg: (0.1, 0.1, 0.1),              // #1a1a1a
            tab_active: (1.0, 1.0, 1.0),      // White
            tab_inactive: (0.92, 0.92, 0.92), // #ebebeb
            cursor: (0.2, 0.4, 0.8),          // Blue
            selection: (0.3, 0.5, 0.8, 0.3),  // Blue with alpha
        }
    }
}
