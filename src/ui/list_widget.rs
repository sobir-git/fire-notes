//! Reusable list widget with keyboard and mouse support
//!
//! This widget provides:
//! - Keyboard navigation (up/down)
//! - Mouse click selection
//! - Scroll handling for long lists
//! - Filtering support

/// A generic list widget for displaying selectable items
#[derive(Debug, Clone)]
pub struct ListWidget<T> {
    /// All items in the list
    items: Vec<T>,
    /// Indices of items that pass the current filter
    filtered_indices: Vec<usize>,
    /// Currently selected index (into filtered_indices)
    selected_index: usize,
    /// Scroll offset for rendering
    scroll_offset: usize,
    /// Maximum visible items (set by renderer)
    max_visible: usize,
}

impl<T> ListWidget<T> {
    /// Create a new list widget with items
    pub fn new(items: Vec<T>) -> Self {
        let filtered_indices: Vec<usize> = (0..items.len()).collect();
        Self {
            items,
            filtered_indices,
            selected_index: 0,
            scroll_offset: 0,
            max_visible: 10,
        }
    }

    /// Get all items
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// Get filtered indices
    pub fn filtered_indices(&self) -> &[usize] {
        &self.filtered_indices
    }

    /// Get the currently selected index (into filtered list)
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Get the currently selected item, if any
    pub fn selected_item(&self) -> Option<&T> {
        self.filtered_indices
            .get(self.selected_index)
            .and_then(|&idx| self.items.get(idx))
    }

    /// Get the original index of the selected item
    pub fn selected_original_index(&self) -> Option<usize> {
        self.filtered_indices.get(self.selected_index).copied()
    }

    /// Set max visible items (called by renderer)
    pub fn set_max_visible(&mut self, max: usize) {
        self.max_visible = max.max(1);
    }

    /// Get scroll offset
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Move selection up
    pub fn select_up(&mut self) -> bool {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.ensure_visible();
            true
        } else {
            false
        }
    }

    /// Move selection down
    pub fn select_down(&mut self) -> bool {
        if !self.filtered_indices.is_empty() && self.selected_index < self.filtered_indices.len() - 1 {
            self.selected_index += 1;
            self.ensure_visible();
            true
        } else {
            false
        }
    }

    /// Select item at a specific filtered index
    pub fn select_index(&mut self, index: usize) -> bool {
        if index < self.filtered_indices.len() {
            self.selected_index = index;
            self.ensure_visible();
            true
        } else {
            false
        }
    }

    /// Select item by clicking at a y position within the list area
    /// Returns true if selection changed
    pub fn select_at_position(&mut self, relative_y: f32, item_height: f32) -> bool {
        let clicked_index = self.scroll_offset + (relative_y / item_height) as usize;
        self.select_index(clicked_index)
    }

    /// Ensure the selected item is visible
    fn ensure_visible(&mut self) {
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + self.max_visible {
            self.scroll_offset = self.selected_index - self.max_visible + 1;
        }
    }

    /// Filter items using a predicate
    pub fn filter<F>(&mut self, predicate: F)
    where
        F: Fn(&T) -> bool,
    {
        self.filtered_indices = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| predicate(item))
            .map(|(i, _)| i)
            .collect();
        
        // Reset selection to first item
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    /// Clear filter (show all items)
    pub fn clear_filter(&mut self) {
        self.filtered_indices = (0..self.items.len()).collect();
        self.selected_index = 0;
        self.scroll_offset = 0;
    }

    /// Check if list is empty (after filtering)
    pub fn is_empty(&self) -> bool {
        self.filtered_indices.is_empty()
    }

    /// Get number of filtered items
    pub fn len(&self) -> usize {
        self.filtered_indices.len()
    }

    /// Get visible items for rendering
    pub fn visible_items(&self) -> impl Iterator<Item = (usize, &T, bool)> {
        let start = self.scroll_offset;
        let end = (start + self.max_visible).min(self.filtered_indices.len());
        
        (start..end).map(move |visible_idx| {
            let original_idx = self.filtered_indices[visible_idx];
            let item = &self.items[original_idx];
            let is_selected = visible_idx == self.selected_index;
            (visible_idx, item, is_selected)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection() {
        let mut list = ListWidget::new(vec!["a", "b", "c"]);
        assert_eq!(list.selected_index(), 0);
        
        list.select_down();
        assert_eq!(list.selected_index(), 1);
        
        list.select_up();
        assert_eq!(list.selected_index(), 0);
    }

    #[test]
    fn test_filter() {
        let mut list = ListWidget::new(vec!["apple", "banana", "cherry"]);
        list.filter(|s| s.contains("a"));
        assert_eq!(list.len(), 2); // apple, banana
    }
}
