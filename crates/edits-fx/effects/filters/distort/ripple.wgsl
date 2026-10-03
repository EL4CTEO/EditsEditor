//! id = "ripple"
//! name = "Ripple"
//! kind = "filter"
//! category = "distort"
//! description = "Concentric water ripples from a center."
//! tags = ["ripple", "water", "drop", "wave"]
//! params = [
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "amplitude", type = "float", default = 8.0, min = 0.0, max = 100.0 },
//!   { name = "frequency", type = "float", default = 30.0, min = 1.0, max = 200.0 },
//!   { name = "speed", type = "float", default = 2.0, min = -20.0, max = 20.0 },
//!   { name = "decay", type = "float", default = 2.0, min = 0.0, max = 20.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = centered(uv, p.center);
    let r = length(d);
    let w = sin(r * p.frequency - time() * p.speed * TAU) * exp(-r * p.decay);
    let n = d / max(r, 0.0001);
    return src_clamp(uv + n / vec2f(aspect(), 1.0) * w * p.amplitude * texel().y);
}
