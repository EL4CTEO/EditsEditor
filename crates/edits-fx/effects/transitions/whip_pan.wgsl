//! id = "whip_pan"
//! name = "Whip Pan"
//! kind = "transition"
//! category = "motion"
//! description = "Very fast pan with heavy motion blur between clips — energetic AMV cut."
//! tags = ["whip", "pan", "motion blur", "fast", "amv"]
//! params = [
//!   { name = "angle", type = "angle", default = 0.0 },
//!   { name = "blur", type = "float", default = 1.0, min = 0.0, max = 3.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let d = dir_deg(p.angle);
    let e = t * t * (3.0 - 2.0 * t);
    let speed = sin(t * PI);
    var acc = vec4f(0.0);
    let n = 24;
    for (var i = 0; i < n; i++) {
        let s = (f32(i) / f32(n - 1) - 0.5) * 0.35 * speed * p.blur;
        let off = d * (e * 2.0 + s);
        let q = uv + off;
        let fq = q;
        let tq = q - d * 2.0;
        var c = vec4f(0.0);
        if (e + s < 0.5) { c = src_mirror(fq); } else { c = textureSampleLevel(t1, samp, 1.0 - abs(fract(tq * 0.5) * 2.0 - 1.0), 0.0); }
        acc += c;
    }
    return acc / f32(n);
}
