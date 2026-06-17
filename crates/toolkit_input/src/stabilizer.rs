use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Configuration for the spring-damper stroke stabilizer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StabilizerConfig {
    /// Spring constant controlling how strongly the virtual point is pulled
    /// toward the cursor. Higher values = snappier, lower = smoother.
    pub spring_constant: f32,
    /// Velocity damping factor applied each step (0..1). Higher = more damping.
    pub damping: f32,
    /// Minimum distance (in pixels) the cursor must be from the virtual point
    /// before movement is produced. Filters out jitter.
    pub dead_zone: f32,
    /// Whether the stabilizer is active. When disabled, `update` returns the
    /// raw cursor position.
    pub enabled: bool,
}

impl Default for StabilizerConfig {
    fn default() -> Self {
        Self {
            spring_constant: 0.5,
            damping: 0.8,
            dead_zone: 1.0,
            enabled: true,
        }
    }
}

/// A spring-damper "lazy mouse" stroke stabilizer.
///
/// Tracks a virtual point that is pulled toward the real cursor position via
/// spring dynamics, producing smooth output that lags slightly behind the
/// real input.
#[derive(Debug, Clone)]
pub struct StrokeStabilizer {
    config: StabilizerConfig,
    virtual_pos: Vec2,
    velocity: Vec2,
    active: bool,
}

impl StrokeStabilizer {
    /// Create a new stabilizer with the given config. The virtual point starts
    /// at the origin until [`reset`](Self::reset) is called.
    pub fn new(config: StabilizerConfig) -> Self {
        Self {
            config,
            virtual_pos: Vec2::ZERO,
            velocity: Vec2::ZERO,
            active: false,
        }
    }

    /// Snap the virtual point to the given position, clearing all velocity.
    /// This should be called at the start of each new stroke.
    pub fn reset(&mut self, position: Vec2) {
        self.virtual_pos = position;
        self.velocity = Vec2::ZERO;
        self.active = true;
    }

    /// Advance the spring simulation by `dt_seconds` toward `cursor_position`.
    ///
    /// Returns `Some(smoothed_position)` when the virtual point has moved
    /// outside the dead zone, or `None` if the displacement is too small.
    /// When the stabilizer is disabled, returns `Some(cursor_position)` directly.
    pub fn update(&mut self, cursor_position: Vec2, dt_seconds: f32) -> Option<Vec2> {
        if !self.config.enabled {
            self.virtual_pos = cursor_position;
            return Some(cursor_position);
        }

        let displacement = cursor_position - self.virtual_pos;
        let distance = displacement.length();

        // Dead zone: if the cursor is very close to the virtual point,
        // don't produce output.
        if distance < self.config.dead_zone {
            // Still dampen existing velocity.
            self.velocity *= self.config.damping;
            return None;
        }

        // Spring force: F = k * displacement
        let spring_force = self.config.spring_constant * displacement;
        self.velocity += spring_force * dt_seconds;

        // Move the virtual point
        self.virtual_pos += self.velocity * dt_seconds;

        // Damping
        self.velocity *= self.config.damping;

        Some(self.virtual_pos)
    }

    /// The current smoothed position of the virtual point.
    pub fn current_position(&self) -> Vec2 {
        self.virtual_pos
    }

    /// Whether the stabilizer has been reset and is actively tracking a stroke.
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn config(&self) -> &StabilizerConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: StabilizerConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let cfg = StabilizerConfig::default();
        assert_eq!(cfg.spring_constant, 0.5);
        assert_eq!(cfg.damping, 0.8);
        assert_eq!(cfg.dead_zone, 1.0);
        assert!(cfg.enabled);
    }

    #[test]
    fn reset_snaps_position() {
        let mut stab = StrokeStabilizer::new(StabilizerConfig::default());
        stab.reset(Vec2::new(100.0, 200.0));
        assert_eq!(stab.current_position(), Vec2::new(100.0, 200.0));
        assert!(stab.is_active());
    }

    #[test]
    fn converges_toward_target() {
        let mut stab = StrokeStabilizer::new(StabilizerConfig {
            spring_constant: 50.0,
            damping: 0.85,
            dead_zone: 0.1,
            enabled: true,
        });
        stab.reset(Vec2::ZERO);

        let target = Vec2::new(100.0, 0.0);
        let dt = 1.0 / 60.0;

        for _ in 0..3000 {
            stab.update(target, dt);
        }

        let final_pos = stab.current_position();
        let error = (final_pos - target).length();
        assert!(
            error < 5.0,
            "Stabilizer should converge near target, but error = {error}"
        );
    }

    #[test]
    fn dead_zone_filters_small_movements() {
        let mut stab = StrokeStabilizer::new(StabilizerConfig {
            dead_zone: 5.0,
            ..Default::default()
        });
        stab.reset(Vec2::new(50.0, 50.0));

        // Move cursor only 2 pixels away (within dead zone)
        let result = stab.update(Vec2::new(51.0, 50.0), 1.0 / 60.0);
        assert!(
            result.is_none(),
            "Should return None when within dead zone"
        );
    }

    #[test]
    fn disabled_returns_raw_position() {
        let mut stab = StrokeStabilizer::new(StabilizerConfig {
            enabled: false,
            ..Default::default()
        });
        stab.reset(Vec2::ZERO);

        let cursor = Vec2::new(42.0, 99.0);
        let result = stab.update(cursor, 1.0 / 60.0);
        assert_eq!(result, Some(cursor));
        assert_eq!(stab.current_position(), cursor);
    }

    #[test]
    fn velocity_is_cleared_on_reset() {
        let mut stab = StrokeStabilizer::new(StabilizerConfig {
            spring_constant: 10.0,
            damping: 0.9,
            dead_zone: 0.0,
            enabled: true,
        });
        stab.reset(Vec2::ZERO);

        // Build up some velocity
        for _ in 0..30 {
            stab.update(Vec2::new(100.0, 0.0), 1.0 / 60.0);
        }

        // Reset to a new position
        stab.reset(Vec2::new(200.0, 200.0));
        assert_eq!(stab.current_position(), Vec2::new(200.0, 200.0));

        // A single update toward the same position should barely move
        let before = stab.current_position();
        stab.update(Vec2::new(200.0, 200.0), 1.0 / 60.0);
        let after = stab.current_position();
        let drift = (after - before).length();
        assert!(
            drift < 0.01,
            "After reset, velocity should be zero so drift should be negligible, got {drift}"
        );
    }

    #[test]
    fn not_active_before_reset() {
        let stab = StrokeStabilizer::new(StabilizerConfig::default());
        assert!(!stab.is_active());
    }
}
