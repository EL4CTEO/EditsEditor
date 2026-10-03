//! id = "shatter"
//! name = "Shatter (Voronoi)"
//! kind = "transition"
//! category = "geometric"
//! description = "The image breaks into shards that fly outward revealing the next clip."
//! tags = ["shatter", "glass", "break", "voronoi", "impact"]
//! params = [
//!   { name = "cells", type = "float", default = 10.0, min = 2.0, max = 60.0 },
//!   { name = "spread", type = "float", default = 0.5, min = 0.0, max = 2.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let q = uv * vec2f(aspect(), 1.0) * p.cells;
    let cell = floor(q);
    var best = vec2f(0.0);
    var bd = 9.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let c = cell + vec2f(f32(x), f32(y));
            let pt = c + hash22(c);
            let dd = distance(q, pt);
            if (dd < bd) { bd = dd; best = c; }
        }
    }
    let delay = hash12(best + 3.1) * 0.5;
    let k = clamp((t - delay) / 0.5, 0.0, 1.0);
    let dir = normalize(hash22(best) - 0.5 + 0.0001) * p.spread;
    let shard_uv = uv - dir * k * k - vec2f(0.0, -k * k * 0.5);
    let shard = src(shard_uv) * (1.0 - k);
    return over(shard, to_img(uv));
}
