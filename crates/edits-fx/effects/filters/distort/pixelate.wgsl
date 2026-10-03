//! id = "pixelate"
//! name = "Pixelate / Mosaic"
//! kind = "filter"
//! category = "distort"
//! description = "Square, hexagon or round-dot mosaic."
//! tags = ["pixelate", "mosaic", "pixel", "censor", "retro"]
//! params = [
//!   { name = "size", type = "float", default = 16.0, min = 1.0, max = 300.0, desc = "Cell size in pixels" },
//!   { name = "shape", type = "enum", options = ["square", "hexagon", "dots"], default = "square" },
//! ]
fn hex_center(q: vec2f) -> vec2f {
    let r = vec2f(1.0, 1.7320508);
    let h = r * 0.5;
    let a = fmod_v2(q, 1.0) ;
    let ga = floor(q / r) * r + h;
    let gb = floor((q - h) / r) * r + r;
    let da = q - ga;
    let db = q - gb;
    return select(gb, ga, dot(da, da) < dot(db, db));
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let s = max(p.size, 1.0);
    let px = uv * res();
    let m = i32(p.shape);
    if (m == 1) {
        let c = hex_center(px / s) * s;
        return src_clamp(c / res());
    }
    let cell = (floor(px / s) + 0.5) * s;
    let c = src_clamp(cell / res());
    if (m == 2) {
        let d = length(px - cell) / (s * 0.5);
        return c * (1.0 - smoothstep(0.85, 1.0, d));
    }
    return c;
}
