//! id = "heat_haze"
//! name = "Heat Haze"
//! kind = "filter"
//! category = "distort"
//! description = "Rising shimmering air distortion (deserts, fire, aura)."
//! tags = ["heat", "haze", "shimmer", "aura", "fire"]
//! params = [
//!   { name = "amount", type = "float", default = 4.0, min = 0.0, max = 50.0 },
//!   { name = "speed", type = "float", default = 1.0, min = 0.0, max = 10.0 },
//!   { name = "scale", type = "float", default = 60.0, min = 5.0, max = 500.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let q = uv * res() / p.scale + vec2f(0.0, time() * p.speed * 3.0);
    let d = vec2f(gnoise(q), gnoise(q * 1.7 + 4.0));
    return src_clamp(uv + d * p.amount * texel());
}
