//! id = "embers"
//! name = "Embers / Fireflies"
//! kind = "generator"
//! category = "particles"
//! description = "Glowing particles rising and drifting upward (embers, fireflies, spirit particles)."
//! tags = ["embers", "fire", "fireflies", "particles", "magic", "spirit"]
//! params = [
//!   { name = "count", type = "float", default = 60.0, min = 1.0, max = 300.0 },
//!   { name = "speed", type = "float", default = 0.15, min = 0.0, max = 2.0 },
//!   { name = "size", type = "float", default = 1.0, min = 0.2, max = 5.0 },
//!   { name = "color", type = "color", default = "#ff7a1a" },
//!   { name = "glow", type = "float", default = 1.5, min = 0.0, max = 5.0 },
//!   { name = "direction", type = "angle", default = -90.0, desc = "Movement direction (-90 = up)" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let a = vec2f(aspect(), 1.0);
    var v = 0.0;
    let n = i32(clamp(p.count, 1.0, 300.0));
    let dir = dir_deg(p.direction);
    for (var i = 0; i < n; i++) {
        let fi = f32(i) + seed() * 100.0;
        let h = hash22(vec2f(fi, 1.7));
        let life = fract(time() * p.speed * (0.5 + h.y) + h.x);
        var pos = vec2f(h.x, hash11(fi * 3.3));
        pos += dir * life * 1.2 + vec2f(sin(life * 10.0 + fi) * 0.03, cos(life * 7.0 + fi) * 0.02);
        pos = fract(pos);
        let d = length((uv - pos) * a);
        let s = 0.004 * p.size * (0.5 + h.y);
        let fade = sin(life * PI);
        v += (smoothstep(s, 0.0, d) + s * s * p.glow * 8.0 / (d * d + s * s * 4.0) * 0.2) * fade;
    }
    v = clamp(v, 0.0, 2.0);
    return vec4f(p.color.rgb * v, clamp(v, 0.0, 1.0));
}
