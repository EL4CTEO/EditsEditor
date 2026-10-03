//! id = "swirl"
//! name = "Swirl / Twirl"
//! kind = "filter"
//! category = "distort"
//! description = "Twist the image around a center."
//! tags = ["swirl", "twirl", "twist", "vortex"]
//! params = [
//!   { name = "angle", type = "angle", default = 180.0, min = -1080.0, max = 1080.0 },
//!   { name = "radius", type = "float", default = 0.5, min = 0.01, max = 2.0 },
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = centered(uv, p.center);
    let r = length(d);
    let k = 1.0 - smoothstep(0.0, p.radius, r);
    let q = rot2(radians(p.angle) * k * k) * d;
    return src(q / vec2f(aspect(), 1.0) + p.center);
}
