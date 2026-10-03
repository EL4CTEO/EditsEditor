//! id = "duotone"
//! name = "Duotone"
//! kind = "filter"
//! category = "color"
//! description = "Two-color poster look with adjustable contrast."
//! tags = ["color", "duotone", "poster", "graphic"]
//! params = [
//!   { name = "shadow", type = "color", default = "#1b0b52" },
//!   { name = "highlight", type = "color", default = "#ffcc00" },
//!   { name = "contrast", type = "float", default = 1.2, min = 0.2, max = 4.0 },
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    let l = clamp((luma(c.rgb) - 0.5) * p.contrast + 0.5, 0.0, 1.0);
    let d = mix(to_srgb(p.shadow.rgb), to_srgb(p.highlight.rgb), l);
    return grade_end(vec4f(mix(c.rgb, d, p.amount), c.a));
}
