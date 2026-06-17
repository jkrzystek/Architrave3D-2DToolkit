//! Recompute per-vertex normals and tangents.

use glam::Vec3;
use toolkit_geometry::Mesh;

/// Recompute per-vertex normals as the area-weighted average of incident face
/// normals (the cross-product magnitude provides the area weighting).
pub fn recompute_normals(mesh: &mut Mesh) {
    let mut acc = vec![Vec3::ZERO; mesh.vertices.len()];
    for tri in mesh.indices.chunks_exact(3) {
        let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        let p0 = mesh.vertices[i0].position_vec3();
        let p1 = mesh.vertices[i1].position_vec3();
        let p2 = mesh.vertices[i2].position_vec3();
        let n = (p1 - p0).cross(p2 - p0);
        acc[i0] += n;
        acc[i1] += n;
        acc[i2] += n;
    }
    for (v, n) in mesh.vertices.iter_mut().zip(acc) {
        v.normal = n.normalize_or_zero().into();
    }
}

/// Recompute per-vertex tangents from UVs (Lengyel's method), storing the
/// handedness in `tangent.w`. Requires meaningful UV coordinates and normals.
pub fn recompute_tangents(mesh: &mut Mesh) {
    let n = mesh.vertices.len();
    let mut tan1 = vec![Vec3::ZERO; n];
    let mut tan2 = vec![Vec3::ZERO; n];

    for tri in mesh.indices.chunks_exact(3) {
        let (i0, i1, i2) = (tri[0] as usize, tri[1] as usize, tri[2] as usize);
        let p0 = mesh.vertices[i0].position_vec3();
        let p1 = mesh.vertices[i1].position_vec3();
        let p2 = mesh.vertices[i2].position_vec3();
        let w0 = mesh.vertices[i0].uv_vec2();
        let w1 = mesh.vertices[i1].uv_vec2();
        let w2 = mesh.vertices[i2].uv_vec2();

        let e1 = p1 - p0;
        let e2 = p2 - p0;
        let du1 = w1 - w0;
        let du2 = w2 - w0;
        let denom = du1.x * du2.y - du2.x * du1.y;
        if denom.abs() < 1e-12 {
            continue;
        }
        let r = 1.0 / denom;
        let sdir = (e1 * du2.y - e2 * du1.y) * r;
        let tdir = (e2 * du1.x - e1 * du2.x) * r;
        for &i in &[i0, i1, i2] {
            tan1[i] += sdir;
            tan2[i] += tdir;
        }
    }

    for (i, v) in mesh.vertices.iter_mut().enumerate() {
        let n = v.normal_vec3();
        let t = tan1[i];
        // Gram-Schmidt orthogonalize against the normal.
        let tangent = (t - n * n.dot(t)).normalize_or_zero();
        let handedness = if n.cross(t).dot(tan2[i]) < 0.0 {
            -1.0
        } else {
            1.0
        };
        v.tangent = [tangent.x, tangent.y, tangent.z, handedness];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn normals_face_outward_on_cube() {
        let mut cube = Mesh::cube(2.0);
        // Wipe normals, then recompute from geometry.
        for v in &mut cube.vertices {
            v.normal = [0.0; 3];
        }
        recompute_normals(&mut cube);
        for v in &cube.vertices {
            // On a centered cube, each corner normal points away from origin.
            assert!(v.normal_vec3().dot(v.position_vec3()) > 0.0);
        }
    }

    #[test]
    fn tangents_are_unit_and_orthogonal() {
        let mut plane = Mesh::plane(2.0, 2.0, 2);
        recompute_tangents(&mut plane);
        for v in &plane.vertices {
            let t = Vec3::new(v.tangent[0], v.tangent[1], v.tangent[2]);
            assert!((t.length() - 1.0).abs() < 1e-3, "tangent not unit: {t:?}");
            // Orthogonal to the (upward) normal.
            assert!(t.dot(v.normal_vec3()).abs() < 1e-3);
            assert!((v.tangent[3].abs() - 1.0).abs() < 1e-6);
        }
    }
}
