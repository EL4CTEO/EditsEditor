//! id = "bulge"
//! name = "Bulge / Pinch"
//! kind = "filter"
//! category = "distort"
//! description = "Magnify (positive) or pinch (negative) a circular area."
//! tags = ["bulge", "pinch", "magnify", "fisheye"]
//! params = [
//!   { name = "strength", type = "float", default = 0.5, min = -1.0, max = 3.0 },
//!   { name = "radius", type = "float", default = 0.35, min = 0.01, max = 2.0 },
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = centered(uv, p.center);
    let r = length(d) / p.radius;
    var q = d;
    if (r < 1.0) {
        let k = pow(r, 1.0 + p.strength) / max(r, 0.0001);
        q = d * mix(1.0, k, smoothstep(1.0, 0.0, r) * 0.999 + 0.001);
    }
    return src(q / vec2f(aspect(), 1.0) + p.center);
}
