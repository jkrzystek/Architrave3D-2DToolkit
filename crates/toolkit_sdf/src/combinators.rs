//! CSG combinators and transforms over [`Sdf`] nodes. Each holds boxed child
//! nodes, so you can build an arbitrary constructive-solid-geometry tree at
//! runtime. The classic exact ops use min/max; the smooth ops blend with a
//! polynomial for organic joins.

use glam::Vec3;

use crate::primitives::Sdf;

/// Polynomial smooth-minimum (used by the smooth combinators). `k` controls the
/// blend radius.
pub fn smin(a: f32, b: f32, k: f32) -> f32 {
    if k <= 1e-6 {
        return a.min(b);
    }
    let h = (k - (a - b).abs()).max(0.0) / k;
    a.min(b) - h * h * k * 0.25
}

/// Boolean union: everything inside either child (exact, `min`).
pub struct Union(pub Box<dyn Sdf>, pub Box<dyn Sdf>);
impl Sdf for Union {
    fn distance(&self, p: Vec3) -> f32 {
        self.0.distance(p).min(self.1.distance(p))
    }
}

/// Boolean intersection: only where both children overlap (exact, `max`).
pub struct Intersection(pub Box<dyn Sdf>, pub Box<dyn Sdf>);
impl Sdf for Intersection {
    fn distance(&self, p: Vec3) -> f32 {
        self.0.distance(p).max(self.1.distance(p))
    }
}

/// Boolean subtraction: first child with the second carved out of it.
pub struct Subtraction(pub Box<dyn Sdf>, pub Box<dyn Sdf>);
impl Sdf for Subtraction {
    fn distance(&self, p: Vec3) -> f32 {
        self.0.distance(p).max(-self.1.distance(p))
    }
}

/// Smooth union with blend radius `k`.
pub struct SmoothUnion {
    pub a: Box<dyn Sdf>,
    pub b: Box<dyn Sdf>,
    pub k: f32,
}
impl Sdf for SmoothUnion {
    fn distance(&self, p: Vec3) -> f32 {
        smin(self.a.distance(p), self.b.distance(p), self.k)
    }
}

/// Smooth subtraction with blend radius `k`.
pub struct SmoothSubtraction {
    pub a: Box<dyn Sdf>,
    pub b: Box<dyn Sdf>,
    pub k: f32,
}
impl Sdf for SmoothSubtraction {
    fn distance(&self, p: Vec3) -> f32 {
        // smax(a, -b) = -smin(-a, b)
        -smin(-self.a.distance(p), self.b.distance(p), self.k)
    }
}

/// Smooth intersection with blend radius `k`.
pub struct SmoothIntersection {
    pub a: Box<dyn Sdf>,
    pub b: Box<dyn Sdf>,
    pub k: f32,
}
impl Sdf for SmoothIntersection {
    fn distance(&self, p: Vec3) -> f32 {
        -smin(-self.a.distance(p), -self.b.distance(p), self.k)
    }
}

/// Translate a child node by `offset`.
pub struct Translate {
    pub node: Box<dyn Sdf>,
    pub offset: Vec3,
}
impl Sdf for Translate {
    fn distance(&self, p: Vec3) -> f32 {
        self.node.distance(p - self.offset)
    }
}

/// Uniformly scale a child node by `factor` (distance is rescaled to remain a
/// valid bound).
pub struct Scale {
    pub node: Box<dyn Sdf>,
    pub factor: f32,
}
impl Sdf for Scale {
    fn distance(&self, p: Vec3) -> f32 {
        let s = self.factor.max(1e-6);
        self.node.distance(p / s) * s
    }
}

// -- Ergonomic constructors --------------------------------------------------

pub fn union(a: Box<dyn Sdf>, b: Box<dyn Sdf>) -> Box<dyn Sdf> {
    Box::new(Union(a, b))
}
pub fn intersection(a: Box<dyn Sdf>, b: Box<dyn Sdf>) -> Box<dyn Sdf> {
    Box::new(Intersection(a, b))
}
pub fn subtraction(a: Box<dyn Sdf>, b: Box<dyn Sdf>) -> Box<dyn Sdf> {
    Box::new(Subtraction(a, b))
}
pub fn smooth_union(a: Box<dyn Sdf>, b: Box<dyn Sdf>, k: f32) -> Box<dyn Sdf> {
    Box::new(SmoothUnion { a, b, k })
}
pub fn smooth_subtraction(a: Box<dyn Sdf>, b: Box<dyn Sdf>, k: f32) -> Box<dyn Sdf> {
    Box::new(SmoothSubtraction { a, b, k })
}
pub fn smooth_intersection(a: Box<dyn Sdf>, b: Box<dyn Sdf>, k: f32) -> Box<dyn Sdf> {
    Box::new(SmoothIntersection { a, b, k })
}
pub fn translate(node: Box<dyn Sdf>, offset: Vec3) -> Box<dyn Sdf> {
    Box::new(Translate { node, offset })
}
pub fn scale(node: Box<dyn Sdf>, factor: f32) -> Box<dyn Sdf> {
    Box::new(Scale { node, factor })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::Sphere;

    fn sphere(r: f32) -> Box<dyn Sdf> {
        Box::new(Sphere { radius: r })
    }

    #[test]
    fn union_is_min() {
        let a = sphere(1.0);
        let b = translate(sphere(1.0), Vec3::new(3.0, 0.0, 0.0));
        let u = union(a, b);
        // Origin is inside the first sphere -> negative.
        assert!(u.distance(Vec3::ZERO) < 0.0);
        // (3,0,0) is inside the second sphere -> negative.
        assert!(u.distance(Vec3::new(3.0, 0.0, 0.0)) < 0.0);
    }

    #[test]
    fn subtraction_carves() {
        // Big sphere minus a smaller one at origin: origin becomes outside.
        let big = sphere(2.0);
        let hole = sphere(1.0);
        let s = subtraction(big, hole);
        assert!(s.distance(Vec3::ZERO) > 0.0); // carved out
        assert!(s.distance(Vec3::new(1.5, 0.0, 0.0)) < 0.0); // still solid shell
    }

    #[test]
    fn intersection_keeps_overlap() {
        let a = sphere(2.0);
        let b = translate(sphere(2.0), Vec3::new(2.0, 0.0, 0.0));
        let i = intersection(a, b);
        // Midpoint (1,0,0) is inside both.
        assert!(i.distance(Vec3::new(1.0, 0.0, 0.0)) < 0.0);
        // Far left is inside `a` only -> outside the intersection.
        assert!(i.distance(Vec3::new(-1.5, 0.0, 0.0)) > 0.0);
    }

    #[test]
    fn smooth_union_not_greater_than_exact() {
        let a = sphere(1.0);
        let b = translate(sphere(1.0), Vec3::new(1.5, 0.0, 0.0));
        let exact = union(sphere(1.0), translate(sphere(1.0), Vec3::new(1.5, 0.0, 0.0)));
        let smooth = smooth_union(a, b, 0.5);
        let p = Vec3::new(0.75, 0.0, 0.0);
        assert!(smooth.distance(p) <= exact.distance(p) + 1e-5);
    }

    #[test]
    fn translate_moves_surface() {
        let s = translate(sphere(1.0), Vec3::new(5.0, 0.0, 0.0));
        assert!((s.distance(Vec3::new(5.0, 0.0, 0.0)) + 1.0).abs() < 1e-5); // center -> -radius
        assert!((s.distance(Vec3::new(6.0, 0.0, 0.0))).abs() < 1e-5); // on surface
    }

    #[test]
    fn scale_grows_radius() {
        let s = scale(sphere(1.0), 2.0);
        // Surface now at radius 2.
        assert!((s.distance(Vec3::new(2.0, 0.0, 0.0))).abs() < 1e-4);
    }
}
