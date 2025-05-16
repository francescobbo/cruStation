struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Position is pre-transformed to the viewport's NDC, add z=0, w=1
    out.clip_position = vec4<f32>(model.position.x, model.position.y, 0.0, 1.0);
    out.color = model.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) u32 {
    let bgr555_color: u32 = pack_rgb_f32_to_bgr555(in.color, false);

    return bgr555_color;
}
