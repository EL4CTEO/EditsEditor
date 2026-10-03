//! id = "polar_coordinates"
//! name = "Polar Coordinates"
//! kind = "filter"
//! category = "distort"
//! description = "Wrap the image into a circle (rect to polar) or unwrap (polar to rect)."
//! tags = ["polar", "circle", "tiny planet", "wrap"]
//! params = [
//!   { name = "direction", type = "enum", options = ["rect_to_polar", "polar_to_rect"], default = "rect_to_polar" },
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//!   { name = "rotation", type = "angle", default = 0.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    var q = uv;
    if (i32(p.direction) == 0) {
        let d = centered(uv, vec2f(0.5));
        let a = atan2(d.y, d.x) + radians(p.rotation);
        q = vec2f(fract(a / TAU + 0.5), length(d) * 2.0);
    } else {
        let a = (uv.x - 0.5) * TAU + radians(p.rotation);
        let r = uv.y * 0.5;
        q = vec2f(cos(a), sin(a)) * r / vec2f(aspect(), 1.0) + 0.5;
    }
    return src(mix(uv, q, p.amount));
}
