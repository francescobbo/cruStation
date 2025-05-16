const MASK_R_5BIT_PSX: u32 = 0x1Fu;
const MASK_G_5BIT_PSX: u32 = 0x1Fu << 5u;
const MASK_B_5BIT_PSX: u32 = 0x1Fu << 10u;
const A_MASK_BIT_PSX: u32 = 0x1u << 15u;

// Unpacks a u16 (passed as u32) BGR555 value to vec3<f32> RGB (0.0-1.0)
// Assumes PS1 BGR555 format: MASK_BBBBB_GGGGG_RRRRR
fn unpack_bgr555_to_rgb_f32(bgr555_val: u32) -> vec3<f32> {
    let r_5 = (bgr555_val & MASK_R_5BIT_PSX);
    let g_5 = (bgr555_val & MASK_G_5BIT_PSX) >> 5u;
    let b_5 = (bgr555_val & MASK_B_5BIT_PSX) >> 10u;
    
    // What do we do with the alpha bit?
    // let alpha = (bgr555_val & A_MASK_BIT_PSX) >> 15u;

    // Convert 5-bit components (0-31) to f32 (0.0 - 1.0)
    return vec3<f32>(f32(r_5) / 31.0, f32(g_5) / 31.0, f32(b_5) / 31.0);
}

// Packs vec3<f32> RGB (0.0-1.0) into a u32 representing a BGR555 value.
// PS1 BGR555 format: A_BBBBB_GGGGG_RRRRR
fn pack_rgb_f32_to_bgr555(rgb: vec3<f32>, alpha: bool) -> u32 {
    let r_clamped = clamp(rgb.r, 0.0, 1.0);
    let g_clamped = clamp(rgb.g, 0.0, 1.0);
    let b_clamped = clamp(rgb.b, 0.0, 1.0);

    let r_5 = u32(r_clamped * 31.999); // Map 0.0-1.0 to 0-31
    let g_5 = u32(g_clamped * 31.999);
    let b_5 = u32(b_clamped * 31.999);
    
    let a_val = select(0u, A_MASK_BIT_PSX, alpha);

    return a_val | (b_5 << 10u) | (g_5 << 5u) | r_5;
}

fn load_bgr555_texel_from_uv(tex: texture_2d<u32>, uv_coords: vec2<f32>) -> u32 {
    // Get texture dimensions for the base level (LOD 0)
    let texture_dims = textureDimensions(tex, 0u);

    // Convert normalized UV coordinates (0.0-1.0) to floating point pixel coordinates
    let pixel_coords_f = uv_coords * vec2<f32>(f32(texture_dims.x), f32(texture_dims.y));

    // Convert to integer pixel coordinates
    let pixel_coords_i = vec2<i32>(floor(pixel_coords_f));
    
    // Clamp coordinates to be within valid texture range [0, dim-1]
    let clamped_coords = clamp(pixel_coords_i, vec2<i32>(0), vec2<i32>(texture_dims) - vec2<i32>(1));

    // Load the texel value. For R16Uint, this returns vec4<u32> with the data in .r
    return textureLoad(tex, clamped_coords, 0u).r; // LOD 0u
}
