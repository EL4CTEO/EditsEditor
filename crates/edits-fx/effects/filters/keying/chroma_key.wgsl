//! id = "chroma_key"
//! name = "Chroma Key"
//! kind = "filter"
//! category = "keying"
//! description = "Remove a background color (green/blue screen) with spill suppression."
//! tags = ["key", "green screen", "chroma key", "remove background"]
//! params = [
//!   { name = "key_color", type = "color", default = "#00ff00" },
//!   { name = "tolerance", type = "float", default = 0.3, min = 0.0, max = 1.0 },
//!   { name = "softness", type = "float", default = 0.1, min = 0.0, max = 1.0 },
//!   { name = "spill", type = "float", default = 0.5, min = 0.0, max = 1.0 },
//! ]
fn to_ycc(c: vec3f) -> vec2f {
    return vec2f(-0.169 * c.r - 0.331 * c.g + 0.5 * c.b, 0.5 * c.r - 0.419 * c.g - 0.081 * c.b);
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = unpremul(src_clamp(uv));
    let s = to_srgb(c.rgb);
    let k = to_srgb(p.key_color.rgb);
    let d = distance(to_ycc(s), to_ycc(k)) * 2.0;
    let a = smoothstep(p.tolerance, p.tolerance + p.softness + 0.0001, d);
    var rgb = s;
    let kl = luma(k);
    let spill_amt = clamp((1.0 - a) + 0.3, 0.0, 1.0) * p.spill;
    rgb = mix(rgb, vec3f(luma(rgb)) + (rgb - vec3f(luma(rgb))) * 0.5, spill_amt * (1.0 - a));
    return premul(vec4f(to_linear(rgb), c.a * a));
}
