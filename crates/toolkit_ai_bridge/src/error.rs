use std::fmt;

#[derive(Debug)]
pub enum BridgeError {
    ResourceNotFound(String),
    ToolNotFound(String),
    InvalidArguments(String),
    Internal(String),
    SerializationError(String),
    LockError(String),
    ProtocolError(String),
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResourceNotFound(uri) => write!(f, "resource not found: {uri}"),
            Self::ToolNotFound(name) => write!(f, "tool not found: {name}"),
            Self::InvalidArguments(msg) => write!(f, "invalid arguments: {msg}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
            Self::SerializationError(msg) => write!(f, "serialization error: {msg}"),
            Self::LockError(msg) => write!(f, "lock error: {msg}"),
            Self::ProtocolError(msg) => write!(f, "protocol error: {msg}"),
        }
    }
}

impl std::error::Error for BridgeError {}

impl From<serde_json::Error> for BridgeError {
    fn from(e: serde_json::Error) -> Self {
        Self::SerializationError(e.to_string())
    }
}

pub type BridgeResult<T> = Result<T, BridgeError>;
