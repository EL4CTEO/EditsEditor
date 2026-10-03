//! id = "vignette"
//! name = "Vignette"
//! kind = "filter"
//! category = "color"
//! description = "Darken (or color) the edges of the frame."
//! tags = ["vignette", "edges", "cinematic", "focus"]
//! params = [
//!   { name = "amount", type = "float", default = 0.5, min = 0.0, max = 1.0 },
//!   { name = "radius", type = "float", default = 0.75, min = 0.0, max = 2.0 },
//!   { name = "softness", type = "float", default = 0.45, min = 0.01, max = 2.0 },
//!   { name = "roundness", type = "float", default = 1.0, min = 0.0, max = 1.0, desc = "1 = circle, 0 = follows frame aspect" },
//!   { name = "color", type = "color", default = "#000000" },
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = src_clamp(uv);
    let d = (uv - p.center) * mix(vec2f(1.0), vec2f(aspect(), 1.0), p.roundness) * 2.0;
    let r = length(d) / mix(1.4142, 1.0 + aspect() * 0.4, p.roundness);
    let v = smoothstep(p.radius, p.radius + p.softness, r) * p.amount;
    return vec4f(mix(c.rgb, p.color.rgb * c.a, v), c.a);
}
