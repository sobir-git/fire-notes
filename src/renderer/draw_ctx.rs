//! DrawCtx — the rendering context passed to every node's draw() method.
//!
//! Owned by NodeRenderer; borrowed mutably by each draw call.
//! This is the ONLY place femtovg types are imported outside of draw/*.rs.

use std::time::Instant;
use femtovg::{Canvas, FontId, renderer::OpenGl};

use crate::theme::Theme;
use super::flame::FlameSystem;

pub struct DrawCtx<'a> {
    pub canvas:          &'a mut Canvas<OpenGl>,
    pub fonts:           &'a [FontId],
    pub theme:           &'a Theme,
    pub scale:           f32,
    pub animation_start: Instant,
    pub flame:           &'a mut FlameSystem,
}
