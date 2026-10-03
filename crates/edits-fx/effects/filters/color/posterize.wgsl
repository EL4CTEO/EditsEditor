//! id = "posterize"
//! name = "Posterize"
//! kind = "filter"
//! category = "color"
//! description = "Reduce the number of color levels."
//! tags = ["color", "posterize", "levels", "graphic"]
//! params = [
//!   { name = "levels", type = "float", default = 5.0, min = 2.0, max = 64.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    let n = max(p.levels - 1.0, 1.0);
    return grade_end(vec4f(floor(c.rgb * n + 0.5) / n, c.a));
}
