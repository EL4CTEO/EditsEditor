//! id = "luma_wipe"
//! name = "Luma Wipe"
//! kind = "transition"
//! category = "wipe"
//! description = "Reveals the next clip ordered by the brightness of the outgoing (or incoming) image."
//! tags = ["luma", "wipe", "brightness", "reveal"]
//! params = [
//!   { name = "softness", type = "float", default = 0.1, min = 0.0, max = 0.5 },
//!   { name = "use_incoming", type = "bool", default = false },
//!   { name = "invert", type = "bool", default = false },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    var l = luma(unpremul(from_img(uv)).rgb);
    if (p.use_incoming > 0.5) { l = luma(unpremul(to_img(uv)).rgb); }
    if (p.invert > 0.5) { l = 1.0 - l; }
    let t = progress() * (1.0 + p.softness);
    let k = smoothstep(t - p.softness, t, l);
    return mix(to_img(uv), from_img(uv), k);
}
