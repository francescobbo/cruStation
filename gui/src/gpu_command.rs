// Represents a color as used by PS1 primitives (typically R,G,B bytes)
#[derive(Copy, Clone, Debug)]
pub struct PsxColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

// Represents a vertex as understood by the PS1 GPU (simplified for now)
#[derive(Copy, Clone, Debug)]
pub struct PsxVertex {
    pub x: i16,         // Screen X coordinate (relative to drawing offset)
    pub y: i16,         // Screen Y coordinate (relative to drawing offset)
    pub color: PsxColor,  // Per-vertex color for Gouraud shading
    // pub u: u8, pub v: u8, // Texture coordinates (0-255 range) - for later
}

impl PsxVertex {
    /// Creates a new vertex with the given screen coordinates and color.
    pub fn from_position_and_color(position: u32, color: u32) -> Self {
        let x = position & 0xfff;
        let y = position >> 16 & 0xfff;
    
        let r = color as u8;
        let g = (color >> 8) as u8;
        let b = (color >> 16) as u8;

        Self {
            x: x as i16,
            y: y as i16,
            color: PsxColor { r, g, b },
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PsxUv {
    pub u: u8, // 0-255 range, relative to texture page origin
    pub v: u8, // 0-255 range, relative to texture page origin
}

// For GP0(2Ch), the color is global to the primitive, not per-vertex.
// Vertices only carry X,Y. UVs are separate.
#[derive(Copy, Clone, Debug)]
pub struct PsxPrimitiveVertex {
    pub x: i16,
    pub y: i16,
}

// Enum representing different commands the PS1 GPU can execute.
// This is a simplified set to begin with.
#[derive(Clone, Debug)]
pub enum GpuCommand {
    /// Sets the drawing area within VRAM. Primitives are clipped to this.
    SetDrawingArea {
        x1: u16, // Top-left X
        y1: u16, // Top-left Y
        x2: u16, // Bottom-right X
        y2: u16, // Bottom-right Y
    },
    /// Sets an offset applied to all incoming vertex coordinates.
    SetDrawingOffset {
        x: i16,
        y: i16,
    },
    /// Draws a solid or Gouraud-shaded triangle.
    DrawGouraudTriangle {
        vertices: [PsxVertex; 3],
    },

    /// Draw textured quads
    DrawTexturedQuad {
        vertices: [PsxPrimitiveVertex; 4],
        uvs: [PsxUv; 4],
        clut_attr: u16,
        texpage_attr: u16,
        modulation_color: PsxColor,
    },

    /// Uploads a block of pixel data to VRAM.
    WriteToVram {
        x: u16, // Destination X in VRAM
        y: u16, // Destination Y in VRAM
        w: u16, // Width of the block
        h: u16, // Height of the block
        pixel_data: Vec<u16>,
    },
    /// Defines which part of VRAM is shown on the display.
    SetDisplayArea {
        x: u16, // Top-left X in VRAM for display
        y: u16, // Top-left Y in VRAM for display
        w: u16, // Width of display area (e.g., 320, 640)
        h: u16, // Height of display area (e.g., 240, 480)
    }
    // TODO: Add more commands:
    // - DrawLine
    // - DrawRectangle
    // - CopyVramToVram
    // - ReadFromVram
    // - Textured primitives with proper UVs, texpage, CLUTs
    // - Commands to set texture blend modes, dithering, etc.
}