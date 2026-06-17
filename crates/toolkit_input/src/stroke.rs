use glam::Vec2;
use serde::{Deserialize, Serialize};

/// A single recorded point in a stroke.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StrokePoint {
    pub position: Vec2,
    pub pressure: f32,
    pub tilt: Vec2,
    pub timestamp_ms: f64,
}

/// A recorded stroke consisting of a sequence of [`StrokePoint`]s.
///
/// Useful for undo/redo, replay, and analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stroke {
    points: Vec<StrokePoint>,
    start_time: f64,
    end_time: f64,
}

impl Stroke {
    /// Create a new empty stroke.
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            start_time: 0.0,
            end_time: 0.0,
        }
    }

    /// Append a point to the stroke.
    pub fn push_point(&mut self, point: StrokePoint) {
        if self.points.is_empty() {
            self.start_time = point.timestamp_ms;
        }
        self.end_time = point.timestamp_ms;
        self.points.push(point);
    }

    /// The number of points in this stroke.
    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    /// Access the raw point data.
    pub fn points(&self) -> &[StrokePoint] {
        &self.points
    }

    /// Duration of the stroke in milliseconds.
    pub fn duration_ms(&self) -> f64 {
        if self.points.len() < 2 {
            return 0.0;
        }
        self.end_time - self.start_time
    }

    /// Axis-aligned bounding box of all points: `(min, max)`.
    ///
    /// Returns `(Vec2::ZERO, Vec2::ZERO)` for an empty stroke.
    pub fn bounding_box(&self) -> (Vec2, Vec2) {
        if self.points.is_empty() {
            return (Vec2::ZERO, Vec2::ZERO);
        }
        let mut min = self.points[0].position;
        let mut max = self.points[0].position;
        for p in &self.points[1..] {
            min = min.min(p.position);
            max = max.max(p.position);
        }
        (min, max)
    }

    /// Re-sample the stroke at even pixel intervals using linear interpolation.
    ///
    /// `interval_px` is the desired distance (in pixels) between consecutive
    /// points in the output stroke. Pressure and tilt are linearly interpolated.
    ///
    /// Returns an empty stroke if the input has fewer than 2 points or
    /// `interval_px <= 0.0`.
    pub fn resample(&self, interval_px: f32) -> Stroke {
        if self.points.len() < 2 || interval_px <= 0.0 {
            return Stroke::new();
        }

        let mut result = Stroke::new();

        // Always include the first point.
        result.push_point(self.points[0]);

        let mut accumulated_dist = 0.0_f32;
        let mut prev = self.points[0];

        for &curr in &self.points[1..] {
            let segment_vec = curr.position - prev.position;
            let segment_len = segment_vec.length();

            if segment_len < 1e-9 {
                prev = curr;
                continue;
            }

            let mut offset = interval_px - accumulated_dist;
            while offset <= segment_len {
                let t = offset / segment_len;
                let interp = StrokePoint {
                    position: prev.position.lerp(curr.position, t),
                    pressure: prev.pressure + (curr.pressure - prev.pressure) * t,
                    tilt: prev.tilt.lerp(curr.tilt, t),
                    timestamp_ms: prev.timestamp_ms
                        + (curr.timestamp_ms - prev.timestamp_ms) * t as f64,
                };
                result.push_point(interp);
                offset += interval_px;
            }

            accumulated_dist = segment_len - (offset - interval_px);
            prev = curr;
        }

        result
    }
}

impl Default for Stroke {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f32, y: f32, t: f64) -> StrokePoint {
        StrokePoint {
            position: Vec2::new(x, y),
            pressure: 1.0,
            tilt: Vec2::ZERO,
            timestamp_ms: t,
        }
    }

    #[test]
    fn empty_stroke() {
        let s = Stroke::new();
        assert_eq!(s.point_count(), 0);
        assert_eq!(s.duration_ms(), 0.0);
        assert_eq!(s.bounding_box(), (Vec2::ZERO, Vec2::ZERO));
    }

    #[test]
    fn push_and_count() {
        let mut s = Stroke::new();
        s.push_point(pt(0.0, 0.0, 0.0));
        s.push_point(pt(10.0, 5.0, 100.0));
        assert_eq!(s.point_count(), 2);
        assert_eq!(s.points()[0].position, Vec2::new(0.0, 0.0));
    }

    #[test]
    fn duration_of_single_point() {
        let mut s = Stroke::new();
        s.push_point(pt(0.0, 0.0, 42.0));
        assert_eq!(s.duration_ms(), 0.0);
    }

    #[test]
    fn duration_of_multiple_points() {
        let mut s = Stroke::new();
        s.push_point(pt(0.0, 0.0, 100.0));
        s.push_point(pt(1.0, 1.0, 200.0));
        s.push_point(pt(2.0, 2.0, 350.0));
        assert_eq!(s.duration_ms(), 250.0);
    }

    #[test]
    fn bounding_box_correct() {
        let mut s = Stroke::new();
        s.push_point(pt(5.0, 10.0, 0.0));
        s.push_point(pt(-3.0, 20.0, 1.0));
        s.push_point(pt(8.0, -1.0, 2.0));

        let (min, max) = s.bounding_box();
        assert_eq!(min, Vec2::new(-3.0, -1.0));
        assert_eq!(max, Vec2::new(8.0, 20.0));
    }

    #[test]
    fn resample_produces_even_spacing() {
        let mut s = Stroke::new();
        // Horizontal line from (0,0) to (100,0)
        s.push_point(pt(0.0, 0.0, 0.0));
        s.push_point(pt(100.0, 0.0, 1000.0));

        let resampled = s.resample(10.0);

        // First point is at 0, then points at 10, 20, ..., 100
        assert_eq!(resampled.point_count(), 11);

        for (i, p) in resampled.points().iter().enumerate() {
            let expected_x = i as f32 * 10.0;
            assert!(
                (p.position.x - expected_x).abs() < 0.01,
                "Point {i}: expected x={expected_x}, got {}",
                p.position.x
            );
            assert!(
                p.position.y.abs() < 0.01,
                "Point {i}: expected y=0, got {}",
                p.position.y
            );
        }
    }

    #[test]
    fn resample_empty_stroke_returns_empty() {
        let s = Stroke::new();
        let resampled = s.resample(5.0);
        assert_eq!(resampled.point_count(), 0);
    }

    #[test]
    fn resample_single_point_returns_empty() {
        let mut s = Stroke::new();
        s.push_point(pt(0.0, 0.0, 0.0));
        let resampled = s.resample(5.0);
        assert_eq!(resampled.point_count(), 0);
    }

    #[test]
    fn resample_zero_interval_returns_empty() {
        let mut s = Stroke::new();
        s.push_point(pt(0.0, 0.0, 0.0));
        s.push_point(pt(10.0, 0.0, 100.0));
        let resampled = s.resample(0.0);
        assert_eq!(resampled.point_count(), 0);
    }

    #[test]
    fn resample_multi_segment_diagonal() {
        let mut s = Stroke::new();
        // L-shaped path: (0,0) -> (30,0) -> (30,40)
        // Segment 1 length = 30, segment 2 length = 40, total = 70
        s.push_point(pt(0.0, 0.0, 0.0));
        s.push_point(pt(30.0, 0.0, 300.0));
        s.push_point(pt(30.0, 40.0, 700.0));

        let resampled = s.resample(10.0);

        // Total path length = 70, interval = 10: points at 0, 10, 20, 30, 40, 50, 60, 70
        // That's first point + 7 interval points = 8 total
        assert_eq!(resampled.point_count(), 8);

        // Verify spacing between consecutive points
        for i in 1..resampled.point_count() {
            let d = resampled.points()[i]
                .position
                .distance(resampled.points()[i - 1].position);
            assert!(
                (d - 10.0).abs() < 0.1,
                "Spacing between point {} and {}: expected ~10.0, got {d}",
                i - 1,
                i
            );
        }
    }

    #[test]
    fn resample_interpolates_pressure() {
        let mut s = Stroke::new();
        s.push_point(StrokePoint {
            position: Vec2::new(0.0, 0.0),
            pressure: 0.0,
            tilt: Vec2::ZERO,
            timestamp_ms: 0.0,
        });
        s.push_point(StrokePoint {
            position: Vec2::new(100.0, 0.0),
            pressure: 1.0,
            tilt: Vec2::ZERO,
            timestamp_ms: 1000.0,
        });

        let resampled = s.resample(50.0);
        // Points at x=0 (pressure 0.0), x=50 (pressure 0.5), x=100 (pressure 1.0)
        assert_eq!(resampled.point_count(), 3);
        assert!((resampled.points()[1].pressure - 0.5).abs() < 0.01);
    }
}
