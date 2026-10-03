//! id = "tilt_shift"
//! name = "Tilt Shift"
//! kind = "filter"
//! category = "blur"
//! description = "Miniature effect: sharp band, blurred top and bottom."
//! tags = ["blur", "miniature", "focus"]
//! params = [
//!   { name = "radius", type = "float", default = 14.0, min = 0.0, max = 60.0 },
//!   { name = "focus", type = "float", default = 0.5, min = 0.0, max = 1.0, desc = "Vertical position of the sharp band" },
//!   { name = "width", type = "float", default = 0.15, min = 0.0, max = 1.0, desc = "Height of the sharp band" },
//!   { name = "angle", type = "angle", default = 0.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = rot2(radians(p.angle)) * (uv - vec2f(0.5, p.focus));
    let k = smoothstep(p.width * 0.5, p.width * 0.5 + 0.25, abs(d.y));
    let r = p.radius * k;
    var acc = vec4f(0.0);
    let golden = 2.39996323;
    for (var i = 0; i < 32; i++) {
        let rr = sqrt(f32(i) / 32.0) * r;
        let a = f32(i) * golden;
        acc += src_clamp(uv + vec2f(cos(a), sin(a)) * rr * texel());
    }
    return acc / 32.0;
}
