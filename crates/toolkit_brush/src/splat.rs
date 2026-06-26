//! UV-space stroke splatting for generative mesh painting.
//!
//! Projects a 3D brush stroke onto mesh triangles, then maps the affected
//! triangle fragments into UV space to paint seamlessly across texture islands.

use glam::{Mat3, Vec2, Vec3};
use toolkit_geometry::Mesh;
use toolkit_image::Image;

/// Reoriented Normal Mapping (RNM) blend.
///
/// `base` and `detail` are encoded normals in `[0,1]` (object-space or
/// tangent-space). Returns a blended normal also in `[0,1]`.
pub fn rnm_blend(base: Vec3, detail: Vec3) -> Vec3 {
    let t = base * Vec3::new(2.0, 2.0, 2.0) + Vec3::new(-1.0, -1.0, 0.0);
    let u = detail * Vec3::new(-2.0, -2.0, 2.0) + Vec3::new(1.0, 1.0, -1.0);
    let r = t * t.dot(u) - u * t.z;
    (r.normalize() * 0.5 + Vec3::splat(0.5)).clamp(Vec3::ZERO, Vec3::ONE)
}

/// Target maps for a single stroke splat.
pub struct SplatTarget<'a> {
    pub color_map: &'a mut Image,
    pub normal_map: &'a mut Image,
    pub roughness_map: &'a mut Image,
}

/// Splat a brush stroke from a 3D anchor onto UV maps.
///
/// * `mesh` — the mesh to paint on.
/// * `targets` — the UV target maps to write into.
/// * `anchor` — world-space center of the stroke.
/// * `rotation` — `Mat3` whose first column is the brush X axis, second column
///   is the brush Y axis, third column is the surface normal. The brush is
///   oriented in the XY plane of this frame.
/// * `base_color` — RGB stroke color.
/// * `size` — brush radius in world units.
/// * `brush_alpha` — a closure `|dist|` that returns opacity in `[0,1]` where
///   `dist` is the normalized distance from the brush center.
/// * `normal_offset` — optional object-space normal perturbation to stamp.
pub fn splat_stroke_seamless(
    mesh: &Mesh,
    targets: SplatTarget,
    anchor: Vec3,
    rotation: Mat3,
    base_color: Vec3,
    size: f32,
    brush_alpha: impl Fn(f32) -> f32,
    normal_offset: Option<Vec3>,
) {
    if size <= 0.0 || mesh.triangle_count() == 0 {
        return;
    }

    let inv_rot = rotation.transpose();
    let size_sq = size * size;
    let (cw, ch) = (targets.color_map.width(), targets.color_map.height());

    for tri_idx in 0..mesh.triangle_count() {
        let base = tri_idx * 3;
        let i0 = mesh.indices[base] as usize;
        let i1 = mesh.indices[base + 1] as usize;
        let i2 = mesh.indices[base + 2] as usize;
        let v0 = &mesh.vertices[i0];
        let v1 = &mesh.vertices[i1];
        let v2 = &mesh.vertices[i2];

        let p0 = v0.position_vec3();
        let p1 = v1.position_vec3();
        let p2 = v2.position_vec3();

        // Quick reject via triangle bbox.
        let min = p0.min(p1).min(p2);
        let max = p0.max(p1).max(p2);
        let closest = anchor.clamp(min, max);
        if closest.distance_squared(anchor) > size_sq {
            continue;
        }

        // Subdivide the triangle and splat each sub-sample.
        let subdiv = 4usize;
        for iy in 0..=subdiv {
            for ix in 0..=subdiv {
                let u = ix as f32 / subdiv as f32;
                let v = iy as f32 / subdiv as f32;
                if u + v > 1.0 {
                    continue;
                }
                let w = 1.0 - u - v;

                let pos = w * p0 + u * p1 + v * p2;
                let local = inv_rot * (pos - anchor);
                let dist_sq = local.x * local.x + local.y * local.y;
                if dist_sq > size_sq {
                    continue;
                }

                let dist = dist_sq.sqrt() / size;
                let alpha = brush_alpha(dist).clamp(0.0, 1.0);
                if alpha < 1.0 / 255.0 {
                    continue;
                }

                let uv = w * v0.uv_vec2() + u * v1.uv_vec2() + v * v2.uv_vec2();
                let tx = (uv.x * cw as f32).clamp(0.0, cw as f32 - 1.0) as u32;
                let ty = (uv.y * ch as f32).clamp(0.0, ch as f32 - 1.0) as u32;

                // Color splat.
                let existing = sample_linear(targets.color_map, uv);
                let blended = existing.lerp(base_color, alpha);
                targets.color_map.set_pixel(tx, ty, vec3_to_rgba(blended));

                // Normal splat using RNM.
                if let Some(noff) = normal_offset {
                    let existing_n = sample_linear(targets.normal_map, uv);
                    let stamp_n = (noff.normalize() * 0.5 + Vec3::splat(0.5)).clamp(Vec3::ZERO, Vec3::ONE);
                    let blended_n = rnm_blend(existing_n, stamp_n);
                    targets.normal_map.set_pixel(tx, ty, vec3_to_rgba(blended_n));
                }

                // Roughness splat (single channel stored in R).
                let existing_r = sample_linear(targets.roughness_map, uv);
                let stamp_r = Vec3::splat(0.5 + alpha * 0.2); // slightly vary roughness
                let blended_r = existing_r.lerp(stamp_r, alpha);
                targets.roughness_map.set_pixel(tx, ty, vec3_to_rgba(blended_r));
            }
        }
    }
}

fn sample_linear(img: &Image, uv: Vec2) -> Vec3 {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let x = uv.x * w;
    let y = uv.y * h;
    let x0 = x.floor().clamp(0.0, w - 1.0) as u32;
    let y0 = y.floor().clamp(0.0, h - 1.0) as u32;
    rgba_to_vec3(img.pixel(x0, y0).unwrap_or([0, 0, 0, 255]))
}

fn rgba_to_vec3(px: [u8; 4]) -> Vec3 {
    Vec3::new(px[0] as f32 / 255.0, px[1] as f32 / 255.0, px[2] as f32 / 255.0)
}

fn vec3_to_rgba(v: Vec3) -> [u8; 4] {
    let c = v.clamp(Vec3::ZERO, Vec3::ONE) * 255.0;
    [c.x as u8, c.y as u8, c.z as u8, 255]
}

#[cfg(test)]
mod tests {
    use super::*;
    use toolkit_geometry::Mesh;

    #[test]
    fn rnm_flat_identity() {
        let flat = Vec3::new(0.5, 0.5, 1.0); // approx flat normal encoded
        let result = rnm_blend(flat, flat);
        assert!(result.z > 0.8, "RNM of flat should stay roughly flat");
    }

    #[test]
    fn splat_runs_on_plane() {
        let mesh = Mesh::plane(2.0, 2.0, 1);
        let mut color = Image::new(64, 64);
        let mut normal = Image::new(64, 64);
        let mut rough = Image::new(64, 64);

        splat_stroke_seamless(
            &mesh,
            SplatTarget {
                color_map: &mut color,
                normal_map: &mut normal,
                roughness_map: &mut rough,
            },
            Vec3::ZERO,
            Mat3::IDENTITY,
            Vec3::new(1.0, 0.0, 0.0),
            0.5,
            |d| 1.0 - d,
            Some(Vec3::Z),
        );

        // At least one pixel should be non-black.
        let mut found = false;
        for y in 0..64 {
            for x in 0..64 {
                if let Some([r, g, b, _]) = color.pixel(x, y) {
                    if r > 10 || g > 10 || b > 10 {
                        found = true;
                    }
                }
            }
        }
        assert!(found, "splat should paint at least one texel");
    }
}
