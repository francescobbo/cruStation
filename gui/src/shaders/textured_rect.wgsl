struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Uniforms for texture parameters and modulation
struct Uniforms {
    modulation_color: vec3<f32>,
    texture_mode: u32, // 0: CLUT4, 1: CLUT8, 2: Direct15bit
    clut_vram_base_x: u32,
    clut_vram_base_y: u32,
    tex_page_base_x_words: u32, 
    tex_page_base_y_words: u32,
};
@group(1) @binding(0) var<uniform> uniforms: Uniforms;

// VRAM texture itself
@group(0) @binding(0) var vram_texture: texture_2d<u32>;
@group(0) @binding(1) var vram_sampler: sampler;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 0.0, 1.0);
    out.uv = model.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) u32 {
    var texture_rgb: vec3<f32>;

    // Load the primary texel data (which could be indices or direct color)
    // This gives us the 16-bit word from VRAM based on interpolated UVs.
    let index_data_word = load_bgr555_texel_from_uv(vram_texture, in.uv);

    if (uniforms.texture_mode == 2u) { // Mode 2: 15-bit Direct color
        texture_rgb = unpack_bgr555_to_rgb_f32(index_data_word);
    } else if (uniforms.texture_mode == 0u) { // Mode 0: 4-bit CLUT
        // Determine which of the 4 indices in index_data_word to use.
        // `in.uv.x` is normalized UV across the *entire VRAM texture width*.
        // VRAM word X coordinate (float) for the current fragment:
        let vram_word_coord_f_x = in.uv.x * f32(textureDimensions(vram_texture, 0u).x);

        // The sub-index (0, 1, 2, or 3) within the 16-bit word.
        // fract(vram_word_coord_f_x) gives the fractional part within the current VRAM word.
        // Multiply by 4 because there are 4 indices per word.
        let sub_index_selector = u32(floor(fract(vram_word_coord_f_x) * 4.0));

        // Extract the 4-bit index. Assuming indices are packed LSB to MSB:
        // Index 0: bits 0-3, Index 1: bits 4-7, etc.
        let clut_4bit_index = (index_data_word >> (sub_index_selector * 4u)) & 0xFu;
        
        // Calculate VRAM coordinates of the CLUT entry
        // CLUT is typically a 16x1 strip of BGR555 colors.
        let clut_entry_vram_x = i32(uniforms.clut_vram_base_x + clut_4bit_index);
        let clut_entry_vram_y = i32(uniforms.clut_vram_base_y);

        // Load the actual BGR555 color from the CLUT in VRAM
        let clut_color_bgr555 = load_bgr555_from_vram_coords(
            vram_texture, // Sample from the same full VRAM texture
            vec2<i32>(clut_entry_vram_x, clut_entry_vram_y)
        );
        
        if (clut_color_bgr555 == 0u) {
            // On the PSX, texture color 0000h is fully-transparent, that means
            // textures cannot contain Black pixels. However, in some cases, Color
            // 8000h (Black with semi-transparent flag) can be used, depending on the
            // rendering command:
            discard;
        }

        texture_rgb = unpack_bgr555_to_rgb_f32(clut_color_bgr555);
    } else if (uniforms.texture_mode == 1u) { // Mode 1: 8-bit CLUT
        // Similar logic for 8-bit CLUT (2 indices per 16-bit word)
        let vram_word_coord_f_x = in.uv.x * f32(textureDimensions(vram_texture, 0u).x);
        let sub_index_selector = u32(floor(fract(vram_word_coord_f_x) * 2.0)); // 2 indices per word

        // Extract the 8-bit index
        let clut_8bit_index = (index_data_word >> (sub_index_selector * 8u)) & 0xFFu;

        // Calculate CLUT entry coordinates. CLUT for 8-bit can be wider (e.g., 16x16 for 256 colors).
        // For a simple 256x1 strip starting at clut_vram_base_x/y:
        let clut_entry_vram_x = i32(uniforms.clut_vram_base_x + clut_8bit_index);
        let clut_entry_vram_y = i32(uniforms.clut_vram_base_y);
        // If CLUT is 16 entries wide (e.g. 16x16 block):
        // let clut_entry_vram_x = i32(uniforms.clut_vram_base_x + (clut_8bit_index % 16u));
        // let clut_entry_vram_y = i32(uniforms.clut_vram_base_y + (clut_8bit_index / 16u));


        let clut_color_bgr555 = load_bgr555_from_vram_coords(
            vram_texture,
            vec2<i32>(clut_entry_vram_x, clut_entry_vram_y)
        );
        texture_rgb = unpack_bgr555_to_rgb_f32(clut_color_bgr555);
    } else { // Fallback or unhandled mode
        texture_rgb = vec3<f32>(1.0, 0.0, 1.0); // Magenta for error/unhandled
    }

    // Modulate and pack the final color
    let final_rgb = texture_rgb * uniforms.modulation_color;
    let output_is_transparent = false; // Placeholder for STP/blending logic
    let final_bgr555_u32 = pack_rgb_f32_to_bgr555(final_rgb, output_is_transparent);

    return final_bgr555_u32;
}