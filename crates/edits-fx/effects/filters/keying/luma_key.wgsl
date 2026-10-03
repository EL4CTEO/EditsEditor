//! id = "luma_key"
//! name = "Luma Key"
//! kind = "filter"
//! category = "keying"
//! description = "Make dark (or bright) areas transparent — use black-background overlays (sparks, flares) without blend modes."
//! tags = ["key", "luma key", "black background", "overlay"]
//! params = [
//!   { name = "threshold", type = "float", default = 0.1, min = 0.0, max = 1.0 },
//!   { name = "softness", type = "float", default = 0.1, min = 0.0, max = 1.0 },
//!   { name = "invert", type = "bool", default = false, desc = "Key out bright areas instead" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = src_clamp(uv);
    let l = luma(to_srgb(unpremul(c).rgb));
    var a = smoothstep(p.threshold, p.threshold + p.softness + 0.0001, l);
    if (p.invert > 0.5) { a = 1.0 - a; }
    return c * a;
}
