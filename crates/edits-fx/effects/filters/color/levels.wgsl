//! id = "levels"
//! name = "Levels"
//! kind = "filter"
//! category = "color"
//! description = "Input/output black & white points with midtone gamma."
//! tags = ["color", "levels", "contrast"]
//! params = [
//!   { name = "in_black", type = "float", default = 0.0, min = 0.0, max = 1.0 },
//!   { name = "in_white", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//!   { name = "gamma", type = "float", default = 1.0, min = 0.1, max = 5.0 },
//!   { name = "out_black", type = "float", default = 0.0, min = 0.0, max = 1.0 },
//!   { name = "out_white", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    var c = grade_begin(src_clamp(uv));
    var v = clamp((c.rgb - p.in_black) / max(p.in_white - p.in_black, 0.0001), vec3f(0.0), vec3f(1.0));
    v = pow(v, vec3f(1.0 / p.gamma));
    v = mix(vec3f(p.out_black), vec3f(p.out_white), v);
    return grade_end(vec4f(v, c.a));
}
