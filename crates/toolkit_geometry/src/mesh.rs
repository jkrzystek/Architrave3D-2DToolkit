use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;
use toolkit_core::{MaterialId, MeshId};

use crate::vertex::Vertex;

// ---------------------------------------------------------------------------
// Aabb
// ---------------------------------------------------------------------------

/// Axis-aligned bounding box.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl PartialEq for Aabb {
    fn eq(&self, other: &Self) -> bool {
        self.min == other.min && self.max == other.max
    }
}

impl Aabb {
    /// An empty AABB with inverted bounds, useful as the identity for expansion.
    pub const EMPTY: Self = Self {
        min: Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY),
        max: Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY),
    };

    /// Create an AABB from explicit min/max corners.
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Center point of the box.
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Full extents (max - min) along each axis.
    pub fn extents(&self) -> Vec3 {
        self.max - self.min
    }

    /// Half-extents (half the size along each axis).
    pub fn half_extents(&self) -> Vec3 {
        self.extents() * 0.5
    }

    /// Returns `true` if `point` lies inside or on the surface of this AABB.
    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    /// Returns `true` if this AABB overlaps with `other`.
    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }

    /// Expand this AABB to include the given point.
    pub fn expand_to_include(&mut self, point: Vec3) {
        self.min = self.min.min(point);
        self.max = self.max.max(point);
    }

    /// Expand this AABB to include another AABB.
    pub fn expand_to_include_aabb(&mut self, other: &Aabb) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    /// Surface area of the box (used in SAH cost calculations).
    pub fn surface_area(&self) -> f32 {
        let e = self.extents();
        2.0 * (e.x * e.y + e.y * e.z + e.z * e.x)
    }

    /// Build the smallest AABB enclosing all points in the iterator.
    pub fn from_points(iter: impl IntoIterator<Item = Vec3>) -> Self {
        let mut aabb = Self::EMPTY;
        for p in iter {
            aabb.expand_to_include(p);
        }
        aabb
    }
}

// ---------------------------------------------------------------------------
// SubMesh
// ---------------------------------------------------------------------------

/// A contiguous range of indices that share a material.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SubMesh {
    pub index_start: u32,
    pub index_count: u32,
    pub material_id: Option<MaterialId>,
}

// ---------------------------------------------------------------------------
// Mesh
// ---------------------------------------------------------------------------

/// CPU-side mesh data: vertices, indices, sub-meshes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mesh {
    pub id: MeshId,
    pub name: String,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub sub_meshes: Vec<SubMesh>,
}

impl Mesh {
    /// Create an empty mesh with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: MeshId::new(),
            name: name.into(),
            vertices: Vec::new(),
            indices: Vec::new(),
            sub_meshes: Vec::new(),
        }
    }

    /// Create a mesh populated with vertices and indices.
    pub fn with_vertices(
        name: impl Into<String>,
        vertices: Vec<Vertex>,
        indices: Vec<u32>,
    ) -> Self {
        Self {
            id: MeshId::new(),
            name: name.into(),
            vertices,
            indices,
            sub_meshes: Vec::new(),
        }
    }

    /// Number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Number of triangles (index count / 3).
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Returns `true` when the mesh has no vertices.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    /// Compute the axis-aligned bounding box of all vertex positions.
    pub fn bounding_box(&self) -> Aabb {
        Aabb::from_points(self.vertices.iter().map(|v| v.position_vec3()))
    }

    // -- Primitive generators ------------------------------------------------

    /// Generate an axis-aligned cube centered at the origin.
    pub fn cube(size: f32) -> Self {
        let h = size * 0.5;

        // 6 faces, each with 4 vertices and 2 triangles.
        let face_data: [(Vec3, [Vec3; 4], [Vec2; 4]); 6] = [
            // +Z face (front)
            (
                Vec3::Z,
                [
                    Vec3::new(-h, -h, h),
                    Vec3::new(h, -h, h),
                    Vec3::new(h, h, h),
                    Vec3::new(-h, h, h),
                ],
                [
                    Vec2::new(0.0, 1.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(0.0, 0.0),
                ],
            ),
            // -Z face (back)
            (
                Vec3::NEG_Z,
                [
                    Vec3::new(h, -h, -h),
                    Vec3::new(-h, -h, -h),
                    Vec3::new(-h, h, -h),
                    Vec3::new(h, h, -h),
                ],
                [
                    Vec2::new(0.0, 1.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(0.0, 0.0),
                ],
            ),
            // +X face (right)
            (
                Vec3::X,
                [
                    Vec3::new(h, -h, h),
                    Vec3::new(h, -h, -h),
                    Vec3::new(h, h, -h),
                    Vec3::new(h, h, h),
                ],
                [
                    Vec2::new(0.0, 1.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(0.0, 0.0),
                ],
            ),
            // -X face (left)
            (
                Vec3::NEG_X,
                [
                    Vec3::new(-h, -h, -h),
                    Vec3::new(-h, -h, h),
                    Vec3::new(-h, h, h),
                    Vec3::new(-h, h, -h),
                ],
                [
                    Vec2::new(0.0, 1.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(0.0, 0.0),
                ],
            ),
            // +Y face (top)
            (
                Vec3::Y,
                [
                    Vec3::new(-h, h, h),
                    Vec3::new(h, h, h),
                    Vec3::new(h, h, -h),
                    Vec3::new(-h, h, -h),
                ],
                [
                    Vec2::new(0.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(0.0, 1.0),
                ],
            ),
            // -Y face (bottom)
            (
                Vec3::NEG_Y,
                [
                    Vec3::new(-h, -h, -h),
                    Vec3::new(h, -h, -h),
                    Vec3::new(h, -h, h),
                    Vec3::new(-h, -h, h),
                ],
                [
                    Vec2::new(0.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(1.0, 1.0),
                    Vec2::new(0.0, 1.0),
                ],
            ),
        ];

        let mut vertices = Vec::with_capacity(24);
        let mut indices = Vec::with_capacity(36);

        for (normal, positions, uvs) in &face_data {
            let base = vertices.len() as u32;
            for i in 0..4 {
                vertices.push(Vertex::new(positions[i], *normal, uvs[i]));
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        Self::with_vertices("cube", vertices, indices)
    }

    /// Generate a plane on the XZ axis centered at the origin, facing +Y.
    ///
    /// `subdivisions` is the number of quads along each axis (minimum 1).
    pub fn plane(width: f32, height: f32, subdivisions: u32) -> Self {
        let subdivisions = subdivisions.max(1);
        let rows = subdivisions + 1;
        let cols = subdivisions + 1;

        let mut vertices = Vec::with_capacity((rows * cols) as usize);
        let mut indices = Vec::with_capacity((subdivisions * subdivisions * 6) as usize);

        for row in 0..rows {
            for col in 0..cols {
                let u = col as f32 / subdivisions as f32;
                let v = row as f32 / subdivisions as f32;
                let x = (u - 0.5) * width;
                let z = (v - 0.5) * height;
                vertices.push(Vertex::new(
                    Vec3::new(x, 0.0, z),
                    Vec3::Y,
                    Vec2::new(u, v),
                ));
            }
        }

        for row in 0..subdivisions {
            for col in 0..subdivisions {
                let tl = row * cols + col;
                let tr = tl + 1;
                let bl = tl + cols;
                let br = bl + 1;
                indices.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
            }
        }

        Self::with_vertices("plane", vertices, indices)
    }

    /// Generate a UV sphere centered at the origin.
    ///
    /// * `sectors` - longitudinal slices (minimum 3)
    /// * `stacks` - latitudinal slices (minimum 2)
    pub fn uv_sphere(radius: f32, sectors: u32, stacks: u32) -> Self {
        let sectors = sectors.max(3);
        let stacks = stacks.max(2);

        let mut vertices = Vec::with_capacity(((stacks + 1) * (sectors + 1)) as usize);
        let mut indices = Vec::new();

        for i in 0..=stacks {
            let stack_angle = PI / 2.0 - (i as f32) * PI / (stacks as f32);
            let xy = radius * stack_angle.cos();
            let y = radius * stack_angle.sin();

            for j in 0..=sectors {
                let sector_angle = 2.0 * PI * (j as f32) / (sectors as f32);
                let x = xy * sector_angle.cos();
                let z = xy * sector_angle.sin();

                let position = Vec3::new(x, y, z);
                let normal = position.normalize_or_zero();
                let u = j as f32 / sectors as f32;
                let v = i as f32 / stacks as f32;

                vertices.push(Vertex::new(position, normal, Vec2::new(u, v)));
            }
        }

        // Indices
        for i in 0..stacks {
            for j in 0..sectors {
                let first = i * (sectors + 1) + j;
                let second = first + sectors + 1;

                if i != 0 {
                    indices.extend_from_slice(&[first, second, first + 1]);
                }
                if i != stacks - 1 {
                    indices.extend_from_slice(&[first + 1, second, second + 1]);
                }
            }
        }

        Self::with_vertices("uv_sphere", vertices, indices)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Cube ----------------------------------------------------------------

    #[test]
    fn cube_vertex_and_index_counts() {
        let cube = Mesh::cube(1.0);
        assert_eq!(cube.vertex_count(), 24); // 6 faces * 4 verts
        assert_eq!(cube.triangle_count(), 12); // 6 faces * 2 tris
    }

    #[test]
    fn cube_bounding_box() {
        let cube = Mesh::cube(2.0);
        let bb = cube.bounding_box();
        let eps = 1e-5;
        assert!((bb.min.x - (-1.0)).abs() < eps);
        assert!((bb.min.y - (-1.0)).abs() < eps);
        assert!((bb.min.z - (-1.0)).abs() < eps);
        assert!((bb.max.x - 1.0).abs() < eps);
        assert!((bb.max.y - 1.0).abs() < eps);
        assert!((bb.max.z - 1.0).abs() < eps);
    }

    #[test]
    fn cube_normals_point_outward() {
        let cube = Mesh::cube(1.0);
        // For each face the normal should be a unit axis direction.
        for v in &cube.vertices {
            let n = v.normal_vec3();
            let len = n.length();
            assert!(
                (len - 1.0).abs() < 1e-5,
                "normal is not unit length: {:?}",
                n
            );
            let abs = n.abs();
            // Exactly one component should be 1.
            let ones = (abs.x > 0.5) as u32 + (abs.y > 0.5) as u32 + (abs.z > 0.5) as u32;
            assert_eq!(ones, 1, "normal should be axis-aligned: {:?}", n);
        }
    }

    // -- Plane ---------------------------------------------------------------

    #[test]
    fn plane_vertex_and_index_counts() {
        let plane = Mesh::plane(1.0, 1.0, 3);
        // (3+1)^2 = 16 verts, 3*3*2 = 18 tris
        assert_eq!(plane.vertex_count(), 16);
        assert_eq!(plane.triangle_count(), 18);
    }

    #[test]
    fn plane_is_flat_on_y() {
        let plane = Mesh::plane(5.0, 5.0, 2);
        for v in &plane.vertices {
            assert!((v.position_vec3().y).abs() < 1e-6);
            assert_eq!(v.normal_vec3(), Vec3::Y);
        }
    }

    // -- UV sphere -----------------------------------------------------------

    #[test]
    fn sphere_vertex_count() {
        let sphere = Mesh::uv_sphere(1.0, 16, 8);
        assert_eq!(sphere.vertex_count(), (8 + 1) * (16 + 1)); // 153
    }

    #[test]
    fn sphere_bounding_box() {
        let sphere = Mesh::uv_sphere(2.0, 32, 16);
        let bb = sphere.bounding_box();
        let eps = 0.1; // sphere approximation
        assert!(bb.min.x >= -2.0 - eps);
        assert!(bb.max.x <= 2.0 + eps);
        assert!(bb.min.y >= -2.0 - eps);
        assert!(bb.max.y <= 2.0 + eps);
    }

    #[test]
    fn sphere_normals_point_outward() {
        let sphere = Mesh::uv_sphere(1.0, 16, 8);
        for v in &sphere.vertices {
            let pos = v.position_vec3();
            if pos.length() < 1e-6 {
                continue; // degenerate pole vertex
            }
            let expected = pos.normalize();
            let n = v.normal_vec3();
            let diff = (n - expected).length();
            assert!(
                diff < 1e-4,
                "normal {:?} does not match expected {:?} for position {:?}",
                n,
                expected,
                pos
            );
        }
    }

    // -- Aabb ----------------------------------------------------------------

    #[test]
    fn aabb_contains_point() {
        let bb = Aabb::new(Vec3::ZERO, Vec3::ONE);
        assert!(bb.contains_point(Vec3::splat(0.5)));
        assert!(!bb.contains_point(Vec3::splat(1.5)));
        assert!(bb.contains_point(Vec3::ZERO)); // boundary
    }

    #[test]
    fn aabb_intersects() {
        let a = Aabb::new(Vec3::ZERO, Vec3::ONE);
        let b = Aabb::new(Vec3::splat(0.5), Vec3::splat(1.5));
        assert!(a.intersects(&b));
        let c = Aabb::new(Vec3::splat(2.0), Vec3::splat(3.0));
        assert!(!a.intersects(&c));
    }

    #[test]
    fn aabb_expand_to_include() {
        let mut bb = Aabb::EMPTY;
        bb.expand_to_include(Vec3::new(-1.0, 0.0, 0.0));
        bb.expand_to_include(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(bb.min, Vec3::new(-1.0, 0.0, 0.0));
        assert_eq!(bb.max, Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn aabb_surface_area() {
        let bb = Aabb::new(Vec3::ZERO, Vec3::new(2.0, 3.0, 4.0));
        // 2*(2*3 + 3*4 + 4*2) = 2*(6+12+8) = 52
        assert!((bb.surface_area() - 52.0).abs() < 1e-6);
    }

    #[test]
    fn aabb_from_points() {
        let points = vec![
            Vec3::new(1.0, -2.0, 0.0),
            Vec3::new(-3.0, 4.0, 5.0),
            Vec3::ZERO,
        ];
        let bb = Aabb::from_points(points);
        assert_eq!(bb.min, Vec3::new(-3.0, -2.0, 0.0));
        assert_eq!(bb.max, Vec3::new(1.0, 4.0, 5.0));
    }

    #[test]
    fn mesh_is_empty() {
        let m = Mesh::new("empty");
        assert!(m.is_empty());
        let m = Mesh::cube(1.0);
        assert!(!m.is_empty());
    }
}
