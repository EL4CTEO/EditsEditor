//! id = "crt"
//! name = "CRT Monitor"
//! kind = "filter"
//! category = "glitch"
//! description = "Curved CRT screen with RGB shadow mask, scanlines, glow and vignette."
//! tags = ["crt", "retro", "monitor", "arcade", "scanlines"]
//! params = [
//!   { name = "curvature", type = "float", default = 0.15, min = 0.0, max = 1.0 },
//!   { name = "scanlines", type = "float", default = 0.4, min = 0.0, max = 1.0 },
//!   { name = "mask", type = "float", default = 0.3, min = 0.0, max = 1.0 },
//!   { name = "line_count", type = "float", default = 360.0, min = 50.0, max = 2000.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    var d = uv * 2.0 - 1.0;
    d *= 1.0 + p.curvature * dot(d.yx, d.yx) * 0.25;
    let q = d * 0.5 + 0.5;
    var c = src(q).rgb;
    let s = 1.0 - p.scanlines * (0.5 + 0.5 * sin(q.y * p.line_count * TAU));
    let m = fract(uv.x * res().x / 3.0);
    let mask = mix(vec3f(1.0), select(select(vec3f(0.6, 0.6, 1.2), vec3f(0.6, 1.2, 0.6), m < 0.666), vec3f(1.2, 0.6, 0.6), m < 0.333), p.mask);
    c *= s * mask;
    let v = smoothstep(1.2, 0.5, length(d));
    return vec4f(c * v * 1.15, inside(q));
}
