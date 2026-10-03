//! id = "spin_blur"
//! name = "Spin Blur"
//! kind = "filter"
//! category = "blur"
//! description = "Rotational blur around a center (for spins and twirl transitions)."
//! tags = ["blur", "rotate", "spin"]
//! params = [
//!   { name = "angle", type = "angle", default = 10.0, min = 0.0, max = 360.0, desc = "Blur arc in degrees" },
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "samples", type = "int", default = 32, min = 4, max = 96 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let n = i32(clamp(p.samples, 4.0, 96.0));
    let a = radians(p.angle);
    var acc = vec4f(0.0);
    for (var i = 0; i < n; i++) {
        let t = (f32(i) / f32(n - 1) - 0.5) * a;
        acc += src_clamp(uv_transform(uv, p.center, t, 1.0));
    }
    return acc / f32(n);
}
