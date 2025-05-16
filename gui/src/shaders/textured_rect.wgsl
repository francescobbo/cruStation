struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Uniforms for texture parameters and modulation
struct TextureParams {
    modulation_color: vec3<f32>,
};
@group(1) @binding(0) var<uniform> tex_params: TextureParams;

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
    // Load the BGR555 texel from the texture using the UV coordinates
    let sampled_bgr555_u32 = load_bgr555_texel_from_uv(vram_texture, in.uv);
    
    let texture_rgb = unpack_bgr555_to_rgb_f32(sampled_bgr555_u32);
    
    // Modulate texture color with the command's blend color.
    let final_color = texture_rgb * tex_params.modulation_color;
    let final_bgr555_u32 = pack_rgb_f32_to_bgr555(final_color, false);

    return final_bgr555_u32;
}
