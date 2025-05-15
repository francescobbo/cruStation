use bytemuck::{Pod, Zeroable};

// We'll keep ScreenQuadVertex as it's for the host GPU to draw the VRAM view.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ScreenQuadVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
}

impl ScreenQuadVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ScreenQuadVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1, format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

// --- Constants for VRAM (our offscreen "screen" texture) ---
pub const VRAM_WIDTH: u32 = 1024;
pub const VRAM_HEIGHT: u32 = 512;
// Format for our VRAM texture on the host GPU.
// PS1 VRAM is 16-bit, but Rgba8UnormSrgb is easier for direct viewing and WGPU.
// We'll need to handle color conversion when processing PS1 commands.
pub const VRAM_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

// --- Constants for the Display Area on Screen ---
// These will be controlled by GpuCommand::SetDisplayArea later.
// For now, we can have default values.
pub const DEFAULT_DISPLAY_X_IN_VRAM: u32 = 0;
pub const DEFAULT_DISPLAY_Y_IN_VRAM: u32 = 0;
pub const DEFAULT_DISPLAY_WIDTH: u32 = 640; // Default visible width
pub const DEFAULT_DISPLAY_HEIGHT: u32 = 480; // Default visible height

// Calculate texture coordinates for the default display area.
// These are used by SCREEN_QUAD_VERTICES to sample the correct part of VRAM.
// These might become dynamic if SetDisplayArea changes them frequently.
const DISPLAY_U_MIN: f32 = DEFAULT_DISPLAY_X_IN_VRAM as f32 / VRAM_WIDTH as f32;
const DISPLAY_V_MIN: f32 = DEFAULT_DISPLAY_Y_IN_VRAM as f32 / VRAM_HEIGHT as f32;
const DISPLAY_U_MAX: f32 = (DEFAULT_DISPLAY_X_IN_VRAM + DEFAULT_DISPLAY_WIDTH) as f32 / VRAM_WIDTH as f32;
const DISPLAY_V_MAX: f32 = (DEFAULT_DISPLAY_Y_IN_VRAM + DEFAULT_DISPLAY_HEIGHT) as f32 / VRAM_HEIGHT as f32;

pub const SCREEN_QUAD_VERTICES: &[ScreenQuadVertex] = &[
    ScreenQuadVertex { position: [-1.0,  1.0], tex_coords: [DISPLAY_U_MIN, DISPLAY_V_MIN] },
    ScreenQuadVertex { position: [-1.0, -1.0], tex_coords: [DISPLAY_U_MIN, DISPLAY_V_MAX] },
    ScreenQuadVertex { position: [ 1.0,  1.0], tex_coords: [DISPLAY_U_MAX, DISPLAY_V_MIN] },

    ScreenQuadVertex { position: [-1.0, -1.0], tex_coords: [DISPLAY_U_MIN, DISPLAY_V_MAX] },
    ScreenQuadVertex { position: [ 1.0, -1.0], tex_coords: [DISPLAY_U_MAX, DISPLAY_V_MAX] },
    ScreenQuadVertex { position: [ 1.0,  1.0], tex_coords: [DISPLAY_U_MAX, DISPLAY_V_MIN] },
];

// PS1 Vertex structure for WGPU buffer
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 2], // Transformed to NDC for the current viewport
    pub color: [f32; 3],
}

impl GpuVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { // position
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute { // color
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}