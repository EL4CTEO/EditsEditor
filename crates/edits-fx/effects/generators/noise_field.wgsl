//! id = "noise_field"
//! name = "Noise Field"
//! kind = "generator"
//! category = "texture"
//! description = "Animated fractal noise (clouds, smoke, mist), mapped between two colors."
//! tags = ["noise", "fbm", "clouds", "smoke", "texture"]
//! params = [
//!   { name = "scale", type = "float", default = 3.0, min = 0.1, max = 50.0 },
//!   { name = "speed", type = "float", default = 0.2, min = 0.0, max = 5.0 },
//!   { name = "octaves", type = "int", default = 5, min = 1, max = 8 },
//!   { name = "contrast", type = "float", default = 1.0, min = 0.1, max = 5.0 },
//!   { name = "color_a", type = "color", default = "#000000" },
//!   { name = "color_b", type = "color", default = "#ffffff" },
//!   { name = "alpha_from_luma", type = "bool", default = false },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let q = uv * vec2f(aspect(), 1.0) * p.scale;
    let t = time() * p.speed;
    let o = i32(clamp(p.octaves, 1.0, 8.0));
    let warp = vec2f(fbm(q + t, o), fbm(q + 5.3 - t, o));
    var v = fbm(q + warp * 0.8 + seed() * 3.0, o) * 0.5 + 0.5;
    v = clamp((v - 0.5) * p.contrast + 0.5, 0.0, 1.0);
    let c = mix(p.color_a.rgb, p.color_b.rgb, v);
    if (p.alpha_from_luma > 0.5) { return vec4f(c * v, v); }
    return vec4f(c, 1.0);
}
