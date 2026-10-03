//! id = "digital_noise"
//! name = "Digital Noise / Static"
//! kind = "filter"
//! category = "glitch"
//! description = "TV static and digital noise mixed over the image."
//! tags = ["noise", "static", "tv", "glitch"]
//! params = [
//!   { name = "amount", type = "float", default = 0.3, min = 0.0, max = 1.0 },
//!   { name = "size", type = "float", default = 2.0, min = 1.0, max = 32.0 },
//!   { name = "colored", type = "bool", default = false },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = src_clamp(uv);
    let q = floor(uv * res() / p.size);
    let f = floor(time() * 30.0);
    var n = vec3f(hash12(q + f * 13.1));
    if (p.colored > 0.5) { n = hash33(vec3f(q, f)); }
    return vec4f(mix(c.rgb, n * max(c.a, p.amount), p.amount), max(c.a, p.amount));
}
