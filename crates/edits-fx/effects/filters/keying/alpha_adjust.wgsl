//! id = "alpha_adjust"
//! name = "Alpha Adjust"
//! kind = "filter"
//! category = "keying"
//! description = "Choke/grow, invert or threshold the layer's alpha channel."
//! tags = ["alpha", "matte", "choke", "transparency"]
//! params = [
//!   { name = "gamma", type = "float", default = 1.0, min = 0.1, max = 10.0 },
//!   { name = "invert", type = "bool", default = false },
//!   { name = "threshold", type = "float", default = -1.0, min = -1.0, max = 1.0, desc = "Hard alpha cutoff (-1 = off)" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = unpremul(src_clamp(uv));
    var a = pow(c.a, p.gamma);
    if (p.threshold >= 0.0) { a = step(p.threshold, a); }
    if (p.invert > 0.5) { a = 1.0 - a; }
    return premul(vec4f(c.rgb, a));
}
