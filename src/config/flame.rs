//! Flame/particle animation constants

#![allow(dead_code)]

/// Maximum number of flame particles
pub const MAX_PARTICLES: usize = 500;
/// Minimum time between flame updates in milliseconds (~60 FPS)
pub const UPDATE_INTERVAL_MS: u64 = 16;
/// Probability of particle spawning behind text (vs in front)
pub const BEHIND_TEXT_RATIO: f32 = 0.7;
/// Base spawn rate for new flames
pub const BASE_SPAWN_RATE: f32 = 0.4;
/// Particle lifetime range (min, max) in seconds
pub const LIFE_MIN: f32 = 0.4;
pub const LIFE_MAX: f32 = 0.7;
/// Typing flame expiry time in seconds
pub const TYPING_FLAME_EXPIRY: f32 = 1.0;
