//! id = "surface_blur"
//! name = "Surface Blur"
//! kind = "filter"
//! category = "blur"
//! description = "Edge-preserving (bilateral) smoothing. Cleans noise while keeping line art."
//! tags = ["blur", "denoise", "smooth", "skin"]
//! params = [
//!   { name = "radius", type = "float", default = 4.0, min = 1.0, max = 16.0 },
//!   { name = "threshold", type = "float", default = 0.1, min = 0.01, max = 1.0, desc = "Color difference that counts as an edge" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let c = src_clamp(uv);
    var acc = vec4f(0.0);
    var wsum = 0.0;
    for (var y = -3; y <= 3; y++) {
        for (var x = -3; x <= 3; x++) {
            let o = vec2f(f32(x), f32(y)) * p.radius / 3.0;
            let s = src_clamp(uv + o * texel());
            let dc = length(s.rgb - c.rgb);
            let w = exp(-dot(o, o) / (2.0 * p.radius * p.radius)) * exp(-(dc * dc) / (2.0 * p.threshold * p.threshold));
            acc += s * w;
            wsum += w;
        }
    }
    return acc / max(wsum, 0.0001);
}
