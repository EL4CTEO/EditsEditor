//! id = "hue_saturation"
//! name = "Hue / Saturation"
//! kind = "filter"
//! category = "color"
//! description = "Shift hue, change saturation and lightness."
//! tags = ["color", "hue", "saturation", "lightness"]
//! params = [
//!   { name = "hue", type = "angle", default = 0.0, min = -180.0, max = 180.0, desc = "Hue shift in degrees" },
//!   { name = "saturation", type = "float", default = 1.0, min = 0.0, max = 4.0, desc = "Saturation multiplier" },
//!   { name = "lightness", type = "float", default = 0.0, min = -1.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    var h = rgb2hsv(saturate3(c.rgb));
    h.x = fract(h.x + p.hue / 360.0);
    h.y = clamp(h.y * p.saturation, 0.0, 1.0);
    var rgb = hsv2rgb(h);
    rgb = select(mix(rgb, vec3f(0.0), -p.lightness), mix(rgb, vec3f(1.0), p.lightness), p.lightness > 0.0);
    return grade_end(vec4f(rgb, c.a));
}
