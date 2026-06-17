use toolkit_core::{TextureId, TextureFormat as TkFormat, ToolkitError, ToolkitResult};

pub struct GpuTexture {
    pub id: TextureId,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
    byte_size: u64,
}

pub struct TextureDescriptor {
    pub width: u32,
    pub height: u32,
    pub format: TkFormat,
    pub usage: TextureUsage,
    pub label: String,
}

#[derive(Debug, Clone, Copy)]
pub enum TextureUsage {
    RenderTarget,
    ShaderResource,
    Storage,
    RenderTargetAndResource,
}

impl TextureUsage {
    fn to_wgpu(self) -> wgpu::TextureUsages {
        match self {
            Self::RenderTarget => {
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC
            }
            Self::ShaderResource => {
                wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST
            }
            Self::Storage => {
                wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST
            }
            Self::RenderTargetAndResource => {
                wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::COPY_DST
            }
        }
    }
}

impl GpuTexture {
    pub fn new(device: &wgpu::Device, desc: &TextureDescriptor) -> ToolkitResult<Self> {
        if desc.width == 0 || desc.height == 0 {
            return Err(ToolkitError::InvalidDimensions {
                width: desc.width,
                height: desc.height,
            });
        }

        let wgpu_format = tk_format_to_wgpu(desc.format);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&desc.label),
            size: wgpu::Extent3d {
                width: desc.width,
                height: desc.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu_format,
            usage: desc.usage.to_wgpu(),
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let byte_size =
            desc.width as u64 * desc.height as u64 * desc.format.bytes_per_pixel() as u64;

        Ok(Self {
            id: TextureId::new(),
            texture,
            view,
            width: desc.width,
            height: desc.height,
            format: wgpu_format,
            byte_size,
        })
    }

    pub fn upload(&self, queue: &wgpu::Queue, data: &[u8]) {
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.width * self.format.block_copy_size(None).unwrap_or(4)),
                rows_per_image: Some(self.height),
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn byte_size(&self) -> u64 {
        self.byte_size
    }
}

fn tk_format_to_wgpu(f: TkFormat) -> wgpu::TextureFormat {
    match f {
        TkFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        TkFormat::Rgba8Srgb => wgpu::TextureFormat::Rgba8UnormSrgb,
        TkFormat::Rgba16Float => wgpu::TextureFormat::Rgba16Float,
        TkFormat::Rgba32Float => wgpu::TextureFormat::Rgba32Float,
        TkFormat::R8Unorm => wgpu::TextureFormat::R8Unorm,
        TkFormat::R16Float => wgpu::TextureFormat::R16Float,
        TkFormat::R32Float => wgpu::TextureFormat::R32Float,
        TkFormat::Rg8Unorm => wgpu::TextureFormat::Rg8Unorm,
        TkFormat::Rg16Float => wgpu::TextureFormat::Rg16Float,
        TkFormat::Depth32Float => wgpu::TextureFormat::Depth32Float,
    }
}

pub fn wgpu_format_to_tk(f: wgpu::TextureFormat) -> Option<TkFormat> {
    Some(match f {
        wgpu::TextureFormat::Rgba8Unorm => TkFormat::Rgba8Unorm,
        wgpu::TextureFormat::Rgba8UnormSrgb => TkFormat::Rgba8Srgb,
        wgpu::TextureFormat::Rgba16Float => TkFormat::Rgba16Float,
        wgpu::TextureFormat::Rgba32Float => TkFormat::Rgba32Float,
        wgpu::TextureFormat::R8Unorm => TkFormat::R8Unorm,
        wgpu::TextureFormat::R16Float => TkFormat::R16Float,
        wgpu::TextureFormat::R32Float => TkFormat::R32Float,
        wgpu::TextureFormat::Rg8Unorm => TkFormat::Rg8Unorm,
        wgpu::TextureFormat::Rg16Float => TkFormat::Rg16Float,
        wgpu::TextureFormat::Depth32Float => TkFormat::Depth32Float,
        _ => return None,
    })
}
