//! id = "tint"
//! name = "Tint"
//! kind = "filter"
//! category = "color"
//! description = "Map black and white to two colors (monochrome tint)."
//! tags = ["color", "tint", "monochrome"]
//! params = [
//!   { name = "black", type = "color", default = "#000000" },
//!   { name = "white", type = "color", default = "#ff4fa3" },
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    let l = luma(c.rgb);
    let t = mix(to_srgb(p.black.rgb), to_srgb(p.white.rgb), l);
    return grade_end(vec4f(mix(c.rgb, t, p.amount), c.a));
}
