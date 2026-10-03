//! id = "flash_cut"
//! name = "Flash Cut"
//! kind = "transition"
//! category = "light"
//! description = "Over-exposes the outgoing clip into a white flash and fades the new clip out of it. The AMV classic."
//! tags = ["flash", "white", "impact", "amv", "exposure"]
//! params = [
//!   { name = "color", type = "color", default = "#ffffff" },
//!   { name = "exposure", type = "float", default = 3.0, min = 0.0, max = 8.0 },
//!   { name = "peak", type = "float", default = 0.4, min = 0.05, max = 0.95, desc = "Where the flash peaks" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let col = vec4f(p.color.rgb, 1.0);
    if (t < p.peak) {
        let k = t / p.peak;
        let a = from_img(uv);
        return mix(vec4f(a.rgb * exp2(p.exposure * k * k), a.a), col, k * k);
    }
    let k = (t - p.peak) / (1.0 - p.peak);
    let b = to_img(uv);
    return mix(col, b, ease_out_expo(k));
}
