//! id = "directional_blur"
//! name = "Directional Blur"
//! kind = "filter"
//! category = "blur"
//! description = "Motion blur along an angle. Animate length for whip/speed moments."
//! tags = ["blur", "motion", "speed", "whip"]
//! params = [
//!   { name = "length", type = "float", default = 40.0, min = 0.0, max = 600.0, desc = "Blur length in pixels" },
//!   { name = "angle", type = "angle", default = 0.0, desc = "Direction (degrees, 0 = horizontal)" },
//!   { name = "samples", type = "int", default = 32, min = 4, max = 96, desc = "Quality" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let n = i32(clamp(p.samples, 4.0, 96.0));
    let d = dir_deg(p.angle) * p.length * texel();
    var acc = vec4f(0.0);
    for (var i = 0; i < n; i++) {
        let t = f32(i) / f32(n - 1) - 0.5;
        acc += src_clamp(uv + d * t);
    }
    return acc / f32(n);
}
