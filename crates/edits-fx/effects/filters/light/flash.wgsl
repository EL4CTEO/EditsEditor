//! id = "flash"
//! name = "Flash"
//! kind = "filter"
//! category = "light"
//! description = "Flash the image toward a color (white flash on hits). Animate `amount` or use the flash presets."
//! tags = ["flash", "white flash", "hit", "impact", "amv"]
//! params = [
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//!   { name = "color", type = "color", default = "#ffffff" },
//!   { name = "exposure", type = "float", default = 0.0, min = 0.0, max = 6.0, desc = "Additional over-exposure in stops before blending" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = src_clamp(uv);
    let exposed = c.rgb * exp2(p.exposure * p.amount);
    let target_c = p.color.rgb * max(c.a, p.color.a);
    return vec4f(mix(exposed, target_c, p.amount * p.color.a), max(c.a, p.amount * p.color.a));
}
