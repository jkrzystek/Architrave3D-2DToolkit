use toolkit_core::{ToolkitError, ToolkitResult};

pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter_info: wgpu::AdapterInfo,
}

pub struct GpuContextDescriptor {
    pub power_preference: wgpu::PowerPreference,
    pub force_fallback: bool,
    pub required_features: wgpu::Features,
    pub required_limits: wgpu::Limits,
}

impl Default for GpuContextDescriptor {
    fn default() -> Self {
        Self {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback: false,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
        }
    }
}

impl GpuContext {
    pub async fn new(desc: &GpuContextDescriptor) -> ToolkitResult<Self> {
        let instance = wgpu::Instance::default();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: desc.power_preference,
                compatible_surface: None,
                force_fallback_adapter: desc.force_fallback,
            })
            .await
            .ok_or_else(|| ToolkitError::GpuError("No suitable GPU adapter found".into()))?;

        let adapter_info = adapter.get_info();
        log::info!(
            "GPU adapter: {} ({:?}, {:?})",
            adapter_info.name,
            adapter_info.backend,
            adapter_info.device_type
        );

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("toolkit_render device"),
                    required_features: desc.required_features,
                    required_limits: desc.required_limits.clone(),
                    ..Default::default()
                },
                None,
            )
            .await
            .map_err(|e| ToolkitError::GpuError(format!("Device request failed: {e}")))?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            adapter_info,
        })
    }

    pub fn create_shader_module(&self, label: &str, source: &str) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
    }

    pub fn create_buffer(&self, label: &str, size: u64, usage: wgpu::BufferUsages) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage,
            mapped_at_creation: false,
        })
    }

    pub fn create_buffer_init(&self, label: &str, data: &[u8], usage: wgpu::BufferUsages) -> wgpu::Buffer {
        use wgpu::util::DeviceExt;
        self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: data,
            usage,
        })
    }

    pub fn backend_name(&self) -> &str {
        match self.adapter_info.backend {
            wgpu::Backend::Vulkan => "Vulkan",
            wgpu::Backend::Metal => "Metal",
            wgpu::Backend::Dx12 => "DirectX 12",
            wgpu::Backend::Gl => "OpenGL",
            wgpu::Backend::BrowserWebGpu => "WebGPU",
            _ => "Unknown",
        }
    }
}
