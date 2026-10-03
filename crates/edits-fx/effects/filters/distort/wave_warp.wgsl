//! id = "wave_warp"
//! name = "Wave Warp"
//! kind = "filter"
//! category = "distort"
//! description = "Sine-wave displacement that travels over time (water, dream, heat)."
//! tags = ["wave", "warp", "water", "dream", "wobble"]
//! params = [
//!   { name = "amplitude", type = "float", default = 10.0, min = 0.0, max = 200.0, desc = "Pixels" },
//!   { name = "wavelength", type = "float", default = 120.0, min = 4.0, max = 2000.0, desc = "Pixels" },
//!   { name = "speed", type = "float", default = 1.0, min = -20.0, max = 20.0, desc = "Cycles per second" },
//!   { name = "angle", type = "angle", default = 0.0, desc = "Direction of displacement" },
//!   { name = "edge", type = "enum", options = ["transparent", "clamp", "mirror", "wrap"], default = "clamp" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let dir = dir_deg(p.angle);
    let perp = vec2f(-dir.y, dir.x);
    let pos = dot(uv * res(), perp);
    let s = sin(pos / p.wavelength * TAU - time() * p.speed * TAU);
    return src_edge(uv + dir * s * p.amplitude * texel(), p.edge);
}
