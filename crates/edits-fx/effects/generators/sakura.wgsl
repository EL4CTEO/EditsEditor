//! id = "sakura"
//! name = "Sakura Petals"
//! kind = "generator"
//! category = "anime"
//! description = "Falling, tumbling cherry blossom petals (transparent background)."
//! tags = ["sakura", "cherry blossom", "petals", "anime", "spring", "romance"]
//! params = [
//!   { name = "density", type = "float", default = 1.0, min = 0.1, max = 5.0 },
//!   { name = "speed", type = "float", default = 0.6, min = 0.0, max = 5.0 },
//!   { name = "size", type = "float", default = 1.0, min = 0.2, max = 4.0 },
//!   { name = "wind", type = "float", default = 0.5, min = -3.0, max = 3.0 },
//!   { name = "color", type = "color", default = "#ffb7c5" },
//! ]
fn petal(f: vec2f, ang: f32, s: f32) -> f32 {
    let q = rot2(ang) * f / s;
    let r = length(q * vec2f(1.0, 1.8));
    let notch = smoothstep(0.08, 0.0, abs(q.x)) * step(0.3, q.y);
    return smoothstep(0.5, 0.42, r) * (1.0 - notch);
}
fn layer(uv: vec2f, scale: f32, sp: f32, p: Params) -> vec2f {
    let t = time() * p.speed * sp;
    var q = uv * vec2f(aspect(), 1.0) * scale * p.density;
    q += vec2f(-t * p.wind, -t);
    let cell = floor(q);
    let h = hash22(cell + scale);
    var f = fract(q) - 0.5;
    f += vec2f(sin(t * 2.0 + h.x * 10.0), cos(t * 1.5 + h.y * 10.0)) * 0.15;
    let ang = t * (1.0 + h.x * 2.0) + h.y * 6.0;
    let flip = abs(cos(t * 1.3 + h.x * 5.0)) * 0.7 + 0.3;
    let v = petal(f * vec2f(1.0 / flip, 1.0), ang, 0.25 * p.size * (0.6 + h.y * 0.6)) * step(0.45, h.x);
    return vec2f(v, h.y);
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    var c = vec3f(0.0);
    var a = 0.0;
    for (var i = 0; i < 3; i++) {
        let s = 3.0 + f32(i) * 2.5;
        let l = layer(uv, s, 0.7 + f32(i) * 0.25, p);
        let col = mix(p.color.rgb, vec3f(1.0, 0.92, 0.95), l.y * 0.5);
        c = c * (1.0 - l.x) + col * l.x;
        a = a + l.x * (1.0 - a);
    }
    return vec4f(c, a);
}
