use crate::vertex_data::{GpuVertex, ScreenQuadVertex}; // Updated vertex type
use crate::texture::TextureRenderTarget;

// Creates the render pipeline for drawing Gouraud-shaded primitives to VRAM.
pub fn create_gouraud_primitive_pipeline(
    device: &wgpu::Device,
    target_vram_format: wgpu::TextureFormat, // e.g., VRAM_FORMAT
) -> wgpu::RenderPipeline {
    let shader_source = include_str!("shaders/gouraud_primitive.wgsl");
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Gouraud Primitive Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Gouraud Primitive Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Gouraud Primitive Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: Some("vs_main"),
            buffers: &[GpuVertex::desc()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_vram_format,
                // PS1 blending modes are specific. Start with replace or basic alpha.
                // Example: Basic alpha blending if your GpuVertex had an alpha component.
                // blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                blend: Some(wgpu::BlendState::REPLACE), // For now
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw, 
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

// Pipeline for drawing the VRAM display area to the screen
pub fn create_vram_to_screen_pipeline(
    device: &wgpu::Device,
    target_surface_format: wgpu::TextureFormat,
    vram_texture_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader_source = include_str!("shaders/screen_quad.wgsl");
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Screen Quad Shader (VRAM to Screen)"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("VRAM to Screen Pipeline Layout"),
        bind_group_layouts: &[vram_texture_bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("VRAM to Screen Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: Some("vs_main"),
            buffers: &[ScreenQuadVertex::desc()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_surface_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(), // Default for quad
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

// Bind group for accessing VRAM as a texture (used by vram_to_screen_pipeline and future textured prims)
pub fn create_vram_bind_group_resources(
    device: &wgpu::Device,
    vram_render_target: &TextureRenderTarget,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("VRAM Texture Bind Group Layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true }, // Or false for nearest neighbor like PS1
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                // PS1 uses nearest neighbor typically, but linear can be okay for dev
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("VRAM Texture Bind Group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&vram_render_target.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&vram_render_target.sampler), // Consider a nearest-neighbor sampler
            },
        ],
    });

    (bind_group_layout, bind_group)
}