//! id = "sepia"
//! name = "Sepia"
//! kind = "filter"
//! category = "color"
//! description = "Warm sepia tone (flashbacks, memories)."
//! tags = ["color", "sepia", "vintage", "flashback"]
//! params = [
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    let s = vec3f(dot(c.rgb, vec3f(0.393, 0.769, 0.189)), dot(c.rgb, vec3f(0.349, 0.686, 0.168)), dot(c.rgb, vec3f(0.272, 0.534, 0.131)));
    return grade_end(vec4f(mix(c.rgb, s, p.amount), c.a));
}
