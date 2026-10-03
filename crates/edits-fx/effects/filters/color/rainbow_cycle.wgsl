//! id = "rainbow_cycle"
//! name = "Rainbow Cycle"
//! kind = "filter"
//! category = "color"
//! description = "Continuously cycling hue with an optional spatial rainbow sweep."
//! tags = ["color", "rainbow", "hue", "cycle", "psychedelic"]
//! params = [
//!   { name = "speed", type = "float", default = 0.5, min = -10.0, max = 10.0, desc = "Hue turns per second" },
//!   { name = "spread", type = "float", default = 0.0, min = 0.0, max = 5.0, desc = "Spatial hue gradient amount" },
//!   { name = "angle", type = "angle", default = 45.0 },
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    var h = rgb2hsv(saturate3(c.rgb));
    let s = dot(uv - 0.5, dir_deg(p.angle)) * p.spread;
    h.x = fract(h.x + time() * p.speed + s);
    return grade_end(vec4f(mix(c.rgb, hsv2rgb(h), p.amount), c.a));
}
