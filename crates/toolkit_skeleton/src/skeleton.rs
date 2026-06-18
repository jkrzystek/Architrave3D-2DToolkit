//! The joint hierarchy and its bind pose.

use glam::Mat4;
use serde::{Deserialize, Serialize};
use toolkit_scene::Transform;

/// One joint (bone) in a skeleton. The hierarchy is encoded by `parent`
/// indices into the owning [`Skeleton`]'s joint list; a root has `None`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Joint {
    pub name: String,
    /// Index of the parent joint, or `None` for a root.
    pub parent: Option<usize>,
    /// Rest-pose local transform relative to the parent.
    pub local_bind: Transform,
}

impl Joint {
    pub fn new(name: impl Into<String>, parent: Option<usize>, local_bind: Transform) -> Self {
        Self {
            name: name.into(),
            parent,
            local_bind,
        }
    }
}

/// A skeleton: an ordered list of joints plus each joint's cached
/// inverse-bind matrix (world→joint-local at rest), which skinning needs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Skeleton {
    joints: Vec<Joint>,
    /// `inverse_bind[i]` maps a model-space point into joint `i`'s local space
    /// at the bind pose. Cached so skinning does not recompute it per frame.
    inverse_bind: Vec<Mat4>,
}

impl Skeleton {
    /// Build a skeleton from joints, computing each global bind matrix (by
    /// walking parent chains, so joint order is unconstrained) and its inverse.
    pub fn new(joints: Vec<Joint>) -> Self {
        let inverse_bind = (0..joints.len())
            .map(|i| global_matrix(&joints, i, |j| j.local_bind.to_matrix()).inverse())
            .collect();
        Self {
            joints,
            inverse_bind,
        }
    }

    pub fn joints(&self) -> &[Joint] {
        &self.joints
    }

    pub fn len(&self) -> usize {
        self.joints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.joints.is_empty()
    }

    pub fn inverse_bind_matrices(&self) -> &[Mat4] {
        &self.inverse_bind
    }

    /// Global (model-space) bind matrix of joint `i`.
    pub fn global_bind_matrix(&self, i: usize) -> Mat4 {
        global_matrix(&self.joints, i, |j| j.local_bind.to_matrix())
    }
}

/// Accumulate a joint's global matrix by composing local matrices up the parent
/// chain: `M_global = M_parent_global * M_local`. `local_of` extracts the local
/// matrix to use (bind pose, or an animated pose).
pub(crate) fn global_matrix(
    joints: &[Joint],
    i: usize,
    local_of: impl Fn(&Joint) -> Mat4,
) -> Mat4 {
    let mut m = local_of(&joints[i]);
    let mut parent = joints[i].parent;
    while let Some(p) = parent {
        m = local_of(&joints[p]) * m;
        parent = joints[p].parent;
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    /// Root at origin, child offset +Y by 2.
    pub(crate) fn two_joint_chain() -> Skeleton {
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
    fn global_bind_accumulates_up_chain() {
        let skel = two_joint_chain();
        let child_world = skel.global_bind_matrix(1);
        let origin = child_world.transform_point3(Vec3::ZERO);
        assert!((origin - Vec3::new(0.0, 2.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn inverse_bind_maps_world_to_local() {
        let skel = two_joint_chain();
        let inv = skel.inverse_bind_matrices()[1];
        // The child's world bind origin maps back to its own local origin.
        let local = inv.transform_point3(Vec3::new(0.0, 2.0, 0.0));
        assert!(local.length() < 1e-5);
    }

    #[test]
    fn order_independent_parent_after_child() {
        // Declare child (index 0) before root (index 1) to prove order doesn't
        // matter for the global computation.
        let skel = Skeleton::new(vec![
            Joint::new(
                "child",
                Some(1),
                Transform::from_translation(Vec3::new(1.0, 0.0, 0.0)),
            ),
            Joint::new(
                "root",
                None,
                Transform::from_translation(Vec3::new(0.0, 5.0, 0.0)),
            ),
        ]);
        let child_world = skel.global_bind_matrix(0).transform_point3(Vec3::ZERO);
        assert!((child_world - Vec3::new(1.0, 5.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn serde_roundtrip() {
        let skel = two_joint_chain();
        let json = serde_json::to_string(&skel).unwrap();
        let back: Skeleton = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 2);
    }
}
