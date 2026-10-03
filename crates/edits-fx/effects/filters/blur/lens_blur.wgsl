//! id = "lens_blur"
//! name = "Lens Blur (Bokeh)"
//! kind = "filter"
//! category = "blur"
//! description = "Disc-shaped defocus with bright bokeh highlights."
//! tags = ["blur", "bokeh", "defocus", "dreamy"]
//! params = [
//!   { name = "radius", type = "float", default = 10.0, min = 0.0, max = 60.0, desc = "Disc radius in pixels" },
//!   { name = "highlight", type = "float", default = 2.0, min = 0.0, max = 10.0, desc = "Boost of bright areas (bokeh balls)" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    var acc = vec4f(0.0);
    var wsum = 0.0;
    let golden = 2.39996323;
    let n = 64;
    for (var i = 0; i < n; i++) {
        let r = sqrt(f32(i) / f32(n)) * p.radius;
        let a = f32(i) * golden;
        let c = src_clamp(uv + vec2f(cos(a), sin(a)) * r * texel());
        let w = 1.0 + pow(luma(c.rgb), 3.0) * p.highlight;
        acc += c * w;
        wsum += w;
    }
    return acc / wsum;
}
