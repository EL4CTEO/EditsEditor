//! id = "brightness_contrast"
//! name = "Brightness & Contrast"
//! kind = "filter"
//! category = "color"
//! description = "Basic brightness and contrast (perceptual space)."
//! tags = ["color", "brightness", "contrast", "basic"]
//! params = [
//!   { name = "brightness", type = "float", default = 0.0, min = -1.0, max = 1.0 },
//!   { name = "contrast", type = "float", default = 0.0, min = -1.0, max = 3.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    var c = grade_begin(src_clamp(uv));
    c = vec4f((c.rgb - 0.5) * (1.0 + p.contrast) + 0.5 + p.brightness, c.a);
    return grade_end(c);
}
