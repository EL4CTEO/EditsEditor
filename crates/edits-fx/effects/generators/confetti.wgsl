//! id = "confetti"
//! name = "Confetti"
//! kind = "generator"
//! category = "particles"
//! description = "Colorful tumbling confetti pieces falling (celebrations, outros)."
//! tags = ["confetti", "party", "celebration", "particles"]
//! params = [
//!   { name = "density", type = "float", default = 1.0, min = 0.1, max = 5.0 },
//!   { name = "speed", type = "float", default = 0.8, min = 0.0, max = 5.0 },
//!   { name = "size", type = "float", default = 1.0, min = 0.2, max = 4.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    var c = vec3f(0.0);
    var a = 0.0;
    for (var i = 0; i < 3; i++) {
        let scale = (4.0 + f32(i) * 3.0) * p.density;
        let t = time() * p.speed * (0.8 + f32(i) * 0.3);
        var q = uv * vec2f(aspect(), 1.0) * scale + vec2f(0.0, -t);
        let cell = floor(q);
        let h = hash22(cell + f32(i) * 10.0);
        var f = fract(q) - 0.5 - (h - 0.5) * 0.5;
        f = rot2(t * 3.0 * (h.x - 0.5) + h.y * 6.0) * f;
        let flip = cos(t * 4.0 * h.y + h.x * 10.0);
        let s = 0.12 * p.size;
        let v = (1.0 - smoothstep(s * 0.9, s, abs(f.x))) * (1.0 - smoothstep(s * 0.45 * abs(flip), s * 0.45 * abs(flip) + 0.02, abs(f.y))) * step(0.5, h.x);
        let col = to_linear(hsv2rgb(vec3f(h.y, 0.75, 0.6 + 0.4 * abs(flip))));
        c = c * (1.0 - v) + col * v;
        a = a + v * (1.0 - a);
    }
    return vec4f(c, a);
}
