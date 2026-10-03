//! id = "morph"
//! name = "Luma Morph"
//! kind = "transition"
//! category = "organic"
//! description = "Displaces each image by the other's luminance while crossfading — a liquid morph."
//! tags = ["morph", "displace", "liquid", "smooth"]
//! params = [ { name = "strength", type = "float", default = 0.1, min = 0.0, max = 0.5 } ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let a = src_clamp(uv);
    let b = orig_clamp(uv);
    let oa = (a.rg - 0.5) * p.strength;
    let ob = (b.rg - 0.5) * p.strength;
    let ca = src_clamp(uv + ob * t);
    let cb = orig_clamp(uv - oa * (1.0 - t));
    return mix(ca, cb, t);
}
