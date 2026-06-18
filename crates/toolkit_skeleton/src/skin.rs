//! Per-vertex skin binding and linear-blend skinning.

use glam::Mat4;
use serde::{Deserialize, Serialize};
use toolkit_geometry::Mesh;

/// Up to four joint influences for one vertex. `joints[k]` indexes into the
/// skeleton; `weights[k]` is its influence. Weights should sum to 1.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SkinWeights {
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}

impl SkinWeights {
    pub fn new(joints: [u16; 4], weights: [f32; 4]) -> Self {
        Self { joints, weights }
    }

    /// A rigid binding: the vertex follows a single joint.
    pub fn rigid(joint: u16) -> Self {
        Self {
            joints: [joint, 0, 0, 0],
            weights: [1.0, 0.0, 0.0, 0.0],
        }
    }

    /// Return a copy with weights rescaled to sum to 1 (falls back to rigid on
    /// the first joint if all weights are zero).
    pub fn normalized(self) -> Self {
        let sum: f32 = self.weights.iter().sum();
        if sum <= f32::EPSILON {
            return Self::rigid(self.joints[0]);
        }
        let mut w = self.weights;
        for x in &mut w {
            *x /= sum;
        }
        Self {
            joints: self.joints,
            weights: w,
        }
    }
}

impl Default for SkinWeights {
    fn default() -> Self {
        Self::rigid(0)
    }
}

/// Skin binding for a whole mesh: one [`SkinWeights`] per vertex, in vertex
/// order.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Skin {
    pub weights: Vec<SkinWeights>,
}

impl Skin {
    pub fn new(weights: Vec<SkinWeights>) -> Self {
        Self { weights }
    }

    /// Whether this skin has exactly one binding per vertex of `mesh`.
    pub fn matches(&self, mesh: &Mesh) -> bool {
        self.weights.len() == mesh.vertex_count()
    }
}

/// Apply linear-blend skinning, returning a deformed copy of `mesh`.
///
/// For each vertex the joint matrices are blended by weight and applied to the
/// position and normal. `palette` is a skinning-matrix list (see
/// [`crate::Pose::skinning_matrices`]). Vertices whose skin index is out of
/// range, or beyond the skin's length, are left unchanged.
pub fn apply_skin(mesh: &Mesh, skin: &Skin, palette: &[Mat4]) -> Mesh {
    let mut out = mesh.clone();
    for (i, vertex) in out.vertices.iter_mut().enumerate() {
        let Some(sw) = skin.weights.get(i) else {
            continue;
        };
        let sw = sw.normalized();

        // Blend the influencing matrices weighted by skin weight.
        let mut blended = Mat4::ZERO;
        let mut any = false;
        for k in 0..4 {
            let w = sw.weights[k];
            if w == 0.0 {
                continue;
            }
            if let Some(m) = palette.get(sw.joints[k] as usize) {
                blended += *m * w;
                any = true;
            }
        }
        if !any {
            continue;
        }

        let pos = blended.transform_point3(vertex.position_vec3());
        let nrm = blended.transform_vector3(vertex.normal_vec3()).normalize_or_zero();
        vertex.position = pos.into();
        vertex.normal = nrm.into();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pose::Pose;
    use crate::skeleton::{Joint, Skeleton};
    use glam::{Quat, Vec3};
    use toolkit_geometry::Vertex;
    use toolkit_scene::Transform;

    #[test]
    fn weights_normalize() {
        let w = SkinWeights::new([0, 1, 0, 0], [3.0, 1.0, 0.0, 0.0]).normalized();
        assert!((w.weights[0] - 0.75).abs() < 1e-6);
        assert!((w.weights[1] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn zero_weights_fall_back_to_rigid() {
        let w = SkinWeights::new([5, 0, 0, 0], [0.0; 4]).normalized();
        assert_eq!(w.joints[0], 5);
        assert_eq!(w.weights[0], 1.0);
    }

    #[test]
    fn rest_pose_leaves_mesh_unchanged() {
        let skel = Skeleton::new(vec![Joint::new("root", None, Transform::IDENTITY)]);
        let pose = Pose::rest(&skel);
        let mesh = Mesh::with_vertices(
            "m",
            vec![Vertex::position_only(Vec3::new(1.0, 2.0, 3.0))],
            vec![],
        );
        let skin = Skin::new(vec![SkinWeights::rigid(0)]);
        let deformed = apply_skin(&mesh, &skin, &pose.skinning_matrices(&skel));
        assert!((deformed.vertices[0].position_vec3() - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-5);
    }

    #[test]
    fn rigid_vertex_follows_its_joint() {
        // Single joint rotated 90° about Z; a rigidly-bound vertex rotates with it.
        let skel = Skeleton::new(vec![Joint::new("root", None, Transform::IDENTITY)]);
        let mut pose = Pose::rest(&skel);
        pose.local[0] = Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
        let mesh = Mesh::with_vertices(
            "m",
            vec![Vertex::new(Vec3::X, Vec3::X, glam::Vec2::ZERO)],
            vec![],
        );
        let skin = Skin::new(vec![SkinWeights::rigid(0)]);
        let deformed = apply_skin(&mesh, &skin, &pose.skinning_matrices(&skel));
        // +X rotates to +Y.
        assert!((deformed.vertices[0].position_vec3() - Vec3::Y).length() < 1e-4);
        assert!((deformed.vertices[0].normal_vec3() - Vec3::Y).length() < 1e-4);
    }

    #[test]
    fn blend_between_two_joints_lands_midway() {
        // Two roots: joint 0 stays, joint 1 translated +X by 4. A 50/50 vertex
        // at origin should move halfway, to +X by 2.
        let skel = Skeleton::new(vec![
            Joint::new("a", None, Transform::IDENTITY),
            Joint::new("b", None, Transform::IDENTITY),
        ]);
        let mut pose = Pose::rest(&skel);
        pose.local[1] = Transform::from_translation(Vec3::new(4.0, 0.0, 0.0));
        let mesh = Mesh::with_vertices("m", vec![Vertex::position_only(Vec3::ZERO)], vec![]);
        let skin = Skin::new(vec![SkinWeights::new([0, 1, 0, 0], [0.5, 0.5, 0.0, 0.0])]);
        let deformed = apply_skin(&mesh, &skin, &pose.skinning_matrices(&skel));
        assert!((deformed.vertices[0].position_vec3() - Vec3::new(2.0, 0.0, 0.0)).length() < 1e-4);
    }

    #[test]
    fn skin_matches_vertex_count() {
        let mesh = Mesh::with_vertices("m", vec![Vertex::position_only(Vec3::ZERO)], vec![]);
        assert!(Skin::new(vec![SkinWeights::rigid(0)]).matches(&mesh));
        assert!(!Skin::new(vec![]).matches(&mesh));
    }
}
