//! id = "sharpen"
//! name = "Sharpen"
//! kind = "filter"
//! category = "blur"
//! description = "Unsharp mask sharpening. Makes anime line art crisp."
//! tags = ["sharpen", "crisp", "detail"]
//! params = [
//!   { name = "amount", type = "float", default = 0.8, min = 0.0, max = 5.0 },
//!   { name = "radius", type = "float", default = 1.5, min = 0.5, max = 8.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let o = texel() * p.radius;
    let c = src_clamp(uv);
    let b = (src_clamp(uv + vec2f(o.x, 0.0)) + src_clamp(uv - vec2f(o.x, 0.0)) + src_clamp(uv + vec2f(0.0, o.y)) + src_clamp(uv - vec2f(0.0, o.y))) * 0.25;
    let s = c + (c - b) * p.amount;
    return vec4f(max(s.rgb, vec3f(0.0)), c.a);
}
