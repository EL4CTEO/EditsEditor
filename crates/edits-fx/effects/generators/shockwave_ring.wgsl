//! id = "shockwave_ring"
//! name = "Shockwave Ring"
//! kind = "generator"
//! category = "anime"
//! description = "Expanding glowing ring(s) from a point. Animate progress via clip time (radius grows with time)."
//! tags = ["shockwave", "ring", "impact", "energy", "amv"]
//! params = [
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "speed", type = "float", default = 1.2, min = 0.0, max = 10.0, desc = "Radius growth per second (frame heights)" },
//!   { name = "width", type = "float", default = 0.03, min = 0.001, max = 0.5 },
//!   { name = "rings", type = "int", default = 2, min = 1, max = 8 },
//!   { name = "spacing", type = "float", default = 0.12, min = 0.0, max = 1.0, desc = "Seconds between rings" },
//!   { name = "color", type = "color", default = "#9fd8ff" },
//!   { name = "intensity", type = "float", default = 2.0, min = 0.0, max = 10.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let r = length(centered(uv, p.center));
    var v = 0.0;
    let n = i32(clamp(p.rings, 1.0, 8.0));
    for (var i = 0; i < n; i++) {
        let t = ltime() - f32(i) * p.spacing;
        if (t < 0.0) { continue; }
        let rad = t * p.speed;
        let fade = exp(-t * 2.0);
        let x = (r - rad) / p.width;
        v += exp(-x * x) * fade;
    }
    let c = p.color.rgb * v * p.intensity;
    return vec4f(c, clamp(v, 0.0, 1.0));
}
