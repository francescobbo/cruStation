use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use wgpu::util::DeviceExt;
use winit::window::Window;
use winit::dpi::PhysicalSize;

use crate::vertex_data::{
    self, GpuTexturedVertex, GpuVertex, ScreenQuadVertex, SCREEN_QUAD_VERTICES, VRAM_FORMAT, VRAM_HEIGHT, VRAM_WIDTH
};
use crate::texture::TextureRenderTarget;
use crate::render_pipelines::{self, TexturedRectUniforms};
use crate::gpu_command::{GpuCommand, PsxVertex as CommandPsxVertex};
use crate::PsxColor; // Alias to avoid conflict

pub struct Renderer {
    // WGPU Core
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    // VRAM
    vram_texture: TextureRenderTarget,
    // Pipelines
    gouraud_pipeline: wgpu::RenderPipeline,
    textured_rect_pipeline: wgpu::RenderPipeline,
    vram_to_screen_pipeline: wgpu::RenderPipeline,
    // Resources for VRAM display
    screen_quad_vertex_buffer: wgpu::Buffer,
    textured_rect_uniform_buffer: wgpu::Buffer,
    textured_rect_uniform_bgl: wgpu::BindGroupLayout,
    vram_bind_group: wgpu::BindGroup,
    // Dynamic primitive drawing
    primitive_vertex_buffer: wgpu::Buffer,
    // Store current display parameters derived from PsxGpuRegisters
    // to avoid re-calculating vertex buffer or uniforms every frame if unchanged.
    current_display_params: (u32, u32, u32, u32), // x, y, w, h in VRAM

    // Calculated viewport for rendering the 4:3 (or content aspect) area onto the window
    final_display_viewport_x: f32,
    final_display_viewport_y: f32,
    final_display_viewport_width: f32,
    final_display_viewport_height: f32,

    // Temporary texture for sourcing textures VRAM to VRAM
    // (e.g., for CLUT or texture page operations)
    temporary_texture: TextureRenderTarget,
    temporary_texture_bg: wgpu::BindGroup,
    vram_sampler_bgl: wgpu::BindGroupLayout,

    command_rx: crossbeam_channel::Receiver<GpuCommand>,

    drawing_area_x1: u16,
    drawing_area_y1: u16,
    drawing_area_x2: u16,
    drawing_area_y2: u16,
    drawing_offset_x: i16,
    drawing_offset_y: i16,

    vram_debug: bool
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> (Arc<Mutex<Self>>, crossbeam_channel::Sender<GpuCommand>) {
        let window_size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            ..wgpu::InstanceDescriptor::default()
        });
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Renderer Device"),
                required_features: wgpu::Features::TEXTURE_BINDING_ARRAY | wgpu::Features::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING, // For later flexibility
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(surface_caps.formats[0]);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let vram_texture = TextureRenderTarget::new(
            &device, VRAM_WIDTH, VRAM_HEIGHT, VRAM_FORMAT,
                wgpu::TextureUsages::RENDER_ATTACHMENT | 
                wgpu::TextureUsages::TEXTURE_BINDING | 
                wgpu::TextureUsages::COPY_DST | 
                wgpu::TextureUsages::COPY_SRC,
            Some("VRAM Texture"),
        );

        // Create the Texture Source Cache
        // It needs to be sampleable (TEXTURE_BINDING) and a copy destination (COPY_DST).
        let texture_source_cache = TextureRenderTarget::new(
            &device,
            VRAM_WIDTH, VRAM_HEIGHT,
            VRAM_FORMAT, // Same format as VRAM for direct copies
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            Some("Texture Source Cache"),
        );

        let gouraud_pipeline = render_pipelines::create_gouraud_primitive_pipeline(&device, vram_texture.format);

        let (vram_bgl, vram_bind_group) =
            render_pipelines::create_vram_bind_group_resources(&device, &vram_texture);
        
        let vram_to_screen_pipeline = render_pipelines::create_vram_to_screen_pipeline(
            &device, surface_config.format, &vram_bgl,
        );

        // Initial screen quad uses default display area from vertex_data
        let screen_quad_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Screen Quad VB"),
            contents: bytemuck::cast_slice(SCREEN_QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        
        let primitive_vertex_capacity = 6 * 10;
        let primitive_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dynamic Primitive VB"),
            size: (primitive_vertex_capacity * std::mem::size_of::<GpuVertex>()) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        let initial_display_params = (
            vertex_data::DEFAULT_DISPLAY_X_IN_VRAM,
            vertex_data::DEFAULT_DISPLAY_Y_IN_VRAM,
            vertex_data::DEFAULT_DISPLAY_WIDTH,
            vertex_data::DEFAULT_DISPLAY_HEIGHT,
        );

        let (textured_rect_pipeline, textured_rect_uniform_bgl) =
            render_pipelines::create_textured_rect_pipeline(&device, vram_texture.format, &vram_bgl);

        // Uniform buffer for textured rectangle shader
        let textured_rect_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Textured Rect Uniform Buffer"),
            contents: bytemuck::cast_slice(&[TexturedRectUniforms {
                modulation_color: [0.5, 0.5, 0.5],
                texture_mode: 0,
                clut_vram_base_x: 0,
                clut_vram_base_y: 0,
                tex_page_base_x_words: 0,
                tex_page_base_y_words: 0,
                _padding: [0; 0],
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Bind group for sampling from the texture_source_cache
        let texture_source_cache_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Source Cache Bind Group"),
            layout: &vram_bgl, // Reuse the same layout structure
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_source_cache.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    // You can use texture_source_cache.sampler or a globally defined sampler
                    resource: wgpu::BindingResource::Sampler(&texture_source_cache.sampler),
                },
            ],
        });

        let (command_tx, command_rx) = crossbeam_channel::unbounded();

        let mut renderer = Self {
            window, surface, device, queue, surface_config,
            vram_texture,
            gouraud_pipeline, textured_rect_pipeline, vram_to_screen_pipeline,
            screen_quad_vertex_buffer, vram_bind_group,
            textured_rect_uniform_buffer, textured_rect_uniform_bgl,
            primitive_vertex_buffer,
            current_display_params: initial_display_params,
            final_display_viewport_x: 0.0,
            final_display_viewport_y: 0.0,
            final_display_viewport_width: window_size.width as f32,
            final_display_viewport_height: window_size.height as f32,

            drawing_area_x1: 0,
            drawing_area_y1: 0,
            drawing_area_x2: 1023,
            drawing_area_y2: 511,
            drawing_offset_x: 0,
            drawing_offset_y: 0,

            temporary_texture: texture_source_cache,
            temporary_texture_bg: texture_source_cache_bind_group,
            vram_sampler_bgl: vram_bgl,

            command_rx,

            vram_debug: false,
        };

        renderer.update_final_display_viewport(); // Calculate initial viewport
        renderer.update_screen_quad_tex_coords(); // Set initial tex coords for screen quad

        (Arc::new(Mutex::new(renderer)), command_tx)
    }

    pub fn start_command_thread(renderer: Arc<Mutex<Self>>) {
        let renderer = renderer.clone();
        let command_rx = renderer.lock().unwrap().command_rx.clone();

        std::thread::spawn(move || {
            loop {
                // Wait for commands from the emulator
                if let Ok(command) = command_rx.recv() {
                    // Process the command
                    // For now, just log it
                    // println!("Received command: {:?}", command);
                    // log::info!("Received command: {:?}", command);

                    renderer.lock().unwrap().process_gpu_command(command);
                }
            }
        });
    }

    /// Calculates the viewport for rendering the VRAM content with aspect ratio correction.
    fn update_final_display_viewport(&mut self) {
        let window_width = self.surface_config.width as f32;
        let window_height = self.surface_config.height as f32;

        // Content dimensions from VRAM that we want to display
        let content_source_width;
        let content_source_height;

        if self.vram_debug {
            content_source_width = VRAM_WIDTH as f32;
            content_source_height = VRAM_HEIGHT as f32;
        } else {
            content_source_width = self.current_display_params.2 as f32;
            content_source_height = self.current_display_params.3 as f32;
        }

        if window_width == 0.0 || window_height == 0.0 || content_source_width == 0.0 || content_source_height == 0.0 {
            // Invalid dimensions, render to full window or nothing
            self.final_display_viewport_x = 0.0;
            self.final_display_viewport_y = 0.0;
            self.final_display_viewport_width = window_width;
            self.final_display_viewport_height = window_height;
            log::warn!("Updating final viewport with zero dimension. Window: {}x{}, Content: {}x{}",
                window_width, window_height, content_source_width, content_source_height);
            return;
        }

        let content_aspect_ratio = content_source_width / content_source_height;
        let window_aspect_ratio = window_width / window_height;

        let target_render_w: f32;
        let target_render_h: f32;

        if window_aspect_ratio > content_aspect_ratio {
            // Window is wider than content (pillarbox effect)
            target_render_h = window_height;
            target_render_w = target_render_h * content_aspect_ratio;
        } else {
            // Window is taller or same aspect as content (letterbox effect)
            target_render_w = window_width;
            target_render_h = target_render_w / content_aspect_ratio;
        }

        self.final_display_viewport_x = ((window_width - target_render_w) / 2.0).round();
        self.final_display_viewport_y = ((window_height - target_render_h) / 2.0).round();
        // Ensure viewport dimensions are at least 1.0 if target_render dimensions are positive
        self.final_display_viewport_width = target_render_w.round().max(1.0);
        self.final_display_viewport_height = target_render_h.round().max(1.0);

        log::debug!(
            "Final Viewport: X:{:.1} Y:{:.1} W:{:.1} H:{:.1} for Win:{}x{} ContentSrc:{}x{}",
            self.final_display_viewport_x, self.final_display_viewport_y,
            self.final_display_viewport_width, self.final_display_viewport_height,
            window_width, window_height, content_source_width, content_source_height
        );
    }
    
    /// Updates the texture coordinates for the screen quad based on current_display_source_params.
    fn update_screen_quad_tex_coords(&mut self) {
        let (mut src_x, mut src_y, mut src_w, mut src_h) = self.current_display_params;

        if self.vram_debug {
            src_x = 0;
            src_y = 0;
            src_w = VRAM_WIDTH;
            src_h = VRAM_HEIGHT;
            log::debug!("Debug mode: Setting screen quad UVs for full VRAM.");
        }

        if src_w == 0 || src_h == 0 { // Avoid division by zero if VRAM dimensions are zero
            log::warn!("Cannot update screen quad tex coords with zero VRAM source dimensions.");
            return;
        }

        let u_min: f32 = src_x as f32 / VRAM_WIDTH as f32;
        let v_min = src_y as f32 / VRAM_HEIGHT as f32;
        let u_max = (src_x + src_w) as f32 / VRAM_WIDTH as f32;
        let v_max = (src_y + src_h) as f32 / VRAM_HEIGHT as f32;

        let updated_quad_verts: [ScreenQuadVertex; 6] = [
            ScreenQuadVertex { position: [-1.0,  1.0], tex_coords: [u_min, v_min] }, // Top-Left Quad maps to Top-Left VRAM region
            ScreenQuadVertex { position: [-1.0, -1.0], tex_coords: [u_min, v_max] }, // Bottom-Left Quad maps to Bottom-Left VRAM region
            ScreenQuadVertex { position: [ 1.0,  1.0], tex_coords: [u_max, v_min] }, // Top-Right Quad maps to Top-Right VRAM region

            ScreenQuadVertex { position: [-1.0, -1.0], tex_coords: [u_min, v_max] },
            ScreenQuadVertex { position: [ 1.0, -1.0], tex_coords: [u_max, v_max] }, // Bottom-Right Quad maps to Bottom-Right VRAM region
            ScreenQuadVertex { position: [ 1.0,  1.0], tex_coords: [u_max, v_min] },
        ];
        self.queue.write_buffer(&self.screen_quad_vertex_buffer, 0, bytemuck::cast_slice(&updated_quad_verts));
        log::debug!("Screen quad tex coords updated for VRAM source: ({},{}) {}x{}", src_x, src_y, src_w, src_h);
    }

    pub fn resize_surface(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.surface_config.width = new_size.width;
            self.surface_config.height = new_size.height;
            self.surface.configure(&self.device, &self.surface_config);
            self.update_final_display_viewport();
        }
    }

    fn transform_command_vertex(&self, psx_v: &CommandPsxVertex, viewport_w: f32, viewport_h: f32) -> GpuVertex {
        let x_offset = psx_v.x as f32 + self.drawing_offset_x as f32;
        let y_offset = psx_v.y as f32 + self.drawing_offset_y as f32;

        // Convert to NDC for the current viewport (derived from drawing_area)
        // Viewport origin (drawing_area_x1, drawing_area_y1) maps to NDC (-1, 1) with Y-flip
        let ndc_x = (x_offset / viewport_w) * 2.0 - 1.0;
        let ndc_y = 1.0 - (y_offset / viewport_h) * 2.0; // Y flipped and scaled

        GpuVertex {
            position: [ndc_x, ndc_y],
            color: [
                psx_v.color.r as f32 / 255.0,
                psx_v.color.g as f32 / 255.0,
                psx_v.color.b as f32 / 255.0,
            ],
        }
    }

    pub fn process_gpu_command(&mut self, command: GpuCommand) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Renderer Command Encoder"),
        });
        
        // Vec to store WGPU-ready vertices for a single draw call or small batch
        let mut current_wgpu_vertices_batch: Vec<GpuVertex> = Vec::new();
        let mut textured_wgpu_vertices_batch: Vec<GpuTexturedVertex> = Vec::new();

        match command {
            GpuCommand::SetDrawingArea { x1, x2, y1, y2 } => {
                // Update the drawing area for the next draw command
                self.drawing_area_x1 = x1;
                self.drawing_area_y1 = y1;
                self.drawing_area_x2 = x2;
                self.drawing_area_y2 = y2;
            }
            GpuCommand::SetDrawingOffset { x, y } => {
                // Update the drawing offset for the next draw command
                self.drawing_offset_x = x;
                self.drawing_offset_y = y;
            }
            GpuCommand::DrawGouraudTriangle { vertices: psx_vertices } => {
                current_wgpu_vertices_batch.clear(); // Prepare for new vertices

                println!("Drawing Gouraud triangle with vertices: {:?}", psx_vertices);

                let da_x1 = self.drawing_area_x1;
                let da_y1 = self.drawing_area_y1;
                // Ensure width and height are at least 1 for valid viewport/scissor
                let da_w = (self.drawing_area_x2.saturating_sub(da_x1) + 1).max(1);
                let da_h = (self.drawing_area_y2.saturating_sub(da_y1) + 1).max(1);
                let viewport_w_for_norm = da_w as f32;
                let viewport_h_for_norm = da_h as f32;

                for psx_v in psx_vertices.iter() {
                    current_wgpu_vertices_batch.push(
                        self.transform_command_vertex(psx_v, viewport_w_for_norm, viewport_h_for_norm)
                    );
                }

                // Create a render pass specifically for this draw command (or a small batch)
                // The `pass` variable, and thus the mutable borrow of `encoder`,
                // is scoped tightly within this block.
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("VRAM Gouraud Triangle Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &self.vram_texture.view,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None, occlusion_query_set: None,
                });

                pass.set_viewport(da_x1 as f32, da_y1 as f32, da_w as f32, da_h as f32, 0.0, 1.0);
                pass.set_scissor_rect(da_x1 as u32, da_y1 as u32, da_w as u32, da_h as u32);
                pass.set_pipeline(&self.gouraud_pipeline);

                self.queue.write_buffer(&self.primitive_vertex_buffer, 0, bytemuck::cast_slice(&current_wgpu_vertices_batch));
                pass.set_vertex_buffer(0, self.primitive_vertex_buffer.slice(..(current_wgpu_vertices_batch.len() * std::mem::size_of::<GpuVertex>()) as wgpu::BufferAddress));
                pass.draw(0..current_wgpu_vertices_batch.len() as u32, 0..1);
            }
            GpuCommand::WriteToVram { x, y, w, h, pixel_data } => {
                // This command uses `self.queue.write_texture`, not a render pass on `encoder`.
                // It's important that any previous render pass created from `encoder` is finished.
                // Due to the new tight scoping of `pass` above, this is guaranteed.

                if w == 0 || h == 0 {
                    log::warn!("WriteToVram command with zero width or height. Ignoring.");
                    return;
                }

                if h == 1 {
                    // This is probably a CLUT
                    print!("CLUT write detected: {}x{} at ({},{}): ", w, h, x, y);
                    for i in 0..w {
                        let clut_color = pixel_data[i as usize];
                        print!("({:04x}), ", clut_color);
                    }
                    println!();
                } else {
                    println!("VRAM write detected: {}x{} at ({},{})", w, h, x, y);
                }

                if w > 0 && h > 0 {
                    self.queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &self.vram_texture.texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d { x: x as u32, y: y as u32, z: 0 },
                            aspect: wgpu::TextureAspect::All,
                        },
                        bytemuck::cast_slice(&pixel_data),
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(w as u32 * 2),
                            rows_per_image: Some(h as u32),
                        },
                        wgpu::Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
                    );
                }
            }
            GpuCommand::SetDisplayArea { x, y, w, h } => {
                let new_display_source_params = (
                    x as u32,
                    y as u32,
                    (w as u32).max(1),
                    (h as u32).max(1),
                );
                if new_display_source_params != self.current_display_params {
                    self.current_display_params = new_display_source_params;
                    self.update_screen_quad_tex_coords();
                    self.update_final_display_viewport();
                }
            }
            GpuCommand::DrawTexturedQuad {
                vertices: [v0, v1, v2, v3],
                uvs: [uv0, uv1, uv2, uv3],
                clut_attr, texpage_attr,
                modulation_color,
            } => {
                textured_wgpu_vertices_batch.clear();

                encoder.copy_texture_to_texture(
                    wgpu::TexelCopyTextureInfo { // Source from main VRAM
                        texture: &self.vram_texture.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyTextureInfo { // Destination to temporary texture
                        texture: &self.temporary_texture.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::Extent3d {
                        width: VRAM_WIDTH,
                        height: VRAM_HEIGHT,
                        depth_or_array_layers: 1,
                    },
                );

                let clut_vx = ((clut_attr & 0x3F) * 16) as u32; // X in VRAM words (pixels / 16 wide)
                let clut_vy = ((clut_attr >> 6) & 0x1FF) as u32; // Y in VRAM words (scanlines)

                // Decode TexPage Attributes (Simplified for 15-bit direct) ---
                // TX (bits 0-3): Texture Page X Base (N * 64 pixels)
                let tex_page_x_base = ((texpage_attr >> 0) & 0xF) as u32 * 64;
                // TY (bit 4): Texture Page Y Base (N * 256 pixels)
                let tex_page_y_base = ((texpage_attr >> 4) & 0x1) as u32 * 256;
                // TP (bits 7-8): Texture Color Mode.
                let tp_mode = ((texpage_attr >> 7) & 0x3) as u32;
                
                // let _texture_color_mode = (texpage_attr >> 7) & 0x3;
                // log::debug!("TexPage Attr: {:#06x} -> Base: ({}, {}), Mode: {}", texpage_attr, tex_page_x_base, tex_page_y_base, _texture_color_mode);

                // Prepare Vertices (Position and UVs) ---
                let psx_quad_vertices = [
                    (&v0, &uv0), (&v2, &uv2), (&v1, &uv1), // Triangle 1 (TL, BL, TR)
                    (&v1, &uv1), (&v2, &uv2), (&v3, &uv3), // Triangle 2 (TR, BL, BR)
                ];

                // print!("Drawing textured quad with TL: {:?} and uvs: {:?}. ", v0, [uv0, uv1, uv2, uv3]);
                // println!("TPX: {} TY: {} MODE: {} CLUT: ({}, {}) ", tex_page_x_base, tex_page_y_base, tp_mode, clut_vx, clut_vy);

                // Drawing area for viewport setup
                let da_x1 = self.drawing_area_x1;
                let da_y1 = self.drawing_area_y1;
                let da_w = (self.drawing_area_x2.saturating_sub(da_x1) + 1).max(1);
                let da_h = (self.drawing_area_y2.saturating_sub(da_y1) + 1).max(1);
                let viewport_w_for_norm = da_w as f32;
                let viewport_h_for_norm = da_h as f32;

                for (psx_prim_v, psx_uv) in psx_quad_vertices.iter() {
                    // Transform X,Y to NDC for current WGPU viewport
                    let transformed_pos_v = self.transform_command_vertex(
                        &CommandPsxVertex { x: psx_prim_v.x, y: psx_prim_v.y, color: PsxColor { r: 0, g: 0, b: 0 } },
                        viewport_w_for_norm, viewport_h_for_norm
                    );

                    // Calculate absolute UV in VRAM, then normalize for shader (0.0-1.0)
                    let abs_u_vram = tex_page_x_base as f32 + (psx_uv.u as f32) / 4.0;
                    let abs_v_vram = tex_page_y_base as f32 + psx_uv.v as f32;

                    let norm_u = abs_u_vram / VRAM_WIDTH as f32;
                    let norm_v = abs_v_vram / VRAM_HEIGHT as f32;
                    
                    textured_wgpu_vertices_batch.push(GpuTexturedVertex {
                        position: transformed_pos_v.position, // Already [f32; 2]
                        uv: [norm_u, norm_v],
                    });
                }

                // Prepare Uniforms ---
                // Modulation color: PS1's 0x80 (128) often means "1.0x multiplier".
                // So, we normalize R,G,B (0-255) to effectively 0.0-2.0 range for multiplication.
                let mod_r_factor = modulation_color.r as f32 / 128.0;
                let mod_g_factor = modulation_color.g as f32 / 128.0;
                let mod_b_factor = modulation_color.b as f32 / 128.0;

                let tex_page_x_base_words_val = ((texpage_attr >> 0) & 0xF) * 64; // This is VRAM X word offset of page
                let tex_page_y_base_words_val = ((texpage_attr >> 4) & 0x1) * 256; // VRAM Y word offset of page

                let uniforms = TexturedRectUniforms {
                    modulation_color: [mod_r_factor, mod_g_factor, mod_b_factor],
                    texture_mode: tp_mode,
                    clut_vram_base_x: clut_vx,
                    clut_vram_base_y: clut_vy,
                    tex_page_base_x_words: tex_page_x_base_words_val as u32,
                    tex_page_base_y_words: tex_page_y_base_words_val as u32,
                    _padding: [0; 0]
                };
                self.queue.write_buffer(&self.textured_rect_uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

                // --- 4. Setup Render Pass and Draw ---
                if !textured_wgpu_vertices_batch.is_empty() {
                    let uniform_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("Textured Rect Uniform Bind Group"),
                        layout: &self.textured_rect_uniform_bgl,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: self.textured_rect_uniform_buffer.as_entire_binding(),
                            }
                        ],
                    });

                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("VRAM Textured Rectangle Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &self.vram_texture.view,
                            resolve_target: None,
                            ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None, occlusion_query_set: None,
                    });
                    pass.set_pipeline(&self.textured_rect_pipeline);
                    pass.set_viewport(da_x1 as f32, da_y1 as f32, da_w as f32, da_h as f32, 0.0, 1.0);
                    pass.set_scissor_rect(da_x1 as u32, da_y1 as u32, da_w as u32, da_h as u32);
                    
                    pass.set_bind_group(0, &self.temporary_texture_bg, &[]); // Group 0: VRAM texture/sampler
                    pass.set_bind_group(1, &uniform_bind_group, &[]);   // Group 1: Uniforms

                    self.queue.write_buffer(&self.primitive_vertex_buffer, 0, bytemuck::cast_slice(&textured_wgpu_vertices_batch));
                    pass.set_vertex_buffer(0, self.primitive_vertex_buffer.slice(..(textured_wgpu_vertices_batch.len() * std::mem::size_of::<GpuTexturedVertex>()) as wgpu::BufferAddress));
                    pass.draw(0..textured_wgpu_vertices_batch.len() as u32, 0..1);
                }
            }
        }

        // All RenderPass objects created from `encoder` are now definitely dropped.
        self.queue.submit(std::iter::once(encoder.finish()));

        self.update_final_display_viewport();
    }

    pub fn present_display(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output_surface_texture = self.surface.get_current_texture()?;
        let output_surface_view = output_surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("VRAM to Screen Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render VRAM to Window"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_surface_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None, occlusion_query_set: None,
            });

            // Set the viewport to the calculated 4:3 area
            render_pass.set_viewport(
                self.final_display_viewport_x,
                self.final_display_viewport_y,
                self.final_display_viewport_width,
                self.final_display_viewport_height,
                0.0, // min_depth
                1.0, // max_depth
            );
            
            render_pass.set_pipeline(&self.vram_to_screen_pipeline);
            render_pass.set_bind_group(0, &self.vram_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.screen_quad_vertex_buffer.slice(..));
            render_pass.draw(0..6, 0..1); // SCREEN_QUAD_VERTICES always has 6 vertices
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        output_surface_texture.present();
        Ok(())
    }
}