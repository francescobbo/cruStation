use crate::vertex_data::{GpuTexturedVertex, GpuVertex, ScreenQuadVertex}; // Updated vertex type
use crate::texture::TextureRenderTarget;

const COMMON_SHADER_WGSL: &str = include_str!("shaders/common.wgsl"); // Assuming you save commons.wgsl there

// Creates the render pipeline for drawing Gouraud-shaded primitives to VRAM.
pub fn create_gouraud_primitive_pipeline(
    device: &wgpu::Device,
    target_vram_format: wgpu::TextureFormat, // e.g., VRAM_FORMAT
) -> wgpu::RenderPipeline {
    let shader_source = format!("{}\n{}", COMMON_SHADER_WGSL, include_str!("shaders/gouraud_primitive.wgsl"));
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
                blend: None,
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
    let shader_source = format!("{}\n{}", COMMON_SHADER_WGSL, include_str!("shaders/screen_quad.wgsl"));
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
                blend: None,
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
                    sample_type: wgpu::TextureSampleType::Uint,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
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

// Uniform buffer struct for textured rectangle shader
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TexturedRectUniforms {
    pub modulation_color: [f32; 3], // R, G, B (normalized 0.0-1.0)
    pub texture_mode: u32,          // 0: CLUT4, 1: CLUT8, 2: Direct15bit

    // CLUT VRAM base coordinates (in 16-bit VRAM words)
    pub clut_vram_base_x: u32,
    pub clut_vram_base_y: u32,

    pub tex_page_base_x_words: u32,
    pub tex_page_base_y_words: u32,

    pub _padding: [u32; 0], // Padding to align to 16 bytes
}

pub fn create_textured_rect_pipeline(
    device: &wgpu::Device,
    target_vram_format: wgpu::TextureFormat,
    vram_texture_bind_group_layout: &wgpu::BindGroupLayout, // Group 0 for VRAM
) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) { // Return uniform BGL too
    let shader_source = format!("{}\n{}", COMMON_SHADER_WGSL, include_str!("shaders/textured_rect.wgsl"));
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Textured Rectangle Shader"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    // Bind group layout for uniforms (Group 1)
    let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Textured Rect Uniform BGL"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT, // Modulation color used in fragment
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Textured Rectangle Pipeline Layout"),
        bind_group_layouts: &[vram_texture_bind_group_layout, &uniform_bind_group_layout], // Group 0: VRAM, Group 1: Uniforms
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Textured Rectangle Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: Some("vs_main"),
            buffers: &[GpuTexturedVertex::desc()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_vram_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw, // PS1 is usually CCW for polys
            cull_mode: None,                  // Usually no culling for 2D rects
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    });
    (pipeline, uniform_bind_group_layout)
}
