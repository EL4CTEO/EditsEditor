//! id = "shockwave"
//! name = "Shockwave"
//! kind = "filter"
//! category = "distort"
//! description = "Expanding refractive ring. Animate `radius` from 0 to ~1.5 on a hit."
//! tags = ["shockwave", "impact", "ring", "distortion", "amv", "hit"]
//! params = [
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "radius", type = "float", default = 0.4, min = 0.0, max = 3.0, desc = "Ring radius (frame heights)" },
//!   { name = "width", type = "float", default = 0.08, min = 0.005, max = 1.0 },
//!   { name = "strength", type = "float", default = 0.04, min = -0.3, max = 0.3 },
//!   { name = "chroma", type = "float", default = 0.5, min = 0.0, max = 2.0, desc = "Chromatic split on the ring" },
//!   { name = "brighten", type = "float", default = 0.2, min = 0.0, max = 2.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = centered(uv, p.center);
    let r = length(d);
    let x = (r - p.radius) / p.width;
    let ring = exp(-x * x * 4.0) * sign(x) * -1.0;
    let n = d / max(r, 0.0001);
    let off = n / vec2f(aspect(), 1.0) * ring * p.strength;
    let cg = src_clamp(uv + off);
    let cr = src_clamp(uv + off * (1.0 + p.chroma));
    let cb = src_clamp(uv + off * (1.0 - p.chroma));
    let glow = exp(-x * x * 4.0) * p.brighten;
    return vec4f(vec3f(cr.r, cg.g, cb.b) * (1.0 + glow), cg.a);
}
