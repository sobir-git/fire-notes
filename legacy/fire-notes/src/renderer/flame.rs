//! Flame particle effects for selection and typing highlights
//!
//! Architecture:
//! - FlameSystem manages particle lifecycle and rendering
//! - Spawning uses random sampling to distribute particles evenly
//! - Budget is enforced per-frame, not per-position
//! - Rate-limited to 60 FPS for consistent performance

use crate::config::flame as cfg;
use femtovg::{Canvas, Color, Paint, Path, renderer::OpenGl};
use rand::Rng;
use rand::seq::SliceRandom;
use std::time::{Duration, Instant};

// ============================================================================
// Constants
// ============================================================================

/// Number of particles to spawn per frame (when under budget)
const SPAWNS_PER_FRAME: usize = 15;

// ============================================================================
// Flame Particle
// ============================================================================

#[derive(Clone)]
struct FlameParticle {
    x: f32,
    y: f32,
    velocity_x: f32,
    velocity_y: f32,
    life: f32,
    max_life: f32,
    size: f32,
    noise_offset: f32,
    behind_text: bool,
}

impl FlameParticle {
    fn update(&mut self, dt: f32, time: f32) -> bool {
        self.life -= dt;
        if self.life <= 0.0 {
            return false;
        }

        let waft = (time * 3.0 + self.noise_offset).sin() * 8.0
            + (time * 5.0 + self.noise_offset * 2.0).cos() * 4.0;
        
        self.x += (self.velocity_x + waft) * dt;
        self.y -= self.velocity_y * dt;
        self.velocity_y += 15.0 * dt * (1.0 - self.life / self.max_life);
        self.velocity_x *= 0.92;
        
        true
    }

    fn color(&self) -> (f32, f32, f32, f32) {
        let life_ratio = (self.life / self.max_life).clamp(0.0, 1.0);
        
        let (r, g, b) = if life_ratio > 0.7 {
            (1.0, 0.75, 0.15)
        } else if life_ratio > 0.4 {
            (0.95, 0.45, 0.05)
        } else if life_ratio > 0.15 {
            (0.7, 0.15, 0.0)
        } else {
            (0.3, 0.05, 0.0)
        };
        
        (r, g, b, life_ratio * 0.18)
    }

    fn render_size(&self) -> f32 {
        let life_ratio = (self.life / self.max_life).clamp(0.0, 1.0);
        self.size * (0.6 + life_ratio * 0.4)
    }
}

// ============================================================================
// Flame System
// ============================================================================

/// Particle system for flame effects
pub struct FlameSystem {
    particles: Vec<FlameParticle>,
    last_update: Instant,
    last_spawn: Instant,
}

impl FlameSystem {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            particles: Vec::with_capacity(cfg::MAX_PARTICLES),
            last_update: now,
            last_spawn: now,
        }
    }

    pub fn has_active_flames(&self) -> bool {
        !self.particles.is_empty()
    }

    pub fn clear(&mut self) {
        self.particles.clear();
    }

    /// Update flame system with source positions
    /// 
    /// # Arguments
    /// * `char_positions` - (x, y, line_bottom, age) tuples for spawn sources
    /// * `scale` - Display scale factor
    pub fn update_legacy(&mut self, char_positions: &[(f32, f32, f32, f32)], scale: f32) {
        let now = Instant::now();
        let dt = self.last_update.elapsed().as_secs_f32().min(0.1);
        let time = now.elapsed().as_secs_f32() * 2.0;
        self.last_update = now;

        // Update existing particles
        self.particles.retain_mut(|p| p.update(dt, time));

        // Rate-limit spawning
        if now.duration_since(self.last_spawn) < Duration::from_millis(cfg::UPDATE_INTERVAL_MS) {
            return;
        }
        self.last_spawn = now;

        if char_positions.is_empty() {
            return;
        }

        // Calculate spawn budget for this frame
        let budget = cfg::MAX_PARTICLES.saturating_sub(self.particles.len());
        if budget == 0 {
            return;
        }

        let mut rng = rand::thread_rng();
        
        // Randomly sample positions to spawn from (ensures even distribution)
        let spawn_count = SPAWNS_PER_FRAME.min(budget).min(char_positions.len());
        
        // If we have fewer positions than spawn_count, try each position
        // Otherwise, randomly select positions to try
        if char_positions.len() <= spawn_count * 2 {
            // Few positions: try them all
            for &(x, y, line_bottom, age) in char_positions {
                if self.particles.len() >= cfg::MAX_PARTICLES {
                    break;
                }
                self.try_spawn(x, y, line_bottom, age, scale, &mut rng);
            }
        } else {
            // Many positions: randomly sample
            let mut indices: Vec<usize> = (0..char_positions.len()).collect();
            indices.shuffle(&mut rng);
            
            for &idx in indices.iter().take(spawn_count * 3) {
                if self.particles.len() >= cfg::MAX_PARTICLES {
                    break;
                }
                let (x, y, line_bottom, age) = char_positions[idx];
                self.try_spawn(x, y, line_bottom, age, scale, &mut rng);
            }
        }
    }

    fn try_spawn(&mut self, x: f32, y: f32, line_bottom: f32, age: f32, scale: f32, rng: &mut impl Rng) {
        let age_factor = 1.0 - age;
        
        // Random chance based on spawn rate and age
        if rng.gen::<f32>() > cfg::BASE_SPAWN_RATE * age_factor {
            return;
        }

        // Bottom edge modifier
        let is_bottom = (y - line_bottom).abs() < 2.0;
        let (spawn_chance, size_mult, vel_mult, life_mult) = if is_bottom {
            (0.15, 1.5, 0.6, 0.7)
        } else {
            (1.0, 1.0, 1.0, 1.0)
        };

        if rng.gen::<f32>() > spawn_chance {
            return;
        }

        self.particles.push(FlameParticle {
            x: x + rng.gen_range(-4.0..4.0) * scale,
            y: y + rng.gen_range(0.0..5.0) * scale,
            velocity_x: rng.gen_range(-12.0..12.0) * scale,
            velocity_y: rng.gen_range(30.0..55.0) * scale * vel_mult,
            life: rng.gen_range(cfg::LIFE_MIN..cfg::LIFE_MAX) * life_mult,
            max_life: cfg::LIFE_MAX,
            size: rng.gen_range(2.5..4.5) * scale * size_mult,
            noise_offset: rng.gen_range(0.0..std::f32::consts::TAU),
            behind_text: rng.gen::<f32>() < cfg::BEHIND_TEXT_RATIO,
        });
    }

    /// Draw flames for a specific layer
    pub fn draw_layer(&self, canvas: &mut Canvas<OpenGl>, behind_text: bool) {
        canvas.save();

        for p in &self.particles {
            if p.behind_text != behind_text {
                continue;
            }

            let (r, g, b, alpha) = p.color();
            let size = p.render_size();

            let mut path = Path::new();
            path.circle(p.x, p.y, size);
            canvas.fill_path(&path, &Paint::color(Color::rgba(
                (r * 255.0) as u8,
                (g * 255.0) as u8,
                (b * 255.0) as u8,
                (alpha * 255.0) as u8,
            )));

            // Glow for bright particles
            if p.life / p.max_life > 0.5 {
                let mut glow = Path::new();
                glow.circle(p.x, p.y, size * 2.0);
                canvas.fill_path(&glow, &Paint::color(Color::rgba(
                    (r * 255.0) as u8,
                    (g * 0.4 * 255.0) as u8,
                    0,
                    (alpha * 255.0 / 6.0) as u8,
                )));
            }
        }

        canvas.restore();
    }
}

impl Default for FlameSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flame_system_budget() {
        let mut system = FlameSystem::new();
        
        // Create many positions
        let positions: Vec<_> = (0..1000)
            .map(|i| (i as f32, 100.0, 120.0, 0.0))
            .collect();
        
        // Update multiple times
        for _ in 0..100 {
            system.update_legacy(&positions, 1.0);
        }
        
        // Should be capped at MAX_PARTICLES
        assert!(system.particles.len() <= cfg::MAX_PARTICLES);
    }

    #[test]
    fn test_empty_positions() {
        let mut system = FlameSystem::new();
        system.update_legacy(&[], 1.0);
        assert!(!system.has_active_flames());
    }
}
