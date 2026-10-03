//! id = "plasma"
//! name = "Plasma"
//! kind = "generator"
//! category = "background"
//! description = "Classic psychedelic plasma with cycling palette."
//! tags = ["plasma", "psychedelic", "retro", "demoscene", "background"]
//! params = [
//!   { name = "scale", type = "float", default = 4.0, min = 0.5, max = 30.0 },
//!   { name = "speed", type = "float", default = 1.0, min = 0.0, max = 10.0 },
//!   { name = "saturation", type = "float", default = 0.8, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let q = (uv - 0.5) * vec2f(aspect(), 1.0) * p.scale;
    let t = time() * p.speed;
    var v = sin(q.x + t) + sin((q.y + t) * 0.5) + sin((q.x + q.y + t) * 0.5);
    let c2 = q + vec2f(sin(t / 3.0), cos(t / 2.0)) * 2.0;
    v += sin(sqrt(dot(c2, c2) + 1.0) + t);
    let col = hsv2rgb(vec3f(fract(v * 0.15 + t * 0.05), p.saturation, 1.0));
    return vec4f(to_linear(col), 1.0);
}
