//! id = "glitch"
//! name = "Glitch"
//! kind = "filter"
//! category = "glitch"
//! description = "Digital block glitch: displaced slices, channel shifts and color corruption. Animate `intensity` on hits."
//! tags = ["glitch", "digital", "corrupt", "amv", "datamosh"]
//! params = [
//!   { name = "intensity", type = "float", default = 0.5, min = 0.0, max = 1.0 },
//!   { name = "speed", type = "float", default = 12.0, min = 0.5, max = 60.0, desc = "Glitch changes per second" },
//!   { name = "block_size", type = "float", default = 40.0, min = 2.0, max = 400.0 },
//!   { name = "rgb_shift", type = "float", default = 20.0, min = 0.0, max = 200.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = floor(time() * p.speed) + seed() * 77.0;
    let px = uv * res();
    let row = floor(px.y / p.block_size);
    let blk = floor(px / (p.block_size * vec2f(4.0, 1.0)));
    let r1 = hash12(vec2f(row, t));
    let r2 = hash12(blk + t * 1.3);
    var q = uv;
    let on = step(1.0 - p.intensity * 0.6, r1);
    q.x += (hash12(vec2f(row, t + 9.0)) - 0.5) * 0.3 * p.intensity * on;
    let blk_on = step(1.0 - p.intensity * 0.35, r2);
    q += (hash22(blk + t) - 0.5) * 0.1 * blk_on;
    let sh = p.rgb_shift * p.intensity * (0.3 + on) * texel().x;
    let c = src_clamp(q);
    var o = vec4f(src_clamp(q + vec2f(sh, 0.0)).r, c.g, src_clamp(q - vec2f(sh, 0.0)).b, c.a);
    if (blk_on > 0.5 && hash12(blk + t * 2.1) > 0.6) { o = vec4f(o.gbr, o.a); }
    if (blk_on > 0.5 && hash12(blk + t * 3.7) > 0.85) { o = vec4f(vec3f(1.0) - o.rgb * o.a, o.a); }
    return o;
}
