use std::fmt;

/// Errors produced while importing or exporting assets.
#[derive(Debug)]
pub enum AssetError {
    /// An I/O failure reading or writing a file.
    Io(std::io::Error),
    /// The file's text/structure could not be parsed.
    Parse(String),
    /// A glTF-specific decoding failure.
    Gltf(String),
    /// The asset referenced external data that could not be resolved.
    UnsupportedFeature(String),
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetError::Io(e) => write!(f, "io error: {e}"),
            AssetError::Parse(m) => write!(f, "parse error: {m}"),
            AssetError::Gltf(m) => write!(f, "glTF error: {m}"),
            AssetError::UnsupportedFeature(m) => write!(f, "unsupported: {m}"),
        }
    }
}

impl std::error::Error for AssetError {}

impl From<std::io::Error> for AssetError {
    fn from(e: std::io::Error) -> Self {
        AssetError::Io(e)
    }
}

pub type AssetResult<T> = Result<T, AssetError>;
