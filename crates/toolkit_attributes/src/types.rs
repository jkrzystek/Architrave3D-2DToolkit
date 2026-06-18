//! Attribute domains, value types, and the columnar storage for one channel.
//!
//! An [`Attribute`] is a single named channel: a flat column of values, all of
//! one [`AttributeType`], one entry per element of its domain. Storing data
//! columnar (struct-of-arrays) keeps it cache-friendly and trivially mappable to
//! GPU buffers, and matches how procedural tools think ("the `Cd` attribute on
//! points").

use glam::{Vec2, Vec3, Vec4};
use serde::{Deserialize, Serialize};

/// Which geometry element an attribute set is bound to.
///
/// `Detail` is a single global value for the whole geometry (Houdini's "detail"
/// domain), useful for parameters that travel with the data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Domain {
    Point,
    Vertex,
    Edge,
    Face,
    Primitive,
    Detail,
}

impl Domain {
    /// All domains, in a stable order.
    pub const ALL: [Domain; 6] = [
        Domain::Point,
        Domain::Vertex,
        Domain::Edge,
        Domain::Face,
        Domain::Primitive,
        Domain::Detail,
    ];

    /// Lowercase identifier (`"point"`, `"face"`, ...).
    pub fn name(self) -> &'static str {
        match self {
            Domain::Point => "point",
            Domain::Vertex => "vertex",
            Domain::Edge => "edge",
            Domain::Face => "face",
            Domain::Primitive => "primitive",
            Domain::Detail => "detail",
        }
    }
}

/// The scalar/vector type stored in an attribute column.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttributeType {
    Float,
    Int,
    Bool,
    Vec2,
    Vec3,
    Vec4,
    /// RGBA stored linear; same layout as `Vec4` but semantically a color.
    Color,
    Str,
}

/// Columnar storage for one attribute, one entry per element.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AttributeData {
    Float(Vec<f32>),
    Int(Vec<i32>),
    Bool(Vec<bool>),
    Vec2(Vec<[f32; 2]>),
    Vec3(Vec<[f32; 3]>),
    Vec4(Vec<[f32; 4]>),
    Color(Vec<[f32; 4]>),
    Str(Vec<String>),
}

impl AttributeData {
    /// Allocate `len` default-valued entries of the given type.
    pub fn filled(ty: AttributeType, len: usize) -> Self {
        match ty {
            AttributeType::Float => AttributeData::Float(vec![0.0; len]),
            AttributeType::Int => AttributeData::Int(vec![0; len]),
            AttributeType::Bool => AttributeData::Bool(vec![false; len]),
            AttributeType::Vec2 => AttributeData::Vec2(vec![[0.0; 2]; len]),
            AttributeType::Vec3 => AttributeData::Vec3(vec![[0.0; 3]; len]),
            AttributeType::Vec4 => AttributeData::Vec4(vec![[0.0; 4]; len]),
            AttributeType::Color => AttributeData::Color(vec![[0.0, 0.0, 0.0, 1.0]; len]),
            AttributeType::Str => AttributeData::Str(vec![String::new(); len]),
        }
    }

    /// The element type held by this column.
    pub fn ty(&self) -> AttributeType {
        match self {
            AttributeData::Float(_) => AttributeType::Float,
            AttributeData::Int(_) => AttributeType::Int,
            AttributeData::Bool(_) => AttributeType::Bool,
            AttributeData::Vec2(_) => AttributeType::Vec2,
            AttributeData::Vec3(_) => AttributeType::Vec3,
            AttributeData::Vec4(_) => AttributeType::Vec4,
            AttributeData::Color(_) => AttributeType::Color,
            AttributeData::Str(_) => AttributeType::Str,
        }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        match self {
            AttributeData::Float(v) => v.len(),
            AttributeData::Int(v) => v.len(),
            AttributeData::Bool(v) => v.len(),
            AttributeData::Vec2(v) => v.len(),
            AttributeData::Vec3(v) => v.len(),
            AttributeData::Vec4(v) => v.len(),
            AttributeData::Color(v) => v.len(),
            AttributeData::Str(v) => v.len(),
        }
    }

    /// Whether the column is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Grow or shrink to `len`, appending type-appropriate defaults.
    pub fn resize(&mut self, len: usize) {
        match self {
            AttributeData::Float(v) => v.resize(len, 0.0),
            AttributeData::Int(v) => v.resize(len, 0),
            AttributeData::Bool(v) => v.resize(len, false),
            AttributeData::Vec2(v) => v.resize(len, [0.0; 2]),
            AttributeData::Vec3(v) => v.resize(len, [0.0; 3]),
            AttributeData::Vec4(v) => v.resize(len, [0.0; 4]),
            AttributeData::Color(v) => v.resize(len, [0.0, 0.0, 0.0, 1.0]),
            AttributeData::Str(v) => v.resize(len, String::new()),
        }
    }
}

/// A single named attribute channel.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub data: AttributeData,
}

impl Attribute {
    /// Create a channel of `len` default values.
    pub fn new(name: impl Into<String>, ty: AttributeType, len: usize) -> Self {
        Self {
            name: name.into(),
            data: AttributeData::filled(ty, len),
        }
    }

    /// Wrap existing columnar data.
    pub fn from_data(name: impl Into<String>, data: AttributeData) -> Self {
        Self {
            name: name.into(),
            data,
        }
    }

    pub fn ty(&self) -> AttributeType {
        self.data.ty()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn resize(&mut self, len: usize) {
        self.data.resize(len);
    }

    // -- Typed accessors -----------------------------------------------------
    // Each returns `None` when the stored type does not match the request, so
    // callers never silently read garbage from the wrong column type.

    pub fn as_float(&self) -> Option<&[f32]> {
        match &self.data {
            AttributeData::Float(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_float_mut(&mut self) -> Option<&mut Vec<f32>> {
        match &mut self.data {
            AttributeData::Float(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<&[i32]> {
        match &self.data {
            AttributeData::Int(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<&[bool]> {
        match &self.data {
            AttributeData::Bool(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&[String]> {
        match &self.data {
            AttributeData::Str(v) => Some(v),
            _ => None,
        }
    }

    /// Read entry `i` as a `Vec2` (works for `Vec2` columns).
    pub fn get_vec2(&self, i: usize) -> Option<Vec2> {
        match &self.data {
            AttributeData::Vec2(v) => v.get(i).map(|a| Vec2::from(*a)),
            _ => None,
        }
    }

    /// Read entry `i` as a `Vec3` (works for `Vec3` columns).
    pub fn get_vec3(&self, i: usize) -> Option<Vec3> {
        match &self.data {
            AttributeData::Vec3(v) => v.get(i).map(|a| Vec3::from(*a)),
            _ => None,
        }
    }

    /// Read entry `i` as a `Vec4` (works for `Vec4` and `Color` columns).
    pub fn get_vec4(&self, i: usize) -> Option<Vec4> {
        match &self.data {
            AttributeData::Vec4(v) | AttributeData::Color(v) => v.get(i).map(|a| Vec4::from(*a)),
            _ => None,
        }
    }

    /// Read entry `i` as `f32` (works for `Float` columns).
    pub fn get_float(&self, i: usize) -> Option<f32> {
        self.as_float().and_then(|v| v.get(i).copied())
    }

    /// Write entry `i` from a `Vec3`; returns `false` if the type/index is wrong.
    pub fn set_vec3(&mut self, i: usize, value: Vec3) -> bool {
        match &mut self.data {
            AttributeData::Vec3(v) if i < v.len() => {
                v[i] = value.into();
                true
            }
            _ => false,
        }
    }

    /// Write entry `i` from a `Vec2`; returns `false` if the type/index is wrong.
    pub fn set_vec2(&mut self, i: usize, value: Vec2) -> bool {
        match &mut self.data {
            AttributeData::Vec2(v) if i < v.len() => {
                v[i] = value.into();
                true
            }
            _ => false,
        }
    }

    /// Write entry `i` from a `Vec4` (also accepts `Color` columns).
    pub fn set_vec4(&mut self, i: usize, value: Vec4) -> bool {
        match &mut self.data {
            AttributeData::Vec4(v) | AttributeData::Color(v) if i < v.len() => {
                v[i] = value.into();
                true
            }
            _ => false,
        }
    }

    /// Write entry `i` from an `f32`; returns `false` if the type/index is wrong.
    pub fn set_float(&mut self, i: usize, value: f32) -> bool {
        match &mut self.data {
            AttributeData::Float(v) if i < v.len() => {
                v[i] = value;
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filled_has_expected_defaults() {
        let c = AttributeData::filled(AttributeType::Color, 2);
        match c {
            AttributeData::Color(v) => assert_eq!(v, vec![[0.0, 0.0, 0.0, 1.0]; 2]),
            _ => panic!("wrong variant"),
        }
        assert_eq!(AttributeData::filled(AttributeType::Float, 3).len(), 3);
    }

    #[test]
    fn typed_get_set_roundtrip() {
        let mut a = Attribute::new("P", AttributeType::Vec3, 4);
        assert!(a.set_vec3(1, Vec3::new(1.0, 2.0, 3.0)));
        assert_eq!(a.get_vec3(1), Some(Vec3::new(1.0, 2.0, 3.0)));
        // Wrong type access returns None / false.
        assert_eq!(a.get_float(1), None);
        assert!(!a.set_float(1, 5.0));
    }

    #[test]
    fn color_reads_as_vec4() {
        let mut a = Attribute::new("Cd", AttributeType::Color, 1);
        assert!(a.set_vec4(0, Vec4::new(0.2, 0.4, 0.6, 1.0)));
        assert_eq!(a.get_vec4(0), Some(Vec4::new(0.2, 0.4, 0.6, 1.0)));
    }

    #[test]
    fn resize_appends_defaults() {
        let mut a = Attribute::new("w", AttributeType::Float, 2);
        a.set_float(0, 1.0);
        a.resize(4);
        assert_eq!(a.len(), 4);
        assert_eq!(a.get_float(0), Some(1.0));
        assert_eq!(a.get_float(3), Some(0.0));
    }

    #[test]
    fn ty_matches_data() {
        assert_eq!(Attribute::new("x", AttributeType::Int, 1).ty(), AttributeType::Int);
    }

    #[test]
    fn domain_names_are_stable() {
        assert_eq!(Domain::Point.name(), "point");
        assert_eq!(Domain::ALL.len(), 6);
    }

    #[test]
    fn serde_roundtrip() {
        let a = Attribute::new("Cd", AttributeType::Vec3, 3);
        let json = serde_json::to_string(&a).unwrap();
        let back: Attribute = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }
}
