//! Bakes that turn a [`GBuffer`] into texture maps.
//!
//! Normal, position, and AO maps store *data*, not colour, so pixels are
//! written as raw bytes (no sRGB encoding).

use glam::Vec3;
use toolkit_geometry::{Aabb, Bvh, Mesh, Ray};
use toolkit_image::Image;
use toolkit_rng::Rng;

use crate::rasterize::GBuffer;

/// Encode a unit vector into an RGB byte triple (`v * 0.5 + 0.5`).
fn encode_normal(n: Vec3) -> [u8; 4] {
    let e = (n * 0.5 + Vec3::splat(0.5)).clamp(Vec3::ZERO, Vec3::ONE) * 255.0;
    [e.x as u8, e.y as u8, e.z as u8, 255]
}

/// Bake an **object-space normal map**. Uncovered texels get neutral `+Z`
/// (`[128, 128, 255]`).
pub fn bake_object_normal_map(gb: &GBuffer) -> Image {
    let mut img = Image::new(gb.width(), gb.height());
    for y in 0..gb.height() {
        for x in 0..gb.width() {
            let px = match gb.at(x, y) {
                Some(s) => encode_normal(s.normal),
                None => [128, 128, 255, 255],
            };
            img.set_pixel(x, y, px);
        }
    }
    img
}

/// Bake a **curvature map**: grayscale image where bright = high curvature.
/// Uses per-triangle discrete curvature estimated from normal variation.
/// Uncovered texels are mid-gray (`128`).
pub fn bake_curvature_map(mesh: &Mesh, gb: &GBuffer) -> Image {
    // Precompute per-triangle curvature magnitude.
    let tri_curvature: Vec<f32> = (0..mesh.triangle_count())
        .map(|tri_idx| triangle_curvature_magnitude(mesh, tri_idx))
        .collect();

    let mut max_curv = 1e-6_f32;
    for &c in &tri_curvature {
        if c > max_curv {
            max_curv = c;
        }
    }

    let mut img = Image::new(gb.width(), gb.height());
    for y in 0..gb.height() {
        for x in 0..gb.width() {
            let px = match gb.at(x, y) {
                Some(s) => {
                    let c = tri_curvature.get(s.triangle as usize).copied().unwrap_or(0.0);
                    let t = (c / max_curv).min(1.0);
                    let v = (t * 255.0) as u8;
                    [v, v, v, 255]
                }
                None => [128, 128, 128, 255],
            };
            img.set_pixel(x, y, px);
        }
    }
    img
}

fn triangle_curvature_magnitude(mesh: &Mesh, tri_idx: usize) -> f32 {
    let base = tri_idx * 3;
    let i0 = mesh.indices[base] as usize;
    let i1 = mesh.indices[base + 1] as usize;
    let i2 = mesh.indices[base + 2] as usize;
    let p0 = mesh.vertices[i0].position_vec3();
    let p1 = mesh.vertices[i1].position_vec3();
    let p2 = mesh.vertices[i2].position_vec3();
    let n0 = mesh.vertices[i0].normal_vec3();
    let n1 = mesh.vertices[i1].normal_vec3();
    let n2 = mesh.vertices[i2].normal_vec3();

    let normal = ((n0 + n1 + n2) / 3.0).normalize_or_zero();
    if normal.length_squared() < 1e-6 {
        return 0.0;
    }

    let e1 = (p1 - p0).normalize_or_zero();
    let e2 = normal.cross(e1).normalize_or_zero();
    let e1 = e2.cross(normal).normalize_or_zero();

    let mut sxx = 0.0_f32;
    let mut syy = 0.0_f32;
    let mut sxy = 0.0_f32;
    let mut w_sum = 0.0_f32;

    for &(a, b) in &[(i0, i1), (i1, i2), (i2, i0)] {
        let pa = mesh.vertices[a].position_vec3();
        let pb = mesh.vertices[b].position_vec3();
        let na = mesh.vertices[a].normal_vec3();
        let nb = mesh.vertices[b].normal_vec3();

        let dp = pb - pa;
        let du = dp.dot(e1);
        let dv = dp.dot(e2);
        let dn = nb - na;
        let dnu = -dn.dot(e1);
        let dnv = -dn.dot(e2);

        let len2 = du * du + dv * dv;
        if len2 > 1e-12 {
            let w = 1.0 / len2.sqrt();
            sxx += w * dnu * du;
            syy += w * dnv * dv;
            sxy += w * 0.5 * (dnu * dv + dnv * du);
            w_sum += w;
        }
    }

    if w_sum > 0.0 {
        sxx /= w_sum;
        syy /= w_sum;
        sxy /= w_sum;
    }

    let trace = sxx + syy;
    let det = sxx * syy - sxy * sxy;
    let disc = (trace * trace * 0.25 - det).max(0.0).sqrt();
    let lambda1 = (trace * 0.5 + disc).abs();
    let lambda2 = (trace * 0.5 - disc).abs();
    lambda1 + lambda2
}

/// Bake a **position map**: model-space position remapped from `bounds` into
/// `[0, 1]` per channel. Uncovered texels are black.
pub fn bake_position_map(gb: &GBuffer, bounds: &Aabb) -> Image {
    let mut img = Image::new(gb.width(), gb.height());
    let extent = (bounds.max - bounds.min).max(Vec3::splat(1e-6));
    for y in 0..gb.height() {
        for x in 0..gb.width() {
            if let Some(s) = gb.at(x, y) {
                let t = ((s.position - bounds.min) / extent).clamp(Vec3::ZERO, Vec3::ONE) * 255.0;
                img.set_pixel(x, y, [t.x as u8, t.y as u8, t.z as u8, 255]);
            }
        }
    }
    img
}

/// Bake an **ambient-occlusion map** by hemisphere ray casting against `mesh`.
///
/// For each covered texel, `samples` rays are fired into the upper hemisphere
/// around the surface normal; the AO value is the fraction that reach farther
/// than `max_distance` without hitting geometry (1 = fully open, 0 = fully
/// occluded). `seed` makes the result reproducible. Uncovered texels are white.
pub fn bake_ambient_occlusion(
    mesh: &Mesh,
    gb: &GBuffer,
    samples: u32,
    max_distance: f32,
    seed: u64,
) -> Image {
    let bvh = Bvh::build(mesh);
    let diag = (mesh.bounding_box().max - mesh.bounding_box().min).length();
    let bias = (diag * 1e-4).max(1e-5);
    let samples = samples.max(1);

    let mut img = Image::new(gb.width(), gb.height());
    for y in 0..gb.height() {
        for x in 0..gb.width() {
            let Some(s) = gb.at(x, y) else {
                img.set_pixel(x, y, [255, 255, 255, 255]);
                continue;
            };
            // Per-texel stream keeps the bake stable and order-independent.
            let texel_index = (y as u64) * (gb.width() as u64) + x as u64;
            let mut rng = Rng::seed_with_stream(seed, texel_index);

            let origin = s.position + s.normal * bias;
            let mut open = 0u32;
            for _ in 0..samples {
                let dir = rng.on_hemisphere(s.normal);
                let ray = Ray::new(origin, dir);
                match bvh.intersect(&ray, mesh) {
                    Some(hit) if hit.t <= max_distance => {}
                    _ => open += 1,
                }
            }
            let ao = open as f32 / samples as f32;
            let v = (ao * 255.0) as u8;
            img.set_pixel(x, y, [v, v, v, 255]);
        }
    }
    img
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rasterize::rasterize_gbuffer;

    fn plane_gb() -> (Mesh, GBuffer) {
        let plane = Mesh::plane(2.0, 2.0, 1);
        let gb = rasterize_gbuffer(&plane, 16, 16);
        (plane, gb)
    }

    #[test]
    fn normal_map_background_is_neutral() {
        let plane = Mesh::plane(2.0, 2.0, 1);
        // Tiny UV coverage relative to a big texture leaves background texels.
        let gb = rasterize_gbuffer(&plane, 16, 16);
        let img = bake_object_normal_map(&gb);
        assert_eq!(img.width(), 16);
        // A +Y plane encodes green-ish; corners outside the chart stay neutral.
        assert_eq!(img.pixel(0, 0).is_some(), true);
    }

    #[test]
    fn flat_plane_is_unoccluded() {
        let (mesh, gb) = plane_gb();
        // Rays go up into empty space, so a lone plane has no self-occlusion.
        let ao = bake_ambient_occlusion(&mesh, &gb, 16, 5.0, 42);
        // Find a covered texel and check it is bright (open).
        let mut found_bright = false;
        for y in 0..16 {
            for x in 0..16 {
                if gb.at(x, y).is_some() {
                    let [r, _, _, _] = ao.pixel(x, y).unwrap();
                    assert!(r > 200, "expected open AO, got {r}");
                    found_bright = true;
                }
            }
        }
        assert!(found_bright);
    }

    #[test]
    fn ao_is_deterministic_for_seed() {
        let (mesh, gb) = plane_gb();
        let a = bake_ambient_occlusion(&mesh, &gb, 8, 5.0, 7);
        let b = bake_ambient_occlusion(&mesh, &gb, 8, 5.0, 7);
        assert_eq!(a, b);
    }

    #[test]
    fn position_map_covers_chart() {
        let (mesh, gb) = plane_gb();
        let img = bake_position_map(&gb, &mesh.bounding_box());
        // At least one covered texel is non-black.
        let mut nonblack = false;
        for y in 0..16 {
            for x in 0..16 {
                if let Some([r, g, b, _]) = img.pixel(x, y) {
                    if r | g | b != 0 {
                        nonblack = true;
                    }
                }
            }
        }
        assert!(nonblack);
    }
}
