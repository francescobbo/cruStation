mod state;
mod texture;
mod render_pipelines;
mod vertex_data;
// Shaders are included via include_str! in render_pipelines.rs, so no mod shaders; here.

use std::sync::Arc;
use winit::{
    event::{Event, WindowEvent, KeyEvent, ElementState},
    event_loop::{EventLoop, ControlFlow},
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
    dpi::LogicalSize,
};
use log::info;

// The main asynchronous function that sets up and runs the application.
async fn run() {
    // Initialize the logger (e.g., env_logger) to see WGPU logs and our own.
    // You can set the RUST_LOG environment variable (e.g., RUST_LOG=info,wgpu_core=warn)
    env_logger::init();
    info!("Application starting...");

    // 1. Create an EventLoop: Handles window events and manages the application lifecycle.
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    
    // 2. Create a Window: The OS window where graphics will be displayed.
    let window = Arc::new(WindowBuilder::new() // Wrap in Arc for sharing with State
        .with_title("WGPU Offscreen Render (Refactored)")
        .with_inner_size(LogicalSize::new(800, 600)) // Initial window size.
        .build(&event_loop)
        .expect("Failed to build window"));

    // 3. Create the Application State:
    //    This initializes WGPU, loads shaders, creates pipelines, buffers, etc.
    let mut app_state = state::State::new(window.clone()).await;

    // Flag to track if the surface is currently configured and ready for rendering.
    // This is useful for handling cases like minimization where the surface might be 0x0.
    let mut surface_configured_for_rendering = true;

    // 4. Run the Event Loop:
    //    This is the heart of the application, processing events and redrawing the window.
    event_loop.run(move |event, elwt| { // elwt is EventLoopWindowTarget
        match event {
            // --- Window Events ---
            Event::WindowEvent {
                ref event,      // The specific window event.
                window_id,      // ID of the window that generated the event.
            } if window_id == app_state.window().id() => { // Process events only for our main window.
                
                // First, give the State a chance to handle input if it wants to.
                if !app_state.input(event) {
                    // If State didn't consume the event, process common window events here.
                    match event {
                        // Close requested (e.g., user clicked the 'X' button).
                        WindowEvent::CloseRequested
                        // Or, Escape key pressed.
                        | WindowEvent::KeyboardInput {
                            event: KeyEvent {
                                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                                    state: ElementState::Pressed, // Only on key press, not release.
                                    .. // Ignore other fields like logical_key, repeat.
                                },
                            .. // Ignore other fields of KeyboardInput.
                        } => {
                            info!("Exit requested.");
                            elwt.exit(); // Tell the event loop to stop.
                        }

                        // Window resized.
                        WindowEvent::Resized(physical_size) => {
                            info!("Window resized to: {:?}", physical_size);
                            if physical_size.width == 0 || physical_size.height == 0 {
                                // Window is likely minimized, surface cannot be configured.
                                surface_configured_for_rendering = false;
                                log::warn!("Window minimized or zero size, pausing rendering.");
                            } else {
                                surface_configured_for_rendering = true;
                                app_state.resize(*physical_size);
                            }
                        }
                        
                        // Window needs to be redrawn.
                        // This event is typically triggered by `window.request_redraw()` or by the OS.
                        WindowEvent::RedrawRequested => {
                           if !surface_configured_for_rendering {
                                // Skip rendering if surface is not valid (e.g., minimized).
                                return;
                           }
                           
                            // 1. Update application state (e.g., animations, logic).
                            app_state.update();
                            
                            // 2. Render the frame.
                            match app_state.render() {
                                Ok(_) => {} // Successfully rendered.
                                // --- Handle Surface Errors ---
                                // Surface lost: Needs to be reconfigured.
                                Err(wgpu::SurfaceError::Lost) => {
                                    log::warn!("Surface lost! Reconfiguring...");
                                    app_state.resize(app_state.window().inner_size()); // Use current window size.
                                }
                                // Out of memory: Critical error, usually means we should exit.
                                Err(wgpu::SurfaceError::OutOfMemory) => {
                                    log::error!("WGPU OutOfMemory error! Exiting.");
                                    elwt.exit();
                                }
                                // Other errors (Outdated, Timeout): These are usually recoverable by the next frame.
                                Err(e) => eprintln!("Unhandled WGPU surface error: {:?}", e),
                            }
                        }
                        _ => {} // Ignore other window events for now.
                    }
                }
            }
            // --- Application Lifecycle Events ---
            // This event is sent when the event loop is about to block and wait for new events.
            // It's a good place to request a redraw if your application needs to render continuously
            // or if state has changed that requires a new frame.
            Event::AboutToWait => {
                // Explicitly request a redraw. This ensures `WindowEvent::RedrawRequested` is emitted.
                app_state.window().request_redraw();
            }

            _ => {} // Ignore other event types (e.g., device events).
        }
    }).expect("Event loop error");
}

// Entry point of the application.
fn main() {
    // `pollster::block_on` runs an async function to completion on the current thread.
    // This is a simple way to execute the async `run` function from the synchronous `main`.
    pollster::block_on(run());
}
