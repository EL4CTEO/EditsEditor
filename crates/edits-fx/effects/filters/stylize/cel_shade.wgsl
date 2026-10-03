//! id = "cel_shade"
//! name = "Cel Shade (Toon)"
//! kind = "filter"
//! category = "stylize"
//! description = "Flat cartoon shading bands with dark outlines."
//! tags = ["cel", "toon", "cartoon", "anime", "outline"]
//! params = [
//!   { name = "bands", type = "float", default = 4.0, min = 2.0, max = 12.0 },
//!   { name = "outline", type = "float", default = 1.0, min = 0.0, max = 4.0 },
//!   { name = "saturation", type = "float", default = 1.2, min = 0.0, max = 3.0 },
//! ]
fn L(uv: vec2f) -> f32 { return luma(unpremul(src_clamp(uv)).rgb); }
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    var h = rgb2hsv(saturate3(c.rgb));
    h.z = floor(h.z * p.bands + 0.5) / p.bands;
    h.y = clamp(h.y * p.saturation, 0.0, 1.0);
    let e = texel();
    let g = abs(L(uv + vec2f(e.x, 0.0)) - L(uv - vec2f(e.x, 0.0))) + abs(L(uv + vec2f(0.0, e.y)) - L(uv - vec2f(0.0, e.y)));
    let edge = smoothstep(0.08, 0.25, g * p.outline);
    return grade_end(vec4f(hsv2rgb(h) * (1.0 - edge), c.a));
}
