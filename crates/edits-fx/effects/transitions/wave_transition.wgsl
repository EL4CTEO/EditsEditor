//! id = "wave"
//! name = "Wave"
//! kind = "transition"
//! category = "organic"
//! description = "A sine wave front sweeps across, warping the image as it passes."
//! tags = ["wave", "sweep", "warp"]
//! params = [
//!   { name = "amplitude", type = "float", default = 0.08, min = 0.0, max = 0.5 },
//!   { name = "waves", type = "float", default = 3.0, min = 0.5, max = 20.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let front = uv.x + sin(uv.y * p.waves * TAU) * p.amplitude;
    let k = smoothstep(t * 1.4 - 0.2, t * 1.4 - 0.1, front);
    let warp = sin(t * PI) * p.amplitude * 0.5 * sin(uv.y * 20.0 + t * 10.0);
    let q = uv + vec2f(warp, 0.0);
    return mix(orig_clamp(q), src_clamp(q), k);
}
