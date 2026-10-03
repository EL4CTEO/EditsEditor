//! id = "tile"
//! name = "Motion Tile"
//! kind = "filter"
//! category = "distort"
//! description = "Tile/repeat the image in a grid, optionally mirrored, with scrolling offset."
//! tags = ["tile", "repeat", "grid", "pattern"]
//! params = [
//!   { name = "count", type = "vec2", default = [3.0, 3.0], desc = "Tiles across / down" },
//!   { name = "offset", type = "vec2", default = [0.0, 0.0], desc = "Scroll (tiles)" },
//!   { name = "mirror_edges", type = "bool", default = false },
//!   { name = "gap", type = "float", default = 0.0, min = 0.0, max = 0.4, desc = "Gap between tiles (fraction)" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let q = uv * max(p.count, vec2f(0.01)) + p.offset;
    var f = fract(q);
    if (p.mirror_edges > 0.5) {
        let cell = floor(q);
        f = select(f, 1.0 - f, fmod_v2(cell, 2.0) > vec2f(0.5));
    }
    let g = p.gap * 0.5;
    let inside_tile = select(0.0, 1.0, all(f > vec2f(g)) && all(f < vec2f(1.0 - g)));
    f = (f - g) / max(1.0 - 2.0 * g, 0.0001);
    return src_clamp(f) * inside_tile;
}
