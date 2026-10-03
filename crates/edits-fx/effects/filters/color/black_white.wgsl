//! id = "black_white"
//! name = "Black & White"
//! kind = "filter"
//! category = "color"
//! description = "Monochrome conversion with per-channel weights (like a color filter on B&W film)."
//! tags = ["color", "black and white", "monochrome", "grayscale"]
//! params = [
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//!   { name = "weights", type = "vec3", default = [0.2126, 0.7152, 0.0722], desc = "Channel weights" },
//!   { name = "contrast", type = "float", default = 1.0, min = 0.2, max = 3.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    let w = p.weights / max(p.weights.x + p.weights.y + p.weights.z, 0.0001);
    let l = clamp((dot(c.rgb, w) - 0.5) * p.contrast + 0.5, 0.0, 1.0);
    return grade_end(vec4f(mix(c.rgb, vec3f(l), p.amount), c.a));
}
