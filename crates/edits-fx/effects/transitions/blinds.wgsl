//! id = "blinds"
//! name = "Blinds"
//! kind = "transition"
//! category = "geometric"
//! description = "Venetian blinds open onto the next clip."
//! tags = ["blinds", "stripes", "venetian"]
//! params = [
//!   { name = "count", type = "float", default = 10.0, min = 2.0, max = 100.0 },
//!   { name = "angle", type = "angle", default = 0.0 },
//!   { name = "stagger", type = "float", default = 0.3, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = dir_deg(p.angle + 90.0);
    let x = dot(uv - 0.5, d) + 0.5;
    let i = floor(x * p.count);
    let f = fract(x * p.count);
    let delay = i / p.count * p.stagger;
    let t = clamp((progress() - delay) / max(1.0 - p.stagger, 0.0001), 0.0, 1.0);
    return mix(from_img(uv), to_img(uv), step(f, t));
}
