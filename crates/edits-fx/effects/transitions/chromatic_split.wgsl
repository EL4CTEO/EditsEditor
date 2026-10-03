//! id = "chromatic_split"
//! name = "Chromatic Split"
//! kind = "transition"
//! category = "glitch"
//! description = "RGB channels fly apart and recombine on the new clip."
//! tags = ["rgb", "chromatic", "split", "amv"]
//! params = [
//!   { name = "amount", type = "float", default = 0.08, min = 0.0, max = 0.5 },
//!   { name = "angle", type = "angle", default = 0.0 },
//! ]
fn pick(uv: vec2f, t: f32) -> vec4f {
    if (t < 0.5) { return src_clamp(uv); }
    return orig_clamp(uv);
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let k = sin(t * PI) * p.amount;
    let d = dir_deg(p.angle) * k;
    let r = pick(uv + d, t).r;
    let g = pick(uv, t);
    let b = pick(uv - d, t).b;
    return vec4f(r, g.g, b, g.a);
}
