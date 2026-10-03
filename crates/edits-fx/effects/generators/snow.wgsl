//! id = "snow"
//! name = "Snow"
//! kind = "generator"
//! category = "weather"
//! description = "Gently falling, drifting snowflakes in multiple depth layers (transparent background)."
//! tags = ["snow", "winter", "weather", "particles"]
//! params = [
//!   { name = "density", type = "float", default = 1.0, min = 0.1, max = 5.0 },
//!   { name = "speed", type = "float", default = 1.0, min = 0.0, max = 10.0 },
//!   { name = "size", type = "float", default = 1.0, min = 0.2, max = 5.0 },
//!   { name = "wind", type = "float", default = 0.2, min = -3.0, max = 3.0 },
//!   { name = "color", type = "color", default = "#ffffff" },
//! ]
fn flakes(uv: vec2f, scale: f32, sp: f32, p: Params) -> f32 {
    let t = time() * p.speed * sp;
    var q = uv * vec2f(aspect(), 1.0) * scale * p.density;
    q += vec2f(t * p.wind + sin(t * 0.5 + q.y * 0.3) * 0.3, -t);
    let cell = floor(q);
    let f = fract(q) - 0.5;
    let h = hash22(cell);
    let pos = (h - 0.5) * 0.7;
    let r = length(f - pos);
    let s = 0.05 * p.size * (0.5 + h.y);
    return smoothstep(s, s * 0.3, r) * step(0.3, h.x);
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    var v = 0.0;
    v += flakes(uv, 5.0, 0.6, p) * 0.5;
    v += flakes(uv, 9.0, 0.9, p) * 0.75;
    v += flakes(uv, 15.0, 1.2, p);
    v = clamp(v, 0.0, 1.0);
    return vec4f(p.color.rgb * v, v);
}
