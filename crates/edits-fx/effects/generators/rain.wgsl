//! id = "rain"
//! name = "Rain"
//! kind = "generator"
//! category = "weather"
//! description = "Streaking rain drops (transparent background). Sad anime scene essential."
//! tags = ["rain", "weather", "sad", "storm"]
//! params = [
//!   { name = "density", type = "float", default = 1.0, min = 0.1, max = 5.0 },
//!   { name = "speed", type = "float", default = 1.5, min = 0.0, max = 10.0 },
//!   { name = "angle", type = "angle", default = 10.0, desc = "Tilt in degrees" },
//!   { name = "length", type = "float", default = 0.08, min = 0.01, max = 0.5 },
//!   { name = "color", type = "color", default = "#cfdcff" },
//!   { name = "opacity", type = "float", default = 0.6, min = 0.0, max = 1.0 },
//! ]
fn drops(uv: vec2f, scale: f32, p: Params) -> f32 {
    let q0 = rot2(radians(p.angle)) * ((uv - 0.5) * vec2f(aspect(), 1.0));
    var q = q0 * vec2f(scale * 8.0 * p.density, scale);
    let col = floor(q.x);
    let h = hash11(col + scale);
    q.y += time() * p.speed * (1.0 + h) * 3.0 + h * 10.0;
    let f = fract(q.y);
    let x = fract(q.x) - 0.5;
    let streak = smoothstep(p.length * 5.0, 0.0, f) * smoothstep(0.08, 0.0, abs(x));
    return streak * step(0.4, h);
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let v = clamp(drops(uv, 1.0, p) * 0.6 + drops(uv, 1.7, p) * 0.8 + drops(uv, 2.6, p), 0.0, 1.0) * p.opacity;
    return vec4f(p.color.rgb * v, v);
}
