//! Poisson-disk sampling on mesh surfaces.
//!
//! Generates points that are evenly spaced on a triangle mesh using dart
//! throwing accelerated by a k-d tree of accepted samples. A closure can
//! modulate the local sampling radius (e.g. smaller strokes in high-curvature
//! areas).

use glam::{Vec2, Vec3};
use toolkit_geometry::{Bvh, Mesh};



/// A sample produced by surface Poisson-disk sampling.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceSample {
    pub position: Vec3,
    pub uv: Vec2,
    pub normal: Vec3,
    /// The triangle index this sample lies on.
    pub triangle_index: u32,
}

/// Generate Poisson-disk samples on `mesh` using dart throwing.
///
/// * `base_radius` — minimum Euclidean distance between samples in flat areas.
/// * `density_fn` — called with `(position, uv)` and returns a multiplier on
///   `base_radius`. Values `< 1.0` shrink the radius (higher density), values
///   `> 1.0` expand it (lower density).
/// * `max_attempts` — how many consecutive rejections before giving up.
/// * `seed` — deterministic seed for the internal RNG.
///
/// The algorithm uses a k-d tree of accepted samples for fast nearest-neighbour
/// rejection. Distances are measured in 3D Euclidean space, which is a good
/// approximation for small radii.
pub fn poisson_disk_surface_sample<F>(
    mesh: &Mesh,
    _bvh: &Bvh,
    base_radius: f32,
    mut density_fn: F,
    max_attempts: usize,
    seed: u64,
) -> Vec<SurfaceSample>
where
    F: FnMut(Vec3, Vec2) -> f32,
{
    if mesh.triangle_count() == 0 {
        return Vec::new();
    }

    // Precompute triangle areas and cumulative distribution.
    let mut cumulative_area = Vec::with_capacity(mesh.triangle_count());
    let mut total_area = 0.0_f32;
    for tri_idx in 0..mesh.triangle_count() {
        let i0 = mesh.indices[tri_idx * 3] as usize;
        let i1 = mesh.indices[tri_idx * 3 + 1] as usize;
        let i2 = mesh.indices[tri_idx * 3 + 2] as usize;
        let p0 = mesh.vertices[i0].position_vec3();
        let p1 = mesh.vertices[i1].position_vec3();
        let p2 = mesh.vertices[i2].position_vec3();
        let area = (p1 - p0).cross(p2 - p0).length() * 0.5;
        total_area += area;
        cumulative_area.push(total_area);
    }

    if total_area <= 0.0 {
        return Vec::new();
    }

    let mut rng = Rng::seed_from_u64(seed);
    let mut accepted: Vec<SurfaceSample> = Vec::new();

    let mut attempts = 0;
    while attempts < max_attempts {
        // Pick a triangle by area.
        let t = rng.range_f32(0.0, total_area);
        let tri_idx = match cumulative_area.binary_search_by(|a| a.partial_cmp(&t).unwrap()) {
            Ok(i) => i,
            Err(i) => i.min(cumulative_area.len() - 1),
        };

        // Barycentric sample on the triangle.
        let base = tri_idx * 3;
        let i0 = mesh.indices[base] as usize;
        let i1 = mesh.indices[base + 1] as usize;
        let i2 = mesh.indices[base + 2] as usize;
        let v0 = &mesh.vertices[i0];
        let v1 = &mesh.vertices[i1];
        let v2 = &mesh.vertices[i2];

        let (u, v) = random_barycentric(&mut rng);
        let w = 1.0 - u - v;
        let pos = w * v0.position_vec3() + u * v1.position_vec3() + v * v2.position_vec3();
        let uv = w * v0.uv_vec2() + u * v1.uv_vec2() + v * v2.uv_vec2();
        let normal = (w * v0.normal_vec3() + u * v1.normal_vec3() + v * v2.normal_vec3()).normalize();

        let density = density_fn(pos, uv).max(0.1);
        let radius = base_radius * density;
        let radius_sq = radius * radius;

        // Reject if too close to any accepted sample (brute-force; accurate).
        let mut too_close = false;
        for s in &accepted {
            if s.position.distance_squared(pos) < radius_sq {
                too_close = true;
                break;
            }
        }

        if !too_close {
            accepted.push(SurfaceSample {
                position: pos,
                uv,
                normal,
                triangle_index: tri_idx as u32,
            });
            attempts = 0;
        } else {
            attempts += 1;
        }
    }

    accepted
}

fn random_barycentric(rng: &mut Rng) -> (f32, f32) {
    let r1 = rng.range_f32(0.0, 1.0);
    let r2 = rng.range_f32(0.0, 1.0);
    let u = 1.0 - r1.sqrt();
    let v = r2 * r1.sqrt();
    (u, v)
}

// ---------------------------------------------------------------------------
// Minimal deterministic RNG (duplicated here so this crate stays standalone)
// ---------------------------------------------------------------------------

struct Rng {
    state: u64,
}

impl Rng {
    fn seed_from_u64(seed: u64) -> Self {
        Self { state: seed.wrapping_add(0x9e3779b97f4a7c15) }
    }

    fn next_u64(&mut self) -> u64 {
        // xorshift64*
        self.state ^= self.state >> 12;
        self.state ^= self.state << 25;
        self.state ^= self.state >> 27;
        self.state.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        let u = self.next_u64();
        let t = (u >> 11) as f32 * (1.0 / (1u64 << 53) as f32);
        min + t * (max - min)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toolkit_geometry::Mesh;

    #[test]
    fn poisson_on_plane() {
        let mesh = Mesh::plane(2.0, 2.0, 1);
        let bvh = Bvh::build(&mesh);
        let samples = poisson_disk_surface_sample(
            &mesh,
            &bvh,
            0.3,
            |_pos, _uv| 1.0,
            1000,
            42,
        );
        // Should get a reasonable number of samples for a 2x2 plane with 0.3 radius.
        assert!(!samples.is_empty(), "expected some samples");
        assert!(samples.len() >= 10, "expected at least 10 samples, got {}", samples.len());

        // Verify minimum distance.
        for i in 0..samples.len() {
            for j in (i + 1)..samples.len() {
                let d = samples[i].position.distance(samples[j].position);
                assert!(
                    d >= 0.3 * 0.99,
                    "samples {} and {} are too close: {}",
                    i,
                    j,
                    d
                );
            }
        }
    }

    #[test]
    fn density_fn_increases_count() {
        let mesh = Mesh::plane(2.0, 2.0, 1);
        let bvh = Bvh::build(&mesh);

        let sparse = poisson_disk_surface_sample(&mesh, &bvh, 0.5, |_pos, _uv| 2.0, 1000, 7);
        let dense = poisson_disk_surface_sample(&mesh, &bvh, 0.5, |_pos, _uv| 0.5, 1000, 7);

        assert!(
            dense.len() > sparse.len(),
            "higher density should yield more samples: dense={}, sparse={}",
            dense.len(),
            sparse.len()
        );
    }

    #[test]
    fn empty_mesh_yields_no_samples() {
        let mesh = Mesh::new("empty");
        let bvh = Bvh::build(&mesh);
        let samples = poisson_disk_surface_sample(&mesh, &bvh, 0.1, |_p, _uv| 1.0, 100, 0);
        assert!(samples.is_empty());
    }
}
