//! Animate a [`Transform`] by sampling separate translation/rotation/scale
//! tracks, and apply the result to a scene node.

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use toolkit_scene::{NodeKey, Scene, Transform};

use crate::track::Track;

/// Three keyframe tracks that together animate a [`Transform`]. Any track may be
/// empty, in which case that component holds its identity value.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TransformAnimation {
    pub translation: Track<Vec3>,
    pub rotation: Track<Quat>,
    pub scale: Track<Vec3>,
}

impl TransformAnimation {
    pub fn new() -> Self {
        Self::default()
    }

    /// Total duration = the latest keyframe across all three tracks.
    pub fn duration(&self) -> f32 {
        self.translation
            .duration()
            .max(self.rotation.duration())
            .max(self.scale.duration())
    }

    /// Sample the full transform at `time`. Missing tracks fall back to the
    /// identity (zero translation, identity rotation, unit scale).
    pub fn sample(&self, time: f32) -> Transform {
        Transform {
            translation: self.translation.sample(time).unwrap_or(Vec3::ZERO),
            rotation: self.rotation.sample(time).unwrap_or(Quat::IDENTITY),
            scale: self.scale.sample(time).unwrap_or(Vec3::ONE),
        }
    }
}

/// Sample `anim` at `time` and write the result to a scene node's local
/// transform. Returns `false` if the node key is invalid.
pub fn apply_to_node(
    scene: &mut Scene,
    key: NodeKey,
    anim: &TransformAnimation,
    time: f32,
) -> bool {
    match scene.get_mut(key) {
        Some(node) => {
            node.transform = anim.sample(time);
            true
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toolkit_scene::NodeData;

    fn moving_anim() -> TransformAnimation {
        let mut anim = TransformAnimation::new();
        anim.translation.add(0.0, Vec3::ZERO).add(2.0, Vec3::new(10.0, 0.0, 0.0));
        anim.scale.add(0.0, Vec3::ONE).add(2.0, Vec3::splat(3.0));
        anim
    }

    #[test]
    fn samples_components() {
        let anim = moving_anim();
        let t = anim.sample(1.0);
        assert!((t.translation - Vec3::new(5.0, 0.0, 0.0)).length() < 1e-5);
        assert!((t.scale - Vec3::splat(2.0)).length() < 1e-5);
        // Rotation track empty -> identity.
        assert!(t.rotation.angle_between(Quat::IDENTITY) < 1e-5);
    }

    #[test]
    fn duration_is_max_of_tracks() {
        let mut anim = TransformAnimation::new();
        anim.translation.add(0.0, Vec3::ZERO).add(1.0, Vec3::X);
        anim.scale.add(0.0, Vec3::ONE).add(5.0, Vec3::splat(2.0));
        assert_eq!(anim.duration(), 5.0);
    }

    #[test]
    fn apply_to_node_sets_transform() {
        let mut scene = Scene::new();
        let node = scene.add_node("anim", Transform::IDENTITY, NodeData::Empty);
        let anim = moving_anim();
        assert!(apply_to_node(&mut scene, node, &anim, 2.0));
        let t = scene.get(node).unwrap().transform;
        assert!((t.translation - Vec3::new(10.0, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn apply_to_invalid_node_fails() {
        let mut scene = Scene::new();
        let node = scene.add_node("x", Transform::IDENTITY, NodeData::Empty);
        scene.remove(node);
        assert!(!apply_to_node(&mut scene, node, &moving_anim(), 0.0));
    }
}
