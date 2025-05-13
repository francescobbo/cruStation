use bytemuck::{Pod, Zeroable};

// Describes a single vertex for our 3D triangle.
// It includes a 3D position and a 3-component color.
#[repr(C)] // Ensures C-compatible memory layout, important for sending to GPU.
#[derive(Copy, Clone, Debug, Pod, Zeroable)] // Derive traits for easy copying, debugging, and safe casting.
pub struct TriangleVertex {
    pub position: [f32; 3], // x, y, z coordinates
    pub color: [f32; 3],    // r, g, b color values
}

impl TriangleVertex {
    // Returns a description of how this vertex data is laid out in a GPU buffer.
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TriangleVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

// The actual vertex data for our rainbow triangle.
// These coordinates are in Normalized Device Coordinates (NDC) relative to the
// offscreen texture it's rendered onto.
pub const TRIANGLE_VERTICES: &[TriangleVertex] = &[
    TriangleVertex { position: [0.0, 0.75, 0.0], color: [1.0, 0.0, 0.0] },  // Top
    TriangleVertex { position: [-0.75, -0.75, 0.0], color: [0.0, 1.0, 0.0] }, // Bottom left
    TriangleVertex { position: [0.75, -0.75, 0.0], color: [0.0, 0.0, 1.0] },  // Bottom right
];


// Describes a single vertex for the full-screen quad used to draw the offscreen texture.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ScreenQuadVertex {
    pub position: [f32; 2],   // x, y Normalized Device Coordinates (NDC) for the quad itself.
    pub tex_coords: [f32; 2], // u, v texture coordinates to sample from the offscreen texture.
}

impl ScreenQuadVertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ScreenQuadVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

// --- Constants for the offscreen "screen" texture ---

// Full dimensions of the offscreen texture (the "buffer space").
pub const SCREEN_TEXTURE_WIDTH: u32 = 1024;
pub const SCREEN_TEXTURE_HEIGHT: u32 = 512;

// Dimensions of the sub-region of the offscreen texture that will be rendered to the screen.
// This region originates at (0,0) of the SCREEN_TEXTURE.
pub const VISIBLE_REGION_WIDTH: u32 = 640;
pub const VISIBLE_REGION_HEIGHT: u32 = 480;

// Format of the offscreen texture.
pub const SCREEN_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

// Calculate the texture coordinate limits for the visible sub-region.
// Texture coordinates are normalized (0.0 to 1.0).
// U_max corresponds to the right edge of the visible region.
// V_max corresponds to the bottom edge of the visible region (since V=0 is top).
const VISIBLE_U_MAX: f32 = VISIBLE_REGION_WIDTH as f32 / SCREEN_TEXTURE_WIDTH as f32;
const VISIBLE_V_MAX: f32 = VISIBLE_REGION_HEIGHT as f32 / SCREEN_TEXTURE_HEIGHT as f32;
// The visible region starts at (0,0) of the texture, so U_min and V_min are 0.0.
const VISIBLE_U_MIN: f32 = 0.0;
const VISIBLE_V_MIN: f32 = 0.0;

// Vertex data for a quad that covers the entire screen.
// The texture coordinates are adjusted to sample only the VISIBLE_REGION
// from the larger SCREEN_TEXTURE.
// WGPU texture coordinates have (0,0) at the top-left.
// The quad vertices map screen corners to these texture sub-region coordinates:
//   - Top-Left screen corner (-1, 1) samples Tex(U_MIN, V_MIN)
//   - Bottom-Left screen corner (-1, -1) samples Tex(U_MIN, V_MAX)
//   - Top-Right screen corner (1, 1) samples Tex(U_MAX, V_MIN)
//   - Bottom-Right screen corner (1, -1) samples Tex(U_MAX, V_MAX)
pub const SCREEN_QUAD_VERTICES: &[ScreenQuadVertex] = &[
    // Triangle 1: Defines the Top-Left, Bottom-Left, and Top-Right of the screen quad.
    ScreenQuadVertex { position: [-1.0,  1.0], tex_coords: [VISIBLE_U_MIN, VISIBLE_V_MIN] }, // Top-Left Quad -> Top-Left of Visible Tex Region
    ScreenQuadVertex { position: [-1.0, -1.0], tex_coords: [VISIBLE_U_MIN, VISIBLE_V_MAX] }, // Bottom-Left Quad -> Bottom-Left of Visible Tex Region
    ScreenQuadVertex { position: [ 1.0,  1.0], tex_coords: [VISIBLE_U_MAX, VISIBLE_V_MIN] }, // Top-Right Quad -> Top-Right of Visible Tex Region

    // Triangle 2: Defines the Bottom-Left, Bottom-Right, and Top-Right of the screen quad.
    ScreenQuadVertex { position: [-1.0, -1.0], tex_coords: [VISIBLE_U_MIN, VISIBLE_V_MAX] }, // Bottom-Left Quad -> Bottom-Left of Visible Tex Region
    ScreenQuadVertex { position: [ 1.0, -1.0], tex_coords: [VISIBLE_U_MAX, VISIBLE_V_MAX] }, // Bottom-Right Quad -> Bottom-Right of Visible Tex Region
    ScreenQuadVertex { position: [ 1.0,  1.0], tex_coords: [VISIBLE_U_MAX, VISIBLE_V_MIN] }, // Top-Right Quad -> Top-Right of Visible Tex Region
];

/*
// Previous SCREEN_QUAD_VERTICES for reference (mapped entire texture)
// const FULL_U_MIN: f32 = 0.0;
// const FULL_V_MIN: f32 = 0.0;
// const FULL_U_MAX: f32 = 1.0;
// const FULL_V_MAX: f32 = 1.0;
// pub const SCREEN_QUAD_VERTICES_OLD: &[ScreenQuadVertex] = &[
//     // Triangle 1
//     ScreenQuadVertex { position: [-1.0, -1.0], tex_coords: [FULL_U_MIN, FULL_V_MAX] }, // Bottom Left Quad -> (U=0, V=1) in Tex
//     ScreenQuadVertex { position: [ 1.0, -1.0], tex_coords: [FULL_U_MAX, FULL_V_MAX] }, // Bottom Right Quad -> (U=1, V=1) in Tex
//     ScreenQuadVertex { position: [ 1.0,  1.0], tex_coords: [FULL_U_MAX, FULL_V_MIN] }, // Top Right Quad -> (U=1, V=0) in Tex
//     // Triangle 2
//     ScreenQuadVertex { position: [-1.0, -1.0], tex_coords: [FULL_U_MIN, FULL_V_MAX] }, // Bottom Left Quad
//     ScreenQuadVertex { position: [ 1.0,  1.0], tex_coords: [FULL_U_MAX, FULL_V_MIN] }, // Top Right Quad
//     ScreenQuadVertex { position: [-1.0,  1.0], tex_coords: [FULL_U_MIN, FULL_V_MIN] }, // Top Left Quad
// ];
*/
