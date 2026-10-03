//! id = "night_vision"
//! name = "Night Vision"
//! kind = "filter"
//! category = "color"
//! description = "Green phosphor night vision with noise and scanlines."
//! tags = ["night vision", "green", "military", "surveillance"]
//! params = [
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//!   { name = "noise", type = "float", default = 0.15, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    let l = pow(luma(c.rgb), 0.7) * 1.4;
    let n = (hash12(uv * res() + time() * 60.0) - 0.5) * p.noise;
    let s = 0.9 + 0.1 * sin(uv.y * res().y * 1.5);
    let v = 1.0 - smoothstep(0.4, 0.8, length(centered(uv, vec2f(0.5))));
    let g = vec3f(0.1, 1.0, 0.2) * (l + n) * s * v;
    return grade_end(vec4f(mix(c.rgb, g, p.amount), c.a));
}
