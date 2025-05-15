mod renderer;
mod texture;
mod render_pipelines;
mod vertex_data;
pub mod gpu_command;

use std::sync::{Arc, Mutex};
use winit::{
    dpi::LogicalSize, event::Event, event_loop::{ControlFlow, EventLoop}, window::WindowBuilder
};

pub use crate::gpu_command::GpuCommand;
pub use crate::gpu_command::PsxColor;
pub use crate::gpu_command::PsxVertex;

pub struct EmuGui {
    window: Arc<winit::window::Window>,
    renderer: Arc<Mutex<renderer::Renderer>>,
}

impl EmuGui {
    pub async fn new() -> (Self, EventLoop<()>, crossbeam_channel::Sender<gpu_command::GpuCommand>) {
        let event_loop = EventLoop::new().expect("Failed to create event loop");
        let window = Arc::new(WindowBuilder::new()
            .with_title("PS1 Emulator")
            .with_inner_size(LogicalSize::new(
                crate::vertex_data::DEFAULT_DISPLAY_WIDTH,
                crate::vertex_data::DEFAULT_DISPLAY_HEIGHT
            ))
            .build(&event_loop)
            .unwrap());

        let (renderer, command_tx) = renderer::Renderer::new(window.clone()).await;

        (Self { window, renderer }, event_loop, command_tx)
    }

    pub fn run(&mut self, event_loop: EventLoop<()>) {
        renderer::Renderer::start_command_thread(self.renderer.clone());

        event_loop.run(move |event, elwt| {
            match event {
                Event::WindowEvent { event: winit_event, window_id, .. } if window_id == self.window.id() => {
                    match winit_event {
                        winit::event::WindowEvent::CloseRequested => {  
                            elwt.exit();
                        }
                        winit::event::WindowEvent::Resized(physical_size) => {
                            self.renderer.lock().unwrap().resize_surface(physical_size);
                        }
                        winit::event::WindowEvent::RedrawRequested => {
                            self.renderer.lock().unwrap().present_display().expect("Failed to present display");
                        }
                        _ => {}
                    }
                }
                Event::AboutToWait => {
                    match self.renderer.lock().unwrap().present_display() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            log::warn!("Surface lost, reconfiguring.");
                            self.renderer.lock().unwrap().resize_surface(self.window.inner_size());
                        },
                        // The system is out of memory, we should probably quit
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            log::error!("OutOfMemory error, exiting.");
                            elwt.exit();
                        }
                        Err(e) => {
                            // These should be recoverable in the next frame
                            // log::error!("Failed to present display: {}", e);
                        }
                    }

                    // 100 FPS!
                    elwt.set_control_flow(ControlFlow::WaitUntil(std::time::Instant::now() + std::time::Duration::from_millis(10)));
                }
                Event::LoopExiting => {
                    log::info!("Emulator loop exiting.");
                }
                _ => {}
            }
        }).expect("Event loop error");
    }
}
