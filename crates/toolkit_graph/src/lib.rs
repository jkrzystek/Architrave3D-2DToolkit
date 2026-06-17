pub mod node;
pub mod dag;
pub mod evaluation;

pub use node::{DataType, NodeInstance, NodeTemplate, NodeValue, PortDefinition};
pub use dag::NodeGraph;
pub use evaluation::{
    evaluate_graph, NodeRegistry,
    FloatConstant, AddFloat, MultiplyFloat, MixFloat,
};
