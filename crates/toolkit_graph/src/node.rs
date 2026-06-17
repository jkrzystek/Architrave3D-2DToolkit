use serde::{Deserialize, Serialize};
use toolkit_core::NodeId;

/// Types that can flow through node connections.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataType {
    Float,
    Vec2,
    Vec3,
    Vec4,
    Color,
    Image,
    Mesh,
    Any,
}

impl DataType {
    /// Returns `true` when a connection from `output_type` to `input_type` is
    /// type-compatible.  `Any` on either side is always compatible, and
    /// identical types are compatible.
    pub fn compatible(output_type: DataType, input_type: DataType) -> bool {
        output_type == input_type
            || output_type == DataType::Any
            || input_type == DataType::Any
    }
}

/// Definition of a single input or output port on a node template.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PortDefinition {
    pub name: String,
    pub data_type: DataType,
    pub default_value: Option<NodeValue>,
}

impl PortDefinition {
    pub fn new(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            default_value: None,
        }
    }

    pub fn with_default(mut self, value: NodeValue) -> Self {
        self.default_value = Some(value);
        self
    }
}

/// A concrete value flowing through the graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NodeValue {
    Float(f32),
    Vec2(f32, f32),
    Vec3(f32, f32, f32),
    Vec4(f32, f32, f32, f32),
    Color([f32; 4]),
    Buffer(Vec<u8>),
    None,
}

impl NodeValue {
    /// Extract as f32, returning 0.0 for non-float variants.
    pub fn as_float(&self) -> f32 {
        match self {
            NodeValue::Float(v) => *v,
            _ => 0.0,
        }
    }
}

impl PartialEq for NodeValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (NodeValue::Float(a), NodeValue::Float(b)) => a == b,
            (NodeValue::Vec2(a0, a1), NodeValue::Vec2(b0, b1)) => a0 == b0 && a1 == b1,
            (NodeValue::Vec3(a0, a1, a2), NodeValue::Vec3(b0, b1, b2)) => {
                a0 == b0 && a1 == b1 && a2 == b2
            }
            (NodeValue::Vec4(a0, a1, a2, a3), NodeValue::Vec4(b0, b1, b2, b3)) => {
                a0 == b0 && a1 == b1 && a2 == b2 && a3 == b3
            }
            (NodeValue::Color(a), NodeValue::Color(b)) => a == b,
            (NodeValue::Buffer(a), NodeValue::Buffer(b)) => a == b,
            (NodeValue::None, NodeValue::None) => true,
            _ => false,
        }
    }
}

/// A registered node type that knows its port layout and how to compute
/// outputs from inputs.
pub trait NodeTemplate: Send + Sync {
    fn name(&self) -> &str;
    fn inputs(&self) -> Vec<PortDefinition>;
    fn outputs(&self) -> Vec<PortDefinition>;
    fn evaluate(&self, inputs: &[NodeValue]) -> Vec<NodeValue>;
}

/// A live instance of a node placed in a `NodeGraph`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeInstance {
    pub id: NodeId,
    pub template_name: String,
    pub input_values: Vec<NodeValue>,
    pub cached_outputs: Option<Vec<NodeValue>>,
    pub dirty: bool,
    pub position: (f32, f32),
}

impl NodeInstance {
    pub fn new(id: NodeId, template_name: String, position: (f32, f32)) -> Self {
        Self {
            id,
            template_name,
            input_values: Vec::new(),
            cached_outputs: None,
            dirty: true,
            position,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_type_compatibility_same() {
        assert!(DataType::compatible(DataType::Float, DataType::Float));
        assert!(DataType::compatible(DataType::Vec3, DataType::Vec3));
    }

    #[test]
    fn data_type_compatibility_any() {
        assert!(DataType::compatible(DataType::Any, DataType::Float));
        assert!(DataType::compatible(DataType::Mesh, DataType::Any));
        assert!(DataType::compatible(DataType::Any, DataType::Any));
    }

    #[test]
    fn data_type_incompatible() {
        assert!(!DataType::compatible(DataType::Float, DataType::Vec2));
        assert!(!DataType::compatible(DataType::Image, DataType::Mesh));
    }

    #[test]
    fn node_value_as_float() {
        assert_eq!(NodeValue::Float(3.14).as_float(), 3.14);
        assert_eq!(NodeValue::None.as_float(), 0.0);
        assert_eq!(NodeValue::Vec2(1.0, 2.0).as_float(), 0.0);
    }

    #[test]
    fn node_value_equality() {
        assert_eq!(NodeValue::Float(1.0), NodeValue::Float(1.0));
        assert_ne!(NodeValue::Float(1.0), NodeValue::Float(2.0));
        assert_eq!(NodeValue::None, NodeValue::None);
        assert_ne!(NodeValue::Float(1.0), NodeValue::None);
        assert_eq!(
            NodeValue::Color([1.0, 0.0, 0.0, 1.0]),
            NodeValue::Color([1.0, 0.0, 0.0, 1.0])
        );
        assert_eq!(
            NodeValue::Buffer(vec![1, 2, 3]),
            NodeValue::Buffer(vec![1, 2, 3])
        );
    }

    #[test]
    fn port_definition_with_default() {
        let port = PortDefinition::new("value", DataType::Float)
            .with_default(NodeValue::Float(1.0));
        assert_eq!(port.name, "value");
        assert_eq!(port.data_type, DataType::Float);
        assert_eq!(port.default_value, Some(NodeValue::Float(1.0)));
    }

    #[test]
    fn node_instance_starts_dirty() {
        let inst = NodeInstance::new(NodeId::new(), "Test".into(), (0.0, 0.0));
        assert!(inst.dirty);
        assert!(inst.cached_outputs.is_none());
    }
}
