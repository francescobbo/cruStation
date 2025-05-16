struct VertexInput {
    @location(0) position: vec2<f32>,   // NDC
    @location(1) tex_coords: vec2<f32>, // Texture coordinates
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>, // Pass tex_coords to fragment shader
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 0.0, 1.0); // xy from input, z=0, w=1
    out.tex_coords = model.tex_coords;
    return out;
}

@group(0) @binding(0) var t_screen: texture_2d<u32>;
@group(0) @binding(1) var s_screen: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Load the BGR555 texel from the texture using the UV coordinates
    let sampled_bgr555_u32 = load_bgr555_texel_from_uv(t_screen, in.tex_coords);
    
    let rgb_color = unpack_bgr555_to_rgb_f32(sampled_bgr555_u32);

    return vec4<f32>(rgb_color, 1.0);
}
