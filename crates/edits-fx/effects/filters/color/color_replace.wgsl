//! id = "color_replace"
//! name = "Color Replace"
//! kind = "filter"
//! category = "color"
//! description = "Shift one hue range to another hue (change hair/eye/outfit colors)."
//! tags = ["color", "replace", "hue", "recolor"]
//! params = [
//!   { name = "source_hue", type = "angle", default = 0.0 },
//!   { name = "target_hue", type = "angle", default = 200.0 },
//!   { name = "range", type = "float", default = 25.0, min = 1.0, max = 180.0 },
//!   { name = "softness", type = "float", default = 15.0, min = 0.0, max = 90.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    var h = rgb2hsv(saturate3(c.rgb));
    let dh = abs(fract(h.x - p.source_hue / 360.0 + 0.5) - 0.5) * 360.0;
    let k = 1.0 - smoothstep(p.range, p.range + p.softness, dh);
    let shift = (p.target_hue - p.source_hue) / 360.0;
    h.x = fract(h.x + shift * k);
    return grade_end(vec4f(hsv2rgb(h), c.a));
}
