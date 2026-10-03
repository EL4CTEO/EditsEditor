//! id = "pixelize"
//! name = "Pixelize"
//! kind = "transition"
//! category = "glitch"
//! description = "Pixelate out, swap, pixelate back in (retro game transition)."
//! tags = ["pixel", "pixelate", "retro", "8bit"]
//! params = [ { name = "max_size", type = "float", default = 60.0, min = 4.0, max = 400.0 } ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let k = 1.0 - abs(t - 0.5) * 2.0;
    let s = max(1.0, floor(p.max_size * k * k));
    let q = (floor(uv * res() / s) + 0.5) * s / res();
    return mix(src_clamp(q), orig_clamp(q), step(0.5, t));
}
