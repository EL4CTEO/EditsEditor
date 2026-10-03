//! id = "edge_detect"
//! name = "Edge Detect"
//! kind = "filter"
//! category = "stylize"
//! description = "Sobel edges as glowing lines or ink, over black or the source."
//! tags = ["edge", "outline", "sobel", "lines", "neon"]
//! params = [
//!   { name = "strength", type = "float", default = 2.0, min = 0.0, max = 10.0 },
//!   { name = "color", type = "color", default = "#ffffff" },
//!   { name = "use_source_color", type = "bool", default = false },
//!   { name = "background", type = "enum", options = ["black", "source", "transparent", "white"], default = "black" },
//! ]
fn L(uv: vec2f) -> f32 { return luma(unpremul(src_clamp(uv)).rgb); }
fn effect(uv: vec2f, p: Params) -> vec4f {
    let e = texel();
    let tl = L(uv + vec2f(-e.x, -e.y)); let t = L(uv + vec2f(0.0, -e.y)); let tr = L(uv + vec2f(e.x, -e.y));
    let l = L(uv + vec2f(-e.x, 0.0)); let r = L(uv + vec2f(e.x, 0.0));
    let bl = L(uv + vec2f(-e.x, e.y)); let b = L(uv + vec2f(0.0, e.y)); let br = L(uv + vec2f(e.x, e.y));
    let gx = -tl - 2.0 * l - bl + tr + 2.0 * r + br;
    let gy = -tl - 2.0 * t - tr + bl + 2.0 * b + br;
    let g = clamp(length(vec2f(gx, gy)) * p.strength, 0.0, 1.0);
    let s = src_clamp(uv);
    var col = p.color.rgb;
    if (p.use_source_color > 0.5) { col = unpremul(s).rgb * 1.5; }
    let m = i32(p.background);
    if (m == 1) { return vec4f(s.rgb + col * g, s.a); }
    if (m == 2) { return vec4f(col * g, g); }
    if (m == 3) { return vec4f(mix(vec3f(1.0), vec3f(0.0), g), 1.0); }
    return vec4f(col * g, 1.0);
}
