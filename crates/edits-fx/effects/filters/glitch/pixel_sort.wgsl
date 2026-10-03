//! id = "pixel_sort"
//! name = "Pixel Sort (Streaks)"
//! kind = "filter"
//! category = "glitch"
//! description = "Fake pixel sorting: bright pixels smear in streaks along a direction (databending aesthetic)."
//! tags = ["pixel sort", "datamosh", "glitch", "streak", "databend"]
//! params = [
//!   { name = "length", type = "float", default = 120.0, min = 1.0, max = 1000.0, desc = "Streak length in pixels" },
//!   { name = "threshold", type = "float", default = 0.55, min = 0.0, max = 1.0 },
//!   { name = "angle", type = "angle", default = 90.0 },
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = dir_deg(p.angle) * texel();
    let c = src_clamp(uv);
    var best = c;
    var bl = luma(c.rgb);
    let n = 40;
    let col = hash11(floor(dot(uv * res(), vec2f(-dir_deg(p.angle).y, dir_deg(p.angle).x))) + seed());
    let len = p.length * (0.3 + col);
    for (var i = 1; i <= n; i++) {
        let s = src_clamp(uv - d * f32(i) / f32(n) * len);
        let l = luma(s.rgb);
        if (l > p.threshold && l > bl) { best = s; bl = l; }
    }
    return mix(c, best, p.amount);
}
