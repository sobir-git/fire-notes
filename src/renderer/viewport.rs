//! Viewport bounds and visibility culling
//!
//! Provides a clean abstraction for determining what's visible on screen,
//! used by both text rendering and flame effects.

#![allow(dead_code)] // Many methods are for future use / API completeness

/// Represents the visible viewport bounds in screen coordinates
#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    /// Left edge of visible area (x coordinate)
    pub left: f32,
    /// Right edge of visible area (x coordinate)  
    pub right: f32,
    /// Top edge of visible area (y coordinate)
    pub top: f32,
    /// Bottom edge of visible area (y coordinate)
    pub bottom: f32,
    /// Display scale factor
    pub scale: f32,
}

impl Viewport {
    /// Create a viewport from screen dimensions and content area
    pub fn new(width: f32, height: f32, content_top: f32, scale: f32) -> Self {
        Self {
            left: 0.0,
            right: width,
            top: content_top,
            bottom: height,
            scale,
        }
    }

    /// Check if a point is within the visible viewport
    #[inline]
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }

    /// Check if a point is within horizontal bounds (with margin)
    #[inline]
    pub fn is_horizontally_visible(&self, x: f32, margin: f32) -> bool {
        x >= self.left - margin && x <= self.right + margin
    }

    /// Check if a point is within vertical bounds (with margin)
    #[inline]
    pub fn is_vertically_visible(&self, y: f32, margin: f32) -> bool {
        y >= self.top - margin && y <= self.bottom + margin
    }

    /// Check if a rectangle overlaps with the viewport
    #[inline]
    pub fn intersects_rect(&self, x: f32, y: f32, width: f32, height: f32) -> bool {
        x + width >= self.left && x <= self.right && 
        y + height >= self.top && y <= self.bottom
    }

    /// Get the visible width
    #[inline]
    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    /// Get the visible height
    #[inline]
    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_contains_point() {
        let vp = Viewport::new(800.0, 600.0, 40.0, 1.0);
        assert!(vp.contains_point(400.0, 300.0)); // center
        assert!(!vp.contains_point(400.0, 20.0)); // above content area
        assert!(!vp.contains_point(-10.0, 300.0)); // left of screen
    }

    #[test]
    fn test_viewport_horizontal_visibility() {
        let vp = Viewport::new(800.0, 600.0, 40.0, 1.0);
        assert!(vp.is_horizontally_visible(400.0, 0.0));
        assert!(vp.is_horizontally_visible(-5.0, 10.0)); // within margin
        assert!(!vp.is_horizontally_visible(-20.0, 10.0)); // outside margin
    }
}
