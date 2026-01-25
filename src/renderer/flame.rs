//! Flame particle effects for selection highlighting

use femtovg::{Canvas, Color, Paint, Path, renderer::OpenGl};
use rand::Rng;
use std::time::Instant;

#[derive(Clone)]
pub struct FlameParticle {
    pub x: f32,
    pub y: f32,
    pub velocity_y: f32,
    pub velocity_x: f32,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub noise_offset: f32,
    pub behind_text: bool, // true = render behind text, false = render in front
}

pub struct FlameSystem {
    particles: Vec<FlameParticle>,
    last_update: Instant,
}

impl FlameSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            last_update: Instant::now(),
        }
    }

    pub fn has_active_flames(&self) -> bool {
        !self.particles.is_empty()
    }

    pub fn clear(&mut self) {
        self.particles.clear();
    }

    pub fn update(&mut self, char_positions: &[(f32, f32, f32, f32)], scale: f32) {
        let dt = self.last_update.elapsed().as_secs_f32();
        self.last_update = Instant::now();

        let mut rng = rand::thread_rng();
        let time = self.last_update.elapsed().as_secs_f32() * 2.0;

        // Update existing particles with realistic fire physics
        self.particles.retain_mut(|p| {
            p.life -= dt;

            // Rising with turbulent waft - realistic fire behavior
            let waft = (time * 3.0 + p.noise_offset).sin() * 8.0
                + (time * 5.0 + p.noise_offset * 2.0).cos() * 4.0;
            p.x += (p.velocity_x + waft) * dt;
            p.y -= p.velocity_y * dt;

            // Buoyancy increases as particle rises (hot air rises faster)
            p.velocity_y += 15.0 * dt * (1.0 - p.life / p.max_life);
            p.velocity_x *= 0.92; // Air resistance

            p.life > 0.0
        });

        // Spawn dense particles with lower opacity
        for &(char_x, char_y, line_bottom_y, age) in char_positions {
            // Decrease spawn rate based on age (0.0 = just typed, 1.0 = 1 second old)
            let age_factor = 1.0 - age; // 1.0 when fresh, 0.0 when old
            let base_spawn_rate = 0.4 * age_factor; // Decreases from 0.4 to 0.0
            
            if rng.gen_range(0.0..1.0) > base_spawn_rate {
                continue;
            }

            // Determine if this is near the bottom of selection
            let is_bottom_edge = (char_y - line_bottom_y).abs() < 2.0;

            // Bottom edges have much less activity but bigger particles
            let (spawn_chance, size_mult, velocity_mult, life_mult) = if is_bottom_edge {
                (0.15, 1.5, 0.6, 0.7) // Bottom: fewer but larger flames
            } else {
                (1.0, 1.0, 1.0, 1.0)
            };

            if rng.gen_range(0.0..1.0) > spawn_chance {
                continue;
            }

            // Spawn from character position with horizontal spread
            let offset_x = rng.gen_range(-4.0..4.0) * scale;
            // Start flames at text level and below
            let offset_y = rng.gen_range(0.0..5.0) * scale;

            self.particles.push(FlameParticle {
                x: char_x + offset_x,
                y: char_y + offset_y,
                velocity_y: rng.gen_range(30.0..55.0) * scale * velocity_mult,  // Moderate upward
                velocity_x: rng.gen_range(-12.0..12.0) * scale,
                life: rng.gen_range(0.4..0.7) * life_mult,  // Slightly longer life
                max_life: 0.7,
                size: rng.gen_range(2.5..4.5) * scale * size_mult,
                noise_offset: rng.gen_range(0.0..std::f32::consts::TAU),
                behind_text: rng.gen_range(0.0..1.0) < 0.7, // 70% behind, 30% in front
            });
        }

        // Higher particle limit for dense flames
        if self.particles.len() > 700 {
            let to_remove = self.particles.len() - 700;
            self.particles.drain(0..to_remove);
        }
    }

    pub fn draw_layer(
        &self,
        canvas: &mut Canvas<OpenGl>,
        _char_positions: &[(f32, f32, f32, f32)],
        _scale: f32,
        behind_text: bool,
    ) {
        canvas.save();

        for particle in &self.particles {
            // Only draw particles for this layer
            if particle.behind_text != behind_text {
                continue;
            }

            let life_ratio = (particle.life / particle.max_life).clamp(0.0, 1.0);

            // Allow flames to rise high above text (no constraint)
            let constrained_y = particle.y;

            // Realistic fire palette: starts bright, fades to deep red embers
            let (r, g, b) = if life_ratio > 0.7 {
                // Bright yellow-orange core (less white for realism)
                (1.0, 0.75, 0.15)
            } else if life_ratio > 0.4 {
                // Orange flames
                (0.95, 0.45, 0.05)
            } else if life_ratio > 0.15 {
                // Deep red
                (0.7, 0.15, 0.0)
            } else {
                // Dark embers
                (0.3, 0.05, 0.0)
            };

            // Lower opacity for subtle, numerous flames
            let alpha = (life_ratio * 0.18 * 255.0) as u8;
            let size = particle.size * (0.6 + life_ratio * 0.4);

            // Draw core
            let mut path = Path::new();
            path.circle(particle.x, constrained_y, size);
            let paint = Paint::color(Color::rgba(
                (r * 255.0) as u8,
                (g * 255.0) as u8,
                (b * 255.0) as u8,
                alpha,
            ));
            canvas.fill_path(&path, &paint);

            // Subtle glow only for brighter particles
            if life_ratio > 0.5 {
                let mut glow_path = Path::new();
                glow_path.circle(particle.x, constrained_y, size * 2.0);
                let glow_paint = Paint::color(Color::rgba(
                    (r * 255.0) as u8,
                    (g * 0.4 * 255.0) as u8,
                    0,
                    alpha / 6,
                ));
                canvas.fill_path(&glow_path, &glow_paint);
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
