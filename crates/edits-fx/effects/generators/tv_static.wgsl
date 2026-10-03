//! id = "tv_static"
//! name = "TV Static"
//! kind = "generator"
//! category = "texture"
//! description = "Full-frame analog TV snow with rolling bars."
//! tags = ["static", "tv", "noise", "snow", "signal lost"]
//! params = [
//!   { name = "size", type = "float", default = 2.0, min = 1.0, max = 16.0 },
//!   { name = "bars", type = "float", default = 0.3, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let q = floor(uv * res() / p.size);
    let n = hash12(q + floor(time() * 60.0) * 7.7);
    let bar = 1.0 - p.bars * smoothstep(0.0, 0.1, abs(fract(uv.y - time() * 0.3) - 0.5) - 0.35);
    let v = n * bar;
    return vec4f(vec3f(v), 1.0);
}
