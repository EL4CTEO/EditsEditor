//! id = "radial_blur"
//! name = "Radial (Zoom) Blur"
//! kind = "filter"
//! category = "blur"
//! description = "Zoom blur from a center point. The classic AMV impact/zoom-in look."
//! tags = ["blur", "zoom", "impact", "amv", "speed"]
//! params = [
//!   { name = "amount", type = "float", default = 0.15, min = 0.0, max = 1.0, desc = "Blur strength (fraction of distance to center)" },
//!   { name = "center", type = "point", default = [0.5, 0.5], desc = "Zoom center (uv)" },
//!   { name = "samples", type = "int", default = 32, min = 4, max = 96 },
//!   { name = "falloff", type = "float", default = 0.0, min = 0.0, max = 1.0, desc = "Keep the center sharp (0..1)" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let n = i32(clamp(p.samples, 4.0, 96.0));
    let d = uv - p.center;
    let k = p.amount * smoothstep(0.0, max(p.falloff, 0.0001), length(d * vec2f(aspect(), 1.0)) );
    var acc = vec4f(0.0);
    for (var i = 0; i < n; i++) {
        let s = 1.0 - k * f32(i) / f32(n);
        acc += src_clamp(p.center + d * s);
    }
    return acc / f32(n);
}
