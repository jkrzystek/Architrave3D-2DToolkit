//! Interpret an L-system string as 3D turtle graphics, producing line segments.
//!
//! The turtle carries an orientation (quaternion) with local axes: forward
//! `+Y`, up `+Z`, left `+X`. Commands rotate about those local axes, so the
//! same string draws consistently in 3D.

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Turtle parameters.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TurtleConfig {
    /// Distance moved by `F`/`f`.
    pub step: f32,
    /// Rotation applied by turn commands, in radians.
    pub angle: f32,
}

impl Default for TurtleConfig {
    fn default() -> Self {
        Self {
            step: 1.0,
            angle: std::f32::consts::FRAC_PI_2,
        }
    }
}

#[derive(Clone, Copy)]
struct State {
    position: Vec3,
    orientation: Quat,
}

/// A drawn line segment.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Segment {
    pub start: Vec3,
    pub end: Vec3,
}

/// Interpret `commands` as turtle graphics, returning the drawn segments.
///
/// Recognised symbols:
/// - `F` move forward drawing a segment; `f` move forward without drawing
/// - `+`/`-` yaw left/right · `&`/`^` pitch down/up · `\`/`/` roll left/right
/// - `|` turn 180° · `[`/`]` push/pop turtle state (branching)
///
/// Other symbols are ignored (they are L-system variables).
pub fn interpret(commands: &str, config: &TurtleConfig) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut stack: Vec<State> = Vec::new();
    let mut state = State {
        position: Vec3::ZERO,
        orientation: Quat::IDENTITY,
    };
    let a = config.angle;

    for ch in commands.chars() {
        match ch {
            'F' => {
                let heading = state.orientation * Vec3::Y;
                let end = state.position + heading * config.step;
                segments.push(Segment {
                    start: state.position,
                    end,
                });
                state.position = end;
            }
            'f' => {
                let heading = state.orientation * Vec3::Y;
                state.position += heading * config.step;
            }
            '+' => state.orientation = state.orientation * Quat::from_rotation_z(a),
            '-' => state.orientation = state.orientation * Quat::from_rotation_z(-a),
            '&' => state.orientation = state.orientation * Quat::from_rotation_x(a),
            '^' => state.orientation = state.orientation * Quat::from_rotation_x(-a),
            '\\' => state.orientation = state.orientation * Quat::from_rotation_y(a),
            '/' => state.orientation = state.orientation * Quat::from_rotation_y(-a),
            '|' => state.orientation = state.orientation * Quat::from_rotation_z(std::f32::consts::PI),
            '[' => stack.push(state),
            ']' => {
                if let Some(s) = stack.pop() {
                    state = s;
                }
            }
            _ => {}
        }
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_draws_along_up_axis() {
        let segs = interpret("F", &TurtleConfig { step: 2.0, angle: 0.0 });
        assert_eq!(segs.len(), 1);
        assert!((segs[0].start).length() < 1e-6);
        assert!((segs[0].end - Vec3::new(0.0, 2.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn lowercase_f_moves_without_drawing() {
        let segs = interpret("fF", &TurtleConfig { step: 1.0, angle: 0.0 });
        assert_eq!(segs.len(), 1);
        // The drawn segment starts where the silent move ended.
        assert!((segs[0].start - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn turn_changes_direction() {
        // Forward, yaw 90°, forward: an L shape in the XY plane.
        let cfg = TurtleConfig {
            step: 1.0,
            angle: std::f32::consts::FRAC_PI_2,
        };
        let segs = interpret("F+F", &cfg);
        assert_eq!(segs.len(), 2);
        // First goes +Y; after +90° about Z the heading turns toward -X.
        let dir2 = (segs[1].end - segs[1].start).normalize();
        assert!((dir2 - Vec3::NEG_X).length() < 1e-4);
    }

    #[test]
    fn brackets_restore_state() {
        // Branch off, then return: the final segment resumes from the trunk top.
        let cfg = TurtleConfig {
            step: 1.0,
            angle: std::f32::consts::FRAC_PI_2,
        };
        let segs = interpret("F[+F]F", &cfg);
        assert_eq!(segs.len(), 3);
        // Trunk: 0->Y. Branch from Y toward -X. Final F resumes at Y going +Y.
        let last = segs[2];
        assert!((last.start - Vec3::new(0.0, 1.0, 0.0)).length() < 1e-5);
        assert!((last.end - Vec3::new(0.0, 2.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn pitch_moves_out_of_plane() {
        let cfg = TurtleConfig {
            step: 1.0,
            angle: std::f32::consts::FRAC_PI_2,
        };
        // Pitch about local X should tilt the heading off the Y axis.
        let segs = interpret("&F", &cfg);
        let dir = (segs[0].end - segs[0].start).normalize();
        assert!(dir.y.abs() < 1e-4, "heading should have left the Y axis");
    }

    #[test]
    fn serde_roundtrip() {
        let seg = Segment {
            start: Vec3::ZERO,
            end: Vec3::Y,
        };
        let json = serde_json::to_string(&seg).unwrap();
        let back: Segment = serde_json::from_str(&json).unwrap();
        assert_eq!(seg, back);
    }
}
