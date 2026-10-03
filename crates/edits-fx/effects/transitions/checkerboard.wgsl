//! id = "checkerboard"
//! name = "Checkerboard / Grid Pop"
//! kind = "transition"
//! category = "geometric"
//! description = "Grid cells pop to the next clip in random order with a scale-in."
//! tags = ["grid", "checker", "tiles", "pop"]
//! params = [
//!   { name = "cells", type = "float", default = 8.0, min = 2.0, max = 64.0 },
//!   { name = "order", type = "enum", options = ["random", "diagonal", "checker", "spiral"], default = "random" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let g = vec2f(p.cells * aspect(), p.cells);
    let cell = floor(uv * g);
    let f = fract(uv * g) - 0.5;
    let m = i32(p.order);
    var delay = hash12(cell + seed());
    if (m == 1) { delay = (cell.x + cell.y) / (g.x + g.y); }
    else if (m == 2) { delay = fmod_pos(cell.x + cell.y, 2.0) * 0.5 + hash12(cell) * 0.2; }
    else if (m == 3) { delay = length(cell - g * 0.5) / length(g * 0.5); }
    let t = clamp((progress() - delay * 0.6) / 0.4, 0.0, 1.0);
    let inside_cell = select(0.0, 1.0, max(abs(f.x), abs(f.y)) < t * 0.5 + 0.0001);
    return mix(from_img(uv), to_img(uv), inside_cell * step(0.0001, t));
}
