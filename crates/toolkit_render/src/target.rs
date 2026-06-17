use toolkit_core::{TextureFormat, ToolkitError, ToolkitResult, ViewportId};

use crate::context::GpuContext;
use crate::texture::{GpuTexture, TextureDescriptor, TextureUsage};

pub struct RenderTarget {
    pub id: ViewportId,
    pub color: GpuTexture,
    pub depth: GpuTexture,
    pub width: u32,
    pub height: u32,
}

impl RenderTarget {
    pub fn new(
        gpu: &GpuContext,
        width: u32,
        height: u32,
        color_format: TextureFormat,
    ) -> ToolkitResult<Self> {
        if width == 0 || height == 0 {
            return Err(ToolkitError::InvalidDimensions { width, height });
        }

        let color = GpuTexture::new(
            &gpu.device,
            &TextureDescriptor {
                width,
                height,
                format: color_format,
                usage: TextureUsage::RenderTargetAndResource,
                label: "render_target_color".into(),
            },
        )?;

        let depth = GpuTexture::new(
            &gpu.device,
            &TextureDescriptor {
                width,
                height,
                format: TextureFormat::Depth32Float,
                usage: TextureUsage::RenderTarget,
                label: "render_target_depth".into(),
            },
        )?;

        Ok(Self {
            id: ViewportId::new(),
            color,
            depth,
            width,
            height,
        })
    }

    pub fn resize(
        &mut self,
        gpu: &GpuContext,
        width: u32,
        height: u32,
    ) -> ToolkitResult<()> {
        if width == self.width && height == self.height {
            return Ok(());
        }

        let new_target = Self::new(gpu, width, height, TextureFormat::Rgba8Unorm)?;
        self.color = new_target.color;
        self.depth = new_target.depth;
        self.width = width;
        self.height = height;
        Ok(())
    }

    pub fn color_view(&self) -> &wgpu::TextureView {
        &self.color.view
    }

    pub fn depth_view(&self) -> &wgpu::TextureView {
        &self.depth.view
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }
}
