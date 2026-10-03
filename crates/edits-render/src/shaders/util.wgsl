#include common
struct Util {
    a: vec4f,   // mix t / mode
    b: vec4f,   // color / misc
};
@group(0) @binding(0) var<uniform> U: Util;

// lerp(t0, t1, a.x)
@fragment
fn fs_mix(in: VsOut) -> @location(0) vec4f {
    return mix(textureSampleLevel(t0, samp, in.uv, 0.0), textureSampleLevel(t1, samp, in.uv, 0.0), U.a.x);
}

// copy t0 (resampling) with optional multiplier
@fragment
fn fs_copy(in: VsOut) -> @location(0) vec4f {
    return textureSampleLevel(t0, samp, in.uv, 0.0) * U.a.x;
}

// solid fill
@fragment
fn fs_fill(in: VsOut) -> @location(0) vec4f {
    return U.b;
}

fn ign(x: f32) -> f32 {
    return fract(52.9829189 * fract(dot(vec2f(x, x * 0.7), vec2f(0.06711056, 0.00583715))));
}

// Final output into an sRGB target. a.x: 0 straight alpha, 1 premultiplied, 2 over background color (b).
// a.y: dither amount. Output is linear; the sRGB render target encodes it.
@fragment
fn fs_output(in: VsOut) -> @location(0) vec4f {
    var c = textureSampleLevel(t0, samp, in.uv, 0.0);
    let mode = i32(U.a.x);
    if (mode == 2) { c = c + vec4f(U.b.rgb * U.b.a, U.b.a) * (1.0 - c.a); }
    var rgb = select(vec3f(0.0), c.rgb / c.a, c.a > 1e-5);
    if (mode == 1) { rgb = c.rgb; }
    // interleaved-gradient-noise dither in sRGB space to avoid banding in gradients/glows
    let n = fract(52.9829189 * fract(dot(in.pos.xy, vec2f(0.06711056, 0.00583715)))) - 0.5;
    let s = to_srgb(clamp(rgb, vec3f(0.0), vec3f(1.0))) + n * U.a.y / 255.0;
    return vec4f(to_linear(clamp(s, vec3f(0.0), vec3f(1.0))), clamp(c.a, 0.0, 1.0));
}

// Unpack straight-alpha texture into premultiplied working space (used after uploads).
@fragment
fn fs_premul(in: VsOut) -> @location(0) vec4f {
    let c = textureSampleLevel(t0, samp, in.uv, 0.0);
    return vec4f(c.rgb * c.a, c.a);
}

// Checkerboard backdrop for transparent previews (b.x = cell px, resolution in b.zw)
@fragment
fn fs_checker(in: VsOut) -> @location(0) vec4f {
    let p = floor(in.uv * U.b.zw / max(U.b.x, 1.0));
    let k = fract((p.x + p.y) * 0.5) * 2.0;
    let g = mix(0.18, 0.28, k);
    let c = textureSampleLevel(t0, samp, in.uv, 0.0);
    return c + vec4f(vec3f(g), 1.0) * (1.0 - c.a);
}
