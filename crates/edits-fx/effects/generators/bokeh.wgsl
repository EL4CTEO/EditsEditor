//! id = "bokeh"
//! name = "Bokeh Lights"
//! kind = "generator"
//! category = "particles"
//! description = "Soft out-of-focus light circles drifting slowly (romantic / night city vibes)."
//! tags = ["bokeh", "lights", "dreamy", "romantic", "night"]
//! params = [
//!   { name = "count", type = "float", default = 30.0, min = 1.0, max = 120.0 },
//!   { name = "size", type = "float", default = 1.0, min = 0.2, max = 4.0 },
//!   { name = "speed", type = "float", default = 0.05, min = 0.0, max = 1.0 },
//!   { name = "color_a", type = "color", default = "#ffb347" },
//!   { name = "color_b", type = "color", default = "#ff4fa3" },
//!   { name = "intensity", type = "float", default = 0.6, min = 0.0, max = 3.0 },
//! ]
fn effect(uv: vec2f, p: Params) -> vec4f {
    let a = vec2f(aspect(), 1.0);
    var c = vec3f(0.0);
    let n = i32(clamp(p.count, 1.0, 120.0));
    for (var i = 0; i < n; i++) {
        let fi = f32(i) + seed() * 50.0;
        let h = hash22(vec2f(fi, 9.1));
        var pos = vec2f(h.x, hash11(fi * 2.7));
        pos += vec2f(sin(time() * p.speed * 6.0 + fi), cos(time() * p.speed * 4.0 + fi * 1.3)) * 0.05;
        let r = (0.03 + 0.06 * h.y) * p.size;
        let d = length((uv - pos) * a);
        let disc = smoothstep(r, r * 0.85, d) * (0.6 + 0.4 * smoothstep(r * 0.6, r, d));
        c += mix(p.color_a.rgb, p.color_b.rgb, h.y) * disc * (0.3 + 0.7 * hash11(fi * 5.1));
    }
    c *= p.intensity;
    return vec4f(c, clamp(luma(c) * 2.0, 0.0, 1.0));
}
