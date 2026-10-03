//! id = "glitch_transition"
//! name = "Glitch Transition"
//! kind = "transition"
//! category = "glitch"
//! description = "Digital corruption bridges the cut: displaced blocks, RGB tearing and random swaps."
//! tags = ["glitch", "digital", "amv", "cut"]
//! params = [
//!   { name = "intensity", type = "float", default = 1.0, min = 0.0, max = 2.0 },
//!   { name = "block_size", type = "float", default = 30.0, min = 4.0, max = 200.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let k = sin(t * PI) * p.intensity;
    let f = floor(t * 30.0);
    let row = floor(uv.y * res().y / p.block_size);
    let blk = floor(uv * res() / (p.block_size * vec2f(3.0, 1.0)));
    var q = uv;
    q.x += (hash12(vec2f(row, f)) - 0.5) * 0.25 * k * step(0.5, hash12(vec2f(row * 1.7, f)));
    let sw = hash12(blk + f) < t;
    let sh = 0.02 * k;
    var c = vec4f(0.0);
    if (sw) {
        c = vec4f(orig_clamp(q + vec2f(sh, 0.0)).r, orig_clamp(q).g, orig_clamp(q - vec2f(sh, 0.0)).b, orig_clamp(q).a);
    } else {
        c = vec4f(src_clamp(q + vec2f(sh, 0.0)).r, src_clamp(q).g, src_clamp(q - vec2f(sh, 0.0)).b, src_clamp(q).a);
    }
    return c;
}
