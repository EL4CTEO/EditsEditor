//! id = "manga_screentone"
//! name = "Manga Screentone"
//! kind = "filter"
//! category = "stylize"
//! description = "Black & white manga panel: ink lines + dotted screentone in midtones + white highlights."
//! tags = ["manga", "screentone", "comic", "ink", "black and white", "anime"]
//! params = [
//!   { name = "tone_size", type = "float", default = 5.0, min = 2.0, max = 24.0 },
//!   { name = "line_strength", type = "float", default = 1.0, min = 0.0, max = 4.0 },
//!   { name = "shadow", type = "float", default = 0.3, min = 0.0, max = 1.0, desc = "Solid black below this luma" },
//!   { name = "highlight", type = "float", default = 0.7, min = 0.0, max = 1.0, desc = "Pure white above this luma" },
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//! ]
fn L(uv: vec2f) -> f32 { return luma(to_srgb(unpremul(src_clamp(uv)).rgb)); }
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    let l = luma(c.rgb);
    let e = texel();
    let gx = L(uv + vec2f(e.x, 0.0)) - L(uv - vec2f(e.x, 0.0));
    let gy = L(uv + vec2f(0.0, e.y)) - L(uv - vec2f(0.0, e.y));
    let edge = clamp(length(vec2f(gx, gy)) * 4.0 * p.line_strength, 0.0, 1.0);
    let cell = fract(rot2(0.785) * (uv * res()) / p.tone_size) - 0.5;
    let dotm = 1.0 - smoothstep(0.25, 0.32, length(cell));
    var v = 1.0;
    if (l < p.shadow) { v = 0.0; }
    else if (l < p.highlight) { v = 1.0 - dotm; }
    v = v * (1.0 - edge);
    return grade_end(vec4f(mix(c.rgb, vec3f(v), p.amount), c.a));
}
