//! id = "sketch"
//! name = "Pencil Sketch"
//! kind = "filter"
//! category = "stylize"
//! description = "Graphite pencil drawing with hatching."
//! tags = ["sketch", "pencil", "drawing", "hatching", "art"]
//! params = [
//!   { name = "strength", type = "float", default = 1.5, min = 0.0, max = 5.0 },
//!   { name = "hatching", type = "float", default = 0.5, min = 0.0, max = 1.0 },
//!   { name = "paper", type = "color", default = "#f4efe6" },
//! ]
fn L(uv: vec2f) -> f32 { return luma(to_srgb(unpremul(src_clamp(uv)).rgb)); }
fn effect(uv: vec2f, p: Params) -> vec4f {
    let e = texel();
    let gx = L(uv + vec2f(e.x, 0.0)) - L(uv - vec2f(e.x, 0.0));
    let gy = L(uv + vec2f(0.0, e.y)) - L(uv - vec2f(0.0, e.y));
    let edge = clamp(length(vec2f(gx, gy)) * 3.0 * p.strength, 0.0, 1.0);
    let l = L(uv);
    let px = uv * res();
    let h1 = step(0.5, fract((px.x + px.y) / 6.0)) * (1.0 - smoothstep(0.3, 0.6, l));
    let h2 = step(0.5, fract((px.x - px.y) / 6.0)) * (1.0 - smoothstep(0.1, 0.35, l));
    let ink = clamp(edge + (h1 + h2) * 0.35 * p.hatching, 0.0, 1.0);
    let paper = to_srgb(p.paper.rgb);
    let g = mix(paper, vec3f(0.12), ink);
    return grade_end(vec4f(g, src_clamp(uv).a));
}
