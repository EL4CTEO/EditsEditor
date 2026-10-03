//! id = "exposure"
//! name = "Exposure"
//! kind = "filter"
//! category = "color"
//! description = "Photographic exposure in stops, with offset and gamma (linear space)."
//! tags = ["color", "exposure", "brightness", "gamma"]
//! params = [
//!   { name = "stops", type = "float", default = 0.0, min = -6.0, max = 6.0 },
//!   { name = "offset", type = "float", default = 0.0, min = -0.5, max = 0.5 },
//!   { name = "gamma", type = "float", default = 1.0, min = 0.2, max = 5.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = unpremul(src_clamp(uv));
    var rgb = c.rgb * exp2(p.stops) + p.offset;
    rgb = pow(max(rgb, vec3f(0.0)), vec3f(1.0 / p.gamma));
    return premul(vec4f(rgb, c.a));
}
