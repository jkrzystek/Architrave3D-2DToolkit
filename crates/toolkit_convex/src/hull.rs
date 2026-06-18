//! 3D convex hull via the incremental algorithm.
//!
//! We seed a tetrahedron from extreme points, then add the remaining points one
//! at a time: a point sees the faces it lies in front of; those faces are
//! removed and replaced with new faces fanning from the point to the *horizon*
//! (the loop of edges bordering the visible region). Faces are oriented against
//! a fixed interior point, which sidesteps winding bookkeeping.

use std::collections::HashMap;

use glam::Vec3;
use serde::{Deserialize, Serialize};
use toolkit_geometry::{Mesh, Vertex};

/// A convex hull: deduplicated vertices and outward-wound triangle faces.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConvexHull {
    pub vertices: Vec<Vec3>,
    /// Triangles as indices into `vertices`, wound counter-clockwise when
    /// viewed from outside.
    pub faces: Vec<[usize; 3]>,
}

impl ConvexHull {
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Outward normal of face `i`.
    pub fn face_normal(&self, i: usize) -> Vec3 {
        let [a, b, c] = self.faces[i];
        (self.vertices[b] - self.vertices[a])
            .cross(self.vertices[c] - self.vertices[a])
            .normalize_or_zero()
    }

    /// Whether `p` lies inside (or on) the hull, within `eps`.
    pub fn contains(&self, p: Vec3, eps: f32) -> bool {
        for i in 0..self.faces.len() {
            let a = self.vertices[self.faces[i][0]];
            if self.face_normal(i).dot(p - a) > eps {
                return false;
            }
        }
        true
    }

    /// Triangulated mesh with flat per-face normals.
    pub fn to_mesh(&self) -> Mesh {
        let mut vertices = Vec::with_capacity(self.faces.len() * 3);
        let mut indices = Vec::with_capacity(self.faces.len() * 3);
        for i in 0..self.faces.len() {
            let n = self.face_normal(i);
            for &vi in &self.faces[i] {
                indices.push(vertices.len() as u32);
                vertices.push(Vertex::new(self.vertices[vi], n, glam::Vec2::ZERO));
            }
        }
        Mesh::with_vertices("hull", vertices, indices)
    }
}

#[derive(Clone, Copy)]
struct Face {
    idx: [usize; 3],
    normal: Vec3,
    /// Plane offset: `normal · x = offset` on the face.
    offset: f32,
}

/// Build the convex hull of `points`. Returns `None` if fewer than four points
/// are given or they are degenerate (collinear or coplanar — no 3D volume).
pub fn convex_hull(points: &[Vec3]) -> Option<ConvexHull> {
    if points.len() < 4 {
        return None;
    }

    let (min, max) = bounds(points);
    let scale = (max - min).length().max(1.0);
    let eps = 1e-7 * scale;

    let [p0, p1, p2, p3] = initial_tetra(points, eps)?;
    let interior = (points[p0] + points[p1] + points[p2] + points[p3]) / 4.0;

    let mut faces = vec![
        make_face(points, [p0, p1, p2], interior),
        make_face(points, [p0, p1, p3], interior),
        make_face(points, [p0, p2, p3], interior),
        make_face(points, [p1, p2, p3], interior),
    ];

    let seed = [p0, p1, p2, p3];
    for (pi, &p) in points.iter().enumerate() {
        if seed.contains(&pi) {
            continue;
        }
        add_point(&mut faces, points, pi, p, interior, eps);
    }

    Some(finalize(&faces, points))
}

fn add_point(
    faces: &mut Vec<Face>,
    points: &[Vec3],
    pi: usize,
    p: Vec3,
    interior: Vec3,
    eps: f32,
) {
    // Faces the new point lies in front of.
    let visible: Vec<usize> = (0..faces.len())
        .filter(|&f| faces[f].normal.dot(p) - faces[f].offset > eps)
        .collect();
    if visible.is_empty() {
        return; // inside the current hull
    }

    // Horizon edges border exactly one visible face. Count undirected edges
    // across the visible set; those seen once are the horizon.
    let mut edge_count: HashMap<(usize, usize), i32> = HashMap::new();
    for &f in &visible {
        let [a, b, c] = faces[f].idx;
        for (u, v) in [(a, b), (b, c), (c, a)] {
            *edge_count.entry(undirected(u, v)).or_insert(0) += 1;
        }
    }

    // Remove visible faces (high indices first to keep the rest valid).
    let mut vis_sorted = visible.clone();
    vis_sorted.sort_unstable_by(|a, b| b.cmp(a));
    for f in vis_sorted {
        faces.swap_remove(f);
    }

    // Fan new faces from the apex to each horizon edge.
    for (edge, count) in edge_count {
        if count == 1 {
            faces.push(make_face(points, [edge.0, edge.1, pi], interior));
        }
    }
}

fn make_face(points: &[Vec3], idx: [usize; 3], interior: Vec3) -> Face {
    let [a, b, c] = idx;
    let mut normal = (points[b] - points[a]).cross(points[c] - points[a]).normalize_or_zero();
    let mut idx = idx;
    // Orient outward: normal should point away from the interior point.
    if normal.dot(interior - points[a]) > 0.0 {
        normal = -normal;
        idx.swap(1, 2);
    }
    Face {
        idx,
        normal,
        offset: normal.dot(points[a]),
    }
}

fn finalize(faces: &[Face], points: &[Vec3]) -> ConvexHull {
    // Collect the used original indices and remap to a compact vertex list.
    let mut remap: HashMap<usize, usize> = HashMap::new();
    let mut vertices = Vec::new();
    let mut out_faces = Vec::with_capacity(faces.len());
    for f in faces {
        let mut tri = [0usize; 3];
        for (k, &orig) in f.idx.iter().enumerate() {
            let new = *remap.entry(orig).or_insert_with(|| {
                vertices.push(points[orig]);
                vertices.len() - 1
            });
            tri[k] = new;
        }
        out_faces.push(tri);
    }
    ConvexHull { vertices, faces: out_faces }
}

fn undirected(u: usize, v: usize) -> (usize, usize) {
    if u < v {
        (u, v)
    } else {
        (v, u)
    }
}

fn bounds(points: &[Vec3]) -> (Vec3, Vec3) {
    let mut min = points[0];
    let mut max = points[0];
    for &p in points {
        min = min.min(p);
        max = max.max(p);
    }
    (min, max)
}

/// Find four points forming a non-degenerate tetrahedron.
fn initial_tetra(points: &[Vec3], eps: f32) -> Option<[usize; 4]> {
    // p0, p1: the most separated pair among per-axis extreme points.
    let extremes = axis_extremes(points);
    let (mut p0, mut p1, mut best) = (0usize, 0usize, -1.0f32);
    for i in 0..extremes.len() {
        for j in i + 1..extremes.len() {
            let d = (points[extremes[i]] - points[extremes[j]]).length_squared();
            if d > best {
                best = d;
                p0 = extremes[i];
                p1 = extremes[j];
            }
        }
    }
    if best <= eps * eps {
        return None; // all coincident
    }

    // p2: farthest from the line p0p1.
    let line = (points[p1] - points[p0]).normalize();
    let p2 = (0..points.len())
        .max_by(|&a, &b| {
            dist_to_line(points[a], points[p0], line)
                .total_cmp(&dist_to_line(points[b], points[p0], line))
        })?;
    if dist_to_line(points[p2], points[p0], line) <= eps {
        return None; // collinear
    }

    // p3: farthest from the plane (p0, p1, p2).
    let normal = (points[p1] - points[p0])
        .cross(points[p2] - points[p0])
        .normalize();
    let p3 = (0..points.len())
        .max_by(|&a, &b| {
            (normal.dot(points[a] - points[p0]).abs())
                .total_cmp(&normal.dot(points[b] - points[p0]).abs())
        })?;
    if normal.dot(points[p3] - points[p0]).abs() <= eps {
        return None; // coplanar
    }

    Some([p0, p1, p2, p3])
}

fn axis_extremes(points: &[Vec3]) -> Vec<usize> {
    let mut out = Vec::new();
    for axis in 0..3 {
        let comp = |p: Vec3| match axis {
            0 => p.x,
            1 => p.y,
            _ => p.z,
        };
        let lo = (0..points.len()).min_by(|&a, &b| comp(points[a]).total_cmp(&comp(points[b]))).unwrap();
        let hi = (0..points.len()).max_by(|&a, &b| comp(points[a]).total_cmp(&comp(points[b]))).unwrap();
        out.push(lo);
        out.push(hi);
    }
    out.sort_unstable();
    out.dedup();
    out
}

fn dist_to_line(p: Vec3, origin: Vec3, dir: Vec3) -> f32 {
    let v = p - origin;
    (v - dir * v.dot(dir)).length()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube_points() -> Vec<Vec3> {
        vec![
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, -1.0),
            Vec3::new(-1.0, 1.0, -1.0),
            Vec3::new(-1.0, -1.0, 1.0),
            Vec3::new(1.0, -1.0, 1.0),
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(-1.0, 1.0, 1.0),
        ]
    }

    #[test]
    fn cube_hull_has_twelve_triangles() {
        let hull = convex_hull(&cube_points()).unwrap();
        // A box hull is 6 quads = 12 triangles, 8 vertices.
        assert_eq!(hull.face_count(), 12);
        assert_eq!(hull.vertex_count(), 8);
    }

    #[test]
    fn interior_points_are_ignored() {
        let mut pts = cube_points();
        // Add points strictly inside; the hull should be unchanged.
        pts.push(Vec3::ZERO);
        pts.push(Vec3::new(0.2, -0.3, 0.1));
        let hull = convex_hull(&pts).unwrap();
        assert_eq!(hull.vertex_count(), 8);
        assert_eq!(hull.face_count(), 12);
    }

    #[test]
    fn faces_point_outward() {
        let hull = convex_hull(&cube_points()).unwrap();
        let centroid: Vec3 = hull.vertices.iter().copied().sum::<Vec3>() / hull.vertices.len() as f32;
        for i in 0..hull.face_count() {
            let a = hull.vertices[hull.faces[i][0]];
            // Outward normal points away from the centroid.
            assert!(hull.face_normal(i).dot(a - centroid) > 0.0);
        }
    }

    #[test]
    fn contains_classifies_points() {
        let hull = convex_hull(&cube_points()).unwrap();
        assert!(hull.contains(Vec3::ZERO, 1e-5));
        assert!(hull.contains(Vec3::new(0.9, 0.9, 0.9), 1e-5));
        assert!(!hull.contains(Vec3::new(2.0, 0.0, 0.0), 1e-5));
    }

    #[test]
    fn to_mesh_triangle_count_matches() {
        let hull = convex_hull(&cube_points()).unwrap();
        let mesh = hull.to_mesh();
        assert_eq!(mesh.triangle_count(), hull.face_count());
    }

    #[test]
    fn degenerate_inputs_return_none() {
        // Too few points.
        assert!(convex_hull(&[Vec3::ZERO, Vec3::X, Vec3::Y]).is_none());
        // Coplanar (all z = 0).
        let flat = vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::new(1.0, 1.0, 0.0)];
        assert!(convex_hull(&flat).is_none());
    }

    #[test]
    fn tetrahedron_is_minimal_hull() {
        let pts = vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::Z];
        let hull = convex_hull(&pts).unwrap();
        assert_eq!(hull.vertex_count(), 4);
        assert_eq!(hull.face_count(), 4);
    }

    #[test]
    fn serde_roundtrip() {
        let hull = convex_hull(&cube_points()).unwrap();
        let json = serde_json::to_string(&hull).unwrap();
        let back: ConvexHull = serde_json::from_str(&json).unwrap();
        assert_eq!(back.face_count(), hull.face_count());
    }
}
