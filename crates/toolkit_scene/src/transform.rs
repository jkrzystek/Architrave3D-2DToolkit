use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

/// A local affine transform expressed as translation, rotation, and scale (TRS).
///
/// Storing TRS separately (rather than a raw matrix) keeps the components
/// independently editable — which is what gizmos, animation, and property
/// panels need — while still composing to a standard column-major matrix.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform {
    pub const IDENTITY: Self = Self {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    pub fn from_translation(translation: Vec3) -> Self {
        Self {
            translation,
            ..Self::IDENTITY
        }
    }

    pub fn from_rotation(rotation: Quat) -> Self {
        Self {
            rotation,
            ..Self::IDENTITY
        }
    }

    pub fn from_scale(scale: Vec3) -> Self {
        Self {
            scale,
            ..Self::IDENTITY
        }
    }

    /// Compose this transform into a 4x4 matrix.
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }

    /// Decompose a matrix back into a TRS transform.
    ///
    /// Note: a matrix with shear cannot be represented exactly; the closest
    /// scale/rotation is returned (glam's `to_scale_rotation_translation`).
    pub fn from_matrix(matrix: &Mat4) -> Self {
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// Combine `self` (applied second) with `child` (applied first):
    /// `result = self * child`. Used to fold a parent's world transform into a
    /// child's local transform.
    pub fn mul_transform(&self, child: &Transform) -> Transform {
        Transform::from_matrix(&(self.to_matrix() * child.to_matrix()))
    }

    /// Transform a point from local space to the space this transform maps into.
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        self.rotation * (point * self.scale) + self.translation
    }

    /// Rotate a direction vector (ignores translation and scale sign issues).
    pub fn transform_vector(&self, vector: Vec3) -> Vec3 {
        self.rotation * (vector * self.scale)
    }

    /// The local right (+X), up (+Y), and forward (-Z) basis vectors.
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }
    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::NEG_Z
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_roundtrips_through_matrix() {
        let t = Transform::IDENTITY;
        let back = Transform::from_matrix(&t.to_matrix());
        assert!((back.translation - t.translation).length() < 1e-6);
        assert!((back.scale - t.scale).length() < 1e-6);
    }

    #[test]
    fn translation_applies_to_point() {
        let t = Transform::from_translation(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(t.transform_point(Vec3::ZERO), Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn scale_then_translate_order() {
        let t = Transform {
            translation: Vec3::new(10.0, 0.0, 0.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::splat(2.0),
        };
        // Point (1,0,0): scaled to (2,0,0), then translated to (12,0,0).
        assert_eq!(t.transform_point(Vec3::X), Vec3::new(12.0, 0.0, 0.0));
    }

    #[test]
    fn rotation_basis_vectors() {
        let t = Transform::from_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
        // Rotating +X by 90deg about Y gives -Z.
        assert!((t.right() - Vec3::NEG_Z).length() < 1e-5);
    }

    #[test]
    fn mul_transform_composes() {
        let parent = Transform::from_translation(Vec3::new(5.0, 0.0, 0.0));
        let child = Transform::from_translation(Vec3::new(0.0, 3.0, 0.0));
        let world = parent.mul_transform(&child);
        assert!((world.translation - Vec3::new(5.0, 3.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn decompose_recovers_components() {
        let original = Transform {
            translation: Vec3::new(1.0, -2.0, 3.0),
            rotation: Quat::from_rotation_z(0.5),
            scale: Vec3::splat(1.5),
        };
        let back = Transform::from_matrix(&original.to_matrix());
        assert!((back.translation - original.translation).length() < 1e-4);
        assert!((back.scale - original.scale).length() < 1e-4);
        assert!(back.rotation.dot(original.rotation).abs() > 0.999);
    }
}
