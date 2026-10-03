//! id = "ink"
//! name = "Ink Bleed"
//! kind = "transition"
//! category = "organic"
//! description = "Liquid ink spreading from a point reveals the next clip."
//! tags = ["ink", "liquid", "organic", "paint", "watercolor"]
//! params = [
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "turbulence", type = "float", default = 0.3, min = 0.0, max = 1.0 },
//!   { name = "softness", type = "float", default = 0.05, min = 0.0, max = 0.5 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = length(centered(uv, p.center));
    let n = fbm(uv * vec2f(aspect(), 1.0) * 4.0 + seed() * 3.0, 5);
    let r = progress() * 1.6 - 0.1;
    let v = d + n * p.turbulence;
    let k = smoothstep(r - p.softness, r + p.softness, v);
    return mix(to_img(uv), from_img(uv), k);
}
