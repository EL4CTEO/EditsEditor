//! id = "solarize"
//! name = "Solarize"
//! kind = "filter"
//! category = "color"
//! description = "Inverts tones above a threshold (psychedelic darkroom effect)."
//! tags = ["color", "solarize", "psychedelic"]
//! params = [
//!   { name = "threshold", type = "float", default = 0.5, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    let o = select(c.rgb, 1.0 - c.rgb, c.rgb > vec3f(p.threshold));
    return grade_end(vec4f(o, c.a));
}
