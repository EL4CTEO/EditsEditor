//! id = "hearts"
//! name = "Floating Hearts"
//! kind = "generator"
//! category = "particles"
//! description = "Hearts floating upward with a gentle sway (romance edits, kawaii)."
//! tags = ["hearts", "love", "romance", "kawaii", "cute"]
//! params = [
//!   { name = "density", type = "float", default = 1.0, min = 0.1, max = 5.0 },
//!   { name = "speed", type = "float", default = 0.3, min = 0.0, max = 3.0 },
//!   { name = "size", type = "float", default = 1.0, min = 0.2, max = 4.0 },
//!   { name = "color", type = "color", default = "#ff4f8b" },
//! ]
fn heart(p0: vec2f) -> f32 {
    var p = vec2f(abs(p0.x), -p0.y + 0.6);
    var d = 0.0;
    if (p.y + p.x > 1.0) { d = length(p - vec2f(0.25, 0.75)) - 0.3535; }
    else { d = sqrt(min(dot(p - vec2f(0.0, 1.0), p - vec2f(0.0, 1.0)), dot(p - 0.5 * max(p.x + p.y, 0.0), p - 0.5 * max(p.x + p.y, 0.0)))) * sign(p.x - p.y); }
    return d;
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = time() * p.speed;
    var q = uv * vec2f(aspect(), 1.0) * 5.0 * p.density + vec2f(0.0, t * 3.0);
    let cell = floor(q);
    let h = hash22(cell + seed());
    var f = fract(q) - 0.5;
    f.x += sin(t * 3.0 + h.x * 10.0) * 0.15;
    let s = 0.3 * p.size * (0.5 + h.y * 0.5);
    let d = heart(f / s * 1.6 + vec2f(0.0, 0.3));
    let v = smoothstep(0.05, -0.05, d) * step(0.6, h.x);
    let col = mix(p.color.rgb, vec3f(1.0, 0.75, 0.85), h.y * 0.5);
    return vec4f(col * v, v);
}
