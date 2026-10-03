//! id = "neon_edges"
//! name = "Neon Edges"
//! kind = "filter"
//! category = "stylize"
//! description = "Glowing neon outlines with color cycling, over a darkened image."
//! tags = ["neon", "edges", "glow", "cyberpunk", "outline"]
//! params = [
//!   { name = "strength", type = "float", default = 3.0, min = 0.0, max = 10.0 },
//!   { name = "darken", type = "float", default = 0.7, min = 0.0, max = 1.0 },
//!   { name = "hue_speed", type = "float", default = 0.2, min = -5.0, max = 5.0 },
//!   { name = "glow", type = "float", default = 2.0, min = 0.0, max = 8.0, desc = "Edge thickness" },
//! ]
fn E(uv: vec2f, w: f32) -> f32 {
    let e = texel() * w;
    let gx = luma(src_clamp(uv + vec2f(e.x, 0.0)).rgb) - luma(src_clamp(uv - vec2f(e.x, 0.0)).rgb);
    let gy = luma(src_clamp(uv + vec2f(0.0, e.y)).rgb) - luma(src_clamp(uv - vec2f(0.0, e.y)).rgb);
    return length(vec2f(gx, gy));
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = src_clamp(uv);
    let e = clamp((E(uv, 1.0) + E(uv, p.glow) * 0.6) * p.strength, 0.0, 1.5);
    let hue = fract(uv.x * 0.5 + uv.y * 0.3 + time() * p.hue_speed);
    let neon = to_linear(hsv2rgb(vec3f(hue, 0.8, 1.0))) * e * 2.0;
    return vec4f(c.rgb * (1.0 - p.darken) + neon, c.a);
}
