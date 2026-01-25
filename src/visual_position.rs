//! Visual position utilities for handling tab width and character-to-visual conversions
//!
//! This module provides a centralized abstraction for converting between:
//! - Character positions (how text is stored in the buffer)
//! - Visual positions (how text appears on screen, with tabs taking multiple spaces)

/// Width of a tab character in visual columns
pub const TAB_WIDTH: usize = 4;

/// A line of text with visual position awareness
/// 
/// This type encapsulates a line of text and provides methods to work with
/// visual positions (accounting for tab width) without manual character iteration.
pub struct VisualLine<'a> {
    content: &'a str,
}

impl<'a> VisualLine<'a> {
    /// Create a new VisualLine from string content
    pub fn new(content: &'a str) -> Self {
        Self { content }
    }
    
    /// Convert a character column to a visual column
    #[allow(dead_code)]
    pub fn char_col_to_visual_col(&self, char_col: usize) -> usize {
        char_col_to_visual_col(self.content, char_col)
    }
    
    /// Convert a visual column to a character column
    pub fn visual_col_to_char_col(&self, visual_col: usize) -> usize {
        visual_col_to_char_col(self.content, visual_col)
    }
    
    /// Get the visual x position for a character column
    pub fn char_col_to_visual_x(&self, char_col: usize, base_x: f32, char_width: f32) -> f32 {
        char_col_to_visual_x(self.content, char_col, base_x, char_width)
    }
    
    /// Get the center visual x position for a character column
    pub fn char_col_to_visual_center_x(&self, char_col: usize, base_x: f32, char_width: f32) -> f32 {
        char_col_to_visual_center_x(self.content, char_col, base_x, char_width)
    }
    
    /// Get the character at a given character column
    #[allow(dead_code)]
    pub fn char_at(&self, char_col: usize) -> Option<char> {
        self.content.chars().nth(char_col)
    }
    
    /// Get the visual width of the character at a given character column
    #[allow(dead_code)]
    pub fn char_visual_width_at(&self, char_col: usize) -> usize {
        self.char_at(char_col)
            .map(get_char_visual_width)
            .unwrap_or(1)
    }
    
    /// Get the total visual width of the line
    #[allow(dead_code)]
    pub fn visual_width(&self) -> usize {
        self.content.chars().map(get_char_visual_width).sum()
    }
    
    /// Get the number of characters in the line
    #[allow(dead_code)]
    pub fn char_count(&self) -> usize {
        self.content.chars().count()
    }
}

/// Get the visual width of a character in columns
/// 
/// # Arguments
/// * `ch` - The character to measure
/// 
/// # Returns
/// The number of visual columns this character occupies
#[inline]
pub fn get_char_visual_width(ch: char) -> usize {
    match ch {
        '\t' => TAB_WIDTH,
        '\n' => 0,
        _ => 1,
    }
}

/// Convert a character column to a visual column for a given line content
/// 
/// # Arguments
/// * `line_content` - The text content of the line
/// * `char_col` - The character column position (0-based)
/// 
/// # Returns
/// The visual column position accounting for tab width
#[allow(dead_code)]
pub fn char_col_to_visual_col(line_content: &str, char_col: usize) -> usize {
    let mut visual_col = 0;
    
    for (idx, ch) in line_content.chars().enumerate() {
        if idx >= char_col {
            break;
        }
        visual_col += get_char_visual_width(ch);
    }
    
    visual_col
}

/// Convert a visual column to a character column for a given line content
/// 
/// # Arguments
/// * `line_content` - The text content of the line
/// * `visual_col` - The visual column position (0-based)
/// 
/// # Returns
/// The character column position that corresponds to the visual column
pub fn visual_col_to_char_col(line_content: &str, visual_col: usize) -> usize {
    let mut char_col = 0;
    let mut current_visual_col = 0;
    
    for ch in line_content.chars() {
        if current_visual_col >= visual_col {
            break;
        }
        
        current_visual_col += get_char_visual_width(ch);
        char_col += 1;
    }
    
    char_col
}

/// Calculate the visual x position for a character at a given column
/// 
/// # Arguments
/// * `line_content` - The text content of the line
/// * `char_col` - The character column position
/// * `base_x` - The base x position (e.g., padding - scroll_x)
/// * `char_width` - The width of a single character in pixels
/// 
/// # Returns
/// The visual x position in pixels
pub fn char_col_to_visual_x(line_content: &str, char_col: usize, base_x: f32, char_width: f32) -> f32 {
    let mut visual_x = base_x;
    
    for (idx, ch) in line_content.chars().enumerate() {
        if idx >= char_col {
            break;
        }
        visual_x += char_width * get_char_visual_width(ch) as f32;
    }
    
    visual_x
}

/// Calculate the center visual x position for a character at a given column
/// 
/// # Arguments
/// * `line_content` - The text content of the line
/// * `char_col` - The character column position
/// * `base_x` - The base x position (e.g., padding - scroll_x)
/// * `char_width` - The width of a single character in pixels
/// 
/// # Returns
/// The center visual x position in pixels, accounting for tab width
pub fn char_col_to_visual_center_x(line_content: &str, char_col: usize, base_x: f32, char_width: f32) -> f32 {
    let start_x = char_col_to_visual_x(line_content, char_col, base_x, char_width);
    
    // Get the character at this position to determine its visual width
    let ch = line_content.chars().nth(char_col).unwrap_or(' ');
    let char_visual_width = get_char_visual_width(ch);
    
    start_x + (char_width * char_visual_width as f32 * 0.5)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_to_visual_no_tabs() {
        assert_eq!(char_col_to_visual_col("hello", 0), 0);
        assert_eq!(char_col_to_visual_col("hello", 3), 3);
        assert_eq!(char_col_to_visual_col("hello", 5), 5);
    }

    #[test]
    fn test_char_to_visual_with_tabs() {
        assert_eq!(char_col_to_visual_col("\thello", 0), 0);
        assert_eq!(char_col_to_visual_col("\thello", 1), 4); // After tab
        assert_eq!(char_col_to_visual_col("\thello", 2), 5); // After tab + 'h'
        assert_eq!(char_col_to_visual_col("a\tb", 1), 1);    // After 'a'
        assert_eq!(char_col_to_visual_col("a\tb", 2), 5);    // After 'a' + tab
    }

    #[test]
    fn test_visual_to_char_no_tabs() {
        assert_eq!(visual_col_to_char_col("hello", 0), 0);
        assert_eq!(visual_col_to_char_col("hello", 3), 3);
        assert_eq!(visual_col_to_char_col("hello", 5), 5);
    }

    #[test]
    fn test_visual_to_char_with_tabs() {
        assert_eq!(visual_col_to_char_col("\thello", 0), 0);
        assert_eq!(visual_col_to_char_col("\thello", 3), 0); // Still on tab
        assert_eq!(visual_col_to_char_col("\thello", 4), 1); // After tab
        assert_eq!(visual_col_to_char_col("\thello", 5), 2); // After tab + 'h'
    }

    #[test]
    fn test_visual_x_calculation() {
        let base_x = 10.0;
        let char_width = 8.0;
        
        assert_eq!(char_col_to_visual_x("hello", 0, base_x, char_width), 10.0);
        assert_eq!(char_col_to_visual_x("hello", 3, base_x, char_width), 34.0); // 10 + 3*8
        assert_eq!(char_col_to_visual_x("\thello", 1, base_x, char_width), 42.0); // 10 + 4*8
    }
}
