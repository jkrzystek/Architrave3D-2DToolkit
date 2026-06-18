//! Rasterize a mesh's UV charts into a texture-space *geometry buffer*: for
//! every texel covered by a triangle, the interpolated surface position and
//! normal. This is the foundation every bake (normal, position, AO) reads from.

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use toolkit_geometry::Mesh;

/// Per-texel surface data recovered by UV rasterization.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeometrySample {
    /// Interpolated model-space position.
    pub position: Vec3,
    /// Interpolated (renormalised) model-space normal.
    pub normal: Vec3,
    /// Source triangle index.
    pub triangle: u32,
}

/// A texture-space buffer of optional [`GeometrySample`]s — `None` where no
/// triangle covers the texel.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GBuffer {
    width: u32,
    height: u32,
    samples: Vec<Option<GeometrySample>>,
}

impl GBuffer {
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn at(&self, x: u32, y: u32) -> Option<&GeometrySample> {
        self.samples
            .get((y as usize) * (self.width as usize) + x as usize)
            .and_then(|s| s.as_ref())
    }

    /// Number of covered texels.
    pub fn filled_count(&self) -> usize {
        self.samples.iter().filter(|s| s.is_some()).count()
    }
}

/// Rasterize `mesh` into a `width`×`height` geometry buffer using its UVs as
/// texture coordinates. Triangles are sampled at texel centres; the last
/// triangle to cover a texel wins (charts are assumed non-overlapping).
pub fn rasterize_gbuffer(mesh: &Mesh, width: u32, height: u32) -> GBuffer {
    let mut samples = vec![None; (width as usize) * (height as usize)];
    if width == 0 || height == 0 {
        return GBuffer { width, height, samples };
    }

    let (fw, fh) = (width as f32, height as f32);

    for tri in 0..mesh.triangle_count() {
        let i0 = mesh.indices[tri * 3] as usize;
        let i1 = mesh.indices[tri * 3 + 1] as usize;
        let i2 = mesh.indices[tri * 3 + 2] as usize;
        let (v0, v1, v2) = (&mesh.vertices[i0], &mesh.vertices[i1], &mesh.vertices[i2]);

        // UVs scaled into texel space.
        let t0 = v0.uv_vec2() * Vec2::new(fw, fh);
        let t1 = v1.uv_vec2() * Vec2::new(fw, fh);
        let t2 = v2.uv_vec2() * Vec2::new(fw, fh);

        // Texel bounding box (clamped to the buffer).
        let min = t0.min(t1).min(t2);
        let max = t0.max(t1).max(t2);
        let x0 = (min.x.floor().max(0.0)) as u32;
        let y0 = (min.y.floor().max(0.0)) as u32;
        let x1 = (max.x.ceil().min(fw)) as u32;
        let y1 = (max.y.ceil().min(fh)) as u32;

        for y in y0..y1 {
            for x in x0..x1 {
                let p = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                let Some((a, b, c)) = barycentric(p, t0, t1, t2) else {
                    continue;
                };
                // Small epsilon so shared edges fill without gaps.
                if a < -1e-4 || b < -1e-4 || c < -1e-4 {
                    continue;
                }
                let position = a * v0.position_vec3() + b * v1.position_vec3() + c * v2.position_vec3();
                let normal = (a * v0.normal_vec3() + b * v1.normal_vec3() + c * v2.normal_vec3())
                    .normalize_or_zero();
                let idx = (y as usize) * (width as usize) + x as usize;
                samples[idx] = Some(GeometrySample {
                    position,
                    normal,
                    triangle: tri as u32,
                });
            }
        }
    }

    GBuffer { width, height, samples }
}

/// Barycentric coordinates of `p` with respect to triangle `(a, b, c)`.
/// Returns `None` for a degenerate triangle.
fn barycentric(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> Option<(f32, f32, f32)> {
    let v0 = b - a;
    let v1 = c - a;
    let v2 = p - a;
    let d00 = v0.dot(v0);
    let d01 = v0.dot(v1);
    let d11 = v1.dot(v1);
    let d20 = v2.dot(v0);
    let d21 = v2.dot(v1);
    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-12 {
        return None;
    }
    let v = (d11 * d20 - d01 * d21) / denom;
    let w = (d00 * d21 - d01 * d20) / denom;
    Some((1.0 - v - w, v, w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plane_fills_texels() {
        let plane = Mesh::plane(2.0, 2.0, 1);
        let gb = rasterize_gbuffer(&plane, 16, 16);
        // A full-UV plane should cover most of the texture.
        assert!(gb.filled_count() > 200);
    }

    #[test]
    fn empty_buffer_for_zero_size() {
        let plane = Mesh::plane(2.0, 2.0, 1);
        let gb = rasterize_gbuffer(&plane, 0, 0);
        assert_eq!(gb.filled_count(), 0);
    }

    #[test]
    fn samples_carry_normals() {
        let plane = Mesh::plane(2.0, 2.0, 1);
        let gb = rasterize_gbuffer(&plane, 8, 8);
        let filled = (0..8)
            .flat_map(|y| (0..8).map(move |x| (x, y)))
            .find_map(|(x, y)| gb.at(x, y));
        let s = filled.expect("at least one filled texel");
        // A flat XZ plane faces +Y.
        assert!(s.normal.length() > 0.5);
    }

    #[test]
    fn barycentric_centroid() {
        let a = Vec2::ZERO;
        let b = Vec2::new(3.0, 0.0);
        let c = Vec2::new(0.0, 3.0);
        let (u, v, w) = barycentric(Vec2::new(1.0, 1.0), a, b, c).unwrap();
        assert!((u - 1.0 / 3.0).abs() < 1e-5);
        assert!((v - 1.0 / 3.0).abs() < 1e-5);
        assert!((w - 1.0 / 3.0).abs() < 1e-5);
    }

    #[test]
    fn serde_roundtrip() {
        let plane = Mesh::plane(2.0, 2.0, 1);
        let gb = rasterize_gbuffer(&plane, 4, 4);
        let json = serde_json::to_string(&gb).unwrap();
        let back: GBuffer = serde_json::from_str(&json).unwrap();
        assert_eq!(back.filled_count(), gb.filled_count());
    }
}
