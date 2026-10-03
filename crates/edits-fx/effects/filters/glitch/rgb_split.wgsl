//! id = "rgb_split"
//! name = "RGB Split"
//! kind = "filter"
//! category = "glitch"
//! description = "Offset red and blue channels in a direction. Pulse it on beats."
//! tags = ["rgb split", "chromatic", "glitch", "amv"]
//! params = [
//!   { name = "amount", type = "float", default = 10.0, min = 0.0, max = 300.0, desc = "Offset in pixels" },
//!   { name = "angle", type = "angle", default = 0.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = dir_deg(p.angle) * p.amount * texel();
    let r = src(uv + d);
    let g = src(uv);
    let b = src(uv - d);
    return vec4f(r.r, g.g, b.b, max(g.a, max(r.a, b.a)));
}
