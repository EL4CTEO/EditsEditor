//! id = "squeeze"
//! name = "Squeeze / Stretch"
//! kind = "filter"
//! category = "distort"
//! description = "Squash and stretch around a center (cartoon impact deformation)."
//! tags = ["squeeze", "stretch", "squash", "cartoon"]
//! params = [
//!   { name = "amount", type = "float", default = 0.3, min = -0.9, max = 3.0, desc = "Positive = stretch horizontally" },
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "preserve_area", type = "bool", default = true },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let sx = 1.0 + p.amount;
    var sy = 1.0;
    if (p.preserve_area > 0.5) { sy = 1.0 / sx; }
    let d = (uv - p.center) / vec2f(sx, sy);
    return src(d + p.center);
}
