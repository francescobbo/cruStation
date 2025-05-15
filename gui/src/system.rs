use std::collections::VecDeque;
use crate::gpu_command::GpuCommand; // Assuming GpuCommand is the communication medium

// Represents the internal state of the PS1 GPU, as seen by the PS1 CPU.
// This is NOT WGPU state. It's pure data representing the target hardware.
#[derive(Debug, Clone)]
pub struct PsxGpuRegisters {
    // Drawing environment
    pub drawing_area_x1: u16,
    pub drawing_area_y1: u16,
    pub drawing_area_x2: u16,
    pub drawing_area_y2: u16,
    pub drawing_offset_x: i16,
    pub drawing_offset_y: i16,
    pub dither_enabled: bool,
    // Texture window, not used yet
    // pub texture_window_x_mask: u8, ...

    // Display environment
    pub display_vram_x: u16, // Top-left X of the display area in VRAM
    pub display_vram_y: u16, // Top-left Y
    pub display_horiz_start: u16, // Output signal timing, affects positioning
    pub display_horiz_end: u16,
    pub display_vert_start: u16,
    pub display_vert_end: u16,
    pub display_width: u32, // Actual width being output (derived from h_start/end)
    pub display_height: u32, // Actual height (derived from v_start/end)

    // Other status like texture page, CLUT, semi-transparency mode, etc.
    // For now, keep it simple.
}

impl Default for PsxGpuRegisters {
    fn default() -> Self {
        Self {
            drawing_area_x1: 0,
            drawing_area_y1: 0,
            drawing_area_x2: crate::vertex_data::VRAM_WIDTH as u16 - 1,
            drawing_area_y2: crate::vertex_data::VRAM_HEIGHT as u16 - 1,
            drawing_offset_x: 0,
            drawing_offset_y: 0,
            dither_enabled: false, // Typically true on PS1 for 15-bit color

            display_vram_x: crate::vertex_data::DEFAULT_DISPLAY_X_IN_VRAM as u16,
            display_vram_y: crate::vertex_data::DEFAULT_DISPLAY_Y_IN_VRAM as u16,
            display_horiz_start: 0, // Placeholder values
            display_horiz_end: 2560, // Common for 320 width modes after dotclock scaling
            display_vert_start: 0,
            display_vert_end: 240, // Common for NTSC
            display_width: crate::vertex_data::DEFAULT_DISPLAY_WIDTH,
            display_height: crate::vertex_data::DEFAULT_DISPLAY_HEIGHT,
        }
    }
}


pub struct System {
    // Placeholder for CPU, Memory, DMA etc.
    // pub cpu: Cpu,
    // pub memory: Memory,
    pub cycle_counter: u64,
    pub gpu_registers: PsxGpuRegisters,
    pub pending_gpu_commands: VecDeque<GpuCommand>,
    // For testing, count frames or commands
    pub frames_simulated: u32,
}

impl System {
    pub fn new() -> Self {
        Self {
            cycle_counter: 0,
            gpu_registers: PsxGpuRegisters::default(),
            pending_gpu_commands: VecDeque::new(),
            frames_simulated: 0,
        }
    }

    /// Simulate running the PS1 system (CPU, DMA, timers) for roughly one frame's worth of cycles.
    pub fn run_cycles_for_frame(&mut self) {
        // In a real emulator, this would be driven by precise cycle counts for CPU, DMA, etc.
        // For now, let's just simulate some GPU command generation each "frame".
        self.cycle_counter += 33_000_000 / 60; // Rough NTSC cycles per frame

        // --- Example: Simulate generating some GPU commands ---
        // This logic would normally come from the emulated CPU executing game code.
        if self.frames_simulated == 0 { // Only on the first frame for this example
            self.pending_gpu_commands.push_back(GpuCommand::SetDrawingArea {
                x1: 10, y1: 10, x2: 630, y2: 470,
            });
            self.pending_gpu_commands.push_back(GpuCommand::SetDrawingOffset { x: 0, y: 0 });

            let y_offset = (self.frames_simulated % 200) as i16; // Move triangle slightly

            self.pending_gpu_commands.push_back(GpuCommand::DrawGouraudTriangle {
                vertices: [
                    crate::gpu_command::PsxVertex { x: 160, y: 100 + y_offset, color: crate::gpu_command::PsxColor { r: 255, g: 0, b: 255 } },
                    crate::gpu_command::PsxVertex { x: 80,  y: 200 + y_offset, color: crate::gpu_command::PsxColor { r: 255, g: 255, b: 0 } },
                    crate::gpu_command::PsxVertex { x: 240, y: 200 + y_offset, color: crate::gpu_command::PsxColor { r: 0, g: 255, b: 255 } },
                ],
            });
             // Simulate a VRAM write (e.g. texture upload)
            let dummy_texture_data: Vec<u16> = vec![0x7FFF; 32 * 32]; // White 16-bit pixels
            self.pending_gpu_commands.push_back(GpuCommand::WriteToVram {
                x: 300, y: 50, w: 32, h: 32, pixel_data: dummy_texture_data
            });
        }
         // Update display area slightly to test dynamic VRAM view (will require renderer to handle this)
        // self.gpu_registers.display_vram_x = (self.frames_simulated % 100) as u16;


        self.frames_simulated += 1;
    }

    pub fn take_pending_gpu_commands(&mut self) -> VecDeque<GpuCommand> {
        std::mem::take(&mut self.pending_gpu_commands)
    }

    pub fn get_gpu_registers(&self) -> &PsxGpuRegisters {
        &self.gpu_registers
    }
}