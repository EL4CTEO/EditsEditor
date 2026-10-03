//! id = "shake"
//! name = "Camera Shake"
//! kind = "filter"
//! category = "distort"
//! description = "Organic noise-driven camera shake with position, rotation and zoom jitter plus optional motion smear. Keyframe `amplitude` for hits."
//! tags = ["shake", "camera", "impact", "amv", "hit", "wiggle"]
//! params = [
//!   { name = "amplitude", type = "float", default = 20.0, min = 0.0, max = 300.0, desc = "Position shake in pixels" },
//!   { name = "rotation", type = "float", default = 2.0, min = 0.0, max = 45.0, desc = "Rotation shake in degrees" },
//!   { name = "zoom", type = "float", default = 0.0, min = 0.0, max = 0.5, desc = "Zoom jitter" },
//!   { name = "frequency", type = "float", default = 12.0, min = 0.1, max = 60.0, desc = "Shakes per second" },
//!   { name = "scale_fill", type = "float", default = 1.05, min = 1.0, max = 2.0, desc = "Zoom in to hide edges" },
//!   { name = "smear", type = "float", default = 0.5, min = 0.0, max = 1.0, desc = "Motion blur along the shake" },
//!   { name = "edge", type = "enum", options = ["transparent", "clamp", "mirror", "wrap"], default = "mirror" },
//! ]
fn shake_uv(uv: vec2f, p: Params, t: f32) -> vec2f {
    let s = seed() * 13.7;
    let off = vec2f(noise1(t * p.frequency + s), noise1(t * p.frequency + s + 31.4)) * p.amplitude * texel();
    let rot = noise1(t * p.frequency * 0.8 + s + 77.0) * radians(p.rotation);
    let z = 1.0 + noise1(t * p.frequency * 0.6 + s + 12.0) * p.zoom;
    var d = (uv - 0.5 - off) * vec2f(aspect(), 1.0);
    d = rot2(rot) * d / (z * p.scale_fill);
    return d / vec2f(aspect(), 1.0) + 0.5;
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = time();
    if (p.smear <= 0.01) { return src_edge(shake_uv(uv, p, t), p.edge); }
    var acc = vec4f(0.0);
    let n = 8;
    for (var i = 0; i < n; i++) {
        let dt = (f32(i) / f32(n - 1) - 0.5) * p.smear / max(p.frequency, 0.1) * 0.5;
        acc += src_edge(shake_uv(uv, p, t + dt), p.edge);
    }
    return acc / f32(n);
}
