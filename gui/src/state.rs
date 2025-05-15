use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::vertex_data::{
    self, SCREEN_QUAD_VERTICES, SCREEN_TEXTURE_FORMAT, SCREEN_TEXTURE_HEIGHT, SCREEN_TEXTURE_WIDTH, TRIANGLE_VERTICES
};
use crate::texture::TextureRenderTarget;
use crate::render_pipelines;

// The main state of our WGPU application.
// Encapsulates all WGPU objects and resources needed for rendering.
pub struct State {
    // --- WGPU Core Components ---
    surface: wgpu::Surface<'static>, // The drawing surface linked to the window.
    device: wgpu::Device,         // The logical GPU device.
    queue: wgpu::Queue,           // Command queue for submitting work to the GPU.
    config: wgpu::SurfaceConfiguration, // Configuration for the drawing surface.
    size: winit::dpi::PhysicalSize<u32>, // Current physical size of the window.

    // Handle to the winit Window. Wrapped in Arc to allow sharing if needed.
    window: Arc<Window>,

    // --- Resources for Pass 1: Rendering the Triangle to Offscreen Texture ---
    triangle_render_pipeline: wgpu::RenderPipeline, // Pipeline for drawing the triangle.
    triangle_vertex_buffer: wgpu::Buffer,           // GPU buffer holding triangle vertex data.

    // --- Offscreen "Screen" Texture Resources ---
    // This is our intermediate render target.
    screen_render_target: TextureRenderTarget,

    // --- Resources for Pass 2: Rendering the "Screen" Texture to the Window's Surface ---
    screen_quad_render_pipeline: wgpu::RenderPipeline, // Pipeline for drawing the textured quad.
    screen_quad_vertex_buffer: wgpu::Buffer, // GPU buffer holding quad vertex data.
    // screen_quad_bind_group_layout: wgpu::BindGroupLayout, // Layout for screen texture bindings. (Owned by pipeline now)
    screen_quad_bind_group: wgpu::BindGroup, // Bind group containing the screen texture and sampler.
}

impl State {
    // Asynchronously creates a new State instance.
    // This involves setting up WGPU, creating resources, and compiling pipelines.
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        // --- Initialize WGPU ---
        // 1. Create an Instance: Entry point to WGPU.
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(), // Use all available graphics backends (Vulkan, Metal, DX12, etc.).
            ..Default::default()
        });

        // 2. Create a Surface: A platform-specific surface to draw on (from the winit window).
        // The `window` argument is wrapped in `Arc` by the caller.
        // The surface needs to live as long as the window that created it.
        // `State` owns the window so this is safe.
        let surface = instance.create_surface(window.clone()).expect("Failed to create surface");

        // 3. Request an Adapter: Represents a physical GPU.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(), // Default power profile.
                compatible_surface: Some(&surface), // Find an adapter compatible with our surface.
                force_fallback_adapter: false, // Don't force a software fallback if a hardware GPU is available.
            })
            .await
            .expect("Failed to find a suitable adapter");

        // 4. Request a Device and Queue:
        //    - Device: Logical connection to the GPU, used for creating resources.
        //    - Queue: Used to submit command buffers to the GPU.
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Primary Device"),
                    required_features: wgpu::Features::empty(), // No special features needed for this example.
                    required_limits: wgpu::Limits::default(), // Default resource limits.
                },
                None, // Trace path for debugging, not used here.
            )
            .await
            .expect("Failed to create device and queue");

        // 5. Configure the Surface:
        //    Define how the surface will be used and its properties (format, size, etc.).
        let surface_caps = surface.get_capabilities(&adapter);
        // Choose an sRGB format if available, otherwise the first supported format.
        // sRGB is generally preferred for color accuracy on displays.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, // The surface will be used as a render target.
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0], // VSync mode (Fifo is common).
            alpha_mode: surface_caps.alpha_modes[0], // How to handle alpha compositing.
            view_formats: vec![], // For texture views with different formats, not needed here.
            desired_maximum_frame_latency: 2, // Standard frame latency.
        };
        surface.configure(&device, &config);

        // --- Create the Offscreen "Screen" Texture Render Target ---
        let screen_render_target = TextureRenderTarget::new(
            &device,
            SCREEN_TEXTURE_WIDTH,
            SCREEN_TEXTURE_HEIGHT,
            SCREEN_TEXTURE_FORMAT,
            // This texture will be rendered to (RENDER_ATTACHMENT)
            // and then sampled from (TEXTURE_BINDING) in the second pass.
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            Some("Screen Render Target Texture"),
        );

        // --- Create Resources for Pass 1: Triangle Rendering ---
        // Vertex buffer for the triangle.
        let triangle_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Triangle Vertex Buffer"),
            contents: bytemuck::cast_slice(TRIANGLE_VERTICES), // Vertex data.
            usage: wgpu::BufferUsages::VERTEX, // This buffer will be used as a vertex buffer.
        });

        // Render pipeline for the triangle (renders to the offscreen texture).
        let triangle_render_pipeline = render_pipelines::create_triangle_render_pipeline(
            &device,
            screen_render_target.format, // Target format is our offscreen texture's format.
        );

        // --- Create Resources for Pass 2: Screen Quad Rendering ---
        // Vertex buffer for the full-screen quad.
        let screen_quad_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Screen Quad Vertex Buffer"),
            contents: bytemuck::cast_slice(SCREEN_QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Create bind group layout and bind group for the screen texture.
        // These resources allow the screen quad shader to access the offscreen texture.
        let (screen_quad_bind_group_layout, screen_quad_bind_group) =
            render_pipelines::create_screen_texture_bind_group_resources(
                &device,
                &screen_render_target,
            );
        
        // Render pipeline for the screen quad (renders the offscreen texture to the window surface).
        let screen_quad_render_pipeline = render_pipelines::create_screen_quad_render_pipeline(
            &device,
            config.format, // Target format is the window surface's format.
            &screen_quad_bind_group_layout, // Pass the layout for screen texture.
        );


        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            triangle_render_pipeline,
            triangle_vertex_buffer,
            screen_render_target,
            screen_quad_render_pipeline,
            screen_quad_vertex_buffer,
            screen_quad_bind_group,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    // Handles window resize events.
    // Reconfigures the surface to match the new window size.
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            log::info!("Surface reconfigured to: {}x{}", new_size.width, new_size.height);
        } else {
            log::warn!("Window resized to zero dimensions, skipping surface reconfiguration.");
        }
        // Note: The offscreen texture (screen_render_target) is NOT resized here.
        // It remains at its fixed dimensions (SCREEN_TEXTURE_WIDTH, SCREEN_TEXTURE_HEIGHT).
        // If you wanted it to resize with the window, you'd need to:
        // 1. Recreate self.screen_render_target.texture, .view, and potentially .sampler.
        // 2. Recreate self.screen_quad_bind_group because it references the old .view and .sampler.
    }

    // Placeholder for handling input events.
    // Returns true if the event was consumed, false otherwise.
    #[allow(unused_variables)] // Temporarily allow unused 'event'
    pub fn input(&mut self, event: &winit::event::WindowEvent) -> bool {
        // Example:
        // match event {
        //     winit::event::WindowEvent::KeyboardInput { input, .. } => {
        //         if input.state == winit::event::ElementState::Pressed {
        //             // handle key press
        //             return true;
        //         }
        //     }
        //     _ => {}
        // }
        false // Event not consumed by this handler.
    }

    // Placeholder for update logic (e.g., animations, physics).
    // Called once per frame before rendering.
    pub fn update(&mut self) {
        // Example:
        // self.camera.update();
        // self.object_transform = self.object_transform * cgmath::Matrix4::from_angle_y(cgmath::Deg(0.5));
    }

    // Renders a single frame.
    // This involves two render passes:
    // 1. Render the triangle to the offscreen "screen" texture.
    // 2. Render the "screen" texture (via a quad) to the window's surface.
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // 1. Get the next available texture from the swap chain to draw on.
        let output_surface_texture = self.surface.get_current_texture()?;
        
        // 2. Create a TextureView for this output texture.
        //    This view describes how the render pass will access the texture.
        let output_surface_view = output_surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // 3. Create a CommandEncoder to record GPU commands.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Main Render Encoder"),
            });

        // --- Pass 1: Render Triangle to the Offscreen "Screen" Texture ---
        { // Scoped to ensure `render_pass_to_texture` is dropped, finishing the pass.
            let mut render_pass_to_texture = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass to Screen Texture"),
                // Define where the color output of this pass will go.
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.screen_render_target.view, // Target is our offscreen texture's view.
                    resolve_target: None, // No multisampling, so no resolve target needed.
                    ops: wgpu::Operations {
                        // How to handle the texture at the start of the pass:
                        load: wgpu::LoadOp::Clear(wgpu::Color { // Clear it with a specific color.
                            r: 0.05, g: 0.05, b: 0.1, a: 1.0, // Dark blueish background for the offscreen texture.
                        }),
                        // How to handle the texture at the end of the pass:
                        store: wgpu::StoreOp::Store, // Store the rendered result.
                    },
                })],
                depth_stencil_attachment: None, // No depth/stencil buffer.
                timestamp_writes: None, // No GPU timestamps.
                occlusion_query_set: None, // No occlusion queries.
            });

            // The viewport remaps Normalized Device Coordinates (NDC, -1 to 1)
            // to a specific pixel rectangle on the render target. Here, NDC
            // will map to the top-left 640x480 area of our 1024x512 texture.
            render_pass_to_texture.set_viewport(
                0.0, 0.0,
                vertex_data::VISIBLE_REGION_WIDTH as f32, vertex_data::VISIBLE_REGION_HEIGHT as f32,
                0.0, 1.0, // Z range
            );

            // The scissor rectangle clips all rendering to this specified pixel
            // rectangle. Primitives (or parts of them) outside this rect are
            // discarded.
            render_pass_to_texture.set_scissor_rect(
                0, 0,
                vertex_data::VISIBLE_REGION_WIDTH, vertex_data::VISIBLE_REGION_HEIGHT,
            );

            // Set the pipeline for this pass.
            render_pass_to_texture.set_pipeline(&self.triangle_render_pipeline);
            // Set the vertex buffer for the triangle. Slot 0.
            render_pass_to_texture.set_vertex_buffer(0, self.triangle_vertex_buffer.slice(..));
            // Draw the triangle: 3 vertices, 1 instance.
            render_pass_to_texture.draw(0..TRIANGLE_VERTICES.len() as u32, 0..1);
        } // `render_pass_to_texture` is dropped here, finalizing this render pass.


        // --- Pass 2: Render the "Screen" Texture (via a Quad) to the Window's Surface ---
        { // Scoped for the second render pass.
            let mut render_pass_to_surface = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass to Window Surface"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_surface_view, // Target is the window's current surface texture view.
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { // Clear the window with a different color.
                            r: 0.1, g: 0.2, b: 0.3, a: 1.0, // Default background color for the window.
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Set the pipeline for drawing the textured quad.
            render_pass_to_surface.set_pipeline(&self.screen_quad_render_pipeline);
            // Set the bind group containing the screen texture and sampler. Group 0.
            render_pass_to_surface.set_bind_group(0, &self.screen_quad_bind_group, &[]);
            // Set the vertex buffer for the quad. Slot 0.
            render_pass_to_surface.set_vertex_buffer(0, self.screen_quad_vertex_buffer.slice(..));
            // Draw the quad: 6 vertices (2 triangles), 1 instance.
            render_pass_to_surface.draw(0..SCREEN_QUAD_VERTICES.len() as u32, 0..1);
        } // `render_pass_to_surface` is dropped here.

        // 4. Submit the recorded commands to the GPU queue for execution.
        self.queue.submit(std::iter::once(encoder.finish()));
        
        // 5. Present the rendered texture to the screen.
        output_surface_texture.present();

        Ok(())
    }
}
