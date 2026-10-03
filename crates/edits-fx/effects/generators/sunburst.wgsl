//! id = "sunburst"
//! name = "Sunburst"
//! kind = "generator"
//! category = "background"
//! description = "Rotating radial rays in two colors (retro / anime title backgrounds)."
//! tags = ["sunburst", "rays", "retro", "background", "title"]
//! params = [
//!   { name = "rays", type = "float", default = 16.0, min = 2.0, max = 100.0 },
//!   { name = "speed", type = "float", default = 0.05, min = -2.0, max = 2.0, desc = "Turns per second" },
//!   { name = "color_a", type = "color", default = "#ff2e63" },
//!   { name = "color_b", type = "color", default = "#ffb6b9" },
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "vignette", type = "float", default = 0.4, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = centered(uv, p.center);
    let a = atan2(d.y, d.x) / TAU + time() * p.speed;
    let s = step(0.5, fract(a * p.rays));
    var c = mix(p.color_a.rgb, p.color_b.rgb, s);
    c *= 1.0 - p.vignette * smoothstep(0.2, 1.2, length(d));
    return vec4f(c, 1.0);
}
