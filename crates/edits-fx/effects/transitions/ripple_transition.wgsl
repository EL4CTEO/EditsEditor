//! id = "ripple_transition"
//! name = "Ripple"
//! kind = "transition"
//! category = "organic"
//! description = "Water ripple wave carries the cut."
//! tags = ["ripple", "water", "wave", "dreamy"]
//! params = [
//!   { name = "amplitude", type = "float", default = 0.05, min = 0.0, max = 0.3 },
//!   { name = "frequency", type = "float", default = 30.0, min = 1.0, max = 100.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let d = centered(uv, vec2f(0.5));
    let r = length(d);
    let w = sin(r * p.frequency - t * 20.0) * p.amplitude * sin(t * PI);
    let q = uv + d / max(r, 0.0001) / vec2f(aspect(), 1.0) * w;
    return mix(src_clamp(q), orig_clamp(q), smoothstep(0.2, 0.8, t));
}
