//! id = "color_pop"
//! name = "Color Pop (Selective Color)"
//! kind = "filter"
//! category = "color"
//! description = "Keep one hue in color and desaturate everything else (e.g. keep red eyes / hair)."
//! tags = ["color", "selective", "pop", "keep color", "amv"]
//! params = [
//!   { name = "hue", type = "angle", default = 0.0, desc = "Hue to keep (0 red, 120 green, 240 blue)" },
//!   { name = "range", type = "float", default = 30.0, min = 1.0, max = 180.0, desc = "Hue tolerance in degrees" },
//!   { name = "softness", type = "float", default = 15.0, min = 0.0, max = 90.0 },
//!   { name = "min_saturation", type = "float", default = 0.15, min = 0.0, max = 1.0 },
//!   { name = "amount", type = "float", default = 1.0, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = grade_begin(src_clamp(uv));
    let h = rgb2hsv(saturate3(c.rgb));
    let dh = abs(fract(h.x - p.hue / 360.0 + 0.5) - 0.5) * 360.0;
    let keep = (1.0 - smoothstep(p.range, p.range + p.softness, dh)) * smoothstep(p.min_saturation * 0.5, p.min_saturation, h.y);
    let g = vec3f(luma(c.rgb));
    return grade_end(vec4f(mix(c.rgb, mix(g, c.rgb, keep), p.amount), c.a));
}
