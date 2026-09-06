//! node_renderer — color conversion helper used by draw submodules.
//!
//! All actual rendering logic has moved to `draw/` submodules.
//! This file exists only so `draw/*.rs` can use `super::super::node_renderer::nc`.

use femtovg::Color;
use crate::layout::node::Color as NColor;

pub(crate) fn nc(c: NColor) -> Color { Color::rgbaf(c.r, c.g, c.b, c.a) }
