use std::fmt;

#[derive(Debug)]
pub enum ToolkitError {
    InvalidId(String),
    LayerNotFound(u64),
    TextureNotFound(u64),
    InvalidDimensions { width: u32, height: u32 },
    SerializationError(String),
    GpuError(String),
    IoError(std::io::Error),
    Custom(String),
}

impl fmt::Display for ToolkitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(msg) => write!(f, "Invalid ID: {msg}"),
            Self::LayerNotFound(id) => write!(f, "Layer not found: {id}"),
            Self::TextureNotFound(id) => write!(f, "Texture not found: {id}"),
            Self::InvalidDimensions { width, height } => {
                write!(f, "Invalid dimensions: {width}x{height}")
            }
            Self::SerializationError(msg) => write!(f, "Serialization error: {msg}"),
            Self::GpuError(msg) => write!(f, "GPU error: {msg}"),
            Self::IoError(e) => write!(f, "IO error: {e}"),
            Self::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for ToolkitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ToolkitError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

impl From<serde_json::Error> for ToolkitError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerializationError(e.to_string())
    }
}

pub type ToolkitResult<T> = Result<T, ToolkitError>;
