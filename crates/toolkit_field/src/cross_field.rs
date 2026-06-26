//! Odeco Cross-Field: smooth principal-curvature directions on a mesh.
//!
//! Computes a 4-direction cross-field (4-RoSy) that aligns with principal
//! curvature directions and is globally smooth across the surface. The field is
//! encoded using a 4th-power complex representation to eliminate the 90°
//! ambiguity, then smoothed iteratively.

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use toolkit_geometry::Mesh;

/// A smooth cross-field over a triangle mesh.
///
/// At every vertex it stores the principal curvature direction `v1` (direction
/// of maximum curvature) and the normal. The cross-field is a 4-RoSy field:
/// rotating `v1` by 90° yields an equivalent direction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OdecoCrossField {
    /// Per-vertex principal direction (max curvature).
    pub vertex_directions: Vec<Vec3>,
    /// Per-vertex normals.
    pub vertex_normals: Vec<Vec3>,
    /// Per-vertex curvature magnitude (|κ_max| + |κ_min|).
    pub vertex_curvature: Vec<f32>,
}

impl OdecoCrossField {
    /// Sample the principal direction at a barycentric point on `triangle_index`.
    pub fn sample_principal_directions(&self, mesh: &Mesh, triangle_index: u32, bary: (f32, f32, f32)) -> (Vec3, Vec3) {
        let base = triangle_index as usize * 3;
        let i0 = mesh.indices[base] as usize;
        let i1 = mesh.indices[base + 1] as usize;
        let i2 = mesh.indices[base + 2] as usize;
        let (w, u, v) = bary;

        let v1 = (w * self.vertex_directions[i0] + u * self.vertex_directions[i1] + v * self.vertex_directions[i2]).normalize();
        let n = (w * self.vertex_normals[i0] + u * self.vertex_normals[i1] + v * self.vertex_normals[i2]).normalize();
        (v1, n)
    }

    /// Sample the normal at a point on the mesh (barycentric coordinates).
    pub fn sample_normal(&self, mesh: &Mesh, triangle_index: u32, bary: (f32, f32, f32)) -> Vec3 {
        let base = triangle_index as usize * 3;
        let i0 = mesh.indices[base] as usize;
        let i1 = mesh.indices[base + 1] as usize;
        let i2 = mesh.indices[base + 2] as usize;
        let (w, u, v) = bary;
        (w * self.vertex_normals[i0] + u * self.vertex_normals[i1] + v * self.vertex_normals[i2]).normalize()
    }

    /// Sample curvature magnitude.
    pub fn sample_curvature(&self, mesh: &Mesh, triangle_index: u32, bary: (f32, f32, f32)) -> f32 {
        let base = triangle_index as usize * 3;
        let i0 = mesh.indices[base] as usize;
        let i1 = mesh.indices[base + 1] as usize;
        let i2 = mesh.indices[base + 2] as usize;
        let (w, u, v) = bary;
        w * self.vertex_curvature[i0] + u * self.vertex_curvature[i1] + v * self.vertex_curvature[i2]
    }
}

/// Compute a smooth Odeco cross-field from a mesh.
///
/// * `smooth_iterations` — number of smoothing passes (8–20 is typical).
pub fn compute_odeco_cross_field(mesh: &Mesh, smooth_iterations: usize) -> OdecoCrossField {
    let n = mesh.vertex_count();
    if n == 0 {
        return OdecoCrossField {
            vertex_directions: Vec::new(),
            vertex_normals: Vec::new(),
            vertex_curvature: Vec::new(),
        };
    }

    // 1. Per-triangle curvature directions.
    let tri_dirs: Vec<(Vec3, Vec3, f32)> = (0..mesh.triangle_count())
        .map(|tri_idx| triangle_curvature(mesh, tri_idx))
        .collect();

    // 2. Build vertex adjacency (vertex -> list of adjacent triangles).
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for tri_idx in 0..mesh.triangle_count() {
        let base = tri_idx * 3;
        for k in 0..3 {
            let vi = mesh.indices[base + k] as usize;
            adj[vi].push(tri_idx);
        }
    }

    // 3. Initialize per-vertex fields by area-weighted averaging.
    let mut dirs: Vec<Vec3> = vec![Vec3::ZERO; n];
    let mut norms: Vec<Vec3> = vec![Vec3::ZERO; n];
    let mut curvs: Vec<f32> = vec![0.0; n];
    let mut weights: Vec<f32> = vec![0.0; n];

    for tri_idx in 0..mesh.triangle_count() {
        let base = tri_idx * 3;
        let i0 = mesh.indices[base] as usize;
        let i1 = mesh.indices[base + 1] as usize;
        let i2 = mesh.indices[base + 2] as usize;
        let p0 = mesh.vertices[i0].position_vec3();
        let p1 = mesh.vertices[i1].position_vec3();
    let p2 = mesh.vertices[i2].position_vec3();
        let area = (p1 - p0).cross(p2 - p0).length() * 0.5;

        let (v1, n, c) = tri_dirs[tri_idx];
        for &vi in &[i0, i1, i2] {
            dirs[vi] += area * v1;
            norms[vi] += area * n;
            curvs[vi] += area * c;
            weights[vi] += area;
        }
    }

    for i in 0..n {
        if weights[i] > 0.0 {
            dirs[i] = (dirs[i] / weights[i]).normalize_or_zero();
            norms[i] = (norms[i] / weights[i]).normalize_or_zero();
            curvs[i] /= weights[i];
        }
    }

    // 4. Smooth the cross-field while aligning neighbors (respect 4-RoSy).
    for _ in 0..smooth_iterations {
        let mut new_dirs = dirs.clone();
        for vi in 0..n {
            if adj[vi].is_empty() {
                continue;
            }
            // Collect neighbor vertex indices.
            let mut neigh = Vec::new();
            for &tri_idx in &adj[vi] {
                let base = tri_idx * 3;
                for k in 0..3 {
                    let vj = mesh.indices[base + k] as usize;
                    if vj != vi {
                        neigh.push(vj);
                    }
                }
            }
            if neigh.is_empty() {
                continue;
            }

            let mut sum = Vec3::ZERO;
            for &vj in &neigh {
                let d = align_4rosy(dirs[vj], dirs[vi], norms[vi]);
                sum += d;
            }
            let avg = sum / neigh.len() as f32;
            // Project onto tangent plane to keep the direction perpendicular to normal.
            let avg_tangent = (avg - norms[vi] * avg.dot(norms[vi])).normalize_or_zero();
            new_dirs[vi] = align_4rosy(avg_tangent, dirs[vi], norms[vi]);
        }
        dirs = new_dirs;
    }

    OdecoCrossField {
        vertex_directions: dirs,
        vertex_normals: norms,
        vertex_curvature: curvs,
    }
}

// ---------------------------------------------------------------------------
// Triangle-level curvature
// ---------------------------------------------------------------------------

/// Compute principal curvature direction, normal, and magnitude for one triangle.
///
/// Uses the discrete shape operator approach: fit a quadratic patch to the
/// one-ring and extract eigenvectors. For robustness we use a simplified
/// version based on the covariance of edge vectors projected onto the tangent
/// plane.
fn triangle_curvature(mesh: &Mesh, tri_idx: usize) -> (Vec3, Vec3, f32) {
    let base = tri_idx * 3;
    let i0 = mesh.indices[base] as usize;
    let i1 = mesh.indices[base + 1] as usize;
    let i2 = mesh.indices[base + 2] as usize;
    let p0 = mesh.vertices[i0].position_vec3();
    let p1 = mesh.vertices[i1].position_vec3();
    let _p2 = mesh.vertices[i2].position_vec3();
    let n0 = mesh.vertices[i0].normal_vec3();
    let n1 = mesh.vertices[i1].normal_vec3();
    let n2 = mesh.vertices[i2].normal_vec3();

    let normal = ((n0 + n1 + n2) / 3.0).normalize_or_zero();
    if normal.length_squared() < 1e-6 {
        // Degenerate — return arbitrary tangent frame.
        let e1 = (p1 - p0).normalize_or_zero();
        let _e2 = normal.cross(e1).normalize_or_zero();
        return (e1, Vec3::Y, 0.0);
    }

    // Build a local tangent frame.
    let e1 = (p1 - p0).normalize_or_zero();
    let e2 = normal.cross(e1).normalize_or_zero();
    let e1 = e2.cross(normal).normalize_or_zero(); // Re-orthonormalize

    // Project edges onto tangent plane and estimate 2x2 curvature tensor.
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
        // dn ≈ -S * dp  =>  dn·e1 ≈ -(sxx*du + sxy*dv), dn·e2 ≈ -(sxy*du + syy*dv)
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

    // Eigen-decomposition of the 2x2 symmetric matrix [[sxx, sxy], [sxy, syy]].
    let trace = sxx + syy;
    let det = sxx * syy - sxy * sxy;
    let disc = (trace * trace * 0.25 - det).max(0.0).sqrt();
    let lambda1 = trace * 0.5 + disc;
    let lambda2 = trace * 0.5 - disc;

    let v_local = if sxy.abs() < 1e-8 {
        Vec2::X
    } else {
        let dx = lambda1 - syy;
        let dy = sxy;
        let len = (dx * dx + dy * dy).sqrt();
        if len > 1e-8 {
            Vec2::new(dx / len, dy / len)
        } else {
            Vec2::X
        }
    };

    let v1 = (v_local.x * e1 + v_local.y * e2).normalize_or_zero();
    let curvature_mag = lambda1.abs() + lambda2.abs();

    (v1, normal, curvature_mag)
}

// ---------------------------------------------------------------------------
// 4-RoSy alignment
// ---------------------------------------------------------------------------

/// Align `candidate` to `reference` respecting 4-fold rotational symmetry.
///
/// Both vectors are assumed to lie in the plane perpendicular to `normal`.
/// Returns the rotation of `candidate` by a multiple of 90° that is closest
/// to `reference`.
fn align_4rosy(candidate: Vec3, reference: Vec3, normal: Vec3) -> Vec3 {
    let cos0 = candidate.dot(reference);
    let perp = normal.cross(candidate);
    let cos1 = perp.dot(reference); // rotated +90°

    // Four candidates: candidate, perp, -candidate, -perp
    let vals = [cos0, cos1, -cos0, -cos1];
    let mut best = 0;
    let mut best_v = cos0;
    for (i, &v) in vals.iter().enumerate() {
        if v > best_v {
            best = i;
            best_v = v;
        }
    }

    match best {
        0 => candidate,
        1 => perp,
        2 => -candidate,
        _ => -perp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_field_on_plane() {
        let plane = Mesh::plane(2.0, 2.0, 2);
        let field = compute_odeco_cross_field(&plane, 10);
        assert_eq!(field.vertex_directions.len(), plane.vertex_count());
        assert_eq!(field.vertex_normals.len(), plane.vertex_count());
        // Plane normals should point roughly +Y.
        for n in &field.vertex_normals {
            assert!(n.y > 0.5, "plane normal should point up");
        }
    }

    #[test]
    fn cross_field_on_sphere() {
        let sphere = Mesh::uv_sphere(1.0, 16, 8);
        let field = compute_odeco_cross_field(&sphere, 10);
        assert_eq!(field.vertex_directions.len(), sphere.vertex_count());
        // Directions should be tangent (perpendicular to normal).
        for i in 0..sphere.vertex_count() {
            let d = field.vertex_directions[i];
            let n = field.vertex_normals[i];
            let pos = sphere.vertices[i].position_vec3();
            // Skip poles where curvature estimation is degenerate.
            if (pos.x * pos.x + pos.z * pos.z).sqrt() < 0.2 {
                continue;
            }
            if d.length_squared() < 1e-6 {
                continue;
            }
            let dot = d.dot(n).abs();
            assert!(dot < 0.25, "direction should be tangent, got dot={}", dot);
        }
    }

    #[test]
    fn align_4rosy_picks_closest() {
        let n = Vec3::Y;
        let ref_dir = Vec3::X;
        let c = Vec3::Z; // perpendicular to ref_dir in XY plane under Y normal
        let aligned = align_4rosy(c, ref_dir, n);
        // Rotating Z by +90° around Y gives X, which is closest to ref_dir=X.
        assert!((aligned - Vec3::X).length() < 1e-4 || (aligned + Vec3::X).length() < 1e-4);
    }
}
