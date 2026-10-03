//! id = "vibrance"
//! name = "Vibrance"
//! kind = "filter"
//! category = "color"
//! description = "Boost muted colors more than saturated ones (protects skin tones). Makes anime colors pop."
//! tags = ["color", "vibrance", "saturation", "pop", "anime"]
//! params = [
//!   { name = "amount", type = "float", default = 0.5, min = -1.0, max = 3.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    let mx = max(c.r, max(c.g, c.b));
    let mn = min(c.r, min(c.g, c.b));
    let sat = mx - mn;
    let l = luma(c.rgb);
    let k = p.amount * (1.0 - sat);
    return grade_end(vec4f(mix(vec3f(l), c.rgb, 1.0 + k), c.a));
}
