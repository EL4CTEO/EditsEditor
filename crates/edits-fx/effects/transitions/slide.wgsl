//! id = "slide"
//! name = "Slide / Push"
//! kind = "transition"
//! category = "motion"
//! description = "The next clip slides in. Modes: push (both move), cover (new slides over), uncover (old slides away)."
//! tags = ["slide", "push", "cover", "motion"]
//! params = [
//!   { name = "angle", type = "angle", default = 180.0, desc = "Direction the content moves (180 = new clip enters from the right)" },
//!   { name = "slide_mode", type = "enum", options = ["push", "cover", "uncover"], default = "push" },
//!   { name = "blur", type = "float", default = 0.5, min = 0.0, max = 1.0, desc = "Motion blur amount" },
//! ]
fn sample_pair(uv: vec2f, t: f32, d: vec2f, m: i32) -> vec4f {
    var off_from = vec2f(0.0);
    var off_to = -d;
    if (m == 0) { off_from = d * t; off_to = -d * (1.0 - t); }
    else if (m == 1) { off_to = -d * (1.0 - t); }
    else { off_from = d * t; off_to = vec2f(0.0); }
    let a = from_img(uv - off_from);
    let b = to_img(uv - off_to);
    if (m == 2) { return over(a, b); }
    return over(b, a);
}
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = -dir_deg(p.angle);
    let t = progress();
    let m = i32(p.slide_mode);
    let speed = sin(t * PI);
    var acc = vec4f(0.0);
    let n = 8;
    for (var i = 0; i < n; i++) {
        let dt = (f32(i) / f32(n - 1) - 0.5) * 0.08 * p.blur * speed;
        acc += sample_pair(uv, clamp(t + dt, 0.0, 1.0), d, m);
    }
    return acc / f32(n);
}
