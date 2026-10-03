//! id = "kaleido_pattern"
//! name = "Kaleidoscope Pattern"
//! kind = "generator"
//! category = "background"
//! description = "Procedural rotating kaleidoscopic mandala."
//! tags = ["kaleidoscope", "mandala", "psychedelic", "pattern"]
//! params = [
//!   { name = "segments", type = "float", default = 8.0, min = 2.0, max = 32.0 },
//!   { name = "speed", type = "float", default = 0.3, min = -5.0, max = 5.0 },
//!   { name = "scale", type = "float", default = 3.0, min = 0.5, max = 20.0 },
//!   { name = "saturation", type = "float", default = 0.8, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = centered(uv, vec2f(0.5));
    var a = atan2(d.y, d.x) + time() * p.speed;
    let seg = TAU / floor(p.segments);
    a = abs(fmod_pos(a, seg) - seg * 0.5);
    let r = length(d);
    let q = vec2f(cos(a), sin(a)) * r * p.scale;
    let n = fbm(q + time() * 0.2, 4);
    let col = hsv2rgb(vec3f(fract(n + r + time() * 0.05), p.saturation, smoothstep(-0.5, 0.6, n)));
    return vec4f(to_linear(col), 1.0);
}
