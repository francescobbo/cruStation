struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

struct Uniforms {
    offset: vec2<i32>,
};

@group(0) @binding(0) var<uniform> u_uniforms: Uniforms;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Calculate the effective position
    // In WGSL, vector types are parameterized, e.g., vec2<i32>
    let position_i32: vec2<i32> = in.position + u_uniforms.offset;

    // Convert VRAM coordinates (0;1023, 0;511) into WebGPU/NDC coordinates
    // (-1;1, -1;1 for X/Y, 0;1 for Z)

    // Explicit type conversion from i32 to f32 is required
    let xpos: f32 = (f32(position_i32.x) / 512.0) - 1.0;
    // VRAM puts 0 at the top, WebGPU/OpenGL NDC Y is bottom-up, so mirror vertically.
    let ypos: f32 = 1.0 - (f32(position_i32.y) / 256.0);

    // Assign to the built-in position output
    // Z is 0.0 (near plane in WebGPU), W is 1.0 for perspective division
    out.clip_position = vec4<f32>(xpos, ypos, 0.0, 1.0);

    // Pass the color through to the fragment shader
    out.color = in.color;
}