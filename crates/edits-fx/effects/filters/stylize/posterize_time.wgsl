//! id = "noise_overlay"
//! name = "Paper / Noise Texture"
//! kind = "filter"
//! category = "stylize"
//! description = "Static paper/canvas texture multiplied over the image."
//! tags = ["texture", "paper", "canvas", "grunge"]
//! params = [
//!   { name = "amount", type = "float", default = 0.25, min = 0.0, max = 1.0 },
//!   { name = "scale", type = "float", default = 3.0, min = 0.5, max = 50.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = src_clamp(uv);
    let q = uv * res() / p.scale;
    let n = fbm(q * 0.05, 5) * 0.5 + 0.5;
    let f = hash12(floor(q)) ;
    let t = mix(1.0, n * 0.7 + f * 0.3 + 0.2, p.amount);
    return vec4f(c.rgb * t, c.a);
}
