//! id = "gradient"
//! name = "Gradient"
//! kind = "generator"
//! category = "background"
//! description = "Linear, radial, conic or diamond gradient with up to 4 colors and animated offset."
//! tags = ["gradient", "background", "color"]
//! params = [
//!   { name = "shape", type = "enum", options = ["linear", "radial", "conic", "diamond"], default = "linear" },
//!   { name = "color1", type = "color", default = "#1a0533" },
//!   { name = "color2", type = "color", default = "#6a11cb" },
//!   { name = "color3", type = "color", default = "#ff4e9b" },
//!   { name = "color4", type = "color", default = "#ffd36e" },
//!   { name = "stops", type = "int", default = 3, min = 2, max = 4, desc = "Number of colors used" },
//!   { name = "angle", type = "angle", default = 90.0 },
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "offset", type = "float", default = 0.0, desc = "Animated shift (cycles)" },
//!   { name = "repeat", type = "bool", default = false },
//! ]
fn grad(t: f32, p: Params) -> vec3f {
    let n = clamp(p.stops, 2.0, 4.0) - 1.0;
    let x = clamp(t, 0.0, 1.0) * n;
    var cols = array<vec3f, 4>(p.color1.rgb, p.color2.rgb, p.color3.rgb, p.color4.rgb);
    let i = i32(min(floor(x), n - 1.0));
    return mix(cols[i], cols[i + 1], x - f32(i));
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = centered(uv, p.center);
    var t = 0.0;
    let m = i32(p.shape);
    if (m == 0) { t = dot(uv - 0.5, dir_deg(p.angle)) + 0.5; }
    else if (m == 1) { t = length(d) * 1.4; }
    else if (m == 2) { t = fract(atan2(d.y, d.x) / TAU + 0.5 + p.angle / 360.0); }
    else { t = (abs(d.x) + abs(d.y)) * 1.2; }
    t += p.offset;
    if (p.repeat > 0.5) { t = 1.0 - abs(fract(t * 0.5) * 2.0 - 1.0); } else { t = clamp(t, 0.0, 1.0); }
    return vec4f(grad(t, p), 1.0);
}
