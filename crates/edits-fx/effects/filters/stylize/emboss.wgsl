//! id = "emboss"
//! name = "Emboss"
//! kind = "filter"
//! category = "stylize"
//! description = "Raised relief look."
//! tags = ["emboss", "relief", "metal"]
//! params = [
//!   { name = "strength", type = "float", default = 2.0, min = 0.0, max = 10.0 },
//!   { name = "angle", type = "angle", default = 135.0 },
//!   { name = "mix_source", type = "float", default = 0.0, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = dir_deg(p.angle) * texel();
    let a = luma(src_clamp(uv + d).rgb);
    let b = luma(src_clamp(uv - d).rgb);
    let e = 0.5 + (a - b) * p.strength;
    let s = src_clamp(uv);
    let g = vec3f(e);
    return vec4f(mix(g, s.rgb * e * 2.0, p.mix_source), s.a);
}
