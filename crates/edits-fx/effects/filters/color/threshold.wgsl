//! id = "threshold"
//! name = "Threshold"
//! kind = "filter"
//! category = "color"
//! description = "Pure black/white (or two colors) by luminance threshold — manga panel look."
//! tags = ["color", "threshold", "black and white", "manga"]
//! params = [
//!   { name = "level", type = "float", default = 0.5, min = 0.0, max = 1.0 },
//!   { name = "softness", type = "float", default = 0.02, min = 0.0, max = 0.5 },
//!   { name = "dark", type = "color", default = "#000000" },
//!   { name = "light", type = "color", default = "#ffffff" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    let k = smoothstep(p.level - p.softness, p.level + p.softness, luma(c.rgb));
    return grade_end(vec4f(mix(to_srgb(p.dark.rgb), to_srgb(p.light.rgb), k), c.a));
}
