//! id = "hexagonalize"
//! name = "Hexagonalize"
//! kind = "transition"
//! category = "geometric"
//! description = "Image dissolves into hexagon cells and re-forms."
//! tags = ["hexagon", "cells", "geometric", "tech"]
//! params = [ { name = "max_size", type = "float", default = 50.0, min = 4.0, max = 300.0 } ]
fn hexc(q: vec2f) -> vec2f {
    let r = vec2f(1.0, 1.7320508);
    let h = r * 0.5;
    let ga = floor(q / r) * r + h;
    let gb = floor((q - h) / r) * r + r;
    return select(gb, ga, dot(q - ga, q - ga) < dot(q - gb, q - gb));
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let k = 1.0 - abs(t - 0.5) * 2.0;
    let s = max(1.0, p.max_size * k);
    let c = hexc(uv * res() / s) * s / res();
    let q = mix(uv, c, smoothstep(0.0, 0.2, k));
    return mix(src_clamp(q), orig_clamp(q), smoothstep(0.4, 0.6, t));
}
