//! id = "color_overlay"
//! name = "Color Overlay"
//! kind = "filter"
//! category = "color"
//! description = "Fill the layer with a color (keeps alpha). Handy for silhouettes and color flashes."
//! tags = ["color", "fill", "overlay", "silhouette"]
//! params = [
//!   { name = "color", type = "color", default = "#ffffff" },
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = src_clamp(uv);
    return vec4f(mix(c.rgb, p.color.rgb * c.a, p.amount), c.a);
}
