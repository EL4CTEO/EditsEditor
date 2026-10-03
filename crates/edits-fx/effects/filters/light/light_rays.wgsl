//! id = "light_rays"
//! name = "Light Rays (God Rays)"
//! kind = "filter"
//! category = "light"
//! description = "Volumetric rays streaming from bright areas away from a light point."
//! tags = ["rays", "god rays", "light", "volumetric", "dramatic"]
//! params = [
//!   { name = "center", type = "point", default = [0.5, 0.3], desc = "Light source position" },
//!   { name = "length", type = "float", default = 0.4, min = 0.0, max = 1.0, desc = "Ray length" },
//!   { name = "intensity", type = "float", default = 1.0, min = 0.0, max = 5.0 },
//!   { name = "threshold", type = "float", default = 0.6, min = 0.0, max = 1.5 },
//!   { name = "color", type = "color", default = "#fff2d6" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let n = 48;
    let d = (uv - p.center) * p.length / f32(n);
    var pos = uv;
    var acc = vec3f(0.0);
    var w = 1.0;
    let jitter = hash12(uv * res() + time()) ;
    pos -= d * jitter;
    for (var i = 0; i < n; i++) {
        pos -= d;
        let s = src_clamp(pos);
        let b = max(luma(s.rgb) - p.threshold, 0.0);
        acc += s.rgb * b * w;
        w *= 0.96;
    }
    let o = orig(uv);
    let rays = acc / f32(n) * p.intensity * 4.0 * p.color.rgb;
    return vec4f(o.rgb + rays, max(o.a, clamp(luma(rays), 0.0, 1.0)));
}
