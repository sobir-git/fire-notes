//! Layout helpers — cursor/selection position calculations.

use std::collections::HashMap;

/// Result of checking if a character position is in a flame zone
#[derive(Clone, Copy)]
pub enum FlameHit {
    None,
    Selection,
    Typing(f32), // age factor
}

/// Build a spatial hash map for O(1) flame position lookups.
pub fn build_flame_lookup(
    char_positions: &[(f32, f32, f32, f32)],
    char_width: f32,
    line_height: f32,
) -> HashMap<(i32, i32), FlameHit> {
    let mut map = HashMap::with_capacity(char_positions.len());
    let cell_w = char_width.max(1.0);
    let cell_h = line_height.max(1.0);

    for &(cx, cy, _, age) in char_positions {
        let grid_x = (cx / cell_w) as i32;
        let grid_y = (cy / cell_h) as i32;

        let hit = if age > 0.0 {
            FlameHit::Typing(age)
        } else {
            FlameHit::Selection
        };

        map.entry((grid_x, grid_y))
            .and_modify(|existing| {
                if matches!(hit, FlameHit::Typing(_)) {
                    *existing = hit;
                }
            })
            .or_insert(hit);
    }
    map
}
