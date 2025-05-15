use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use wgpu::util::DeviceExt;
use winit::window::Window;
use winit::dpi::PhysicalSize;

use crate::vertex_data::{
    self, GpuVertex, ScreenQuadVertex, SCREEN_QUAD_VERTICES, VRAM_WIDTH, VRAM_HEIGHT, VRAM_FORMAT,
};
use crate::texture::TextureRenderTarget;
use crate::render_pipelines;
use crate::gpu_command::{GpuCommand, PsxVertex as CommandPsxVertex}; // Alias to avoid conflict
use crate::system::PsxGpuRegisters; // To read GPU state

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
    vram_to_screen_pipeline: wgpu::RenderPipeline,
    // Resources for VRAM display
    screen_quad_vertex_buffer: wgpu::Buffer,
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

    command_rx: crossbeam_channel::Receiver<GpuCommand>,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> (Arc<Mutex<Self>>, crossbeam_channel::Sender<GpuCommand>) {
        let window_size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
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
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
            Some("VRAM Texture"),
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

        let (command_tx, command_rx) = crossbeam_channel::unbounded();

        let mut renderer = Self {
            window, surface, device, queue, surface_config,
            vram_texture,
            gouraud_pipeline, vram_to_screen_pipeline,
            screen_quad_vertex_buffer, vram_bind_group,
            primitive_vertex_buffer,
            current_display_params: initial_display_params,
            final_display_viewport_x: 0.0,
            final_display_viewport_y: 0.0,
            final_display_viewport_width: window_size.width as f32,
            final_display_viewport_height: window_size.height as f32,

            command_rx
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

                    renderer.lock().unwrap().process_gpu_command(&PsxGpuRegisters::default(), command);
                }
            }
        });
    }

    /// Calculates the viewport for rendering the VRAM content with aspect ratio correction.
    fn update_final_display_viewport(&mut self) {
        let window_width = self.surface_config.width as f32;
        let window_height = self.surface_config.height as f32;

        // Content dimensions from VRAM that we want to display
        let content_source_width = self.current_display_params.2 as f32;
        let content_source_height = self.current_display_params.3 as f32;

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

        let mut target_render_w: f32;
        let mut target_render_h: f32;

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
        let (src_x, src_y, src_w, src_h) = self.current_display_params;
        if src_w == 0 || src_h == 0 { // Avoid division by zero if VRAM dimensions are zero
            log::warn!("Cannot update screen quad tex coords with zero VRAM source dimensions.");
            return;
        }

        let u_min = src_x as f32 / VRAM_WIDTH as f32;
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

    fn transform_command_vertex(&self, psx_v: &CommandPsxVertex, regs: &PsxGpuRegisters, viewport_w: f32, viewport_h: f32) -> GpuVertex {
        let x_offset = psx_v.x as f32 + regs.drawing_offset_x as f32;
        let y_offset = psx_v.y as f32 + regs.drawing_offset_y as f32;

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

    pub fn process_gpu_command(&mut self, gpu_regs: &PsxGpuRegisters, command: GpuCommand) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Renderer Command Encoder"),
        });
        
        // Vec to store WGPU-ready vertices for a single draw call or small batch
        let mut current_wgpu_vertices_batch: Vec<GpuVertex> = Vec::new();

        // Handle display area updates (affects screen_quad_vertex_buffer for present_display)
        let new_display_params = (
            gpu_regs.display_vram_x as u32,
            gpu_regs.display_vram_y as u32,
            gpu_regs.display_width,
            gpu_regs.display_height,
        );

        if new_display_params != self.current_display_params && 
           new_display_params.2 > 0 && new_display_params.3 > 0 { // Ensure valid width/height
            self.current_display_params = new_display_params;
            log::info!("Display area updated: VRAM@({},{}) {}x{}. Updating screen quad.",
                self.current_display_params.0, self.current_display_params.1,
                self.current_display_params.2, self.current_display_params.3);

            let (du_min, dv_min, du_max, dv_max) = (
                self.current_display_params.0 as f32 / VRAM_WIDTH as f32,
                self.current_display_params.1 as f32 / VRAM_HEIGHT as f32,
                (self.current_display_params.0 + self.current_display_params.2) as f32 / VRAM_WIDTH as f32,
                (self.current_display_params.1 + self.current_display_params.3) as f32 / VRAM_HEIGHT as f32,
            );
            let updated_quad_verts: [ScreenQuadVertex; 6] = [
                ScreenQuadVertex { position: [-1.0,  1.0], tex_coords: [du_min, dv_min] },
                ScreenQuadVertex { position: [-1.0, -1.0], tex_coords: [du_min, dv_max] },
                ScreenQuadVertex { position: [ 1.0,  1.0], tex_coords: [du_max, dv_min] },
                ScreenQuadVertex { position: [-1.0, -1.0], tex_coords: [du_min, dv_max] },
                ScreenQuadVertex { position: [ 1.0, -1.0], tex_coords: [du_max, dv_max] },
                ScreenQuadVertex { position: [ 1.0,  1.0], tex_coords: [du_max, dv_min] },
            ];
            self.update_screen_quad_tex_coords(); // Update what part of VRAM is sampled
            self.update_final_display_viewport(); // Update how it's presented in the window
            self.queue.write_buffer(&self.screen_quad_vertex_buffer, 0, bytemuck::cast_slice(&updated_quad_verts));
        }

        match command {
            GpuCommand::SetDrawingArea { .. } | GpuCommand::SetDrawingOffset { .. } => {
                // These commands modify `gpu_regs`. The `gpu_regs` parameter is passed by the
                // Emulator for each call to `process_gpu_commands`. When a drawing command
                // needs these values, it will read the latest from the `gpu_regs` argument.
                // No direct WGPU encoder action here; viewport/scissor are set per render pass.
            }
            GpuCommand::DrawGouraudTriangle { vertices: psx_vertices } => {
                current_wgpu_vertices_batch.clear(); // Prepare for new vertices

                let da_x1 = gpu_regs.drawing_area_x1;
                let da_y1 = gpu_regs.drawing_area_y1;
                // Ensure width and height are at least 1 for valid viewport/scissor
                let da_w = (gpu_regs.drawing_area_x2.saturating_sub(da_x1) + 1).max(1);
                let da_h = (gpu_regs.drawing_area_y2.saturating_sub(da_y1) + 1).max(1);
                let viewport_w_for_norm = da_w as f32;
                let viewport_h_for_norm = da_h as f32;

                for psx_v in psx_vertices.iter() {
                    current_wgpu_vertices_batch.push(
                        self.transform_command_vertex(psx_v, gpu_regs, viewport_w_for_norm, viewport_h_for_norm)
                    );
                }

                if !current_wgpu_vertices_batch.is_empty() {
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

                    // `pass` is dropped here, releasing the mutable borrow on `encoder`.
                }
            }
            GpuCommand::WriteToVram { x, y, w, h, pixel_data } => {
                // This command uses `self.queue.write_texture`, not a render pass on `encoder`.
                // It's important that any previous render pass created from `encoder` is finished.
                // Due to the new tight scoping of `pass` above, this is guaranteed.

                let mut rgba_data: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
                if pixel_data.is_empty() && w > 0 && h > 0 {
                    rgba_data.resize((w * h * 4) as usize, 0); // Fill with black
                } else {
                    for &pixel16 in &pixel_data {
                        // Assuming BGR555 format for pixel16 (common for PS1)
                        // B = bits 0-4, G = bits 5-9, R = bits 10-14
                        let b5 = (pixel16 & 0x001F) as u8;
                        let g5 = ((pixel16 >> 5) & 0x001F) as u8;
                        let r5 = ((pixel16 >> 10) & 0x001F) as u8;

                        // Convert 5-bit components to 8-bit
                        // (val5 * 255 + 15) / 31 is a good approximation, or bit replication: (val5 << 3) | (val5 >> 2)
                        let r8 = (r5 << 3) | (r5 >> 2);
                        let g8 = (g5 << 3) | (g5 >> 2);
                        let b8 = (b5 << 3) | (b5 >> 2);
                        
                        rgba_data.push(r8);
                        rgba_data.push(g8);
                        rgba_data.push(b8);
                        rgba_data.push(255); // Alpha
                    }
                }
                
                if w > 0 && h > 0 && !rgba_data.is_empty() {
                    self.queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &self.vram_texture.texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d { x: x as u32, y: y as u32, z: 0 },
                            aspect: wgpu::TextureAspect::All,
                        },
                        &rgba_data,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(w as u32 * 4), // 4 bytes per RGBA8 pixel
                            rows_per_image: Some(h as u32),
                        },
                        wgpu::Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
                    );
                }
            }
            GpuCommand::SetDisplayArea { .. } => {
                // This updates `gpu_regs` on the System side.
                // The `Renderer` checks `gpu_regs.display_...` at the start of this function
                // (or before `present_display`) to update its own `current_display_params`
                // and the `screen_quad_vertex_buffer` if necessary. No direct encoder use here.
            }
            GpuCommand::DrawTexturedQuad { .. } => {
                log::warn!("DrawTexturedQuad not yet implemented in Renderer.");
                // Similar to DrawGouraudTriangle, this would start its own render pass
                // using a textured pipeline.
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