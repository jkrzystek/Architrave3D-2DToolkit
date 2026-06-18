//! A pose: per-joint local transforms, and the skinning matrices it produces.

use glam::Mat4;
use serde::{Deserialize, Serialize};
use toolkit_scene::Transform;

use crate::skeleton::Skeleton;

/// A set of local transforms, one per joint, replacing the bind pose. Animation
/// systems write into `local`; skinning reads it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pose {
    pub local: Vec<Transform>,
}

impl Pose {
    /// A pose equal to the skeleton's rest/bind pose.
    pub fn rest(skeleton: &Skeleton) -> Self {
        Self {
            local: skeleton.joints().iter().map(|j| j.local_bind).collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.local.len()
    }

    pub fn is_empty(&self) -> bool {
        self.local.is_empty()
    }

    /// Global (model-space) matrix for each joint under this pose. Composes
    /// posed local matrices up each joint's parent chain (order-independent).
    pub fn global_matrices(&self, skeleton: &Skeleton) -> Vec<Mat4> {
        let joints = skeleton.joints();
        let locals: Vec<Mat4> = self.local.iter().map(|t| t.to_matrix()).collect();
        (0..joints.len())
            .map(|i| {
                let mut m = locals[i];
                let mut parent = joints[i].parent;
                while let Some(p) = parent {
                    m = locals[p] * m;
                    parent = joints[p].parent;
                }
                m
            })
            .collect()
    }

    /// Skinning matrix palette: `global_pose[i] * inverse_bind[i]`. Multiplying
    /// a bind-pose model-space vertex by this moves it to the posed position.
    pub fn skinning_matrices(&self, skeleton: &Skeleton) -> Vec<Mat4> {
        let globals = self.global_matrices(skeleton);
        let inv = skeleton.inverse_bind_matrices();
        globals
            .iter()
            .zip(inv.iter())
            .map(|(g, ib)| *g * *ib)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton::{Joint, Skeleton};
    use glam::{Quat, Vec3};

    fn chain() -> Skeleton {
        Skeleton::new(vec![
            Joint::new("root", None, Transform::IDENTITY),
            Joint::new(
                "child",
                Some(0),
                Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)),
            ),
        ])
    }

    #[test]
    fn rest_pose_skinning_is_identity() {
        let skel = chain();
        let pose = Pose::rest(&skel);
        for m in pose.skinning_matrices(&skel) {
            // At rest, skinning matrices are identity (within float error).
            assert!((m - Mat4::IDENTITY).abs_diff_eq(Mat4::ZERO, 1e-5));
        }
    }

    #[test]
    fn rotating_root_moves_child() {
        let skel = chain();
        let mut pose = Pose::rest(&skel);
        // Rotate the root 90° about Z.
        pose.local[0] = Transform::from_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2));
        let mats = pose.skinning_matrices(&skel);
        // The child's bind position (0,2,0) should swing to about (-2,0,0).
        let moved = mats[1].transform_point3(Vec3::new(0.0, 2.0, 0.0));
        assert!((moved - Vec3::new(-2.0, 0.0, 0.0)).length() < 1e-4);
    }

    #[test]
    fn serde_roundtrip() {
        let skel = chain();
        let pose = Pose::rest(&skel);
        let json = serde_json::to_string(&pose).unwrap();
        let back: Pose = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 2);
    }
}
