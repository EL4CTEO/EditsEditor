//! id = "color_balance"
//! name = "Color Balance (Lift/Gamma/Gain)"
//! kind = "filter"
//! category = "color"
//! description = "Tint shadows, midtones and highlights separately. Colors are offsets around gray (#808080 = neutral)."
//! tags = ["color", "grade", "lift gamma gain", "split toning"]
//! params = [
//!   { name = "shadows", type = "color", default = "#808080" },
//!   { name = "midtones", type = "color", default = "#808080" },
//!   { name = "highlights", type = "color", default = "#808080" },
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 2.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    let l = luma(c.rgb);
    let sh = (to_srgb(p.shadows.rgb) - 0.5) * (1.0 - smoothstep(0.0, 0.5, l));
    let md = (to_srgb(p.midtones.rgb) - 0.5) * (1.0 - abs(l - 0.5) * 2.0);
    let hi = (to_srgb(p.highlights.rgb) - 0.5) * smoothstep(0.5, 1.0, l);
    return grade_end(vec4f(c.rgb + (sh + md + hi) * p.amount, c.a));
}
