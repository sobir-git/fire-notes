//! Scroll state management and event processing
//!
//! This module provides a robust, centralized abstraction for handling scroll events
//! with proper state management and configuration.

use std::time::{Duration, Instant};

use crate::config::scroll;

/// Scroll direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
}

/// Scroll input type from the OS
#[derive(Debug, Clone, Copy)]
pub enum ScrollInput {
    /// Discrete wheel notches (e.g., mouse wheel)
    LineDelta(f32),
    /// Continuous pixel-based scrolling (e.g., touchpad)
    PixelDelta(f32),
}

impl ScrollInput {
    /// Convert scroll input to normalized scroll amount
    /// Returns (direction, magnitude) where magnitude is number of scroll units
    pub fn to_scroll_amount(self) -> Option<(ScrollDirection, u32)> {
        let (delta, divisor) = match self {
            ScrollInput::LineDelta(y) => (y, 1.0),
            ScrollInput::PixelDelta(y) => (y, 15.0), // ~15px per scroll unit
        };

        if delta.abs() < 0.001 {
            return None;
        }

        let direction = if delta > 0.0 {
            ScrollDirection::Up
        } else {
            ScrollDirection::Down
        };

        let magnitude = (delta.abs() / divisor).max(1.0) as u32;
        Some((direction, magnitude))
    }
}

/// Configuration for scroll behavior
#[derive(Debug, Clone, Copy)]
pub struct ScrollConfig {
    /// Lines to scroll per wheel tick/unit
    pub lines_per_tick: usize,
    /// Whether to ignore the first scroll notch for smoother feel
    pub ignore_first_notch: bool,
    /// Timeout after which scroll state resets (in milliseconds)
    pub reset_timeout_ms: u64,
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            lines_per_tick: scroll::LINES_PER_WHEEL_TICK,
            ignore_first_notch: true,
            reset_timeout_ms: 1000,
        }
    }
}

/// Scroll state machine
#[derive(Debug)]
pub struct ScrollState {
    config: ScrollConfig,
    /// Whether we've ignored the first notch in this scroll session
    first_notch_ignored: bool,
    /// Last scroll event timestamp for timeout detection
    last_scroll_time: Option<Instant>,
}

impl ScrollState {
    /// Create new scroll state with default configuration
    pub fn new() -> Self {
        Self::with_config(ScrollConfig::default())
    }

    /// Create new scroll state with custom configuration
    pub fn with_config(config: ScrollConfig) -> Self {
        Self {
            config,
            first_notch_ignored: false,
            last_scroll_time: None,
        }
    }

    /// Process a scroll input and return the number of lines to scroll
    /// Returns None if the scroll should be ignored (e.g., first notch)
    pub fn process_scroll(&mut self, input: ScrollInput) -> Option<(ScrollDirection, usize)> {
        let (direction, magnitude) = input.to_scroll_amount()?;

        // Check if we should reset state due to timeout
        if let Some(last_time) = self.last_scroll_time {
            if last_time.elapsed() > Duration::from_millis(self.config.reset_timeout_ms) {
                self.reset();
            }
        }

        self.last_scroll_time = Some(Instant::now());

        // Handle first notch ignore logic
        if self.config.ignore_first_notch && !self.first_notch_ignored {
            self.first_notch_ignored = true;
            return None; // Ignore first notch
        }

        // Calculate total lines to scroll
        let lines = magnitude as usize * self.config.lines_per_tick;
        Some((direction, lines))
    }

    /// Reset the scroll state (call when scroll interaction ends)
    pub fn reset(&mut self) {
        self.first_notch_ignored = false;
        self.last_scroll_time = None;
    }

    /// Get current configuration
    #[allow(dead_code)]
    pub fn config(&self) -> &ScrollConfig {
        &self.config
    }

    /// Update configuration
    #[allow(dead_code)]
    pub fn set_config(&mut self, config: ScrollConfig) {
        self.config = config;
    }
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scroll_input_conversion() {
        let input = ScrollInput::LineDelta(3.0);
        let result = input.to_scroll_amount();
        assert_eq!(result, Some((ScrollDirection::Up, 3)));

        let input = ScrollInput::LineDelta(-2.0);
        let result = input.to_scroll_amount();
        assert_eq!(result, Some((ScrollDirection::Down, 2)));

        let input = ScrollInput::PixelDelta(30.0);
        let result = input.to_scroll_amount();
        assert_eq!(result, Some((ScrollDirection::Up, 2)));
    }

    #[test]
    fn test_first_notch_ignore() {
        let mut state = ScrollState::new();
        
        // First scroll should be ignored
        let result = state.process_scroll(ScrollInput::LineDelta(1.0));
        assert_eq!(result, None);

        // Second scroll should work
        let result = state.process_scroll(ScrollInput::LineDelta(1.0));
        assert_eq!(result, Some((ScrollDirection::Up, 1)));

        // Reset and test again
        state.reset();
        let result = state.process_scroll(ScrollInput::LineDelta(1.0));
        assert_eq!(result, None);
    }

    #[test]
    fn test_scroll_without_first_notch_ignore() {
        let config = ScrollConfig {
            ignore_first_notch: false,
            ..Default::default()
        };
        let mut state = ScrollState::with_config(config);

        // First scroll should work immediately
        let result = state.process_scroll(ScrollInput::LineDelta(1.0));
        assert_eq!(result, Some((ScrollDirection::Up, 1)));
    }
}
