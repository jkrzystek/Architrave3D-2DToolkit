//! Least Squares Conformal Maps (Lévy et al. 2002).
//!
//! LSCM flattens a 3D surface patch into the plane while minimising angular
//! distortion. Each triangle contributes a complex linear equation enforcing
//! the Cauchy-Riemann (conformality) condition; two pinned vertices fix the
//! remaining similarity gauge. The resulting sparse least-squares system is
//! solved with [`crate::solver`].

use glam::{Vec2, Vec3};

use crate::solver::{solve_least_squares, SparseMatrix};

/// Result of unwrapping: one UV per input vertex, normalised into the unit
/// square (aspect ratio preserved).
#[derive(Clone, Debug)]
pub struct UnwrapResult {
    pub uvs: Vec<Vec2>,
}

/// Unwrap a triangle patch with LSCM.
///
/// * `positions` — 3D vertex positions.
/// * `triangles` — triangle vertex indices (must reference `positions`).
///
/// Returns per-vertex UVs normalised to `[0,1]²`. The patch should be
/// disk-topology (cut open) for a meaningful result; closed surfaces need a
/// seam first.
pub fn unwrap_lscm(positions: &[Vec3], triangles: &[[usize; 3]]) -> UnwrapResult {
    let n = positions.len();
    if n < 3 || triangles.is_empty() {
        return UnwrapResult {
            uvs: vec![Vec2::ZERO; n],
        };
    }

    // -- Choose two pinned vertices: vertex 0 and the one farthest from it. --
    let p0 = positions[0];
    let (pin_b, _) = positions
        .iter()
        .enumerate()
        .map(|(i, p)| (i, p.distance_squared(p0)))
        .fold((0usize, 0.0f32), |acc, (i, d)| if d > acc.1 { (i, d) } else { acc });
    let pin_a = 0;
    let pin_b = if pin_b == pin_a { n - 1 } else { pin_b };

    // Pinned UV values (gauge fix). Normalisation happens afterwards.
    let pinned = [(pin_a, Vec2::new(0.0, 0.0)), (pin_b, Vec2::new(1.0, 0.0))];
    let is_pinned = |v: usize| pinned.iter().any(|(p, _)| *p == v);

    // Map each free vertex's u and v to a column in the unknown vector.
    // Layout: [u_free..., v_free...].
    let mut free_index = vec![usize::MAX; n];
    let mut n_free = 0;
    for v in 0..n {
        if !is_pinned(v) {
            free_index[v] = n_free;
            n_free += 1;
        }
    }
    let u_col = |v: usize| free_index[v];
    let v_col = |v: usize| n_free + free_index[v];
    let n_cols = 2 * n_free;
    let n_rows = 2 * triangles.len();

    let mut a = SparseMatrix::new(n_rows, n_cols);
    let mut b = vec![0.0f32; n_rows];

    let pin_uv = |v: usize| pinned.iter().find(|(p, _)| *p == v).map(|(_, uv)| *uv);

    for (t, tri) in triangles.iter().enumerate() {
        let [i0, i1, i2] = *tri;
        let (q0, q1, q2) = (positions[i0], positions[i1], positions[i2]);

        // Local orthonormal triangle frame.
        let e1 = q1 - q0;
        let len1 = e1.length();
        if len1 < 1e-12 {
            continue;
        }
        let x_axis = e1 / len1;
        let cross = (q1 - q0).cross(q2 - q0);
        let area2 = cross.length(); // 2 * area
        if area2 < 1e-12 {
            continue;
        }
        let y_axis = cross.normalize().cross(x_axis);

        // Local 2D coords as complex numbers W = (re, im).
        let w0 = Vec2::new(0.0, 0.0);
        let w1 = Vec2::new(len1, 0.0);
        let d = q2 - q0;
        let w2 = Vec2::new(d.dot(x_axis), d.dot(y_axis));

        let scale = 1.0 / area2.sqrt();
        // c_j (complex) coefficients of the conformality equation.
        let c = [
            (w2 - w1) * scale, // c0
            (w0 - w2) * scale, // c1
            (w1 - w0) * scale, // c2
        ];
        let verts = [i0, i1, i2];

        let row_re = 2 * t;
        let row_im = 2 * t + 1;

        for k in 0..3 {
            let v = verts[k];
            let (ca, cb) = (c[k].x, c[k].y);
            // Real row: a*u - b*v ; Imag row: b*u + a*v
            if is_pinned(v) {
                let uv = pin_uv(v).unwrap();
                b[row_re] -= ca * uv.x - cb * uv.y;
                b[row_im] -= cb * uv.x + ca * uv.y;
            } else {
                a.push(row_re, u_col(v), ca);
                a.push(row_re, v_col(v), -cb);
                a.push(row_im, u_col(v), cb);
                a.push(row_im, v_col(v), ca);
            }
        }
    }

    let solution = solve_least_squares(&a, &b, (n_cols * 2).max(200), 1e-8);

    let mut uvs = vec![Vec2::ZERO; n];
    for v in 0..n {
        if let Some(uv) = pin_uv(v) {
            uvs[v] = uv;
        } else {
            uvs[v] = Vec2::new(solution[u_col(v)], solution[v_col(v)]);
        }
    }

    normalize_to_unit_square(&mut uvs);
    UnwrapResult { uvs }
}

/// Translate/scale UVs to fit the unit square, preserving aspect ratio.
pub fn normalize_to_unit_square(uvs: &mut [Vec2]) {
    if uvs.is_empty() {
        return;
    }
    let mut min = Vec2::splat(f32::INFINITY);
    let mut max = Vec2::splat(f32::NEG_INFINITY);
    for uv in uvs.iter() {
        min = min.min(*uv);
        max = max.max(*uv);
    }
    let extent = (max - min).max_element().max(1e-12);
    for uv in uvs.iter_mut() {
        *uv = (*uv - min) / extent;
    }
}

/// Mean angular (conformal) distortion across triangles: 0 means perfectly
/// angle-preserving. Useful to validate an unwrap.
pub fn conformal_distortion(positions: &[Vec3], uvs: &[Vec2], triangles: &[[usize; 3]]) -> f32 {
    let mut total = 0.0;
    let mut count = 0;
    for tri in triangles {
        let [i0, i1, i2] = *tri;
        // Corner angles in 3D vs in UV; compare.
        let angles_3d = corner_angles3(positions[i0], positions[i1], positions[i2]);
        let angles_uv = corner_angles2(uvs[i0], uvs[i1], uvs[i2]);
        for k in 0..3 {
            total += (angles_3d[k] - angles_uv[k]).abs();
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        total / count as f32
    }
}

fn corner_angles3(a: Vec3, b: Vec3, c: Vec3) -> [f32; 3] {
    let ab = (b - a).normalize_or_zero();
    let ac = (c - a).normalize_or_zero();
    let ba = -ab;
    let bc = (c - b).normalize_or_zero();
    let ca = -ac;
    let cb = -bc;
    [
        ab.dot(ac).clamp(-1.0, 1.0).acos(),
        ba.dot(bc).clamp(-1.0, 1.0).acos(),
        ca.dot(cb).clamp(-1.0, 1.0).acos(),
    ]
}

fn corner_angles2(a: Vec2, b: Vec2, c: Vec2) -> [f32; 3] {
    let ab = (b - a).normalize_or_zero();
    let ac = (c - a).normalize_or_zero();
    let ba = -ab;
    let bc = (c - b).normalize_or_zero();
    let ca = -ac;
    let cb = -bc;
    [
        ab.dot(ac).clamp(-1.0, 1.0).acos(),
        ba.dot(bc).clamp(-1.0, 1.0).acos(),
        ca.dot(cb).clamp(-1.0, 1.0).acos(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat (planar) patch in 3D — LSCM should reproduce it conformally
    /// (zero angular distortion up to numerical error).
    fn planar_grid() -> (Vec<Vec3>, Vec<[usize; 3]>) {
        let mut positions = Vec::new();
        for y in 0..3 {
            for x in 0..3 {
                positions.push(Vec3::new(x as f32, y as f32 * 0.5, 0.0));
            }
        }
        let mut tris = Vec::new();
        for y in 0..2 {
            for x in 0..2 {
                let tl = y * 3 + x;
                let tr = tl + 1;
                let bl = tl + 3;
                let br = bl + 1;
                tris.push([tl, bl, tr]);
                tris.push([tr, bl, br]);
            }
        }
        (positions, tris)
    }

    #[test]
    fn unwrap_produces_uv_per_vertex() {
        let (pos, tris) = planar_grid();
        let result = unwrap_lscm(&pos, &tris);
        assert_eq!(result.uvs.len(), pos.len());
    }

    #[test]
    fn unwrap_fits_unit_square() {
        let (pos, tris) = planar_grid();
        let result = unwrap_lscm(&pos, &tris);
        for uv in &result.uvs {
            assert!(uv.x >= -1e-4 && uv.x <= 1.0 + 1e-4);
            assert!(uv.y >= -1e-4 && uv.y <= 1.0 + 1e-4);
        }
    }

    #[test]
    fn planar_patch_is_near_conformal() {
        let (pos, tris) = planar_grid();
        let result = unwrap_lscm(&pos, &tris);
        let distortion = conformal_distortion(&pos, &result.uvs, &tris);
        // A flat patch should flatten with almost no angular distortion.
        assert!(distortion < 0.02, "distortion too high: {distortion}");
    }

    #[test]
    fn unwrap_is_non_degenerate() {
        let (pos, tris) = planar_grid();
        let result = unwrap_lscm(&pos, &tris);
        // Total UV area must be clearly positive (no full collapse).
        let mut area = 0.0;
        for tri in &tris {
            let a = result.uvs[tri[0]];
            let b = result.uvs[tri[1]];
            let c = result.uvs[tri[2]];
            area += ((b - a).perp_dot(c - a)).abs() * 0.5;
        }
        assert!(area > 0.05, "uv area too small: {area}");
    }

    #[test]
    fn curved_patch_unwraps() {
        // A gently curved strip (half cylinder) — should still flatten.
        let mut pos = Vec::new();
        let n = 6;
        for i in 0..n {
            let a = std::f32::consts::PI * i as f32 / (n - 1) as f32;
            pos.push(Vec3::new(a.cos(), 0.0, a.sin()));
            pos.push(Vec3::new(a.cos(), 1.0, a.sin()));
        }
        let mut tris = Vec::new();
        for i in 0..n - 1 {
            let b = i * 2;
            tris.push([b, b + 1, b + 2]);
            tris.push([b + 2, b + 1, b + 3]);
        }
        let result = unwrap_lscm(&pos, &tris);
        let distortion = conformal_distortion(&pos, &result.uvs, &tris);
        assert!(distortion < 0.05, "distortion: {distortion}");
    }
}
