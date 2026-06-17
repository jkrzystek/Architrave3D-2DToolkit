use crate::context::GpuContext;

pub struct RenderPipelineDescriptor<'a> {
    pub label: &'a str,
    pub vertex_shader: &'a str,
    pub fragment_shader: &'a str,
    pub vertex_buffer_layouts: &'a [wgpu::VertexBufferLayout<'a>],
    pub color_format: wgpu::TextureFormat,
    pub depth_format: Option<wgpu::TextureFormat>,
    pub cull_mode: Option<wgpu::Face>,
    pub topology: wgpu::PrimitiveTopology,
    pub blend_state: Option<wgpu::BlendState>,
    pub bind_group_layouts: &'a [&'a wgpu::BindGroupLayout],
}

impl<'a> Default for RenderPipelineDescriptor<'a> {
    fn default() -> Self {
        Self {
            label: "default_pipeline",
            vertex_shader: "",
            fragment_shader: "",
            vertex_buffer_layouts: &[],
            color_format: wgpu::TextureFormat::Rgba8Unorm,
            depth_format: Some(wgpu::TextureFormat::Depth32Float),
            cull_mode: Some(wgpu::Face::Back),
            topology: wgpu::PrimitiveTopology::TriangleList,
            blend_state: None,
            bind_group_layouts: &[],
        }
    }
}

pub fn create_render_pipeline(
    gpu: &GpuContext,
    desc: &RenderPipelineDescriptor,
) -> wgpu::RenderPipeline {
    let vs_module = gpu.create_shader_module(&format!("{}_vs", desc.label), desc.vertex_shader);
    let fs_module = gpu.create_shader_module(&format!("{}_fs", desc.label), desc.fragment_shader);

    let pipeline_layout = gpu
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{}_layout", desc.label)),
            bind_group_layouts: desc.bind_group_layouts,
            push_constant_ranges: &[],
        });

    let depth_stencil = desc.depth_format.map(|format| wgpu::DepthStencilState {
        format,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::Less,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    });

    gpu.device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(desc.label),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: Some("vs_main"),
                buffers: desc.vertex_buffer_layouts,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: desc.color_format,
                    blend: desc.blend_state,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: desc.topology,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: desc.cull_mode,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
}

pub struct ComputePipelineDescriptor<'a> {
    pub label: &'a str,
    pub shader_source: &'a str,
    pub entry_point: &'a str,
    pub bind_group_layouts: &'a [&'a wgpu::BindGroupLayout],
}

pub fn create_compute_pipeline(
    gpu: &GpuContext,
    desc: &ComputePipelineDescriptor,
) -> wgpu::ComputePipeline {
    let module = gpu.create_shader_module(desc.label, desc.shader_source);

    let layout = gpu
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(&format!("{}_layout", desc.label)),
            bind_group_layouts: desc.bind_group_layouts,
            push_constant_ranges: &[],
        });

    gpu.device
        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(desc.label),
            layout: Some(&layout),
            module: &module,
            entry_point: Some(desc.entry_point),
            compilation_options: Default::default(),
            cache: None,
        })
}
