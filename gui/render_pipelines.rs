use crate::vertex_data::{TriangleVertex, ScreenQuadVertex};
use crate::texture::TextureRenderTarget;

// Creates the render pipeline for drawing the rainbow triangle.
// This pipeline is configured to render to an offscreen texture.
//
// Args:
// - device: The WGPU device.
// - target_texture_format: The format of the texture this pipeline will render to (e.g., SCREEN_TEXTURE_FORMAT).
pub fn create_triangle_render_pipeline(
    device: &wgpu::Device,
    target_texture_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    // Load the WGSL shader code for the triangle.
    let shader_source = include_str!("shaders/triangle.wgsl");
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Triangle Shader Module"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    // Define the layout of the pipeline (e.g., bind groups, push constants).
    // The triangle pipeline doesn't use any bind groups.
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Triangle Pipeline Layout"),
        bind_group_layouts: &[], // No bind groups needed for this simple triangle.
        push_constant_ranges: &[], // No push constants needed.
    });

    // Create the render pipeline.
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Triangle Render Pipeline"),
        layout: Some(&pipeline_layout),
        // Vertex stage:
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: "vs_main", // Name of the vertex shader function.
            buffers: &[TriangleVertex::desc()], // Description of the vertex buffer layout.
        },
        // Fragment stage:
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_main", // Name of the fragment shader function.
            // Defines the color targets this pipeline will render to.
            targets: &[Some(wgpu::ColorTargetState {
                format: target_texture_format, // Must match the format of the render target.
                blend: Some(wgpu::BlendState::REPLACE), // Overwrite existing pixels.
                write_mask: wgpu::ColorWrites::ALL, // Write to all color channels (RGBA).
            })],
        }),
        // How to interpret vertices (e.g., as triangles, lines).
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList, // Draw triangles.
            strip_index_format: None, // Not using triangle strips.
            front_face: wgpu::FrontFace::Ccw, // Counter-clockwise triangles are front-facing.
            cull_mode: Some(wgpu::Face::Back), // Cull back-facing triangles.
            polygon_mode: wgpu::PolygonMode::Fill, // Fill triangles, don't draw wireframes.
            unclipped_depth: false, // No special depth clipping.
            conservative: false,    // No conservative rasterization.
        },
        depth_stencil: None, // No depth or stencil buffer for this pipeline.
        // Multisampling configuration.
        multisample: wgpu::MultisampleState {
            count: 1, // No multisampling.
            mask: !0, // Use all samples.
            alpha_to_coverage_enabled: false, // No alpha-to-coverage.
        },
        multiview: None, // Not using multiview rendering.
    })
}

// Creates the render pipeline for drawing the full-screen quad,
// which is textured with the content of the offscreen "screen" texture.
// This pipeline renders to the main window's surface.
//
// Args:
// - device: The WGPU device.
// - target_surface_format: The format of the window's surface this pipeline renders to.
// - screen_texture_bind_group_layout: The layout for the bind group containing the screen texture.
pub fn create_screen_quad_render_pipeline(
    device: &wgpu::Device,
    target_surface_format: wgpu::TextureFormat,
    screen_texture_bind_group_layout: &wgpu::BindGroupLayout, // Takes the layout as input
) -> wgpu::RenderPipeline {
    // Load the WGSL shader code for the screen quad.
    let shader_source = include_str!("shaders/screen_quad.wgsl");
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Screen Quad Shader Module"),
        source: wgpu::ShaderSource::Wgsl(shader_source.into()),
    });

    // Define the layout of this pipeline. It uses one bind group (for the texture).
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Screen Quad Pipeline Layout"),
        bind_group_layouts: &[screen_texture_bind_group_layout], // Uses the provided bind group layout.
        push_constant_ranges: &[],
    });

    // Create the render pipeline.
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Screen Quad Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: "vs_main",
            buffers: &[ScreenQuadVertex::desc()], // Uses ScreenQuadVertex layout.
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: target_surface_format, // Renders to the window's surface format.
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default() // Sensible defaults for other primitive state.
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(), // Default multisample state.
        multiview: None,
    })
}

// Creates the BindGroupLayout and BindGroup for the screen texture.
// This defines how the screen texture and its sampler are made available to the shader.
pub fn create_screen_texture_bind_group_resources(
    device: &wgpu::Device,
    screen_render_target: &TextureRenderTarget,
) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
    // Define the layout of bindings for the screen texture.
    // This describes what resources (texture, sampler) are in the bind group and how they are used.
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Screen Texture Bind Group Layout"),
        entries: &[
            // Entry for the screen texture itself.
            wgpu::BindGroupLayoutEntry {
                binding: 0, // Corresponds to `@binding(0)` in the screen_quad.wgsl shader.
                visibility: wgpu::ShaderStages::FRAGMENT, // Accessible only in the fragment shader.
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true }, // Texture contains float data, and can be filtered.
                    view_dimension: wgpu::TextureViewDimension::D2, // It's a 2D texture.
                    multisampled: false, // Not multisampled.
                },
                count: None, // A single texture, not an array of textures.
            },
            // Entry for the sampler associated with the screen texture.
            wgpu::BindGroupLayoutEntry {
                binding: 1, // Corresponds to `@binding(1)` in the screen_quad.wgsl shader.
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), // A filtering sampler.
                count: None,
            },
        ],
    });

    // Create the actual bind group, which bundles the texture view and sampler.
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Screen Texture Bind Group"),
        layout: &bind_group_layout, // Uses the layout defined above.
        entries: &[
            // The screen texture's view.
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&screen_render_target.view),
            },
            // The screen texture's sampler.
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&screen_render_target.sampler),
            },
        ],
    });

    (bind_group_layout, bind_group)
}
