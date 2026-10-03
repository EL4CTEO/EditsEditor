//! id = "shape_wipe"
//! name = "Shape Wipe"
//! kind = "transition"
//! category = "wipe"
//! description = "Reveal through a growing shape: diamond, square, heart or star."
//! tags = ["wipe", "diamond", "heart", "star", "cute"]
//! params = [
//!   { name = "shape", type = "enum", options = ["diamond", "square", "heart", "star"], default = "diamond" },
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "rotation", type = "angle", default = 0.0 },
//!   { name = "softness", type = "float", default = 0.02, min = 0.0, max = 0.5 },
//! ]
fn sd_heart(p0: vec2f) -> f32 {
    var p = vec2f(abs(p0.x), -p0.y + 0.6);
    if (p.y + p.x > 1.0) { return sqrt(dot(p - vec2f(0.25, 0.75), p - vec2f(0.25, 0.75))) - sqrt(2.0) / 4.0; }
    let a = p - vec2f(0.0, 1.0);
    let b = p - 0.5 * max(p.x + p.y, 0.0);
    return sqrt(min(dot(a, a), dot(b, b))) * sign(p.x - p.y);
}
fn sd_star(p0: vec2f, r: f32) -> f32 {
    let k1 = vec2f(0.809016994, -0.587785252);
    let k2 = vec2f(-k1.x, k1.y);
    var p = vec2f(abs(p0.x), p0.y);
    p -= 2.0 * max(dot(k1, p), 0.0) * k1;
    p -= 2.0 * max(dot(k2, p), 0.0) * k2;
    p = vec2f(abs(p.x), p.y - r);
    let ba = 0.4 * vec2f(-k1.y, k1.x) - vec2f(0.0, 1.0);
    let h = clamp(dot(p, ba) / dot(ba, ba), 0.0, r);
    return length(p - ba * h) * sign(p.y * ba.x - p.x * ba.y);
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = rot2(radians(p.rotation)) * centered(uv, p.center);
    let s = progress() * 2.2;
    let m = i32(p.shape);
    var dist = 0.0;
    if (m == 0) { dist = (abs(d.x) + abs(d.y)) - s; }
    else if (m == 1) { dist = max(abs(d.x), abs(d.y)) - s * 0.8; }
    else if (m == 2) { dist = sd_heart(d / max(s * 1.2, 0.0001)) * s; }
    else { dist = sd_star(d, max(s * 1.1, 0.0001)); }
    let k = smoothstep(-p.softness, 0.0, dist);
    return mix(to_img(uv), from_img(uv), k);
}
