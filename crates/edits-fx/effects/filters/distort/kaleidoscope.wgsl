//! id = "kaleidoscope"
//! name = "Kaleidoscope"
//! kind = "filter"
//! category = "distort"
//! description = "Radial mirror segments. Animate `rotation` for hypnotic loops."
//! tags = ["kaleidoscope", "symmetry", "psychedelic", "trippy"]
//! params = [
//!   { name = "segments", type = "float", default = 6.0, min = 2.0, max = 32.0 },
//!   { name = "rotation", type = "angle", default = 0.0 },
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "zoom", type = "float", default = 1.0, min = 0.1, max = 10.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = centered(uv, p.center) / p.zoom;
    var a = atan2(d.y, d.x) + radians(p.rotation);
    let r = length(d);
    let seg = TAU / max(floor(p.segments), 2.0);
    a = fmod_pos(a, seg);
    a = abs(a - seg * 0.5);
    let q = vec2f(cos(a), sin(a)) * r;
    return src_mirror(q / vec2f(aspect(), 1.0) + p.center);
}
