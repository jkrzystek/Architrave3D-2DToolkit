use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};

/// GPU-ready vertex with position, normal, UV, and tangent attributes.
///
/// Fields are stored as raw float arrays so the struct can derive [`Pod`] and
/// [`Zeroable`] without requiring glam's `bytemuck` feature.  Convenience
/// accessors convert to/from `glam` types.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub tangent: [f32; 4],
}

impl Vertex {
    /// Create a vertex with the given position, normal, and UV coordinates.
    /// Tangent defaults to `(1, 0, 0, 1)`.
    pub fn new(position: Vec3, normal: Vec3, uv: Vec2) -> Self {
        Self {
            position: position.into(),
            normal: normal.into(),
            uv: uv.into(),
            tangent: [1.0, 0.0, 0.0, 1.0],
        }
    }

    /// Create a vertex with only a position; normal, UV, and tangent use defaults.
    pub fn position_only(pos: Vec3) -> Self {
        Self {
            position: pos.into(),
            normal: [0.0; 3],
            uv: [0.0; 2],
            tangent: [1.0, 0.0, 0.0, 1.0],
        }
    }

    // -- Glam accessors ------------------------------------------------------

    #[inline]
    pub fn position_vec3(&self) -> Vec3 {
        Vec3::from(self.position)
    }

    #[inline]
    pub fn normal_vec3(&self) -> Vec3 {
        Vec3::from(self.normal)
    }

    #[inline]
    pub fn uv_vec2(&self) -> Vec2 {
        Vec2::from(self.uv)
    }

    #[inline]
    pub fn tangent_vec4(&self) -> Vec4 {
        Vec4::from(self.tangent)
    }
}

/// Describes the format of a single vertex attribute for GPU binding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VertexAttribute {
    /// Byte offset of this attribute within the vertex.
    pub offset: u32,
    /// Format name (e.g. `"float32x3"`, `"float32x2"`, `"float32x4"`).
    pub format: &'static str,
    /// Shader location index.
    pub shader_location: u32,
}

/// Describes the memory layout of [`Vertex`] for GPU pipeline configuration.
pub struct VertexLayout;

impl VertexLayout {
    /// Size in bytes of one vertex.
    pub fn stride() -> u32 {
        std::mem::size_of::<Vertex>() as u32
    }

    /// Returns the attribute descriptors in shader-location order:
    ///   0 = position (float32x3)
    ///   1 = normal   (float32x3)
    ///   2 = uv       (float32x2)
    ///   3 = tangent  (float32x4)
    pub fn attributes() -> Vec<VertexAttribute> {
        vec![
            VertexAttribute {
                offset: 0,
                format: "float32x3",
                shader_location: 0,
            },
            VertexAttribute {
                offset: 12,
                format: "float32x3",
                shader_location: 1,
            },
            VertexAttribute {
                offset: 24,
                format: "float32x2",
                shader_location: 2,
            },
            VertexAttribute {
                offset: 32,
                format: "float32x4",
                shader_location: 3,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertex_new_sets_default_tangent() {
        let v = Vertex::new(Vec3::ONE, Vec3::Y, Vec2::ZERO);
        assert_eq!(v.tangent_vec4(), Vec4::new(1.0, 0.0, 0.0, 1.0));
    }

    #[test]
    fn vertex_position_only_zeros_normal_and_uv() {
        let v = Vertex::position_only(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(v.normal_vec3(), Vec3::ZERO);
        assert_eq!(v.uv_vec2(), Vec2::ZERO);
    }

    #[test]
    fn vertex_stride_matches_size() {
        // 3+3+2+4 = 12 floats * 4 bytes = 48
        assert_eq!(VertexLayout::stride(), 48);
    }

    #[test]
    fn vertex_attributes_offsets_are_correct() {
        let attrs = VertexLayout::attributes();
        assert_eq!(attrs.len(), 4);
        assert_eq!(attrs[0].offset, 0);  // position
        assert_eq!(attrs[1].offset, 12); // normal  (3 * 4)
        assert_eq!(attrs[2].offset, 24); // uv      (6 * 4)
        assert_eq!(attrs[3].offset, 32); // tangent (8 * 4)
    }

    #[test]
    fn vertex_is_pod() {
        let v = Vertex::position_only(Vec3::ZERO);
        let bytes: &[u8] = bytemuck::bytes_of(&v);
        assert_eq!(bytes.len(), 48);
    }
}
