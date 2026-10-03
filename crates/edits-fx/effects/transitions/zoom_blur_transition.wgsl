//! id = "zoom_out"
//! name = "Zoom Out"
//! kind = "transition"
//! category = "motion"
//! description = "Outgoing clip shrinks away into the distance while the incoming clip zooms out from a close-up."
//! tags = ["zoom", "out", "shrink", "motion"]
//! params = [
//!   { name = "blur", type = "float", default = 0.25, min = 0.0, max = 1.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let t = progress();
    let e = t * t * (3.0 - 2.0 * t);
    var acc = vec4f(0.0);
    let n = 12;
    for (var i = 0; i < n; i++) {
        let f = 1.0 + p.blur * sin(t * PI) * f32(i) / f32(n);
        let sa = mix(1.0, 0.0001, e) * f;
        let sb = mix(4.0, 1.0, e) / f;
        let a = src((uv - 0.5) / sa + 0.5);
        let b = orig_clamp((uv - 0.5) / sb + 0.5);
        acc += over(a * (1.0 - e), b);
    }
    return acc / f32(n);
}
