//! The error quadric used by QEM simplification.
//!
//! A quadric stores, in 10 unique coefficients, the symmetric 4×4 matrix `Q`
//! such that `pᵀ Q p` (with `p = (x, y, z, 1)`) is the sum of squared distances
//! from a point to a set of planes. Summing the quadrics of a vertex's incident
//! triangle planes gives the cost of moving that vertex.

use glam::Vec3;

/// Symmetric 4×4 quadric, stored as its upper-triangular entries.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Quadric {
    // a², ab, ac, ad, b², bc, bd, c², cd, d²
    m: [f32; 10],
}

impl Quadric {
    /// The zero quadric (no constraint).
    pub fn zero() -> Self {
        Self { m: [0.0; 10] }
    }

    /// Quadric for the plane through `point` with unit `normal`.
    ///
    /// The input normal is normalized, so this produces a pure distance-only
    /// quadric with no area weighting. Useful for user-specified constraint
    /// planes where every plane should contribute equally regardless of size.
    pub fn from_plane(normal: Vec3, point: Vec3) -> Self {
        let n = normal.normalize_or_zero();
        let (a, b, c) = (n.x, n.y, n.z);
        let d = -n.dot(point);
        Self {
            m: [
                a * a,
                a * b,
                a * c,
                a * d,
                b * b,
                b * c,
                b * d,
                c * c,
                c * d,
                d * d,
            ],
        }
    }

    /// Area-weighted quadric for the triangle plane through `point` with raw
    /// (un-normalized) `normal`.
    ///
    /// In standard QEM the normal is the raw cross product whose magnitude
    /// equals twice the triangle area, so larger triangles naturally contribute
    /// more error. This is the recommended constructor for mesh simplification.
    pub fn from_plane_area_weighted(normal: Vec3, point: Vec3) -> Self {
        let (a, b, c) = (normal.x, normal.y, normal.z);
        let d = -normal.dot(point);
        Self {
            m: [
                a * a,
                a * b,
                a * c,
                a * d,
                b * b,
                b * c,
                b * d,
                c * c,
                c * d,
                d * d,
            ],
        }
    }

    /// Sum two quadrics (combining their plane sets).
    pub fn add(self, other: Quadric) -> Quadric {
        let mut m = self.m;
        for i in 0..10 {
            m[i] += other.m[i];
        }
        Quadric { m }
    }

    /// Accumulate `other` in place.
    pub fn add_assign(&mut self, other: Quadric) {
        for i in 0..10 {
            self.m[i] += other.m[i];
        }
    }

    /// Evaluate `pᵀ Q p`: the summed squared plane distance at `p`.
    pub fn error(&self, p: Vec3) -> f32 {
        let (x, y, z) = (p.x, p.y, p.z);
        let m = &self.m;
        m[0] * x * x
            + 2.0 * m[1] * x * y
            + 2.0 * m[2] * x * z
            + 2.0 * m[3] * x
            + m[4] * y * y
            + 2.0 * m[5] * y * z
            + 2.0 * m[6] * y
            + m[7] * z * z
            + 2.0 * m[8] * z
            + m[9]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_on_plane_has_zero_error() {
        let q = Quadric::from_plane(Vec3::Y, Vec3::new(0.0, 2.0, 0.0));
        // Any point with y = 2 lies on the plane.
        assert!(q.error(Vec3::new(5.0, 2.0, -3.0)).abs() < 1e-5);
        // A point 1 unit off the plane has error 1² = 1.
        assert!((q.error(Vec3::new(0.0, 3.0, 0.0)) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn summed_planes_accumulate_error() {
        let q1 = Quadric::from_plane(Vec3::X, Vec3::ZERO);
        let q2 = Quadric::from_plane(Vec3::Y, Vec3::ZERO);
        let q = q1.add(q2);
        // Distance² to x=0 plus distance² to y=0 at (1,1,0) = 1 + 1.
        assert!((q.error(Vec3::new(1.0, 1.0, 0.0)) - 2.0).abs() < 1e-5);
    }
}
