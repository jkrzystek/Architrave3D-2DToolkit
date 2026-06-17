use crate::id::{LayerId, TextureId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    SoftLight,
    HardLight,
    ColorDodge,
    ColorBurn,
    Darken,
    Lighten,
    Difference,
    Exclusion,
    Add,
    Subtract,
    PassThrough,
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LayerKind {
    Paint,
    Fill,
    Folder,
    Mask,
    Adjustment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DocumentCommand {
    AddLayer {
        name: String,
        kind: LayerKind,
        parent: Option<LayerId>,
    },
    RemoveLayer {
        id: LayerId,
    },
    SetLayerOpacity {
        id: LayerId,
        opacity: f32,
    },
    SetLayerVisibility {
        id: LayerId,
        visible: bool,
    },
    SetLayerBlendMode {
        id: LayerId,
        mode: BlendMode,
    },
    MoveLayer {
        id: LayerId,
        new_parent: Option<LayerId>,
        index: usize,
    },
    RenameLayer {
        id: LayerId,
        name: String,
    },
    Undo,
    Redo,
}

#[derive(Debug, Clone)]
pub enum RenderCommand {
    UploadTexture {
        id: TextureId,
        data: Vec<u8>,
        width: u32,
        height: u32,
        format: TextureFormat,
    },
    RemoveTexture {
        id: TextureId,
    },
    InvalidateLayer {
        id: LayerId,
    },
    InvalidateViewport,
    ResizeViewport {
        width: u32,
        height: u32,
    },
    SetClearColor {
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TextureFormat {
    Rgba8Unorm,
    Rgba8Srgb,
    Rgba16Float,
    Rgba32Float,
    R8Unorm,
    R16Float,
    R32Float,
    Rg8Unorm,
    Rg16Float,
    Depth32Float,
}

impl TextureFormat {
    pub fn bytes_per_pixel(self) -> u32 {
        match self {
            Self::R8Unorm => 1,
            Self::Rg8Unorm => 2,
            Self::Rgba8Unorm | Self::Rgba8Srgb => 4,
            Self::R16Float => 2,
            Self::Rg16Float => 4,
            Self::R32Float | Self::Depth32Float => 4,
            Self::Rgba16Float => 8,
            Self::Rgba32Float => 16,
        }
    }

    pub fn channel_count(self) -> u32 {
        match self {
            Self::R8Unorm | Self::R16Float | Self::R32Float | Self::Depth32Float => 1,
            Self::Rg8Unorm | Self::Rg16Float => 2,
            Self::Rgba8Unorm | Self::Rgba8Srgb | Self::Rgba16Float | Self::Rgba32Float => 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_mode_default_is_normal() {
        assert_eq!(BlendMode::default(), BlendMode::Normal);
    }

    #[test]
    fn texture_format_bytes() {
        assert_eq!(TextureFormat::Rgba8Unorm.bytes_per_pixel(), 4);
        assert_eq!(TextureFormat::Rgba16Float.bytes_per_pixel(), 8);
        assert_eq!(TextureFormat::Rgba32Float.bytes_per_pixel(), 16);
        assert_eq!(TextureFormat::R8Unorm.bytes_per_pixel(), 1);
    }

    #[test]
    fn texture_format_channels() {
        assert_eq!(TextureFormat::R8Unorm.channel_count(), 1);
        assert_eq!(TextureFormat::Rg8Unorm.channel_count(), 2);
        assert_eq!(TextureFormat::Rgba8Unorm.channel_count(), 4);
    }

    #[test]
    fn document_command_serialization() {
        let cmd = DocumentCommand::AddLayer {
            name: "Base".into(),
            kind: LayerKind::Paint,
            parent: None,
        };
        let json = serde_json::to_string(&cmd).unwrap();
        let deserialized: DocumentCommand = serde_json::from_str(&json).unwrap();
        match deserialized {
            DocumentCommand::AddLayer { name, kind, parent } => {
                assert_eq!(name, "Base");
                assert_eq!(kind, LayerKind::Paint);
                assert!(parent.is_none());
            }
            _ => panic!("wrong variant"),
        }
    }
}
