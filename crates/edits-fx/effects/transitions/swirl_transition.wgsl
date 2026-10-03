//! id = "swirl_transition"
//! name = "Swirl"
//! kind = "transition"
//! category = "organic"
//! description = "Twist into a vortex and untwist into the next clip."
//! tags = ["swirl", "vortex", "twist", "trippy"]
//! params = [
//!   { name = "strength", type = "float", default = 6.0, min = 0.0, max = 30.0 },
//!   { name = "radius", type = "float", default = 1.0, min = 0.1, max = 3.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let d = centered(uv, vec2f(0.5));
    let r = length(d);
    let k = sin(t * PI) * p.strength * max(0.0, 1.0 - r / p.radius);
    let q = rot2(k * k * 0.3 + k) * d / vec2f(aspect(), 1.0) + 0.5;
    return mix(src_mirror(q), orig_clamp(q), smoothstep(0.4, 0.6, t));
}
