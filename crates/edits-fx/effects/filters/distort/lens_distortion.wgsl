//! id = "lens_distortion"
//! name = "Lens Distortion"
//! kind = "filter"
//! category = "distort"
//! description = "Barrel (positive) or pincushion (negative) distortion with optional chromatic fringing."
//! tags = ["lens", "barrel", "fisheye", "distortion", "chromatic"]
//! params = [
//!   { name = "k1", type = "float", default = 0.2, min = -1.0, max = 2.0 },
//!   { name = "k2", type = "float", default = 0.0, min = -1.0, max = 2.0 },
//!   { name = "zoom", type = "float", default = 1.0, min = 0.2, max = 3.0 },
//!   { name = "fringe", type = "float", default = 0.0, min = 0.0, max = 0.1, desc = "Chromatic fringing" },
//! ]
fn distort(uv: vec2f, k1: f32, k2: f32, z: f32) -> vec2f {
    let d = centered(uv, vec2f(0.5));
    let r2 = dot(d, d);
    let f = 1.0 + k1 * r2 + k2 * r2 * r2;
    return d * f / z / vec2f(aspect(), 1.0) + 0.5;
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let g = src(distort(uv, p.k1, p.k2, p.zoom));
    if (p.fringe <= 0.0) { return g; }
    let r = src(distort(uv, p.k1 + p.fringe, p.k2, p.zoom));
    let b = src(distort(uv, p.k1 - p.fringe, p.k2, p.zoom));
    return vec4f(r.r, g.g, b.b, max(g.a, max(r.a, b.a)));
}
