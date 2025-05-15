// Input structure for the vertex shader.
// Matches `GpuVertex` in Rust.
struct VertexInput {
    @location(0) position: vec2<f32>, // Already in viewport's NDC
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
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0); // PS1 often has a 'semi-transparency' bit or blend mode
}
