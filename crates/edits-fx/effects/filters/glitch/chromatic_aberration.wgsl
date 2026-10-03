//! id = "chromatic_aberration"
//! name = "Chromatic Aberration"
//! kind = "filter"
//! category = "glitch"
//! description = "Radial lens color fringing, stronger towards the edges, with spectral blur."
//! tags = ["chromatic aberration", "lens", "fringe", "rgb", "amv"]
//! params = [
//!   { name = "amount", type = "float", default = 0.01, min = 0.0, max = 0.2 },
//!   { name = "center", type = "point", default = [0.5, 0.5] },
//!   { name = "samples", type = "int", default = 8, min = 3, max = 32, desc = "Spectral samples (smoothness)" },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let d = uv - p.center;
    let n = i32(clamp(p.samples, 3.0, 32.0));
    var acc = vec4f(0.0);
    var wsum = vec3f(0.0);
    for (var i = 0; i < n; i++) {
        let t = f32(i) / f32(n - 1);
        let s = src(p.center + d * (1.0 - p.amount * (t - 0.5) * 2.0));
        let w = clamp(vec3f(1.0 - t * 2.0, 1.0 - abs(t - 0.5) * 2.0, t * 2.0 - 1.0), vec3f(0.0), vec3f(1.0)) + 0.001;
        acc += vec4f(s.rgb * w, s.a);
        wsum += w;
    }
    return vec4f(acc.rgb / wsum, acc.a / f32(n));
}
