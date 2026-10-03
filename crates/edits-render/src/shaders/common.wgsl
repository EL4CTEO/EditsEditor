// Shared bindings for built-in passes (same layout as effects).
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var t0: texture_2d<f32>;
@group(0) @binding(3) var t1: texture_2d<f32>;
@group(0) @binding(4) var t2: texture_2d<f32>;
@group(0) @binding(5) var samp_wrap: sampler;
@group(0) @binding(6) var samp_near: sampler;

struct VsOut {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
};

@vertex
fn vs_full(@builtin(vertex_index) vi: u32) -> VsOut {
    var o: VsOut;
    let x = f32((vi << 1u) & 2u);
    let y = f32(vi & 2u);
    o.pos = vec4f(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    o.uv = vec2f(x, y);
    return o;
}

fn to_srgb(c: vec3f) -> vec3f {
    let lo = c * 12.92;
    let hi = 1.055 * pow(max(c, vec3f(0.0)), vec3f(1.0 / 2.4)) - 0.055;
    return select(hi, lo, c <= vec3f(0.0031308));
}
fn to_linear(c: vec3f) -> vec3f {
    let lo = c / 12.92;
    let hi = pow(max((c + 0.055) / 1.055, vec3f(0.0)), vec3f(2.4));
    return select(hi, lo, c <= vec3f(0.04045));
}
