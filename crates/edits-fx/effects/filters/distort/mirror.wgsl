//! id = "mirror"
//! name = "Mirror"
//! kind = "filter"
//! category = "distort"
//! description = "Mirror the image across an axis (or into quadrants)."
//! tags = ["mirror", "symmetry", "reflect"]
//! params = [
//!   { name = "axis", type = "enum", options = ["left_to_right", "right_to_left", "top_to_bottom", "bottom_to_top", "quad"], default = "left_to_right" },
//!   { name = "position", type = "float", default = 0.5, min = 0.0, max = 1.0, desc = "Mirror line position" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    var q = uv;
    let m = i32(p.axis);
    if (m == 0) { if (q.x > p.position) { q.x = 2.0 * p.position - q.x; } }
    else if (m == 1) { if (q.x < p.position) { q.x = 2.0 * p.position - q.x; } }
    else if (m == 2) { if (q.y > p.position) { q.y = 2.0 * p.position - q.y; } }
    else if (m == 3) { if (q.y < p.position) { q.y = 2.0 * p.position - q.y; } }
    else { q = min(uv, 1.0 - uv); }
    return src_clamp(q);
}
