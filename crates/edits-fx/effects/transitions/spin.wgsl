//! id = "spin"
//! name = "Spin"
//! kind = "transition"
//! category = "motion"
//! description = "Rotate out and in with rotational blur and a zoom dip."
//! tags = ["spin", "rotate", "twist", "amv"]
//! params = [
//!   { name = "turns", type = "float", default = 1.0, min = -5.0, max = 5.0 },
//!   { name = "zoom", type = "float", default = 0.6, min = 0.1, max = 1.0, desc = "Scale at the midpoint" },
//!   { name = "blur", type = "float", default = 1.0, min = 0.0, max = 2.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let ang = (t * t * (3.0 - 2.0 * t)) * p.turns * TAU;
    let sc = mix(1.0, p.zoom, sin(t * PI));
    let speed = sin(t * PI) * p.blur;
    var acc = vec4f(0.0);
    let n = 16;
    for (var i = 0; i < n; i++) {
        let a = ang + (f32(i) / f32(n - 1) - 0.5) * 0.6 * speed;
        let q = uv_transform(uv, vec2f(0.5), a, sc);
        if (t < 0.5) { acc += src_mirror(q); } else { acc += textureSampleLevel(t1, samp, 1.0 - abs(fract(q * 0.5) * 2.0 - 1.0), 0.0); }
    }
    return acc / f32(n);
}
