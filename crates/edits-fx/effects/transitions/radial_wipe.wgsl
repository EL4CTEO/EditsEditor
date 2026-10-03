//! id = "radial_wipe"
//! name = "Clock Wipe"
//! kind = "transition"
//! category = "wipe"
//! description = "Radial sweep like a clock hand."
//! tags = ["wipe", "clock", "radial"]
//! params = [
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "start_angle", type = "angle", default = -90.0 },
//!   { name = "softness", type = "float", default = 0.02, min = 0.0, max = 0.5 },
//!   { name = "clockwise", type = "bool", default = true },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = centered(uv, p.center);
    var a = (atan2(d.y, d.x) - radians(p.start_angle)) / TAU;
    a = fract(a);
    if (p.clockwise < 0.5) { a = 1.0 - a; }
    let t = progress() * (1.0 + p.softness);
    let k = smoothstep(t - p.softness, t, a);
    return mix(to_img(uv), from_img(uv), k);
}
